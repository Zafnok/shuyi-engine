//! A game running headless, for the operations that play it (`run`,
//! `scene`) and the MCP server's session ([`crate::mcp`]): [`Session`],
//! which any game gives as a [`Headless`] over its [`Harness`].
//!
//! The engine drives the session and reads it; the game says only how its
//! Harness starts and, if it has them, what its scene is
//! ([`Headless::with_scene`]), what happened in its rules
//! ([`Headless::with_events`]) and how its frame looks
//! ([`Headless::with_picture`]). The modules its game installed are read
//! from the game itself.
//!
//! With perf on ([`Session::set_perf`]), a [`Headless`] measures each
//! frame with this machine's clock and keeps its record, read with
//! [`Session::perf`] (ADR-0046). In a debug build of the tool program
//! each record has the heap's counts too, from the program's own heap
//! counter (`shuyi_alloc_count::global`), which counts the whole program,
//! and the game's frames, spans and asset decodes are tagged in it
//! (`shuyi_alloc_count::tags`): [`Session::heap_snapshot`] takes what holds
//! the heap by tag. Start a session with [`measured_start`] to tag its
//! start, the game's load, `load`.

use std::fs;
use std::path::Path;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use shuyi_alloc_count::{global, tags};
use shuyi_assets::format::{DecodeWatch, Decoded, watch_decodes};
use shuyi_platform::Platform as _;
use shuyi_platform::harness::{FRAME_DT, Harness};
use shuyi_platform::heap;
use shuyi_platform::session::{Played, SessionRecord, Stamps};
use shuyi_ui::GameData;
use shuyi_ui::audio::{AudioRequest, MusicCommand};
use shuyi_ui::log::LogLine;
use shuyi_ui::module::InstalledModule;
use shuyi_ui::perf::{HeapCounts, HeapSnapshot, MemoryScopes, PerfClock, TagClass};

use super::codes::{self, Code};
use super::perf::PerfRead;
use crate::frame_png::{Image, Script};
use crate::game::{GameTools, code_of, unsupported};

/// The scene a stage shows, as text and as data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
// A view a game builds with `Scene::new`: it may grow (ADR-0039).
#[non_exhaustive]
pub struct Scene {
    /// The screen it is on.
    pub screen: String,
    /// What kind of scene: `grid map`, or the game's own word.
    pub kind: String,
    /// The scene as text: for a grid map, its tiles, units and cursor in
    /// the scene's text form (`MapScene::to_text`).
    pub text: String,
    /// What the game says of it as data: its stage's state.
    pub data: serde_json::Value,
}

impl Scene {
    /// The scene of kind `kind` on the screen `screen`, as the text `text`,
    /// with no data (`null`; assign [`data`](Self::data)).
    pub fn new(
        screen: impl Into<String>,
        kind: impl Into<String>,
        text: impl Into<String>,
    ) -> Self {
        Self {
            screen: screen.into(),
            kind: kind.into(),
            text: text.into(),
            data: serde_json::Value::Null,
        }
    }
}

/// Something that happened while a session ran, with its frame (frames
/// count from 1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Event {
    /// A screen opened.
    Opened {
        /// The frame.
        frame: u32,
        /// Its name.
        screen: String,
    },
    /// A screen closed.
    Closed {
        /// The frame.
        frame: u32,
        /// Its name.
        screen: String,
    },
    /// A music command to the platform.
    Music {
        /// The frame.
        frame: u32,
        /// `load`, `start`, `gain` or `stop`.
        command: String,
        /// The track's cue.
        cue: String,
    },
    /// A voiced line asked to play.
    Voice {
        /// The frame.
        frame: u32,
        /// The line's id.
        line: String,
    },
    /// The game asked to quit.
    Quit {
        /// The frame.
        frame: u32,
    },
    /// Something in the game's rules, in the game's words (it gives no
    /// frame).
    Game {
        /// What happened.
        text: String,
    },
}

/// A sound cue asked to play.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Sound {
    /// The frame.
    pub frame: u32,
    /// The cue.
    pub cue: String,
}

/// A line of the game's log.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
// An answer's, read field by field: it may grow (ADR-0039).
#[non_exhaustive]
pub struct LogEntry {
    /// The frame it was given in (frames count from 1; 0 is before the
    /// first).
    pub frame: u32,
    /// How serious it is: `error`, `warn`, `info` or `debug`.
    pub level: String,
    /// What it is about: `assets`, `audio`, `input`, `screens`, `storage`,
    /// `platform` or `game`.
    pub category: String,
    /// What it says, in English.
    pub text: String,
}

impl From<&LogLine> for LogEntry {
    fn from(line: &LogLine) -> Self {
        Self {
            frame: line.frame,
            level: line.level.name().to_owned(),
            category: line.category.name().to_owned(),
            text: line.text.clone(),
        }
    }
}

/// A module installed in the game, with what it declared (ADR-0040).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
// An answer's, read field by field: it may grow (ADR-0039).
#[non_exhaustive]
pub struct ModuleEntry {
    /// Its name.
    pub name: String,
    /// The hooks the game calls on it, in the frame loop: `before_update`
    /// (before the screens' update), `draw_over` (over their drawing).
    pub hooks: Vec<String>,
    /// The types of the state it put on the context.
    pub states: Vec<String>,
    /// Its entries' names on the debug menu, as they read at install, in
    /// the order declared; the menu lists them after the game's.
    pub debug_entries: Vec<String>,
}

impl From<&InstalledModule> for ModuleEntry {
    fn from(module: &InstalledModule) -> Self {
        Self {
            name: module.name.to_owned(),
            hooks: module.hooks.iter().map(|h| h.name().to_owned()).collect(),
            states: module.states.iter().map(|&s| s.to_owned()).collect(),
            debug_entries: module.debug_entries.clone(),
        }
    }
}

/// A session record to play, in place of a game's start options.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
// An input: built with `SessionInput::new` and its fields assigned, it may
// grow (ADR-0039).
#[non_exhaustive]
pub struct SessionInput {
    /// The record's file, from the project's root.
    pub file: String,
    /// Play a record stamped by another build or with other content too
    /// (`--any-stamp`), which is otherwise refused; it may then differ for
    /// that reason alone.
    #[serde(default)]
    pub any_stamp: bool,
}

impl SessionInput {
    /// The record in the file `file`, refused if stamped by another build.
    pub fn new(file: impl Into<String>) -> Self {
        Self {
            file: file.into(),
            any_stamp: false,
        }
    }
}

/// How far a session record played.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
// An answer's, read field by field: it may grow (ADR-0039).
#[non_exhaustive]
pub struct SessionPlayed {
    /// The record's file, as given.
    pub file: String,
    /// The record's frames played.
    pub frames: u32,
    /// The frames the record holds.
    pub of: u32,
    /// The first frame that played differently: the playing stopped there,
    /// and neither the script nor `frames` played.
    pub first_difference: Option<SessionDifference>,
}

/// The first frame of a session record that played differently.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
// An answer's, read field by field: it may grow (ADR-0039).
#[non_exhaustive]
pub struct SessionDifference {
    /// The frame, counted from 1.
    pub frame: u32,
    /// The hash the record holds for it (sixteen hex digits).
    pub recorded: String,
    /// The hash it played with.
    pub played: String,
}

impl SessionPlayed {
    /// `played` of the record in `file`.
    pub(crate) fn of(file: &str, played: &Played) -> Self {
        Self {
            file: file.to_owned(),
            frames: played.frames,
            of: played.of,
            first_difference: played.first_difference.as_ref().map(|difference| {
                SessionDifference {
                    frame: difference.frame,
                    recorded: difference.recorded.to_string(),
                    played: difference.played.to_string(),
                }
            }),
        }
    }
}

/// The session of `game` made ready to play the record `input` names, from
/// `root`, and played: or the problem code and why not. A record another
/// build stamped is refused unless `input` says, by the game's own stamp
/// ([`GameTools::made`]) if it gives one.
pub(crate) fn from_record(
    game: &dyn GameTools,
    root: &Path,
    input: &SessionInput,
) -> Result<(Box<dyn Session>, Played), (Code, String)> {
    let file = &input.file;
    let text =
        fs::read_to_string(root.join(file)).map_err(|e| (codes::FILE, format!("{file}: {e}")))?;
    let record =
        SessionRecord::from_ron(&text).map_err(|e| (codes::FILE, format!("{file}: {e}")))?;
    if !input.any_stamp
        && let (Some(made), Some(now)) = (&record.made, game.made())
        && let Err(mismatch) = made.check(&now)
    {
        let error = format!(
            "{file} was {mismatch}: make it again with this build, or play it with --any-stamp"
        );
        return Err((codes::OTHER_STAMP, error));
    }
    let stamps = if input.any_stamp {
        Stamps::Ignore
    } else {
        Stamps::Check
    };
    let mut session = game
        .session_from(&record, stamps)
        .map_err(|e| (code_of(&e), e))?;
    let played = session.play_record().ok_or_else(|| {
        let error = unsupported("run --session and scene --session", "session_from");
        (codes::NOT_OFFERED, error)
    })?;
    Ok((session, played))
}

/// A game running headless.
pub trait Session {
    /// Plays `script`'s steps, in order.
    fn play(&mut self, script: &Script);

    /// Runs `frames` frames with no input.
    fn idle(&mut self, frames: u32);

    /// Plays the session record it was made ready for
    /// ([`GameTools::session_from`]) to
    /// its end, or its first frame that plays differently, and says how far
    /// it has got. Asked again, it plays nothing more: its limits then say
    /// what a script's frames took over from it. `None` for a session
    /// started from options, the default.
    fn play_record(&mut self) -> Option<Played> {
        None
    }

    /// How many frames have run.
    fn frame_count(&self) -> u32;

    /// The open screens' names, bottom first.
    fn screens(&self) -> Vec<String>;

    /// The frame the game asked to quit in, if it has.
    fn quit_frame(&self) -> Option<u32>;

    /// The current frame in the snapshot text format.
    fn snapshot(&self) -> String;

    /// What happened, in frame order: screens opening and closing, music,
    /// voices, the quit, then the game's own events.
    fn events(&self) -> Vec<Event>;

    /// The sound cues asked for, in order.
    fn sounds(&self) -> Vec<Sound>;

    /// The scene on screen, or why there is none.
    fn scene(&self) -> Result<Scene, String>;

    /// The game's log, oldest first: its last lines (none by default).
    fn log(&self) -> Vec<LogEntry> {
        Vec::new()
    }

    /// How many older lines the log dropped (0 by default).
    fn log_dropped(&self) -> u32 {
        0
    }

    /// The modules installed in the game, in install order (none by
    /// default).
    fn modules(&self) -> Vec<ModuleEntry> {
        Vec::new()
    }

    /// Measures each frame from now on, or stops (`false`). By default a
    /// session measures nothing.
    fn set_perf(&mut self, on: bool) {
        let _ = on;
    }

    /// Whether its records' heap counts are this tool program's own heap
    /// counter's, which counts the whole program (every thread's
    /// allocations), not a fake's. By default not.
    fn counts_process_heap(&self) -> bool {
        false
    }

    /// The frames' records kept, numbered `since` or more, the first `max`
    /// of them ([`PerfRead`]). By default nothing, measured.
    fn perf(&self, since: u64, max: u32) -> PerfRead {
        let _ = max;
        PerfRead::none(since)
    }

    /// What holds the heap now, by memory tag, at the frames run: a fake
    /// memory's, or with perf on ([`set_perf`](Self::set_perf)) this
    /// program's heap counter's, in a build that counts it. By default
    /// none.
    fn heap_snapshot(&mut self) -> Option<HeapSnapshot> {
        None
    }

    /// The current frame as the game paints it, each console pixel
    /// `scale` × `scale` (the game's choice if `None`). By default the
    /// game offers none.
    fn picture(&self, scale: Option<u32>) -> Result<Image, String> {
        let _ = scale;
        Err(unsupported(
            "a picture of a session's frame",
            "session, with Headless::with_picture",
        ))
    }
}

/// What a game says its scene is, from its Harness.
type SceneOf<G> = Box<dyn Fn(&Harness<G>) -> Result<Scene, String>>;

/// How a game paints its Harness's frame, at a scale if one is given.
type PictureOf<G> = Box<dyn Fn(&Harness<G>, Option<u32>) -> Result<Image, String>>;

/// What a game says happened in its rules, from its Harness.
type EventsOf<G> = Box<dyn Fn(&Harness<G>) -> Vec<String>>;

/// A [`Session`] over a game's [`Harness`].
pub struct Headless<G: GameData> {
    harness: Harness<G>,
    scene: SceneOf<G>,
    events: EventsOf<G>,
    picture: Option<PictureOf<G>>,
    /// Whether [`Session::set_perf`] gave the Harness its clock.
    perf_clock: bool,
    /// Whether [`Session::set_perf`] gave the Harness this program's heap
    /// counter.
    perf_memory: bool,
    /// Whether [`Session::set_perf`] gave the Harness this program's memory
    /// tags.
    perf_scopes: bool,
}

/// Whether [`Session::set_perf`] gives a [`Headless`] a fake clock from
/// now on ([`measure_with_a_fake_clock`]).
static FAKE_CLOCK: AtomicBool = AtomicBool::new(false);

/// Has every [`Headless`] measure with a fake clock from now on: `bench`
/// counts instructions, which a real clock's readings could change from
/// run to run, and reads no time.
pub(crate) fn measure_with_a_fake_clock() {
    FAKE_CLOCK.store(true, Ordering::Relaxed);
}

/// The clock a [`Headless`] measures with: this machine's, unless
/// [`measure_with_a_fake_clock`] was called.
fn perf_clock() -> PerfClock {
    if FAKE_CLOCK.load(Ordering::Relaxed) {
        PerfClock::fake()
    } else {
        PerfClock::Real(now_us)
    }
}

/// The microseconds since this machine's clock was first read here: the
/// real clock a [`Headless`] measures with.
fn now_us() -> u64 {
    static START: OnceLock<Instant> = OnceLock::new();
    let start = START.get_or_init(Instant::now);
    u64::try_from(start.elapsed().as_micros()).unwrap_or(u64::MAX)
}

/// This program's heap counter's reading: the real memory a [`Headless`]
/// reads, handed over only in a build that counts (zeros otherwise).
///
/// It is also `bench`'s frame marker: a measured frame reads the heap
/// once, at its end, so Callgrind dumps its count before each call
/// (`crate::bench`'s `MARKER`). Never inlined, so it keeps its name.
#[inline(never)]
pub(crate) fn process_heap() -> HeapCounts {
    global::reading().map_or_else(HeapCounts::default, |counts| {
        HeapCounts::new(counts.live_bytes, counts.peak_bytes, counts.allocs)
    })
}

/// This program's heap counter's snapshot by tag: the real snapshots a
/// [`Headless`] takes, handed over only in a build that counts (an empty
/// one otherwise).
fn process_snapshot() -> HeapSnapshot {
    let (counts, rows) = (global::reading(), global::rows());
    match (counts, rows) {
        (Some(counts), Some(rows)) => heap::snapshot(counts, &rows, tags::overflowed()),
        _ => HeapSnapshot::default(),
    }
}

/// A session started by `start` with perf on: in a build that counts the
/// heap, what the start allocates (the game's load) counts under the
/// `load` phase, and each asset decode under its kind, as in a game's
/// window. Use it where a session will be measured, so its snapshots say
/// what the load holds.
pub fn measured_start<T>(start: impl FnOnce() -> T) -> T {
    let Some(scopes) = memory_scopes() else {
        return start();
    };
    tag_decodes();
    let before = scopes.enter(TagClass::Phase, "load");
    let started = start();
    scopes.leave(before);
    started
}

/// This program's heap counter's memory tags (`shuyi_alloc_count::tags`),
/// in a build that counts the heap (ADR-0046 §6).
fn memory_scopes() -> Option<MemoryScopes> {
    global::COUNTING.then(|| MemoryScopes::new(tags::tag, tags::enter, tags::leave))
}

/// In a build that counts the heap, tags what each asset decode allocates
/// with its kind ([`TagClass::Asset`]). Set once for the program: another
/// watch set first stays.
fn tag_decodes() {
    if global::COUNTING {
        let _ = watch_decodes(DecodeWatch::new(decode_before, decode_after));
    }
}

/// Enters the tag of the asset kind `kind`: the token is the tag before.
fn decode_before(kind: &'static str) -> u32 {
    u32::from(tags::enter(tags::tag(TagClass::Asset as u8, kind)))
}

/// Leaves the tag [`decode_before`] entered.
fn decode_after(token: u32, _: &Decoded<'_>) {
    tags::leave(u16::try_from(token).unwrap_or(tags::OTHER));
}

impl<G: GameData> Headless<G> {
    /// A session of `harness`, with no scene and no events of the game's.
    pub fn new(harness: Harness<G>) -> Self {
        Self {
            harness,
            scene: Box::new(|h| {
                Err(format!(
                    "the game's tools give no scene (the top screen is `{}`)",
                    h.top_screen()
                ))
            }),
            events: Box::new(|_| Vec::new()),
            picture: None,
            perf_clock: false,
            perf_memory: false,
            perf_scopes: false,
        }
    }

    /// This session, its scene from `scene`.
    #[must_use]
    pub fn with_scene(
        mut self,
        scene: impl Fn(&Harness<G>) -> Result<Scene, String> + 'static,
    ) -> Self {
        self.scene = Box::new(scene);
        self
    }

    /// This session, the game's events from `events`.
    #[must_use]
    pub fn with_events(mut self, events: impl Fn(&Harness<G>) -> Vec<String> + 'static) -> Self {
        self.events = Box::new(events);
        self
    }

    /// This session, its frame painted by `picture` (most simply the
    /// game's `frame-png` painter over the Harness's frame).
    #[must_use]
    pub fn with_picture(
        mut self,
        picture: impl Fn(&Harness<G>, Option<u32>) -> Result<Image, String> + 'static,
    ) -> Self {
        self.picture = Some(Box::new(picture));
        self
    }

    /// The Harness it drives.
    pub fn harness(&self) -> &Harness<G> {
        &self.harness
    }

    /// Whether the game has loaded: a record stopped before its load, or a
    /// load that failed, leaves none to read.
    fn loaded(&self) -> bool {
        self.harness.app().game().is_some()
    }
}

/// The screens that closed and opened between stacks `before` and
/// `after` (bottom first): those above what they share, the closed from
/// the top down, then the opened from the bottom up.
fn changes(before: &[&str], after: &[&str]) -> (Vec<String>, Vec<String>) {
    let shared = before.iter().zip(after).take_while(|(a, b)| a == b).count();
    let closed = before[shared..].iter().rev().map(|s| (*s).to_owned());
    let opened = after[shared..].iter().map(|s| (*s).to_owned());
    (closed.collect(), opened.collect())
}

/// A music command's name and cue.
fn music(command: &MusicCommand) -> (&'static str, &str) {
    match command {
        MusicCommand::Load { cue } => ("load", cue),
        MusicCommand::Start { cue, .. } => ("start", cue),
        MusicCommand::Gain { cue, .. } => ("gain", cue),
        MusicCommand::Stop { cue } => ("stop", cue),
    }
}

impl<G: GameData> Session for Headless<G> {
    fn play(&mut self, script: &Script) {
        script.play(&mut self.harness);
    }

    fn idle(&mut self, frames: u32) {
        for _ in 0..frames {
            self.harness.frame(&[], FRAME_DT);
        }
    }

    fn play_record(&mut self) -> Option<Played> {
        let mut played = self.harness.play_all().clone();
        if let Some(errors) = self.harness.app().errors() {
            played
                .limits
                .push(format!("the game didn't load: {errors}"));
        } else if !self.loaded() {
            played
                .limits
                .push("the record stopped before the game loaded".to_owned());
        }
        Some(played)
    }

    fn frame_count(&self) -> u32 {
        self.harness.frame_count()
    }

    fn screens(&self) -> Vec<String> {
        self.harness
            .screens()
            .into_iter()
            .map(str::to_owned)
            .collect()
    }

    fn quit_frame(&self) -> Option<u32> {
        self.harness.quit_frame()
    }

    fn snapshot(&self) -> String {
        if self.loaded() {
            self.harness.snapshot()
        } else {
            String::new()
        }
    }

    fn events(&self) -> Vec<Event> {
        let mut timed: Vec<(u32, Event)> = Vec::new();
        for pair in self.harness.screen_log().windows(2) {
            let ((_, before), (frame, after)) = (&pair[0], &pair[1]);
            let frame = *frame;
            let (closed, opened) = changes(before, after);
            timed.extend(
                closed
                    .into_iter()
                    .map(|screen| (frame, Event::Closed { frame, screen })),
            );
            timed.extend(
                opened
                    .into_iter()
                    .map(|screen| (frame, Event::Opened { frame, screen })),
            );
        }
        for (frame, command) in self.harness.music_log() {
            let (command, cue) = music(command);
            timed.push((
                frame,
                Event::Music {
                    frame,
                    command: command.to_owned(),
                    cue: cue.to_owned(),
                },
            ));
        }
        for (frame, request) in self.harness.audio_log() {
            if let AudioRequest::PlayVoice { line, .. } = request {
                let line = line.to_string();
                timed.push((frame, Event::Voice { frame, line }));
            }
        }
        if let Some(frame) = self.harness.quit_frame() {
            timed.push((frame, Event::Quit { frame }));
        }
        // A stable sort: within a frame, screens, then music, voices and
        // the quit.
        timed.sort_by_key(|(frame, _)| *frame);
        let game = if self.loaded() {
            (self.events)(&self.harness)
        } else {
            Vec::new()
        };
        let game = game.into_iter().map(|text| Event::Game { text });
        timed
            .into_iter()
            .map(|(_, event)| event)
            .chain(game)
            .collect()
    }

    fn sounds(&self) -> Vec<Sound> {
        self.harness
            .audio_log()
            .into_iter()
            .filter_map(|(frame, request)| match request {
                AudioRequest::PlaySound { cue, .. } | AudioRequest::StartSound { cue, .. } => {
                    Some(Sound {
                        frame,
                        cue: cue.clone(),
                    })
                }
                _ => None,
            })
            .collect()
    }

    fn scene(&self) -> Result<Scene, String> {
        if !self.loaded() {
            return Err("the game hasn't loaded".to_owned());
        }
        (self.scene)(&self.harness)
    }

    fn log(&self) -> Vec<LogEntry> {
        self.harness.log().lines().map(LogEntry::from).collect()
    }

    fn log_dropped(&self) -> u32 {
        self.harness.log().dropped()
    }

    fn modules(&self) -> Vec<ModuleEntry> {
        let Some(game) = self.harness.app().game() else {
            return Vec::new();
        };
        game.modules().iter().map(ModuleEntry::from).collect()
    }

    /// Gives the Harness this machine's clock, and in a build that counts
    /// the heap this program's heap counter and its memory tags (and tags
    /// the asset decodes), each unless its platform has one already (a
    /// test's fake, kept so its records stay the same); `false` takes away
    /// what it gave. Before the game has loaded it does nothing.
    fn set_perf(&mut self, on: bool) {
        if !self.loaded() {
            return;
        }
        let platform = self.harness.platform();
        let (clock, memory) = (platform.clock().is_none(), !platform.has_memory());
        let scopes = platform.memory_scopes().is_none();
        if on {
            if !self.perf_clock && clock {
                self.harness.set_clock(Some(perf_clock()));
                self.perf_clock = true;
            }
            if !self.perf_memory && memory && global::COUNTING {
                self.harness.set_memory(Some(process_heap));
                self.harness.set_heap_snapshots(Some(process_snapshot));
                self.perf_memory = true;
            }
            if !self.perf_scopes
                && scopes
                && let Some(scopes) = memory_scopes()
            {
                self.harness.set_scopes(Some(scopes));
                tag_decodes();
                self.perf_scopes = true;
            }
            return;
        }
        if core::mem::take(&mut self.perf_clock) {
            self.harness.set_clock(None);
        }
        if core::mem::take(&mut self.perf_memory) {
            self.harness.set_memory(None);
            self.harness.set_heap_snapshots(None);
        }
        if core::mem::take(&mut self.perf_scopes) {
            self.harness.set_scopes(None);
        }
    }

    fn counts_process_heap(&self) -> bool {
        self.perf_memory
    }

    fn perf(&self, since: u64, max: u32) -> PerfRead {
        PerfRead::of(self.harness.perf(), since, max)
    }

    fn heap_snapshot(&mut self) -> Option<HeapSnapshot> {
        self.harness.heap_snapshot()
    }

    fn picture(&self, scale: Option<u32>) -> Result<Image, String> {
        if !self.loaded() {
            return Err("the game hasn't loaded".to_owned());
        }
        match &self.picture {
            Some(picture) => picture(&self.harness, scale),
            None => Err(unsupported(
                "a picture of a session's frame",
                "session, with Headless::with_picture",
            )),
        }
    }
}

#[cfg(test)]
mod tests;
