//! The sounds a game adds at run time ([`AudioQueue::add_sound`]): samples
//! made or read while the game runs (a mod's sounds, a synth, sounds read from
//! the player's own files), which become cues as a manifest's are. The queue
//! keeps their sizes against the limits, refuses a cue of the manifest, and
//! silences a cue removed.

use crate::prelude::*;
use alloc::collections::{BTreeMap, BTreeSet};

use shuyi_assets::audio::AudioManifest;

use super::{AudioQueue, AudioRequest, Samples, SharedSamples};

/// The most bytes of samples the sounds added hold at once, all together:
/// 32 MiB, two bytes a sample.
pub const ADDED_BYTES_MAX: u64 = 32 * 1024 * 1024;

/// The longest a sound added may play, in seconds.
pub const ADDED_SECONDS_MAX: u64 = 60;

/// Why [`AudioQueue::add_sound`] refused a sound. The game's log says it too.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum AddSoundError {
    /// The audio manifest has a sound cue of that name, which an added sound
    /// can't replace.
    #[error("sound \"{0}\" is a cue of the audio manifest: an added sound can't replace it")]
    InManifest(String),
    /// The sound plays longer than [`ADDED_SECONDS_MAX`].
    #[error(
        "sound \"{cue}\" plays {millis} milliseconds: an added sound plays {ADDED_SECONDS_MAX} seconds at most"
    )]
    TooLong {
        /// The sound cue.
        cue: String,
        /// How long it plays, in milliseconds.
        millis: u64,
    },
    /// The sounds added would hold more than [`ADDED_BYTES_MAX`] bytes.
    #[error(
        "sound \"{cue}\" would make the sounds added {bytes} bytes: they hold {ADDED_BYTES_MAX} at most"
    )]
    TooMuch {
        /// The sound cue.
        cue: String,
        /// The bytes they would hold with it.
        bytes: u64,
    },
}

/// The queue's account of the sounds added.
#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct Added {
    /// The manifest's sound cues, which can't be added.
    manifest: BTreeSet<String>,
    /// The cues added and not removed, with their bytes.
    sounds: BTreeMap<String, u64>,
    /// The bytes of `sounds`, all together.
    bytes: u64,
    /// The cues removed and not added again, each with whether a play of it
    /// was logged.
    removed: BTreeMap<String, bool>,
    /// What was refused, or played after its removal, for the log.
    problems: Vec<String>,
}

impl Added {
    /// Whether `cue` was removed, so a play of it plays nothing: the first
    /// such play since its removal is a problem for the log.
    pub(super) fn silences(&mut self, cue: &str) -> bool {
        let Some(logged) = self.removed.get_mut(cue) else {
            return false;
        };
        if !*logged {
            *logged = true;
            self.problems.push(format!(
                "sound \"{cue}\" was removed: it plays nothing until it is added again"
            ));
        }
        true
    }
}

impl<V> AudioQueue<V> {
    /// Tells the queue the sound cues of `manifest`, which
    /// [`add_sound`](Self::add_sound) refuses. The context tells it the
    /// game's.
    pub fn set_manifest(&mut self, manifest: &AudioManifest) {
        self.added.manifest = manifest.sounds.keys().cloned().collect();
    }

    /// Makes `samples` the sound cue `cue`: from now on
    /// [`play_sound`](Self::play_sound) and
    /// [`start_sound`](Self::start_sound) of `cue` play it as a cue of the
    /// manifest would, at its own volume, full, and once. Adding a cue added
    /// before replaces it. Refused, with a problem in the game's log:
    ///
    /// - a cue the audio manifest has ([`AddSoundError::InManifest`]): a
    ///   checked cue isn't replaced by accident;
    /// - a sound longer than [`ADDED_SECONDS_MAX`]
    ///   ([`AddSoundError::TooLong`]);
    /// - a sound that would make the sounds added hold more than
    ///   [`ADDED_BYTES_MAX`] ([`AddSoundError::TooMuch`]).
    ///
    /// ```
    /// # use shuyi_ui::audio::{AudioQueue, Samples};
    /// // A tenth of a second of a square wave, 8-bit, at 11,025 a second.
    /// let bytes: Vec<u8> = (0..1102).map(|i| if i % 50 < 25 { 64 } else { 192 }).collect();
    /// let beep = Samples::from_u8(11_025, 1, &bytes).unwrap();
    /// let mut audio = AudioQueue::<()>::default();
    /// audio.add_sound("beep", beep).unwrap();
    /// audio.play_sound("beep");
    /// assert_eq!(audio.pending().len(), 2);
    /// ```
    pub fn add_sound(&mut self, cue: &str, samples: Samples) -> Result<(), AddSoundError> {
        let checked = self.check_added(cue, &samples);
        if let Err(error) = &checked {
            self.added.problems.push(error.to_string());
            return checked;
        }
        let bytes = samples.bytes();
        let old = self.added.sounds.insert(cue.to_owned(), bytes);
        self.added.bytes = self.added.bytes - old.unwrap_or(0) + bytes;
        self.added.removed.remove(cue);
        self.requests.push(AudioRequest::AddSound {
            cue: cue.to_owned(),
            samples: SharedSamples::new(samples),
        });
        Ok(())
    }

    /// Whether `samples` may be added as `cue`.
    fn check_added(&self, cue: &str, samples: &Samples) -> Result<(), AddSoundError> {
        if self.added.manifest.contains(cue) {
            return Err(AddSoundError::InManifest(cue.to_owned()));
        }
        let longest = u64::from(samples.rate()).saturating_mul(ADDED_SECONDS_MAX);
        if samples.frames() > longest {
            return Err(AddSoundError::TooLong {
                cue: cue.to_owned(),
                millis: samples.millis(),
            });
        }
        let replaced = self.added.sounds.get(cue).copied().unwrap_or(0);
        let bytes = (self.added.bytes - replaced).saturating_add(samples.bytes());
        if bytes > ADDED_BYTES_MAX {
            return Err(AddSoundError::TooMuch {
                cue: cue.to_owned(),
                bytes,
            });
        }
        Ok(())
    }

    /// Removes the sound cue `cue` that [`add_sound`](Self::add_sound)
    /// made, stopping what plays it and freeing its samples. From then on a
    /// play of it plays nothing, and the first is a problem in the game's
    /// log, until it is added again. A cue that wasn't added is a problem
    /// in the log too.
    pub fn remove_sound(&mut self, cue: &str) {
        let Some(bytes) = self.added.sounds.remove(cue) else {
            self.added
                .problems
                .push(format!("sound \"{cue}\" can't be removed: it wasn't added"));
            return;
        };
        self.added.bytes -= bytes;
        self.added.removed.insert(cue.to_owned(), false);
        self.requests.push(AudioRequest::RemoveSound {
            cue: cue.to_owned(),
        });
    }

    /// Whether `cue` was added and not removed.
    pub fn is_added(&self, cue: &str) -> bool {
        self.added.sounds.contains_key(cue)
    }

    /// The bytes of samples the sounds added hold now, all together: at most
    /// [`ADDED_BYTES_MAX`].
    pub fn added_bytes(&self) -> u64 {
        self.added.bytes
    }

    /// Whether `cue` was ever added, removed since or not: a play of it is
    /// one of a cue the game made.
    pub(crate) fn knows_added(&self, cue: &str) -> bool {
        self.added.sounds.contains_key(cue) || self.added.removed.contains_key(cue)
    }

    /// Takes the problems for the game's log, oldest first:
    /// [`Game`](crate::Game) logs them every frame.
    pub fn take_problems(&mut self) -> Vec<String> {
        core::mem::take(&mut self.added.problems)
    }
}

#[cfg(test)]
mod tests;
