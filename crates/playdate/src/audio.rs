//! [`Speaker`]: the shared player's [`Backend`] on the SDK's sample players
//! (sounds in memory, each playback a player of its own) and file players
//! (music and voice clips streamed from the `.pdx`).
//!
//! `shuyi export playdate` bakes each sound under
//! [`BAKED_SOUND_DIR`] and each track under [`BAKED_MUSIC_DIR`], and the SDK
//! opens a file by its path without the extension. A track the game doesn't
//! ship there is silence, with the player's warning.
//!
//! The SDK can pause both kinds of player and set their rates, so this keeps
//! no default of [`Backend`]'s.

use alloc::borrow::ToOwned;
use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use core::cell::Cell;
use core::future::{Future, ready};
use core::pin::pin;
use core::task::{Context, Poll, Waker};

use shuyi_assets::audio::{
    AudioManifest, BAKED_MUSIC_DIR, BAKED_SOUND_DIR, Files, without_extension,
};
use shuyi_platform::audio::player::{Backend, Player, RATE_ONE, Unsupported};

use crate::sdk::Sdk;

/// How many sounds play at once: one sample player each, made at the start.
/// A sound asked for while all play stops one of them, each in turn.
pub const SAMPLE_PLAYERS: usize = 8;

/// Where the voice clips would be in the `.pdx`: the export bakes none yet.
const VOICE_DIR: &str = "voice";

/// A rate of [`Backend::set_rate`]'s as the SDK takes it.
fn rate_factor(rate: u32) -> f32 {
    // A rate is small: the precision lost is far below hearing.
    #[allow(clippy::cast_precision_loss)]
    let rate = rate as f32;
    #[allow(clippy::cast_precision_loss)]
    let one = RATE_ONE as f32;
    rate / one
}

/// One sample player.
#[derive(Debug)]
struct Slot<S: Sdk> {
    player: S::SamplePlayer,
    /// What it last played.
    sample: Option<S::Sample>,
    /// Which playback it is on: an old playback's handle does nothing.
    serial: u32,
    /// Whether it is paused: it can't be taken for another sound.
    paused: bool,
}

/// The SDK's speaker.
#[derive(Debug)]
pub struct Speaker<S: Sdk> {
    sdk: S,
    slots: Vec<Slot<S>>,
    /// The samples loaded, by path: cues that share a file share its memory.
    loaded: BTreeMap<String, S::Sample>,
    /// The serial of the last playback.
    serial: u32,
    /// The slot a sound takes when all play: they are taken in turn.
    oldest: usize,
}

/// A sound the SDK holds, or a file player open on a track or a clip.
#[derive(Debug)]
pub enum Clip<S: Sdk> {
    /// A sound in memory, for the whole game.
    Sample {
        /// The SDK's sample.
        sample: S::Sample,
        /// Its rate from its next play, 16.16.
        rate: Cell<u32>,
    },
    /// A file player, closed when this goes.
    File(FileClip<S>),
}

/// A file player open on its file: dropping it stops and frees it, which is
/// how a track the player lets go of leaves the memory.
#[derive(Debug)]
pub struct FileClip<S: Sdk> {
    sdk: S,
    player: S::FilePlayer,
    /// Whether it has played: playing again starts it from the top.
    played: Cell<bool>,
    /// Its rate from its next play, 16.16.
    rate: Cell<u32>,
}

impl<S: Sdk> Drop for FileClip<S> {
    fn drop(&mut self) {
        self.sdk.file_close(self.player);
    }
}

/// One playback.
#[derive(Debug)]
pub enum Playing<S: Sdk> {
    /// Nothing plays: the SDK has no sample player to give.
    Nothing,
    /// A sample player, on playback `serial`.
    Sample {
        /// Which of the speaker's sample players.
        slot: usize,
        /// Which playback.
        serial: u32,
    },
    /// A file player.
    File {
        /// The file player.
        player: S::FilePlayer,
        /// Whether it loops, which its resume says again.
        looped: bool,
        /// Whether it is paused: a track that ended has nothing to resume.
        paused: Cell<bool>,
    },
}

impl<S: Sdk> Speaker<S> {
    /// The speaker on `sdk`, with its sample players made.
    pub fn new(sdk: S) -> Self {
        let slots = (0..SAMPLE_PLAYERS)
            .filter_map(|_| sdk.sample_player_new())
            .map(|player| Slot {
                player,
                sample: None,
                serial: 0,
                paused: false,
            })
            .collect();
        Self {
            sdk,
            slots,
            loaded: BTreeMap::new(),
            serial: 0,
            oldest: 0,
        }
    }

    /// The slot for a new sound: one that is free, else the next in turn that
    /// isn't paused. `None` if all are paused.
    fn free_slot(&mut self) -> Option<usize> {
        let free = self
            .slots
            .iter()
            .position(|slot| !slot.paused && !self.sdk.sample_player_is_playing(slot.player));
        if free.is_some() {
            return free;
        }
        let count = self.slots.len();
        let taken = (0..count)
            .map(|turn| (self.oldest + turn) % count)
            .find(|&at| !self.slots[at].paused)?;
        self.oldest = (taken + 1) % count;
        Some(taken)
    }

    /// The slot of `playing`, if it is still on that playback.
    fn slot_of(&mut self, playing: &Playing<S>) -> Option<&mut Slot<S>> {
        match playing {
            Playing::Sample { slot, serial } => {
                self.slots.get_mut(*slot).filter(|at| at.serial == *serial)
            }
            _ => None,
        }
    }

    /// The sample players on `sample`.
    fn slots_on(&self, sample: S::Sample) -> impl Iterator<Item = &Slot<S>> {
        self.slots
            .iter()
            .filter(move |slot| slot.sample == Some(sample))
    }
}

impl<S: Sdk> Backend for Speaker<S> {
    type Sound = Clip<S>;
    type Loading = Option<Result<Clip<S>, String>>;
    type Playing = Playing<S>;

    /// The SDK loads a sound by its path ([`load_sound_file`](Self::load_sound_file)):
    /// it is baked to a format only `pdc` writes, and no bytes come here.
    fn load_sound(&mut self, _bytes: &[u8]) -> impl Future<Output = Result<Clip<S>, String>> {
        // check-text: not player text (for the log)
        ready(Err("the Playdate loads a sound from its path".to_owned()))
    }

    fn load_sound_file(
        &mut self,
        file: &str,
        _files: &Files<'_>,
    ) -> impl Future<Output = Result<Clip<S>, String>> {
        let path = format!("{BAKED_SOUND_DIR}/{}", without_extension(file));
        let sample = match self.loaded.get(&path) {
            Some(sample) => Some(*sample),
            None => self.sdk.sample_load(&path),
        };
        ready(match sample {
            Some(sample) => {
                self.loaded.insert(path, sample);
                Ok(Clip::Sample {
                    sample,
                    rate: Cell::new(RATE_ONE),
                })
            }
            // check-text: not player text (for the log)
            None => Err("not in the .pdx".to_owned()),
        })
    }

    fn load_music(&mut self, path: &str) -> Self::Loading {
        let opened = self.sdk.file_open(without_extension(path));
        Some(match opened {
            Some(player) => Ok(Clip::File(FileClip {
                sdk: self.sdk.clone(),
                player,
                played: Cell::new(false),
                rate: Cell::new(RATE_ONE),
            })),
            // check-text: not player text (for the log)
            None => Err("not in the .pdx".to_owned()),
        })
    }

    fn poll_music(&mut self, loading: &mut Self::Loading) -> Option<Result<Clip<S>, String>> {
        loading.take()
    }

    fn length(&self, sound: &Clip<S>) -> Option<f64> {
        let seconds = match sound {
            Clip::Sample { sample, .. } => self.sdk.sample_length(*sample),
            Clip::File(file) => self.sdk.file_length(file.player),
        };
        (seconds > 0.0).then(|| f64::from(seconds))
    }

    fn play(&mut self, sound: &Clip<S>, volume: f32, looped: bool) -> Playing<S> {
        match sound {
            Clip::Sample { sample, rate } => {
                let Some(slot) = self.free_slot() else {
                    return Playing::Nothing;
                };
                self.serial = self.serial.wrapping_add(1);
                let at = &mut self.slots[slot];
                at.sample = Some(*sample);
                at.serial = self.serial;
                let factor = rate_factor(rate.get());
                self.sdk
                    .sample_player_play(at.player, *sample, volume, looped, factor);
                Playing::Sample {
                    slot,
                    serial: self.serial,
                }
            }
            Clip::File(file) => {
                if file.played.replace(true) {
                    self.sdk.file_stop(file.player);
                    self.sdk.file_rewind(file.player);
                }
                self.sdk
                    .file_set_rate(file.player, rate_factor(file.rate.get()));
                self.sdk.file_set_volume(file.player, volume);
                if !self.sdk.file_play(file.player, looped) {
                    // check-text: not player text (for the log)
                    self.sdk.log("audio: the SDK could not start a track");
                }
                Playing::File {
                    player: file.player,
                    looped,
                    paused: Cell::new(false),
                }
            }
        }
    }

    fn set_volume(&mut self, sound: &Clip<S>, volume: f32) {
        match sound {
            Clip::Sample { sample, .. } => {
                for slot in self.slots_on(*sample) {
                    self.sdk.sample_player_set_volume(slot.player, volume);
                }
            }
            Clip::File(file) => self.sdk.file_set_volume(file.player, volume),
        }
    }

    fn stop(&mut self, sound: &Clip<S>) {
        match sound {
            Clip::Sample { sample, .. } => {
                for slot in self.slots_on(*sample) {
                    self.sdk.sample_player_stop(slot.player);
                }
            }
            Clip::File(file) => self.sdk.file_stop(file.player),
        }
    }

    fn set_playing_volume(&mut self, playing: &Playing<S>, volume: f32) {
        if let Playing::File { player, .. } = playing {
            self.sdk.file_set_volume(*player, volume);
        } else if let Some(player) = self.slot_of(playing).map(|slot| slot.player) {
            self.sdk.sample_player_set_volume(player, volume);
        }
    }

    fn stop_playing(&mut self, playing: Playing<S>) {
        if let Playing::File { player, .. } = playing {
            self.sdk.file_stop(player);
        } else if let Some(slot) = self.slot_of(&playing) {
            slot.paused = false;
            let player = slot.player;
            self.sdk.sample_player_stop(player);
        }
    }

    fn pause(&mut self, playing: &Playing<S>) -> Result<(), Unsupported> {
        if let Playing::File { player, paused, .. } = playing {
            if self.sdk.file_is_playing(*player) {
                self.sdk.file_pause(*player);
                paused.set(true);
            }
        } else if let Some(player) = self.slot_of(playing).map(|slot| slot.player)
            && self.sdk.sample_player_is_playing(player)
        {
            self.sdk.sample_player_set_paused(player, true);
            if let Some(slot) = self.slot_of(playing) {
                slot.paused = true;
            }
        }
        Ok(())
    }

    fn resume(&mut self, playing: &Playing<S>) -> Result<(), Unsupported> {
        if let Playing::File {
            player,
            looped,
            paused,
        } = playing
        {
            // The SDK goes on from where its pause left the file.
            if paused.replace(false) && !self.sdk.file_play(*player, *looped) {
                // check-text: not player text (for the log)
                self.sdk.log("audio: the SDK could not resume a track");
            }
        } else if let Some(slot) = self.slot_of(playing).filter(|slot| slot.paused) {
            slot.paused = false;
            let player = slot.player;
            self.sdk.sample_player_set_paused(player, false);
        }
        Ok(())
    }

    fn set_rate(&mut self, sound: &Clip<S>, rate: u32) -> Result<(), Unsupported> {
        match sound {
            Clip::Sample { rate: at, .. } => at.set(rate),
            Clip::File(file) => file.rate.set(rate),
        }
        Ok(())
    }

    fn warn(&mut self, line: &str) {
        // check-text: not player text (for the log)
        self.sdk.log(&format!("audio: {line}"));
    }
}

/// A file the Playdate reads for itself: no bytes come from here.
fn no_bytes(_: &str) -> shuyi_assets::audio::Bytes {
    None
}

/// How the player's warnings name a sound of the manifest: by its path in
/// the `.pdx`.
fn shown(file: &str) -> String {
    format!("{BAKED_SOUND_DIR}/{}", without_extension(file))
}

/// Whether a sound is left out on purpose: the export bakes what the game
/// ships, so that is known there.
fn left_out(_: &str) -> bool {
    false
}

/// The player of `manifest` on `speaker`: its sounds loaded, from the `.pdx`.
pub fn player<S: Sdk>(
    speaker: &mut Speaker<S>,
    manifest: AudioManifest,
    seed: u64,
) -> Player<Speaker<S>> {
    let files = Files::new(&no_bytes, &shown, &left_out, BAKED_MUSIC_DIR);
    let music = BAKED_MUSIC_DIR.to_owned();
    let voice = VOICE_DIR.to_owned();
    ready_now(Player::load(speaker, manifest, &files, music, voice, seed))
}

/// What `future` gives, when nothing it waits on is pending: on the SDK every
/// load is done when it returns.
fn ready_now<F: Future>(future: F) -> F::Output {
    let mut future = pin!(future);
    let mut cx = Context::from_waker(Waker::noop());
    loop {
        if let Poll::Ready(value) = future.as_mut().poll(&mut cx) {
            return value;
        }
    }
}

#[cfg(test)]
mod tests;
