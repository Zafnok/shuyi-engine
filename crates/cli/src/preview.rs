//! `preview`: the game played by visitors as pictures from this machine,
//! never as a program they download (ADR-0141). A visitor presses the
//! game's actions and gets the frame after each one back, as a PNG with
//! the text it shows; the program, its files, its scene and its state
//! stay here. What a visitor sees they can still record and copy.
//!
//! It serves HTTP ([`server`]), on this machine's own address by default,
//! and answers four requests, every other path with `404`:
//!
//! - `POST /session`: a new session, its worker started; its first frame;
//! - `POST /session/<id>` with `{"action": "<name>"}`: the action pressed,
//!   and the frame after it;
//! - `DELETE /session/<id>`: the session ended.
//!
//! A frame is JSON ([`worker::Shown`] and the session's id): `png`, the
//! picture in base64; `text`, the text it shows; `actions`, the actions a
//! visitor may press; `ended`, whether the game quit. A refusal is
//! `{"error": "…"}` with its status. Nothing serves a script, a frame
//! count, a scene, a snapshot, a log or a file.
//!
//! Each session is a **worker**: this program again, run as
//! `preview-worker` ([`worker`]), which starts the game's session as the
//! server's options say, the same for every visitor. It takes the actions
//! a player can rebind, with the developer's debug tools off
//! ([`Session::for_visitor`](crate::contract::session::Session::for_visitor)),
//! and runs at most `--frames` frames after each, until the frame holds
//! still. The server bounds each session by [`Limits`]: the sessions at
//! once, per address and started a minute; the actions and their rate;
//! idle and whole time; the time one call may take and all the game's
//! time; a worker's memory, where the system says it (Linux); and the bytes
//! its answers send.

mod http;
pub(crate) mod server;
pub(crate) mod worker;

#[cfg(test)]
mod tests;

use std::net::{IpAddr, Ipv4Addr, TcpListener};
use std::sync::Arc;
use std::time::Duration;

use shuyi_platform::harness::MAX_FRAMES;

use crate::contract::codes::{self, Code};
use crate::frame_png::MAX_SCALE;
use crate::game::{GameArg, GameOption, GameTools, game_arg};
use crate::project::Project;
use server::{Server, Worker};

/// The limits a server keeps, each a flag of `preview` ([`usage`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Limits {
    /// The most sessions at once.
    pub(crate) sessions: usize,
    /// The most sessions of one address at once.
    pub(crate) per_address: usize,
    /// The most sessions one address may start a minute.
    pub(crate) starts: u32,
    /// The most actions one session may press.
    pub(crate) actions: u32,
    /// The most actions a session may press each second.
    pub(crate) rate: u32,
    /// How long a session may sit idle.
    pub(crate) idle: Duration,
    /// How long a session may last.
    pub(crate) lifetime: Duration,
    /// How long a start or an action may take.
    pub(crate) deadline: Duration,
    /// How long a session's starts and actions may take in all: the
    /// machine's time it may use.
    pub(crate) cpu: Duration,
    /// The most memory a worker may hold, in bytes, where the system says.
    pub(crate) memory: u64,
    /// The most bytes a session's answers may send.
    pub(crate) egress: u64,
    /// The most connections served at once.
    pub(crate) connections: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            sessions: 8,
            per_address: 2,
            starts: 10,
            actions: 1_000,
            rate: 8,
            idle: Duration::from_secs(120),
            lifetime: Duration::from_mins(30),
            deadline: Duration::from_secs(5),
            cpu: Duration::from_secs(120),
            memory: 512 << 20,
            egress: 64 << 20,
            connections: 64,
        }
    }
}

/// `preview`'s arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Options {
    /// The address it listens on.
    pub(crate) bind: IpAddr,
    /// Its port: 0 for any free one.
    pub(crate) port: u16,
    /// What it bounds.
    pub(crate) limits: Limits,
    /// The most frames an action runs before its picture.
    pub(crate) frames: u32,
    /// Each console pixel `scale` × `scale` pixels in a picture.
    pub(crate) scale: u32,
    /// Whether it stops when its standard input closes.
    pub(crate) until_stdin_closes: bool,
    /// The game's start options, the same for every session.
    pub(crate) start: Vec<GameArg>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            bind: IpAddr::V4(Ipv4Addr::LOCALHOST),
            port: 8_000,
            limits: Limits::default(),
            frames: 300,
            scale: 1,
            until_stdin_closes: false,
            start: Vec::new(),
        }
    }
}

/// The flags that take a whole number, each with its least and most.
const NUMBERS: [(&str, u64, u64); 14] = [
    ("--port", 0, 65_535),
    ("--sessions", 1, 1_000),
    ("--per-address", 1, 1_000),
    ("--starts", 1, 10_000),
    ("--actions", 1, 1_000_000),
    ("--rate", 1, 1_000),
    ("--frames", 1, MAX_FRAMES as u64),
    ("--scale", 1, MAX_SCALE as u64),
    ("--idle", 1, 86_400),
    ("--lifetime", 1, 86_400),
    ("--deadline", 1, 600),
    ("--cpu", 1, 86_400),
    ("--memory", 16, 1 << 20),
    ("--egress", 1, 1 << 20),
];

impl Options {
    /// Sets the numeric flag `flag` to `n`, in range.
    fn set(&mut self, flag: &str, n: u64) {
        let count = |n: u64| usize::try_from(n).unwrap_or(usize::MAX);
        let small = |n: u64| u32::try_from(n).unwrap_or(u32::MAX);
        let limits = &mut self.limits;
        match flag {
            "--port" => self.port = u16::try_from(n).unwrap_or_default(),
            "--sessions" => limits.sessions = count(n),
            "--per-address" => limits.per_address = count(n),
            "--starts" => limits.starts = small(n),
            "--actions" => limits.actions = small(n),
            "--rate" => limits.rate = small(n),
            "--frames" => self.frames = small(n),
            "--scale" => self.scale = small(n),
            "--idle" => limits.idle = Duration::from_secs(n),
            "--lifetime" => limits.lifetime = Duration::from_secs(n),
            "--deadline" => limits.deadline = Duration::from_secs(n),
            "--cpu" => limits.cpu = Duration::from_secs(n),
            "--memory" => limits.memory = n << 20,
            _ => limits.egress = n << 20,
        }
    }

    /// The arguments that start a worker of these options.
    pub(crate) fn worker_args(&self) -> Vec<String> {
        let mut args = vec![
            "preview-worker".to_owned(),
            "--frames".to_owned(),
            self.frames.to_string(),
            "--scale".to_owned(),
            self.scale.to_string(),
        ];
        for arg in &self.start {
            args.push(arg.flag.clone());
            args.extend(arg.value.clone());
        }
        args
    }
}

/// Reads `preview`'s (and `preview-worker`'s) arguments, the game's start
/// `options` among them.
pub(crate) fn parse_args(args: &[String], options: &[GameOption]) -> Result<Options, String> {
    let mut parsed = Options::default();
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if let Some(&(flag, least, most)) = NUMBERS.iter().find(|(flag, ..)| flag == arg) {
            let text = iter.next().ok_or_else(|| format!("{flag} needs a value"))?;
            let n = text
                .parse::<u64>()
                .ok()
                .filter(|n| (least..=most).contains(n))
                .ok_or_else(|| format!("{flag}: a whole number {least} to {most}, not {text}"))?;
            parsed.set(flag, n);
            continue;
        }
        match arg.as_str() {
            "--until-stdin-closes" => parsed.until_stdin_closes = true,
            "--bind" => {
                let text = iter.next().ok_or("--bind needs a value")?;
                parsed.bind = text
                    .parse()
                    .map_err(|_| format!("--bind: not an IP address: {text}"))?;
            }
            _ => {
                let value = || iter.next().cloned().ok_or(format!("{arg} needs a value"));
                parsed.start.push(game_arg(arg, options, value)?);
            }
        }
    }
    Ok(parsed)
}

/// The usage of `preview`, with `command` as the tool's name.
pub(crate) fn usage(command: &str) -> String {
    format!(
        "\
usage: {command} preview [--port N] [--bind ADDRESS] [limits] [the game's options]

Serves the game to visitors as pictures from this machine: a visitor presses
the game's actions and gets each frame back as a PNG with its text, never the
program, its files, its scene or its state. What a visitor sees they can still
record. Each session's game runs in a worker of its own, this program again.

  --port N              its port, 8000 by default; 0 for any free one
  --bind ADDRESS        the address it listens on: 127.0.0.1, this machine, by default
  --sessions N          the most sessions at once (8)
  --per-address N       the most sessions of one address at once (2)
  --starts N            the most sessions one address may start a minute (10)
  --actions N           the most actions a session may press (1000)
  --rate N              the most actions a session may press each second (8)
  --frames N            the most frames an action runs before its picture (300)
  --scale N             each console pixel N x N pixels in a picture (1)
  --idle SECONDS        a session idle that long ends (120)
  --lifetime SECONDS    a session ends that long after it started (1800)
  --deadline SECONDS    a start or an action that takes longer ends its session (5)
  --cpu SECONDS         the game's time a session may use in all (120)
  --memory MB           a worker that holds more ends its session (512; Linux only)
  --egress MB           the most a session's answers may send (64)
  --until-stdin-closes  stop when standard input closes, as a script or a test does

The game's own options (frame-png --help lists them) start every session alike:
its seed and start are the server's, never a visitor's. It answers POST
/session, POST /session/<id> with {{\"action\": \"<name>\"}} and DELETE
/session/<id>; every other request is refused. Ctrl+C stops it."
    )
}

/// `preview` on `project`, with the game's tools: serves until stopped.
/// The exit code: 2 for wrong arguments, 1 for a game that offers no
/// preview or a server that couldn't start.
pub(crate) fn main(project: &Project, args: &[String], game: &dyn GameTools) -> u8 {
    let command = project.config.command();
    let fail = |code: Code, error: &str, help: Option<&str>| {
        eprint!("{}", codes::said("preview", code.0, error, help));
        codes::exit_code(Some(code.0))
    };
    if args.iter().any(|arg| arg == "--help") {
        println!("{}", usage(command));
        return 0;
    }
    let options = match parse_args(args, game.frame_options()) {
        Ok(options) => options,
        Err(e) => return fail(codes::WRONG_ARGUMENTS, &e, Some(&usage(command))),
    };
    // A game whose tools offer no preview, or don't start, says so now,
    // not to its first visitor.
    if let Err(e) = worker::Visit::start(game, &options.start, options.frames, options.scale) {
        return fail(codes::NOT_OFFERED, &e, None);
    }
    let program = match std::env::current_exe() {
        Ok(program) => program,
        Err(e) => {
            let error = format!("the game's tool program can't find itself: {e}");
            return fail(codes::GAME_PROGRAM, &error, None);
        }
    };
    let listener = match TcpListener::bind((options.bind, options.port)) {
        Ok(listener) => listener,
        Err(e) => {
            let error = format!("cannot listen on {}:{}: {e}", options.bind, options.port);
            let help = "pick another with --port, or --port 0 for any free one";
            return fail(codes::WRONG_ARGUMENTS, &error, Some(help));
        }
    };
    let (args, root) = (options.worker_args(), project.root.clone());
    let launch = move || Worker::process(&program, &args, &root);
    let server = Arc::new(Server::new(options.limits, Box::new(launch)));
    match listener.local_addr() {
        Ok(address) => println!("preview: listening on http://{address}/"),
        Err(e) => return fail(codes::WRONG_ARGUMENTS, &e.to_string(), None),
    }
    if server::resident(std::process::id()).is_none() {
        eprintln!("preview: this system doesn't say a worker's memory: --memory isn't kept here");
    }
    if options.until_stdin_closes {
        std::thread::spawn(|| {
            let _ = std::io::copy(&mut std::io::stdin().lock(), &mut std::io::sink());
            std::process::exit(0);
        });
    }
    server::serve(&listener, &server);
    0
}
