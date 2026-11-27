//! The game's sounds: a short blip for each of the engine's menu and cursor
//! sounds, each a sound effect written as a few numbers in `files/sfx/`
//! (`.sfx.ron`), which the sound hook renders to a WAV as it reads it
//! (`audio::playable`).

use crate::prelude::*;

use shuyi::prelude::audio::{AudioConfig, AudioManifest, CreditRef, CueRules, SoundCue};
#[cfg(feature = "std")]
use shuyi::prelude::audio::{Bytes, Files, playable};

/// The sound files, by their names under `files/`: the highlight's move,
/// a choice made, backing out and the cursor's tick.
pub const SOUNDS: [&str; 4] = [
    "sfx/move.sfx.ron",
    "sfx/select.sfx.ron",
    "sfx/cancel.sfx.ron",
    "sfx/cursor.sfx.ron",
];

/// How soon the cursor's tick and the move's blip may play again, in
/// milliseconds: a cursor that moves several cells in one frame ticks once.
pub const STEP_GAP_MS: u32 = 40;

/// The audio manifest: the engine's four sound cues ([`AudioConfig`]),
/// each a file of [`SOUNDS`], the cursor's and the move's with a gap of
/// [`STEP_GAP_MS`]; no music.
pub fn manifest() -> AudioManifest {
    let config = AudioConfig::DEFAULT;
    let [moved, select, cancel, cursor] = SOUNDS;
    let cues = [
        (config.move_cue, moved),
        (config.select_cue, select),
        (config.cancel_cue, cancel),
        (config.cursor_cue, cursor),
    ];
    let mut audio = AudioManifest::default();
    for (cue, file) in cues {
        let mut sound = SoundCue::new(vec![file.to_owned()], 60, CreditRef::Own);
        if cue == config.cursor_cue || cue == config.move_cue {
            sound = sound.with_rules(CueRules::DEFAULT.with_min_gap_ms(STEP_GAP_MS));
        }
        audio.sounds.insert(cue.to_owned(), sound);
    }
    audio
}

/// The sound files for the platform: the game's files ([`crate::file`]),
/// each sound effect rendered to its WAV; no music folder.
#[cfg(feature = "std")]
pub fn files() -> Files<'static> {
    Files::new(&file, &shown, &left_out, "music")
}

/// How errors name a sound file: under `files/`, as `cargo shuyi check`
/// does.
#[cfg(feature = "std")]
fn shown(name: &str) -> String {
    format!("files/{name}")
}

/// Whether a build leaves the sound file out on purpose: none does.
#[cfg(feature = "std")]
fn left_out(_: &str) -> bool {
    false
}

/// The sound file `name` (under `files/`) as the platform plays it: a
/// `.sfx.ron` rendered to its WAV; `None` if there is no such file.
#[cfg(feature = "std")]
pub fn file(name: &str) -> Bytes {
    playable(name, crate::file(name))
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    /// The cursor's tick and the move's blip have a gap, so a cursor that
    /// moves several cells in one frame ticks once; the others have none.
    #[test]
    fn the_cursor_and_the_move_have_a_gap_and_the_others_none() {
        let config = AudioConfig::DEFAULT;
        let audio = manifest();
        let gaps: Vec<(&str, u32)> = audio
            .sounds
            .iter()
            .map(|(cue, sound)| (cue.as_str(), sound.rules.min_gap_ms))
            .collect();
        let mut want = vec![
            (config.move_cue, STEP_GAP_MS),
            (config.select_cue, 0),
            (config.cancel_cue, 0),
            (config.cursor_cue, STEP_GAP_MS),
        ];
        want.sort_unstable();
        assert_eq!(gaps, want);
    }

    /// Every cue the engine plays names a sound effect of `files/sfx/`,
    /// which the hook hands the platform as a WAV the audio check accepts.
    #[test]
    fn every_cue_the_engine_plays_has_a_sound_the_audio_check_accepts() {
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
                assert_eq!(&bytes[..4], b"RIFF", "{name} is rendered to a WAV");
                assert_eq!(
                    shuyi::prelude::audio::format_problem(name, &bytes),
                    None,
                    "{name}"
                );
            }
        }
        assert!(audio.music.is_empty(), "{:?}", audio.music);
        assert_eq!(file("sfx/loud.sfx.ron"), None);
        let files = files();
        assert_eq!((files.sound)(SOUNDS[0]), file(SOUNDS[0]));
        assert_eq!((files.sound_shown)(SOUNDS[0]), "files/sfx/move.sfx.ron");
        assert!(!(files.sound_left_out)(SOUNDS[0]));
        assert!(files.music.is_none());
    }

    /// Each sound is a blip near the pitch and length the game's blips
    /// had when code made them: the cursor's shortest, the cancel's lowest.
    #[test]
    fn each_sound_is_a_short_blip_at_its_pitch() {
        let want = [(660, 50), (880, 80), (440, 80), (990, 30)];
        for (name, (hz, ms)) in SOUNDS.into_iter().zip(want) {
            let text = crate::file(name).unwrap_or_else(|| panic!("no {name}"));
            let source = core::str::from_utf8(&text).unwrap();
            let def = shuyi::assets::sfx::from_source(name, source).unwrap();
            assert_eq!((def.hz, def.length_ms()), (hz, ms), "{name}");
        }
    }
}
