//! [`RunReport`]: what a shipped game says in its console on every run, when
//! it turns the report on ([`Driver::with_report`](crate::Driver::with_report)):
//! how its start went, and then how fast it runs, so a tester's log shows a
//! slow start or a stutter without a bench build.
//!
//! - **The start:** when the loading card and the game's first frame were
//!   shown (milliseconds after `init`), how long the load took, the game
//!   stack's deepest use after it, and the files the load read: how many,
//!   which the bundle lacks, and the slowest. A loader names a step that
//!   reads no file with [`mark`], so its time is its own.
//! - **Each minute** (or [`RunReport::every_seconds`]): the top screen, the
//!   frames a second, the times of `App::frame` and of the draw, the stack,
//!   the music; and, with [`RunReport::phases`], each phase of the draw.
//!
//! Off, nothing is measured or logged. Times are whole microseconds from the
//! SDK's high-resolution timer, shown in milliseconds.

use alloc::borrow::ToOwned;
use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use core::sync::atomic::{AtomicBool, Ordering};

use shuyi_assets::bundle::Watcher;
use shuyi_ui::frame::onebit::Phase;

/// What the report says, and where else it writes the start: off until a
/// game makes one.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct RunReport {
    pub(crate) file: Option<String>,
    pub(crate) every_seconds: u32,
    pub(crate) slowest: usize,
    pub(crate) phases: bool,
    /// The game stack's size, which `main!` knows.
    pub(crate) stack_bytes: Option<usize>,
}

impl Default for RunReport {
    fn default() -> Self {
        Self::new()
    }
}

impl RunReport {
    /// The report in the console only: the running lines every 60 seconds,
    /// the 10 slowest reads, no draw phases.
    pub const fn new() -> Self {
        Self {
            file: None,
            every_seconds: 60,
            slowest: 10,
            phases: false,
            stack_bytes: None,
        }
    }

    /// Writes the start's lines and every read to `path` in the game's data
    /// folder too, once the first frame is shown: the Simulator has no
    /// console to read.
    #[must_use]
    pub fn file(mut self, path: &str) -> Self {
        self.file = Some(path.to_owned());
        self
    }

    /// Logs the running lines every `seconds` (at least 1).
    #[must_use]
    pub const fn every_seconds(mut self, seconds: u32) -> Self {
        self.every_seconds = if seconds == 0 { 1 } else { seconds };
        self
    }

    /// Names the `count` slowest reads.
    #[must_use]
    pub const fn slowest(mut self, count: usize) -> Self {
        self.slowest = count;
        self
    }

    /// Times each phase of the draw ([`Phase`]), for a measuring build: the
    /// backend then draws every changed frame whole, through
    /// `onebit::render_marked`, instead of only where it changed.
    #[must_use]
    pub const fn phases(mut self, on: bool) -> Self {
        self.phases = on;
        self
    }

    /// The game stack is `bytes` long, for the stack line.
    #[doc(hidden)]
    #[must_use]
    pub const fn stack_bytes(mut self, bytes: usize) -> Self {
        self.stack_bytes = Some(bytes);
        self
    }
}

/// `n` with its thousands grouped: `1,234,567`.
pub(crate) fn grouped(n: u64) -> String {
    let digits = format!("{n}");
    let mut out = String::new();
    for (i, digit) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

/// `us` microseconds in milliseconds, to a tenth: `12.5`.
fn ms_tenths(us: u64) -> String {
    let tenths = us.saturating_add(50) / 100;
    format!("{}.{}", grouped(tenths / 10), tenths % 10)
}

/// `us` microseconds in whole milliseconds, rounded: `1,235`.
fn ms(us: u64) -> String {
    grouped(us.saturating_add(500) / 1000)
}

/// How long something took, time after time: the fewest, the most and the
/// average microseconds.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct Timing {
    count: u32,
    min_us: u64,
    max_us: u64,
    total_us: u64,
}

impl Timing {
    /// No times yet.
    pub const fn new() -> Self {
        Self {
            count: 0,
            min_us: 0,
            max_us: 0,
            total_us: 0,
        }
    }

    /// Counts one more time of `us` microseconds.
    pub fn add(&mut self, us: u64) {
        if self.count == 0 {
            self.min_us = us;
            self.max_us = us;
        } else {
            self.min_us = self.min_us.min(us);
            self.max_us = self.max_us.max(us);
        }
        self.count = self.count.saturating_add(1);
        self.total_us = self.total_us.saturating_add(us);
    }

    /// How many times were counted.
    pub const fn count(&self) -> u32 {
        self.count
    }

    /// The fewest microseconds, 0 before any.
    pub const fn min_us(&self) -> u64 {
        self.min_us
    }

    /// The most microseconds, 0 before any.
    pub const fn max_us(&self) -> u64 {
        self.max_us
    }

    /// The average microseconds, rounded down; 0 before any.
    pub fn avg_us(&self) -> u64 {
        self.total_us / u64::from(self.count.max(1))
    }

    /// `name: min 1.0 / avg 1.5 / max 3.0 ms, 1,800 times`, or `name: none`
    /// before any.
    pub fn line(&self, name: &str) -> String {
        if self.count == 0 {
            return format!("{name}: none");
        }
        format!(
            "{name}: min {} / avg {} / max {} ms, {} times",
            ms_tenths(self.min_us),
            ms_tenths(self.avg_us()),
            ms_tenths(self.max_us),
            grouped(u64::from(self.count))
        )
    }
}

/// Each phase of the draw, time after time, in [`Phase::ALL`]'s order.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Phases([Timing; Phase::ALL.len()]);

impl Phases {
    /// Counts `us` microseconds of `phase`.
    pub(crate) fn add(&mut self, phase: Phase, us: u64) {
        if let Some(i) = Phase::ALL.iter().position(|&p| p == phase) {
            self.0[i].add(us);
        }
    }

    /// Each phase's average and most.
    pub(crate) fn line(&self) -> String {
        let each: Vec<String> = Phase::ALL
            .iter()
            .zip(&self.0)
            .map(|(phase, t)| {
                format!(
                    "{} {} / {}",
                    phase.name(),
                    ms_tenths(t.avg_us()),
                    ms_tenths(t.max_us)
                )
            })
            .collect();
        format!("Draw by phase, avg / max ms: {}", each.join(", "))
    }
}

/// What the backend measured of its draws since the last take.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Measured {
    /// Each draw that reached the SDK's frame buffer: rendered and copied.
    pub(crate) draw: Timing,
    /// Each phase of those draws, when phases are timed.
    pub(crate) phases: Option<Phases>,
}

/// One entry of the reads: a file read, or a step [`mark`]ed.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Read {
    /// The path, or the step's name in brackets.
    name: String,
    /// Whether the bundle has it; `None` for a step.
    found: Option<bool>,
    /// When, in the timer's microseconds.
    at_us: u64,
}

/// The files the load reads from the bundle (`shuyi_assets::bundle::watch`),
/// each with when: a file's time is from its read to the next read, the
/// parsing and checking of what it holds. Only roughly so: a loader may read
/// several files before it parses them.
#[derive(Debug, Clone)]
pub(crate) struct Reads {
    /// The timer.
    clock: fn() -> u64,
    reads: Vec<Read>,
}

/// One file's reads, or one step, summed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FileTime {
    /// From each of its reads to the next read.
    us: u64,
    /// How many times it was read.
    reads: u32,
    found: Option<bool>,
}

impl Reads {
    /// No reads yet, timed by `clock`.
    pub(crate) const fn new(clock: fn() -> u64) -> Self {
        Self {
            clock,
            reads: Vec::new(),
        }
    }

    /// Notes a read of `path` now.
    fn add(&mut self, path: &str, found: bool) {
        let at_us = (self.clock)();
        self.reads.push(Read {
            name: path.to_owned(),
            found: Some(found),
            at_us,
        });
    }

    /// Notes that the step `name` began now.
    fn mark(&mut self, name: &str) {
        let at_us = (self.clock)();
        self.reads.push(Read {
            name: format!("({name})"),
            found: None,
            at_us,
        });
    }

    /// Each file and step by name, the last one's time running to `end_us`.
    fn files(&self, end_us: u64) -> BTreeMap<&str, FileTime> {
        let mut files: BTreeMap<&str, FileTime> = BTreeMap::new();
        let ends = self.reads.iter().skip(1).map(|r| r.at_us).chain([end_us]);
        for (read, end) in self.reads.iter().zip(ends) {
            let file = files.entry(&read.name).or_insert(FileTime {
                us: 0,
                reads: 0,
                found: read.found,
            });
            file.us += end.saturating_sub(read.at_us);
            file.reads += 1;
        }
        files
    }

    /// The files and steps by time, the slowest first, by name where equal.
    fn by_time(&self, end_us: u64) -> Vec<(&str, FileTime)> {
        let mut files: Vec<(&str, FileTime)> = self.files(end_us).into_iter().collect();
        files.sort_by(|a, b| b.1.us.cmp(&a.1.us).then(a.0.cmp(b.0)));
        files
    }

    /// How many reads of how many files, which the bundle lacks, and the
    /// `slowest` slowest files and steps.
    pub(crate) fn summary(&self, end_us: u64, slowest: usize) -> Vec<String> {
        let files = self.by_time(end_us);
        let missing: Vec<&str> = files
            .iter()
            .filter(|(_, f)| f.found == Some(false))
            .map(|(name, _)| *name)
            .collect();
        let reads = self.reads.iter().filter(|r| r.found.is_some()).count();
        let read = files.iter().filter(|(_, f)| f.found.is_some()).count();
        let mut lines = Vec::from([format!(
            "Bundle: {} reads of {} files, {} not in the bundle",
            grouped(reads as u64),
            grouped(read as u64),
            grouped(missing.len() as u64)
        )]);
        if !missing.is_empty() {
            lines.push(format!("Not in the bundle: {}", missing.join(", ")));
        }
        let names: Vec<String> = files
            .iter()
            .take(slowest)
            .map(|(name, f)| format!("{name} {}", ms(f.us)))
            .collect();
        if !names.is_empty() {
            lines.push(format!(
                "Slowest reads, ms to the next read: {}",
                names.join(", ")
            ));
        }
        lines
    }

    /// Every file and step, the slowest first: its time, and how many times
    /// it was read when more than once.
    pub(crate) fn each(&self, end_us: u64) -> Vec<String> {
        self.by_time(end_us)
            .into_iter()
            .map(|(name, f)| {
                let reads = if f.reads > 1 {
                    format!(", {} reads", grouped(u64::from(f.reads)))
                } else {
                    String::new()
                };
                let missing = if f.found == Some(false) {
                    ", not in the bundle"
                } else {
                    ""
                };
                format!("  {name}: {} ms{reads}{missing}", ms_tenths(f.us))
            })
            .collect()
    }
}

/// Runs `f` on the reads being noted, if any: the bundle's watcher is a
/// plain function, so they live outside the driver. The tests keep one per
/// thread, as each runs a game of its own.
#[cfg(test)]
pub(crate) fn with_reads<R>(f: impl FnOnce(&mut Option<Reads>) -> R) -> R {
    std::thread_local! {
        static READS: core::cell::RefCell<Option<Reads>> = const { core::cell::RefCell::new(None) };
    }
    READS.with_borrow_mut(f)
}

/// Runs `f` on the reads being noted, if any: the bundle's watcher is a
/// plain function, so they live outside the driver.
#[cfg(all(not(test), shuyi_playdate_sdk))]
pub(crate) fn with_reads<R>(f: impl FnOnce(&mut Option<Reads>) -> R) -> R {
    crate::device::with_reads(f)
}

/// Off the device no game runs, so there are no reads to note.
#[cfg(all(not(test), not(shuyi_playdate_sdk)))]
pub(crate) fn with_reads<R>(f: impl FnOnce(&mut Option<Reads>) -> R) -> R {
    f(&mut None)
}

/// Whether [`noted`] is the bundle's watcher.
static WATCHING: AtomicBool = AtomicBool::new(false);

/// Sets [`noted`] as the bundle's watcher (`shuyi_assets::bundle::watch`):
/// `Ok` if it is, from now or from an earlier report; else the other
/// watcher, which was set first.
pub(crate) fn watch_bundle(watcher: Watcher) -> Result<(), Watcher> {
    match shuyi_assets::bundle::watch(watcher) {
        Ok(()) => {
            WATCHING.store(true, Ordering::Relaxed);
            Ok(())
        }
        Err(_) if WATCHING.load(Ordering::Relaxed) => Ok(()),
        Err(other) => Err(other),
    }
}

/// The bundle's watcher while the report is on: notes the read, during the
/// load.
pub(crate) fn noted(path: &str, found: bool) {
    with_reads(|reads| {
        if let Some(reads) = reads {
            reads.add(path, found);
        }
    });
}

/// Names the step `name` of the game's load, which reads no file: the time
/// from here to the next read is its own, not the last file's. Nothing
/// happens unless the report is on and the game is loading.
pub fn mark(name: &str) {
    with_reads(|reads| {
        if let Some(reads) = reads {
            reads.mark(name);
        }
    });
}

/// The line of the game stack's deepest use.
pub(crate) fn stack_line(used: Option<usize>, size: Option<usize>) -> String {
    match (used, size) {
        (Some(used), Some(size)) => format!(
            "Game stack used: {} of {} bytes",
            grouped(used as u64),
            grouped(size as u64)
        ),
        (Some(used), None) => format!("Game stack used: {} bytes", grouped(used as u64)),
        (None, _) => "Game stack: measured on the device only".to_owned(),
    }
}

/// The start's line of when `what` reached the screen, `us` after `init`.
pub(crate) fn shown_line(what: &str, us: u64) -> String {
    format!("{what} shown {} ms after start", ms(us))
}

/// The start's line of the load's length.
pub(crate) fn load_line(us: u64) -> String {
    format!("Game data loaded in {} ms", ms(us))
}

/// The line said when another watcher of the bundle was set first.
// check-text: not player text (for the console)
pub(crate) const REFUSED: &str = "Bundle reads: not timed, as another watcher was set first";

/// The running numbers over one stretch ([`RunReport::every_seconds`]).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Stretch {
    /// Each `App::frame`.
    pub(crate) frame: Timing,
}

/// What the running lines name besides the stretch's own numbers.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Now<'a> {
    /// The top screen, if one is open.
    pub(crate) screen: Option<&'a str>,
    /// The stretch's length in milliseconds.
    pub(crate) ms: u64,
    pub(crate) stack_used: Option<usize>,
    /// The music heard, if any.
    pub(crate) music: Option<&'a str>,
    /// Whether a file player ran out of data in the stretch.
    pub(crate) underran: bool,
}

impl Stretch {
    /// The running lines.
    pub(crate) fn lines(
        &self,
        now: &Now<'_>,
        measured: &Measured,
        report: &RunReport,
    ) -> Vec<String> {
        let frames = u64::from(self.frame.count());
        let tenths = frames * 10_000 / now.ms.max(1);
        let screen = now.screen.unwrap_or("no screen");
        let music = match (now.music, now.underran) {
            (Some(cue), true) => format!("Music: {cue}, and a file player ran out of data"),
            (Some(cue), false) => format!("Music: {cue}, no file player ran out of data"),
            (None, true) => "Music: none, and a file player ran out of data".to_owned(),
            (None, false) => "Music: none".to_owned(),
        };
        let mut lines = Vec::from([
            format!(
                "On {screen} for {} seconds: {}.{} frames a second",
                grouped((now.ms + 500) / 1000),
                grouped(tenths / 10),
                tenths % 10
            ),
            self.frame.line("App::frame"),
            measured.draw.line("Draw and flush"),
        ]);
        if let Some(phases) = &measured.phases {
            lines.push(phases.line());
        }
        lines.push(stack_line(now.stack_used, report.stack_bytes));
        lines.push(music);
        lines
    }
}

#[cfg(test)]
mod tests;
