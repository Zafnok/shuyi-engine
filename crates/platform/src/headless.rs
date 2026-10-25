//! [`HeadlessPlatform`]: a backend with no window, no clock (unless a test
//! hands it one) and no sound card. It plays scripted input and system events, records the frames
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

use shuyi_ui::RawInputEvent;
use shuyi_ui::audio::MusicCommand;
use shuyi_ui::frame::Rgb;
use shuyi_ui::frame::glyph_buffer::GlyphBuffer;
use shuyi_ui::perf::PerfClock;
use shuyi_ui::storage::{MemoryStorage, Storage};

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

/// The headless backend's sound device: it records what the last frame
/// played and works out which music "sounds" from the music commands and
/// the clock.
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

    /// Follows one music command: a `Start` starts the count at `now`; a
    /// `Stop` (or a fresh `Load`) of that track ends it.
    fn apply(&mut self, command: &MusicCommand, now: Millis) {
        match command {
            MusicCommand::Start { cue } => self.started = Some((cue.clone(), now)),
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
        self.music.clear();
        self.music.extend_from_slice(frame.music);
        for command in frame.music {
            self.apply(command, now);
        }
    }

    fn stop_all(&mut self) {
        self.started = None;
        self.stopped = true;
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
    /// The clock it gives to measure frames with, if a test set one.
    clock: Option<PerfClock>,
    /// How many frames were presented.
    presented: u32,
    /// The last of them.
    last: Option<Presented>,
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
            clock: None,
            presented: 0,
            last: None,
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

    fn clock(&self) -> Option<PerfClock> {
        self.clock
    }
}

#[cfg(test)]
mod tests;
