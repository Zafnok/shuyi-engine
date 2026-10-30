//! [`HeadlessPlatform`]: a backend with no window, no clock, no heap
//! counter and no memory tags (unless a test hands it them) and no sound
//! card. It plays scripted input and system events, records the frames
//! presented to it, keeps its storage in memory, and stands in for the
//! music player ([`Speaker`]), so a screen that keeps time with the music
//! is tested too.
//!
//! The test Harness runs games on it, through the same
//! [`App::frame`](crate::App::frame) the devices do. It is also the
//! smallest complete backend, for anyone writing one
//! (`docs/platform-backends.md`).

use crate::alloc_prelude::*;
use alloc::collections::VecDeque;
use alloc::vec;

use shuyi_ui::audio::{MusicCommand, SoundId};
use shuyi_ui::frame::Rgb;
use shuyi_ui::frame::glyph_buffer::GlyphBuffer;
use shuyi_ui::frame_cap::FrameCap;
use shuyi_ui::perf::{
    HeapCounts, HeapSnapshot, MemoryReading, MemoryScopes, PerfClock, TagClass, TagRow,
};
use shuyi_ui::storage::{MemoryStorage, Storage};
use shuyi_ui::{PlatformFacts, RawInputEvent};

use crate::app::Card;
use crate::audio::{Audio, AudioFrame, SoundRequest};
use crate::platform::{DisplayInfo, Frame, Millis, Platform, Show, SystemEvent};
use crate::text_input::{TextAnswer, TextAsk, TextInput};

/// A frame as the headless backend keeps it.
#[derive(Debug, Clone, PartialEq)]
pub enum Presented {
    /// The loading card.
    Card(Card),
    /// Why the game couldn't start.
    Errors(String),
    /// The game's console.
    Game {
        /// Its cells and pictures.
        buffer: GlyphBuffer,
        /// The colour behind it.
        background: Rgb,
        /// Whether it asked to fill the screen.
        fullscreen: bool,
    },
}

impl Presented {
    /// The game's console, if this frame shows it.
    pub fn buffer(&self) -> Option<&GlyphBuffer> {
        match self {
            Self::Game { buffer, .. } => Some(buffer),
            Self::Card(_) | Self::Errors(_) => None,
        }
    }

    /// The frame as it was presented, kept: `old`'s buffer is reused when
    /// both show the game.
    fn keep(old: &mut Option<Self>, frame: &Frame<'_>) {
        match (old.as_mut(), frame.show) {
            (
                Some(Self::Game {
                    buffer,
                    background,
                    fullscreen,
                }),
                Show::Game {
                    buffer: new,
                    background: new_background,
                    ..
                },
            ) => {
                if buffer != new {
                    buffer.clone_from(new);
                }
                *background = new_background;
                *fullscreen = frame.fullscreen;
            }
            (
                _,
                Show::Game {
                    buffer, background, ..
                },
            ) => {
                *old = Some(Self::Game {
                    buffer: buffer.clone(),
                    background,
                    fullscreen: frame.fullscreen,
                });
            }
            (_, Show::Card(card)) => *old = Some(Self::Card(card.clone())),
            (_, Show::Errors(errors)) => *old = Some(Self::Errors(errors.to_owned())),
        }
    }
}

/// A sound started with a handle that the [`Speaker`] holds as playing.
/// Only the `Speaker` builds one (ADR-0039).
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct PlayingSound {
    /// Its handle.
    pub id: SoundId,
    /// Its sound cue.
    pub cue: String,
    /// Whether [`SoundRequest::PauseSounds`] paused it.
    pub paused: bool,
}

/// The headless backend's sound device: it records what the last frame
/// played, works out which music "sounds" from the music commands and the
/// clock, and keeps the sounds started with a handle until they stop (one
/// that plays once too: it knows no lengths). Like `shuyi-app`, it mutes
/// rather than pauses: while [`suspended`](Self::suspended) its music
/// clock runs on.
#[derive(Debug, Clone, Default)]
pub struct Speaker {
    /// Seconds a track takes to load: it sounds that long after its
    /// `Start`.
    load_delay: f64,
    /// Whether no track ever sounds (the music files are missing).
    silent: bool,
    /// The track started last, and when.
    started: Option<(String, Millis)>,
    /// The last frame's sound and voice requests.
    requests: Vec<SoundRequest>,
    /// The last frame's music commands.
    music: Vec<MusicCommand>,
    /// The last frame's volumes: music, sound and voice. Full until told.
    volumes: Option<(f32, f32, f32)>,
    /// Whether everything was stopped (the game quit).
    stopped: bool,
    /// The sounds started with a handle and not stopped, in id order.
    playing: Vec<PlayingSound>,
    /// Whether the system suspended the game ([`Audio::suspend`]).
    suspended: bool,
}

impl Speaker {
    /// Makes music take `seconds` to load: a track sounds that long after
    /// the game starts it (at once by default). Negative or NaN counts as
    /// 0.
    pub fn set_load_delay(&mut self, seconds: f64) {
        self.load_delay = seconds.max(0.0);
    }

    /// Makes no music ever sound, as when the music files are missing.
    pub fn set_silent(&mut self) {
        self.silent = true;
    }

    /// The sound and voice requests of the last frame that played any
    /// audio.
    pub fn requests(&self) -> &[SoundRequest] {
        &self.requests
    }

    /// The music commands of the last frame that played audio.
    pub fn music(&self) -> &[MusicCommand] {
        &self.music
    }

    /// The music, sound and voice volumes the last frame played at, 0–1;
    /// full before any.
    pub fn volumes(&self) -> (f32, f32, f32) {
        self.volumes.unwrap_or((1.0, 1.0, 1.0))
    }

    /// Whether the game stopped all its audio (it quit).
    pub fn stopped(&self) -> bool {
        self.stopped
    }

    /// The sounds started with a handle and not yet stopped, in id order.
    pub fn sounds_playing(&self) -> &[PlayingSound] {
        &self.playing
    }

    /// Whether the audio is held because the system suspended the game.
    pub fn suspended(&self) -> bool {
        self.suspended
    }

    /// Follows one sound request: a start adds a sound playing (or
    /// replaces the one of its id, as a new game's queue counts from 1
    /// again), a stop takes it away.
    fn hear(&mut self, request: &SoundRequest) {
        match request {
            SoundRequest::Start { id, cue, .. } => {
                let sound = PlayingSound {
                    id: *id,
                    cue: cue.clone(),
                    paused: false,
                };
                match self.playing.binary_search_by_key(id, |p| p.id) {
                    Ok(at) => self.playing[at] = sound,
                    Err(at) => self.playing.insert(at, sound),
                }
            }
            SoundRequest::Stop { id } => self.playing.retain(|p| p.id != *id),
            SoundRequest::PauseSounds => self.playing.iter_mut().for_each(|p| p.paused = true),
            SoundRequest::ResumeSounds => self.playing.iter_mut().for_each(|p| p.paused = false),
            _ => {}
        }
    }

    /// Follows one music command: a `Start` starts the count at `now`; a
    /// `Stop` (or a fresh `Load`) of that track ends it.
    fn apply(&mut self, command: &MusicCommand, now: Millis) {
        match command {
            MusicCommand::Start { cue, .. } => self.started = Some((cue.clone(), now)),
            MusicCommand::Stop { cue } | MusicCommand::Load { cue } => {
                if self.started.as_ref().is_some_and(|(c, _)| c == cue) {
                    self.started = None;
                }
            }
            MusicCommand::Gain { .. } => {}
        }
    }
}

impl Audio for Speaker {
    fn music_playing(&self, now: Millis) -> Option<(&str, f64)> {
        let (cue, at) = self.started.as_ref().filter(|_| !self.silent)?;
        let elapsed = now.since(*at).as_secs_f64() - self.load_delay;
        (elapsed >= 0.0).then_some((cue.as_str(), elapsed))
    }

    fn play(&mut self, frame: &AudioFrame<'_>, now: Millis) {
        self.volumes = Some((frame.music_volume, frame.sound_volume, frame.voice_volume));
        self.requests.clear();
        self.requests.extend_from_slice(frame.requests);
        for request in frame.requests {
            self.hear(request);
        }
        self.music.clear();
        self.music.extend_from_slice(frame.music);
        for command in frame.music {
            self.apply(command, now);
        }
    }

    fn stop_all(&mut self) {
        self.started = None;
        self.playing.clear();
        self.stopped = true;
    }

    fn suspend(&mut self) {
        self.suspended = true;
    }

    fn resume(&mut self) {
        self.suspended = false;
    }
}

/// A system keyboard that answers from a script: each time it opens it
/// gives the next answer, at the next poll.
#[derive(Debug, Clone, Default)]
pub struct ScriptedKeyboard {
    /// The answers still to give, in order.
    answers: VecDeque<TextAnswer>,
    /// Every ask it was opened with, in order.
    asks: Vec<TextAsk>,
    /// The answer to the ask that is open.
    pending: Option<TextAnswer>,
}

impl ScriptedKeyboard {
    /// A keyboard that answers `answers`, one per opening, then cancels.
    pub fn new(answers: impl IntoIterator<Item = TextAnswer>) -> Self {
        Self {
            answers: answers.into_iter().collect(),
            ..Self::default()
        }
    }

    /// Every ask it was opened with, in order.
    pub fn asks(&self) -> &[TextAsk] {
        &self.asks
    }
}

impl TextInput for ScriptedKeyboard {
    fn open(&mut self, ask: &TextAsk) -> bool {
        if self.pending.is_some() {
            return false;
        }
        self.asks.push(ask.clone());
        self.pending = Some(self.answers.pop_front().unwrap_or(TextAnswer::Cancelled));
        true
    }

    fn poll(&mut self) -> Option<TextAnswer> {
        self.pending.take()
    }
}

/// Where a [`HeadlessPlatform`]'s memory readings come from.
#[derive(Debug, Clone, Copy)]
enum Memory {
    /// It reads none.
    None,
    /// Fixed amounts by the frames presented
    /// ([`HeadlessPlatform::with_fake_memory`]).
    Fake,
    /// A real counter's reading.
    Real(fn() -> HeapCounts),
}

/// The fake memory's reading after `n` frames presented: live bytes
/// 1,048,576 + 16,384 × (*n* mod 30), the peak the most of those so far,
/// and *n* mod 7 allocations.
fn fake_heap(n: u32) -> HeapCounts {
    const BASE: u64 = 1 << 20;
    const STEP: u64 = 16 * 1024;
    let wide = u64::from(n);
    HeapCounts::new(BASE + STEP * (wide % 30), BASE + STEP * wide.min(29), n % 7)
}

/// The fake memory's snapshot after `n` frames presented, its rows adding
/// up to [`fake_heap`]'s live bytes: `image` (an asset kind) 49,152 bytes
/// in 3 blocks, `load` 524,288 in 40, `update` 16,384 × (*n* mod 30) in
/// *n* mod 30, and `other` 475,136 in 12. Each row's peak is the most of
/// its live bytes so far, and its bytes ever allocated its peak; a row
/// with nothing ever allocated (`update` before the first frame) is left
/// out, as a counter leaves it out.
fn fake_snapshot(n: u32) -> HeapSnapshot {
    const STEP: u64 = 16 * 1024;
    let heap = fake_heap(n);
    let row = |class, name: &'static str, live_bytes, live_blocks, peak_bytes| {
        let mut row = TagRow::new(class, name, live_bytes, live_blocks);
        row.allocated_bytes = peak_bytes;
        row.peak_bytes = peak_bytes;
        row
    };
    let fixed = |class, name, bytes, blocks| row(class, name, bytes, blocks, bytes);
    let mut rows = vec![
        fixed(TagClass::Asset, "image", 49_152, 3),
        fixed(TagClass::Phase, "load", 524_288, 40),
        fixed(TagClass::Other, "other", 475_136, 12),
    ];
    if n > 0 {
        let (wide, peak) = (u64::from(n), STEP * u64::from(n.min(29)));
        rows.push(row(
            TagClass::Phase,
            "update",
            STEP * (wide % 30),
            n % 30,
            peak,
        ));
    }
    HeapSnapshot::new(heap.live_bytes, heap.peak_bytes, rows)
}

/// A backend with no devices, for tests and tools.
#[derive(Debug)]
pub struct HeadlessPlatform {
    display: DisplayInfo,
    /// The input of the polls to come, one list per poll.
    input: VecDeque<Vec<RawInputEvent>>,
    /// The system's events for the next poll.
    system: Vec<SystemEvent>,
    /// The storage, until the game takes it.
    storage: Option<Box<dyn Storage>>,
    speaker: Speaker,
    keyboard: Option<ScriptedKeyboard>,
    /// Where the game is said to run.
    facts: PlatformFacts,
    /// The clock it gives to measure frames with, if a test set one.
    clock: Option<PerfClock>,
    /// Where its memory readings come from.
    memory: Memory,
    /// How many times `App` asked for the memory.
    memory_calls: u32,
    /// The memory tags it gives, if a test set them.
    scopes: Option<MemoryScopes>,
    /// Where its heap snapshots come from beside the fake memory: a real
    /// counter's.
    snapshots: Option<fn() -> HeapSnapshot>,
    /// How many frames were presented.
    presented: u32,
    /// The last of them.
    last: Option<Presented>,
    /// The frame cap the last of them asked for, and its serial.
    frame_cap: Option<(FrameCap, u32)>,
}

impl Default for HeadlessPlatform {
    fn default() -> Self {
        Self::new()
    }
}

impl HeadlessPlatform {
    /// The display headless frames are said to be shown on, unless set.
    pub const DISPLAY: DisplayInfo = DisplayInfo::new(1280, 720);

    /// A backend with an empty memory storage, no input waiting and no
    /// system keyboard.
    pub fn new() -> Self {
        Self {
            display: Self::DISPLAY,
            input: VecDeque::new(),
            system: Vec::new(),
            storage: None,
            speaker: Speaker::default(),
            keyboard: None,
            facts: PlatformFacts::default(),
            clock: None,
            memory: Memory::None,
            memory_calls: 0,
            scopes: None,
            snapshots: None,
            presented: 0,
            last: None,
            frame_cap: None,
        }
    }

    /// The same backend on `display`.
    #[must_use]
    pub fn with_display(mut self, display: DisplayInfo) -> Self {
        self.display = display;
        self
    }

    /// The same backend handing the game `storage` (a test's earlier run).
    #[must_use]
    pub fn with_storage(mut self, storage: Box<dyn Storage>) -> Self {
        self.storage = Some(storage);
        self
    }

    /// The same backend saying the game runs where `facts` say (the
    /// `desktop` target's until called): a target's, read from its
    /// profile with [`PlatformFacts::from_profile`].
    #[must_use]
    pub fn with_facts(mut self, facts: PlatformFacts) -> Self {
        self.facts = facts;
        self
    }

    /// The same backend with a system keyboard of its own.
    #[must_use]
    pub fn with_keyboard(mut self, keyboard: ScriptedKeyboard) -> Self {
        self.keyboard = Some(keyboard);
        self
    }

    /// The same backend giving `clock` to measure frames with
    /// ([`Platform::clock`]): a fake one ([`PerfClock::fake`]) makes a
    /// test's records the same on every run.
    #[must_use]
    pub fn with_clock(mut self, clock: PerfClock) -> Self {
        self.clock = Some(clock);
        self
    }

    /// Gives `clock` to measure frames with from now on, or none. `App`
    /// asks for it on its first frame only.
    pub fn set_clock(&mut self, clock: Option<PerfClock>) {
        self.clock = clock;
    }

    /// The same backend reading a fake heap ([`Platform::memory`]) that
    /// moves by fixed amounts, so a test's records are the same on every
    /// run: with *n* the frames it has presented, live bytes 1,048,576 +
    /// 16,384 × (*n* mod 30), the peak the most of those so far, and *n*
    /// mod 7 allocations.
    ///
    /// It takes fake heap snapshots too ([`Platform::heap_snapshot`]),
    /// whose rows add up to those live bytes: `Asset`/`image` 49,152 bytes
    /// in 3 blocks; `Phase`/`load` 524,288 in 40; `Phase`/`update` 16,384 ×
    /// (*n* mod 30) in *n* mod 30, left out before the first frame; and
    /// `Other`/`other` 475,136 in 12. Each row's peak is the most of its
    /// live bytes so far, and its bytes ever allocated equal its peak.
    #[must_use]
    pub fn with_fake_memory(mut self) -> Self {
        self.memory = Memory::Fake;
        self
    }

    /// Reads the heap with `memory` from now on (a real counter's
    /// reading), or reads none; the fake memory is dropped either way.
    pub fn set_memory(&mut self, memory: Option<fn() -> HeapCounts>) {
        self.memory = memory.map_or(Memory::None, Memory::Real);
    }

    /// Takes heap snapshots with `snapshot` from now on (a real counter's),
    /// or takes none; the fake memory's snapshots come first.
    pub fn set_heap_snapshots(&mut self, snapshot: Option<fn() -> HeapSnapshot>) {
        self.snapshots = snapshot;
    }

    /// Whether it reads the heap: a fake, or a counter set.
    pub fn has_memory(&self) -> bool {
        !matches!(self.memory, Memory::None)
    }

    /// How many times `App` asked for the memory ([`Platform::memory`]).
    pub fn memory_calls(&self) -> u32 {
        self.memory_calls
    }

    /// The same backend giving `scopes` as its memory tags
    /// ([`Platform::memory_scopes`]): a test's fake ones, which note what
    /// is entered.
    #[must_use]
    pub fn with_scopes(mut self, scopes: MemoryScopes) -> Self {
        self.scopes = Some(scopes);
        self
    }

    /// Gives `scopes` as its memory tags from now on, or none. `App` asks
    /// for them on its first frame only.
    pub fn set_scopes(&mut self, scopes: Option<MemoryScopes>) {
        self.scopes = scopes;
    }

    /// Queues `events` as the input of one poll, after those already
    /// queued.
    pub fn push_input(&mut self, events: &[RawInputEvent]) {
        self.input.push_back(events.to_vec());
    }

    /// Queues `event` for the next poll of the system's events.
    pub fn push_system(&mut self, event: SystemEvent) {
        self.system.push(event);
    }

    /// How many frames were presented.
    pub fn presented(&self) -> u32 {
        self.presented
    }

    /// The last frame presented.
    pub fn last_frame(&self) -> Option<&Presented> {
        self.last.as_ref()
    }

    /// The frame cap the last frame presented asked for, and its serial:
    /// the headless backend doesn't pace, and reports nothing.
    pub fn frame_cap(&self) -> Option<(FrameCap, u32)> {
        self.frame_cap
    }

    /// The sound device.
    pub fn speaker(&self) -> &Speaker {
        &self.speaker
    }

    /// The sound device, to set how the music loads.
    pub fn speaker_mut(&mut self) -> &mut Speaker {
        &mut self.speaker
    }

    /// The system keyboard, if it has one.
    pub fn keyboard(&self) -> Option<&ScriptedKeyboard> {
        self.keyboard.as_ref()
    }
}

impl Platform for HeadlessPlatform {
    fn display(&self) -> DisplayInfo {
        self.display
    }

    fn present(&mut self, frame: &Frame<'_>) {
        self.presented += 1;
        self.frame_cap = Some((frame.frame_cap, frame.frame_cap_serial));
        Presented::keep(&mut self.last, frame);
    }

    fn poll_input(&mut self, out: &mut Vec<RawInputEvent>) {
        if let Some(events) = self.input.pop_front() {
            out.extend(events);
        }
    }

    fn system_events(&mut self, out: &mut Vec<SystemEvent>) {
        out.append(&mut self.system);
    }

    fn storage(&mut self) -> Box<dyn Storage> {
        self.storage
            .take()
            .unwrap_or_else(|| Box::new(MemoryStorage::new()))
    }

    fn audio(&mut self) -> &mut dyn Audio {
        &mut self.speaker
    }

    fn text_input(&mut self) -> Option<&mut dyn TextInput> {
        self.keyboard.as_mut().map(|k| k as &mut dyn TextInput)
    }

    fn facts(&self) -> PlatformFacts {
        self.facts.clone()
    }

    fn clock(&self) -> Option<PerfClock> {
        self.clock
    }

    fn memory_scopes(&self) -> Option<MemoryScopes> {
        self.scopes
    }

    fn memory(&mut self) -> Option<MemoryReading> {
        self.memory_calls = self.memory_calls.saturating_add(1);
        let heap = match self.memory {
            Memory::None => return None,
            Memory::Fake => fake_heap(self.presented),
            Memory::Real(read) => read(),
        };
        Some(MemoryReading::new(heap))
    }

    fn heap_snapshot(&mut self) -> Option<HeapSnapshot> {
        match self.memory {
            Memory::Fake => Some(fake_snapshot(self.presented)),
            Memory::None | Memory::Real(_) => self.snapshots.map(|snapshot| snapshot()),
        }
    }
}

#[cfg(test)]
mod tests;
