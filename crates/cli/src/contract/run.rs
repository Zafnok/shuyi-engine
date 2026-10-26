//! `run` and `scene`: the game started headless as the game's options
//! say, a script of keys, controller buttons and waits played, and some
//! frames more. `run` answers with the top screen, the frame as text, the
//! scene, what happened, the sounds asked for, the game's log and its
//! modules; `scene` with the scene and the log. With `--perf`, `run` also
//! measures each frame and answers a summary of the times, and with
//! `--perf-out` writes every kept frame as a JSON line ([`super::perf`]).

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use shuyi_ui::log::LOG_LINES;
use shuyi_ui::perf::{FrameRecord, PERF_FRAMES};

use super::perf::{self, PerfSummary};
use super::session::{Event, LogEntry, ModuleEntry, Scene, Session, Sound};
use super::{Answer, Op, codes};
use crate::frame_png::{Script, Step};
use crate::game::{GameArg, GameOption, GameTools, code_of};

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
    /// The game's log, oldest first: its last lines.
    pub log: Vec<LogEntry>,
    /// The modules installed in the game, in install order, with the
    /// hooks the game calls on each.
    pub modules: Vec<ModuleEntry>,
    /// The frames' times, with `perf`: none without it, or when nothing
    /// was measured.
    pub perf: Option<PerfSummary>,
}

/// What `scene` saw.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SceneOutput {
    /// The frames run in all.
    pub frames: u32,
    /// The top screen's name.
    pub top_screen: String,
    /// The scene, if a stage is open.
    pub scene: Option<Scene>,
    /// The game's log, oldest first: its last lines.
    pub log: Vec<LogEntry>,
}

impl RunInput {
    /// Whether the Harness can play it: every step of its script
    /// ([`Script::check`]), and at most [`MAX_IDLE_FRAMES`] frames after.
    pub fn check(&self) -> Result<(), String> {
        self.script.check()?;
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

/// A session of `game` started and played as `input` says, or the answer
/// of `op` that says why not.
fn played<T>(
    op: Op,
    game: &dyn GameTools,
    input: &RunInput,
) -> Result<Box<dyn Session>, Answer<T>> {
    input
        .check()
        .map_err(|e| Answer::failed(op, codes::WRONG_ARGUMENTS, e))?;
    let mut session = game
        .session(&input.start)
        .map_err(|e| Answer::failed(op, code_of(&e), e))?;
    if input.measures() {
        session.set_perf(true);
    }
    session.play(&input.script);
    session.idle(input.frames);
    Ok(session)
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
        log: session.log(),
        modules: session.modules(),
        perf: None,
    };
    (output, limits)
}

/// The summary of every record `session` kept, the records themselves,
/// for the trace file, and the limits of both.
fn perf_of(session: &dyn Session) -> (Option<PerfSummary>, Vec<FrameRecord>, Vec<String>) {
    let read = session.perf(0, u32::try_from(PERF_FRAMES).unwrap_or(u32::MAX));
    let mut limits = vec![perf::TIMES.to_owned()];
    if !read.measured {
        limits.push(perf::unmeasured_limit(session.frame_count()));
        return (None, Vec::new(), limits);
    }
    if read.dropped > 0 {
        limits.push(perf::dropped_limit(read.dropped));
    }
    let summary = perf::summary(&read.records).map(|mut summary| {
        summary.dropped = read.dropped;
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

/// `records` written to `path` as a trace, its folders made.
fn write_trace(path: &Path, records: &[FrameRecord]) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    fs::write(path, perf::lines(records))
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
    };
    (output, limits)
}

/// `run`: plays `input` in `game`; a trace file (`perf_out`) is written
/// from the working folder ([`run_at`] says from where).
pub fn run(game: &dyn GameTools, input: &RunInput) -> Answer<RunOutput> {
    run_at(game, Path::new("."), input)
}

/// `run`: plays `input` in `game`; a trace file (`perf_out`) is written
/// from the project's folder `root`, as the CLI and the MCP server run it.
pub fn run_at(game: &dyn GameTools, root: &Path, input: &RunInput) -> Answer<RunOutput> {
    let session = match played(Op::Run, game, input) {
        Ok(session) => session,
        Err(answer) => return answer,
    };
    let (mut output, mut limits) = run_output(&*session);
    if input.measures() {
        let (mut summary, records, perf_limits) = perf_of(&*session);
        limits.extend(perf_limits);
        if let (Some(file), Some(summary)) = (&input.perf_out, &mut summary) {
            let path = perf_out_path(root, file);
            if let Err(e) = write_trace(&path, &records) {
                return Answer::failed(Op::Run, codes::FILE, format!("--perf-out {file}: {e}"));
            }
            summary.file = Some(path.display().to_string());
        }
        output.perf = summary;
    }
    Answer::new(Op::Run, true, output, limits)
}

/// `scene`: plays `input` in `game`, and answers with the scene. It
/// passes if there is one.
pub fn scene(game: &dyn GameTools, input: &RunInput) -> Answer<SceneOutput> {
    match played(Op::Scene, game, input) {
        Ok(session) => {
            let (output, limits) = scene_output(&*session);
            Answer::new(Op::Scene, output.scene.is_some(), output, limits)
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
         --wait 0.5           let that many seconds pass\n\n\
         --frames N           then run N frames (1/60 s each) with no input\n",
        op.name()
    );
    if op == Op::Run {
        out.push_str(
            "\n--perf               measure each frame, and answer a summary of the times\n\
             --perf-out <file>    also write each kept frame as a JSON line, from the\n                     \
             project's folder (implies --perf)\n",
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
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        let mut value = || {
            iter.next()
                .ok_or_else(|| format!("{arg} needs a value"))
                .cloned()
        };
        if matches!(arg.as_str(), "--script" | "--keys" | "--pad" | "--wait") {
            let text = value()?;
            if let Some(step) = Script::parse_step(arg, &text) {
                input.script.steps.push(step?);
            }
            continue;
        }
        if arg == "--perf" {
            input.perf = true;
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
        let option = options
            .iter()
            .find(|option| option.flag == arg)
            .ok_or_else(|| format!("unknown option: {arg}"))?;
        let value = option.value.map(|_| value()).transpose()?;
        input.start.push(GameArg {
            flag: arg.clone(),
            value,
        });
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
    log_text(&mut out, &output.log);
    scene_text(&mut out, output.scene.as_ref());
    out
}

/// A script of keys, for tests and callers that only press keys.
pub fn keys(script: &str) -> Script {
    Script {
        steps: vec![Step::Keys(script.to_owned())],
    }
}

#[cfg(test)]
mod tests;
