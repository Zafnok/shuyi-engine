//! [`App`]: a whole game behind one call per frame, for the backend's own
//! loop (ADR-0008 §2, ADR-0024). The backend calls [`App::frame`] with its
//! clock; `App` reads the system's events and the input from the
//! [`Platform`], steps the game ([`Game`]: its screens and input state)
//! and its fixed simulation steps ([`TickClock`]), hands the frame's audio
//! to the sound device and the frame to the display.
//!
//! A game starts in a loading phase ([`Start`]): the first frame shows a
//! [`Card`] the game gives, the next loads the game behind it, and the
//! frames after run it. On [`SystemEvent::Suspend`] the game's hook saves
//! and `App` stops stepping; on [`SystemEvent::Resume`] it forgets the
//! time it was away, so a sleep never becomes a burst of catch-up steps.
//!
//! `App` keeps the log (ADR-0030): at the end of every frame it takes the
//! lines the game logged ([`Ctx::log`]), stamps them with the frame's
//! number and keeps the last [`LOG_LINES`] for tests, tools and the
//! backend's sinks ([`App::take_new_log`]).
//!
//! In debug builds `App` also measures each frame (ADR-0046) with the
//! platform's clock ([`Platform::clock`]), and keeps the last
//! [`PERF_FRAMES`] records beside the log ([`App::perf`],
//! [`App::take_new_perf`]). With the platform's memory tags
//! ([`Platform::memory_scopes`]) it tags what each part of a frame
//! allocates: `load` (the game's load), `paint` (presenting the game's
//! frame), `perf` (its record), and `app` for the rest; the game's own
//! frame tags `update`, `draw` and its spans.

use crate::alloc_prelude::*;
use alloc::format;
use core::time::Duration;

use shuyi_ui::audio::{AudioRequest, MusicCommand};
use shuyi_ui::frame::Rgb;
use shuyi_ui::input::frame_seconds;
use shuyi_ui::log::{LOG_LINES, LogCategory, LogFilter, LogLevel, LogLine, LogRing};
use shuyi_ui::perf::{
    FrameRecord, HeapSnapshot, MemoryScopes, PERF_FRAMES, PerfClock, PerfRing, TagClass,
};
use shuyi_ui::storage::Storage;
use shuyi_ui::system_keyboard::TextAnswer;
use shuyi_ui::{Ctx, Game, GameData, IdleMotion, KeyPrompt, Keyboard, RawInputEvent};

use crate::audio::{AudioFrame, SoundRequest};
use crate::platform::{DisplayInfo, Frame, Look, Millis, Platform, Show, SystemEvent};
use crate::text_input::TextInput;
use crate::tick::TickClock;

/// What a frame tells the backend's loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
pub enum FrameOutcome {
    /// Call [`App::frame`] again next frame.
    Continue,
    /// The game has quit (it asked to, or the player closed it). A backend
    /// that can close does; one that can't (a web page) may go on calling
    /// [`App::frame`], which shows the last frame in silence.
    Quit,
}

/// What the screen shows while the game's data loads, from the first frame
/// (a slow device takes seconds to load a game, and shows nothing until a
/// frame is drawn). Drawn by the backend with what it has of its own, since
/// the game's font isn't loaded yet.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Card {
    /// A line of text (the game's name, say), centred, in the backend's
    /// own font.
    Text {
        /// The line.
        line: String,
        /// Its colour.
        ink: Rgb,
        /// The colour of the rest of the screen.
        background: Rgb,
    },
    /// A picture from the game's files, by path, centred at the largest
    /// whole scale that fits.
    Picture {
        /// Its path among the game's files.
        path: String,
        /// The colour of the rest of the screen.
        background: Rgb,
    },
}

/// White, a card's ink until said.
const WHITE: Rgb = Rgb::new(255, 255, 255);
/// Black, a card's background until said.
const BLACK: Rgb = Rgb::new(0, 0, 0);

impl Card {
    /// `line` in white on black.
    pub fn text(line: impl Into<String>) -> Self {
        Self::Text {
            line: line.into(),
            ink: WHITE,
            background: BLACK,
        }
    }

    /// The picture at `path` among the game's files, on black.
    pub fn picture(path: impl Into<String>) -> Self {
        Self::Picture {
            path: path.into(),
            background: BLACK,
        }
    }

    /// The same card in `ink` (a picture has none) on `background`.
    #[must_use]
    pub fn colored(self, ink: Rgb, background: Rgb) -> Self {
        match self {
            Self::Text { line, .. } => Self::Text {
                line,
                ink,
                background,
            },
            Self::Picture { path, .. } => Self::Picture { path, background },
        }
    }

    /// The colour around what it shows.
    pub fn background(&self) -> Rgb {
        match self {
            Self::Text { background, .. } | Self::Picture { background, .. } => *background,
        }
    }
}

/// What the platform gives a game as it loads: what goes into its context
/// ([`ctx`](Self::ctx)). A backend with more to give (the voice manifest it
/// read) adds it with [`Start::before_load`].
#[derive(Debug)]
#[non_exhaustive]
pub struct Setup {
    /// The display, for a game that picks its console to fit it.
    pub display: DisplayInfo,
    /// Where the game's saves and settings persist ([`Platform::storage`]).
    pub storage: Box<dyn Storage>,
    /// A seed from the clock, different each launch: the music's random
    /// picks, and a game's own luck where it wants that.
    pub seed: u64,
    /// The text of the voice manifest the backend read, if it found one.
    pub voice_manifest: Option<String>,
    /// Whether the device has a keyboard: [`Keyboard::Present`] unless the
    /// backend says not.
    pub keyboard: Keyboard,
    /// Whether the game waits for a first key press (browsers block sound
    /// until one): [`KeyPrompt::Waiting`] unless the backend says not.
    pub key_prompt: KeyPrompt,
}

impl Setup {
    /// `ctx` on this platform: its saves and settings in the platform's
    /// storage, the voices the voice manifest allows, idle motion on, the
    /// keyboard and key prompt the backend says, and the music seeded from
    /// the clock so each launch picks differently.
    pub fn ctx<G: GameData>(self, mut ctx: Ctx<G>) -> Ctx<G> {
        ctx.idle_motion = IdleMotion::On;
        ctx.music_seed = self.seed;
        ctx.key_prompt = self.key_prompt;
        ctx.keyboard = self.keyboard;
        let mut ctx = ctx.with_storage(self.storage);
        if let Some(manifest) = &self.voice_manifest {
            ctx.set_voice_manifest(manifest);
        }
        ctx
    }
}

/// A game's load: its data read and its game made from the [`Setup`], or
/// why it can't start (shown instead of the game).
pub type Load<G> = Box<dyn FnOnce(Setup) -> Result<Game<G>, String>>;

/// Where a game's frame finds what it is painted with, in its data.
pub type LookOf<G> = fn(&G) -> Look<'_>;

/// The game's hook for the system's events: it hears each one after `App`
/// has done its part. On [`SystemEvent::Suspend`],
/// [`QuitRequested`](SystemEvent::QuitRequested) and
/// [`LowBattery`](SystemEvent::LowBattery) it is where the game saves what
/// it hasn't; on [`FocusLost`](SystemEvent::FocusLost) it may pause.
pub type SystemHook<G> = fn(&mut Game<G>, SystemEvent);

/// How a game starts: the card shown while it loads, the load, where its
/// look is and its hook for the system's events.
pub struct Start<G: GameData> {
    /// What the first frame shows.
    pub card: Card,
    /// Runs on the second frame, the card on the screen.
    pub load: Load<G>,
    /// What the game's frames are painted with.
    pub look: LookOf<G>,
    /// The game's hook for the system's events, if it has one.
    pub on_system: Option<SystemHook<G>>,
}

impl<G: GameData> Start<G> {
    /// A game that shows `card`, then loads with `load`, painted with what
    /// `look` finds in its data.
    pub fn new(
        card: Card,
        look: LookOf<G>,
        load: impl FnOnce(Setup) -> Result<Game<G>, String> + 'static,
    ) -> Self {
        Self {
            card,
            load: Box::new(load),
            look,
            on_system: None,
        }
    }

    /// The same start with the game's hook for the system's events.
    #[must_use]
    pub fn on_system(mut self, hook: SystemHook<G>) -> Self {
        self.on_system = Some(hook);
        self
    }

    /// The same start with `before` run on the setup ahead of the game's
    /// load: a backend's own part (the voice manifest it read).
    #[must_use]
    pub fn before_load(self, before: impl FnOnce(&mut Setup) + 'static) -> Self {
        let load = self.load;
        Self {
            load: Box::new(move |mut setup| {
                before(&mut setup);
                load(setup)
            }),
            ..self
        }
    }
}

/// Where the game is.
enum Phase<G: GameData> {
    /// Loading: the card, and the load until it has run.
    Loading {
        card: Card,
        load: Option<Load<G>>,
        /// Whether a frame has shown the card.
        shown: bool,
    },
    /// The game runs.
    Running(Box<Game<G>>),
    /// The load failed: why.
    Failed(String),
}

/// The time one frame covers.
#[derive(Debug, Clone, Copy)]
struct Elapsed {
    /// The time on the backend's clock, for the sound device.
    now: Millis,
    /// Seconds since the last frame, for the screens.
    dt: f32,
    /// Fixed simulation steps due.
    ticks: u32,
}

/// `App`'s clock: the backend's time at the last frame, or the driver's
/// own, and the fixed steps owed.
#[derive(Debug, Default)]
struct Clock {
    /// The backend's time at the last frame; `None` before the first and
    /// after a restart, when a frame counts no time.
    last: Option<Millis>,
    /// The time on a driver's own clock ([`App::frame_after`]).
    driven: Duration,
    ticks: TickClock,
}

impl Clock {
    /// A frame at `now` on the backend's clock.
    fn at(&mut self, now: Millis) -> Elapsed {
        let elapsed = self.last.map_or(Duration::ZERO, |last| now.since(last));
        self.last = Some(now);
        Elapsed {
            now,
            dt: elapsed.as_secs_f32(),
            ticks: self.ticks.advance(elapsed),
        }
    }

    /// A frame `seconds` after the last on the driver's own clock.
    fn after(&mut self, seconds: f32) -> Elapsed {
        let dt = frame_seconds(seconds);
        let elapsed = Duration::try_from_secs_f32(dt).unwrap_or(Duration::ZERO);
        self.driven = self.driven.saturating_add(elapsed);
        Elapsed {
            now: Millis(0).after(self.driven),
            dt,
            ticks: self.ticks.advance(elapsed),
        }
    }

    /// Forgets the time since the last frame and the steps owed: the next
    /// frame starts counting again.
    fn restart(&mut self) {
        self.last = None;
        self.ticks.reset();
    }
}

/// What `App` measures in debug builds (ADR-0046): the platform's clock
/// until the game's context takes it, the load's time, this frame's
/// times, and the last frames' records.
struct Perf {
    /// Whether the platform's clock was asked for (on the first frame).
    asked: bool,
    /// The platform's clock, until the game runs and its context takes it.
    clock: Option<PerfClock>,
    /// The platform's memory tags, until the game runs and its context
    /// takes them.
    scopes: Option<MemoryScopes>,
    /// The `app` tag this frame entered, and the scopes it entered with.
    app_tag: Entered,
    /// How long the game's load took.
    load_us: Option<u32>,
    /// This frame's start, while it is measured.
    start: Option<u64>,
    /// This frame's `present` call, once it is measured.
    paint_us: Option<u32>,
    /// The end of the last frame measured; `None` after a frame that
    /// wasn't, so the time away isn't counted as a wait.
    last_end: Option<u64>,
    records: PerfRing,
    /// The sequence number of the first record [`App::take_new_perf`]
    /// hasn't given.
    read: u64,
}

impl Perf {
    fn new() -> Self {
        Self {
            asked: false,
            clock: None,
            scopes: None,
            app_tag: None,
            load_us: None,
            start: None,
            paint_us: None,
            last_end: None,
            records: PerfRing::new(PERF_FRAMES),
            read: 0,
        }
    }

    /// The time on `game`'s clock, in a measured frame only.
    fn now<G: GameData>(&self, game: &mut Game<G>) -> Option<u64> {
        self.start?;
        game.ctx_mut().perf_now()
    }
}

/// A phase tag entered, and the scopes that entered it, to leave it with.
type Entered = Option<(MemoryScopes, u16)>;

/// Enters the phase tag `name`, with `scopes`.
fn enter(scopes: Option<MemoryScopes>, name: &'static str) -> Entered {
    scopes.map(|scopes| (scopes, scopes.enter(TagClass::Phase, name)))
}

/// Leaves the tag [`enter`] entered.
fn leave(entered: Entered) {
    if let Some((scopes, before)) = entered {
        scopes.leave(before);
    }
}

/// The microseconds from `from` to `to`: none if `to` is earlier, and at
/// most `u32::MAX`.
fn micros(from: u64, to: u64) -> u32 {
    u32::try_from(to.saturating_sub(from)).unwrap_or(u32::MAX)
}

/// The lists one frame fills, kept to save allocating them each frame.
struct Lists<V> {
    input: Vec<RawInputEvent>,
    system: Vec<SystemEvent>,
    sounds: Vec<SoundRequest>,
    audio: Vec<AudioRequest<V>>,
    music: Vec<MusicCommand>,
}

/// A game run one frame at a time by a backend's loop.
pub struct App<G: GameData> {
    phase: Phase<G>,
    look: Option<LookOf<G>>,
    on_system: Option<SystemHook<G>>,
    clock: Clock,
    suspended: bool,
    focused: bool,
    quit: bool,
    lists: Lists<G::Voice>,
    /// Frames run: 0 before the first.
    frames: u32,
    /// The last lines logged, stamped with their frame.
    log: LogRing,
    /// What enters `log`.
    log_filter: LogFilter,
    /// The sequence number of the first line [`take_new_log`] hasn't
    /// given.
    ///
    /// [`take_new_log`]: Self::take_new_log
    log_read: u64,
    perf: Perf,
}

impl<G: GameData> App<G> {
    /// A game that starts as `start` says: the card, then the load.
    pub fn new(start: Start<G>) -> Self {
        let Start {
            card,
            load,
            look,
            on_system,
        } = start;
        Self::in_phase(
            Phase::Loading {
                card,
                load: Some(load),
                shown: false,
            },
            Some(look),
            on_system,
        )
    }

    /// `game`, already loaded and running: for a test, which has no look to
    /// paint with ([`Show::Game`]'s `look` is `None`).
    /// What its context logged before is taken, at frame 0.
    pub fn running(game: Game<G>) -> Self {
        let mut app = Self::in_phase(Phase::Running(Box::new(game)), None, None);
        app.take_game_log();
        app
    }

    fn in_phase(
        phase: Phase<G>,
        look: Option<LookOf<G>>,
        on_system: Option<SystemHook<G>>,
    ) -> Self {
        Self {
            phase,
            look,
            on_system,
            clock: Clock::default(),
            suspended: false,
            focused: true,
            quit: false,
            lists: Lists {
                input: Vec::new(),
                system: Vec::new(),
                sounds: Vec::new(),
                audio: Vec::new(),
                music: Vec::new(),
            },
            frames: 0,
            log: LogRing::new(LOG_LINES),
            log_filter: LogFilter::default(),
            log_read: 0,
            perf: Perf::new(),
        }
    }

    /// Gives the game `hook` for the system's events, in place of any it
    /// had.
    pub fn set_system_hook(&mut self, hook: SystemHook<G>) {
        self.on_system = Some(hook);
    }

    /// Runs one frame at `now` on the backend's clock: the system's events,
    /// then the input, the game's update and its fixed steps, the audio,
    /// and the frame to present. The time since the last frame is what the
    /// screens see and the steps are made of; the first frame, and the
    /// first after a load or a [`SystemEvent::Resume`], count none.
    pub fn frame(&mut self, platform: &mut impl Platform, now: Millis) -> FrameOutcome {
        self.begin(platform);
        let elapsed = self.clock.at(now);
        self.end(platform, elapsed)
    }

    /// Runs one frame `seconds` after the last on a clock of the driver's
    /// own, as [`frame`](Self::frame) does otherwise: for a driver whose
    /// frames a clock in whole milliseconds can't give (the test Harness's,
    /// exactly 1/60 s), or a backend whose system says how long the last
    /// frame took. A driver uses this or `frame`, never both.
    pub fn frame_after(&mut self, platform: &mut impl Platform, seconds: f32) -> FrameOutcome {
        self.begin(platform);
        let elapsed = self.clock.after(seconds);
        self.end(platform, elapsed)
    }

    /// A frame's start: its number, its time if it is measured, and the
    /// system's events.
    fn begin(&mut self, platform: &mut impl Platform) {
        self.frames = self.frames.saturating_add(1);
        self.take_clock(platform);
        self.perf.app_tag = enter(self.memory_scopes(), "app");
        self.begin_perf();
        self.system_events(platform);
    }

    /// The rest of a frame, then the lines the game logged in it and its
    /// record.
    fn end(&mut self, platform: &mut impl Platform, elapsed: Elapsed) -> FrameOutcome {
        let outcome = self.run(platform, elapsed);
        self.take_game_log();
        let tagged = enter(self.memory_scopes(), "perf");
        self.end_perf(platform);
        leave(tagged);
        leave(self.perf.app_tag.take());
        outcome
    }

    /// Takes the platform's clock and memory tags on the first frame, in a
    /// build with debug tools, and hands them to a game already running.
    fn take_clock(&mut self, platform: &impl Platform) {
        if core::mem::replace(&mut self.perf.asked, true) {
            return;
        }
        if shuyi_ui::DEBUG_TOOLS {
            self.perf.clock = platform.clock();
            self.perf.scopes = platform.memory_scopes();
        }
        self.give_clock();
    }

    /// Moves the clock and memory tags `App` holds into the running game's
    /// context, each unless that has its own already (a test's).
    fn give_clock(&mut self) {
        let Phase::Running(game) = &mut self.phase else {
            return;
        };
        let ctx = game.ctx_mut();
        if let Some(clock) = self.perf.clock.take()
            && ctx.perf_clock().is_none()
        {
            ctx.set_perf_clock(Some(clock));
        }
        if let Some(scopes) = self.perf.scopes.take()
            && ctx.memory_scopes().is_none()
        {
            ctx.set_memory_scopes(Some(scopes));
        }
    }

    /// The memory tags: the running game's context's, or `App`'s own
    /// before it runs.
    fn memory_scopes(&self) -> Option<MemoryScopes> {
        match &self.phase {
            Phase::Running(game) => game.ctx().memory_scopes(),
            Phase::Loading { .. } | Phase::Failed(_) => self.perf.scopes,
        }
    }

    /// Starts measuring this frame: one of the running game's, not
    /// suspended or quit, with debug tools on and a clock in its context.
    fn begin_perf(&mut self) {
        self.perf.start = None;
        self.perf.paint_us = None;
        if self.suspended || self.quit {
            return;
        }
        if let Phase::Running(game) = &mut self.phase
            && game.ctx().debug_tools
        {
            self.perf.start = game.ctx_mut().begin_perf_frame();
        }
    }

    /// Ends this frame's measuring: its record, if it stepped the game and
    /// presented it, with the platform's memory read at its end.
    fn end_perf(&mut self, platform: &mut impl Platform) {
        let Some(start) = self.perf.start.take() else {
            self.perf.last_end = None;
            return;
        };
        let Phase::Running(game) = &mut self.phase else {
            return;
        };
        let ctx = game.ctx_mut();
        let end = ctx.perf_now();
        let measured = ctx.end_perf_frame();
        let (Some(end), Some(mut record), Some(paint_us)) =
            (end, measured, self.perf.paint_us.take())
        else {
            // Suspended or quit in this frame: no record, and the time
            // away is no wait.
            self.perf.last_end = None;
            return;
        };
        let present_us = self.perf.last_end.map_or(0, |last| micros(last, start));
        record.frame = self.frames;
        record.times.paint_us = paint_us;
        record.times.present_us = present_us;
        record.times.frame_us = present_us.saturating_add(micros(start, end));
        record.heap = platform.memory().map(|memory| memory.heap);
        self.perf.last_end = Some(end);
        self.perf.records.push(record);
    }

    /// Takes the lines the game has logged, stamped with this frame, and
    /// counts those its context dropped.
    fn take_game_log(&mut self) {
        let Phase::Running(game) = &mut self.phase else {
            return;
        };
        let (lines, dropped) = game.ctx_mut().take_log();
        self.log.add_dropped(dropped);
        for line in lines {
            self.log_line(line.level, line.category, line.text);
        }
    }

    /// Keeps a line of this frame's, if the filter lets it in.
    fn log_line(&mut self, level: LogLevel, category: LogCategory, text: String) {
        if self.log_filter.allows(category, level) {
            self.log
                .push(LogLine::new(self.frames, level, category, text));
        }
    }

    /// Reads the system's events and acts on them; a resume restarts the
    /// clock before this frame reads it.
    fn system_events(&mut self, platform: &mut impl Platform) {
        let mut events = core::mem::take(&mut self.lists.system);
        events.clear();
        platform.system_events(&mut events);
        for &event in &events {
            self.on_system_event(event);
        }
        self.lists.system = events;
    }

    fn on_system_event(&mut self, event: SystemEvent) {
        if self.quit {
            return;
        }
        match event {
            SystemEvent::Suspend => self.suspended = true,
            SystemEvent::Resume => {
                self.suspended = false;
                self.clock.restart();
            }
            SystemEvent::FocusLost => self.focused = false,
            SystemEvent::FocusGained => self.focused = true,
            SystemEvent::QuitRequested => self.quit = true,
            SystemEvent::ToggleFullscreen => {
                if let Some(game) = self.game_mut() {
                    game.toggle_fullscreen();
                }
            }
            SystemEvent::LowBattery => {}
        }
        if let (Some(hook), Phase::Running(game)) = (self.on_system, &mut self.phase) {
            hook(game, event);
        }
    }

    fn run(&mut self, platform: &mut impl Platform, elapsed: Elapsed) -> FrameOutcome {
        self.lists.audio.clear();
        self.lists.music.clear();
        if self.suspended && !self.quit {
            // Suspended: nothing steps and nothing is shown.
            drain(platform, &mut self.lists.input);
            return FrameOutcome::Continue;
        }
        let mut loading = None;
        match &mut self.phase {
            Phase::Loading { card, load, shown } => {
                if *shown && !self.quit {
                    loading = load.take();
                } else {
                    *shown = true;
                    platform.present(&Frame::new(Show::Card(card)));
                    drain(platform, &mut self.lists.input);
                }
            }
            Phase::Running(game) if !self.quit => {
                step(platform, game, &mut self.lists, elapsed);
                if game.quit_requested() {
                    platform.audio().stop_all();
                    self.quit = true;
                }
                let painting = self.perf.now(game);
                present_tagged(platform, game, self.look);
                let painted = self.perf.now(game);
                self.perf.paint_us = painting.zip(painted).map(|(from, to)| micros(from, to));
            }
            Phase::Running(game) => {
                // The game has quit: its last frame, in silence.
                drain(platform, &mut self.lists.input);
                present_tagged(platform, game, self.look);
            }
            Phase::Failed(errors) => {
                platform.present(&Frame::new(Show::Errors(errors)));
                drain(platform, &mut self.lists.input);
            }
        }
        if let Some(load) = loading {
            self.load(platform, load, elapsed.now);
        }
        if self.quit {
            FrameOutcome::Quit
        } else {
            FrameOutcome::Continue
        }
    }

    /// Runs the game's load, the card on the screen, and shows the game it
    /// made, or why it couldn't.
    fn load(&mut self, platform: &mut impl Platform, load: Load<G>, now: Millis) {
        let setup = Setup {
            display: platform.display(),
            storage: platform.storage(),
            seed: mix(now.0),
            voice_manifest: None,
            keyboard: Keyboard::Present,
            key_prompt: KeyPrompt::Waiting,
        };
        drain(platform, &mut self.lists.input);
        let tagged = enter(self.perf.scopes, "load");
        let loading = self.perf.clock.as_mut().map(PerfClock::read);
        let loaded = load(setup);
        let done = self.perf.clock.as_mut().map(PerfClock::read);
        leave(tagged);
        self.perf.load_us = loading.zip(done).map(|(from, to)| micros(from, to));
        match loaded {
            Ok(game) => {
                present_game(platform, &game, self.look);
                self.phase = Phase::Running(Box::new(game));
                self.give_clock();
            }
            Err(errors) => {
                let (level, category) = (LogLevel::Error, LogCategory::Platform);
                self.log_line(level, category, format!("the game didn't load: {errors}"));
                platform.present(&Frame::new(Show::Errors(&errors)));
                self.phase = Phase::Failed(errors);
            }
        }
        // The game's time starts now, not before its load.
        self.clock.restart();
    }

    /// The game, once it has loaded.
    pub fn game(&self) -> Option<&Game<G>> {
        match &self.phase {
            Phase::Running(game) => Some(game),
            Phase::Loading { .. } | Phase::Failed(_) => None,
        }
    }

    /// The game, once it has loaded, to change from outside a frame (a
    /// test, a platform's own menu: [`Game::act`]).
    pub fn game_mut(&mut self) -> Option<&mut Game<G>> {
        match &mut self.phase {
            Phase::Running(game) => Some(game),
            Phase::Loading { .. } | Phase::Failed(_) => None,
        }
    }

    /// Ends the run, handing back the game if it loaded.
    pub fn into_game(self) -> Option<Game<G>> {
        match self.phase {
            Phase::Running(game) => Some(*game),
            Phase::Loading { .. } | Phase::Failed(_) => None,
        }
    }

    /// Whether the game hasn't loaded yet.
    pub fn is_loading(&self) -> bool {
        matches!(self.phase, Phase::Loading { .. })
    }

    /// Why the game couldn't start, if its load failed.
    pub fn errors(&self) -> Option<&str> {
        match &self.phase {
            Phase::Failed(errors) => Some(errors),
            Phase::Loading { .. } | Phase::Running(_) => None,
        }
    }

    /// Whether the system has suspended the game.
    pub fn is_suspended(&self) -> bool {
        self.suspended
    }

    /// Whether the game's window has the player's attention (until the
    /// system says otherwise, it has).
    pub fn is_focused(&self) -> bool {
        self.focused
    }

    /// Whether the game has quit.
    pub fn has_quit(&self) -> bool {
        self.quit
    }

    /// The frames run: 0 before the first, then one more for each
    /// [`frame`](Self::frame) or [`frame_after`](Self::frame_after). The
    /// frame the log's lines carry.
    pub fn frame_number(&self) -> u32 {
        self.frames
    }

    /// Which lines the log keeps from now on: [`LogLevel::Info`] and more
    /// serious until set.
    pub fn set_log_filter(&mut self, filter: LogFilter) {
        self.log_filter = filter;
    }

    /// The log: the last [`LOG_LINES`] lines, oldest first, and how many
    /// were dropped.
    pub fn log(&self) -> &LogRing {
        &self.log
    }

    /// The log's lines since the last call, oldest first, for the
    /// backend's sinks: each line is given once.
    pub fn take_new_log(&mut self) -> Vec<LogLine> {
        let lines = self.log.since(self.log_read).cloned().collect();
        self.log_read = self.log.next_seq();
        lines
    }

    /// The last [`PERF_FRAMES`] frames' records, oldest first, and how many
    /// were dropped (ADR-0046). A frame is measured in a build with debug
    /// tools, on a platform with a clock ([`Platform::clock`]), while the
    /// game's [`Ctx::debug_tools`] is on; a loading, suspended, failed or
    /// quit frame makes no record.
    pub fn perf(&self) -> &PerfRing {
        &self.perf.records
    }

    /// The records since the last call, oldest first, for the backend's
    /// sinks: each record is given once.
    pub fn take_new_perf(&mut self) -> Vec<FrameRecord> {
        let records = self.perf.records.since(self.perf.read).cloned().collect();
        self.perf.read = self.perf.records.next_seq();
        records
    }

    /// How long the game's load took, in microseconds, if it was timed: in
    /// a build with debug tools, on a platform with a clock.
    pub fn load_us(&self) -> Option<u32> {
        self.perf.load_us
    }

    /// What holds the heap now, by memory tag (ADR-0046 §6): `platform`'s
    /// snapshot ([`Platform::heap_snapshot`]) stamped with
    /// [`frame_number`](Self::frame_number). Only while debug tools are on:
    /// in a build with them, and the running game's [`Ctx::debug_tools`]
    /// on; `None` otherwise, or where the platform takes none.
    pub fn heap_snapshot(&self, platform: &mut impl Platform) -> Option<HeapSnapshot> {
        let on = match &self.phase {
            Phase::Running(game) => game.ctx().debug_tools,
            Phase::Loading { .. } | Phase::Failed(_) => true,
        };
        if !(shuyi_ui::DEBUG_TOOLS && on) {
            return None;
        }
        let mut snapshot = platform.heap_snapshot()?;
        snapshot.frame = self.frames;
        Some(snapshot)
    }

    /// The audio the screens asked for in the last frame, in order, as they
    /// asked (the device got it as [`SoundRequest`]s).
    pub fn audio(&self) -> &[AudioRequest<G::Voice>] {
        &self.lists.audio
    }

    /// The music commands of the last frame, in order.
    pub fn music(&self) -> &[MusicCommand] {
        &self.lists.music
    }
}

/// Reads the input into `input` and drops it: a frame where nothing takes
/// it.
fn drain(platform: &mut impl Platform, input: &mut Vec<RawInputEvent>) {
    input.clear();
    platform.poll_input(input);
    input.clear();
}

/// One frame of the running game: the input, the system keyboard's
/// answer, the music sounding and the steps due go in; the update runs;
/// its audio goes to the device and a keyboard it asked for opens.
fn step<G: GameData>(
    platform: &mut impl Platform,
    game: &mut Game<G>,
    lists: &mut Lists<G::Voice>,
    elapsed: Elapsed,
) {
    lists.input.clear();
    platform.poll_input(&mut lists.input);
    let keyboard = &mut game.ctx_mut().system_keyboard;
    let text_input = platform.text_input();
    keyboard.set_present(text_input.is_some());
    if keyboard.is_waiting()
        && let Some(answer) = text_input.and_then(TextInput::poll)
    {
        keyboard.answer(answer);
    }
    game.set_music_playing(platform.audio().music_playing(elapsed.now));
    game.set_ticks(elapsed.ticks);
    let out = game.frame(&lists.input, elapsed.dt);
    lists.audio.extend_from_slice(out.audio);
    lists.music.extend_from_slice(out.music);
    let (music_volume, sound_volume, voice_volume) =
        (out.music_volume, out.sound_volume, out.voice_volume);
    lists.sounds.clear();
    let sounds = lists.audio.iter().filter_map(SoundRequest::from_request);
    lists.sounds.extend(sounds);
    let audio = AudioFrame {
        requests: &lists.sounds,
        music: &lists.music,
        music_volume,
        sound_volume,
        voice_volume,
    };
    platform.audio().play(&audio, elapsed.now);
    let keyboard = &mut game.ctx_mut().system_keyboard;
    if let Some(ask) = keyboard.take_ask() {
        let opened = platform.text_input().is_some_and(|input| input.open(&ask));
        if !opened {
            keyboard.answer(TextAnswer::Cancelled);
        }
    }
}

/// Presents `game`'s last frame, on its background, in the window mode
/// its settings ask for.
fn present_game<G: GameData>(
    platform: &mut impl Platform,
    game: &Game<G>,
    look: Option<LookOf<G>>,
) {
    let ctx = game.ctx();
    let show = Show::Game {
        buffer: game.buffer(),
        background: ctx.game.blank_cell(&ctx.palette).bg,
        look: look.map(|look| look(&ctx.game)),
    };
    let mut frame = Frame::new(show);
    frame.fullscreen = ctx.settings().fullscreen;
    platform.present(&frame);
}

/// [`present_game`], under the `paint` tag with the game's memory tags.
fn present_tagged<G: GameData>(
    platform: &mut impl Platform,
    game: &Game<G>,
    look: Option<LookOf<G>>,
) {
    let tagged = enter(game.ctx().memory_scopes(), "paint");
    present_game(platform, game, look);
    leave(tagged);
}

/// `x` mixed (`SplitMix64`'s finalizer), so two launches a millisecond
/// apart get seeds far apart.
fn mix(x: u64) -> u64 {
    let mut z = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

#[cfg(test)]
mod tests;
