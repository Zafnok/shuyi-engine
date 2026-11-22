//! The log's sink installed for real (ticket 3041). One `#[test]`: the
//! logger and the panic hook are global to a process, and this binary is
//! a process of its own.
#![cfg(not(target_arch = "wasm32"))]

use std::sync::atomic::{AtomicBool, Ordering};

use shuyi_app::log_sink::{self, LATEST};
use shuyi_ui::log::{LogCategory, LogFilter, LogLevel};

/// Whether the hook before the sink's ran.
static BEFORE: AtomicBool = AtomicBool::new(false);

/// The text of `dir`'s `latest.log`.
fn latest(dir: &std::path::Path) -> String {
    let path = dir.join(LATEST);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

#[test]
fn the_first_sink_installed_keeps_its_lines_and_a_panics_and_chains_the_hook() {
    std::panic::set_hook(Box::new(|_| BEFORE.store(true, Ordering::SeqCst)));
    let (a, b) = (tempfile::tempdir(), tempfile::tempdir());
    let (a, b) = (
        a.unwrap_or_else(|e| panic!("{e}")),
        b.unwrap_or_else(|e| panic!("{e}")),
    );
    let (logs_a, logs_b) = (a.path().join("logs"), b.path().join("logs"));

    let first = log_sink::install("game-a", "1.0", Some(logs_a.clone()), None);
    assert_eq!(first, LogFilter::default());
    // A second install keeps the first logger: nothing goes to B. It still
    // says the filter its text gives.
    let second = log_sink::install(
        "game-b",
        "2.0",
        Some(logs_b.clone()),
        Some("warn,audio=debug,gilrs=info"),
    );
    assert_eq!(
        second,
        LogFilter::new(LogLevel::Warn)
            .with(LogCategory::Audio, LogLevel::Debug)
            .with_prefix("gilrs", LogLevel::Info)
    );

    log_sink::set_frame(3);
    log::warn!(target: "storage", "settings: not ron");
    log::debug!(target: "gilrs", "not let through");
    let line = line!() + 3;
    let worker = std::thread::Builder::new()
        .name("worker".to_owned())
        .spawn(|| panic!("a known message"));
    let worker = worker.unwrap_or_else(|e| panic!("{e}"));
    assert!(worker.join().is_err(), "it panicked");
    assert!(BEFORE.load(Ordering::SeqCst), "the earlier hook ran");

    let text = latest(&logs_a);
    let lines: Vec<&str> = text.lines().collect();
    assert!(
        lines[0].contains("storage game-a, game version 1.0"),
        "{text}"
    );
    assert!(
        lines
            .iter()
            .any(|l| l.ends_with(" f3 WARN  storage: settings: not ron")),
        "{text}"
    );
    assert!(!text.contains("not let through"), "{text}");
    let panic = format!(
        " f3 ERROR panic: a known message at {}:{line} on the thread worker",
        file!()
    );
    assert!(lines.iter().any(|l| l.ends_with(&panic)), "{panic}\n{text}");
    assert!(!logs_b.exists(), "folder B has no log");
}
