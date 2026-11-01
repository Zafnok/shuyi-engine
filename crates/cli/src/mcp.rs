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
//! | `perf` | the frames' records since a number, and their summary; a heap snapshot and a diff between two | [`PerfOutput`] |
//! | `reset` | the game started again as `start` started it | [`StartOutput`] |
//! | `stop` | the game stopped | [`StopOutput`] |
//!
//! `start` with `link` attaches the session to a debug build already
//! running in its window instead, over the live link ([`attached`]).
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
pub mod attached;
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

use crate::contract::perf::PerfSummary;
use crate::contract::session::{Event, Sound};
use crate::contract::{Op, VERSION, op_schemas};
use crate::frame_png::Script;
use crate::game::{GameArg, GameOption};
use crate::project::Project;
use shuyi_ui::perf::{FrameRecord, HeapDiff, HeapSnapshot, PERF_FRAMES};

/// A session tool's input that takes nothing.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NoInput {}

/// What `start` starts.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, default)]
// An input: built with `default()` and its fields assigned, it may grow
// (ADR-0039).
#[non_exhaustive]
pub struct StartInput {
    /// The game's folder (the one holding its `shuyi.ron`, or one below
    /// it), from the session's folder; the session folder's own project
    /// if none. Not needed with `link`.
    pub project: Option<String>,
    /// The game's own start options (`start`'s answer lists them), in the
    /// order given. None with `link`: the build runs with its own.
    pub start: Vec<GameArg>,
    /// A debug build already running in its window, to attach the session
    /// to over the live link instead of starting the game headless.
    pub link: Option<StartLink>,
}

/// The running build `start` attaches to: its log line `shuyi link:
/// listening on <address>, token <token>` gives both.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, default)]
// An input: built with `default()` and its fields assigned, it may grow
// (ADR-0039).
#[non_exhaustive]
pub struct StartLink {
    /// The build's address, `host:port` (`127.0.0.1:7357`).
    pub address: String,
    /// The token the build logged when it started listening.
    pub token: String,
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
// An answer, read field by field: it may grow (ADR-0039).
#[non_exhaustive]
pub struct StartOutput {
    /// The game's folder; empty when attached over the link.
    pub project: String,
    /// The running build's game, its window's title, when attached over
    /// the link; empty when started headless.
    pub game: String,
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

/// How many records `perf` answers when its input doesn't say.
pub const PERF_MAX: u32 = 120;

/// How many heap snapshots a session keeps for `perf`'s `diff_from`: the
/// last, by frame.
pub const KEPT_SNAPSHOTS: usize = 8;

/// Which of the session's frames' records `perf` reads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, default)]
// An input: built with `default()` and its fields assigned, it may grow
// (ADR-0039).
#[non_exhaustive]
pub struct PerfInput {
    /// The first record's number: 0, or the last answer's `next`.
    pub since: u64,
    /// The most records to answer, 1 to 600.
    #[schemars(range(min = 1, max = 600))]
    pub max: u32,
    /// Take a heap snapshot now: what holds the heap by memory tag. The
    /// session keeps its last 8, by frame.
    pub snapshot: bool,
    /// The frame of a kept snapshot: answer the diff from it to the
    /// snapshot taken now, or, without `snapshot`, to the newest kept.
    pub diff_from: Option<u32>,
}

impl Default for PerfInput {
    fn default() -> Self {
        Self {
            since: 0,
            max: PERF_MAX,
            snapshot: false,
            diff_from: None,
        }
    }
}

impl PerfInput {
    /// Whether the session can answer it: `max` from 1 to 600.
    pub fn check(&self) -> Result<(), String> {
        let most = u32::try_from(PERF_FRAMES).unwrap_or(u32::MAX);
        if (1..=most).contains(&self.max) {
            Ok(())
        } else {
            Err(format!("max: 1 to {most}, not {}", self.max))
        }
    }
}

/// What `perf` read of the session's frames.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
// An answer, read field by field: it may grow (ADR-0039).
#[non_exhaustive]
pub struct PerfOutput {
    /// The frames' records numbered `since` or more, oldest first.
    pub records: Vec<FrameRecord>,
    /// The number to pass as `since` next time.
    pub next: u64,
    /// How many records numbered `since` or more were no longer kept.
    pub dropped: u64,
    /// What the records answered add up to; none when there are none.
    pub summary: Option<PerfSummary>,
    /// The heap snapshot taken now, with `snapshot`: null without it, or
    /// where the heap isn't counted by tag.
    pub snapshot: Option<HeapSnapshot>,
    /// What changed from the snapshot of `diff_from`, with it.
    pub diff: Option<HeapDiff>,
    /// The frames of the snapshots the session keeps, oldest first: the
    /// `diff_from` it takes.
    pub kept: Vec<u32>,
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

/// What `attach`'s tool does.
const ATTACH: &str = "Reach a game's debug build already running in its window, over the live link \
     (`address` and `token`: its log line `shuyi link: listening on ..., token ...`; the \
     build runs with its `link` feature and SHUYI_LINK set). Play `steps` as a player's \
     input (keys, pad buttons, an action by name with `act`, waits in its frames), let \
     `frames` pass, then read its screens, frame as text, log from `log_since`, and, \
     with `screenshot` (a PNG's path), the window's picture; with `perf_since`, its \
     frames' records and their summary; with `perf_follow` (a file), each new frame's \
     record written to the file for `follow_for` seconds, then their summary.";

/// What `start`'s tool does.
const START: &str = "Build the game's tool program and start its game headless in the session, as \
     its start options say (the answer lists them); a game already running is stopped. \
     `project`: the game's folder. Call it again after changing the game's code. With `link` \
     (`address` and `token`, from a debug build's log line `shuyi link: listening on ..., token \
     ...`), attach the session to that build running in its window instead: the session tools \
     play a player's input on it and read its frame, picture and log (no scene, events or \
     sounds), `reset` is refused, and `stop` leaves it running.";

/// What `export`'s tool does.
const EXPORT: &str = "Check the game against `target` (`windows`, `linux`, `macos`, `web`), then \
     build and package it there: a zip, a tar.gz, a signed .app or a web folder and its zip, \
     each with NOTICES.txt, and export-report.json and exposure-report.json beside it, in `out` \
     (dist/export/<target> by default). A package that breaks shuyi.ron's `disclosure` part is \
     removed. `release` remaps the build's paths and keeps the debug file beside the package; \
     `run` runs the package after.";

/// `lint`'s description.
const LINT: &str = "Check the game's code: keys named in code or text (check-keys), screen text as literals \
     (check-text), and the rules crate's fence (clippy's bans on hash maps, floats, clocks, \
     files and outside luck; no screen or device crate under it). Each finding with its \
     file, line and code.";

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
        (Op::Lint, _) => LINT,
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
        (Op::Attach, _) => ATTACH,
        (Op::Export, _) => EXPORT,
        (Op::Start, _) => START,
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
        (Op::Perf, _) => {
            "The session's frames' records (times in microseconds, the game's spans) numbered \
             `since` or more, at most `max`, and their summary: a stream, `step` then `perf` with \
             the last `next`. `snapshot` takes a heap snapshot (live bytes by memory tag; the \
             last 8 are `kept`); `diff_from`, a kept one's frame, answers what grew since."
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
    shuyi_platform::link::base64::encode(bytes)
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
        let cargo = crate::cargo();
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
    // It reads on a thread of its own, whose allocations the heap counter
    // counts with the game's.
    crate::contract::perf::note_other_thread();
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
