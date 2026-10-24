//! The game's side of `shuyi mcp`: `shuyi mcp-host`, run in the game's
//! tool program, answers the server's [`Request`]s, one JSON line each on
//! stdin, with a [`Reply`] line each on stdout, after the mark [`REPLY`]. It answers the
//! operations that run the game with the game's tools, and keeps the
//! session's game ([`Host`]).
//!
//! [`InProcess`] is the same host without a process between, for games'
//! and the engine's tests of the server.

use std::io::{BufRead, Write};
use std::path::Path;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::server::{Launch, Link};
use super::{
    EventsOutput, FramePngInput, FramePngOutput, NoInput, StartInput, StartOption, StartOutput,
    StepInput, StopOutput, base64,
};
use crate::codec::encode_png;
use crate::contract::codes::{self, Code};
use crate::contract::import::Importers;
use crate::contract::run::{self, RunInput};
use crate::contract::session::Session;
use crate::contract::{
    Answer, Op, add, autobalance, bake, bots, check, doctor, explain, import, lint, modules, new,
    playtest, replay,
};
use crate::frame_png::MAX_SCALE;
use crate::game::{GameArg, GameTools, code_of};
use crate::project::{FILE, Project};

/// What starts each reply line, so the server tells replies from what
/// the game prints itself.
pub const REPLY: &str = "shuyi-mcp-reply ";

/// What the server asks the host: the tool of `op`, with its input.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    /// The operation, or the session tool.
    pub op: Op,
    /// Its input, as the tool's schema says.
    #[serde(default)]
    pub input: Value,
}

/// What the host answers.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reply {
    /// The answer, an [`Answer`] as JSON.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub answer: Option<Value>,
    /// A picture, as PNG in base64 (`frame_png`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub png: Option<String>,
    /// Why the host gave no answer: a request it doesn't read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl Reply {
    /// A reply of `answer`.
    fn of(answer: Value) -> Self {
        Self {
            answer: Some(answer),
            ..Self::default()
        }
    }
}

/// `answer` as JSON.
pub fn to_json<T: Serialize>(answer: &Answer<T>) -> Value {
    // An answer of strings, numbers and lists: serializing can't fail.
    serde_json::to_value(answer).unwrap_or(Value::Null)
}

/// The answer of `op` that failed with `code` because of `error`, as
/// JSON.
pub fn failed(op: Op, code: Code, error: impl Into<String>) -> Value {
    to_json(&Answer::<()>::failed(op, code, error))
}

/// `input` read as `I` (nothing given reads as `{}`), and `f`'s answer to
/// it; a [`codes::WRONG_ARGUMENTS`] answer of `op` if it doesn't read.
pub fn answered<I: DeserializeOwned, T: Serialize>(
    op: Op,
    input: Value,
    f: impl FnOnce(I) -> Answer<T>,
) -> Value {
    match read(input) {
        Ok(input) => to_json(&f(input)),
        Err(e) => failed(op, codes::WRONG_ARGUMENTS, e),
    }
}

/// `input` read as `I`, nothing given read as `{}`.
fn read<I: DeserializeOwned>(input: Value) -> Result<I, String> {
    let input = if input.is_null() { json!({}) } else { input };
    serde_json::from_value(input).map_err(|e| format!("input: {e}"))
}

/// The answer of the operation `op` (one of [`Op::ALL`]) to `input`, as
/// JSON: with the game's tools `game` on `project` where it runs the
/// game; `new` makes its folder from `cwd`, and `import` reads from the
/// project's root, or `cwd` outside one. Without the game's tools or a
/// project, an operation that needs them says so, as the CLI does.
pub fn one_off(
    op: Op,
    input: Value,
    cwd: &Path,
    project: &Result<Project, (Code, String)>,
    game: Option<&dyn GameTools>,
) -> Value {
    let no_game = || {
        let error = format!(
            "{FILE} names no game tool program (`game`), and {} runs the game",
            op.name()
        );
        failed(op, codes::PROJECT_FILE, error)
    };
    let needs = |f: &dyn Fn(&dyn GameTools, &Project) -> Value| match (game, project) {
        (_, Err((code, error))) => failed(op, *code, error.clone()),
        (None, Ok(_)) => no_game(),
        (Some(game), Ok(project)) => f(game, project),
    };
    match op {
        Op::New => answered(op, input, |i| new::new(cwd, &i)),
        Op::Doctor => answered(op, input, |i| doctor::doctor(&doctor::ThisMachine, &i)),
        Op::Explain => answered(op, input, |i| explain::explain(&i)),
        Op::Check => {
            let input: check::CheckInput = match read(input) {
                Ok(input) => input,
                Err(e) => return failed(op, codes::WRONG_ARGUMENTS, e),
            };
            match (&input.target, project) {
                (Some(_), Ok(p)) => to_json(&check::targets(&p.config, &p.root, &input, game)),
                (None, _) => needs(&|game, _| to_json(&check::check(game, &input))),
                (Some(_), Err((code, error))) => failed(op, *code, error.clone()),
            }
        }
        Op::Import => {
            let (importers, limit) = match (game, project) {
                (Some(game), _) => (game.importers(), None),
                (None, Ok(_)) => (
                    Importers::standard(),
                    Some(format!(
                        "{FILE} names no game tool program: only the engine's standard kinds \
                         were tried"
                    )),
                ),
                (None, Err(_)) => (
                    Importers::standard(),
                    Some("no project here: only the engine's standard kinds were tried".to_owned()),
                ),
            };
            let root = project.as_ref().map_or(cwd, |p| p.root.as_path());
            answered(op, input, |i| {
                let answer = import::import(&importers, root, &i);
                match &limit {
                    Some(limit) => answer.limited(limit),
                    None => answer,
                }
            })
        }
        Op::Lint => match project {
            Ok(p) => answered(op, input, |i| lint::lint(&lint::ThisMachine, p, &i)),
            Err((code, error)) => failed(op, *code, error.clone()),
        },
        Op::Modules => match project {
            Ok(p) => answered(op, input, |i| modules::modules(p, &i)),
            Err((code, error)) => failed(op, *code, error.clone()),
        },
        Op::Add => match project {
            Ok(p) => answered(op, input, |i| add::add(p, &i)),
            Err((code, error)) => failed(op, *code, error.clone()),
        },
        Op::Run => needs(&|game, _| answered(op, input.clone(), |i| run::run(game, &i))),
        Op::Scene => needs(&|game, _| answered(op, input.clone(), |i| run::scene(game, &i))),
        Op::Replay => {
            needs(&|game, p| answered(op, input.clone(), |i| replay::replay(game, &p.root, &i)))
        }
        Op::Bots => needs(&|game, _| answered(op, input.clone(), |i| bots::bots(game, &i))),
        Op::Playtest => {
            needs(&|game, _| answered(op, input.clone(), |i| playtest::playtest(game, &i)))
        }
        Op::Autobalance => {
            needs(&|game, _| answered(op, input.clone(), |i| autobalance::autobalance(game, &i)))
        }
        Op::Bake => needs(&|game, p| answered(op, input.clone(), |i| bake::bake(game, p, &i))),
        Op::Start | Op::Step | Op::FramePng | Op::Events | Op::Reset | Op::Stop => failed(
            op,
            codes::WRONG_ARGUMENTS,
            format!("{} is a session tool, not an operation", op.name()),
        ),
    }
}

/// The session's game: the options it started with, and the game.
struct Live {
    start: Vec<GameArg>,
    game: Box<dyn Session>,
}

/// The game's side of the server: the game's tools on its project, and
/// the session's game, if one is running.
pub struct Host<'a> {
    tools: &'a dyn GameTools,
    project: Project,
    live: Option<Live>,
}

impl<'a> Host<'a> {
    /// A host of `tools` on `project`, with no game running.
    pub fn new(tools: &'a dyn GameTools, project: Project) -> Self {
        Self {
            tools,
            project,
            live: None,
        }
    }

    /// The reply to `request`.
    pub fn call(&mut self, request: &Request) -> Reply {
        let (op, input) = (request.op, request.input.clone());
        match op {
            Op::Start => Reply::of(answered(op, input, |i: StartInput| self.start(op, i.start))),
            Op::Reset => match (read::<NoInput>(input), &self.live) {
                (Err(e), _) => Reply::of(failed(op, codes::WRONG_ARGUMENTS, e)),
                (Ok(_), Some(live)) => {
                    let start = live.start.clone();
                    Reply::of(to_json(&self.start(op, start)))
                }
                (Ok(_), None) => Reply::of(no_session(op)),
            },
            Op::Step => self.with_game(op, |game| {
                answered(op, input, |i: StepInput| step(game, &i))
            }),
            Op::Scene => self.with_game(op, |game| {
                answered(op, input, |NoInput {}| {
                    let (output, limits) = run::scene_output(game);
                    Answer::new(op, output.scene.is_some(), output, limits)
                })
            }),
            Op::Events => self.with_game(op, |game| {
                answered(op, input, |NoInput {}| {
                    let output = EventsOutput {
                        frames: game.frame_count(),
                        events: game.events(),
                        sounds: game.sounds(),
                    };
                    Answer::new(op, true, output, Vec::new())
                })
            }),
            Op::FramePng => match self.live.as_ref() {
                Some(live) => frame_png(&*live.game, input),
                None => Reply::of(no_session(op)),
            },
            Op::Stop => match (read::<NoInput>(input), self.live.take()) {
                (Err(e), live) => {
                    self.live = live;
                    Reply::of(failed(op, codes::WRONG_ARGUMENTS, e))
                }
                (Ok(_), Some(live)) => {
                    let output = StopOutput {
                        frames: live.game.frame_count(),
                    };
                    Reply::of(to_json(&Answer::new(op, true, output, Vec::new())))
                }
                (Ok(_), None) => Reply::of(no_session(op)),
            },
            _ => Reply::of(one_off(
                op,
                input,
                &self.project.root,
                &Ok(self.project.clone()),
                Some(self.tools),
            )),
        }
    }

    /// The reply of `f` on the session's game, or the answer of `op` that
    /// there is none.
    fn with_game(&mut self, op: Op, f: impl FnOnce(&mut dyn Session) -> Value) -> Reply {
        match self.live.as_mut() {
            Some(live) => Reply::of(f(&mut *live.game)),
            None => Reply::of(no_session(op)),
        }
    }

    /// The session's game started as `start` says, for `start` or `reset`
    /// (`op`).
    fn start(&mut self, op: Op, start: Vec<GameArg>) -> Answer<StartOutput> {
        let game = match self.tools.session(&start) {
            Ok(game) => game,
            Err(e) => return Answer::failed(op, code_of(&e), e),
        };
        let screens = game.screens();
        let output = StartOutput {
            project: self.project.root.display().to_string(),
            start: start.clone(),
            options: self
                .tools
                .frame_options()
                .iter()
                .map(StartOption::from)
                .collect(),
            frames: game.frame_count(),
            top_screen: screens.last().cloned().unwrap_or_default(),
            screens,
        };
        self.live = Some(Live { start, game });
        Answer::new(op, true, output, Vec::new())
    }
}

/// The answer of session tool `op` with no game running.
pub fn no_session(op: Op) -> Value {
    failed(
        op,
        codes::NO_SESSION,
        "no game is running: call `start` first",
    )
}

/// `step`: `input` played on `game`, answered as `run` answers.
fn step(game: &mut dyn Session, input: &StepInput) -> Answer<run::RunOutput> {
    let checked = RunInput {
        start: Vec::new(),
        script: input.script.clone(),
        frames: input.frames,
    }
    .check();
    if let Err(e) = checked {
        return Answer::failed(Op::Step, codes::WRONG_ARGUMENTS, e);
    }
    game.play(&input.script);
    game.idle(input.frames);
    let (output, limits) = run::run_output(game);
    Answer::new(Op::Step, true, output, limits)
}

/// `frame_png`: `game`'s frame painted as `input` says.
fn frame_png(game: &dyn Session, input: Value) -> Reply {
    let op = Op::FramePng;
    let input: FramePngInput = match read(input) {
        Ok(input) => input,
        Err(e) => return Reply::of(failed(op, codes::WRONG_ARGUMENTS, e)),
    };
    if let Some(scale) = input.scale.filter(|s| !(1..=MAX_SCALE).contains(s)) {
        let error = format!("scale: 1 to {MAX_SCALE}, not {scale}");
        return Reply::of(failed(op, codes::WRONG_ARGUMENTS, error));
    }
    let image = match game.picture(input.scale) {
        Ok(image) => image,
        Err(e) => return Reply::of(failed(op, code_of(&e), e)),
    };
    let png = match encode_png(image.width, image.height, &image.rgba) {
        Ok(png) => png,
        Err(e) => return Reply::of(failed(op, codes::GAME_ERROR, format!("the picture: {e}"))),
    };
    let screens = game.screens();
    let output = FramePngOutput {
        frames: game.frame_count(),
        top_screen: screens.last().cloned().unwrap_or_default(),
        width: image.width,
        height: image.height,
    };
    Reply {
        answer: Some(to_json(&Answer::new(op, true, output, Vec::new()))),
        png: Some(base64(&png)),
        error: None,
    }
}

/// `shuyi mcp-host`: answers each request line of `input` on `output`
/// with `game`'s tools on `project`, until `input` ends. Returns the exit
/// code: 0, or 1 if a line couldn't be read or written.
pub fn serve(
    game: &dyn GameTools,
    project: Project,
    input: impl BufRead,
    mut output: impl Write,
) -> u8 {
    let mut host = Host::new(game, project);
    for line in input.lines() {
        let Ok(line) = line else { return 1 };
        if line.trim().is_empty() {
            continue;
        }
        let reply = match serde_json::from_str::<Request>(&line) {
            Ok(request) => host.call(&request),
            Err(e) => Reply {
                error: Some(format!(
                    "the game's tool program doesn't read the request ({e}): is its engine older \
                     than the server's?"
                )),
                ..Reply::default()
            },
        };
        // A reply of strings, numbers and lists: serializing can't fail.
        let line = serde_json::to_string(&reply).unwrap_or_default();
        if writeln!(output, "{REPLY}{line}")
            .and_then(|()| output.flush())
            .is_err()
        {
            return 1;
        }
    }
    0
}

/// The host in this process: a [`Launch`] whose game is `tools`, with no
/// process between, for tests of the server.
pub struct InProcess<'a> {
    tools: &'a dyn GameTools,
}

impl<'a> InProcess<'a> {
    /// Launches hosts of `tools`.
    pub fn new(tools: &'a dyn GameTools) -> Self {
        Self { tools }
    }
}

impl<'a> Launch<'a> for InProcess<'a> {
    fn launch(&self, project: &Project) -> Result<Box<dyn Link + 'a>, (Code, String)> {
        Ok(Box::new(Host::new(self.tools, project.clone())))
    }
}

impl Link for Host<'_> {
    fn call(&mut self, request: &Request) -> Result<Reply, String> {
        Ok(Host::call(self, request))
    }
}

#[cfg(test)]
mod tests;
