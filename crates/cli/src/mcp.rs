//! `shuyi mcp`: the tool contract as an MCP server over stdio (ADR-0005
//! §2, ADR-0031), and `shuyi mcp install`, which registers it with the
//! agents it finds ([`install`]).
//!
//! Each operation of the contract is a tool, its input the operation's
//! input ([`crate::contract::op_schemas`]), its answer the operation's
//! [`Answer`](crate::contract::Answer). The tools that run the game take
//! `project`, the game's folder, from the session's folder (the client's
//! first root, or the server's working folder); the session folder's own
//! project if it is left out. A project in another cargo workspace than
//! the server's is answered by that workspace's own server ([`forward`]).
//!
//! A **session** keeps one game running headless between calls, so an
//! agent can play it, look at it and play on:
//!
//! | Tool | Does | Answers as |
//! | ---- | ---- | ---------- |
//! | `start` | builds the game's tool program and starts its game as its options say | [`StartOutput`] |
//! | `step` | plays a script, then runs idle frames | `run`'s answer |
//! | `scene` | the scene on screen now | `scene`'s answer |
//! | `frame_png` | a picture of the frame now, as an image | [`FramePngOutput`] and the PNG |
//! | `events` | what happened since the start | [`EventsOutput`] |
//! | `reset` | the game started again as `start` started it | [`StartOutput`] |
//! | `stop` | the game stopped | [`StopOutput`] |
//!
//! The one-off `scene` operation is the session's `scene` here: `run`
//! answers with the scene of a one-off run.
//!
//! The server runs no game itself. A tool that runs the game runs it in
//! the game's tool program (`shuyi.ron`'s `game`), started with `cargo
//! run` as `shuyi mcp-host` ([`host`]); the session keeps that program
//! running until `stop`, so `start` builds the game's latest code. A
//! failed answer is an MCP error result with the answer as it is, its
//! `code` and its `help`.

pub mod aside;
pub mod child;
pub mod forward;
pub mod host;
pub mod install;
pub mod server;

use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::Command;
use std::sync::mpsc::{RecvTimeoutError, channel};
use std::time::Duration;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use crate::contract::session::{Event, Sound};
use crate::contract::{Op, VERSION, op_schemas};
use crate::frame_png::Script;
use crate::game::{GameArg, GameOption};
use crate::project::Project;

/// A session tool's input that takes nothing.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NoInput {}

/// What `start` starts.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, default)]
pub struct StartInput {
    /// The game's folder (the one holding its `shuyi.ron`, or one below
    /// it), from the session's folder; the session folder's own project
    /// if none.
    pub project: Option<String>,
    /// The game's own start options (`start`'s answer lists them), in the
    /// order given.
    pub start: Vec<GameArg>,
}

/// A start option the game offers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StartOption {
    /// Its flag (`--stage`).
    pub flag: String,
    /// What its value is called (`N`), if it takes one.
    pub value: Option<String>,
    /// What it does.
    pub help: String,
}

impl From<&GameOption> for StartOption {
    fn from(option: &GameOption) -> Self {
        Self {
            flag: option.flag.to_owned(),
            value: option.value.map(str::to_owned),
            help: option.help.to_owned(),
        }
    }
}

/// What `start` and `reset` started.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StartOutput {
    /// The game's folder.
    pub project: String,
    /// The start options it was started with.
    pub start: Vec<GameArg>,
    /// The start options the game offers.
    pub options: Vec<StartOption>,
    /// The frames run (none yet).
    pub frames: u32,
    /// The top screen's name.
    pub top_screen: String,
    /// The open screens, bottom first.
    pub screens: Vec<String>,
}

/// What `step` plays.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, default)]
pub struct StepInput {
    /// The steps to play, as `run`'s script.
    pub script: Script,
    /// Frames to run after the script with no input.
    pub frames: u32,
}

/// How `frame_png` paints the frame.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, default)]
pub struct FramePngInput {
    /// Pixels per console pixel, 1 to 8; the game's choice if none.
    pub scale: Option<u32>,
}

/// What `frame_png` painted (the picture itself is the tool's image).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FramePngOutput {
    /// The frames run in all.
    pub frames: u32,
    /// The top screen's name.
    pub top_screen: String,
    /// The picture's width in pixels.
    pub width: u32,
    /// Its height in pixels.
    pub height: u32,
}

/// What `events` saw.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EventsOutput {
    /// The frames run in all.
    pub frames: u32,
    /// What happened since the start, in frame order, then the game's own
    /// events.
    pub events: Vec<Event>,
    /// The sound cues asked for since the start, in order.
    pub sounds: Vec<Sound>,
}

/// What `stop` stopped.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StopOutput {
    /// The frames the game had run.
    pub frames: u32,
}

/// The tool named `name`: its operation, and whether it is a session
/// tool ([`Op::SESSION`]).
pub fn tool(name: &str) -> Option<(Op, bool)> {
    if let Some(op) = Op::SESSION.into_iter().find(|op| op.name() == name) {
        return Some((op, true));
    }
    Op::ALL
        .into_iter()
        .find(|op| op.name() == name)
        .map(|op| (op, false))
}

/// What the tool of `op` does, for its description.
fn description(op: Op, session: bool) -> &'static str {
    match (op, session) {
        (Op::New, _) => {
            "Make a new game in `dir` (from the server's working folder) from the engine's \
             template, its parts picked by the answers: `template`, a short name (`grid`, a \
             title and a turn-based grid stage with a bot and records; `blank`, one screen), \
             then `answers` (an answers file) and `set` (answers by question). `force` writes \
             into a folder that isn't empty."
        }
        (Op::Check, _) => {
            "Load the game's content through every validator: each error with its file, line and \
             code. With `baked`, its pack too; with `target`, whether its frame, text and \
             controls fit a target (or `all`)."
        }
        (Op::Lint, _) => {
            "Check the game's code: keys named in code or text (check-keys), screen text              written as literals (check-text), and the rules crate's fence (clippy's bans on hash              maps, floats, clocks, files and outside luck; no screen or device crate under it).              Each finding with its file, line and code."
        }
        (Op::Modules, _) => {
            "The engine's catalog of modules by concern (perspective, bodies and motion, online \
             play, ...): each answer, who gives it (the default engine or a module) and its \
             status, the modules the game's engine builds with what they answer and serve, and \
             the ones the game has on."
        }
        (Op::Add, _) => {
            "Turn a module of the game's engine on (`module`: `physics` or `shuyi-physics`): its \
             feature of `shuyi` in the game's Cargo.toml (it changes the project; a second call \
             changes nothing), its skill copied into the game's skills, what it is for, an \
             alternative already on, and any line the game's code still needs."
        }
        (Op::Run, _) => {
            "Start the game headless as its start options say, play a script of keys, pad \
             buttons and waits, run idle frames: the screens, the scene, the frame as text, what \
             happened, the sounds and the log. A fresh game each call; the session tools keep \
             one running."
        }
        (Op::Scene, false) => "The scene on screen after a one-off run.",
        (Op::Scene, true) => {
            "The scene on screen in the session now (a stage's map and state), and the log. Not \
             ok when no stage is open."
        }
        (Op::Replay, _) => {
            "Re-run a record (a file in the game's folder) by the game's rules: a state hash per \
             step; with `other`, the first step where two runs differ."
        }
        (Op::Bots, _) => {
            "The game's bots play `games` games: how each ended, how long it took, the game's \
             measures."
        }
        (Op::Playtest, _) => {
            "Each of the game's named bots (its `entrants`, or the game's choice) plays `tries` \
             tries of a `stage` (or `every_stage`) on every core; the game's checks say per bot \
             whether it succeeded as often as its band wants, and each bot's time per try."
        }
        (Op::Autobalance, _) => {
            "Change a `stage` with the game's knobs, never its bots, until each of its named \
             bots (`entrants`) succeeds as often as its checks' bands want: the smallest change \
             the search finds, in the knobs' words, and the reports before and after. Each \
             setting's tries run on every core; it stops at `budget` evaluations or after \
             `time` seconds. With `apply`, the game writes the changed stage to its files: it \
             changes the project."
        }
        (Op::Import, _) => {
            "Read a file (from the game's folder) through the format registry as every kind \
             that knows it, or give its errors."
        }
        (Op::Bake, _) => {
            "Bake the game's sources into one pack: what went in, the sizes, how long baking and \
             loading took."
        }
        (Op::Doctor, _) => {
            "Check the toolchain (Rust, Cargo, targets, the linker) and say how to fix each \
             missing piece."
        }
        (Op::Explain, _) => {
            "Every problem code, or one code's page (`SH2003` or `missing-file`): what it means \
             and how to fix it. Every failed answer has a `code`."
        }
        (Op::Start, _) => {
            "Build the game's tool program and start its game headless in the session, as its \
             start options say (the answer lists them); a game already running is stopped. \
             `project`: the game's folder. Call it again after changing the game's code."
        }
        (Op::Step, _) => {
            "Play on in the session: a script of keys, pad buttons and waits, then `frames` idle \
             frames. Answers as `run` does, for everything since the start."
        }
        (Op::FramePng, _) => {
            "A picture of the session's frame now, as the game paints it, as an image."
        }
        (Op::Events, _) => {
            "What happened in the session since the start: screens opened and closed, music, \
             voices, the quit, the game's own events, and the sounds."
        }
        (Op::Reset, _) => {
            "Start the session's game again as `start` started it, without building again."
        }
        (Op::Stop, _) => "Stop the session's game.",
    }
}

/// Whether the tool of `op` takes `project`: the operations that run the
/// game, and `lint`, `modules` and `add`.
pub fn takes_project(op: Op, session: bool) -> bool {
    !session
        && (crate::contract::cli::runs_the_game(op.name())
            || matches!(op, Op::Lint | Op::Modules | Op::Add))
}

/// The schema of `project`, which every tool that runs the game takes.
fn project_schema() -> Value {
    json!({
        "description": "The game's folder (the one holding its `shuyi.ron`, or one below it), \
                        from the session's folder (the agent's working folder); the session \
                        folder's own project if none.",
        "type": ["string", "null"],
    })
}

/// A schema file's JSON, with `project` added to its properties if
/// `project`.
fn schema_value(text: &str, project: bool) -> Value {
    let mut schema: Value = serde_json::from_str(text).unwrap_or_else(|_| json!({}));
    if project
        && let Some(properties) = schema.as_object_mut().and_then(|schema| {
            schema
                .entry("properties")
                .or_insert_with(|| json!({}))
                .as_object_mut()
        })
    {
        properties.insert("project".to_owned(), project_schema());
    }
    schema
}

/// The tool of `op`, as `tools/list` lists it.
fn tool_listing(op: Op, session: bool) -> Value {
    let [input, output] = op_schemas(op, session);
    let mut tool = Map::new();
    tool.insert("name".to_owned(), json!(op.name()));
    tool.insert("description".to_owned(), json!(description(op, session)));
    tool.insert(
        "inputSchema".to_owned(),
        schema_value(&input.text, takes_project(op, session)),
    );
    tool.insert("outputSchema".to_owned(), schema_value(&output.text, false));
    Value::Object(tool)
}

/// Every tool, as `tools/list` lists them: the operations ([`Op::ALL`],
/// the one-off `scene` left out), then the session's ([`Op::SESSION`]).
pub fn tools() -> Vec<Value> {
    let ops = Op::ALL
        .into_iter()
        .filter(|op| *op != Op::Scene)
        .map(|op| tool_listing(op, false));
    let session = Op::SESSION.into_iter().map(|op| tool_listing(op, true));
    ops.chain(session).collect()
}

/// `bytes` in standard base64, with padding: how MCP carries an image.
pub fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = u32::from_be_bytes([
            0,
            chunk[0],
            chunk.get(1).copied().unwrap_or(0),
            chunk.get(2).copied().unwrap_or(0),
        ]);
        for (i, shift) in [18, 12, 6, 0].into_iter().enumerate() {
            if i <= chunk.len() {
                out.push(char::from(ALPHABET[(n >> shift) as usize & 63]));
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// The usage of `shuyi mcp`, with `command` as the tool's name.
pub fn usage(command: &str) -> String {
    format!(
        "\
usage: {command} mcp                    serve the tool contract over MCP on stdin and stdout
       {command} mcp --ready            build what the server needs, then exit
       {command} mcp install [--dry-run] [--yes] [--command \"<how to run shuyi>\"]
                                     register the server with the agents found here

--ready builds the server (it runs through the same command) and the
game's tool program, if shuyi.ron names one, so an agent's first
connection doesn't wait for a compile. A running server follows the
agent's folder and the engine by itself: a project in another workspace
goes to that workspace's own server, and a server whose engine moved
under it hands its calls to a fresh one. install shows what it will
change and asks before writing, keeping a backup of each file it
changes, then runs the server with --ready; --yes doesn't ask, and
--dry-run writes nothing and prints each file as it would write it. The server is run as --command's
words and `mcp` (the project's `command` in shuyi.ron, or `shuyi`, if not
given)."
    )
}

/// `shuyi mcp` with `args`, in the working folder `cwd`: the server on
/// this process's stdin and stdout, or `install`. Returns the exit code.
pub fn main(args: &[String], cwd: &Path) -> u8 {
    match args.split_first() {
        None => {
            if cfg!(windows) {
                // So a build while it runs can write a fresh program.
                aside::this_program();
            }
            let mut server = server::Server::new(cwd.to_path_buf(), Box::new(child::Cargo));
            if std::env::var_os(forward::FORWARDED).is_none() {
                server = server.forwarding(Box::new(forward::Processes));
            }
            let stdout = std::io::stdout();
            match serve(&mut server, BufReader::new(std::io::stdin()), stdout.lock()) {
                Ok(()) => 0,
                Err(e) => {
                    eprintln!("mcp: {e}");
                    1
                }
            }
        }
        Some((first, rest)) if first == "install" => install::main(rest, cwd),
        Some((first, [])) if first == "--ready" => ready(cwd),
        Some(_) => {
            eprintln!("{}", usage("shuyi"));
            2
        }
    }
}

/// `shuyi mcp --ready` in `cwd`: the server is built (this process is
/// it), and so is the game's tool program if the project names one, with
/// cargo's output passed through. Returns the exit code: 1 if the program
/// didn't build.
fn ready(cwd: &Path) -> u8 {
    if let Ok(project) = Project::find(cwd)
        && let Some(program) = &project.config.game
    {
        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
        let built = Command::new(cargo)
            .args(program.cargo_build_args())
            .current_dir(&project.root)
            .status();
        match built {
            Ok(status) if status.success() => {}
            Ok(status) => {
                let code = status
                    .code()
                    .map_or_else(String::new, |c| format!(" (exit code {c})"));
                eprintln!("mcp: the game's tool program didn't build{code}");
                return 1;
            }
            Err(e) => {
                eprintln!("mcp: cannot run cargo: {e}");
                return 1;
            }
        }
    }
    println!("mcp: ready (contract {VERSION})");
    0
}

/// How long the server waits for a message before it looks whether a
/// project's own server it follows is ready.
const POLL: Duration = Duration::from_millis(200);

/// Serves MCP messages from `input`, one per line, writing each message
/// the server has for one on a line of its own, and the notification that
/// its tools changed when they do ([`server::Server::poll`]), until `input`
/// ends.
pub fn serve(
    server: &mut server::Server<'_>,
    input: impl BufRead + Send + 'static,
    mut output: impl Write,
) -> std::io::Result<()> {
    let (sent, lines) = channel();
    std::thread::spawn(move || {
        for line in input.lines() {
            if sent.send(line).is_err() {
                break;
            }
        }
    });
    loop {
        let messages = match lines.recv_timeout(POLL) {
            Ok(line) => {
                let mut messages = server.handle(&line?);
                messages.extend(server.poll());
                messages
            }
            Err(RecvTimeoutError::Timeout) => server.poll(),
            Err(RecvTimeoutError::Disconnected) => return Ok(()),
        };
        for message in &messages {
            writeln!(output, "{message}")?;
        }
        output.flush()?;
    }
}

#[cfg(test)]
mod tests;
