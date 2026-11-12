//! Commands from another thread: a fixed ring of plain data, one writer and one
//! reader, with no lock, no allocation and no `unsafe`.
//!
//! A ring's slots are `AtomicU32`s and a command is written as words, so nothing
//! has to be shared but atomics. The writer stores a slot's words and then the head
//! with `Release`, the reader loads the head with `Acquire` and then the words, and
//! the same the other way for the tail. Only loads and stores: they are what the
//! smallest targets have. The writer may hold slots back, written but not yet in the
//! head, and publish them together ([`Remote::hold`], ticket 3089).
//!
//! [`Mixer::remote`] makes the two ends. The [`Remote`] on the game's side decides
//! which voice a play goes to, so it can answer [`Full`] on the spot, and the
//! [`Inbox`] on the audio thread applies what came and says which voices ended.
//!
//! A [`Pcm`](crate::Pcm) or a source can't go through as words: a backend gives them
//! to the mixer ([`Mixer::add_sound`], [`Mixer::add_stream`]) before the mixer goes
//! to the audio thread, or across by a route of its own.

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU32, Ordering};

use super::{Full, Handle, Mixer, Params, Region, Shared, SoundId, StreamId, VoiceId};

/// The words of a command.
const COMMAND_WORDS: usize = 12;
/// The words of an ended voice.
const EVENT_WORDS: usize = 2;

/// A fixed ring of slots of `width` words, for one writer and one reader.
#[derive(Debug)]
pub(super) struct Ring {
    words: Box<[AtomicU32]>,
    width: usize,
    /// A power of two less one: a slot's index from a counter that runs on.
    mask: u32,
    /// How many slots the reader may read. Only the writer stores it.
    head: AtomicU32,
    /// How many slots were ever written: `head` and those held back. Only the writer
    /// touches it.
    written: AtomicU32,
    /// How many slots were ever read. Only the reader stores it.
    tail: AtomicU32,
}

impl Ring {
    /// A ring of at least `slots` slots, up to 2^31.
    pub(super) fn new(slots: usize, width: usize) -> Self {
        let slots = u32::try_from(slots)
            .unwrap_or(1 << 31)
            .clamp(1, 1 << 31)
            .next_power_of_two();
        let count = usize::try_from(slots).unwrap_or(0) * width;
        Self {
            words: (0..count).map(|_| AtomicU32::new(0)).collect(),
            width,
            mask: slots - 1,
            head: AtomicU32::new(0),
            written: AtomicU32::new(0),
            tail: AtomicU32::new(0),
        }
    }

    /// How many slots it has.
    fn capacity(&self) -> u32 {
        self.mask.wrapping_add(1)
    }

    /// How many slots are free, as the writer sees it.
    pub(super) fn room(&self) -> usize {
        let used = self
            .written
            .load(Ordering::Relaxed)
            .wrapping_sub(self.tail.load(Ordering::Acquire));
        usize::try_from(self.capacity().wrapping_sub(used)).unwrap_or(0)
    }

    /// The index of word `word` of the slot at counter `at`.
    fn index(&self, at: u32, word: usize) -> usize {
        usize::try_from(at & self.mask).unwrap_or(0) * self.width + word
    }

    /// Writes one slot, if one is free, and publishes it with every slot held back
    /// before it unless `held`.
    fn push(&self, slot: &[u32], held: bool) -> bool {
        if self.room() == 0 {
            return false;
        }
        let at = self.written.load(Ordering::Relaxed);
        for (word, value) in slot.iter().enumerate() {
            self.words[self.index(at, word)].store(*value, Ordering::Relaxed);
        }
        self.written.store(at.wrapping_add(1), Ordering::Relaxed);
        if !held {
            self.publish();
        }
        true
    }

    /// Lets the reader read every slot written.
    fn publish(&self) {
        self.head
            .store(self.written.load(Ordering::Relaxed), Ordering::Release);
    }

    /// Reads one slot, if one is there.
    fn pop(&self, slot: &mut [u32]) -> bool {
        let tail = self.tail.load(Ordering::Relaxed);
        if self.head.load(Ordering::Acquire) == tail {
            return false;
        }
        for (word, value) in slot.iter_mut().enumerate() {
            *value = self.words[self.index(tail, word)].load(Ordering::Relaxed);
        }
        self.tail.store(tail.wrapping_add(1), Ordering::Release);
        true
    }

    /// Writes `count` one-word slots, each from `word`; the caller checked the
    /// room. They appear to the reader together.
    pub(super) fn write_words(&self, count: usize, mut word: impl FnMut(usize) -> u32) {
        let mut at = self.written.load(Ordering::Relaxed);
        for slot in 0..count {
            self.words[self.index(at, 0)].store(word(slot), Ordering::Relaxed);
            at = at.wrapping_add(1);
        }
        self.written.store(at, Ordering::Relaxed);
        self.publish();
    }

    /// Reads up to `count` one-word slots, handing each to `word`, and says how
    /// many.
    pub(super) fn read_words(&self, count: usize, mut word: impl FnMut(usize, u32)) -> usize {
        let tail = self.tail.load(Ordering::Relaxed);
        let ready = self.head.load(Ordering::Acquire).wrapping_sub(tail);
        let frames = usize::try_from(ready).map_or(count, |ready| ready.min(count));
        let mut at = tail;
        for slot in 0..frames {
            word(slot, self.words[self.index(at, 0)].load(Ordering::Relaxed));
            at = at.wrapping_add(1);
        }
        self.tail.store(at, Ordering::Release);
        frames
    }
}

/// What the mixer can be told, as plain data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Command {
    Play(VoiceId, SoundId, Params),
    Stream(VoiceId, StreamId, Params),
    SetVolume(VoiceId, u16),
    FadeTo(VoiceId, u16, u64),
    SetBalance(VoiceId, u16, u16),
    Stop(VoiceId),
    Pause(VoiceId),
    Resume(VoiceId),
}

fn pair(low: u16, high: u16) -> u32 {
    u32::from(low) + (u32::from(high) << 16)
}

fn low(word: u32) -> u16 {
    u16::try_from(word & 0xFFFF).unwrap_or(0)
}

fn high(word: u32) -> u16 {
    u16::try_from(word >> 16).unwrap_or(0)
}

fn split(value: u64) -> (u32, u32) {
    (
        u32::try_from(value & 0xFFFF_FFFF).unwrap_or(0),
        u32::try_from(value >> 32).unwrap_or(0),
    )
}

fn join(low: u32, high: u32) -> u64 {
    u64::from(low) + (u64::from(high) << 32)
}

/// What a play's words say past the voice.
fn play_words(words: &mut [u32; COMMAND_WORDS], index: Handle, params: Params) {
    words[3] = index.index;
    words[4] = index.generation;
    words[5] = u32::from(params.volume) + (u32::from(params.looped) << 16);
    words[6] = params.rate;
    words[7] = pair(params.balance.0, params.balance.1);
    if let Some(region) = params.region {
        (words[8], words[9]) = split(region.start());
        (words[10], words[11]) = split(region.end());
    }
}

fn play_params(words: &[u32; COMMAND_WORDS]) -> Params {
    // No region leaves its words at nothing, which is no region.
    let region = Region::new(join(words[8], words[9]), join(words[10], words[11]));
    let mut params = Params::new()
        .with_volume(low(words[5]))
        .with_rate(words[6])
        .with_balance(low(words[7]), high(words[7]));
    params.looped = words[5] & (1 << 16) != 0;
    params.region = region;
    params
}

impl Command {
    /// As a ring's words.
    fn encode(self) -> [u32; COMMAND_WORDS] {
        let mut words = [0; COMMAND_WORDS];
        let voice = match self {
            Self::Play(voice, sound, params) => {
                words[0] = 0;
                play_words(&mut words, sound.0, params);
                voice
            }
            Self::Stream(voice, stream, params) => {
                words[0] = 1;
                play_words(&mut words, stream.0, params);
                voice
            }
            Self::SetVolume(voice, volume) => {
                words[0] = 2;
                words[3] = u32::from(volume);
                voice
            }
            Self::FadeTo(voice, volume, millis) => {
                words[0] = 3;
                words[3] = u32::from(volume);
                (words[4], words[5]) = split(millis);
                voice
            }
            Self::SetBalance(voice, left, right) => {
                words[0] = 4;
                words[3] = pair(left, right);
                voice
            }
            Self::Stop(voice) => {
                words[0] = 5;
                voice
            }
            Self::Pause(voice) => {
                words[0] = 6;
                voice
            }
            Self::Resume(voice) => {
                words[0] = 7;
                voice
            }
        };
        words[1] = voice.0.index;
        words[2] = voice.0.generation;
        words
    }

    /// From a ring's words, which [`encode`](Self::encode) wrote.
    fn decode(words: &[u32; COMMAND_WORDS]) -> Option<Self> {
        let voice = VoiceId(Handle {
            index: words[1],
            generation: words[2],
        });
        let handle = Handle {
            index: words[3],
            generation: words[4],
        };
        Some(match words[0] {
            0 => Self::Play(voice, SoundId(handle), play_params(words)),
            1 => Self::Stream(voice, StreamId(handle), play_params(words)),
            2 => Self::SetVolume(voice, low(words[3])),
            3 => Self::FadeTo(voice, low(words[3]), join(words[4], words[5])),
            4 => Self::SetBalance(voice, low(words[3]), high(words[3])),
            5 => Self::Stop(voice),
            6 => Self::Pause(voice),
            7 => Self::Resume(voice),
            _ => return None,
        })
    }
}

/// Makes the two ends for `voices` voices, with the bank and the stream slots as
/// they are: each one's generation and whether it is taken.
pub(super) fn ends(
    voices: usize,
    sounds: Vec<(u32, bool)>,
    streams: Vec<(u32, bool)>,
    commands: usize,
) -> (Remote, Inbox) {
    let commands = Shared::new(Ring::new(commands, COMMAND_WORDS));
    let ended = Shared::new(Ring::new(voices.max(1), EVENT_WORDS));
    (
        Remote {
            commands: Shared::clone(&commands),
            ended: Shared::clone(&ended),
            voices: alloc::vec![(0, false); voices],
            sounds,
            streams,
            held: false,
        },
        Inbox { commands, ended },
    )
}

/// The next tenant of a free place of `table`, which is taken from now on.
fn reserve(table: &mut [(u32, bool)]) -> Result<Handle, Full> {
    let index = table.iter().position(|place| !place.1).ok_or(Full)?;
    let generation = table[index].0.wrapping_add(1);
    table[index] = (generation, true);
    Ok(Handle {
        index: u32::try_from(index).map_err(|_| Full)?,
        generation,
    })
}

/// Frees `handle`'s place of `table`, if it is still its tenant's.
fn release(table: &mut [(u32, bool)], handle: Handle) {
    if let Some(place) = usize::try_from(handle.index)
        .ok()
        .and_then(|index| table.get_mut(index))
        .filter(|place| place.0 == handle.generation)
    {
        place.1 = false;
    }
}

/// The mixer as a game's thread sees it, when the mixer is on the audio thread:
/// the same operations, none of which waits.
///
/// An operation is refused with [`Full`] when the queue has no room for it. Sized
/// for what a frame sends, that doesn't happen.
#[derive(Debug)]
pub struct Remote {
    commands: Shared<Ring>,
    ended: Shared<Ring>,
    /// Each voice's generation, and whether it is busy.
    voices: Vec<(u32, bool)>,
    /// The same of the bank's places and the stream slots, which this end hands out.
    sounds: Vec<(u32, bool)>,
    streams: Vec<(u32, bool)>,
    /// Whether what is sent is held back until [`publish`](Self::publish).
    held: bool,
}

impl Remote {
    /// Hears which voices ended.
    fn hear(&mut self) {
        let mut slot = [0; EVENT_WORDS];
        while self.ended.pop(&mut slot) {
            let voice = usize::try_from(slot[0]).ok();
            if let Some(entry) = voice.and_then(|voice| self.voices.get_mut(voice))
                && entry.0 == slot[1]
            {
                entry.1 = false;
            }
        }
    }

    fn send(&self, command: Command) -> Result<(), Full> {
        if self.commands.push(&command.encode(), self.held) {
            Ok(())
        } else {
            Err(Full)
        }
    }

    /// Holds back what is sent from here, until [`publish`](Self::publish): the
    /// audio thread may apply commands between any two that are sent, so the plays
    /// of one frame would start in different blocks, a few milliseconds apart, and
    /// copies meant to sound as one wouldn't (ticket 3089). What is held takes room
    /// in the queue as before.
    pub fn hold(&mut self) {
        self.held = true;
    }

    /// Sends everything held back since [`hold`](Self::hold) at once, and sends at
    /// once from here on: the audio thread applies it all before one block.
    pub fn publish(&mut self) {
        self.held = false;
        self.commands.publish();
    }

    /// A free voice's next id, the voice left free until the play has gone.
    fn free_voice(&mut self) -> Result<VoiceId, Full> {
        self.hear();
        let index = self.voices.iter().position(|voice| !voice.1).ok_or(Full)?;
        Ok(VoiceId(Handle {
            index: u32::try_from(index).map_err(|_| Full)?,
            generation: self.voices[index].0.wrapping_add(1),
        }))
    }

    /// Marks `voice` busy.
    fn taken(&mut self, voice: VoiceId) {
        if let Some(entry) = usize::try_from(voice.0.index)
            .ok()
            .and_then(|index| self.voices.get_mut(index))
        {
            *entry = (voice.0.generation, true);
        }
    }

    /// A place in the bank, named now, so that a play can follow at once: a backend
    /// hands the sound over to [`Mixer::put_sound`] by a route of its own, ahead of
    /// that play. [`Full`] says the bank has none free.
    pub fn reserve_sound(&mut self) -> Result<SoundId, Full> {
        reserve(&mut self.sounds).map(SoundId)
    }

    /// Frees a place [`reserve_sound`](Self::reserve_sound) named.
    pub fn release_sound(&mut self, sound: SoundId) {
        release(&mut self.sounds, sound.0);
    }

    /// A stream slot, named now, as [`reserve_sound`](Self::reserve_sound) names a
    /// place in the bank, for [`Mixer::put_stream`].
    pub fn reserve_stream(&mut self) -> Result<StreamId, Full> {
        reserve(&mut self.streams).map(StreamId)
    }

    /// Frees a slot [`reserve_stream`](Self::reserve_stream) named.
    pub fn release_stream(&mut self, stream: StreamId) {
        release(&mut self.streams, stream.0);
    }

    /// [`Mixer::play`], from here.
    pub fn play(&mut self, sound: SoundId, params: Params) -> Result<VoiceId, Full> {
        let voice = self.free_voice()?;
        self.send(Command::Play(voice, sound, params))?;
        self.taken(voice);
        Ok(voice)
    }

    /// [`Mixer::play_stream`], from here.
    pub fn play_stream(&mut self, stream: StreamId, params: Params) -> Result<VoiceId, Full> {
        let voice = self.free_voice()?;
        self.send(Command::Stream(voice, stream, params))?;
        self.taken(voice);
        Ok(voice)
    }

    /// [`Mixer::set_volume`], from here.
    pub fn set_volume(&self, voice: VoiceId, volume: u16) -> Result<(), Full> {
        self.send(Command::SetVolume(voice, volume))
    }

    /// [`Mixer::fade_to`], from here.
    pub fn fade_to(&self, voice: VoiceId, volume: u16, millis: u64) -> Result<(), Full> {
        self.send(Command::FadeTo(voice, volume, millis))
    }

    /// [`Mixer::set_balance`], from here.
    pub fn set_balance(&self, voice: VoiceId, left: u16, right: u16) -> Result<(), Full> {
        self.send(Command::SetBalance(voice, left, right))
    }

    /// [`Mixer::stop`], from here.
    pub fn stop(&self, voice: VoiceId) -> Result<(), Full> {
        self.send(Command::Stop(voice))
    }

    /// [`Mixer::pause`], from here.
    pub fn pause(&self, voice: VoiceId) -> Result<(), Full> {
        self.send(Command::Pause(voice))
    }

    /// [`Mixer::resume`], from here.
    pub fn resume(&self, voice: VoiceId) -> Result<(), Full> {
        self.send(Command::Resume(voice))
    }

    /// Whether `voice` is playing, as far as the audio thread has said: it goes
    /// on saying so until the mixer has reported the voice over.
    pub fn is_playing(&mut self, voice: VoiceId) -> bool {
        self.hear();
        usize::try_from(voice.0.index)
            .ok()
            .and_then(|index| self.voices.get(index))
            .is_some_and(|entry| *entry == (voice.0.generation, true))
    }
}

/// What the audio thread calls before each [`Mixer::render`].
#[derive(Debug)]
pub struct Inbox {
    commands: Shared<Ring>,
    ended: Shared<Ring>,
}

impl Inbox {
    /// Applies every command that has come, and says which voices ended. It takes
    /// no lock and allocates nothing.
    pub fn apply(&mut self, mixer: &mut Mixer) {
        self.apply_with(mixer, |_| {});
    }

    /// [`apply`](Self::apply), calling `before` ahead of each command. A backend's
    /// own hand-over goes there: whatever was handed over before a command was sent
    /// has arrived once that command has, so a play always finds its sound.
    pub fn apply_with(&mut self, mixer: &mut Mixer, mut before: impl FnMut(&mut Mixer)) {
        let mut slot = [0; COMMAND_WORDS];
        while self.commands.pop(&mut slot) {
            before(mixer);
            if let Some(command) = Command::decode(&slot) {
                mixer.apply(command);
            }
        }
        for (index, voice) in (0u32..).zip(mixer.voices.iter_mut()) {
            // Room for every voice at once, so the next pass is only a precaution.
            if voice.ended && self.ended.push(&[index, voice.generation], false) {
                voice.ended = false;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn a_ring_is_a_power_of_two_and_refuses_when_full() {
        let ring = Ring::new(3, 2);
        assert_eq!(ring.room(), 4);
        for n in 0..4 {
            assert!(ring.push(&[n, n + 10], false));
        }
        assert_eq!(ring.room(), 0);
        assert!(!ring.push(&[9, 9], false));
        let mut slot = [0; 2];
        for n in 0..4 {
            assert!(ring.pop(&mut slot));
            assert_eq!(slot, [n, n + 10]);
        }
        assert!(!ring.pop(&mut slot));
        assert_eq!(ring.room(), 4);
    }

    #[test]
    fn a_ring_of_nothing_still_holds_one() {
        let ring = Ring::new(0, 1);
        assert_eq!(ring.room(), 1);
    }

    #[test]
    fn a_rings_counters_wrap_past_their_width() {
        let ring = Ring::new(4, 1);
        ring.head.store(u32::MAX - 1, Ordering::Relaxed);
        ring.written.store(u32::MAX - 1, Ordering::Relaxed);
        ring.tail.store(u32::MAX - 1, Ordering::Relaxed);
        let mut slot = [0; 1];
        for n in 0..16 {
            assert!(ring.push(&[n], false), "{n}");
            assert!(ring.push(&[n + 100], false), "{n}");
            assert_eq!(ring.room(), 2);
            assert!(ring.pop(&mut slot));
            assert_eq!(slot[0], n);
            assert!(ring.pop(&mut slot));
            assert_eq!(slot[0], n + 100);
            assert_eq!(ring.room(), 4);
        }
    }

    #[test]
    fn words_in_bulk_come_out_as_they_went_in() {
        let ring = Ring::new(4, 1);
        ring.write_words(3, |n| u32::try_from(n).unwrap_or(0) * 2);
        let mut seen = Vec::new();
        assert_eq!(ring.read_words(2, |n, word| seen.push((n, word))), 2);
        assert_eq!(seen, [(0, 0), (1, 2)]);
        assert_eq!(ring.room(), 3);
        seen.clear();
        assert_eq!(ring.read_words(9, |n, word| seen.push((n, word))), 1);
        assert_eq!(seen, [(0, 4)]);
        assert_eq!(ring.read_words(9, |_, _| panic!("nothing left")), 0);
    }

    fn handle(index: u32, generation: u32) -> Handle {
        Handle { index, generation }
    }

    prop_compose! {
        fn params()(
            volume: u16, looped: bool, rate: u32, left: u16, right: u16,
            region: Option<(u64, u64)>,
        ) -> Params {
            let mut params = Params::new()
                .with_volume(volume)
                .with_rate(rate)
                .with_balance(left, right);
            params.looped = looped;
            params.region = region.and_then(|(a, b)| Region::new(a.min(b), a.max(b)));
            params
        }
    }

    fn command() -> impl Strategy<Value = Command> {
        let voice = (any::<u32>(), any::<u32>()).prop_map(|(i, g)| VoiceId(handle(i, g)));
        let id = (any::<u32>(), any::<u32>()).prop_map(|(i, g)| handle(i, g));
        prop_oneof![
            (voice.clone(), id.clone(), params()).prop_map(|(v, i, p)| Command::Play(
                v,
                SoundId(i),
                p
            )),
            (voice.clone(), id, params()).prop_map(|(v, i, p)| Command::Stream(v, StreamId(i), p)),
            (voice.clone(), any::<u16>()).prop_map(|(v, n)| Command::SetVolume(v, n)),
            (voice.clone(), any::<u16>(), any::<u64>())
                .prop_map(|(v, n, m)| Command::FadeTo(v, n, m)),
            (voice.clone(), any::<u16>(), any::<u16>())
                .prop_map(|(v, l, r)| Command::SetBalance(v, l, r)),
            voice.clone().prop_map(Command::Stop),
            voice.clone().prop_map(Command::Pause),
            voice.prop_map(Command::Resume),
        ]
    }

    proptest! {
        /// Whatever a command is, its words give it back.
        #[test]
        fn commands_round_trip(command in command()) {
            prop_assert_eq!(Command::decode(&command.encode()), Some(command));
        }
    }

    #[test]
    fn words_that_are_no_command_are_none() {
        assert_eq!(Command::decode(&[8; COMMAND_WORDS]), None);
    }
}
