//! `run` and `scene`: the game started headless as the game's options
//! say, a script of keys, controller buttons, actions by name and waits
//! played, and some frames more. `run` answers with the top screen, the frame as text, the
//! scene, what happened, the sounds asked for, the game's log and its
//! modules; `scene` with the scene and the log. With `--perf`, `run` also
//! measures each frame and answers a summary of the times, and with
//! `--perf-out` writes every kept frame as a JSON line ([`super::perf`]);
//! where the heap is counted by memory tag, it takes a heap snapshot
//! before the script and one after the last frame, and answers what held
//! the heap and what grew. With `--session`, the game starts from a session
//! record instead (`shuyi_platform::session`), played to its end or to the
//! first frame this build plays differently, and the script goes on from
//! there.

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use shuyi_ui::log::LOG_LINES;
use shuyi_ui::perf::{FrameRecord, HeapSnapshot, PERF_FRAMES};

use super::perf::{self, PerfSummary};
use super::session::{
    self, AddedSound, Event, LogEntry, ModuleEntry, Scene, Session, SessionInput, SessionPlayed,
    Sound, from_record, measured_start,
};
use super::{Answer, Op, codes};
use crate::frame_png::{self, Script, Step};
use crate::game::{GameArg, GameOption, GameTools, check_start, code_of, game_arg};

/// The most frames `frames` may ask for: about 28 simulated minutes.
pub const MAX_IDLE_FRAMES: u32 = 100_000;

/// What `run` and `scene` play.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, default)]
// An input: built with `default()` and its fields assigned, it may grow
// (ADR-0039).
#[non_exhaustive]
pub struct RunInput {
    /// The game's own start options (`frame-png --help` lists them), in
    /// the order given.
    pub start: Vec<GameArg>,
    /// The steps to play.
    pub script: Script,
    /// Frames to run after the script with no input.
    pub frames: u32,
    /// Measure each frame, and answer a summary of the times (`run` only).
    pub perf: bool,
    /// Also write every kept frame's record there, a JSON line each; a
    /// relative path is from the project's folder. It implies `perf`.
    pub perf_out: Option<String>,
    /// Start from this session record, played to its end, in place of the
    /// start options: the game's real start is the record's. A frame that
    /// plays differently stops it there, and the script and `frames` don't
    /// play.
    pub session: Option<SessionInput>,
    /// Answer only the value at this JSON Pointer (RFC 6901) in the
    /// scene's data, `/hero/x` (`scene` only).
    pub path: Option<String>,
    /// Answer the JSON Schema of the scene's data, as the game derived it
    /// (`scene` only).
    pub shape: bool,
}

/// What `run` saw.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
// An answer, read field by field: it may grow (ADR-0039).
#[non_exhaustive]
pub struct RunOutput {
    /// The frames run in all.
    pub frames: u32,
    /// The top screen's name; empty once every screen has closed.
    pub top_screen: String,
    /// The open screens, bottom first.
    pub screens: Vec<String>,
    /// The frame the game asked to quit in, if it did.
    pub quit: Option<u32>,
    /// The scene on screen, if a stage is open.
    pub scene: Option<Scene>,
    /// The last frame drawn, in the snapshot text format.
    pub frame: String,
    /// What happened, in frame order, then the game's own events.
    pub events: Vec<Event>,
    /// The sound cues asked for, in order.
    pub sounds: Vec<Sound>,
    /// The sounds the game added at run time and hadn't removed at the
    /// end of the run, in cue order; left out when none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sounds_added: Vec<AddedSound>,
    /// The game's log, oldest first: its last lines.
    pub log: Vec<LogEntry>,
    /// The modules installed in the game, in install order, with the
    /// hooks the game calls on each.
    pub modules: Vec<ModuleEntry>,
    /// The frames' times, with `perf`: none without it, or when nothing
    /// was measured.
    pub perf: Option<PerfSummary>,
    /// How far the session record played, with `session`.
    pub session: Option<SessionPlayed>,
}

/// What `scene` saw.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
// An answer, read field by field: it may grow (ADR-0039).
#[non_exhaustive]
pub struct SceneOutput {
    /// The frames run in all.
    pub frames: u32,
    /// The top screen's name.
    pub top_screen: String,
    /// The scene, if a stage is open; left out with `path` or `shape`,
    /// which answer what was asked instead.
    pub scene: Option<Scene>,
    /// The game's log, oldest first: its last lines.
    pub log: Vec<LogEntry>,
    /// How far the session record played, with `session`.
    pub session: Option<SessionPlayed>,
    /// With `path`, the value at that pointer in the scene's data.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<serde_json::Value>,
    /// With `shape`, the JSON Schema of the scene's data in serialize mode,
    /// if the game gave one (`Scene::with_data`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shape: Option<serde_json::Value>,
}

/// What `scene` is asked for beyond the scene: one value of its data, by
/// pointer, and its data's shape.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SceneAsk<'a> {
    /// A JSON Pointer into the scene's data.
    pub path: Option<&'a str>,
    /// Whether to answer the data's shape.
    pub shape: bool,
}

impl RunInput {
    /// Whether the Harness can play it: every step of its script
    /// ([`frame_png::check`]), at most [`MAX_IDLE_FRAMES`] frames after, and
    /// no start options with a session record, whose start is the game's
    /// real one. Its actions' names are the game's to check
    /// ([`check_actions`]).
    pub fn check(&self) -> Result<(), String> {
        frame_png::check(&self.script)?;
        if self.session.is_some() && !self.start.is_empty() {
            return Err(
                "session: a record starts as the game really starts, with no start options"
                    .to_owned(),
            );
        }
        if self.frames > MAX_IDLE_FRAMES {
            return Err(format!(
                "frames: at most {MAX_IDLE_FRAMES}, not {}",
                self.frames
            ));
        }
        Ok(())
    }

    /// Whether it measures the frames: `perf`, or `perf_out`.
    pub fn measures(&self) -> bool {
        self.perf || self.perf_out.is_some()
    }
}

/// A session played: the heap snapshot taken once it started if it was
/// measured, and how far its record played if it has one.
struct Ran {
    session: Box<dyn Session>,
    first: Option<HeapSnapshot>,
    record: Option<SessionPlayed>,
    /// What the record's playing can't promise.
    limits: Vec<String>,
    /// Whether a frame of the record played differently.
    differs: bool,
}

/// A session of `game` started and played as `input` says, or the answer
/// of `op` that says why not; a record is read from `root`. Measured, also
/// the heap snapshot taken once it started, before the script. A record
/// that plays differently stands on that frame: the script and the frames
/// after it don't play.
fn played<T>(
    op: Op,
    game: &dyn GameTools,
    root: &Path,
    input: &RunInput,
) -> Result<Ran, Answer<T>> {
    input
        .check()
        .map_err(|e| Answer::failed(op, codes::WRONG_ARGUMENTS, e))?;
    if op == Op::Run && (input.path.is_some() || input.shape) {
        let error = "path and shape are `scene`'s: `run` answers the whole scene";
        return Err(Answer::failed(op, codes::WRONG_ARGUMENTS, error));
    }
    check_start(game.frame_options(), &input.start)
        .map_err(|e| Answer::failed(op, codes::WRONG_ARGUMENTS, e))?;
    let start = || match &input.session {
        Some(record) => {
            from_record(game, root, record).map(|(session, played)| (session, Some(played)))
        }
        None => game
            .session(&input.start)
            .map(|session| (session, None))
            .map_err(|e| (code_of(&e), e)),
    };
    let started = if input.measures() {
        measured_start(start)
    } else {
        start()
    };
    let (mut session, played) = started.map_err(|(code, e)| Answer::failed(op, code, e))?;
    let differs = played
        .as_ref()
        .is_some_and(|played| played.first_difference.is_some());
    let mut first = None;
    if !differs {
        check_actions(&*session, &input.script)
            .map_err(|e| Answer::failed(op, codes::WRONG_ARGUMENTS, e))?;
        if input.measures() {
            session.set_perf(true);
            first = session.heap_snapshot();
        }
        session.play(&input.script);
        session.idle(input.frames);
    }
    // Read after the script: its frames add lines (`Session::play_record`).
    let after = played.and_then(|_| session.play_record());
    let (record, limits) = match (&input.session, after) {
        (Some(file), Some(played)) => (Some(SessionPlayed::of(&file.file, &played)), played.limits),
        _ => (None, Vec::new()),
    };
    Ok(Ran {
        session,
        first,
        record,
        limits,
        differs,
    })
}

/// Whether `session`'s game can press every action `script` names: one
/// with a key or a button ([`Session::actions`]). An error names the step
/// and the game's actions, before anything plays (`run`, `scene` and MCP's
/// `step` call it).
pub fn check_actions(session: &dyn Session, script: &Script) -> Result<(), String> {
    let mut actions = None;
    for (i, step) in script.steps.iter().enumerate() {
        let Step::Act(name) = step else {
            continue;
        };
        let actions = actions.get_or_insert_with(|| session.actions());
        if !actions.contains(name) {
            let theirs = if actions.is_empty() {
                "the game has none".to_owned()
            } else {
                format!("the game's are {}", actions.join(", "))
            };
            return Err(format!(
                "step {}: act: no action `{name}` with a key or a button; {theirs}",
                i + 1
            ));
        }
    }
    Ok(())
}

/// What a run of `frames` frames couldn't show: the frames after the game
/// asked to quit (on frame `quit`), the frame if no screen is `open`, and
/// the scene, if there is none because of `no_scene`.
pub fn limits_of(
    frames: u32,
    quit: Option<u32>,
    open: bool,
    no_scene: Option<&str>,
) -> Vec<String> {
    let mut limits = Vec::new();
    if let Some(quit) = quit {
        let after = frames.saturating_sub(quit);
        limits.push(if after == 0 {
            format!("the game asked to quit on frame {quit}, the last")
        } else {
            format!(
                "the game asked to quit on frame {quit}: the {after} frame(s) after it changed \
                 nothing"
            )
        });
    }
    if !open {
        limits.push("no screen is open: the frame is the last one drawn".to_owned());
    }
    if let Some(why) = no_scene {
        limits.push(format!("no scene: {why}"));
    }
    limits
}

/// What `session` couldn't show after its run, `scene` its scene: also
/// the lines its log dropped.
fn limits(session: &dyn Session, scene: &Result<Scene, String>) -> Vec<String> {
    let mut limits = limits_of(
        session.frame_count(),
        session.quit_frame(),
        !session.screens().is_empty(),
        scene.as_ref().err().map(String::as_str),
    );
    let dropped = session.log_dropped();
    if dropped > 0 {
        limits.push(format!(
            "the log keeps the last {LOG_LINES} lines: {dropped} earlier line(s) were dropped"
        ));
    }
    limits
}

/// What `session` shows after its run, as `run` answers it, with its
/// limits.
pub fn run_output(session: &dyn Session) -> (RunOutput, Vec<String>) {
    let scene = session.scene();
    let limits = limits(session, &scene);
    let screens = session.screens();
    let output = RunOutput {
        frames: session.frame_count(),
        top_screen: screens.last().cloned().unwrap_or_default(),
        screens,
        quit: session.quit_frame(),
        scene: scene.ok(),
        frame: session.snapshot(),
        events: session.events(),
        sounds: session.sounds(),
        sounds_added: session.sounds_added(),
        log: session.log(),
        modules: session.modules(),
        perf: None,
        session: None,
    };
    (output, limits)
}

/// The heap snapshots before a run and after it.
type Snapshots = (HeapSnapshot, HeapSnapshot);

/// The summary of every record `session` kept, with the heap snapshots
/// `heap` before and after its run, the records themselves, for the trace
/// file, and the limits of both.
fn perf_of(
    session: &dyn Session,
    heap: Option<&Snapshots>,
) -> (Option<PerfSummary>, Vec<FrameRecord>, Vec<String>) {
    let read = session.perf(0, u32::try_from(PERF_FRAMES).unwrap_or(u32::MAX));
    let mut limits = vec![perf::TIMES.to_owned()];
    if !read.measured {
        limits.push(perf::unmeasured_limit(session.frame_count()));
        return (None, Vec::new(), limits);
    }
    if read.dropped > 0 {
        limits.push(perf::dropped_limit(read.dropped));
    }
    limits.extend(perf::heap_limits(
        &read.records,
        session.counts_process_heap(),
        perf::other_threads(),
    ));
    if heap.is_none() && read.records.iter().any(|r| r.heap.is_some()) {
        limits.push(perf::HEAP_UNTAGGED.to_owned());
    }
    let summary = perf::summary(&read.records).map(|mut summary| {
        summary.dropped = read.dropped;
        if let Some((first, last)) = heap {
            perf::add_heap(&mut summary, first, last);
        }
        summary
    });
    (summary, read.records, limits)
}

/// Where `--perf-out <path>` writes: `path` from the project's folder
/// `root`, as the operating system takes it.
pub fn perf_out_path(root: &Path, path: &str) -> PathBuf {
    let path = root.join(path);
    std::path::absolute(&path).unwrap_or(path)
}

/// `records` written to `path` as a trace, then the heap snapshots `heap`
/// and their diff, its folders made.
fn write_trace(
    path: &Path,
    records: &[FrameRecord],
    heap: Option<&Snapshots>,
) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let mut trace = perf::lines(records);
    if let Some((first, last)) = heap {
        trace.push_str(&perf::heap_lines(first, last));
    }
    fs::write(path, trace)
}

/// The scene `session` shows, as `scene` answers it, with its limits.
pub fn scene_output(session: &dyn Session) -> (SceneOutput, Vec<String>) {
    let scene = session.scene();
    let limits = limits(session, &scene);
    let screens = session.screens();
    let output = SceneOutput {
        frames: session.frame_count(),
        top_screen: screens.last().cloned().unwrap_or_default(),
        scene: scene.ok(),
        log: session.log(),
        session: None,
        value: None,
        shape: None,
    };
    (output, limits)
}

/// `output` answering `ask` in place of its scene: the value at its path,
/// and its data's shape, with what the shape's absence adds to `limits`;
/// or the problem code and why, a path that names nothing. With no scene,
/// or nothing asked, it is left as it is.
pub fn ask_scene(
    output: &mut SceneOutput,
    limits: &mut Vec<String>,
    ask: SceneAsk<'_>,
) -> Result<(), (codes::Code, String)> {
    if ask.path.is_none() && !ask.shape {
        return Ok(());
    }
    let Some(scene) = output.scene.take() else {
        return Ok(());
    };
    if let Some(path) = ask.path {
        let value = session::at_pointer(&scene.data, path).map_err(|e| (codes::NO_VALUE, e))?;
        output.value = Some(value.clone());
    }
    if ask.shape {
        match scene.shape {
            Some(shape) => output.shape = Some(shape.to_value()),
            None => limits.push(NO_SHAPE.to_owned()),
        }
    }
    Ok(())
}

/// The limit of `scene --shape` on a scene whose data has no shape.
pub const NO_SHAPE: &str = "no shape: the game built its scene's data with no `Scene::with_data`";

/// The answer `op` gives with `session`'s scene, asked `ask`, and how far
/// its record played, `record`: it passes if there is a scene and the
/// record played no differently (`differs`).
pub fn scene_answer(
    op: Op,
    session: &dyn Session,
    ask: SceneAsk<'_>,
    record: Option<SessionPlayed>,
    differs: bool,
) -> Answer<SceneOutput> {
    let (mut output, mut limits) = scene_output(session);
    output.session = record;
    let ok = output.scene.is_some() && !differs;
    if let Err((code, error)) = ask_scene(&mut output, &mut limits, ask) {
        return Answer::failed(op, code, error);
    }
    Answer::new(op, ok, output, limits)
}

/// `run`: plays `input` in `game`; a trace file (`perf_out`) is written
/// from the working folder ([`run_at`] says from where).
pub fn run(game: &dyn GameTools, input: &RunInput) -> Answer<RunOutput> {
    run_at(game, Path::new("."), input)
}

/// `run`: plays `input` in `game`; a trace file (`perf_out`) is written
/// from the project's folder `root`, as the CLI and the MCP server run it.
pub fn run_at(game: &dyn GameTools, root: &Path, input: &RunInput) -> Answer<RunOutput> {
    let Ran {
        mut session,
        first,
        record,
        limits: record_limits,
        differs,
    } = match played(Op::Run, game, root, input) {
        Ok(played) => played,
        Err(answer) => return answer,
    };
    // After the last frame, before the answer is read.
    let heap = first.zip(session.heap_snapshot());
    let (mut output, mut limits) = run_output(&*session);
    output.session = record;
    limits.extend(record_limits);
    if input.measures() {
        let (mut summary, records, perf_limits) = perf_of(&*session, heap.as_ref());
        limits.extend(perf_limits);
        if let (Some(file), Some(summary)) = (&input.perf_out, &mut summary) {
            let path = perf_out_path(root, file);
            if let Err(e) = write_trace(&path, &records, heap.as_ref()) {
                return Answer::failed(Op::Run, codes::FILE, format!("--perf-out {file}: {e}"));
            }
            summary.file = Some(path.display().to_string());
        }
        output.perf = summary;
    }
    Answer::new(Op::Run, !differs, output, limits)
}

/// `scene`: plays `input` in `game`, and answers with the scene; a record
/// is read from the working folder ([`scene_at`] says from where).
pub fn scene(game: &dyn GameTools, input: &RunInput) -> Answer<SceneOutput> {
    scene_at(game, Path::new("."), input)
}

/// `scene`: plays `input` in `game`, a record read from the project's folder
/// `root`, and answers with the scene. It passes if there is one and a
/// record played no differently.
pub fn scene_at(game: &dyn GameTools, root: &Path, input: &RunInput) -> Answer<SceneOutput> {
    match played(Op::Scene, game, root, input) {
        Ok(ran) => {
            let ask = SceneAsk {
                path: input.path.as_deref(),
                shape: input.shape,
            };
            let mut answer = scene_answer(Op::Scene, &*ran.session, ask, ran.record, ran.differs);
            answer.limits.extend(ran.limits);
            answer
        }
        Err(answer) => answer,
    }
}

/// The usage of `run` (and `scene`, if `op` says so), with the game's
/// start `options`.
pub fn usage(command: &str, op: Op, options: &[GameOption]) -> String {
    let mut out = format!(
        "usage: {command} {} [steps] [--frames N] [options] [--json]\n\n\
         steps (each may repeat, played in order):\n  \
         --script \"Enter Up\"  press and release each key chord in turn (or --keys)\n  \
         --pad \"South\"        press and release each controller button in turn\n  \
         --act Confirm        press the game's action by its key, or else its button\n  \
         --wait 0.5           let that many seconds pass (60 frames a second, rounded up)\n\n\
         --frames N           then run N frames (1/60 s each) with no input\n\n\
         --session <file>     start from a session record, from the project's folder,\n                     \
         played to its end or the first frame that differs; no start options\n\
         --any-stamp          play a record another build or other content made too\n",
        op.name()
    );
    if op == Op::Scene {
        out.push_str(
            "\n--path <pointer>     answer only the value at that JSON Pointer in the scene's\n                     \
             data (`/hero/x`)\n\
             --shape              answer the JSON Schema of the scene's data\n",
        );
    }
    if op == Op::Run {
        out.push_str(
            "\n--perf               measure each frame, and answer a summary of the times\n\
             --perf-out <file>    also write each kept frame as a JSON line, from the\n                     \
             project's folder (implies --perf)\n\
             --watch              alone: the game in its window instead, its files on\n                     \
             disk reloading as they change (a debug build)\n",
        );
    }
    if !options.is_empty() {
        out.push_str("\nthe game's options:\n");
        for option in options {
            let name = match option.value {
                Some(value) => format!("{} {value}", option.flag),
                None => option.flag.to_owned(),
            };
            let help = option.help.lines().next().unwrap_or_default();
            // Writing to a `String` can't fail.
            let _ = writeln!(out, "  {name:<20} {help}");
        }
    }
    out
}

/// Reads `run`'s and `scene`'s arguments (not `--json`), the game's
/// start `options` among them.
pub fn parse_args(args: &[String], options: &[GameOption]) -> Result<RunInput, String> {
    let mut input = RunInput::default();
    let mut any_stamp = false;
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        let mut value = || {
            iter.next()
                .ok_or_else(|| format!("{arg} needs a value"))
                .cloned()
        };
        if matches!(
            arg.as_str(),
            "--script" | "--keys" | "--pad" | "--act" | "--wait"
        ) {
            let text = value()?;
            if let Some(step) = frame_png::parse_step(arg, &text) {
                input.script.steps.push(step?);
            }
            continue;
        }
        if arg == "--session" {
            input.session = Some(SessionInput::new(value()?));
            continue;
        }
        if arg == "--any-stamp" {
            any_stamp = true;
            continue;
        }
        if arg == "--perf" {
            input.perf = true;
            continue;
        }
        if arg == "--path" {
            input.path = Some(value()?);
            continue;
        }
        if arg == "--shape" {
            input.shape = true;
            continue;
        }
        if arg == "--perf-out" {
            input.perf_out = Some(value()?);
            input.perf = true;
            continue;
        }
        if arg == "--frames" {
            let text = value()?;
            input.frames = text
                .parse()
                .ok()
                .filter(|n| *n <= MAX_IDLE_FRAMES)
                .ok_or_else(|| {
                    format!("--frames: a whole number 0 to {MAX_IDLE_FRAMES}, not {text}")
                })?;
            continue;
        }
        input.start.push(game_arg(arg, options, value)?);
    }
    match &mut input.session {
        Some(session) => session.any_stamp = any_stamp,
        None if any_stamp => return Err("--any-stamp needs --session".to_owned()),
        None => {}
    }
    Ok(input)
}

/// One event, for people.
fn event_text(event: &Event) -> String {
    match event {
        Event::Opened { frame, screen } => format!("frame {frame}: opened {screen}"),
        Event::Closed { frame, screen } => format!("frame {frame}: closed {screen}"),
        Event::Music {
            frame,
            command,
            cue,
        } => format!("frame {frame}: music {command} {cue}"),
        Event::Voice { frame, line } => format!("frame {frame}: voice {line}"),
        Event::Quit { frame } => format!("frame {frame}: quit"),
        Event::Game { text } => format!("game: {text}"),
    }
}

/// The log's warnings and errors, for people.
fn log_text(out: &mut String, log: &[LogEntry]) {
    let serious = |entry: &&LogEntry| matches!(entry.level.as_str(), "warn" | "error");
    if !log.iter().any(|entry| serious(&entry)) {
        return;
    }
    out.push_str("log (warnings and errors; every line with --json):\n");
    for entry in log.iter().filter(serious) {
        let _ = writeln!(
            out,
            "  frame {}: {} {}: {}",
            entry.frame, entry.level, entry.category, entry.text
        );
    }
}

/// `scene`, for people.
fn scene_text(out: &mut String, scene: Option<&Scene>) {
    if let Some(scene) = scene {
        let _ = writeln!(out, "scene ({}, on {}):", scene.kind, scene.screen);
        out.push_str(&scene.text);
        if !scene.text.ends_with('\n') {
            out.push('\n');
        }
        if !scene.data.is_null() {
            let _ = writeln!(out, "{}", scene.data);
        }
    }
}

/// How far a session record played, for people.
fn session_text(out: &mut String, session: Option<&SessionPlayed>) {
    let Some(session) = session else {
        return;
    };
    let _ = writeln!(
        out,
        "session {}: played {} of {} frame(s)",
        session.file, session.frames, session.of
    );
    if let Some(difference) = &session.first_difference {
        let _ = writeln!(
            out,
            "it first differs at frame {}: the record says {}, this build {}",
            difference.frame, difference.recorded, difference.played
        );
    }
}

/// What `run` saw, for people.
pub fn text(output: &RunOutput) -> String {
    let mut out = format!(
        "run: {} frame(s); top screen: {} ({})\n",
        output.frames,
        if output.top_screen.is_empty() {
            "none"
        } else {
            &output.top_screen
        },
        output.screens.join(" > ")
    );
    session_text(&mut out, output.session.as_ref());
    if !output.events.is_empty() {
        out.push_str("events:\n");
        for event in &output.events {
            let _ = writeln!(out, "  {}", event_text(event));
        }
    }
    log_text(&mut out, &output.log);
    if !output.modules.is_empty() {
        let modules: Vec<String> = output
            .modules
            .iter()
            .map(|m| format!("{} ({})", m.name, m.hooks.join(", ")))
            .collect();
        let _ = writeln!(out, "modules: {}", modules.join(", "));
    }
    if !output.sounds.is_empty() {
        let sounds: Vec<String> = output
            .sounds
            .iter()
            .map(|s| format!("{} (frame {})", s.cue, s.frame))
            .collect();
        let _ = writeln!(out, "sounds: {}", sounds.join(", "));
    }
    if !output.sounds_added.is_empty() {
        let added: Vec<String> = output
            .sounds_added
            .iter()
            .map(|s| {
                format!(
                    "{} ({} samples a second, {} channel(s), {} frames, {} milliseconds)",
                    s.cue, s.rate, s.channels, s.frames, s.millis
                )
            })
            .collect();
        let _ = writeln!(out, "sounds added: {}", added.join(", "));
    }
    if let Some(summary) = &output.perf {
        out.push_str(&perf::text(summary));
    }
    scene_text(&mut out, output.scene.as_ref());
    // The glyphs only: the colours, legend and items are in `--json`.
    let glyphs = output
        .frame
        .split_once("--- ")
        .map_or(output.frame.as_str(), |(glyphs, _)| glyphs);
    out.push_str("frame (its colours with --json):\n");
    out.push_str(glyphs);
    if !glyphs.ends_with('\n') {
        out.push('\n');
    }
    out
}

/// What `scene` saw, for people.
pub fn scene_output_text(output: &SceneOutput) -> String {
    let mut out = format!(
        "scene: after {} frame(s), top screen {}\n",
        output.frames, output.top_screen
    );
    session_text(&mut out, output.session.as_ref());
    log_text(&mut out, &output.log);
    scene_text(&mut out, output.scene.as_ref());
    if let Some(value) = &output.value {
        let _ = writeln!(out, "value: {value}");
    }
    if let Some(shape) = &output.shape {
        let pretty = serde_json::to_string_pretty(shape).unwrap_or_default();
        let _ = writeln!(out, "shape:\n{pretty}");
    }
    out
}

/// A script of keys, for tests and callers that only press keys.
pub fn keys(script: &str) -> Script {
    Script::new(vec![Step::Keys(script.to_owned())])
}

#[cfg(test)]
mod tests;
