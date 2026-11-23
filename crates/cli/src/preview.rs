//! The hosted preview: the game played by visitors as pictures from this
//! machine, never as a program they download (ADR-0141). A visitor presses
//! the game's actions and gets the frame after each one back, as a PNG with
//! the text it shows; the program, its files, its scene and its state stay
//! here. What a visitor sees they can still record and copy.
//!
//! `preview` serves it over HTTP ([`server`]), on this machine's own
//! address by default, and answers three requests, every other path with
//! `404`:
//!
//! - `POST /session`: a new session, its worker started, and its first
//!   frame;
//! - `POST /session/<id>` with `{"action": "<name>"}`: the action pressed,
//!   and the frame after it;
//! - `DELETE /session/<id>`: the session ended.
//!
//! A frame is JSON: the session's id, `session`, and what the worker shows
//! ([`worker::Shown`]). A refusal is `{"error": "…"}` with its status. The
//! server keeps [`Limits`]: the sessions at once, and how long a start or
//! an action may take before its worker is killed.
//!
//! Each visitor's game runs in a **worker** of its own ([`worker`]): the
//! game's tool program run as `preview-worker`, one process a session. It
//! starts the game's session as its arguments say, the same for every
//! visitor, takes the actions a player can rebind with the developer's
//! debug tools off
//! ([`Session::for_visitor`](crate::contract::session::Session::for_visitor)),
//! and answers each action, a JSON line, with the frame after it, and
//! nothing else: no script, no frame count, no scene, snapshot or log.

mod http;
mod server;
pub(crate) mod worker;

#[cfg(test)]
mod tests;

use std::net::{IpAddr, Ipv4Addr, TcpListener};
use std::sync::Arc;
use std::time::Duration;

use shuyi_platform::harness::MAX_FRAMES;

use crate::contract::codes::{self, Code};
use crate::frame_png::MAX_SCALE;
use crate::game::{GameArg, GameOption, GameTools, code_of, game_arg};
use crate::project::Project;
use server::{Server, Worker};

/// The limits a server keeps, each a flag of `preview` ([`usage`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Limits {
    /// The most sessions at once, those starting among them.
    pub(crate) sessions: usize,
    /// How long a start or an action may take.
    pub(crate) deadline: Duration,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            sessions: 8,
            deadline: Duration::from_secs(5),
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
            frames: worker::FRAMES,
            scale: 1,
            until_stdin_closes: false,
            start: Vec::new(),
        }
    }
}

/// The flags that take a whole number, each with its least and most.
const NUMBERS: [(&str, u64, u64); 5] = [
    ("--port", 0, 65_535),
    ("--sessions", 1, 1_000),
    ("--frames", 1, MAX_FRAMES as u64),
    ("--scale", 1, MAX_SCALE as u64),
    ("--deadline", 1, 600),
];

impl Options {
    /// Sets the numeric flag `flag`, one of [`NUMBERS`], to `n`, in its
    /// range.
    fn set(&mut self, flag: &str, n: u64) {
        let small = |n: u64| u32::try_from(n).unwrap_or(u32::MAX);
        match flag {
            "--port" => self.port = u16::try_from(n).unwrap_or_default(),
            "--sessions" => self.limits.sessions = usize::try_from(n).unwrap_or(usize::MAX),
            "--frames" => self.frames = small(n),
            "--scale" => self.scale = small(n),
            _ => self.limits.deadline = Duration::from_secs(n),
        }
    }

    /// The arguments that start a worker of these options in the game's
    /// tool program: `before` (its project's, as `shuyi` runs it), then
    /// `preview-worker` and what [`worker::parse_args`] reads.
    pub(crate) fn worker_args(&self, before: &[String]) -> Vec<String> {
        let mut args = before.to_vec();
        args.extend([
            "preview-worker".to_owned(),
            "--frames".to_owned(),
            self.frames.to_string(),
            "--scale".to_owned(),
            self.scale.to_string(),
        ]);
        for arg in &self.start {
            args.push(arg.flag.clone());
            args.extend(arg.value.clone());
        }
        args
    }
}

/// Reads `preview`'s arguments, the game's start `options` among them.
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
    let Options {
        port,
        limits,
        frames,
        scale,
        ..
    } = Options::default();
    format!(
        "\
usage: {command} preview [--port N] [--bind ADDRESS] [limits] [the game's options]

Serves the game to visitors as pictures from this machine: a visitor presses
the game's actions and gets each frame back as a PNG with its text, never the
program, its files, its scene or its state. What a visitor sees they can still
record. Each session's game runs in a worker of its own, this program again.

  --port N              its port ({port}); 0 for any free one
  --bind ADDRESS        the address it listens on: 127.0.0.1, this machine alone, by default
  --sessions N          the most sessions at once ({sessions})
  --frames N            the most frames an action runs before its picture ({frames})
  --scale N             each console pixel N by N pixels in a picture ({scale})
  --deadline SECONDS    a start or an action that takes longer ends its session ({deadline})
  --until-stdin-closes  stop when standard input closes, as a script or a test does

The game's own options (frame-png --help lists them) start every session alike:
its seed and start are the server's, never a visitor's. It answers POST
/session, POST /session/<id> with {{\"action\": \"<name>\"}} and DELETE
/session/<id>, and every other path with 404. Ctrl+C stops it.",
        sessions = limits.sessions,
        deadline = limits.deadline.as_secs(),
    )
}

/// `preview` on `project`, with the game's tools: serves until stopped.
/// The exit code: 0 for `--help`, 2 for wrong arguments or an address it
/// can't listen on, and the code of a game whose tools offer no preview or
/// don't start ([`code_of`]).
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
        return fail(code_of(&e), &e, None);
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
    // A worker is this program again, started as `shuyi` starts it.
    let before = project.config.game.as_ref().map_or(&[][..], |game| &game.args);
    let (args, root) = (options.worker_args(before), project.root.clone());
    let launch = move || Worker::process(&program, &args, &root);
    let server = Arc::new(Server::new(options.limits, Box::new(launch)));
    match listener.local_addr() {
        Ok(address) => println!("preview: listening on http://{address}/"),
        Err(e) => return fail(codes::WRONG_ARGUMENTS, &e.to_string(), None),
    }
    if options.until_stdin_closes {
        let server = Arc::clone(&server);
        std::thread::spawn(move || {
            let _ = std::io::copy(&mut std::io::stdin().lock(), &mut std::io::sink());
            server.close();
            std::process::exit(0);
        });
    }
    server::serve(&listener, &server);
    0
}
