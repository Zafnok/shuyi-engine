//! `crash`: a crash report read (`shuyi_platform::crash::CrashReport`, the
//! file a game's panic hook writes), in `shuyi` itself, with no game:
//! `replay --crash` replays the record it holds ([`super::replay`]).

use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use shuyi_platform::crash::CrashReport;

use super::codes::{self, Code};
use super::{Answer, Op};

/// What `crash` reads.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
// An input: built with `CrashInput::new` and its fields assigned, it may
// grow (ADR-0039).
#[non_exhaustive]
pub struct CrashInput {
    /// The report's file (`crash-<when>.ron`), from the folder `shuyi`
    /// runs in.
    pub report: String,
}

impl CrashInput {
    /// The report in the file `report`.
    pub fn new(report: impl Into<String>) -> Self {
        Self {
            report: report.into(),
        }
    }
}

/// What `crash` read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
// `#[non_exhaustive]` (ADR-0039, kind 3): an answer gains fields.
#[non_exhaustive]
pub struct CrashOutput {
    /// The panic's message.
    pub message: String,
    /// Where it panicked: `file:line:column`.
    pub location: Option<String>,
    /// Whether the panic ended the game.
    pub fatal: bool,
    /// The panicking thread's name, if it has one.
    pub thread: Option<String>,
    /// When it happened: milliseconds since 1970 on the game's clock.
    pub when: u64,
    /// The engine's version.
    pub engine: String,
    /// The game's version, empty if the game gave none.
    pub game: String,
    /// The OS, the processor and the build: `linux x86_64 release`.
    pub target: String,
    /// The frame that was running.
    pub frame: u32,
    /// The screen on top, if the game was running.
    pub top_screen: Option<String>,
    /// The log's last lines, oldest first.
    pub log: Vec<String>,
    /// The backtrace as the standard library wrote it.
    pub backtrace: String,
    /// Whether it holds the game's record of its play, for
    /// `replay --crash`.
    pub has_record: bool,
}

impl From<CrashReport> for CrashOutput {
    fn from(report: CrashReport) -> Self {
        Self {
            has_record: report.record.is_some(),
            message: report.message,
            location: report.location,
            fatal: report.fatal,
            thread: report.thread,
            when: report.when,
            engine: report.engine,
            game: report.game,
            target: report.target,
            frame: report.frame,
            top_screen: report.top_screen,
            log: report.log,
            backtrace: report.backtrace,
        }
    }
}

/// The crash report in the file `path` under `root`, or why it can't be
/// read ([`codes::FILE`]) or isn't one ([`codes::CRASH_REPORT`]).
pub fn read_report(root: &Path, path: &str) -> Result<CrashReport, (Code, String)> {
    let text =
        fs::read_to_string(root.join(path)).map_err(|e| (codes::FILE, format!("{path}: {e}")))?;
    CrashReport::parse(&text).map_err(|e| (codes::CRASH_REPORT, format!("{path}: {e}")))
}

/// `crash`: the report `input.report` under `root`, read.
pub fn crash(input: &CrashInput, root: &Path) -> Answer<CrashOutput> {
    match read_report(root, &input.report) {
        Ok(report) => Answer::new(Op::Crash, true, CrashOutput::from(report), Vec::new()),
        Err((code, e)) => Answer::failed(Op::Crash, code, e),
    }
}

/// `crash`'s input from `args` (without `--json`): one report.
pub fn parse_args(args: &[String]) -> Result<CrashInput, String> {
    match args {
        [report] if !report.starts_with("--") => Ok(CrashInput::new(report.clone())),
        _ => Err("one crash report".to_owned()),
    }
}

/// `crash`'s usage, with `command` as the tool's name.
pub fn usage(command: &str) -> String {
    format!("usage: {command} crash <report> [--json]")
}

/// `lines` under a heading, each indented, if there are any.
fn section<'a>(out: &mut String, heading: &str, lines: impl IntoIterator<Item = &'a str>) {
    let mut lines = lines.into_iter().peekable();
    if lines.peek().is_some() {
        let _ = writeln!(out, "{heading}:");
        for line in lines {
            let _ = writeln!(out, "  {line}");
        }
    }
}

/// What `crash` read, for people.
pub fn text(output: &CrashOutput) -> String {
    let mut out = String::new();
    let place = output.location.as_deref().unwrap_or("an unknown place");
    let _ = writeln!(out, "crash: {} at {place}", output.message);
    let thread = output.thread.as_deref().unwrap_or("unnamed");
    let ended = if output.fatal {
        "ended the game"
    } else {
        "the game ran on"
    };
    let _ = writeln!(out, "  thread {thread}, {ended}, at {} ms", output.when);
    let game = if output.game.is_empty() {
        "(no version)"
    } else {
        &output.game
    };
    let _ = writeln!(
        out,
        "  engine {}, game {game}, {}",
        output.engine, output.target
    );
    let screen = output.top_screen.as_deref().unwrap_or("none");
    let _ = writeln!(out, "  frame {}, top screen {screen}", output.frame);
    out.push_str(if output.has_record {
        "  a record of play: replay --crash replays it\n"
    } else {
        "  no record of play\n"
    });
    section(&mut out, "backtrace", output.backtrace.lines());
    section(&mut out, "log", output.log.iter().map(String::as_str));
    out
}

#[cfg(test)]
mod tests;
