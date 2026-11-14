//! Crash reports (ADR-0079): what a backend's panic hook writes through the
//! game's [`Storage`], and what the game hears of it at its next start
//! ([`Setup::last_crash`](crate::Setup::last_crash)).
//!
//! A report ([`CrashReport`]) is stored at `crash-<when>` ([`crash_key`]),
//! `when` being milliseconds since 1970 on the backend's clock. It holds the
//! panic's message, place, thread and backtrace, the engine's, the game's
//! and the target's versions, the frame and the screen that was running,
//! the last lines of the log and the game's own record of its play. It never
//! holds the player's settings, saves or keymap, nor what the player typed
//! unless the game's record holds it. Reports stay on the machine.
//!
//! A report is a record in storage, like the settings and the profiles, not
//! an asset kind a game ships, so no format is registered for it. Its reader
//! ([`CrashReport::parse`]) still keeps ADR-0007 §5: a size limit, errors and
//! never a panic.

use crate::alloc_prelude::*;
use alloc::format;

use serde::{Deserialize, Serialize};
use shuyi_ui::storage::{Storage, StorageError};

/// The version of the report's text this build writes.
pub const REPORT_VERSION: u32 = 1;
/// The engine's version a report names: every `shuyi-*` crate has the
/// workspace's.
pub const ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");
/// The longest message kept, in bytes.
pub const MESSAGE_BYTES: usize = 4 * 1024;
/// The longest backtrace kept, in bytes.
pub const BACKTRACE_BYTES: usize = 32 * 1024;
/// The most frames of the stack kept ([`CrashReport::frames`]): the
/// innermost.
pub const FRAMES: usize = 128;
/// The most lines of the log kept: the last ones.
pub const LOG_LINES: usize = 200;
/// The longest line of the log kept, in bytes.
pub const LOG_LINE_BYTES: usize = 1024;
/// The longest record of the game's play kept, in bytes: a longer one is
/// left out ([`CrashReport::set_record`]).
pub const RECORD_BYTES: usize = 1024 * 1024;
/// The longest text [`CrashReport::parse`] reads, in bytes.
pub const TEXT_BYTES: usize = 2 * 1024 * 1024;
/// The key that names the last report offered to the game
/// ([`offer_last_crash`]).
pub const OFFERED_KEY: &str = "crash-offered";

/// What every report's key starts with.
const PREFIX: &str = "crash-";

/// What a backend's panic hook found: plain data, written with
/// [`to_text`](Self::to_text) and read with [`parse`](Self::parse). Every
/// field but [`version`](Self::version) and [`message`](Self::message) may
/// be missing from a report's text, so a newer field never stops an older
/// report from reading.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CrashReport {
    /// The version of its text: [`REPORT_VERSION`].
    pub version: u32,
    /// When it happened: milliseconds since 1970 on the backend's clock.
    #[serde(default)]
    pub when: u64,
    /// The panic's message, cut at [`MESSAGE_BYTES`].
    pub message: String,
    /// Whether the panic ended the game: one on the thread that runs it.
    /// A panic on another thread (a sound library's, with no sound device)
    /// is kept, and never offered ([`offer_last_crash`]). A report without
    /// the field, older than it, reads as `true`.
    #[serde(default = "ended_the_game")]
    pub fatal: bool,
    /// Where it panicked: `file:line:column`.
    #[serde(default)]
    pub location: Option<String>,
    /// The thread's name, if it has one.
    #[serde(default)]
    pub thread: Option<String>,
    /// The backtrace as the standard library writes it, cut at
    /// [`BACKTRACE_BYTES`]. A stripped build's frames are `<unknown>`.
    #[serde(default)]
    pub backtrace: String,
    /// The panicking thread's stack as offsets into the executable, the
    /// innermost first, up to [`FRAMES`]: what a debug file of the same
    /// build names (`docs/crash-reports.md`). Filled on Windows and Linux;
    /// empty on macOS, on the web and in a report older than the field.
    #[serde(default)]
    pub frames: Vec<CodeFrame>,
    /// The engine's version ([`ENGINE_VERSION`]).
    #[serde(default)]
    pub engine: String,
    /// The game's version, empty if the game gave none.
    #[serde(default)]
    pub game: String,
    /// The OS, the processor and the build: `linux x86_64 release`.
    #[serde(default)]
    pub target: String,
    /// The frame that was running, counted as
    /// [`App::frame_number`](crate::App::frame_number) counts them.
    #[serde(default)]
    pub frame: u32,
    /// The name of the screen on top, if the game was running.
    #[serde(default)]
    pub top_screen: Option<String>,
    /// The last [`LOG_LINES`] lines of the log, oldest first, each cut at
    /// [`LOG_LINE_BYTES`].
    #[serde(default)]
    pub log: Vec<String>,
    /// The game's record of its play, in the format its tools' `replay`
    /// reads ([`Start::crash_record`](crate::Start::crash_record)).
    #[serde(default)]
    pub record: Option<String>,
}

/// A frame of a report's stack (ticket 2598): where its call returns to,
/// as an offset from the start of the executable in memory. The program
/// loads at another address each run, so the offset, not the address, is
/// what reads against its debug file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CodeFrame {
    /// The return address minus the executable's load address: on Windows
    /// the address relative to the image base, on Linux the address in the
    /// file (a PIE's first segment is at 0). `None` for a frame in another
    /// module, such as a system library.
    #[serde(default)]
    pub offset: Option<u64>,
}

impl CodeFrame {
    /// A frame at `offset` into the executable, or in another module.
    #[must_use]
    pub const fn new(offset: Option<u64>) -> Self {
        Self { offset }
    }
}

impl CrashReport {
    /// A report of `message` at `when`, cut at [`MESSAGE_BYTES`], and
    /// nothing else known yet.
    pub fn new(when: u64, message: impl Into<String>) -> Self {
        let mut message = message.into();
        cut(&mut message, MESSAGE_BYTES);
        Self {
            version: REPORT_VERSION,
            when,
            message,
            fatal: true,
            location: None,
            thread: None,
            backtrace: String::new(),
            frames: Vec::new(),
            engine: ENGINE_VERSION.to_owned(),
            game: String::new(),
            target: String::new(),
            frame: 0,
            top_screen: None,
            log: Vec::new(),
            record: None,
        }
    }

    /// Cuts the message, the backtrace and the log to their limits: the
    /// last [`LOG_LINES`] lines, each at [`LOG_LINE_BYTES`].
    pub fn limit(&mut self) {
        cut(&mut self.message, MESSAGE_BYTES);
        cut(&mut self.backtrace, BACKTRACE_BYTES);
        self.frames.truncate(FRAMES);
        let extra = self.log.len().saturating_sub(LOG_LINES);
        self.log.drain(..extra);
        for line in &mut self.log {
            cut(line, LOG_LINE_BYTES);
        }
    }

    /// Sets the game's record of its play, unless it is over
    /// [`RECORD_BYTES`]: then the record stays `None`, and the log's last
    /// line says why.
    pub fn set_record(&mut self, record: Option<String>) {
        match record {
            Some(text) if text.len() > RECORD_BYTES => {
                self.record = None;
                self.log.push(format!(
                    "the game's record was left out: {} bytes, over the {RECORD_BYTES} a report keeps",
                    text.len()
                ));
                self.limit();
            }
            record => self.record = record,
        }
    }

    /// The report's text: RON, with `\n` line endings on every OS.
    pub fn to_text(&self) -> String {
        let config = ron::ser::PrettyConfig::new().indentor(" ").new_line("\n");
        // RON refuses no value a report holds; a hook writes nothing rather
        // than panic.
        ron::ser::to_string_pretty(self, config).unwrap_or_default()
    }

    /// The report `text` holds, or why it holds none: text over
    /// [`TEXT_BYTES`] isn't read. Never panics.
    pub fn parse(text: &str) -> Result<Self, String> {
        if text.len() > TEXT_BYTES {
            return Err(format!(
                "a crash report of {} bytes, over the {TEXT_BYTES} one may be",
                text.len()
            ));
        }
        ron::from_str(text).map_err(|e| e.to_string())
    }
}

/// What a report without [`CrashReport::fatal`] reads as: every report
/// written before it was offered as a crash.
const fn ended_the_game() -> bool {
    true
}

/// `text` cut at `bytes` bytes, at the last whole character that fits.
fn cut(text: &mut String, bytes: usize) {
    // At or past its end, the boundary is its end: nothing is cut.
    let end = text.floor_char_boundary(bytes);
    text.truncate(end);
}

/// The key a report made at `when` is stored at: `crash-<when>`.
pub fn crash_key(when: u64) -> String {
    format!("{PREFIX}{when}")
}

/// When the report at `key` was made, if `key` is a report's: `crash-` and
/// digits only.
pub fn report_when(key: &str) -> Option<u64> {
    let digits = key.strip_prefix(PREFIX)?;
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    digits.parse().ok()
}

/// The keys of the reports in `storage`, newest first. Every other key is
/// ignored, and a storage that can't list gives none.
pub fn report_keys(storage: &dyn Storage) -> Vec<String> {
    let mut keys: Vec<(u64, String)> = storage
        .list()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|key| Some((report_when(&key)?, key)))
        .collect();
    keys.sort_by_key(|(when, _)| core::cmp::Reverse(*when));
    keys.into_iter().map(|(_, key)| key).collect()
}

/// Deletes all but the newest `n` reports in `storage`.
pub fn keep_newest(storage: &mut dyn Storage, n: usize) -> Result<(), StorageError> {
    for key in report_keys(storage).iter().skip(n) {
        storage.delete(key)?;
    }
    Ok(())
}

/// What a game hears at its start of the last crash: when, the message and
/// the place, and the report's key in its storage. What the player sees of
/// it, if anything, is the game's to word.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct LastCrash {
    /// When it happened: milliseconds since 1970.
    pub when: u64,
    /// The panic's message.
    pub message: String,
    /// Where it panicked: `file:line:column`.
    pub location: Option<String>,
    /// The report's key in the game's storage.
    pub key: String,
}

/// The newest [`fatal`](CrashReport::fatal) report in `storage` newer than
/// the one [`OFFERED_KEY`] names; from then on it names the newest report,
/// so each is offered or skipped once. A report that isn't fatal, or
/// doesn't read, is skipped.
pub fn offer_last_crash(storage: &mut dyn Storage) -> Option<LastCrash> {
    let offered = storage
        .read(OFFERED_KEY)
        .ok()
        .flatten()
        .and_then(|text| ron::from_str::<String>(&text).ok())
        .and_then(|key| report_when(&key));
    let newer: Vec<String> = report_keys(storage)
        .into_iter()
        .filter(|key| report_when(key) > offered)
        .collect();
    let newest = newer.first()?;
    let marked = ron::to_string(newest).unwrap_or_default();
    // A storage that can't write offers it again next time: no harm.
    let _ = storage.write(OFFERED_KEY, &marked);
    newer.into_iter().find_map(|key| {
        let report = CrashReport::parse(&storage.read(&key).ok()??).ok()?;
        report.fatal.then_some(LastCrash {
            when: report.when,
            message: report.message,
            location: report.location,
            key,
        })
    })
}

#[cfg(test)]
mod tests;
