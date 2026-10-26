//! The session over the live link (ADR-0047): `start` with `link`
//! attaches the session to a debug build already running in its window,
//! and the session tools play a player's input on it and read it as
//! `attach` does, through `attach`'s own connection. Nothing else in the
//! build changes: `reset` is refused, and `stop` detaches and leaves it
//! running.
//!
//! The link carries no scene, events or sounds: `scene` answers the
//! frame's text in their place, and the answers say so in their limits.

use serde_json::Value;
use shuyi_platform::link::Phase;

use super::host::{failed, read, to_json};
use super::{
    EventsOutput, FramePngInput, FramePngOutput, NoInput, StartLink, StartOutput, StepInput,
    StopOutput,
};
use crate::contract::attach::{self, AttachInput, AttachStep, Connection, Failed, Seen};
use crate::contract::codes;
use crate::contract::run::{RunOutput, SceneOutput};
use crate::contract::session::{LogEntry, Scene};
use crate::contract::{Answer, Op};

/// The limit of an answer that has no scene.
pub const NO_SCENE: &str = "no scene over the link";

/// The limit of an answer that has no events or sounds.
pub const NO_EVENTS: &str = "no events over the link";

/// The session's build, reached over the live link.
pub struct Attached {
    link: Connection,
    address: String,
    token: String,
    /// Whether the session has ended: stopped, or the link lost.
    ended: bool,
}

impl Attached {
    /// The session attached to the build `to` names, and `start`'s answer;
    /// or `start`'s answer that says why not, with `attach`'s codes.
    pub fn start(to: &StartLink) -> (Option<Self>, Value) {
        let op = Op::Start;
        let mut asked = AttachInput::default();
        asked.address.clone_from(&to.address);
        asked.token.clone_from(&to.token);
        if let Err(e) = asked.check() {
            return (
                None,
                failed(op, codes::WRONG_ARGUMENTS, format!("link: {e}")),
            );
        }
        let attached = Connection::open(&to.address).and_then(|mut link| {
            let game = attach::hello(&mut link, &to.token)?;
            let status = attach::status(&mut link)?;
            Ok((link, game, status))
        });
        let (link, game, status) = match attached {
            Ok(attached) => attached,
            Err((code, error)) => return (None, failed(op, code, error)),
        };
        let output = StartOutput {
            project: String::new(),
            game,
            start: Vec::new(),
            options: Vec::new(),
            frames: status.frames,
            top_screen: status.top_screen,
            screens: status.screens,
        };
        let limits = vec![format!(
            "attached to the build at {}: it runs in its window with its own options; {NO_SCENE}, \
             {NO_EVENTS}, reset is refused, and stop leaves it running",
            to.address
        )];
        let session = Self {
            link,
            address: asked.address,
            token: asked.token,
            ended: false,
        };
        (
            Some(session),
            to_json(&Answer::new(op, true, output, limits)),
        )
    }

    /// Whether the session has ended: `stop` detached, or the link was
    /// lost.
    pub fn ended(&self) -> bool {
        self.ended
    }

    /// Session tool `op`'s answer to `input` over the link, and its
    /// picture, if it has one.
    pub fn call(&mut self, op: Op, input: Value) -> (Value, Option<String>) {
        let answered = match op {
            Op::Step => read(input)
                .map_err(wrong)
                .and_then(|input| self.step(&input)),
            Op::FramePng => read(input)
                .map_err(wrong)
                .and_then(|input| self.frame_png(&input)),
            Op::Scene => read(input).map_err(wrong).and_then(|NoInput {}| {
                let output = self.scene()?;
                Ok((output, None))
            }),
            Op::Events => read(input).map_err(wrong).and_then(|NoInput {}| {
                let output = EventsOutput {
                    frames: attach::status(&mut self.link)?.frames,
                    events: Vec::new(),
                    sounds: Vec::new(),
                };
                let limits = vec![NO_EVENTS.to_owned()];
                Ok((to_json(&Answer::new(op, true, output, limits)), None))
            }),
            Op::Reset => Err(wrong(
                "the link never changes a running build: stop, then start with `project` to run \
                 the game headless"
                    .to_owned(),
            )),
            Op::Stop => read(input).map_err(wrong).and_then(|NoInput {}| {
                self.ended = true;
                let output = StopOutput {
                    frames: attach::status(&mut self.link)?.frames,
                };
                let limits = vec![format!("the build at {} runs on", self.address)];
                Ok((to_json(&Answer::new(op, true, output, limits)), None))
            }),
            _ => Err(wrong(format!("{} is not a session tool", op.name()))),
        };
        match answered {
            Ok(answered) => answered,
            Err((code, error)) => {
                if code == codes::LINK_LOST {
                    self.ended = true;
                }
                (failed(op, code, error), None)
            }
        }
    }

    /// `attach`'s input for the build: `steps` and `frames` after.
    fn input(&self, steps: Vec<AttachStep>, frames: u32) -> AttachInput {
        let mut input = AttachInput::default();
        input.address.clone_from(&self.address);
        input.token.clone_from(&self.token);
        input.steps = steps;
        input.frames = frames;
        input
    }

    /// `step`: `input` played over the link, answered as `run` answers.
    fn step(&mut self, input: &StepInput) -> Result<(Value, Option<String>), Failed> {
        let steps = input.script.steps.iter().map(AttachStep::of).collect();
        let asked = self.input(steps, input.frames);
        input
            .script
            .check()
            .and_then(|()| asked.check())
            .map_err(wrong)?;
        let seen = attach::played(&mut self.link, &asked)?;
        let mut limits = attach::limits_of(&asked, &seen);
        if seen.status.quit {
            limits.push("the build's game has quit".to_owned());
        }
        limits.extend([NO_SCENE, NO_EVENTS].map(str::to_owned));
        let Seen {
            status, frame, log, ..
        } = seen;
        let output = RunOutput {
            frames: status.frames,
            top_screen: status.top_screen,
            screens: status.screens,
            quit: None,
            scene: None,
            frame,
            events: Vec::new(),
            sounds: Vec::new(),
            log: log.lines.into_iter().map(log_entry).collect(),
            modules: Vec::new(),
        };
        Ok((to_json(&Answer::new(Op::Step, true, output, limits)), None))
    }

    /// `scene`: the frame's text in place of a scene, and the log.
    fn scene(&mut self) -> Result<Value, Failed> {
        let asked = self.input(Vec::new(), 0);
        let seen = attach::played(&mut self.link, &asked)?;
        let mut limits = attach::limits_of(&asked, &seen);
        limits.push(NO_SCENE.to_owned());
        let Seen {
            status, frame, log, ..
        } = seen;
        let scene = (status.phase != Phase::Loading && !frame.is_empty())
            .then(|| Scene::new(status.top_screen.clone(), "frame", frame));
        let output = SceneOutput {
            frames: status.frames,
            top_screen: status.top_screen,
            scene,
            log: log.lines.into_iter().map(log_entry).collect(),
        };
        let ok = output.scene.is_some();
        Ok(to_json(&Answer::new(Op::Scene, ok, output, limits)))
    }

    /// `frame_png`: the window's picture, at the window's size.
    fn frame_png(&mut self, input: &FramePngInput) -> Result<(Value, Option<String>), Failed> {
        if input.scale.is_some() {
            return Err(wrong(
                "scale: the window's picture has the window's size".to_owned(),
            ));
        }
        let status = attach::status(&mut self.link)?;
        let shot = attach::picture(&mut self.link)?;
        let output = FramePngOutput {
            frames: status.frames,
            top_screen: status.top_screen,
            width: shot.width,
            height: shot.height,
        };
        let limits = shot
            .limit
            .map(|limit| format!("the picture: {limit}"))
            .into_iter()
            .collect();
        let answer = to_json(&Answer::new(Op::FramePng, true, output, limits));
        Ok((answer, Some(shot.png)))
    }
}

/// A wrong input's failure: `error`, with [`codes::WRONG_ARGUMENTS`].
fn wrong(error: String) -> Failed {
    (codes::WRONG_ARGUMENTS, error)
}

/// A line of the build's log, as the session answers it.
fn log_entry(line: shuyi_platform::link::LogEntry) -> LogEntry {
    LogEntry {
        frame: line.frame,
        level: line.level,
        category: line.category,
        text: line.text,
    }
}

#[cfg(test)]
mod tests;
