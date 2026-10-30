//! The game's sounds, made by code rather than shipped as files: a short
//! blip for each of the engine's menu and cursor sounds, a sine at its own
//! pitch that fades out, as a 16-bit mono WAV at the engine's sample rate.

#[cfg(feature = "std")]
use std::borrow::Cow;

use crate::prelude::*;

use shuyi::assets::audio::{AudioManifest, CreditRef, SoundCue};
#[cfg(feature = "std")]
use shuyi::assets::audio::{Bytes, Files, SAMPLE_RATE};
use shuyi::ui::audio::AudioConfig;

/// Each sound file: its name, pitch in hertz and length in milliseconds.
pub const BLIPS: [(&str, f32, u32); 4] = [
    ("move.wav", 660.0, 50),
    ("select.wav", 880.0, 80),
    ("cancel.wav", 440.0, 80),
    ("cursor.wav", 990.0, 30),
];

/// The audio manifest: the engine's four sound cues ([`AudioConfig`]),
/// each a blip of [`BLIPS`]; no music.
pub fn manifest() -> AudioManifest {
    let config = AudioConfig::DEFAULT;
    let cues = [
        (config.move_cue, "move.wav"),
        (config.select_cue, "select.wav"),
        (config.cancel_cue, "cancel.wav"),
        (config.cursor_cue, "cursor.wav"),
    ];
    let mut audio = AudioManifest::default();
    for (cue, file) in cues {
        let sound = SoundCue::new(vec![file.to_owned()], 60, CreditRef::Own);
        audio.sounds.insert(cue.to_owned(), sound);
    }
    audio
}

/// The sound files for the platform: the blips, made when asked for; no
/// music folder.
#[cfg(feature = "std")]
pub fn files() -> Files<'static> {
    Files::new(&file, &shown, &left_out, "music")
}

/// How errors name a sound file: as the manifest does.
#[cfg(feature = "std")]
fn shown(name: &str) -> String {
    format!("(made by code) {name}")
}

/// Whether a build leaves the sound file out on purpose: none does.
#[cfg(feature = "std")]
fn left_out(_: &str) -> bool {
    false
}

/// The file `name` of [`BLIPS`], or `None`.
#[cfg(feature = "std")]
pub fn file(name: &str) -> Bytes {
    let &(_, pitch, ms) = BLIPS.iter().find(|(file, ..)| *file == name)?;
    Some(Cow::Owned(wav(pitch, ms)))
}

/// A sine of `pitch` hertz, `ms` milliseconds long, fading out, as a WAV.
#[cfg(feature = "std")]
pub fn wav(pitch: f32, ms: u32) -> Vec<u8> {
    let count = SAMPLE_RATE * ms / 1000;
    let samples: Vec<i16> = (0..count)
        .map(|i| {
            // Below 2^24: exact as a float.
            #[allow(clippy::cast_precision_loss)]
            let (t, left) = (i as f32, (count - i) as f32);
            #[allow(clippy::cast_precision_loss)]
            let rate = SAMPLE_RATE as f32;
            #[allow(clippy::cast_precision_loss)]
            let fade = left / count as f32;
            let wave = (t / rate * pitch * core::f32::consts::TAU).sin();
            // Within ±12 000 by construction.
            #[allow(clippy::cast_possible_truncation)]
            let sample = (wave * fade * 12_000.0) as i16;
            sample
        })
        .collect();
    let data = u32::try_from(samples.len() * 2).unwrap_or(0);
    let mut out = Vec::with_capacity(44 + samples.len() * 2);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    // PCM, one channel.
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    // Bytes a second, bytes a frame, bits a sample.
    out.extend_from_slice(&(SAMPLE_RATE * 2).to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data.to_le_bytes());
    for sample in samples {
        out.extend_from_slice(&sample.to_le_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_cue_the_engine_plays_has_a_blip_the_audio_check_accepts() {
        let audio = manifest();
        let config = AudioConfig::DEFAULT;
        for cue in [
            config.move_cue,
            config.select_cue,
            config.cancel_cue,
            config.denied_cue,
            config.cursor_cue,
        ] {
            let sound = &audio.sounds[cue];
            for name in &sound.files {
                let bytes = file(name).unwrap_or_else(|| panic!("no {name}"));
                assert_eq!(
                    shuyi::assets::audio::format_problem(name, &bytes),
                    None,
                    "{name}"
                );
            }
        }
        assert!(audio.music.is_empty(), "{:?}", audio.music);
        assert_eq!(file("loud.wav"), None);
        let files = files();
        assert_eq!((files.sound)("move.wav"), file("move.wav"));
        assert_eq!((files.sound_shown)("move.wav"), "(made by code) move.wav");
        assert!(!(files.sound_left_out)("move.wav"));
        assert!(files.music.is_none());
    }

    #[test]
    fn a_blip_is_a_mono_16_bit_wav_that_fades_out() {
        let bytes = wav(441.0, 10);
        // 441 samples of 2 bytes after a 44-byte header.
        assert_eq!(bytes.len(), 44 + 441 * 2);
        assert_eq!(&bytes[..4], b"RIFF");
        assert_eq!(&bytes[4..8], &(36u32 + 882).to_le_bytes());
        assert_eq!(&bytes[22..24], &1u16.to_le_bytes());
        assert_eq!(&bytes[24..28], &44_100u32.to_le_bytes());
        // Bytes a second, a frame, bits a sample.
        assert_eq!(&bytes[28..32], &88_200u32.to_le_bytes());
        assert_eq!(&bytes[32..34], &2u16.to_le_bytes());
        assert_eq!(&bytes[34..36], &16u16.to_le_bytes());
        assert_eq!(&bytes[40..44], &882u32.to_le_bytes());
        let sample = |i: usize| i16::from_le_bytes([bytes[44 + 2 * i], bytes[45 + 2 * i]]);
        // It starts at 0, swings to near its peak a quarter wave in (25
        // samples at 441 Hz), and is quieter by the end.
        assert_eq!(sample(0), 0);
        assert!(sample(25) > 11_000, "{}", sample(25));
        let loudest = |from: usize| (from..from + 100).map(|i| sample(i).abs()).max();
        assert!(loudest(340) < loudest(0));
    }
}
