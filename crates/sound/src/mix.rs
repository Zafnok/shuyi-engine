//! The mixer: everything playing, added up into one block of samples, in whole
//! numbers, so the same script gives the same samples on every target.
//!
//! - Volumes and balance are `u16`s in 1/4096 ([`UNITY`] is full), the sum is in
//!   `i32`, and the result clamps to `i16`.
//! - A rate is 16.16 ([`RATE_ONE`] is as recorded). A sound's own rate and this rate
//!   make one exact step, and frames between are linearly interpolated.
//! - [`Mixer::render`] takes no lock, allocates nothing, frees nothing, and does no
//!   decoder work or I/O, so a system's audio callback can call it. All the memory
//!   is taken in [`Mixer::new`].
//! - A voice is never stopped to make room for another: with every voice busy,
//!   [`Mixer::play`] says [`Full`]. Which sound loses is the caller's to decide,
//!   before it gets here.
//!
//! A sound is decoded whole and held by the bank ([`Mixer::add_sound`]), and voices
//! name it by [`SoundId`]. A stream is a [`Source`] that has its frames ready; a
//! worker decodes ahead into a [`stream::feed`], whose module says what the two
//! ends promise each other. [`queue`] is how another thread commands a mixer.

use alloc::boxed::Box;
use alloc::vec::Vec;

use crate::pcm::{Channels, Pcm};
use crate::source::Source;

pub mod queue;
mod resample;
pub mod stream;
mod voice;

#[cfg(test)]
mod tests;

// What two owners both hold. A target without shared atomic counts has one thread.
#[cfg(not(target_has_atomic = "ptr"))]
use alloc::rc::Rc as Shared;
#[cfg(target_has_atomic = "ptr")]
use alloc::sync::Arc as Shared;

use self::queue::{Command, Inbox, Remote};
use self::stream::Slot;
use self::voice::{Origin, Sources, Voice};

/// A sound many voices can hold.
pub type SharedPcm = Shared<Pcm>;

/// Full volume, and an even balance, in 1/4096.
pub const UNITY: u16 = 4096;
/// As recorded, in 16.16.
pub const RATE_ONE: u32 = 1 << 16;
/// The slowest rate: a voice always ends, or comes round, in a bounded time.
pub const RATE_MIN: u32 = RATE_ONE >> 8;
/// The fastest rate: what a block costs stays bounded.
pub const RATE_MAX: u32 = RATE_ONE << 8;

/// How many frames the mixer adds up at a time.
const BLOCK_FRAMES: usize = 256;

/// Every voice is busy, or the bank or the queue has no room. Nothing was stopped
/// to make any.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("no room: every voice is busy")]
pub struct Full;

/// A place in a fixed table, and which of its tenants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Handle {
    index: u32,
    generation: u32,
}

/// A playing sound. Once it has ended the id means nothing: an operation on it
/// does nothing, even when its voice is playing something else by then.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VoiceId(Handle);

/// A sound of a mixer's bank.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SoundId(Handle);

/// A stream waiting in a mixer to be played.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StreamId(Handle);

/// Part of a source, in frames: `start` up to, not including, `end`. It plays
/// what comes before it once, then goes round it with no gap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Region {
    start: u64,
    end: u64,
}

impl Region {
    /// From `start` up to `end`, if that is any frames at all.
    pub const fn new(start: u64, end: u64) -> Option<Self> {
        if start < end {
            Some(Self { start, end })
        } else {
            None
        }
    }

    /// Its first frame.
    pub const fn start(self) -> u64 {
        self.start
    }

    /// The frame after its last.
    pub const fn end(self) -> u64 {
        self.end
    }
}

/// How a voice starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Params {
    /// In 1/4096. Over [`UNITY`] it amplifies.
    pub volume: u16,
    /// Starts over at its end. A region loops whether or not this is set.
    pub looped: bool,
    /// In 16.16, held between [`RATE_MIN`] and [`RATE_MAX`]: it moves the pitch
    /// and the length together.
    pub rate: u32,
    /// The left and right gains, in 1/4096. A mono output plays their mean.
    pub balance: (u16, u16),
    /// What it loops, after playing what comes before once.
    pub region: Option<Region>,
}

impl Params {
    /// Full volume, as recorded, centred, once through.
    pub const fn new() -> Self {
        Self {
            volume: UNITY,
            looped: false,
            rate: RATE_ONE,
            balance: (UNITY, UNITY),
            region: None,
        }
    }

    /// At `volume`, in 1/4096.
    #[must_use]
    pub const fn with_volume(mut self, volume: u16) -> Self {
        self.volume = volume;
        self
    }

    /// Starting over at its end.
    #[must_use]
    pub const fn looped(mut self) -> Self {
        self.looped = true;
        self
    }

    /// At `rate`, in 16.16.
    #[must_use]
    pub const fn with_rate(mut self, rate: u32) -> Self {
        self.rate = rate;
        self
    }

    /// With these gains, in 1/4096.
    #[must_use]
    pub const fn with_balance(mut self, left: u16, right: u16) -> Self {
        self.balance = (left, right);
        self
    }

    /// Looping `region`.
    #[must_use]
    pub const fn with_region(mut self, region: Region) -> Self {
        self.region = Some(region);
        self
    }
}

impl Default for Params {
    fn default() -> Self {
        Self::new()
    }
}

/// What a mixer is made with. All its memory comes from these.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Config {
    /// The output's frames a second.
    pub rate: u32,
    /// The output's channels.
    pub channels: Channels,
    /// The most voices sounding at once.
    pub voices: usize,
    /// The most sounds in the bank at once.
    pub sounds: usize,
    /// The most streams held at once.
    pub streams: usize,
}

impl Config {
    /// [`Self::DEFAULT_SOUNDS`] sounds and [`Self::DEFAULT_STREAMS`] streams.
    pub const fn new(rate: u32, channels: Channels, voices: usize) -> Self {
        Self {
            rate,
            channels,
            voices,
            sounds: Self::DEFAULT_SOUNDS,
            streams: Self::DEFAULT_STREAMS,
        }
    }

    /// A bank's size by default.
    pub const DEFAULT_SOUNDS: usize = 256;
    /// Two, for a crossfade.
    pub const DEFAULT_STREAMS: usize = 2;

    /// With room for `sounds` sounds.
    #[must_use]
    pub const fn with_sounds(mut self, sounds: usize) -> Self {
        self.sounds = sounds;
        self
    }

    /// With room for `streams` streams.
    #[must_use]
    pub const fn with_streams(mut self, streams: usize) -> Self {
        self.streams = streams;
        self
    }
}

/// The mix ([module docs](self)).
pub struct Mixer {
    config: Config,
    channels: usize,
    voices: Vec<Voice>,
    sounds: Vec<Option<SharedPcm>>,
    /// Each of `sounds`' slots' tenants so far.
    generations: Vec<u32>,
    streams: Vec<Slot>,
    /// One block, added up.
    block: Box<[i32]>,
    underrun_frames: u64,
}

impl core::fmt::Debug for Mixer {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Mixer")
            .field("config", &self.config)
            .field("underrun_frames", &self.underrun_frames)
            .finish_non_exhaustive()
    }
}

impl Mixer {
    /// A mixer for `config`, with all the memory it will ever use.
    pub fn new(config: Config) -> Self {
        let channels = usize::from(config.channels.count());
        Self {
            config,
            channels,
            voices: alloc::vec![Voice::new(); config.voices],
            sounds: alloc::vec![None; config.sounds],
            generations: alloc::vec![0; config.sounds],
            streams: (0..config.streams).map(|_| Slot::new()).collect(),
            block: alloc::vec![0; BLOCK_FRAMES * channels].into_boxed_slice(),
            underrun_frames: 0,
        }
    }

    /// Its output's format.
    pub const fn config(&self) -> Config {
        self.config
    }

    /// The two ends for commanding this mixer from another thread, with room for
    /// `commands` commands waiting ([`queue`]).
    pub fn remote(&self, commands: usize) -> (Remote, Inbox) {
        let sounds = self
            .generations
            .iter()
            .zip(&self.sounds)
            .map(|(generation, sound)| (*generation, sound.is_some()))
            .collect();
        let streams = self
            .streams
            .iter()
            .map(|slot| (slot.generation, slot.source.is_some()))
            .collect();
        queue::ends(self.voices.len(), sounds, streams, commands)
    }

    /// Puts `pcm` in the place of the bank [`Remote::reserve_sound`] named, and gives
    /// back what was there for the caller to free. It neither allocates nor frees, so
    /// a backend's hand-over can call it on the audio thread.
    pub fn put_sound(&mut self, sound: SoundId, pcm: SharedPcm) -> Option<SharedPcm> {
        let Some(index) = usize::try_from(sound.0.index)
            .ok()
            .filter(|index| *index < self.sounds.len())
        else {
            return Some(pcm);
        };
        self.generations[index] = sound.0.generation;
        self.sounds[index].replace(pcm)
    }

    /// Holds `source` in the stream slot [`Remote::reserve_stream`] named, and gives
    /// back what was there for the caller to free. It neither allocates nor frees.
    pub fn put_stream(
        &mut self,
        stream: StreamId,
        source: Box<dyn Source + Send>,
    ) -> Option<Box<dyn Source + Send>> {
        let Some(slot) = usize::try_from(stream.0.index)
            .ok()
            .and_then(|index| self.streams.get_mut(index))
        else {
            return Some(source);
        };
        let old = slot.source.take();
        slot.hold(source);
        slot.generation = stream.0.generation;
        old
    }

    /// Takes `stream` out of its slot, ending what is playing it, for the caller to
    /// free where it may. A mix never frees one.
    pub fn take_stream(&mut self, stream: StreamId) -> Option<Box<dyn Source + Send>> {
        let index = usize::try_from(stream.0.index).ok()?;
        if self.streams.get(index)?.generation != stream.0.generation {
            return None;
        }
        for voice in &mut self.voices {
            if voice.active && voice.origin == Origin::Stream(index) {
                voice.finish(&mut self.streams);
            }
        }
        let slot = &mut self.streams[index];
        slot.retired = false;
        slot.source.take()
    }

    /// Puts `pcm` in the bank.
    pub fn add_sound(&mut self, pcm: SharedPcm) -> Result<SoundId, Full> {
        let index = self.sounds.iter().position(Option::is_none).ok_or(Full)?;
        self.sounds[index] = Some(pcm);
        self.generations[index] = self.generations[index].wrapping_add(1);
        Ok(SoundId(Handle {
            index: u32::try_from(index).map_err(|_| Full)?,
            generation: self.generations[index],
        }))
    }

    /// Takes `sound` out of the bank and stops what is playing it. The caller
    /// frees it, on a thread that may.
    pub fn remove_sound(&mut self, sound: SoundId) -> Option<SharedPcm> {
        let index = self.sound_index(sound)?;
        for voice in &mut self.voices {
            if voice.active && voice.origin == Origin::Sound(index) {
                voice.finish(&mut self.streams);
            }
        }
        self.sounds[index].take()
    }

    fn sound_index(&self, sound: SoundId) -> Option<usize> {
        let index = usize::try_from(sound.0.index).ok()?;
        let live = self.generations.get(index) == Some(&sound.0.generation);
        (live && self.sounds[index].is_some()).then_some(index)
    }

    /// Holds `source` ready to play. A slot that a finished stream still holds is
    /// freed for it; a backend with its mixer on the audio thread takes those out
    /// first ([`take_finished_stream`](Self::take_finished_stream)).
    pub fn add_stream(&mut self, source: Box<dyn Source + Send>) -> Result<StreamId, Full> {
        let index = self
            .streams
            .iter()
            .position(|slot| slot.source.is_none())
            .or_else(|| self.streams.iter().position(|slot| slot.retired))
            .ok_or(Full)?;
        let slot = &mut self.streams[index];
        slot.hold(source);
        Ok(StreamId(Handle {
            index: u32::try_from(index).map_err(|_| Full)?,
            generation: slot.generation,
        }))
    }

    /// A source whose voice has finished with it, for the caller to free where it
    /// may. A mix never frees one.
    pub fn take_finished_stream(&mut self) -> Option<Box<dyn Source + Send>> {
        let slot = self.streams.iter_mut().find(|slot| slot.retired)?;
        slot.retired = false;
        slot.source.take()
    }

    /// A free voice's next id.
    fn free_voice(&self) -> Result<VoiceId, Full> {
        let index = self
            .voices
            .iter()
            .position(|voice| !voice.active)
            .ok_or(Full)?;
        Ok(VoiceId(Handle {
            index: u32::try_from(index).map_err(|_| Full)?,
            generation: self.voices[index].generation.wrapping_add(1),
        }))
    }

    /// Plays `sound`, or says [`Full`]. A sound no longer in the bank plays as one
    /// with no frames.
    pub fn play(&mut self, sound: SoundId, params: Params) -> Result<VoiceId, Full> {
        let voice = self.free_voice()?;
        self.start_sound(voice, sound, params);
        Ok(voice)
    }

    fn start_sound(&mut self, voice: VoiceId, sound: SoundId, params: Params) {
        let (origin, rate) = match self.sound_index(sound) {
            Some(index) => {
                let rate = self.sounds[index].as_ref().map_or(1, |p| p.format().rate);
                (Origin::Sound(index), rate)
            }
            None => (Origin::Nothing, 1),
        };
        self.start_voice(voice, origin, rate, params);
    }

    fn start_voice(&mut self, voice: VoiceId, origin: Origin, rate: u32, params: Params) {
        let output_rate = self.config.rate;
        if let Some(slot) = usize::try_from(voice.0.index)
            .ok()
            .and_then(|index| self.voices.get_mut(index))
        {
            slot.start(voice.0.generation, origin, rate, output_rate, params);
        }
    }

    /// Plays `source`, a stream that has its frames ready, or says [`Full`].
    pub fn stream(
        &mut self,
        source: impl Source + Send + 'static,
        params: Params,
    ) -> Result<VoiceId, Full> {
        self.free_voice()?;
        let stream = self.add_stream(Box::new(source))?;
        self.play_stream(stream, params)
    }

    /// Plays a stream [`add_stream`](Self::add_stream) holds, or says [`Full`]. A
    /// stream plays once: one that was played, or is gone, plays as one with no
    /// frames.
    pub fn play_stream(&mut self, stream: StreamId, params: Params) -> Result<VoiceId, Full> {
        let voice = self.free_voice()?;
        self.start_stream(voice, stream, params);
        Ok(voice)
    }

    fn start_stream(&mut self, voice: VoiceId, stream: StreamId, params: Params) {
        let slot = usize::try_from(stream.0.index)
            .ok()
            .and_then(|index| self.streams.get_mut(index).map(|slot| (index, slot)))
            .filter(|(_, slot)| slot.generation == stream.0.generation && !slot.used);
        let (origin, rate) = match slot {
            Some((index, slot)) => match slot.source.as_ref() {
                Some(source) => {
                    let rate = source.format().rate;
                    slot.used = true;
                    (Origin::Stream(index), rate)
                }
                None => (Origin::Nothing, 1),
            },
            None => (Origin::Nothing, 1),
        };
        self.start_voice(voice, origin, rate, params);
    }

    /// The voice `id` names, if it is still playing.
    fn voice_mut(&mut self, id: VoiceId) -> Option<&mut Voice> {
        let voice = self.voices.get_mut(usize::try_from(id.0.index).ok()?)?;
        (voice.active && voice.generation == id.0.generation).then_some(voice)
    }

    fn voice(&self, id: VoiceId) -> Option<&Voice> {
        let voice = self.voices.get(usize::try_from(id.0.index).ok()?)?;
        (voice.active && voice.generation == id.0.generation).then_some(voice)
    }

    /// Jumps to `volume`, in 1/4096, ending any fade.
    pub fn set_volume(&mut self, voice: VoiceId, volume: u16) {
        if let Some(voice) = self.voice_mut(voice) {
            voice.set_volume(volume);
        }
    }

    /// Goes to `volume`, in 1/4096, in a straight line, over `millis`. A fade to
    /// nothing leaves the voice playing silently, for the caller to stop.
    pub fn fade_to(&mut self, voice: VoiceId, volume: u16, millis: u64) {
        let frames = frames_of_millis(self.config.rate, millis);
        if let Some(voice) = self.voice_mut(voice) {
            voice.fade_to(volume, frames);
        }
    }

    /// Sets the left and right gains, in 1/4096.
    pub fn set_balance(&mut self, voice: VoiceId, left: u16, right: u16) {
        if let Some(voice) = self.voice_mut(voice) {
            voice.set_balance(left, right);
        }
    }

    /// Ends `voice` now.
    pub fn stop(&mut self, voice: VoiceId) {
        let index = self.voice(voice).map(|_| voice.0.index);
        if let Some(voice) = index
            .and_then(|index| usize::try_from(index).ok())
            .and_then(|index| self.voices.get_mut(index))
        {
            voice.finish(&mut self.streams);
        }
    }

    /// Holds `voice` where it is, its fade too: it sounds nothing until
    /// [`resume`](Self::resume).
    pub fn pause(&mut self, voice: VoiceId) {
        if let Some(voice) = self.voice_mut(voice) {
            voice.paused = true;
        }
    }

    /// Goes on from where [`pause`](Self::pause) held it.
    pub fn resume(&mut self, voice: VoiceId) {
        if let Some(voice) = self.voice_mut(voice) {
            voice.paused = false;
        }
    }

    /// Whether `voice` hasn't ended. A paused voice is playing.
    pub fn is_playing(&self, voice: VoiceId) -> bool {
        self.voice(voice).is_some()
    }

    /// Whether `voice` is held by [`pause`](Self::pause).
    pub fn is_paused(&self, voice: VoiceId) -> bool {
        self.voice(voice).is_some_and(|voice| voice.paused)
    }

    /// `voice`'s volume now, in 1/4096, if it is playing.
    pub fn volume(&self, voice: VoiceId) -> Option<u16> {
        self.voice(voice).map(Voice::volume)
    }

    /// How many voices are playing.
    pub fn playing(&self) -> usize {
        self.voices.iter().filter(|voice| voice.active).count()
    }

    /// How many frames, added over every stream so far, had nothing to play yet
    /// because a worker hadn't got there. A voice stops where it is for those: a
    /// stream is delayed, never skipped.
    pub const fn underrun_frames(&self) -> u64 {
        self.underrun_frames
    }

    /// A command from the queue. It can't be refused: the [`Remote`] chose a free
    /// voice.
    fn apply(&mut self, command: Command) {
        match command {
            Command::Play(voice, sound, params) => self.start_sound(voice, sound, params),
            Command::Stream(voice, stream, params) => self.start_stream(voice, stream, params),
            Command::SetVolume(voice, volume) => self.set_volume(voice, volume),
            Command::FadeTo(voice, volume, millis) => self.fade_to(voice, volume, millis),
            Command::SetBalance(voice, left, right) => self.set_balance(voice, left, right),
            Command::Stop(voice) => self.stop(voice),
            Command::Pause(voice) => self.pause(voice),
            Command::Resume(voice) => self.resume(voice),
        }
    }

    /// Mixes every voice into `out`, whole frames of interleaved samples (a part of
    /// a frame at its end is silence), then removes those that finished. No lock, no
    /// allocation, nothing freed.
    pub fn render(&mut self, out: &mut [i16]) {
        for chunk in out.chunks_mut(self.block.len()) {
            let whole = chunk.len() - chunk.len() % self.channels;
            let block = &mut self.block[..whole];
            block.fill(0);
            let mut sources = Sources {
                sounds: &self.sounds,
                streams: &mut self.streams,
            };
            for voice in &mut self.voices {
                if voice.active && !voice.paused {
                    self.underrun_frames += voice.mix(&mut sources, block, self.channels);
                }
            }
            for (sample, mixed) in chunk.iter_mut().zip(block.iter()) {
                *sample = clamp(*mixed);
            }
            chunk[whole..].fill(0);
        }
    }
}

/// How many whole frames play in `millis` at `rate`, at most `u64::MAX`.
fn frames_of_millis(rate: u32, millis: u64) -> u64 {
    let rate = u64::from(rate);
    (millis / 1000)
        .saturating_mul(rate)
        .saturating_add(millis % 1000 * rate / 1000)
}

/// `value` as a sample: past the ends it stays at the end, it never wraps.
fn clamp(value: i32) -> i16 {
    i16::try_from(value.clamp(i32::from(i16::MIN), i32::from(i16::MAX))).unwrap_or(0)
}
