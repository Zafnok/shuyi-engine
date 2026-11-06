//! The crash hook over a storage the test shares with it. One `#[test]`:
//! a panic hook is global to its process, and this binary is a process of
//! its own.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use shuyi_app::crash::{self, CrashReports};
use shuyi_platform::crash::{CrashReport, LOG_LINES, RECORD_BYTES, report_keys};
use shuyi_ui::storage::{MemoryStorage, Storage, StorageError};

/// A storage the test and the hook share.
#[derive(Debug, Clone, Default)]
struct Shared(Arc<Mutex<MemoryStorage>>);

impl Shared {
    fn store(&self) -> std::sync::MutexGuard<'_, MemoryStorage> {
        self.0.lock().unwrap_or_else(|e| panic!("{e}"))
    }

    /// The reports' keys, newest first.
    fn keys(&self) -> Vec<String> {
        report_keys(&*self.store())
    }

    /// The newest report.
    fn newest(&self) -> CrashReport {
        let key = self.keys().into_iter().next();
        let key = key.unwrap_or_else(|| panic!("no report"));
        let text = self.store().read(&key).ok().flatten();
        let text = text.unwrap_or_else(|| panic!("{key}: nothing stored"));
        CrashReport::parse(&text).unwrap_or_else(|e| panic!("{key}: {e}"))
    }
}

impl Storage for Shared {
    fn read(&self, key: &str) -> Result<Option<String>, StorageError> {
        self.store().read(key)
    }
    fn write(&mut self, key: &str, value: &str) -> Result<(), StorageError> {
        self.store().write(key, value)
    }
    fn delete(&mut self, key: &str) -> Result<(), StorageError> {
        self.store().delete(key)
    }
    fn list(&self) -> Result<Vec<String>, StorageError> {
        self.store().list()
    }
}

/// How often the hook before the crash hook was called.
static BEFORE: AtomicU32 = AtomicU32::new(0);

/// Panics with `message`, caught.
fn panic_with(message: &str) {
    let message = message.to_owned();
    let caught = catch_unwind(AssertUnwindSafe(|| panic!("{message}")));
    assert!(caught.is_err());
}

#[test]
fn the_hook_writes_reports_and_calls_the_one_before() {
    std::panic::set_hook(Box::new(|_| {
        BEFORE.fetch_add(1, Ordering::SeqCst);
    }));
    let storage = Shared::default();
    let shared = storage.clone();
    crash::install(CrashReports::new("1.2.3"), move || Box::new(shared.clone()));
    crash::set_frame(7, Some("stage"));
    for i in 0..=LOG_LINES {
        crash::note_log(format!("f{i} info game: line {i}"));
    }

    // One panic: one report, of what was set.
    panic_with("boom");
    assert_eq!(BEFORE.load(Ordering::SeqCst), 1);
    assert_eq!(storage.keys().len(), 1);
    let report = storage.newest();
    assert_eq!(report.message, "boom");
    let location = report.location.clone().unwrap();
    assert!(location.contains("crash_hook.rs:"), "{location}");
    assert_eq!(report.thread.as_deref(), std::thread::current().name());
    assert_eq!(
        (report.frame, report.top_screen.as_deref()),
        (7, Some("stage"))
    );
    assert_eq!(report.game, "1.2.3");
    assert_eq!(report.engine, shuyi_platform::crash::ENGINE_VERSION);
    assert_eq!(report.log.len(), LOG_LINES);
    assert_eq!(
        report.log.first().map(String::as_str),
        Some("f1 info game: line 1")
    );
    assert_eq!(report.record, None);
    // On the thread that installed the hook, `run`'s: it ended the game.
    assert!(report.fatal);

    // The record goes into the same report, still fatal.
    crash::add_record(Some("(seed: 1)".into()));
    assert_eq!(storage.keys().len(), 1);
    let report = storage.newest();
    assert_eq!(report.record.as_deref(), Some("(seed: 1)"));
    assert!(report.fatal);

    // A panic on another thread, which the game survives, is kept and
    // isn't fatal; the game is offered the one before it.
    let worker = std::thread::Builder::new()
        .name("sound".into())
        .spawn(|| panic!("Can't open PCM device"));
    let joined = worker.unwrap_or_else(|e| panic!("{e}")).join();
    assert!(joined.is_err());
    assert_eq!(storage.keys().len(), 2);
    let report = storage.newest();
    assert_eq!(report.message, "Can't open PCM device");
    assert_eq!(report.thread.as_deref(), Some("sound"));
    assert!(!report.fatal);
    let offered = shuyi_platform::crash::offer_last_crash(&mut storage.clone());
    assert_eq!(offered.map(|last| last.message), Some("boom".to_owned()));

    // One too long is left out, and the log says why.
    panic_with("again");
    crash::add_record(Some("r".repeat(RECORD_BYTES + 1)));
    let report = storage.newest();
    assert!(report.fatal);
    assert_eq!((report.message.as_str(), report.record), ("again", None));
    let why = report.log.last().unwrap();
    assert!(why.contains("record was left out"), "{why}");

    // A panic caught on purpose writes nothing, and the hook before still
    // hears it.
    let before = BEFORE.load(Ordering::SeqCst);
    crash::quiet(|| panic_with("on purpose"));
    assert_eq!(storage.keys().len(), 3);
    assert_eq!(BEFORE.load(Ordering::SeqCst), before + 1);

    // Panics in one millisecond each get a key of their own; five are
    // kept, the newest.
    panic_with("burst 0");
    assert_eq!(storage.keys().len(), 4);
    for i in 1..6 {
        panic_with(&format!("burst {i}"));
    }
    assert_eq!(storage.keys().len(), 5);
    assert_eq!(storage.newest().message, "burst 5");

    // With `keep: 1`, one; installing again adds no second hook.
    let mut one = CrashReports::new("1.2.3");
    one.keep = 1;
    let shared = storage.clone();
    crash::install(one, move || Box::new(shared.clone()));
    let before = BEFORE.load(Ordering::SeqCst);
    panic_with("last");
    assert_eq!(BEFORE.load(Ordering::SeqCst), before + 1);
    assert_eq!(storage.keys().len(), 1);
    assert_eq!(storage.newest().message, "last");

    // Off: nothing more is written.
    crash::install(CrashReports::off(), || Box::new(MemoryStorage::new()));
    panic_with("unreported");
    assert_eq!(storage.newest().message, "last");
    crash::add_record(Some("late".into()));
    assert_eq!(storage.newest().record, None);
}
