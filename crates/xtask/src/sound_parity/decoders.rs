//! The decoder stage's cells (ticket 2210): `shuyi-sound`'s adapters against
//! the corpus. A WAV has one right answer, the numbers that made it. An Ogg
//! file's is what the separate decoder made of it, the same oracle the old
//! backend's recordings are held to, so old and new are compared through it. What
//! a malformed file must do, and how fast against the old path, are here too.

use std::fs;
use std::path::Path;

use serde::Deserialize;
use serde_json::Value;
use shuyi_sound::vorbis::VorbisSource;
use shuyi_sound::wav::WavSource;
use shuyi_sound::{Error, Limits, Pcm, Source, read_all};

use super::compare::Tolerance;
use super::corpus::{self, Signal};
use super::{Limit, cell, judge, median, ratios, verdict};

/// The cells' platform: decoding happens nowhere in particular.
const PLATFORM: &str = "decoders";
/// A chunk small enough to split every packet, in samples.
const SMALL_CHUNK: usize = 334;
/// The file whose comments are checked, and what it says.
const REGION_FILE: &str = "intro_loop.ogg";
const REGION: [(&str, &str); 2] = [("LOOPSTART", "44100"), ("LOOPLENGTH", "44100")];

/// How a platform's decoders are held, by file kind (`budget.ron`).
#[derive(Debug, Clone, Copy, Deserialize)]
pub(crate) struct DecoderLimit {
    /// New against old on the same work. `None` where they aren't the same work:
    /// a browser decodes on its own threads, in its own code.
    ratio: Option<Limit>,
    /// The most of the sound's own length that decoding it may take.
    max_share: f64,
}

fn whole_wav(bytes: &[u8], limits: &Limits) -> Result<Pcm, Error> {
    read_all(&mut WavSource::new(bytes, limits)?, limits)
}

fn whole_ogg(bytes: &[u8], limits: &Limits) -> Result<Pcm, Error> {
    read_all(&mut VorbisSource::new(bytes, limits)?, limits)
}

/// What's wrong with `pcm` as `signal`'s rate and channels and `expected`'s
/// samples, to within `peak` a sample.
fn compare(pcm: &Pcm, signal: &Signal, expected: &[i16], peak: i32) -> Vec<String> {
    let mut problems = Vec::new();
    if pcm.format().rate != signal.rate {
        problems.push(format!("rate {}, not {}", pcm.format().rate, signal.rate));
    }
    if pcm.format().channels.count() != signal.channels {
        let channels = pcm.format().channels.count();
        problems.push(format!("{channels} channels, not {}", signal.channels));
    }
    if pcm.samples().len() != expected.len() {
        let length = pcm.samples().len();
        problems.push(format!("{length} samples, not {}", expected.len()));
    }
    let worst = pcm
        .samples()
        .iter()
        .zip(expected)
        .map(|(a, b)| (i32::from(*a) - i32::from(*b)).abs())
        .enumerate()
        .max_by_key(|(_, difference)| *difference);
    if let Some((at, difference)) = worst.filter(|(_, difference)| *difference > peak) {
        problems.push(format!("{difference} apart at sample {at}, over {peak}"));
    }
    problems
}

/// What a read of `source` in small chunks gives, or why not.
fn in_small_chunks(source: &mut impl Source) -> Result<Vec<i16>, Error> {
    let channels = usize::from(source.format().channels.count());
    let mut chunk = [0; SMALL_CHUNK];
    let mut all = Vec::new();
    loop {
        let frames = source.read(&mut chunk)?;
        if frames == 0 {
            return Ok(all);
        }
        all.extend_from_slice(&chunk[..frames * channels]);
    }
}

/// Every source against its own numbers, and in small chunks.
fn wav_cells() -> Vec<Value> {
    let limits = Limits::default();
    let mut cells = Vec::new();
    for (name, signal) in corpus::sources() {
        let bytes = corpus::wav(&signal);
        let mut problems = match whole_wav(&bytes, &limits) {
            Ok(pcm) => compare(&pcm, &signal, &signal.samples, 0),
            Err(e) => vec![e.to_string()],
        };
        match WavSource::new(&bytes, &limits).and_then(|mut s| in_small_chunks(&mut s)) {
            Ok(samples) if samples == signal.samples => {}
            Ok(_) => problems.push("in small chunks it comes out differently".to_owned()),
            Err(e) => problems.push(e.to_string()),
        }
        cells.push(verdict(PLATFORM, &format!("decoder:{name}"), &problems));
    }
    cells
}

/// One Ogg file against the separate decoder's samples of it.
fn ogg_problems(name: &str, dir: &Path, tolerance: &Tolerance) -> Vec<String> {
    let limits = Limits::default();
    let read = |path: &Path| fs::read(path).map_err(|e| format!("{}: {e}", path.display()));
    let (bytes, oracle) = match (
        read(&dir.join("corpus").join(name)),
        read(&dir.join("oracle").join(format!("{name}.pcm"))),
    ) {
        (Ok(bytes), Ok(oracle)) => (bytes, corpus::samples(&oracle)),
        (bytes, oracle) => return bytes.err().into_iter().chain(oracle.err()).collect(),
    };
    let sources = corpus::sources();
    let source = name.replace(".ogg", ".wav");
    let Some((_, signal)) = sources.iter().find(|(n, _)| *n == source) else {
        return vec![format!("no source {source}")];
    };
    let mut problems = match whole_ogg(&bytes, &limits) {
        Ok(pcm) => compare(&pcm, signal, &oracle, tolerance.decoded_peak),
        Err(e) => return vec![e.to_string()],
    };
    let whole = whole_ogg(&bytes, &limits).map(Pcm::into_samples);
    match VorbisSource::new(&bytes, &limits).and_then(|mut s| in_small_chunks(&mut s)) {
        Ok(samples) if Ok(&samples) == whole.as_ref() => {}
        Ok(_) => problems.push("in small chunks it comes out differently".to_owned()),
        Err(e) => problems.push(e.to_string()),
    }
    if name == REGION_FILE {
        match VorbisSource::new(&bytes, &limits) {
            Ok(source) => {
                for (key, value) in REGION {
                    if source.comment(key) != Some(value) {
                        problems.push(format!("its {key} isn't {value}"));
                    }
                }
            }
            Err(e) => problems.push(e.to_string()),
        }
    }
    problems
}

/// Cases a decoder must refuse with an error, each with what must refuse it.
fn refusal_cells(ogg: &[u8]) -> Vec<Value> {
    let limits = Limits::default();
    let wav = |bytes: &[u8]| whole_wav(bytes, &limits).map(drop);
    let ogg_file = |bytes: &[u8]| whole_ogg(bytes, &limits).map(drop);
    let mut cells = Vec::new();
    let mut judged = |name: &str, answer: Result<(), Error>| {
        let problems = match answer {
            Ok(()) => vec!["it was taken".to_owned()],
            Err(_) => Vec::new(),
        };
        let check = format!("decoder-refuses:{name}");
        cells.push(verdict(PLATFORM, &check, &problems));
    };
    for (name, bytes) in corpus::malformed_wavs() {
        judged(name, wav(&bytes));
    }
    judged("ogg_no_bytes", ogg_file(&[]));
    judged("ogg_of_noise", ogg_file(&corpus::wav_noise()));
    judged("ogg_headers_cut", ogg_file(&ogg[..ogg.len().min(64)]));
    let tiny = Limits::new(Limits::DEFAULT_MAX_RATE, 100);
    judged("ogg_over_frames", whole_ogg(ogg, &tiny).map(drop));
    let slow = Limits::new(1000, Limits::DEFAULT_MAX_FRAMES);
    judged("ogg_over_rate", whole_ogg(ogg, &slow).map(drop));
    let bytes = corpus::wav(&corpus::sources()[0].1);
    judged("wav_over_frames", whole_wav(&bytes, &tiny).map(drop));
    judged("wav_over_rate", whole_wav(&bytes, &slow).map(drop));
    cells
}

/// The decoder stage's correctness: every file, then what must be refused.
pub(crate) fn correctness_cells(
    dir: &Path,
    names: &[&String],
    tolerance: &Tolerance,
) -> Vec<Value> {
    let mut cells = wav_cells();
    for name in names {
        let problems = ogg_problems(name, dir, tolerance);
        cells.push(verdict(PLATFORM, &format!("decoder:{name}"), &problems));
    }
    match names
        .first()
        .map(|name| fs::read(dir.join("corpus").join(name)))
    {
        Some(Ok(ogg)) => cells.extend(refusal_cells(&ogg)),
        _ => cells.push(verdict(
            PLATFORM,
            "decoder-refuses",
            &["no Ogg file to damage".to_owned()],
        )),
    }
    cells
}

/// `<platform>/decoders.jsonl`'s pairs against its limits: equal work against the
/// old path where it is that, the injected slowdown seen, and the share of the
/// sound's length decoding takes.
pub(crate) fn speed_cells(
    platform: &str,
    rows: &[Value],
    limits: &std::collections::BTreeMap<String, DecoderLimit>,
) -> Vec<Value> {
    let mut cells = Vec::new();
    for (file, limit) in limits {
        let side = |name: &str| -> Vec<f64> {
            rows.iter()
                .filter(|row| row["file"] == file.as_str())
                .filter_map(|row| row[name].as_f64())
                .collect()
        };
        let (old, new, burn, audio) = (side("old"), side("new"), side("burn"), side("audio_ms"));
        let name = |check: &str| format!("{check}:{file}");
        if new.is_empty() {
            let check = name("decoder-speed");
            cells.push(cell(platform, &check, true, "failed", "no samples"));
            continue;
        }
        let against = ratios(&old, &new);
        let detail = format!(
            "new against old: median ratio {:.3}, spread {:.3}, {} pairs, new {:.2} ms, old {:.2} ms",
            against.median,
            against.spread,
            against.pairs,
            median(&new),
            median(&old),
        );
        let check = name("decoder-speed");
        match limit.ratio {
            Some(ratio) => {
                cells.push(cell(
                    platform,
                    &check,
                    true,
                    judge(&against, ratio),
                    &detail,
                ));
                let burned = ratios(&new, &burn);
                let caught = burned.pairs >= super::MIN_PAIRS && burned.median > ratio.max_ratio;
                let detail = format!("with the slowdown: median ratio {:.3}", burned.median);
                let status = if caught { "passed" } else { "failed" };
                cells.push(cell(
                    platform,
                    &name("decoder-slowdown"),
                    true,
                    status,
                    &detail,
                ));
            }
            // Measured for the record: not the same work, so not a verdict.
            None => cells.push(cell(platform, &check, false, "unsupported", &detail)),
        }
        let share = median(&new) / median(&audio).max(1.0);
        let status = if share <= limit.max_share {
            "passed"
        } else {
            "failed"
        };
        let detail = format!(
            "{share:.4} of the sound's length, limit {}",
            limit.max_share
        );
        cells.push(cell(
            platform,
            &name("decoder-share"),
            true,
            status,
            &detail,
        ));
    }
    cells
}
