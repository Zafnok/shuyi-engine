//! `run` and `scene`: the game started headless as the game's options
//! say, a script of keys, controller buttons and waits played, and some
//! frames more. `run` answers with the top screen, the frame as text, the
//! scene, what happened and the sounds asked for; `scene` with the scene
//! alone.

use std::fmt::Write as _;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::session::{Event, Scene, Session, Sound};
use super::{Answer, Op};
use crate::frame_png::{Script, Step};
use crate::game::{GameArg, GameOption, GameTools};

/// The most frames `frames` may ask for: about 28 simulated minutes.
pub const MAX_IDLE_FRAMES: u32 = 100_000;

/// What `run` and `scene` play.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, default)]
pub struct RunInput {
    /// The game's own start options (`frame-png --help` lists them), in
    /// the order given.
    pub start: Vec<GameArg>,
    /// The steps to play.
    pub script: Script,
    /// Frames to run after the script with no input.
    pub frames: u32,
}

/// What `run` saw.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
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
}

/// A session of `game` started and played as `input` says, or the answer
/// of `op` that says why not.
fn played<T>(
    op: Op,
    game: &dyn GameTools,
    input: &RunInput,
) -> Result<Box<dyn Session>, Answer<T>> {
    input.check().map_err(|e| Answer::failed(op, e))?;
    let mut session = game
        .session(&input.start)
        .map_err(|e| Answer::failed(op, e))?;
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

/// What `session` couldn't show after its run, `scene` its scene.
fn limits(session: &dyn Session, scene: &Result<Scene, String>) -> Vec<String> {
    limits_of(
        session.frame_count(),
        session.quit_frame(),
        !session.screens().is_empty(),
        scene.as_ref().err().map(String::as_str),
    )
}

/// `run`: plays `input` in `game`.
pub fn run(game: &dyn GameTools, input: &RunInput) -> Answer<RunOutput> {
    let session = match played(Op::Run, game, input) {
        Ok(session) => session,
        Err(answer) => return answer,
    };
    let scene = session.scene();
    let limits = limits(&*session, &scene);
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
    };
    Answer::new(Op::Run, true, output, limits)
}

/// `scene`: plays `input` in `game`, and answers with the scene. It
/// passes if there is one.
pub fn scene(game: &dyn GameTools, input: &RunInput) -> Answer<SceneOutput> {
    let session = match played(Op::Scene, game, input) {
        Ok(session) => session,
        Err(answer) => return answer,
    };
    let scene = session.scene();
    let limits = limits(&*session, &scene);
    let screens = session.screens();
    let output = SceneOutput {
        frames: session.frame_count(),
        top_screen: screens.last().cloned().unwrap_or_default(),
        scene: scene.ok(),
    };
    Answer::new(Op::Scene, output.scene.is_some(), output, limits)
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
    if !output.sounds.is_empty() {
        let sounds: Vec<String> = output
            .sounds
            .iter()
            .map(|s| format!("{} (frame {})", s.cue, s.frame))
            .collect();
        let _ = writeln!(out, "sounds: {}", sounds.join(", "));
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
