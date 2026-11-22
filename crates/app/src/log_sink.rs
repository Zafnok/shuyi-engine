//! Where the log's lines go in a running game (ticket 3041, ADR-0030): a
//! `log::Log` that [`run_app`](crate::run_app) installs first thing
//! ([`install`]), so the engine's lines, `shuyi-app`'s, the libraries' and
//! a panic's message reach the same sinks.
//!
//! - **A file** on a desktop, `latest.log` in the game's logs folder
//!   ([`storage::log_dir`](crate::storage::log_dir)). Each start moves the
//!   last three runs' files along (`previous-1.log` to `previous-3.log`,
//!   oldest last) and starts a new one whose first line names the engine,
//!   the system, the build, the storage and the game's version. A run
//!   writes at most [`LOG_FILE_CAP`], then one line saying the log was cut,
//!   then [`LOG_FILE_ERROR_ROOM`] more of errors alone. Each line is
//!   written whole, unbuffered, so a crash loses nothing written before it.
//! - **stderr** on a desktop, in a debug build: a release build on Windows
//!   has no console.
//! - **The browser's console** on the web, by level
//!   (`console.error` to `console.debug`).
//! - **A crash report's lines** ([`crash::note_log`](crate::crash::note_log)).
//!
//! Each line reads `{seconds} f{frame} {LEVEL} {target}: {text}`: the
//! seconds since [`install`], the frame about to run as the loop set it
//! ([`set_frame`]), 0 before the first, and the line's target, which is a
//! [`LogCategory`](shuyi_ui::log::LogCategory)'s name for the engine's
//! lines, `panic` for a panic's and a module path for a library's.
//!
//! Which lines pass is the default [`LogFilter`]: `info` and more serious
//! for the engine's categories, `warn` and more serious for any other
//! target. A build without debug tools passes warnings and errors alone
//! (ADR-0094 §4).

use std::cell::Cell;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::panic::PanicHookInfo;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Mutex, MutexGuard};

use shuyi_ui::log::{LogFilter, LogLevel};

/// How many bytes a run's file holds before it is cut: 2 MiB.
pub const LOG_FILE_CAP: u64 = 2 * 1024 * 1024;

/// How many bytes of errors a run's file takes after it is cut: 256 KiB.
pub const LOG_FILE_ERROR_ROOM: u64 = 256 * 1024;

/// The file of this run, in the logs folder.
pub const LATEST: &str = "latest.log";

/// How many earlier runs' files are kept beside this one's.
pub const PREVIOUS_RUNS: usize = 3;

/// The target of a panic's line.
pub const PANIC_TARGET: &str = "panic";

/// The frame about to run ([`set_frame`]).
static FRAME: AtomicU32 = AtomicU32::new(0);

thread_local! {
    /// Whether this thread is inside the sink: a line it logs from there
    /// (a panic of the sink's own) is dropped, never waited for.
    static IN_SINK: Cell<bool> = const { Cell::new(false) };
}

/// The frame about to run, `App::frame_number() + 1`, which every line from
/// now on names. The loop sets it before each frame.
pub fn set_frame(frame: u32) {
    FRAME.store(frame, Ordering::Relaxed);
}

/// The frame [`set_frame`] set last: 0 before the first.
pub fn frame() -> u32 {
    FRAME.load(Ordering::Relaxed)
}

/// Installs the sink as the `log` crate's logger, with its file in `dir`
/// (none if `None`), and the panic's line in the one panic hook
/// (`crash`'s). `storage_name` and `game_version` go in the file's first
/// line.
///
/// If a logger was installed before (a game's own), it stays, this one is
/// dropped, and a line on stderr says so. If the folder can't be made or
/// the file opened, there is no file, and a line on stderr says why.
pub fn install(storage_name: &str, game_version: &str, dir: Option<PathBuf>) {
    let filter = LogFilter::default();
    let most = if KEEPS_EVERY_LEVEL {
        filter.most_detailed()
    } else {
        filter.most_detailed().min(LogLevel::Warn)
    };
    let sink: &'static Sink = Box::leak(Box::new(Sink {
        filter,
        stderr: !cfg!(target_arch = "wasm32") && cfg!(debug_assertions),
        start: crate::pacing::seconds(),
        file: Mutex::new(None),
    }));
    crate::crash::hook_once();
    if log::set_logger(sink).is_err() {
        say_on_stderr("shuyi log: a logger was installed before this one; it stays");
        return;
    }
    log::set_max_level(log_filter(most));
    if let Some(dir) = dir {
        match LogFile::open(&dir, &first_line(storage_name, game_version)) {
            Ok(file) => *lock(&sink.file) = Some(file),
            Err(e) => say_on_stderr(&format!("shuyi log: no log file in {}: {e}", dir.display())),
        }
    }
}

/// Logs the panic `info` describes, as one error line of the target
/// [`PANIC_TARGET`]: its message, where and on which thread. The panic
/// hook calls it.
pub(crate) fn log_panic(info: &PanicHookInfo<'_>) {
    let message = crate::crash::payload_text(info).unwrap_or_else(|| "Box<dyn Any>".to_owned());
    let at = info.location().map_or_else(
        || "an unknown place".to_owned(),
        |at| format!("{}:{}", at.file(), at.line()),
    );
    let thread = std::thread::current();
    let name = thread.name().unwrap_or("unnamed");
    log::error!(target: PANIC_TARGET, "{message} at {at} on the thread {name}");
}

/// The sink [`install`] leaks.
struct Sink {
    /// Which lines pass.
    filter: LogFilter,
    /// Whether lines go to stderr too.
    stderr: bool,
    /// `pacing::seconds()` at [`install`].
    start: f64,
    /// The run's file, if there is one.
    file: Mutex<Option<LogFile>>,
}

impl log::Log for Sink {
    fn enabled(&self, metadata: &log::Metadata<'_>) -> bool {
        let level = level_of(metadata.level());
        kept_in_this_build(level) && self.filter.allows_target(metadata.target(), level)
    }

    fn log(&self, record: &log::Record<'_>) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let Some(_inside) = Inside::enter() else {
            return;
        };
        let (level, target) = (level_of(record.level()), record.target());
        let text = record.args().to_string();
        let frame = frame();
        let seconds = crate::pacing::seconds() - self.start;
        let line = format_line(seconds, frame, level, target, &text);
        if self.stderr {
            say_on_stderr(&line);
        }
        #[cfg(target_arch = "wasm32")]
        to_console(level, &line);
        if let Some(file) = lock(&self.file).as_mut() {
            file.write_line(seconds, level, &line);
        }
        let name = level.name();
        crate::crash::note_log(format!("f{frame} {name} {target}: {text}"));
    }

    fn flush(&self) {}
}

/// This thread inside the sink, until dropped.
struct Inside;

impl Inside {
    /// Marks this thread inside the sink, or `None` if it already is.
    fn enter() -> Option<Self> {
        (!IN_SINK.with(|inside| inside.replace(true))).then_some(Self)
    }
}

impl Drop for Inside {
    fn drop(&mut self) {
        IN_SINK.with(|inside| inside.set(false));
    }
}

/// `mutex`'s guard, poisoned or not.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Writes `line` on stderr, if there is one. Never panics (`eprintln!`
/// does when the write fails).
fn say_on_stderr(line: &str) {
    let _ = writeln!(io::stderr(), "{line}");
}

/// Whether this build passes lines of every level: one with debug tools, or
/// with the capture mode (the `capture` feature, never in a shipped build),
/// whose `shuyi-capture: ` lines are `Info` and read by the checks.
const KEEPS_EVERY_LEVEL: bool = shuyi_ui::DEBUG_TOOLS || cfg!(feature = "capture");

/// Whether a line at `level` reaches the sinks in this build, whatever the
/// filter says: every line with debug tools or the capture mode, warnings
/// and errors without (ADR-0094 §4).
pub(crate) fn kept_in_this_build(level: LogLevel) -> bool {
    KEEPS_EVERY_LEVEL || matches!(level, LogLevel::Error | LogLevel::Warn)
}

/// The engine's level of a `log` level: `Trace` is `Debug`.
pub(crate) fn level_of(level: log::Level) -> LogLevel {
    match level {
        log::Level::Error => LogLevel::Error,
        log::Level::Warn => LogLevel::Warn,
        log::Level::Info => LogLevel::Info,
        log::Level::Debug | log::Level::Trace => LogLevel::Debug,
    }
}

/// The `log` level of the engine's `level`.
pub(crate) fn log_level(level: LogLevel) -> log::Level {
    match level {
        LogLevel::Error => log::Level::Error,
        LogLevel::Warn => log::Level::Warn,
        LogLevel::Info => log::Level::Info,
        _ => log::Level::Debug,
    }
}

/// The `log` crate's most detailed level of a sink that asks for `level`.
fn log_filter(level: LogLevel) -> log::LevelFilter {
    log_level(level).to_level_filter()
}

/// The name of `level` in a line: `ERROR`, `WARN`, `INFO` or `DEBUG`.
fn level_word(level: LogLevel) -> &'static str {
    match level {
        LogLevel::Error => "ERROR",
        LogLevel::Warn => "WARN",
        LogLevel::Info => "INFO",
        _ => "DEBUG",
    }
}

/// One line, without its end: `{seconds:>9.3} f{frame} {LEVEL:<5}
/// {target}: {text}`.
pub(crate) fn format_line(
    seconds: f64,
    frame: u32,
    level: LogLevel,
    target: &str,
    text: &str,
) -> String {
    let word = level_word(level);
    format!("{seconds:>9.3} f{frame} {word:<5} {target}: {text}")
}

/// The file's first line: the engine's version, the system and the
/// build, the storage's name and the game's version.
pub(crate) fn first_line(storage_name: &str, game_version: &str) -> String {
    let (os, arch) = (std::env::consts::OS, std::env::consts::ARCH);
    let build = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    let engine = env!("CARGO_PKG_VERSION");
    let game = if game_version.is_empty() {
        "unknown"
    } else {
        game_version
    };
    let text = format!(
        "Shuyi Engine {engine}, {os} {arch} {build}, storage {storage_name}, game version {game}"
    );
    format_line(0.0, 0, LogLevel::Info, "platform", &text)
}

/// Which console function a line at `level` goes to on the web.
#[cfg(any(target_arch = "wasm32", test))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Console {
    /// `console.error`.
    Error,
    /// `console.warn`.
    Warn,
    /// `console.info`.
    Info,
    /// `console.debug`.
    Debug,
}

/// The console function of a line at `level`.
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) fn console_of(level: LogLevel) -> Console {
    match level {
        LogLevel::Error => Console::Error,
        LogLevel::Warn => Console::Warn,
        LogLevel::Info => Console::Info,
        _ => Console::Debug,
    }
}

/// `line` as the console takes it: each NUL replaced by U+FFFD (miniquad
/// panics on one).
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) fn console_text(line: &str) -> std::borrow::Cow<'_, str> {
    if line.contains('\0') {
        line.replace('\0', "\u{FFFD}").into()
    } else {
        line.into()
    }
}

/// Writes `line` to the browser's console, by `level`.
#[cfg(target_arch = "wasm32")]
fn to_console(level: LogLevel, line: &str) {
    use macroquad::miniquad;
    let text = console_text(line);
    match console_of(level) {
        Console::Error => {
            miniquad::error!("{}", text);
        }
        Console::Warn => {
            miniquad::warn!("{}", text);
        }
        Console::Info => {
            miniquad::info!("{}", text);
        }
        Console::Debug => {
            miniquad::debug!("{}", text);
        }
    }
}

/// The name of the `n`th earlier run's file: `previous-1.log` is the last.
fn previous(n: usize) -> String {
    format!("previous-{n}.log")
}

/// A run's file, written line by line ([module docs](self)).
pub(crate) struct LogFile {
    /// The file, opened to append.
    file: File,
    /// How many bytes it holds.
    written: u64,
    /// Bytes before the file is cut.
    cap: u64,
    /// Bytes of errors after it is cut.
    error_room: u64,
    /// Once it is cut, the bytes it may hold: those when the note saying so
    /// was written, and the room for errors.
    cut_at: Option<u64>,
}

impl LogFile {
    /// Makes `dir`, moves the earlier runs' files along ([`rotate`]) and
    /// starts `latest.log` with `first`, with the caps [`LOG_FILE_CAP`] and
    /// [`LOG_FILE_ERROR_ROOM`].
    pub(crate) fn open(dir: &Path, first: &str) -> io::Result<Self> {
        Self::open_capped(dir, first, LOG_FILE_CAP, LOG_FILE_ERROR_ROOM)
    }

    /// [`open`](Self::open), with the caps `cap` and `error_room`.
    pub(crate) fn open_capped(
        dir: &Path,
        first: &str,
        cap: u64,
        error_room: u64,
    ) -> io::Result<Self> {
        fs::create_dir_all(dir)?;
        rotate(dir)?;
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(dir.join(LATEST))?;
        let written = file.metadata().map_or(0, |m| m.len());
        let mut log = Self {
            file,
            written,
            cap,
            error_room,
            cut_at: None,
        };
        log.write_line(0.0, LogLevel::Info, first);
        Ok(log)
    }

    /// Writes `line` and its end, if the caps let a line at `level` in:
    /// up to the cap any line, then the line saying the file was cut (at
    /// `seconds`), then errors alone up to the room for them. A write that
    /// fails is dropped.
    pub(crate) fn write_line(&mut self, seconds: f64, level: LogLevel, line: &str) {
        let bytes = u64::try_from(line.len())
            .unwrap_or(u64::MAX)
            .saturating_add(1);
        let limit = match self.cut_at {
            Some(limit) => limit,
            None if self.written.saturating_add(bytes) <= self.cap => {
                self.write(line);
                return;
            }
            None => {
                let note = format_line(
                    seconds,
                    frame(),
                    LogLevel::Warn,
                    "platform",
                    "the log was cut here: errors alone from now on",
                );
                self.write(&note);
                *self
                    .cut_at
                    .insert(self.written.saturating_add(self.error_room))
            }
        };
        if level == LogLevel::Error && self.written.saturating_add(bytes) <= limit {
            self.write(line);
        }
    }

    /// Writes `line` and its end with one `write_all`.
    fn write(&mut self, line: &str) {
        let whole = format!("{line}\n");
        if self.file.write_all(whole.as_bytes()).is_ok() {
            let bytes = u64::try_from(whole.len()).unwrap_or(u64::MAX);
            self.written = self.written.saturating_add(bytes);
        }
    }
}

/// Moves the earlier runs' files in `dir` along: `previous-2.log` becomes
/// `previous-3.log` and so on, `latest.log` becomes `previous-1.log`, and
/// the oldest is deleted. A file that isn't there is skipped.
pub(crate) fn rotate(dir: &Path) -> io::Result<()> {
    skip_missing(fs::remove_file(dir.join(previous(PREVIOUS_RUNS))))?;
    for n in (1..PREVIOUS_RUNS).rev() {
        skip_missing(fs::rename(dir.join(previous(n)), dir.join(previous(n + 1))))?;
    }
    skip_missing(fs::rename(dir.join(LATEST), dir.join(previous(1))))
}

/// `result`, with a file that isn't there taken as done.
fn skip_missing(result: io::Result<()>) -> io::Result<()> {
    match result {
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        result => result,
    }
}

#[cfg(test)]
mod tests;
