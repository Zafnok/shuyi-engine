//! [`Audio`]: the sound device a backend gives the engine, and the frame of
//! audio [`App`](crate::App) hands it ([`AudioFrame`]).
//!
//! The screens' requests name the game's voice clips by its own variant
//! type; what reaches the device names their files ([`SoundRequest`]), so
//! a backend is the same for every game. The rules of playing (a cue's
//! volume, picking a variant, fades, the music clock) are [`player`]'s, so a
//! backend writes only its small [`Backend`](player::Backend) trait.

pub mod player;

use crate::alloc_prelude::*;

use shuyi_assets::voice::{ClipVariant, clip_file};
use shuyi_ui::audio::{AudioRequest, MusicCommand, SoundId};

use crate::platform::Millis;

/// A sound or voice request as the device gets it: a screen's
/// [`AudioRequest`] with each voice clip named by its file in the game's
/// voice folder ([`clip_file`]). The music's requests are already
/// [`MusicCommand`]s.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[non_exhaustive]
pub enum SoundRequest {
    /// Play a sound cue once, at `volume` (0–1) times the cue's own volume.
    Sound {
        /// The sound cue.
        cue: String,
        /// A multiplier on the cue's volume, 0–1.
        volume: f32,
    },
    /// Play a sound cue until [`Stop`](Self::Stop) of its `id`: looped if
    /// the cue says so, else once. Build it with [`SoundRequest::start`]
    /// (ADR-0039).
    #[non_exhaustive]
    Start {
        /// Its handle.
        id: SoundId,
        /// The sound cue.
        cue: String,
        /// A multiplier on the cue's volume, 0–1.
        volume: f32,
    },
    /// Stop the sound started as `id`, at once: that one playback, not the
    /// cue's others.
    Stop {
        /// Its handle.
        id: SoundId,
    },
    /// Set the volume of the sound started as `id`.
    Volume {
        /// Its handle.
        id: SoundId,
        /// The new multiplier on the cue's volume, 0–1.
        volume: f32,
    },
    /// Pause every sound started with a handle that is playing now; one
    /// started after it plays. Sound cues played once, the music and the
    /// voice go on. A sound paused keeps the volume it is given and sounds
    /// at it when it resumes; one stopped while paused is gone.
    PauseSounds,
    /// Resume the sounds [`PauseSounds`](Self::PauseSounds) paused.
    ResumeSounds,
    /// Say a line: play its clip once, stopping the voice that is playing.
    Voice {
        /// The clip's file in the voice folder.
        clip: String,
    },
    /// Stop the voice that is playing, if any.
    StopVoice,
    /// The clips a scene will ask for, in order, so the device can load
    /// the next few ahead. Replaces the list sent before.
    PreloadVoices {
        /// Their files, in the order they are expected.
        clips: Vec<String>,
    },
}

impl SoundRequest {
    /// A [`Start`](Self::Start) of `cue` as `id` at `volume`.
    pub fn start(id: SoundId, cue: impl Into<String>, volume: f32) -> Self {
        Self::Start {
            id,
            cue: cue.into(),
            volume,
        }
    }

    /// `request` as the device gets it; `None` for a music request, which
    /// the frame's music commands already say.
    pub fn from_request<V: ClipVariant>(request: &AudioRequest<V>) -> Option<Self> {
        Some(match request {
            AudioRequest::PlaySound { cue, volume } => Self::Sound {
                cue: cue.clone(),
                volume: *volume,
            },
            AudioRequest::StartSound { id, cue, volume } => Self::start(*id, cue.clone(), *volume),
            AudioRequest::StopSound { id } => Self::Stop { id: *id },
            AudioRequest::SoundVolume { id, volume } => Self::Volume {
                id: *id,
                volume: *volume,
            },
            AudioRequest::PlayVoice { line, variant } => Self::Voice {
                clip: clip_file(line, *variant),
            },
            AudioRequest::PauseSounds => Self::PauseSounds,
            AudioRequest::ResumeSounds => Self::ResumeSounds,
            AudioRequest::StopVoice => Self::StopVoice,
            AudioRequest::PreloadVoices { lines } => Self::PreloadVoices {
                clips: lines.iter().map(|(l, v)| clip_file(l, *v)).collect(),
            },
            // The music's, and any request this engine doesn't know yet.
            _ => return None,
        })
    }
}

/// One frame's audio, as [`App`](crate::App) hands it to the device.
#[derive(Debug, Clone, Copy)]
pub struct AudioFrame<'a> {
    /// The sound and voice requests the screens made, in order.
    pub requests: &'a [SoundRequest],
    /// What to do to the music, in order.
    pub music: &'a [MusicCommand],
    /// The player's music volume, 0–1: it multiplies every track's own
    /// volume, a track already playing too.
    pub music_volume: f32,
    /// The player's sound volume, 0–1: it multiplies every sound's own.
    pub sound_volume: f32,
    /// How loud voice clips play, 0–1, the one playing too.
    pub voice_volume: f32,
}

/// A backend's sound device.
///
/// **Pausing.** A backend that can pause a sound pauses it; one that
/// can't mutes it (volume 0) and lets it run on silently. That holds for
/// [`SoundRequest::PauseSounds`] and for [`suspend`](Self::suspend). A
/// sound held either way keeps the volume it is given and sounds at it
/// when it is released, and [`music_playing`](Self::music_playing) says
/// where the track really is: a paused track's clock holds, a muted one's
/// runs.
pub trait Audio {
    /// The music sounding at `now`: its cue and the seconds since it really
    /// started (once its file had loaded), or `None` in silence: nothing
    /// asked for, a track still loading or whose file failed. Of two
    /// tracks sounding (a crossfade), the newest: the old one until the
    /// new one has loaded.
    fn music_playing(&self, now: Millis) -> Option<(&str, f64)>;

    /// Plays one frame's `frame` of audio at `now`: the volumes first, then
    /// the requests and the music commands in order. It also checks on
    /// what is still loading, so it is called every frame, with nothing to
    /// play too.
    fn play(&mut self, frame: &AudioFrame<'_>, now: Millis);

    /// Stops the music, the voice and the sounds started with a handle:
    /// the game has quit.
    fn stop_all(&mut self);

    /// Holds everything sounding (the sounds started with a handle, the
    /// voice and the music) while the system suspends the game
    /// ([`SystemEvent::Suspend`](crate::SystemEvent::Suspend)); sound
    /// cues played once finish. Pauses, or mutes where the backend can't
    /// pause. A second call does nothing.
    ///
    /// It has no default: every backend says what it can hold (ADR-0056).
    fn suspend(&mut self);

    /// Brings back what [`suspend`](Self::suspend) held, and leaves a
    /// [`SoundRequest::PauseSounds`] pause in place. A call while not
    /// suspended does nothing.
    fn resume(&mut self);
}

/// A device that plays nothing: for a backend with no sound yet, and tests.
#[derive(Debug, Clone, Copy, Default)]
pub struct Silent;

impl Audio for Silent {
    fn music_playing(&self, _: Millis) -> Option<(&str, f64)> {
        None
    }

    fn play(&mut self, _: &AudioFrame<'_>, _: Millis) {}

    fn stop_all(&mut self) {}

    fn suspend(&mut self) {}

    fn resume(&mut self) {}
}

#[cfg(test)]
mod tests {
    use alloc::vec;

    use shuyi_assets::dialogue::LineId;
    use shuyi_ui::toy::Voice;

    use super::*;

    fn line(id: &str) -> LineId {
        LineId::new(id)
    }

    #[test]
    fn a_voice_is_its_clips_file_and_music_stays_with_the_commands() {
        let play = |request: AudioRequest<Voice>| SoundRequest::from_request(&request);
        assert_eq!(
            play(AudioRequest::PlaySound {
                cue: "click".into(),
                volume: 0.5
            }),
            Some(SoundRequest::Sound {
                cue: "click".into(),
                volume: 0.5
            })
        );
        let hello = line("intro.hello");
        let only = clip_file(&hello, Voice::Only);
        assert_eq!(
            play(AudioRequest::PlayVoice {
                line: hello.clone(),
                variant: Voice::Only
            }),
            Some(SoundRequest::Voice { clip: only.clone() })
        );
        assert_eq!(
            play(AudioRequest::PreloadVoices {
                lines: vec![(hello.clone(), Voice::Only), (hello, Voice::Low)]
            })
            .map(|r| match r {
                SoundRequest::PreloadVoices { clips } => clips.len(),
                _ => 0,
            }),
            Some(2)
        );
        assert_eq!(play(AudioRequest::StopVoice), Some(SoundRequest::StopVoice));
        let mut queue = shuyi_ui::audio::AudioQueue::<Voice>::default();
        let id = queue.start_sound_at("hum", 0.5);
        queue.set_sound_volume(id, 0.25);
        queue.stop_sound(id, 0);
        let got: Vec<_> = queue
            .take()
            .iter()
            .filter_map(SoundRequest::from_request)
            .collect();
        assert_eq!(
            got,
            [
                SoundRequest::start(id, "hum", 0.5),
                SoundRequest::Volume { id, volume: 0.25 },
                SoundRequest::Stop { id },
            ]
        );
        assert_eq!(
            play(AudioRequest::PauseSounds),
            Some(SoundRequest::PauseSounds)
        );
        assert_eq!(
            play(AudioRequest::ResumeSounds),
            Some(SoundRequest::ResumeSounds)
        );
        assert_eq!(play(AudioRequest::play_music("theme", None)), None);
        assert_eq!(play(AudioRequest::StopMusic), None);
    }

    #[test]
    fn silence_plays_nothing() {
        let mut silent = Silent;
        let frame = AudioFrame {
            requests: &[SoundRequest::StopVoice],
            music: &[],
            music_volume: 1.0,
            sound_volume: 1.0,
            voice_volume: 1.0,
        };
        silent.play(&frame, Millis(0));
        silent.suspend();
        silent.resume();
        silent.stop_all();
        assert_eq!(silent.music_playing(Millis(0)), None);
    }
}
