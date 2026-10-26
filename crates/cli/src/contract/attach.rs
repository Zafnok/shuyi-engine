//! `attach`: a build that is already running, reached over the live link
//! (ADR-0047) instead of a game started headless. It needs no project:
//! the build is a debug build of a game with its backend's `link` feature,
//! started with `SHUYI_LINK=<port>`, and its log's line `shuyi link:
//! listening on <address>, token <token>` gives the address and the token.
//!
//! It connects, says hello, plays the steps as a player's input (keys,
//! controller buttons, an action by name, waits counted in the build's
//! own frames), lets `frames` more frames pass, then reads the build's
//! status, its frame as text, its log from `log_since`, and, if asked,
//! the window's picture, written to a file. It changes nothing else in
//! the build. The wire's types are `shuyi_platform::link`'s, the build's
//! own.
//!
//! With `perf_since` it reads the frames' records too, summed up as `run
//! --perf` sums them; with `perf_follow` it follows them instead, every
//! 100 ms for `follow_for` seconds, writing each new record to a file as
//! a perf trace's line as the build runs, so an agent reads the file as
//! it grows.

use std::fmt::Write as _;
use std::fs;
use std::io::{BufRead, BufReader, ErrorKind, Write};
use std::net::{Shutdown, SocketAddr, TcpStream, ToSocketAddrs};
use std::path::Path;
use std::time::Duration;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use shuyi_platform::link::{
    self, Answer as Line, LogAnswer, PerfAnswer, Phase, Played, Request, Status,
};
use shuyi_ui::log::LOG_LINES;
use shuyi_ui::perf::{FrameRecord, HeapSnapshot};

use super::codes::{self, Code};
use super::perf::{self, PerfSummary};
use super::run::MAX_IDLE_FRAMES;
use super::{Answer, Op};
use crate::frame_png::{Script, Step};

/// How many frames a build runs in a second, to turn `--wait`'s seconds
/// into its frames.
pub const FRAMES_PER_SECOND: f32 = 60.0;

/// How long a connection may take.
const CONNECT: Duration = Duration::from_secs(3);

/// How long an answer may take, a play's waits aside.
const ANSWER: Duration = Duration::from_secs(10);

/// One step played over the link.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum AttachStep {
    /// Each key chord pressed and let go in turn (`"Enter Up"`).
    Keys(String),
    /// Each controller button pressed and let go in turn (`"South"`,
    /// `"East+DpadDown"`: those together).
    Pad(String),
    /// That many seconds, played as the build's frames at 60 a second.
    Wait(f32),
    /// The game's action of that name (`Confirm`), pressed by the
    /// player's primary key for it, or its primary button if it has no
    /// key.
    Act(String),
}

/// What `attach` reaches and plays.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, default)]
// An input: built with `default()` and its fields assigned, it may grow
// (ADR-0039).
#[non_exhaustive]
pub struct AttachInput {
    /// The build's address, `host:port` (`127.0.0.1:7357`).
    pub address: String,
    /// The token the build logged when it started listening.
    pub token: String,
    /// The steps to play, in order.
    pub steps: Vec<AttachStep>,
    /// Frames to let pass after the steps, with no input.
    pub frames: u32,
    /// Where to write the window's picture, a PNG file (from the working
    /// folder), if wanted.
    pub screenshot: Option<String>,
    /// The first log line to answer, by its number: 0 for every line kept,
    /// the last answer's `log_next` for the lines since.
    pub log_since: u64,
    /// The first frame's record to answer, by its number, if wanted: 0
    /// for every record kept, the last answer's `perf_next` for the
    /// records since. With `perf_follow`, where the following starts.
    pub perf_since: Option<u64>,
    /// A file to follow the build's frames into (from the working folder):
    /// each new frame's record written as a line as the build runs, for
    /// `follow_for` seconds or until the build quits.
    pub perf_follow: Option<String>,
    /// How many seconds to follow, 1 to 3,600: 10 if not said.
    #[schemars(range(min = 1, max = 3600))]
    pub follow_for: Option<u32>,
    /// Take a heap snapshot with the records: with `perf_since`, one now;
    /// with `perf_follow`, one as the following starts and one as it ends,
    /// and the diff between them.
    pub perf_snapshot: bool,
}

/// How many seconds `perf_follow` follows when not said.
pub const FOLLOW_FOR: u32 = 10;

/// The most seconds `perf_follow` follows.
pub const MAX_FOLLOW_FOR: u32 = 3_600;

/// How often `perf_follow` reads the build's records.
pub const FOLLOW_EVERY: Duration = Duration::from_millis(100);

/// What `perf_follow` wrote.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
// An answer's, read field by field: it may grow (ADR-0039).
#[non_exhaustive]
pub struct AttachFollow {
    /// The file, its absolute path.
    pub file: String,
    /// The lines written: a frame's record each, and with
    /// `perf_snapshot` the two snapshots and their diff.
    pub lines: u32,
    /// The frames the build ran that no read caught: it ran more than
    /// 600 between two reads.
    pub dropped: u64,
    /// Whether the build closed the link before the time was up: it quit.
    pub closed: bool,
}

/// The window's picture, as written.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
// An answer's, read field by field: it may grow (ADR-0039).
#[non_exhaustive]
pub struct AttachScreenshot {
    /// The file, as given.
    pub path: String,
    /// Its width in pixels.
    pub width: u32,
    /// Its height in pixels.
    pub height: u32,
}

/// One line of the build's log, with its number.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
// An answer's, read field by field: it may grow (ADR-0039).
#[non_exhaustive]
pub struct AttachLogEntry {
    /// Its number in the build's log: lines are numbered from 0 as they
    /// are logged.
    pub seq: u64,
    /// The frame it was given in (frames count from 1).
    pub frame: u32,
    /// How serious it is: `error`, `warn`, `info` or `debug`.
    pub level: String,
    /// What it is about: `assets`, `audio`, `input`, `screens`, `storage`,
    /// `platform` or `game`.
    pub category: String,
    /// What it says, in English.
    pub text: String,
}

/// What `attach` saw.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
// An answer, read field by field: it may grow (ADR-0039).
#[non_exhaustive]
pub struct AttachOutput {
    /// The link's version both ends speak.
    pub link: u32,
    /// The game: its window's title.
    pub game: String,
    /// The frames the build has run since it started.
    pub frames: u32,
    /// Where the game is: `loading` (its card shows), `running` or
    /// `failed` (its load failed; the errors show).
    pub phase: String,
    /// The top screen's name; empty with none open.
    pub top_screen: String,
    /// The open screens, bottom first.
    pub screens: Vec<String>,
    /// Whether the system has suspended the build.
    pub suspended: bool,
    /// Whether its window has the player's attention.
    pub focused: bool,
    /// Whether the game has quit.
    pub quit: bool,
    /// The frame, in the snapshot text format; empty while the game
    /// isn't running.
    pub frame: String,
    /// The window's picture, if asked for.
    pub screenshot: Option<AttachScreenshot>,
    /// The build's log lines numbered `log_since` or more, oldest first.
    pub log: Vec<AttachLogEntry>,
    /// The number the build's next log line gets: the next `log_since`.
    pub log_next: u64,
    /// The frames' records numbered `perf_since` or more, oldest first, at
    /// most 600; empty without `perf_since`, and with `perf_follow` (they
    /// are in its file).
    pub perf: Vec<FrameRecord>,
    /// The number to read the records from next time: the next
    /// `perf_since`. Null without `perf_since` or `perf_follow`.
    pub perf_next: Option<u64>,
    /// What the records add up to: those answered, or with `perf_follow`
    /// every one written. Null with none.
    pub perf_summary: Option<PerfSummary>,
    /// The heap snapshot taken with the records, with `perf_snapshot`
    /// (with `perf_follow`, the one as it ended): null where the build
    /// doesn't count the heap by tag.
    pub perf_snapshot: Option<HeapSnapshot>,
    /// What `perf_follow` wrote.
    pub follow: Option<AttachFollow>,
}

impl AttachInput {
    /// Whether it can be played: an address and a token, steps the link
    /// can play, at most [`MAX_IDLE_FRAMES`] frames after.
    pub fn check(&self) -> Result<(), String> {
        if self.address.is_empty() {
            return Err("the build's address is missing: host:port".to_owned());
        }
        if self.token.is_empty() {
            return Err("--token is missing: the build's log line gives it".to_owned());
        }
        if self.frames > MAX_IDLE_FRAMES {
            return Err(format!(
                "frames: at most {MAX_IDLE_FRAMES}, not {}",
                self.frames
            ));
        }
        for step in self.link_steps() {
            step.check()?;
        }
        if let Some(seconds) = self.follow_for {
            if self.perf_follow.is_none() {
                return Err("follow_for: only with perf_follow".to_owned());
            }
            if !(1..=MAX_FOLLOW_FOR).contains(&seconds) {
                return Err(format!(
                    "follow_for: 1 to {MAX_FOLLOW_FOR} seconds, not {seconds}"
                ));
            }
        }
        if self.perf_follow.as_deref().is_some_and(str::is_empty) {
            return Err("perf_follow: the file is missing".to_owned());
        }
        if self.perf_snapshot && self.perf_since.is_none() && self.perf_follow.is_none() {
            return Err("perf_snapshot: only with perf_since or perf_follow".to_owned());
        }
        Ok(())
    }

    /// The steps as the link plays them, the frames after as a wait.
    pub fn link_steps(&self) -> Vec<link::Step> {
        let mut steps: Vec<link::Step> = self.steps.iter().map(AttachStep::to_link).collect();
        if self.frames > 0 {
            steps.push(link::Step::Wait(self.frames));
        }
        steps
    }

    /// Whether the steps or the frames after wait.
    fn waits(&self) -> bool {
        self.frames > 0 || self.steps.iter().any(|s| matches!(s, AttachStep::Wait(_)))
    }
}

impl AttachStep {
    /// `run`'s script step `step`, played over the link.
    pub(crate) fn of(step: &Step) -> Self {
        match step {
            Step::Keys(script) => AttachStep::Keys(script.clone()),
            Step::Pad(script) => AttachStep::Pad(script.clone()),
            Step::Wait(seconds) => AttachStep::Wait(*seconds),
        }
    }

    /// The step as the link plays it: a wait in the build's frames.
    fn to_link(&self) -> link::Step {
        match self {
            AttachStep::Keys(script) => link::Step::Keys(script.clone()),
            AttachStep::Pad(script) => link::Step::Pad(script.clone()),
            AttachStep::Act(name) => link::Step::Act(name.clone()),
            AttachStep::Wait(seconds) => link::Step::Wait(frames_of(*seconds)),
        }
    }
}

/// `seconds` in frames at [`FRAMES_PER_SECOND`], to the nearest.
pub fn frames_of(seconds: f32) -> u32 {
    let frames = (seconds.max(0.0) * FRAMES_PER_SECOND).round();
    // Not negative, and `as` saturates past `u32::MAX`; `Script::parse_step`
    // checks a wait is at most `MAX_WAIT` seconds anyway.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let frames = frames as u32;
    frames
}

/// The usage of `attach`.
pub fn usage(command: &str) -> String {
    format!(
        "usage: {command} attach <host:port> --token <token> [steps] [--frames N] \
         [--screenshot <file.png>] [--log-since N] [--perf-since N] \
         [--perf-follow <file.jsonl> [--for S]] [--perf-snapshot] [--json]\n\n\
         a running debug build of a game with its `link` feature, started with SHUYI_LINK=<port>\n\
         (0 takes a free one); its log line `shuyi link: listening on <address>, token <token>`\n\
         gives both.\n\n\
         steps (each may repeat, played in order, as a player's input):\n  \
         --keys \"Enter Up\"    press and release each key chord in turn (or --script)\n  \
         --pad \"South\"        press and release each controller button in turn\n  \
         --act Confirm        press the game's action by its key, or else its button\n  \
         --wait 0.5           let that many seconds pass, in the build's frames (60 a second)\n\n\
         --frames N           then let N frames pass\n  \
         --screenshot <file>  write the window's picture there, as a PNG\n  \
         --log-since N        the log's lines from number N (the last answer's log_next)\n  \
         --perf-since N       the frames' records from number N (the last answer's perf_next)\n  \
         --perf-follow <file> then write each new frame's record to the file as a JSON line\n                       \
         as the build runs, printing nothing until it ends\n  \
         --for S              follow for S seconds, 1 to 3600 (10), or until the build quits\n  \
         --perf-snapshot      a heap snapshot with the records (following: at its start and end)\n"
    )
}

/// Reads `attach`'s arguments (not `--json`).
pub fn parse_args(args: &[String]) -> Result<AttachInput, String> {
    let mut input = AttachInput::default();
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        let mut value = || {
            iter.next()
                .ok_or_else(|| format!("{arg} needs a value"))
                .cloned()
        };
        if let Some(step) = parse_step(arg, &mut value) {
            input.steps.push(step?);
            continue;
        }
        match arg.as_str() {
            "--token" => input.token = value()?,
            "--screenshot" => input.screenshot = Some(value()?),
            "--frames" => {
                let text = value()?;
                input.frames = text
                    .parse()
                    .ok()
                    .filter(|n| *n <= MAX_IDLE_FRAMES)
                    .ok_or_else(|| {
                        format!("--frames: a whole number 0 to {MAX_IDLE_FRAMES}, not {text}")
                    })?;
            }
            "--log-since" => {
                let text = value()?;
                input.log_since = text
                    .parse()
                    .map_err(|_| format!("--log-since: a whole number, not {text}"))?;
            }
            "--perf-since" => {
                let text = value()?;
                input.perf_since = Some(
                    text.parse()
                        .map_err(|_| format!("--perf-since: a whole number, not {text}"))?,
                );
            }
            "--perf-follow" => input.perf_follow = Some(value()?),
            "--for" => {
                let text = value()?;
                input.follow_for = Some(
                    text.parse()
                        .ok()
                        .filter(|n| (1..=MAX_FOLLOW_FOR).contains(n))
                        .ok_or_else(|| {
                            format!("--for: whole seconds, 1 to {MAX_FOLLOW_FOR}, not {text}")
                        })?,
                );
            }
            "--perf-snapshot" => input.perf_snapshot = true,
            flag if flag.starts_with("--") => return Err(format!("unknown option: {flag}")),
            address if input.address.is_empty() => address.clone_into(&mut input.address),
            other => return Err(format!("one address, not also {other}")),
        }
    }
    if input.address.is_empty() {
        return Err("the build's address is missing: host:port".to_owned());
    }
    if input.token.is_empty() {
        return Err("--token is missing: the build's log line gives it".to_owned());
    }
    if input.follow_for.is_some() && input.perf_follow.is_none() {
        return Err("--for: only with --perf-follow".to_owned());
    }
    if input.perf_snapshot && input.perf_since.is_none() && input.perf_follow.is_none() {
        return Err("--perf-snapshot: only with --perf-since or --perf-follow".to_owned());
    }
    Ok(input)
}

/// The step flag `flag`, if it is one, read with the value `value` gives:
/// `run`'s (`Script::parse_step`) and `--act`.
fn parse_step(
    flag: &str,
    value: &mut impl FnMut() -> Result<String, String>,
) -> Option<Result<AttachStep, String>> {
    if flag == "--act" {
        return Some(value().and_then(|name| {
            let step = AttachStep::Act(name);
            step.to_link()
                .check()
                .map(|()| step)
                .map_err(|e| format!("--{e}"))
        }));
    }
    if !matches!(flag, "--keys" | "--script" | "--pad" | "--wait") {
        return None;
    }
    let text = match value() {
        Ok(text) => text,
        Err(e) => return Some(Err(e)),
    };
    let step = Script::parse_step(flag, &text)?;
    Some(step.map(|step| AttachStep::of(&step)))
}

/// Why `attach` stopped: its code and what to say.
pub(crate) type Failed = (Code, String);

/// The link to a build: its lines both ways. `attach` and MCP's session
/// (`crate::mcp::attached`) speak the link through it.
pub(crate) struct Connection {
    reader: BufReader<TcpStream>,
    writer: TcpStream,
    address: String,
}

impl Connection {
    /// Connects to the build at `address`.
    pub(crate) fn open(address: &str) -> Result<Self, Failed> {
        let no_build = |why: String| {
            let error = format!("no build is listening at {address}: {why}");
            (codes::NO_BUILD, error)
        };
        let addresses: Vec<SocketAddr> = address
            .to_socket_addrs()
            .map_err(|e| (codes::WRONG_ARGUMENTS, format!("{address}: {e}")))?
            .collect();
        let mut last = format!("{address} names no address");
        for socket in addresses {
            match TcpStream::connect_timeout(&socket, CONNECT) {
                Ok(stream) => {
                    let _ = stream.set_nodelay(true);
                    let writer = stream.try_clone().map_err(|e| no_build(e.to_string()))?;
                    return Ok(Self {
                        reader: BufReader::new(stream),
                        writer,
                        address: address.to_owned(),
                    });
                }
                Err(e) => last = e.to_string(),
            }
        }
        Err(no_build(last))
    }

    /// Sends `request` and reads its answer, waiting at most `wait`; `None`
    /// if the build closed the link.
    fn ask(&mut self, request: &Request, wait: Duration) -> Result<Option<Line>, Failed> {
        let lost = |why: String| (codes::LINK_LOST, why);
        let line = format!("{}\n", request.to_line());
        if let Err(e) = self.writer.write_all(line.as_bytes()) {
            return if closed(e.kind()) {
                Ok(None)
            } else {
                Err(lost(format!("the link to {} broke: {e}", self.address)))
            };
        }
        let _ = self.reader.get_ref().set_read_timeout(Some(wait));
        let mut answer = String::new();
        match self.reader.read_line(&mut answer) {
            Ok(0) => Ok(None),
            Ok(_) => Line::from_line(&answer).map(Some).map_err(|e| {
                lost(format!(
                    "the build at {} answered what this shuyi can't read: {e}",
                    self.address
                ))
            }),
            Err(e) if closed(e.kind()) => Ok(None),
            Err(e) if matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {
                Err(lost(format!(
                    "the build at {} didn't answer in {} s",
                    self.address,
                    wait.as_secs()
                )))
            }
            Err(e) => Err(lost(format!("the link to {} broke: {e}", self.address))),
        }
    }

    /// [`ask`](Self::ask), the build closing the link being an error.
    pub(crate) fn answer(&mut self, request: &Request, wait: Duration) -> Result<Line, Failed> {
        self.ask(request, wait)?.ok_or_else(|| {
            let error = format!("the build at {} closed the link", self.address);
            (codes::LINK_LOST, error)
        })
    }
}

impl Drop for Connection {
    /// Closes the link at once: a copy of the socket's handle that a
    /// child process started meanwhile inherited (on Windows) would keep
    /// it open, and the build would think the client still attached.
    fn drop(&mut self) {
        let _ = self.writer.shutdown(Shutdown::Both);
    }
}

/// Whether an error of `kind` means the other end closed the connection.
fn closed(kind: ErrorKind) -> bool {
    matches!(
        kind,
        ErrorKind::ConnectionReset | ErrorKind::ConnectionAborted | ErrorKind::BrokenPipe
    )
}

/// An answer of the wrong kind to `asked`.
fn unexpected(asked: &str, line: &Line) -> Failed {
    let error = match line {
        Line::Error { error } => format!("the build couldn't answer {asked}: {error}"),
        other => format!("the build answered {asked} with {}", other.to_line()),
    };
    (codes::LINK_LOST, error)
}

/// What `attach` read, before it is an output.
pub(crate) struct Seen {
    /// The play's answer, if steps were played.
    pub(crate) played: Option<Played>,
    /// Where the build is, after them.
    pub(crate) status: Status,
    /// Its frame as text.
    pub(crate) frame: String,
    /// Its log from the input's `log_since`.
    pub(crate) log: LogAnswer,
    /// The picture written, and what it doesn't show.
    pub(crate) screenshot: Option<(AttachScreenshot, Option<String>)>,
    /// The frames' records read, or followed into a file.
    pub(crate) perf: Option<PerfSeen>,
}

/// The frames' records `attach` read, or followed into a file.
pub(crate) struct PerfSeen {
    /// The records answered; none when followed.
    records: Vec<FrameRecord>,
    /// The number to read from next time.
    next: u64,
    /// What the records read or written add up to.
    summary: Option<PerfSummary>,
    /// The heap snapshot taken last, if one was.
    snapshot: Option<HeapSnapshot>,
    /// What the following wrote.
    follow: Option<AttachFollow>,
    /// What the records don't show.
    limits: Vec<String>,
}

/// The most records one read over the link answers.
const READ_MAX: u32 = 600;

/// The build's frames' records numbered `since` or more over `link`, the
/// first `max`, with a heap snapshot after this frame if `snapshot`;
/// `None` if the build closed the link.
pub(crate) fn perf_read(
    link: &mut Connection,
    since: u64,
    max: u32,
    snapshot: bool,
) -> Result<Option<PerfAnswer>, Failed> {
    let request = Request::Perf {
        since,
        max,
        snapshot,
    };
    match link.ask(&request, ANSWER)? {
        None => Ok(None),
        Some(Line::Perf(answer)) => Ok(Some(answer)),
        Some(other) => Err(unexpected("its frames' records", &other)),
    }
}

/// The limits of every perf answer of the link: what the times are,
/// nothing measured after `frames` frames, and the heap's counts.
pub(crate) fn perf_limits(records: &[FrameRecord], measured: bool, frames: u32) -> Vec<String> {
    let mut limits = vec![perf::TIMES.to_owned()];
    if !measured {
        limits.push(perf::unmeasured_limit(frames));
    }
    // The build counts its own heap, and nothing of this program's.
    limits.extend(perf::heap_limits(records, false, false));
    limits
}

/// The records `input` asks for from `since`, read once over `link`.
fn perf_once(link: &mut Connection, input: &AttachInput, since: u64) -> Result<PerfSeen, Failed> {
    let Some(answer) = perf_read(link, since, READ_MAX, input.perf_snapshot)? else {
        let error = format!("the build at {} closed the link", link.address);
        return Err((codes::LINK_LOST, error));
    };
    let mut limits = perf_limits(&answer.records, answer.measured, answer.frames);
    if since > answer.next {
        limits.push(perf::RESET.to_owned());
    }
    if answer.dropped > 0 {
        limits.push(perf::dropped_limit(answer.dropped));
    }
    if input.perf_snapshot && answer.snapshot.is_none() {
        limits.push(perf::HEAP_UNTAGGED.to_owned());
    }
    let summary = perf::summary(&answer.records).map(|mut summary| {
        summary.dropped = answer.dropped;
        summary
    });
    Ok(PerfSeen {
        records: answer.records,
        next: answer.next,
        summary,
        snapshot: answer.snapshot,
        follow: None,
        limits,
    })
}

/// A clock a follow reads and waits on.
pub(crate) trait Clock {
    /// The time since some moment.
    fn now(&mut self) -> Duration;
    /// Waits `time`.
    fn sleep(&mut self, time: Duration);
}

/// The wall's clock.
struct Wall(std::time::Instant);

impl Clock for Wall {
    fn now(&mut self) -> Duration {
        self.0.elapsed()
    }

    fn sleep(&mut self, time: Duration) {
        std::thread::sleep(time);
    }
}

/// What a follow read and wrote.
#[derive(Debug, Default)]
pub(crate) struct Followed {
    /// Every record written, oldest first.
    pub(crate) records: Vec<FrameRecord>,
    /// The number to read from next.
    pub(crate) next: u64,
    /// The lines written.
    pub(crate) lines: u32,
    /// The records no read caught.
    pub(crate) dropped: u64,
    /// Whether the build closed the link before the time was up.
    pub(crate) closed: bool,
    /// Whether any frame was measured.
    pub(crate) measured: bool,
    /// The frames the build had run at the last read.
    pub(crate) frames: u32,
    /// The heap snapshots as the following started and as it ended.
    pub(crate) snapshots: (Option<HeapSnapshot>, Option<HeapSnapshot>),
}

/// Follows a build's frames from `since` for `seconds` on `clock`: reads
/// them with `read` (the records from a number, with a heap snapshot if
/// asked; `None` once the build closed the link) every
/// [`FOLLOW_EVERY`], at once again after a full read, and once more when
/// the time is up; and writes each record to `out` as a
/// [`PerfLine::Frame`](shuyi_ui::perf::PerfLine) line, flushed after each
/// read. With `snapshot`, the first read and the last take a heap snapshot,
/// and the two and their diff end the lines.
pub(crate) fn follow(
    read: &mut impl FnMut(u64, bool) -> Result<Option<PerfAnswer>, Failed>,
    since: u64,
    seconds: u32,
    snapshot: bool,
    out: &mut impl Write,
    clock: &mut impl Clock,
) -> Result<Followed, Failed> {
    let written = |e: std::io::Error| (codes::FILE, format!("--perf-follow: {e}"));
    let end = clock.now() + Duration::from_secs(u64::from(seconds));
    let mut followed = Followed {
        next: since,
        ..Followed::default()
    };
    let mut first = true;
    loop {
        let at = clock.now();
        let last = at >= end;
        let snap = snapshot && (first || last);
        let Some(answer) = read(followed.next, snap)? else {
            followed.closed = true;
            break;
        };
        if snap && first {
            followed.snapshots.0.clone_from(&answer.snapshot);
        } else if snap {
            followed.snapshots.1.clone_from(&answer.snapshot);
        }
        out.write_all(perf::lines(&answer.records).as_bytes())
            .and_then(|()| out.flush())
            .map_err(written)?;
        let full = answer.records.len() >= usize::try_from(READ_MAX).unwrap_or(usize::MAX);
        followed.lines = followed
            .lines
            .saturating_add(u32::try_from(answer.records.len()).unwrap_or(u32::MAX));
        followed.dropped += answer.dropped;
        followed.measured |= answer.measured;
        followed.frames = answer.frames;
        followed.next = answer.next;
        followed.records.extend(answer.records);
        first = false;
        if last {
            break;
        }
        if !full {
            let now = clock.now();
            let due = FOLLOW_EVERY.saturating_sub(now.saturating_sub(at));
            clock.sleep(due.min(end.saturating_sub(now)));
        }
    }
    if let (Some(first), Some(last)) = &followed.snapshots {
        out.write_all(perf::heap_lines(first, last).as_bytes())
            .and_then(|()| out.flush())
            .map_err(written)?;
        followed.lines = followed.lines.saturating_add(3);
    }
    Ok(followed)
}

/// Follows the build's frames over `link` into `input`'s file (from
/// `cwd`), from `since` or else from the build's next record.
fn perf_follow(
    link: &mut Connection,
    input: &AttachInput,
    path: &str,
    cwd: &Path,
) -> Result<PerfSeen, Failed> {
    let since = match input.perf_since {
        Some(since) => since,
        None => perf_read(link, 0, READ_MAX, false)?.map_or(0, |answer| answer.next),
    };
    let file = cwd.join(path);
    let mut out =
        fs::File::create(&file).map_err(|e| (codes::FILE, format!("--perf-follow {path}: {e}")))?;
    let seconds = input.follow_for.unwrap_or(FOLLOW_FOR);
    let mut read = |since, snapshot| perf_read(link, since, READ_MAX, snapshot);
    let mut clock = Wall(std::time::Instant::now());
    let followed = follow(
        &mut read,
        since,
        seconds,
        input.perf_snapshot,
        &mut out,
        &mut clock,
    )?;
    Ok(followed_seen(followed, input, &file.display().to_string()))
}

/// What `followed` shows, written to `file`, as `attach` answers it.
pub(crate) fn followed_seen(followed: Followed, input: &AttachInput, file: &str) -> PerfSeen {
    let mut limits = perf_limits(&followed.records, followed.measured, followed.frames);
    if followed.dropped > 0 {
        limits.push(format!(
            "{} frame(s) were dropped between reads: the build ran more than {READ_MAX} frames \
             between two",
            followed.dropped
        ));
    }
    if followed.closed {
        limits.push("the build closed the link before the time was up: it quit".to_owned());
    }
    let (first, last) = followed.snapshots;
    if input.perf_snapshot && !followed.closed && (first.is_none() || last.is_none()) {
        limits.push(perf::HEAP_UNTAGGED.to_owned());
    }
    let summary = perf::summary(&followed.records).map(|mut summary| {
        summary.dropped = followed.dropped;
        summary.file = Some(file.to_owned());
        if let (Some(first), Some(last)) = (&first, &last) {
            perf::add_heap(&mut summary, first, last);
        }
        summary
    });
    PerfSeen {
        records: Vec::new(),
        next: followed.next,
        summary,
        snapshot: last,
        follow: Some(AttachFollow {
            file: file.to_owned(),
            lines: followed.lines,
            dropped: followed.dropped,
            closed: followed.closed,
        }),
        limits,
    }
}

/// `attach`: reaches the build `input` names, plays its steps and reads
/// what it shows; the picture is written from `cwd`.
pub fn attach(input: &AttachInput, cwd: &Path) -> Answer<AttachOutput> {
    if let Err(e) = input.check() {
        return Answer::failed(Op::Attach, codes::WRONG_ARGUMENTS, e);
    }
    let seen = |link: &mut Connection| {
        let game = hello(link, &input.token)?;
        let mut seen = played(link, input)?;
        if let Some(path) = &input.screenshot {
            seen.screenshot = Some(screenshot(link, path, cwd)?);
        }
        if let Some(path) = &input.perf_follow {
            seen.perf = Some(perf_follow(link, input, path, cwd)?);
        } else if let Some(since) = input.perf_since {
            seen.perf = Some(perf_once(link, input, since)?);
        }
        Ok((game, seen))
    };
    match Connection::open(&input.address).and_then(|mut link| seen(&mut link)) {
        Ok((game, seen)) => {
            let limits = limits_of(input, &seen);
            Answer::new(Op::Attach, true, output(game, seen), limits)
        }
        Err((code, error)) => Answer::failed(Op::Attach, code, error),
    }
}

/// The build's status over `link`.
pub(crate) fn status(link: &mut Connection) -> Result<Status, Failed> {
    match link.answer(&Request::Status, ANSWER)? {
        Line::Status(status) => Ok(status),
        other => Err(unexpected("its status", &other)),
    }
}

/// Plays `input`'s steps over `link`, the hello said, then reads the
/// build's status, frame and log; no picture.
pub(crate) fn played(link: &mut Connection, input: &AttachInput) -> Result<Seen, Failed> {
    let steps = input.link_steps();
    let mut played = None;
    if !steps.is_empty() && status(link)?.phase != Phase::Failed {
        let frames: u32 = steps
            .iter()
            .map(|s| match s {
                link::Step::Wait(n) => *n,
                _ => 2,
            })
            .sum();
        let wait = ANSWER + Duration::from_secs(u64::from(frames / 30));
        match link.answer(&Request::Play { steps }, wait)? {
            Line::Played(done) => played = Some(done),
            Line::Error { error } => return Err((codes::WRONG_ARGUMENTS, error)),
            other => return Err(unexpected("the steps", &other)),
        }
    }
    let status = status(link)?;
    let frame = match link.answer(&Request::Frame, ANSWER)? {
        Line::Frame { frame, .. } => frame,
        other => return Err(unexpected("its frame", &other)),
    };
    let since = input.log_since;
    let log = match link.answer(&Request::Log { since }, ANSWER)? {
        Line::Log(log) => log,
        other => return Err(unexpected("its log", &other)),
    };
    Ok(Seen {
        played,
        status,
        frame,
        log,
        screenshot: None,
        perf: None,
    })
}

/// Says hello with `token`: the build's game, or why it won't talk.
pub(crate) fn hello(link: &mut Connection, token: &str) -> Result<String, Failed> {
    let hello = Request::Hello {
        token: token.to_owned(),
        link: link::VERSION,
    };
    match link.ask(&hello, ANSWER)? {
        None => Err((
            codes::LINK_TOKEN,
            format!(
                "the build at {} closed the link at its hello: the token is wrong",
                link.address
            ),
        )),
        Some(Line::Hello {
            link: version,
            game,
        }) if version == link::VERSION => Ok(game),
        Some(Line::Hello { link: version, .. }) => Err((
            codes::LINK_VERSION,
            format!(
                "the build at {} speaks link version {version}, and this shuyi {}",
                link.address,
                link::VERSION
            ),
        )),
        Some(Line::Busy) => Err((
            codes::LINK_LOST,
            format!(
                "another client is attached to the build at {}",
                link.address
            ),
        )),
        Some(other) => Err(unexpected("its hello", &other)),
    }
}

/// The window's picture over `link`.
pub(crate) fn picture(link: &mut Connection) -> Result<link::Screenshot, Failed> {
    match link.answer(&Request::Screenshot, ANSWER)? {
        Line::Screenshot(shot) => Ok(shot),
        other => Err(unexpected("its picture", &other)),
    }
}

/// Asks for the window's picture and writes it to `path` (from `cwd`):
/// what was written, and what the picture doesn't show.
fn screenshot(
    link: &mut Connection,
    path: &str,
    cwd: &Path,
) -> Result<(AttachScreenshot, Option<String>), Failed> {
    let shot = picture(link)?;
    let png = link::base64::decode(&shot.png).map_err(|e| (codes::LINK_LOST, e))?;
    fs::write(cwd.join(path), png)
        .map_err(|e| (codes::FILE, format!("--screenshot {path}: {e}")))?;
    let written = AttachScreenshot {
        path: path.to_owned(),
        width: shot.width,
        height: shot.height,
    };
    Ok((written, shot.limit))
}

/// What `seen` couldn't show, `input` having asked for it.
pub(crate) fn limits_of(input: &AttachInput, seen: &Seen) -> Vec<String> {
    let mut limits = Vec::new();
    let mut add = |limit: String| {
        if !limits.contains(&limit) {
            limits.push(limit);
        }
    };
    match (seen.status.phase, &seen.status.errors) {
        (Phase::Loading, _) => add("the build shows its loading card: no frame".to_owned()),
        (Phase::Failed, errors) => add(format!(
            "the build's game didn't load, so the steps weren't played: {}",
            errors.as_deref().unwrap_or_default()
        )),
        _ => {}
    }
    if let Some(played) = &seen.played {
        if played.waited_loading {
            add("the build was loading: the steps waited for its game".to_owned());
        }
        if played.waited_suspended {
            add("the build was suspended: the steps waited".to_owned());
        }
    }
    if input.waits() {
        add("waits are counted in the build's own frames, not in its clock's seconds".to_owned());
    }
    if let Some((_, Some(limit))) = &seen.screenshot {
        add(format!("the picture: {limit}"));
    }
    if seen.log.dropped > 0 {
        add(format!(
            "the log keeps the last {LOG_LINES} lines: {} line(s) numbered {} or more were \
             dropped before this read",
            seen.log.dropped, input.log_since
        ));
    }
    if let Some(perf) = &seen.perf {
        for limit in &perf.limits {
            add(limit.clone());
        }
    }
    limits
}

/// What `seen` shows of the build of `game`, as `attach` answers it.
fn output(game: String, seen: Seen) -> AttachOutput {
    let status = seen.status;
    let phase = match status.phase {
        Phase::Loading => "loading",
        Phase::Failed => "failed",
        _ => "running",
    };
    let mut output = AttachOutput {
        link: link::VERSION,
        game,
        frames: status.frames,
        phase: phase.to_owned(),
        top_screen: status.top_screen,
        screens: status.screens,
        suspended: status.suspended,
        focused: status.focused,
        quit: status.quit,
        frame: seen.frame,
        screenshot: seen.screenshot.map(|(shot, _)| shot),
        log: seen
            .log
            .lines
            .into_iter()
            .map(|line| AttachLogEntry {
                seq: line.seq,
                frame: line.frame,
                level: line.level,
                category: line.category,
                text: line.text,
            })
            .collect(),
        log_next: seen.log.next,
        perf_next: seen.perf.as_ref().map(|perf| perf.next),
        perf: Vec::new(),
        perf_summary: None,
        perf_snapshot: None,
        follow: None,
    };
    if let Some(perf) = seen.perf {
        output.perf = perf.records;
        output.perf_summary = perf.summary;
        output.perf_snapshot = perf.snapshot;
        output.follow = perf.follow;
    }
    output
}

/// What `attach` saw, for people.
pub fn text(output: &AttachOutput) -> String {
    let screens = if output.screens.is_empty() {
        "none".to_owned()
    } else {
        output.screens.join(" > ")
    };
    let mut out = format!(
        "attach: {} on frame {}, {}; top screen: {} ({screens})\n",
        output.game,
        output.frames,
        output.phase,
        if output.top_screen.is_empty() {
            "none"
        } else {
            &output.top_screen
        },
    );
    if output.suspended || !output.focused || output.quit {
        let said = [
            (output.suspended, "suspended"),
            (!output.focused, "not focused"),
            (output.quit, "quit"),
        ];
        let said: Vec<&str> = said.iter().filter(|(on, _)| *on).map(|(_, s)| *s).collect();
        let _ = writeln!(out, "the build is {}", said.join(", "));
    }
    if let Some(shot) = &output.screenshot {
        let _ = writeln!(
            out,
            "picture: {} ({} x {})",
            shot.path, shot.width, shot.height
        );
    }
    let serious = |entry: &&AttachLogEntry| matches!(entry.level.as_str(), "warn" | "error");
    if output.log.iter().any(|entry| serious(&entry)) {
        out.push_str("log (warnings and errors; every line with --json):\n");
        for entry in output.log.iter().filter(serious) {
            let _ = writeln!(
                out,
                "  {} frame {}: {} {}: {}",
                entry.seq, entry.frame, entry.level, entry.category, entry.text
            );
        }
    }
    let _ = writeln!(out, "log: next line {}", output.log_next);
    if let Some(follow) = &output.follow {
        let _ = writeln!(
            out,
            "followed: {} line(s){}",
            follow.lines,
            if follow.closed {
                ", until the build quit"
            } else {
                ""
            }
        );
    }
    if let Some(summary) = &output.perf_summary {
        out.push_str(&perf::text(summary));
    }
    if let Some(next) = output.perf_next {
        let _ = writeln!(out, "perf: next record {next}");
    }
    let glyphs = output
        .frame
        .split_once("--- ")
        .map_or(output.frame.as_str(), |(glyphs, _)| glyphs);
    if !glyphs.is_empty() {
        out.push_str("frame (its colours with --json):\n");
        out.push_str(glyphs);
        if !glyphs.ends_with('\n') {
            out.push('\n');
        }
    }
    out
}

#[cfg(test)]
mod tests;
