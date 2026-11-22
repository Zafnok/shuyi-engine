//! The line, the levels, the rotation, the cap, the first line and the
//! web's console calls, on a temporary folder: no logger is installed (it
//! is global; `tests/log_sink.rs` installs one in a process of its own).

use super::*;

/// The lines of `dir`'s file called `name`.
fn lines_of(dir: &Path, name: &str) -> Vec<String> {
    let text = fs::read_to_string(dir.join(name)).unwrap_or_else(|e| panic!("{name}: {e}"));
    text.lines().map(ToOwned::to_owned).collect()
}

/// The files in `dir`, sorted.
fn files_in(dir: &Path) -> Vec<String> {
    let entries = fs::read_dir(dir).unwrap_or_else(|e| panic!("{e}"));
    let mut names: Vec<String> = entries
        .map(|e| {
            e.unwrap_or_else(|e| panic!("{e}"))
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    names
}

/// A temporary folder.
fn temp() -> tempfile::TempDir {
    tempfile::tempdir().unwrap_or_else(|e| panic!("{e}"))
}

/// Opens a file in `dir` whose first line is `run`.
fn start(dir: &Path, run: &str) -> LogFile {
    LogFile::open(dir, run).unwrap_or_else(|e| panic!("{e}"))
}

#[test]
fn a_line_names_its_time_frame_level_and_target() {
    set_frame(4);
    let line = format_line(
        12.5,
        frame(),
        LogLevel::Warn,
        "storage",
        "settings: not ron",
    );
    assert_eq!(line, "   12.500 f4 WARN  storage: settings: not ron");
    set_frame(0);
    assert_eq!(
        format_line(0.0, 0, LogLevel::Error, "panic", "boom"),
        "    0.000 f0 ERROR panic: boom"
    );
    assert_eq!(
        format_line(1.0, 9, LogLevel::Debug, "gilrs", "x"),
        "    1.000 f9 DEBUG gilrs: x"
    );
}

#[test]
fn levels_map_both_ways_and_trace_is_debug() {
    for level in LogLevel::ALL {
        assert_eq!(level_of(log_level(level)), level);
    }
    assert_eq!(level_of(log::Level::Trace), LogLevel::Debug);
    assert_eq!(log_filter(LogLevel::Warn), log::LevelFilter::Warn);
}

#[test]
fn the_sink_lets_through_what_its_filter_allows_per_target() {
    use log::Log;
    let sink = Sink {
        filter: LogFilter::default(),
        stderr: false,
        start: 0.0,
        file: Mutex::new(None),
    };
    let at = |target: &str, level: log::Level| {
        sink.enabled(&log::Metadata::builder().target(target).level(level).build())
    };
    assert!(!at("gilrs", log::Level::Debug));
    assert!(at("gilrs", log::Level::Warn));
    assert_eq!(at("audio", log::Level::Info), shuyi_ui::DEBUG_TOOLS);
    assert!(!at("audio", log::Level::Debug));
    assert!(!at("audio", log::Level::Trace));
    assert!(at(PANIC_TARGET, log::Level::Error));
}

#[test]
fn a_build_without_debug_tools_keeps_warnings_and_errors_only() {
    assert!(kept_in_this_build(LogLevel::Error));
    assert!(kept_in_this_build(LogLevel::Warn));
    assert_eq!(kept_in_this_build(LogLevel::Info), shuyi_ui::DEBUG_TOOLS);
    assert_eq!(kept_in_this_build(LogLevel::Debug), shuyi_ui::DEBUG_TOOLS);
}

#[test]
fn each_start_keeps_the_last_three_runs_oldest_last() {
    let dir = temp();
    let logs = dir.path().join("logs");
    for (earlier, files) in [
        (0, vec!["latest.log"]),
        (1, vec!["latest.log", "previous-1.log"]),
        (2, vec!["latest.log", "previous-1.log", "previous-2.log"]),
        (
            3,
            vec![
                "latest.log",
                "previous-1.log",
                "previous-2.log",
                "previous-3.log",
            ],
        ),
        (
            4,
            vec![
                "latest.log",
                "previous-1.log",
                "previous-2.log",
                "previous-3.log",
            ],
        ),
    ] {
        drop(start(&logs, &format!("run {earlier}")));
        assert_eq!(files_in(&logs), files, "after {earlier} earlier runs");
    }
    // Run 4 is this one; runs 1 to 3 the earlier ones, oldest last.
    assert_eq!(lines_of(&logs, "latest.log"), ["run 4"]);
    assert_eq!(lines_of(&logs, "previous-1.log"), ["run 3"]);
    assert_eq!(lines_of(&logs, "previous-2.log"), ["run 2"]);
    assert_eq!(lines_of(&logs, "previous-3.log"), ["run 1"]);
}

#[test]
fn past_the_cap_only_errors_go_in_up_to_their_room() {
    let dir = temp();
    // Lines of 9 bytes and their ends: 10 bytes each.
    let open = |cap, room| {
        LogFile::open_capped(dir.path(), "first....", cap, room).unwrap_or_else(|e| panic!("{e}"))
    };
    let mut file = open(30, 25);
    file.write_line(1.0, LogLevel::Info, "info.....");
    file.write_line(1.0, LogLevel::Warn, "warn.....");
    // 30 bytes: the cap. Past it, the note, then errors alone, 25 bytes.
    file.write_line(2.0, LogLevel::Warn, "warn-cut.");
    file.write_line(2.0, LogLevel::Error, "error-1..");
    file.write_line(2.0, LogLevel::Warn, "warn-cut.");
    file.write_line(2.0, LogLevel::Info, "info-cut.");
    file.write_line(2.0, LogLevel::Error, "error-2..");
    file.write_line(2.0, LogLevel::Error, "error-3..");
    file.write_line(2.0, LogLevel::Error, "e");
    file.write_line(2.0, LogLevel::Error, "e22");
    drop(file);
    let lines = lines_of(dir.path(), LATEST);
    assert_eq!(
        lines[..3],
        ["first....", "info.....", "warn....."],
        "up to the cap, any line"
    );
    // Its frame is whatever another test set last.
    assert!(lines[3].starts_with("    2.000 f"), "{}", lines[3]);
    assert!(
        lines[3].ends_with(" WARN  platform: the log was cut here: errors alone from now on"),
        "{}",
        lines[3]
    );
    assert_eq!(
        lines[4..],
        ["error-1..", "error-2..", "e"],
        "then errors alone, as they fit the room"
    );
}

#[test]
fn the_real_caps_are_two_mib_and_a_quarter_more() {
    assert_eq!(LOG_FILE_CAP, 2 * 1024 * 1024);
    assert_eq!(LOG_FILE_ERROR_ROOM, 256 * 1024);
}

#[test]
fn the_first_line_names_the_engine_the_system_the_build_and_the_game() {
    let line = first_line("a-game", "1.2.3");
    assert!(
        line.starts_with("    0.000 f0 INFO  platform: Shuyi Engine "),
        "{line}"
    );
    assert!(line.contains(env!("CARGO_PKG_VERSION")), "{line}");
    assert!(line.contains(std::env::consts::OS), "{line}");
    assert!(line.contains(std::env::consts::ARCH), "{line}");
    let build = if cfg!(debug_assertions) {
        " debug,"
    } else {
        " release,"
    };
    assert!(line.contains(build), "{line}");
    assert!(
        line.contains("storage a-game, game version 1.2.3"),
        "{line}"
    );
    assert!(first_line("a-game", "").ends_with("game version unknown"));

    let dir = temp();
    drop(start(dir.path(), &line));
    assert_eq!(lines_of(dir.path(), LATEST), [line]);
}

#[test]
fn a_folder_that_cant_be_made_is_an_error() {
    let dir = temp();
    let blocked = dir.path().join("a-file");
    fs::write(&blocked, "not a folder").unwrap_or_else(|e| panic!("{e}"));
    assert!(LogFile::open(&blocked.join("logs"), "x").is_err());
}

#[test]
fn each_level_has_its_console_call_and_a_nul_is_replaced() {
    assert_eq!(console_of(LogLevel::Error), Console::Error);
    assert_eq!(console_of(LogLevel::Warn), Console::Warn);
    assert_eq!(console_of(LogLevel::Info), Console::Info);
    assert_eq!(console_of(LogLevel::Debug), Console::Debug);
    assert_eq!(console_text("a\0b\0"), "a\u{FFFD}b\u{FFFD}");
    assert!(!console_text("a\0b").contains('\0'));
    assert!(matches!(
        console_text("plain"),
        std::borrow::Cow::Borrowed("plain")
    ));
}
