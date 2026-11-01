//! Holds one recording to a scenario's expected signals.
//!
//! A recording is 16-bit stereo at [`RATE`], as the sink gives it: the only
//! conversion is to a fraction of full scale. One marker places everything;
//! there is no per-segment stretching and no gain normalization, so a voice
//! that is late, quiet, swapped or missing shows as what it is.

use serde::Deserialize;

use super::corpus::{MARKER_HEAD_MS, RATE, marker};

/// What the marker must match within, around the first sound, in frames.
const SEARCH_FRAMES: usize = 2048;
/// A sample this far from zero is sound, for finding where sound starts.
const ONSET: i32 = 64;
/// The window of the repeat comparison, in milliseconds.
const ENVELOPE_MS: usize = 50;
/// Below this a level counts as silence in the repeat comparison.
const ENVELOPE_FLOOR: f64 = 0.001;

/// The fixed tolerances (`correctness` in `tools/sound-parity/budget.ron`).
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct Tolerance {
    pub(crate) timing_ms: u32,
    pub(crate) level_db: f64,
    pub(crate) silent_peak: i32,
    pub(crate) silent_tone: f64,
    pub(crate) marker_correlation: f64,
    pub(crate) exact_peak: i32,
    pub(crate) decoded_peak: i32,
    pub(crate) repeat_db: f64,
}

/// Which channels a check reads.
#[derive(Debug, Clone, Copy, Deserialize)]
pub(crate) enum Ch {
    Left,
    Right,
    Both,
}

/// What a waveform is compared with, and so how close it must be.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) enum Class {
    /// The file's own samples: a WAV has one right answer.
    Exact,
    /// What a separate decoder makes of the file: decoders may round apart.
    Decoded,
}

/// One expected signal or fact.
#[derive(Debug, Clone, Deserialize)]
pub(crate) enum Expect {
    NoSignal,
    Rise {
        at: u32,
    },
    Fall {
        at: u32,
    },
    Tone {
        from: u32,
        to: u32,
        ch: Ch,
        hz: f64,
        amp: f64,
    },
    Level {
        from: u32,
        to: u32,
        ch: Ch,
        rms: f64,
    },
    Silent {
        from: u32,
        to: u32,
    },
    Waveform {
        file: String,
        repeats: usize,
        class: Class,
        /// Frames: a backend's mixing block, inside which the end of a pass
        /// may be wrong. 0, the default, holds every frame: what gapless is.
        #[serde(default)]
        smear: usize,
    },
    State {
        at: u32,
        cue: String,
        secs: f64,
    },
    Warned(String),
}

/// The probe's log of a run.
#[derive(Debug, Clone, Default)]
pub(crate) struct Log {
    /// Each step's time and the player's clock: `(at, cue, secs)`.
    pub(crate) steps: Vec<(u32, String, f64)>,
    pub(crate) warnings: Vec<String>,
    pub(crate) failed: Option<String>,
    pub(crate) done: bool,
}

/// A recording's two channels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Recording {
    pub(crate) left: Vec<i16>,
    pub(crate) right: Vec<i16>,
}

impl Recording {
    /// From interleaved samples.
    pub(crate) fn new(samples: &[i16]) -> Self {
        Self {
            left: samples.iter().step_by(2).copied().collect(),
            right: samples.iter().skip(1).step_by(2).copied().collect(),
        }
    }

    fn channels(&self, ch: Ch) -> Vec<&[i16]> {
        match ch {
            Ch::Left => vec![&self.left],
            Ch::Right => vec![&self.right],
            Ch::Both => vec![&self.left, &self.right],
        }
    }

    /// Whether the recording reaches `frame`.
    fn reaches(&self, frame: usize) -> bool {
        self.left.len() >= frame && self.right.len() >= frame
    }
}

/// A range of milliseconds after `origin`, in frames.
fn range(origin: usize, from: u32, to: u32) -> (usize, usize) {
    let end = origin + frames(to);
    ((origin + frames(from)).min(end), end)
}

fn frames(ms: u32) -> usize {
    ms as usize * RATE as usize / 1000
}

#[allow(clippy::cast_precision_loss)]
fn unit(frames: usize) -> f64 {
    frames as f64
}

fn db(ratio: f64) -> f64 {
    20.0 * ratio.log10()
}

/// How alike two runs of samples are, -1 to 1, whatever their levels.
fn correlation(a: &[i16], b: &[i16]) -> f64 {
    let (mut ab, mut aa, mut bb) = (0.0, 0.0, 0.0);
    for (x, y) in a.iter().zip(b) {
        let (x, y) = (f64::from(*x), f64::from(*y));
        ab += x * y;
        aa += x * x;
        bb += y * y;
    }
    if aa == 0.0 || bb == 0.0 {
        0.0
    } else {
        ab / (aa * bb).sqrt()
    }
}

/// Where the marker's first sample is in `recording`, or why it isn't there.
pub(crate) fn align(recording: &Recording, tolerance: &Tolerance) -> Result<usize, String> {
    let (left, right) = marker(MARKER_HEAD_MS);
    let loud = |s: &i16| i32::from(*s).abs() > ONSET;
    let first = [&recording.left, &recording.right]
        .iter()
        .filter_map(|channel| channel.iter().position(loud))
        .min()
        .ok_or("silent recording: no sound at all, so no marker")?;
    let start = first.saturating_sub(SEARCH_FRAMES);
    let mut best = (start, f64::MIN, f64::MIN);
    for offset in start..=first + SEARCH_FRAMES {
        if !recording.reaches(offset + left.len()) {
            break;
        }
        let l = correlation(&recording.left[offset..offset + left.len()], &left);
        let r = correlation(&recording.right[offset..offset + right.len()], &right);
        if l + r > best.1 + best.2 {
            best = (offset, l, r);
        }
    }
    if best.1.min(best.2) < tolerance.marker_correlation {
        return Err(format!(
            "missing marker: its best match has correlation {:.3} on the left and {:.3} on the \
             right, under {}",
            best.1, best.2, tolerance.marker_correlation
        ));
    }
    Ok(best.0)
}

/// The amplitude of `hz` in `samples`, as a fraction of full scale.
fn tone_amplitude(samples: &[i16], hz: f64) -> f64 {
    let step = core::f64::consts::TAU * hz / f64::from(RATE);
    let (mut re, mut im, mut angle) = (0.0, 0.0, 0.0f64);
    for sample in samples {
        let value = f64::from(*sample) / 32_768.0;
        re += value * angle.cos();
        im += value * angle.sin();
        angle += step;
    }
    2.0 * (re * re + im * im).sqrt() / unit(samples.len().max(1))
}

/// The root mean square of `samples`, as a fraction of full scale.
fn rms(samples: &[i16]) -> f64 {
    let sum: f64 = samples
        .iter()
        .map(|s| (f64::from(*s) / 32_768.0).powi(2))
        .sum();
    (sum / unit(samples.len().max(1))).sqrt()
}

fn peak(samples: &[i16]) -> i32 {
    samples
        .iter()
        .map(|s| i32::from(*s).abs())
        .max()
        .unwrap_or(0)
}

/// Why `recording` isn't what `expect`s say, or nothing if it is. `oracle`
/// gives a waveform's expected samples, interleaved.
pub(crate) fn check(
    expect: &[Expect],
    end_ms: u32,
    recording: &Recording,
    log: &Log,
    oracle: &dyn Fn(&str) -> Option<Vec<i16>>,
    tolerance: &Tolerance,
) -> Vec<String> {
    let mut problems = Vec::new();
    if let Some(failed) = &log.failed {
        problems.push(format!("the probe failed: {failed}"));
    }
    if !log.done {
        problems.push("the probe never said it was done".to_owned());
    }
    let clipped = [&recording.left, &recording.right]
        .iter()
        .flat_map(|channel| channel.iter())
        .filter(|s| i32::from(**s).abs() >= 32_767)
        .count();
    if clipped > 0 {
        problems.push(format!("clipping: {clipped} samples at full scale"));
    }
    if expect.iter().any(|e| matches!(e, Expect::NoSignal)) {
        let loudest = peak(&recording.left).max(peak(&recording.right));
        if loudest > tolerance.silent_peak {
            problems.push(format!("expected no signal, and its peak is {loudest}"));
        }
        return problems;
    }
    let origin = match align(recording, tolerance) {
        Ok(origin) => origin,
        Err(e) => {
            problems.push(e);
            return problems;
        }
    };
    if !recording.reaches(origin + frames(end_ms)) {
        problems.push(format!(
            "too short: {} frames, and the scenario ends at {end_ms} ms",
            recording.left.len()
        ));
        return problems;
    }
    for one in expect {
        problems.extend(check_one(one, recording, origin, log, oracle, tolerance));
    }
    problems
}

fn check_one(
    expect: &Expect,
    recording: &Recording,
    origin: usize,
    log: &Log,
    oracle: &dyn Fn(&str) -> Option<Vec<i16>>,
    tolerance: &Tolerance,
) -> Option<String> {
    let timing = tolerance.timing_ms;
    let loud = |s: &i16| i32::from(*s).abs() > tolerance.silent_peak;
    match expect {
        Expect::NoSignal => None,
        Expect::Rise { at } => {
            let (from, to) = range(origin, at.saturating_sub(timing), at + timing);
            let first = recording.left[from..to].iter().position(loud);
            (first.is_none() || first == Some(0)).then(|| {
                format!("no sound starts within {timing} ms of {at} ms (a wrong onset or duration)")
            })
        }
        Expect::Fall { at } => {
            let (from, to) = range(origin, at.saturating_sub(timing), at + timing);
            let window = &recording.left[from..to];
            let last = window.iter().rposition(loud);
            (last.is_none() || last == Some(window.len() - 1)).then(|| {
                format!("the sound doesn't end within {timing} ms of {at} ms (a wrong duration)")
            })
        }
        Expect::Tone {
            from,
            to,
            ch,
            hz,
            amp,
        } => {
            let (start, end) = range(origin, *from, *to);
            recording.channels(*ch).iter().find_map(|channel| {
                let got = tone_amplitude(&channel[start..end], *hz);
                let wrong = if *amp == 0.0 {
                    got > tolerance.silent_tone
                } else {
                    got <= 0.0 || db(got / amp).abs() > tolerance.level_db
                };
                wrong.then(|| {
                    format!("{hz} Hz over {from}..{to} ms is {got:.4}, expected {amp} ({ch:?})")
                })
            })
        }
        Expect::Level {
            from,
            to,
            ch,
            rms: want,
        } => {
            let (start, end) = range(origin, *from, *to);
            recording.channels(*ch).iter().find_map(|channel| {
                let got = rms(&channel[start..end]);
                (got <= 0.0 || db(got / want).abs() > tolerance.level_db).then(|| {
                    format!("the level over {from}..{to} ms is {got:.4}, expected {want} ({ch:?})")
                })
            })
        }
        Expect::Silent { from, to } => {
            let (start, end) = range(origin, *from, *to);
            let loudest = peak(&recording.left[start..end]).max(peak(&recording.right[start..end]));
            (loudest > tolerance.silent_peak)
                .then(|| format!("not silent over {from}..{to} ms: its peak is {loudest}"))
        }
        Expect::Waveform {
            file,
            repeats,
            class,
            smear,
        } => {
            let limit = match class {
                Class::Exact => tolerance.exact_peak,
                Class::Decoded => tolerance.decoded_peak,
            };
            oracle(file).map_or_else(
                || Some(format!("no expected samples for {file}")),
                |expected| waveform(recording, origin, file, &expected, *repeats, *smear, limit),
            )
        }
        Expect::State { at, cue, secs } => {
            let step = log.steps.iter().rfind(|step| step.0 == *at);
            let limit = f64::from(timing) / 1000.0;
            match step {
                None => Some(format!("no state was logged at {at} ms")),
                Some((_, got, _)) if got != cue => {
                    Some(format!("at {at} ms the music is {got:?}, expected {cue:?}"))
                }
                Some((_, _, got)) => ((got - secs).abs() > limit)
                    .then(|| format!("at {at} ms the music clock says {got} s, expected {secs} s")),
            }
        }
        Expect::Warned(text) => (!log.warnings.iter().any(|w| w.contains(text.as_str())))
            .then(|| format!("no warning says {text:?}")),
    }
}

/// `repeats` passes of `file`, sample by sample from the marker.
fn waveform(
    recording: &Recording,
    origin: usize,
    file: &str,
    expected: &[i16],
    repeats: usize,
    smear: usize,
    limit: i32,
) -> Option<String> {
    let per_pass = expected.len() / 2;
    let total = per_pass * repeats;
    if !recording.reaches(origin + total) {
        return Some(format!("too short for {repeats} passes of {file}"));
    }
    let smeared = |frame: usize| {
        smear > 0
            && (1..=repeats).any(|pass| {
                let block = pass * per_pass / smear * smear;
                (block..block + smear).contains(&frame)
            })
    };
    for frame in (0..total).filter(|frame| !smeared(*frame)) {
        let at = frame % per_pass;
        let left = i32::from(recording.left[origin + frame]) - i32::from(expected[at * 2]);
        let right = i32::from(recording.right[origin + frame]) - i32::from(expected[at * 2 + 1]);
        if left.abs().max(right.abs()) > limit {
            return Some(format!(
                "{file} differs by {} at frame {frame} of {total}, over {limit} (pass {})",
                left.abs().max(right.abs()),
                frame / per_pass + 1,
            ));
        }
    }
    None
}

/// Whether two runs of a scenario are alike: each one's level over short
/// windows, away from the moments a command lands (`edges`, milliseconds).
pub(crate) fn repeat_difference(
    a: &Recording,
    b: &Recording,
    edges: &[u32],
    end_ms: u32,
    tolerance: &Tolerance,
) -> Result<f64, String> {
    let (from_a, from_b) = (align(a, tolerance)?, align(b, tolerance)?);
    let window = frames(u32::try_from(ENVELOPE_MS).unwrap_or(0));
    let guard = tolerance.timing_ms as usize + ENVELOPE_MS;
    let mut worst = 0.0f64;
    for ms in (0..end_ms as usize).step_by(ENVELOPE_MS) {
        let near = edges.iter().any(|e| (*e as usize).abs_diff(ms) < guard);
        let at = ms * RATE as usize / 1000;
        if near || !a.reaches(from_a + at + window) || !b.reaches(from_b + at + window) {
            continue;
        }
        let level = |r: &Recording, from: usize| {
            rms(&r.left[from + at..from + at + window]).max(ENVELOPE_FLOOR)
        };
        worst = worst.max(db(level(a, from_a) / level(b, from_b)).abs());
    }
    Ok(worst)
}

/// A fault put into a good recording: the gate proves each is caught.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Fault {
    Silence,
    SwapChannels,
    /// Played `ratio` times too fast.
    WrongRate(f64),
    /// Nothing over this range of milliseconds after the marker.
    Drop(u32, u32),
    Clip,
    MissingMarker,
}

/// `recording` with `fault` in it, as a broken backend would have sounded.
pub(crate) fn inject(recording: &Recording, fault: Fault, tolerance: &Tolerance) -> Recording {
    let mut out = recording.clone();
    let origin = align(recording, tolerance).unwrap_or(0);
    match fault {
        Fault::Silence => {
            out.left.fill(0);
            out.right.fill(0);
        }
        Fault::SwapChannels => core::mem::swap(&mut out.left, &mut out.right),
        Fault::WrongRate(ratio) => {
            let resample = |samples: &[i16]| -> Vec<i16> {
                let count = unit(samples.len()) / ratio;
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                (0..count as usize)
                    .map(|i| samples[(unit(i) * ratio) as usize])
                    .collect()
            };
            out.left = resample(&recording.left);
            out.right = resample(&recording.right);
        }
        Fault::Drop(from, to) => {
            let (start, end) = range(origin, from, to);
            let end = end.min(out.left.len());
            out.left[start..end].fill(0);
            out.right[start..end].fill(0);
        }
        Fault::Clip => {
            for sample in out.left.iter_mut().chain(&mut out.right) {
                *sample = sample.saturating_mul(8);
            }
        }
        Fault::MissingMarker => {
            let end = (origin + frames(MARKER_HEAD_MS)).min(out.left.len());
            out.left[origin..end].fill(0);
            out.right[origin..end].fill(0);
        }
    }
    out
}
