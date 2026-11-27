//! `crash`: a crash report read (`shuyi_platform::crash::CrashReport`, the
//! file a game's panic hook writes), in `shuyi` itself, with no game:
//! `replay --crash` replays the record it holds ([`super::replay`]). With
//! `--symbols`, the report's frames are named from the debug file of its
//! build (`symbols`).

use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use shuyi_platform::crash::{CodeFrame, CrashReport};

use super::codes::{self, Code};
use super::{Answer, Op};

mod symbols;

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
    /// The debug file of the report's build (`<package>.debug`, kept by
    /// `shuyi export --release`), from the same folder: its frames are
    /// named from it. Without it they are offsets.
    #[serde(default)]
    pub symbols: Option<String>,
}

impl CrashInput {
    /// The report in the file `report`, its frames as offsets.
    pub fn new(report: impl Into<String>) -> Self {
        Self {
            report: report.into(),
            symbols: None,
        }
    }
}

/// A frame of a crash report's stack, the innermost first: where in the
/// program it was, and, from the debug file, the function, its file and
/// the line. An inlined function is a frame of its own, at the same
/// offset as the function it was inlined into.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
// `#[non_exhaustive]` (ADR-0039, kind 3): an answer's part gains fields.
#[non_exhaustive]
pub struct NamedFrame {
    /// The return address minus the program's load address, as sixteen
    /// hex digits; `None` for a frame in another module, such as a system
    /// library.
    pub offset: Option<String>,
    /// The function, as the debug file names it (its debug information,
    /// or else its symbol table); `None` without `--symbols`, or where it
    /// names none.
    pub function: Option<String>,
    /// The function's source file, by the name the build gave it (a
    /// release's paths remapped: `/cargo/registry/...`, the package's
    /// name for the game's own).
    pub file: Option<String>,
    /// The line in `file`: of the call, for a frame that called another.
    pub line: Option<u32>,
}

impl NamedFrame {
    /// A frame at `offset` into the program (`None`: in another module),
    /// not named yet.
    pub fn new(offset: Option<u64>) -> Self {
        Self {
            offset: offset.map(|offset| format!("{offset:016x}")),
            function: None,
            file: None,
            line: None,
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
    /// The frames of the panicking thread's stack, the innermost first:
    /// named with `--symbols`, offsets without. Empty for a report from
    /// macOS or the web, or one older than its frames.
    pub frames: Vec<NamedFrame>,
}

impl From<CrashReport> for CrashOutput {
    /// The report, its frames as offsets.
    fn from(report: CrashReport) -> Self {
        Self {
            has_record: report.record.is_some(),
            frames: report
                .frames
                .iter()
                .map(|frame| NamedFrame::new(frame.offset))
                .collect(),
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

/// `crash`: the report `input.report` under `root`, read, and its frames
/// named from the debug file `input.symbols` if there is one: a debug file
/// that can't be read fails with [`codes::FILE`], one that isn't a program
/// with [`codes::DEBUG_FILE`].
pub fn crash(input: &CrashInput, root: &Path) -> Answer<CrashOutput> {
    let report = match read_report(root, &input.report) {
        Ok(report) => report,
        Err((code, e)) => return Answer::failed(Op::Crash, code, e),
    };
    let mut limits = Vec::new();
    if report.frames.is_empty() {
        limits.push(
            "no frames: the report is from macOS or the web, which take none, or from an engine \
             older than its frames"
                .to_owned(),
        );
    }
    let named = match &input.symbols {
        None => {
            if !report.frames.is_empty() {
                limits.push(
                    "no --symbols: the frames are offsets into the program; give the debug file \
                     of its build to name them"
                        .to_owned(),
                );
            }
            None
        }
        Some(file) => match open_symbols(root, file) {
            Ok(symbols) => {
                let (frames, unnamed, inside) = name_frames(&symbols, &report.frames);
                if unnamed > 0 {
                    limits.push(format!(
                        "{unnamed} of {inside} frames fall outside every function: the debug \
                         file may be from another build"
                    ));
                }
                Some(frames)
            }
            Err((code, e)) => return Answer::failed(Op::Crash, code, e),
        },
    };
    let mut output = CrashOutput::from(report);
    if let Some(frames) = named {
        output.frames = frames;
    }
    Answer::new(Op::Crash, true, output, limits)
}

/// The debug file `file` under `root`, read.
fn open_symbols(root: &Path, file: &str) -> Result<symbols::Symbols, (Code, String)> {
    let path = root.join(file);
    fs::File::open(&path).map_err(|e| (codes::FILE, format!("{file}: {e}")))?;
    symbols::Symbols::open(&path).map_err(|e| (codes::DEBUG_FILE, format!("{file}: {e}")))
}

/// `frames` named from `symbols`; with how many of those in the program
/// (with an offset) it names no function for, and how many there are.
fn name_frames(
    symbols: &symbols::Symbols,
    frames: &[CodeFrame],
) -> (Vec<NamedFrame>, usize, usize) {
    let (mut named, mut unnamed, mut inside) = (Vec::new(), 0, 0);
    for frame in frames {
        let Some(offset) = frame.offset else {
            named.push(NamedFrame::new(None));
            continue;
        };
        inside += 1;
        let mut these = symbols.frames(offset);
        if !these.iter().any(|one| one.function.is_some()) {
            unnamed += 1;
        }
        if these.is_empty() {
            these.push(NamedFrame::new(Some(offset)));
        }
        named.append(&mut these);
    }
    (named, unnamed, inside)
}

/// `crash`'s input from `args` (without `--json`): one report, and
/// `--symbols <file>` if given.
pub fn parse_args(args: &[String]) -> Result<CrashInput, String> {
    let wrong = || "one crash report, and --symbols with its debug file".to_owned();
    let (mut report, mut symbols) = (None, None);
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        if arg == "--symbols" {
            let file = args.next().filter(|file| !file.starts_with("--"));
            if symbols.is_some() || file.is_none() {
                return Err(wrong());
            }
            symbols = file.cloned();
        } else if arg.starts_with("--") || report.is_some() {
            return Err(wrong());
        } else {
            report = Some(arg.clone());
        }
    }
    let mut input = CrashInput::new(report.ok_or_else(wrong)?);
    input.symbols = symbols;
    Ok(input)
}

/// `crash`'s usage, with `command` as the tool's name.
pub fn usage(command: &str) -> String {
    format!("usage: {command} crash <report> [--symbols <file>] [--json]")
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

/// A frame for people: its offset, then its function and its place,
/// as far as they are known.
fn frame_line(frame: &NamedFrame) -> String {
    let Some(offset) = &frame.offset else {
        return "in another module".to_owned();
    };
    let mut line = offset.clone();
    if let Some(function) = &frame.function {
        let _ = write!(line, " {function}");
    }
    if let Some(file) = &frame.file {
        let _ = write!(line, " at {file}");
        if let Some(number) = frame.line {
            let _ = write!(line, ":{number}");
        }
    }
    line
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
    let frames: Vec<String> = output.frames.iter().map(frame_line).collect();
    section(&mut out, "frames", frames.iter().map(String::as_str));
    section(&mut out, "backtrace", output.backtrace.lines());
    section(&mut out, "log", output.log.iter().map(String::as_str));
    out
}

#[cfg(test)]
mod tests;
