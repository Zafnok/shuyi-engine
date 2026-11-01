//! The gates' test signals: every WAV made here, whole numbers from a few
//! seeds, so no game file and no third party is in them; the Ogg files are
//! these WAVs through `oggenc`, committed with how they were made
//! (`tools/sound-parity/corpus.ron`).
//!
//! What a scenario expects comes from the same numbers, written apart from
//! any backend or decoder: an oracle that shares no code with what it checks.

use std::fmt::Write as _;

use sha2::{Digest as _, Sha256};

/// The rate recordings are at, and the corpus's main rate.
pub(crate) const RATE: u32 = 44_100;
/// How much of the marker the alignment matches: every file that starts with
/// the marker has at least this much.
pub(crate) const MARKER_HEAD_MS: u32 = 100;
const HALF: i32 = 16_384;
const QUARTER: i32 = 8_192;
const LEFT_SEED: u32 = 0x2573_0001;
const RIGHT_SEED: u32 = 0x2573_0002;

/// A signal: interleaved samples at a rate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Signal {
    pub(crate) rate: u32,
    pub(crate) channels: u16,
    pub(crate) samples: Vec<i16>,
}

/// Noise in `-amp..amp` from a xorshift, the same on every machine.
fn noise(seed: u32, frames: usize, amp: i32) -> Vec<i16> {
    let mut state = seed;
    (0..frames)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            // The top 16 bits as a signed number: within `i16` after scaling.
            let unit = i32::from(state.to_be_bytes()[0]) << 8 | i32::from(state.to_be_bytes()[1]);
            narrow((unit - 32_768) * amp / 32_768)
        })
        .collect()
}

/// A sine of `hz` and amplitude `amp`, from a rotation in whole numbers, so
/// the samples don't depend on a platform's `sin`: only the step does, once,
/// rounded far above the last bit that can differ.
fn tone(hz: u32, rate: u32, frames: usize, amp: i32) -> Vec<i16> {
    const ONE: i64 = 1 << 30;
    let turn = core::f64::consts::PI * f64::from(hz) / f64::from(rate);
    // Under 2 in 2^30, far inside `i64`.
    #[allow(clippy::cast_possible_truncation)]
    let step = (2.0 * turn.sin() * 1_073_741_824.0).round() as i64;
    let (mut x, mut y) = (i64::from(amp) * ONE, 0i64);
    (0..frames)
        .map(|_| {
            let out = y / ONE;
            x -= (step * (y >> 15)) >> 15;
            y += (step * (x >> 15)) >> 15;
            narrow(i32::try_from(out).unwrap_or(0))
        })
        .collect()
}

/// `value` within `i16`.
fn narrow(value: i32) -> i16 {
    i16::try_from(value.clamp(-32_768, 32_767)).unwrap_or(0)
}

/// The samples of `left` and `right` in turn.
fn stereo(left: &[i16], right: &[i16]) -> Vec<i16> {
    left.iter().zip(right).flat_map(|(l, r)| [*l, *r]).collect()
}

const fn frames_of(ms: u32, rate: u32) -> usize {
    (ms as usize) * (rate as usize) / 1000
}

/// The marker's two channels: different noise, so a swap can't match.
pub(crate) fn marker(ms: u32) -> (Vec<i16>, Vec<i16>) {
    let frames = frames_of(ms, RATE);
    (
        noise(LEFT_SEED, frames, HALF),
        noise(RIGHT_SEED, frames, HALF),
    )
}

/// A stereo signal at the main rate: `left_hz` and `right_hz`, half scale.
fn tones(ms: u32, left_hz: u32, right_hz: u32) -> Signal {
    let frames = frames_of(ms, RATE);
    Signal {
        rate: RATE,
        channels: 2,
        samples: stereo(
            &tone(left_hz, RATE, frames, HALF),
            &tone(right_hz, RATE, frames, HALF),
        ),
    }
}

/// `tone` after the marker, so one alignment places the whole file.
fn marked(marker_ms: u32, mut after: Signal) -> Signal {
    let (left, right) = marker(marker_ms);
    let mut samples = stereo(&left, &right);
    samples.append(&mut after.samples);
    after.samples = samples;
    after
}

/// A rising ramp on the left and a falling one on the right, half scale.
fn ramps(ms: u32) -> Signal {
    let frames = i32::try_from(frames_of(ms, RATE)).unwrap_or(1);
    let samples = (0..frames)
        .flat_map(|i| {
            let up = narrow(2 * HALF * i / frames - HALF);
            [up, narrow(-i32::from(up))]
        })
        .collect();
    Signal {
        rate: RATE,
        channels: 2,
        samples,
    }
}

/// Every source file by name. The `.ogg` files are encoded from the `.wav`
/// of the same name.
pub(crate) fn sources() -> Vec<(&'static str, Signal)> {
    let frames_44 = frames_of(1000, RATE);
    let noise_loop = Signal {
        rate: RATE,
        channels: 2,
        samples: stereo(
            &noise(0x2573_0003, frames_44, QUARTER),
            &noise(0x2573_0004, frames_44, QUARTER),
        ),
    };
    let long_tone = tones(10_000, 1000, 500);
    let long_noise = stereo(
        &noise(0x2573_0005, frames_of(10_000, RATE), 1024),
        &noise(0x2573_0006, frames_of(10_000, RATE), 1024),
    );
    let long = Signal {
        samples: long_tone
            .samples
            .iter()
            .zip(&long_noise)
            .map(|(a, b)| narrow(i32::from(*a) + i32::from(*b)))
            .collect(),
        ..long_tone
    };
    let mut intro_loop = tones(1000, 440, 440);
    intro_loop.samples.extend(tones(1000, 880, 880).samples);
    let (left, right) = marker(300);
    vec![
        (
            "marker.wav",
            Signal {
                rate: RATE,
                channels: 2,
                samples: stereo(&left, &right),
            },
        ),
        (
            "silence.wav",
            Signal {
                rate: RATE,
                channels: 2,
                samples: vec![0; frames_44 * 2],
            },
        ),
        ("tone.wav", tones(1000, 440, 660)),
        ("noise_loop.wav", noise_loop),
        ("loop.wav", marked(MARKER_HEAD_MS, ramps(400))),
        ("fmt_44k_s.wav", marked(300, tones(1000, 1000, 500))),
        (
            "fmt_22k_m.wav",
            Signal {
                rate: 22_050,
                channels: 1,
                samples: tone(1000, 22_050, frames_of(1000, 22_050), HALF),
            },
        ),
        (
            "fmt_48k_s.wav",
            Signal {
                rate: 48_000,
                channels: 2,
                samples: stereo(
                    &tone(1000, 48_000, frames_of(1000, 48_000), HALF),
                    &tone(500, 48_000, frames_of(1000, 48_000), HALF),
                ),
            },
        ),
        ("music_a.wav", tones(6000, 440, 440)),
        ("music_b.wav", tones(6000, 1000, 1000)),
        ("intro_loop.wav", intro_loop),
        ("long.wav", long),
    ]
}

/// `signal` as a canonical 16-bit PCM WAV file, written from the format's
/// specification: its own reader in the tests, no library in between.
pub(crate) fn wav(signal: &Signal) -> Vec<u8> {
    let data_len = u32::try_from(signal.samples.len() * 2).unwrap_or(u32::MAX);
    let block = signal.channels * 2;
    let mut out = Vec::with_capacity(44 + signal.samples.len() * 2);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&signal.channels.to_le_bytes());
    out.extend_from_slice(&signal.rate.to_le_bytes());
    out.extend_from_slice(&(signal.rate * u32::from(block)).to_le_bytes());
    out.extend_from_slice(&block.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for sample in &signal.samples {
        out.extend_from_slice(&sample.to_le_bytes());
    }
    out
}

/// The SHA-256 of `bytes`, in hex.
pub(crate) fn sha256(bytes: &[u8]) -> String {
    let mut out = String::new();
    for byte in Sha256::digest(bytes) {
        // Writing to a `String` doesn't fail.
        let _ = write!(out, "{byte:02x}");
    }
    out
}

/// A little-endian PCM file's samples.
pub(crate) fn samples(bytes: &[u8]) -> Vec<i16> {
    let (pairs, _) = bytes.as_chunks::<2>();
    pairs.iter().map(|pair| i16::from_le_bytes(*pair)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source(name: &str) -> Signal {
        sources()
            .into_iter()
            .find(|(n, _)| *n == name)
            .map_or_else(|| panic!("no source {name}"), |(_, signal)| signal)
    }

    #[test]
    fn a_wav_says_what_it_holds() {
        let signal = source("fmt_22k_m.wav");
        let bytes = wav(&signal);
        assert_eq!(&bytes[..4], b"RIFF");
        assert_eq!(&bytes[8..16], b"WAVEfmt ");
        assert_eq!(u16::from_le_bytes([bytes[22], bytes[23]]), 1);
        let rate = u32::from_le_bytes([bytes[24], bytes[25], bytes[26], bytes[27]]);
        assert_eq!(rate, 22_050);
        assert_eq!(samples(&bytes[44..]), signal.samples);
    }

    #[test]
    fn a_tone_has_its_amplitude_and_period() {
        let signal = source("tone.wav");
        let left: Vec<i16> = signal.samples.iter().step_by(2).copied().collect();
        let peak = left.iter().map(|s| i32::from(*s).abs()).max().unwrap_or(0);
        assert!((peak - HALF).abs() < 16, "{peak}");
        // 440 Hz: as many rising zero crossings as cycles, in one second.
        let rising = left.windows(2).filter(|w| w[0] < 0 && w[1] >= 0).count();
        assert!((439..=440).contains(&rising), "{rising}");
    }

    #[test]
    fn the_marker_starts_each_file_that_says_it_does() {
        let (left, right) = marker(MARKER_HEAD_MS);
        let head = stereo(&left, &right);
        for name in ["marker.wav", "loop.wav", "fmt_44k_s.wav"] {
            assert_eq!(&source(name).samples[..head.len()], head, "{name}");
        }
    }

    #[test]
    fn the_channels_of_the_marker_differ() {
        let (left, right) = marker(MARKER_HEAD_MS);
        assert_ne!(left, right);
    }

    #[test]
    fn the_sources_are_the_same_every_time() {
        let once: Vec<String> = sources().iter().map(|(_, s)| sha256(&wav(s))).collect();
        let again: Vec<String> = sources().iter().map(|(_, s)| sha256(&wav(s))).collect();
        assert_eq!(once, again);
    }

    #[test]
    fn sha256_is_the_standard_one() {
        assert_eq!(
            sha256(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
