//! The mixer stage's cells (ticket 3030): `shuyi-sound`'s mixer against expected
//! samples worked out here from the corpus's numbers, by plain arithmetic that
//! shares no code with it. quad-snd has no offline mix to compare with, so nothing
//! here comes from the old path, and its bugs can't become the answer. What a
//! rendered stream may hold of the decoded sound, and how much of a sound's length
//! a mix takes on each platform, are here too.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use serde::Deserialize;
use serde_json::Value;
use shuyi_sound::mix::stream::{LoopedRegion, feed};
use shuyi_sound::mix::{Config, Full, Mixer, Params, Region, SharedPcm};
use shuyi_sound::{Channels, Error, Limits, Pcm, PcmFormat, SharedSource, Source};

use super::corpus::{self, Signal};
use super::{cell, median, ratios, verdict};

/// The cells' platform: a mix to memory happens nowhere in particular.
const PLATFORM: &str = "mixer";
/// How many frames a render asks for at a time, as an audio callback's might.
const BLOCK_FRAMES: usize = 512;
/// How many frames of a stream a worker holds ready.
const RING_FRAMES: usize = 4096;
/// One rounding step each way: what arithmetic in floats and in fixed point may
/// differ by.
const ROUNDING: i32 = 1;

/// How a platform's mix is held (`budget.ron`).
#[derive(Debug, Clone, Copy, Deserialize)]
pub(crate) struct MixerLimit {
    /// The most of the sound's own length that mixing it may take.
    max_share: f64,
    /// The injected slowdown must show at least this much.
    min_slowdown: f64,
}

/// What a stream may hold decoded (`budget.ron`).
#[derive(Debug, Clone, Copy, Deserialize)]
pub(crate) struct MixerMemory {
    /// The most frames decoded and not yet mixed, however long the stream.
    max_held_frames: u64,
}

fn float(n: usize) -> f64 {
    f64::from(u32::try_from(n).unwrap_or(u32::MAX))
}

fn shared(signal: &Signal) -> Result<SharedPcm, String> {
    let limits = Limits::default();
    let format = PcmFormat::new(signal.rate, u32::from(signal.channels), &limits)
        .map_err(|e| e.to_string())?;
    let pcm = Pcm::new(format, signal.samples.clone(), &limits).map_err(|e| e.to_string())?;
    Ok(SharedPcm::new(pcm))
}

fn signal(name: &str) -> Result<Signal, String> {
    corpus::sources()
        .into_iter()
        .find(|(file, _)| *file == name)
        .map(|(_, signal)| signal)
        .ok_or_else(|| format!("no {name} in the corpus"))
}

fn full(_: Full) -> String {
    "the mixer was full".to_owned()
}

/// A mixer at `rate`, with a voice to spare, in stereo.
fn stereo(rate: u32, voices: usize) -> Mixer {
    Mixer::new(Config::new(rate, Channels::Stereo, voices))
}

/// `frames` frames of `mixer`, a block at a time.
fn rendered(mixer: &mut Mixer, frames: usize) -> Vec<i16> {
    let channels = usize::from(mixer.config().channels.count());
    let mut out = vec![0; frames * channels];
    for block in out.chunks_mut(BLOCK_FRAMES * channels) {
        mixer.render(block);
    }
    out
}

/// One frame of a signal, as (left, right), mono on both sides.
fn frame(signal: &Signal, at: usize) -> (i16, i16) {
    let channels = usize::from(signal.channels);
    let index = at * channels;
    (signal.samples[index], signal.samples[index + channels - 1])
}

fn frames_of(signal: &Signal) -> usize {
    signal.samples.len() / usize::from(signal.channels)
}

/// A float as a sample, clamped.
fn sample(value: f64) -> i16 {
    let clamped = value.round().clamp(-32_768.0, 32_767.0);
    format!("{clamped:.0}").parse().unwrap_or(0)
}

/// What `rendered` has wrong against `expected`, to within `peak` a sample.
fn compared(rendered: &[i16], expected: &[i16], peak: i32) -> Vec<String> {
    let mut problems = Vec::new();
    if rendered.len() != expected.len() {
        let (got, wanted) = (rendered.len(), expected.len());
        problems.push(format!("{got} samples, not {wanted}"));
    }
    let worst = rendered
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

/// The result of a case: what the mixer made and what it should have, or why it
/// couldn't be run.
type Case = Result<(Vec<i16>, Vec<i16>, i32), String>;
/// What runs a case.
type Run = fn() -> Case;

/// Held 4,800 frames at the frame where it stopped, as nothing else plays.
fn pause() -> Case {
    let tone = signal("fmt_48k_s.wav")?;
    let mut mixer = stereo(tone.rate, 1);
    let id = mixer.add_sound(shared(&tone)?).map_err(full)?;
    let voice = mixer.play(id, Params::new()).map_err(full)?;
    let mut out = rendered(&mut mixer, 1000);
    mixer.pause(voice);
    out.extend(rendered(&mut mixer, 4800));
    mixer.resume(voice);
    out.extend(rendered(&mut mixer, frames_of(&tone) - 1000 + 10));
    let mut expected = tone.samples[..2000].to_vec();
    expected.extend(vec![0; 9600]);
    expected.extend_from_slice(&tone.samples[2000..]);
    expected.extend(vec![0; 20]);
    Ok((out, expected, 0))
}

/// `signal`, with its position at `i * step` for each output frame `i`, linearly
/// interpolated, on as many frames as that takes, and `pad` frames of silence.
fn stepped(signal: &Signal, step: f64, pad: usize) -> Vec<i16> {
    let frames = frames_of(signal);
    let mut out = Vec::new();
    let mut i = 0usize;
    loop {
        let position = float(i) * step;
        let whole = position.floor();
        let at = format!("{whole:.0}").parse::<usize>().unwrap_or(usize::MAX);
        if at >= frames {
            break;
        }
        let part = position - whole;
        let (a, b) = (frame(signal, at), frame(signal, (at + 1).min(frames - 1)));
        out.push(sample(
            f64::from(a.0) + (f64::from(b.0) - f64::from(a.0)) * part,
        ));
        out.push(sample(
            f64::from(a.1) + (f64::from(b.1) - f64::from(a.1)) * part,
        ));
        i += 1;
    }
    out.extend(vec![0; pad * 2]);
    out
}

/// A sound played at `rate` (16.16) on a stereo output of `output_rate`.
fn at_rate(name: &str, rate: u32, output_rate: u32) -> Case {
    let source = signal(name)?;
    let step = f64::from(source.rate) / f64::from(output_rate) * f64::from(rate) / 65_536.0;
    let expected = stepped(&source, step, 4);
    let mut mixer = stereo(output_rate, 1);
    let id = mixer.add_sound(shared(&source)?).map_err(full)?;
    mixer
        .play(id, Params::new().with_rate(rate))
        .map_err(full)?;
    let out = rendered(&mut mixer, expected.len() / 2);
    Ok((out, expected, ROUNDING))
}

/// Round again and again with no gap, then stopped part of the way.
fn looped() -> Case {
    let source = signal("loop.wav")?;
    let frames = frames_of(&source);
    let mut mixer = stereo(source.rate, 1);
    let id = mixer.add_sound(shared(&source)?).map_err(full)?;
    mixer.play(id, Params::new().looped()).map_err(full)?;
    let out = rendered(&mut mixer, frames * 2 + frames / 2);
    let mut expected = source.samples.clone();
    expected.extend_from_slice(&source.samples);
    expected.extend_from_slice(&source.samples[..frames / 2 * 2]);
    Ok((out, expected, 0))
}

/// What a stream with the corpus's region should play: its intro, and the region
/// twice.
fn intro_loop() -> Result<(Signal, Region, Vec<i16>), String> {
    let source = signal("intro_loop.wav")?;
    let (start, end) = (frames_of(&source) / 2, frames_of(&source));
    let region = Region::new(start as u64, end as u64).ok_or("no region")?;
    let mut expected = source.samples.clone();
    let looped = source.samples[start * 2..end * 2].to_vec();
    expected.extend_from_slice(&looped);
    Ok((source, region, expected))
}

/// A source that can go back, the mixer finding its way round the region itself.
fn region_in_the_mix() -> Case {
    let (source, region, expected) = intro_loop()?;
    let mut mixer = stereo(source.rate, 1);
    let whole = SharedSource::new(shared(&source)?);
    mixer
        .stream(whole, Params::new().with_region(region))
        .map_err(full)?;
    let out = rendered(&mut mixer, expected.len() / 2);
    Ok((out, expected, 0))
}

/// A worker reading a decoder, going back and pre-rolling ahead of the join.
fn region_from_a_worker() -> Case {
    let (source, region, expected) = intro_loop()?;
    let bytes = corpus::wav(&source);
    let limits = Limits::default();
    let open = || shuyi_sound::wav::WavSource::new(&bytes, &limits);
    let mut decoder = LoopedRegion::new(open().map_err(|e| e.to_string())?, region, open);
    let (mut feeder, ring) = feed(decoder.format(), RING_FRAMES);
    let mut mixer = stereo(source.rate, 1);
    mixer.stream(ring, Params::new()).map_err(full)?;
    let mut out = Vec::new();
    let mut block = vec![0; BLOCK_FRAMES * 2];
    while out.len() < expected.len() {
        feeder.fill_from(&mut decoder).map_err(|e| e.to_string())?;
        mixer.render(&mut block);
        out.extend_from_slice(&block);
    }
    out.truncate(expected.len());
    if mixer.underrun_frames() > 0 {
        return Err(format!("{} frames underran", mixer.underrun_frames()));
    }
    Ok((out, expected, 0))
}

/// Both of a crossfade's streams at once, at gains of their own.
fn two_streams() -> Case {
    let (a, b) = (signal("music_a.wav")?, signal("music_b.wav")?);
    let mut mixer = stereo(a.rate, 2);
    let quarter = Params::new().with_volume(1024);
    let half = Params::new().with_volume(2048);
    mixer
        .stream(SharedSource::new(shared(&a)?), quarter)
        .map_err(full)?;
    mixer
        .stream(SharedSource::new(shared(&b)?), half)
        .map_err(full)?;
    let out = rendered(&mut mixer, frames_of(&a));
    let expected = a
        .samples
        .iter()
        .zip(&b.samples)
        .map(|(a, b)| sample(f64::from(*a) * 0.25 + f64::from(*b) * 0.5))
        .collect();
    Ok((out, expected, ROUNDING + 1))
}

/// A volume and a balance, on a stereo sound.
fn gains() -> Case {
    let tone = signal("tone.wav")?;
    let mut mixer = stereo(tone.rate, 1);
    let id = mixer.add_sound(shared(&tone)?).map_err(full)?;
    let params = Params::new().with_volume(3072).with_balance(4096, 1024);
    mixer.play(id, params).map_err(full)?;
    let out = rendered(&mut mixer, frames_of(&tone));
    let expected = tone
        .samples
        .chunks(2)
        .flat_map(|frame| {
            [
                sample(f64::from(frame[0]) * 0.75),
                sample(f64::from(frame[1]) * 0.75 * 0.25),
            ]
        })
        .collect();
    Ok((out, expected, ROUNDING))
}

/// Louder than a sample holds: it clamps, it never wraps.
fn clamps() -> Case {
    let noise = signal("noise_loop.wav")?;
    let mut mixer = stereo(noise.rate, 2);
    let id = mixer.add_sound(shared(&noise)?).map_err(full)?;
    let loud = Params::new().with_volume(4096 * 8);
    mixer.play(id, loud).map_err(full)?;
    mixer.play(id, loud).map_err(full)?;
    let out = rendered(&mut mixer, frames_of(&noise));
    let expected = noise
        .samples
        .iter()
        .map(|s| sample(f64::from(*s) * 16.0))
        .collect();
    Ok((out, expected, ROUNDING))
}

/// A fade in a straight line, then exact silence.
fn fade() -> Case {
    let tone = signal("tone.wav")?;
    let fade_frames = 4410;
    let mut mixer = stereo(tone.rate, 1);
    let id = mixer.add_sound(shared(&tone)?).map_err(full)?;
    let voice = mixer.play(id, Params::new()).map_err(full)?;
    mixer.fade_to(voice, 0, 100);
    let out = rendered(&mut mixer, frames_of(&tone));
    let expected = tone
        .samples
        .chunks(2)
        .enumerate()
        .flat_map(|(at, frame)| {
            let gain = (1.0 - float(at) / float(fade_frames)).max(0.0);
            [
                sample(f64::from(frame[0]) * gain),
                sample(f64::from(frame[1]) * gain),
            ]
        })
        .collect();
    Ok((out, expected, ROUNDING + 1))
}

/// A stereo sound on a mono output is the mean of its sides.
fn stereo_on_mono() -> Case {
    let tone = signal("tone.wav")?;
    let mut mixer = Mixer::new(Config::new(tone.rate, Channels::Mono, 1));
    let id = mixer.add_sound(shared(&tone)?).map_err(full)?;
    mixer.play(id, Params::new()).map_err(full)?;
    let out = rendered(&mut mixer, frames_of(&tone));
    let expected = tone
        .samples
        .chunks(2)
        .map(|frame| sample(f64::midpoint(f64::from(frame[0]), f64::from(frame[1]))))
        .collect();
    Ok((out, expected, ROUNDING))
}

/// With every voice busy a play is refused, and what plays goes on untouched.
fn capacity() -> Case {
    let tone = signal("tone.wav")?;
    let mut mixer = stereo(tone.rate, 2);
    let id = mixer.add_sound(shared(&tone)?).map_err(full)?;
    let half = Params::new().with_volume(2048);
    mixer.play(id, half).map_err(full)?;
    let mut out = rendered(&mut mixer, 1000);
    mixer.play(id, half).map_err(full)?;
    if mixer.play(id, half) != Err(Full) {
        return Err("a third voice of two was taken".to_owned());
    }
    if mixer.stream(SharedSource::new(shared(&tone)?), half).err() != Some(Full) {
        return Err("a stream was taken with every voice busy".to_owned());
    }
    out.extend(rendered(&mut mixer, 1000));
    let half_of = |s: i16| sample(f64::from(s) * 0.5);
    let mut expected: Vec<i16> = tone.samples[..2000].iter().map(|s| half_of(*s)).collect();
    expected.extend(
        tone.samples[2000..4000]
            .iter()
            .zip(&tone.samples)
            .map(|(a, b)| sample(f64::from(*a) * 0.5 + f64::from(*b) * 0.5)),
    );
    Ok((out, expected, ROUNDING + 1))
}

/// Counts what the worker decoded, to see what is held between that and the mix.
struct Decoded {
    source: SharedSource,
    frames: Arc<AtomicU64>,
}

impl Source for Decoded {
    fn format(&self) -> PcmFormat {
        self.source.format()
    }

    fn total_frames(&self) -> Option<u64> {
        self.source.total_frames()
    }

    fn read(&mut self, out: &mut [i16]) -> Result<usize, Error> {
        let frames = self.source.read(out)?;
        self.frames.fetch_add(frames as u64, Ordering::Relaxed);
        Ok(frames)
    }
}

/// The most frames decoded and not yet mixed over the whole of `signal`, streamed
/// through a worker's ring: or, with `whole`, the mistake this measures against,
/// everything decoded before the mix starts.
fn peak_held(signal: &Signal, whole: bool) -> Result<u64, String> {
    let frames = Arc::new(AtomicU64::new(0));
    let mut source = Decoded {
        source: SharedSource::new(shared(signal)?),
        frames: Arc::clone(&frames),
    };
    let (mut feeder, ring) = feed(source.format(), RING_FRAMES);
    let mut mixer = stereo(signal.rate, 1);
    let voice = mixer.stream(ring, Params::new()).map_err(full)?;
    let (mut played, mut peak) = (0u64, 0u64);
    let mut block = vec![0; BLOCK_FRAMES * 2];
    if whole {
        let mut all = vec![0; signal.samples.len()];
        source.read(&mut all).map_err(|e| e.to_string())?;
        peak = frames.load(Ordering::Relaxed);
        feeder.close();
    }
    while mixer.is_playing(voice) {
        feeder.fill_from(&mut source).map_err(|e| e.to_string())?;
        peak = peak.max(frames.load(Ordering::Relaxed).saturating_sub(played));
        mixer.render(&mut block);
        played += BLOCK_FRAMES as u64;
    }
    Ok(peak)
}

/// What the memory limit says of a short and a long stream, and that it would
/// catch a stream decoded whole.
fn memory_cells(limit: MixerMemory) -> Vec<Value> {
    let mut cells = Vec::new();
    let check =
        |name: &str, whole: bool| -> Result<u64, String> { peak_held(&signal(name)?, whole) };
    let (short, long) = (check("tone.wav", false), check("long.wav", false));
    let mut problems = Vec::new();
    match (short, long) {
        (Ok(short), Ok(long)) => {
            for (name, held) in [("short", short), ("long", long)] {
                if held > limit.max_held_frames {
                    let max = limit.max_held_frames;
                    problems.push(format!("the {name} stream held {held} frames, over {max}"));
                }
            }
            if long > short {
                problems.push(format!("a longer stream held more: {long} against {short}"));
            }
        }
        (short, long) => problems.extend(short.err().into_iter().chain(long.err())),
    }
    cells.push(verdict(PLATFORM, "mixer:memory", &problems));
    let caught = check("long.wav", true).is_ok_and(|held| held > limit.max_held_frames);
    let problems = if caught {
        Vec::new()
    } else {
        vec!["a stream decoded whole passed".to_owned()]
    };
    cells.push(verdict(PLATFORM, "mixer-fault:decoded-whole", &problems));
    cells
}

/// Ways a mix goes wrong, each of which [`compared`] must catch.
fn fault_cells() -> Vec<Value> {
    let Ok((good, expected, peak)) = pause() else {
        let problem = "no recording to break".to_owned();
        return vec![verdict(PLATFORM, "mixer-fault", &[problem])];
    };
    let mut faults: Vec<(&str, Vec<i16>)> = Vec::new();
    faults.push(("silent", vec![0; good.len()]));
    faults.push((
        "swapped",
        good.chunks(2).flat_map(|f| [f[1], f[0]]).collect(),
    ));
    faults.push(("frame-lost", good[2..].to_vec()));
    let mut late = vec![0, 0];
    late.extend_from_slice(&good[..good.len() - 2]);
    faults.push(("frame-late", late));
    faults.push(("quiet", good.iter().map(|s| s / 2).collect()));
    let mut wrapped = good.clone();
    if let Some(sample) = wrapped.iter_mut().max_by_key(|s| **s) {
        *sample = sample.wrapping_add(i16::MAX);
    }
    faults.push(("wrapped", wrapped));
    faults
        .into_iter()
        .map(|(name, broken)| {
            let caught = !compared(&broken, &expected, peak).is_empty();
            let problems = if caught {
                Vec::new()
            } else {
                vec!["it wasn't caught".to_owned()]
            };
            verdict(PLATFORM, &format!("mixer-fault:{name}"), &problems)
        })
        .collect()
}

/// The mixer stage's correctness: every case, the memory, and the faults.
pub(crate) fn correctness_cells(memory: MixerMemory) -> Vec<Value> {
    let cases: [(&str, Run); 14] = [
        ("pause", pause),
        ("rate-double", || at_rate("fmt_48k_s.wav", 131_072, 48_000)),
        ("rate-half", || at_rate("tone.wav", 32_768, 44_100)),
        ("resample-mono-up", || {
            at_rate("fmt_22k_m.wav", 65_536, 48_000)
        }),
        ("resample-down", || at_rate("fmt_48k_s.wav", 65_536, 44_100)),
        ("loop", looped),
        ("region-in-the-mix", region_in_the_mix),
        ("region-from-a-worker", region_from_a_worker),
        ("two-streams", two_streams),
        ("gains", gains),
        ("clamp", clamps),
        ("fade", fade),
        ("stereo-on-mono", stereo_on_mono),
        ("capacity", capacity),
    ];
    let mut cells: Vec<Value> = cases
        .iter()
        .map(|(name, case)| {
            let problems = match case() {
                Ok((rendered, expected, peak)) => compared(&rendered, &expected, peak),
                Err(why) => vec![why],
            };
            verdict(PLATFORM, &format!("mixer:{name}"), &problems)
        })
        .collect();
    cells.extend(memory_cells(memory));
    cells.extend(fault_cells());
    cells
}

/// `<platform>/mixer.jsonl`'s renders against its limit: the share of the sound's
/// length a mix takes, and the injected slowdown seen.
pub(crate) fn speed_cells(platform: &str, rows: &[Value], limit: MixerLimit) -> Vec<Value> {
    let side =
        |name: &str| -> Vec<f64> { rows.iter().filter_map(|row| row[name].as_f64()).collect() };
    let (render, burn, audio) = (side("render"), side("burn"), side("audio_ms"));
    if render.is_empty() {
        return vec![cell(platform, "mixer-share", true, "failed", "no samples")];
    }
    let share = median(&render) / median(&audio).max(1.0);
    let status = if share <= limit.max_share {
        "passed"
    } else {
        "failed"
    };
    let detail = format!(
        "{share:.4} of the sound's length, {:.2} ms, limit {}",
        median(&render),
        limit.max_share
    );
    let burned = ratios(&render, &burn);
    let caught = burned.pairs >= super::MIN_PAIRS && burned.median >= limit.min_slowdown;
    let burned_detail = format!("with the slowdown: median ratio {:.3}", burned.median);
    vec![
        cell(platform, "mixer-share", true, status, &detail),
        cell(
            platform,
            "mixer-slowdown",
            true,
            if caught { "passed" } else { "failed" },
            &burned_detail,
        ),
    ]
}

/// The budget's limits by platform, as `Budget` holds them.
pub(crate) type MixerLimits = BTreeMap<String, MixerLimit>;

#[cfg(test)]
mod tests {
    use super::*;

    fn memory() -> MixerMemory {
        MixerMemory {
            max_held_frames: 4096 + 128,
        }
    }

    #[test]
    fn the_mixer_passes_every_case_and_every_fault_is_caught() {
        let cells = correctness_cells(memory());
        let failed: Vec<&Value> = cells
            .iter()
            .filter(|cell| cell["status"] != "passed")
            .collect();
        assert!(failed.is_empty(), "{failed:#?}");
        assert!(cells.len() > 20, "{}", cells.len());
    }

    #[test]
    fn a_limit_under_what_a_stream_holds_fails() {
        let cells = memory_cells(MixerMemory {
            max_held_frames: 10,
        });
        assert_eq!(cells[0]["status"], "failed");
    }

    fn rows(render: f64, burn: f64, count: usize) -> Vec<Value> {
        (0..count)
            .map(|_| serde_json::json!({"render": render, "burn": burn, "audio_ms": 10_000.0}))
            .collect()
    }

    fn statuses(cells: &[Value]) -> Vec<&str> {
        cells
            .iter()
            .map(|cell| cell["status"].as_str().unwrap_or(""))
            .collect()
    }

    #[test]
    fn speed_is_judged_by_the_sounds_length_and_the_slowdown_must_show() {
        let limit = MixerLimit {
            max_share: 0.1,
            min_slowdown: 1.5,
        };
        let judged = |render, burn, count| speed_cells("p", &rows(render, burn, count), limit);
        assert_eq!(statuses(&judged(100.0, 200.0, 9)), ["passed", "passed"]);
        assert_eq!(statuses(&judged(2000.0, 4000.0, 9)), ["failed", "passed"]);
        assert_eq!(statuses(&judged(100.0, 110.0, 9)), ["passed", "failed"]);
        assert_eq!(
            statuses(&judged(100.0, 200.0, 3)),
            ["passed", "failed"],
            "too few pairs to say"
        );
        assert_eq!(statuses(&judged(100.0, 200.0, 0)), ["failed"]);
    }
}
