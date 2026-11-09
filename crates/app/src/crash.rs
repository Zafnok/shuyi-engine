//! Crash reports (ADR-0079): a panic hook that writes a
//! `shuyi_platform::crash::CrashReport` through the game's storage, which
//! [`run`](crate::run) installs before its first frame
//! ([`AppConfig::crash`](crate::AppConfig::crash)).
//!
//! The hook takes the one before it and calls it after writing, so the
//! standard library's message in a terminal and miniquad's console line on
//! the web stay. It reads the frame and the top screen [`run`](crate::run)
//! set ([`set_frame`]) and the last lines of the log ([`note_log`]), from
//! statics, so a panic on another thread (the music's worker) gets them too.
//! A panic caught on purpose runs in [`quiet`] and writes nothing.
//!
//! A report is [`fatal`](CrashReport::fatal) when the panic is on the
//! thread that runs the game, the one `run` installs the hook from: on
//! native `run` catches it and ends the process, or it is outside that
//! catch and ends it itself; on the web every panic aborts. A panic on
//! another thread (quad-snd's own, on a machine with no sound device) is
//! kept, and not offered to the game as a crash.
//!
//! On native, `run` catches a panic of the game's frame: it stops the sound,
//! adds the game's record of its play to the report ([`add_record`]) and
//! ends the process with exit code 101, as an uncaught panic does, without
//! calling the game's system hook, so a game in a broken state never saves
//! over a good save. On the web a panic aborts, so a web report has no
//! record.
//!
//! `SHUYI_PANIC_AT_FRAME=<n>`, read once by `run` on native, panics at frame
//! `n` on purpose ([`PANIC_ON_PURPOSE`]), for the tests.
//!
//! The record and `SHUYI_PANIC_AT_FRAME` are a build with debug tools'
//! only, unless the game says [`CrashReports::with_record_in_release`]
//! ([`CrashReports::records_play`]): together they write a shipped game's
//! state on demand. Without debug tools the log's lines in a report are
//! its warnings and errors.

use std::cell::Cell;
use std::collections::VecDeque;
use std::panic::PanicHookInfo;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, TryLockError};
use std::thread::ThreadId;

use shuyi_platform::crash::{CrashReport, LOG_LINES, crash_key, keep_newest};
use shuyi_ui::storage::Storage;

/// Whether `run` writes crash reports, and with what version of the game.
/// Build it with [`CrashReports::new`] or [`CrashReports::off`]; a game
/// sets [`AppConfig::crash`](crate::AppConfig::crash) to it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct CrashReports {
    /// The game's version, in each report: empty if the game gave none.
    pub game_version: &'static str,
    /// How many reports are kept, the newest: 0 installs no hook.
    pub keep: u32,
    /// Whether a build without debug tools adds the game's record of its
    /// play to a report and reads `SHUYI_PANIC_AT_FRAME` too
    /// ([`with_record_in_release`](Self::with_record_in_release)). A build
    /// with debug tools always does.
    pub record_in_release: bool,
}

impl CrashReports {
    /// Reports naming `game_version`: the newest 5 kept on native, and 1 on
    /// the web, where `localStorage` holds about 5 MB for the whole site.
    /// With the record of play in a build with debug tools only.
    #[must_use]
    pub const fn new(game_version: &'static str) -> Self {
        Self {
            game_version,
            keep: if cfg!(target_arch = "wasm32") { 1 } else { 5 },
            record_in_release: false,
        }
    }

    /// No reports: no hook is installed.
    #[must_use]
    pub const fn off() -> Self {
        Self {
            game_version: "",
            keep: 0,
            record_in_release: false,
        }
    }

    /// The same reports, with the game's record of its play in a build
    /// without debug tools too, and `SHUYI_PANIC_AT_FRAME` read there. Off
    /// until said: together they write the game's state on demand, which a
    /// shipped game gives anyone who runs it.
    #[must_use]
    pub const fn with_record_in_release(mut self) -> Self {
        self.record_in_release = true;
        self
    }

    /// Whether this build adds the record of play and reads
    /// `SHUYI_PANIC_AT_FRAME`: with debug tools, or when the game said
    /// [`with_record_in_release`](Self::with_record_in_release).
    #[must_use]
    pub const fn records_play(&self) -> bool {
        shuyi_ui::DEBUG_TOOLS || self.record_in_release
    }
}

impl Default for CrashReports {
    fn default() -> Self {
        Self::new("")
    }
}

/// The message of the panic `SHUYI_PANIC_AT_FRAME` asks for.
pub const PANIC_ON_PURPOSE: &str = "a panic on purpose (SHUYI_PANIC_AT_FRAME)";

/// A new storage for each report.
type StorageMaker = Arc<dyn Fn() -> Box<dyn Storage> + Send + Sync>;

/// What [`install`] was given last.
struct Installed {
    reports: CrashReports,
    storage: StorageMaker,
    /// The thread [`install`] ran on: the one that runs the game.
    game_thread: ThreadId,
}

/// The report the hook wrote last, and its key, for [`add_record`].
struct Written {
    key: String,
    report: CrashReport,
}

/// Whether the hook is installed: once a process.
static HOOKED: AtomicBool = AtomicBool::new(false);
/// What [`install`] was given last.
static INSTALLED: Mutex<Option<Installed>> = Mutex::new(None);
/// The frame about to run ([`set_frame`]).
static FRAME: AtomicU32 = AtomicU32::new(0);
/// The screen on top as it starts ([`set_frame`]).
static TOP_SCREEN: Mutex<Option<&'static str>> = Mutex::new(None);
/// The last lines of the log ([`note_log`]).
static LOG: Mutex<VecDeque<String>> = Mutex::new(VecDeque::new());
/// The report the hook wrote last.
static WRITTEN: Mutex<Option<Written>> = Mutex::new(None);

thread_local! {
    /// Whether this thread's panics are caught on purpose ([`quiet`]).
    static QUIET: Cell<bool> = const { Cell::new(false) };
}

/// `mutex`'s guard if no one holds it, poisoned or not: a hook never waits.
fn try_lock<T>(mutex: &Mutex<T>) -> Option<MutexGuard<'_, T>> {
    match mutex.try_lock() {
        Ok(guard) => Some(guard),
        Err(TryLockError::Poisoned(poisoned)) => Some(poisoned.into_inner()),
        Err(TryLockError::WouldBlock) => None,
    }
}

/// `mutex`'s guard, poisoned or not.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Writes reports of every panic from now on, as `reports` says, each
/// through a new storage from `storage`, then calls the hook that was there
/// before. The thread it is called on is the game's: a panic there is
/// [`fatal`](CrashReport::fatal), and one on any other thread isn't.
/// Installs nothing if `reports` keeps none. Called again, it changes what
/// the hook does, and installs no second hook.
pub fn install(
    reports: CrashReports,
    storage: impl Fn() -> Box<dyn Storage> + Send + Sync + 'static,
) {
    if reports.keep == 0 {
        *lock(&INSTALLED) = None;
        return;
    }
    *lock(&INSTALLED) = Some(Installed {
        reports,
        storage: Arc::new(storage),
        game_thread: std::thread::current().id(),
    });
    if HOOKED.swap(true, Ordering::SeqCst) {
        return;
    }
    let before = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if !QUIET.with(Cell::get) {
            report(info);
        }
        before(info);
    }));
}

/// The frame about to run (`App::frame_number() + 1`) and the screen on
/// top as it starts, for a report.
pub fn set_frame(frame: u32, top_screen: Option<&'static str>) {
    FRAME.store(frame, Ordering::Relaxed);
    if let Some(mut top) = try_lock(&TOP_SCREEN) {
        *top = top_screen;
    }
}

/// Keeps `line` of the log among the last [`LOG_LINES`], for a report.
pub fn note_log(line: String) {
    let mut log = lock(&LOG);
    if log.len() == LOG_LINES {
        log.pop_front();
    }
    log.push_back(line);
}

/// Runs `f` with reports off on this thread: a panic it catches on purpose
/// is not a crash.
pub fn quiet<R>(f: impl FnOnce() -> R) -> R {
    /// Puts the flag back, even if `f` unwinds.
    struct Restore(bool);
    impl Drop for Restore {
        fn drop(&mut self) {
            QUIET.with(|quiet| quiet.set(self.0));
        }
    }
    let _restore = Restore(QUIET.with(|quiet| quiet.replace(true)));
    f()
}

/// Adds the game's record of its play to the report the hook wrote last,
/// and writes it again under the same key. A record over
/// `shuyi_platform::crash::RECORD_BYTES` is left out, and the report's log
/// says so. Does nothing if no report was written.
pub fn add_record(record: Option<String>) {
    let Some(Written { key, mut report }) = lock(&WRITTEN).take() else {
        return;
    };
    let Some(storage) = lock(&INSTALLED).as_ref().map(|i| Arc::clone(&i.storage)) else {
        return;
    };
    report.set_record(record);
    let _ = storage().write(&key, &report.to_text());
}

/// The frame `SHUYI_PANIC_AT_FRAME` names, on native when `reports`
/// [`records_play`](CrashReports::records_play); never on the web.
pub(crate) fn panic_at_frame(reports: CrashReports) -> Option<u32> {
    if !reports.records_play() {
        return None;
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::env::var("SHUYI_PANIC_AT_FRAME")
            .ok()?
            .trim()
            .parse()
            .ok()
    }
    #[cfg(target_arch = "wasm32")]
    {
        None
    }
}

/// Milliseconds since 1970 on the system's clock.
fn now_ms() -> u64 {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let since = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH);
        since.map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
    }
    #[cfg(target_arch = "wasm32")]
    {
        shuyi_platform::Millis::from_secs_f64(macroquad::miniquad::date::now()).0
    }
}

/// The panic's message: its text, if it has one.
fn message(info: &PanicHookInfo<'_>) -> String {
    let payload = info.payload();
    if let Some(text) = payload.downcast_ref::<&str>() {
        (*text).to_owned()
    } else if let Some(text) = payload.downcast_ref::<String>() {
        text.clone()
    } else {
        "a panic with no text".to_owned()
    }
}

/// The report of the panic `info` describes, as of now, on the game's
/// thread or not.
fn build(info: &PanicHookInfo<'_>, reports: CrashReports, on_game_thread: bool) -> CrashReport {
    let mut report = CrashReport::new(now_ms(), message(info));
    report.fatal = on_game_thread;
    report.location = info
        .location()
        .map(|at| format!("{}:{}:{}", at.file(), at.line(), at.column()));
    report.thread = std::thread::current().name().map(ToOwned::to_owned);
    report.backtrace = std::backtrace::Backtrace::force_capture().to_string();
    reports.game_version.clone_into(&mut report.game);
    let build = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    let (os, arch) = (std::env::consts::OS, std::env::consts::ARCH);
    report.target = format!("{os} {arch} {build}");
    report.frame = FRAME.load(Ordering::Relaxed);
    report.top_screen = try_lock(&TOP_SCREEN)
        .and_then(|top| *top)
        .map(ToOwned::to_owned);
    report.log = try_lock(&LOG)
        .map(|log| log.iter().cloned().collect())
        .unwrap_or_default();
    report.limit();
    report
}

/// Writes the report of `info` through a new storage, at the first free
/// `crash-<when>` from its time, and keeps the newest. A write that fails
/// is dropped.
fn report(info: &PanicHookInfo<'_>) {
    let Some((reports, storage, game_thread)) = try_lock(&INSTALLED).and_then(|i| {
        i.as_ref()
            .map(|i| (i.reports, Arc::clone(&i.storage), i.game_thread))
    }) else {
        return;
    };
    let on_game_thread = std::thread::current().id() == game_thread;
    let mut report = build(info, reports, on_game_thread);
    let mut storage = storage();
    while matches!(storage.read(&crash_key(report.when)), Ok(Some(_))) {
        report.when = report.when.saturating_add(1);
    }
    let key = crash_key(report.when);
    if storage.write(&key, &report.to_text()).is_err() {
        return;
    }
    let keep = usize::try_from(reports.keep).unwrap_or(usize::MAX);
    let _ = keep_newest(storage.as_mut(), keep);
    if let Some(mut written) = try_lock(&WRITTEN) {
        *written = Some(Written { key, report });
    }
}
