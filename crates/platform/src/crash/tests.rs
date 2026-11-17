//! Reports' keys and order, their text both ways, the reader's limits and
//! what the game is offered.

use proptest::prelude::*;
use shuyi_ui::storage::MemoryStorage;

use super::*;

/// A report with every field set.
fn full() -> CrashReport {
    let mut report = CrashReport::new(1_700_000_000_123, "index out of bounds");
    report.location = Some("src/stage.rs:12:5".into());
    report.thread = Some("main".into());
    report.backtrace = "   0: <unknown>\n".into();
    report.frames = vec![CodeFrame::new(Some(0x1a2b)), CodeFrame::new(None)];
    report.game = "0.3.1".into();
    report.target = "linux x86_64 release".into();
    report.frame = 42;
    report.top_screen = Some("stage".into());
    report.log = vec!["f41 info game: moved".into()];
    report.record = Some("(seed: 7, commands: [])".into());
    report.fatal = false;
    report
}

/// `storage` with an empty report at each of `whens`.
fn with_reports(whens: &[u64]) -> MemoryStorage {
    let mut storage = MemoryStorage::new();
    for &when in whens {
        let text = CrashReport::new(when, format!("panic {when}")).to_text();
        storage.write(&crash_key(when), &text).unwrap();
    }
    storage
}

#[test]
fn a_reports_key_is_crash_and_its_time() {
    assert_eq!(crash_key(17), "crash-17");
    assert!(shuyi_ui::storage::is_valid_key(&crash_key(u64::MAX)));
    assert_eq!(report_when("crash-17"), Some(17));
    for other in [
        OFFERED_KEY,
        "crash-",
        "crash-1a",
        "crash--1",
        "crash-+1",
        "settings",
        "crash-99999999999999999999999",
    ] {
        assert_eq!(report_when(other), None, "{other}");
    }
}

#[test]
fn the_keys_are_the_reports_newest_first_by_their_time() {
    let mut storage = with_reports(&[9, 100, 10]);
    storage.write("settings", "()").unwrap();
    storage.write(OFFERED_KEY, "\"crash-9\"").unwrap();
    storage.write("crash-x", "()").unwrap();
    // 100 after 9 by its time, not its text.
    assert_eq!(report_keys(&storage), ["crash-100", "crash-10", "crash-9"]);
}

#[test]
fn keep_newest_deletes_the_older_reports_and_nothing_else() {
    let mut storage = with_reports(&[1, 2, 3, 4, 5, 6]);
    storage.write("settings", "()").unwrap();
    keep_newest(&mut storage, 5).unwrap();
    assert_eq!(report_keys(&storage).len(), 5);
    assert_eq!(storage.read("crash-1").unwrap(), None);
    keep_newest(&mut storage, 1).unwrap();
    assert_eq!(report_keys(&storage), ["crash-6"]);
    keep_newest(&mut storage, 0).unwrap();
    assert_eq!(report_keys(&storage), Vec::<String>::new());
    assert_eq!(storage.read("settings").unwrap().as_deref(), Some("()"));
}

#[test]
fn a_report_reads_back_as_written() {
    let report = full();
    let text = report.to_text();
    assert!(!text.contains('\r'));
    assert_eq!(CrashReport::parse(&text), Ok(report));
}

#[test]
fn an_older_report_without_the_optional_fields_still_reads() {
    let report = CrashReport::parse("(version: 1, message: \"boom\")").unwrap();
    assert_eq!((report.version, report.message.as_str()), (1, "boom"));
    assert_eq!((report.when, report.frame), (0, 0));
    assert!(report.location.is_none() && report.record.is_none() && report.log.is_empty());
    assert_eq!(report.frames, Vec::<CodeFrame>::new());
    assert!(CrashReport::parse("(version: 1)").is_err());
}

#[test]
fn an_older_report_without_fatal_reads_as_fatal() {
    let report = CrashReport::parse("(version: 1, when: 3, message: \"boom\")").unwrap();
    assert!(report.fatal);
    let mut survived = CrashReport::new(3, "boom");
    survived.fatal = false;
    assert!(survived.to_text().contains("fatal: false,"));
    assert!(!CrashReport::parse(&survived.to_text()).unwrap().fatal);
}

#[test]
fn a_report_that_is_not_fatal_is_kept_but_not_offered() {
    let mut storage = with_reports(&[5]);
    let mut survived = CrashReport::new(9, "Can't open PCM device");
    survived.fatal = false;
    storage.write(&crash_key(9), &survived.to_text()).unwrap();
    let last = offer_last_crash(&mut storage).unwrap();
    assert_eq!((last.when, last.key.as_str()), (5, "crash-5"));
    assert_eq!(offer_last_crash(&mut storage), None);
    assert_eq!(report_keys(&storage), ["crash-9", "crash-5"]);
    // Only reports the game survived: none offered, and none again.
    survived.when = 10;
    storage.write(&crash_key(10), &survived.to_text()).unwrap();
    assert_eq!(offer_last_crash(&mut storage), None);
    let marked = storage.read(OFFERED_KEY).unwrap();
    assert_eq!(marked.as_deref(), Some("\"crash-10\""));
}

#[test]
fn an_offered_report_is_not_offered_again_behind_a_newer_one() {
    let mut storage = with_reports(&[5]);
    assert_eq!(offer_last_crash(&mut storage).map(|l| l.when), Some(5));
    let mut survived = CrashReport::new(9, "a thread");
    survived.fatal = false;
    storage.write(&crash_key(9), &survived.to_text()).unwrap();
    assert_eq!(offer_last_crash(&mut storage), None);
}

#[test]
fn the_limits_are_their_sizes() {
    assert_eq!(MESSAGE_BYTES, 4_096);
    assert_eq!(BACKTRACE_BYTES, 32_768);
    assert_eq!((LOG_LINES, LOG_LINE_BYTES), (200, 1_024));
    assert_eq!(RECORD_BYTES, 1_048_576);
    assert_eq!(TEXT_BYTES, 2_097_152);
}

#[test]
fn text_over_the_limit_is_not_read() {
    let mut report = CrashReport::new(1, "boom");
    report.backtrace = "x".repeat(TEXT_BYTES);
    let error = CrashReport::parse(&report.to_text()).unwrap_err();
    assert!(error.contains("over"), "{error}");
}

#[test]
fn text_of_the_limit_exactly_is_read() {
    let report = full();
    let mut text = report.to_text();
    text.push_str(&" ".repeat(TEXT_BYTES - text.len()));
    assert_eq!(text.len(), TEXT_BYTES);
    assert_eq!(CrashReport::parse(&text), Ok(report));
}

#[test]
fn every_cut_of_a_report_reads_or_errs() {
    let text = full().to_text();
    for end in (0..=text.len()).filter(|&end| text.is_char_boundary(end)) {
        let _ = CrashReport::parse(&text[..end]);
    }
}

#[test]
fn the_message_backtrace_and_log_are_cut_to_their_limits() {
    // The limit falls inside an `é`: the cut is at the character before.
    let mut report = CrashReport::new(1, format!("a{}", "é".repeat(MESSAGE_BYTES)));
    assert_eq!(report.message.len(), MESSAGE_BYTES - 1);
    assert!(report.message.ends_with('é'));
    let whole = CrashReport::new(1, "m".repeat(MESSAGE_BYTES));
    assert_eq!(whole.message.len(), MESSAGE_BYTES);
    report.backtrace = "b".repeat(BACKTRACE_BYTES + 5);
    report.log = (0..LOG_LINES + 3).map(|i| format!("{i}")).collect();
    report.log.push("l".repeat(LOG_LINE_BYTES * 2));
    report.frames = (0..FRAMES as u64 + 2)
        .map(|i| CodeFrame::new(Some(i)))
        .collect();
    report.limit();
    assert_eq!(report.frames.len(), FRAMES);
    assert_eq!(report.frames[0], CodeFrame::new(Some(0)));
    assert_eq!(report.backtrace.len(), BACKTRACE_BYTES);
    assert_eq!(report.log.len(), LOG_LINES);
    assert_eq!(report.log[0], "4");
    assert_eq!(report.log.last().map(String::len), Some(LOG_LINE_BYTES));
}

#[test]
fn a_record_over_the_limit_is_left_out_and_the_log_says_why() {
    let mut report = CrashReport::new(1, "boom");
    report.set_record(Some("r".repeat(RECORD_BYTES)));
    assert_eq!(report.record.as_ref().map(String::len), Some(RECORD_BYTES));
    report.set_record(Some("r".repeat(RECORD_BYTES + 1)));
    assert_eq!(report.record, None);
    let why = report.log.last().unwrap();
    assert!(why.contains("record was left out"), "{why}");
}

#[test]
fn the_newest_report_is_offered_once() {
    let mut storage = with_reports(&[5, 7]);
    let last = offer_last_crash(&mut storage).unwrap();
    assert_eq!((last.when, last.message.as_str()), (7, "panic 7"));
    assert_eq!(last.key, "crash-7");
    assert_eq!(offer_last_crash(&mut storage), None);
    // A newer one is offered in its turn.
    let text = CrashReport::new(8, "panic 8").to_text();
    storage.write("crash-8", &text).unwrap();
    assert_eq!(offer_last_crash(&mut storage).map(|l| l.when), Some(8));
    assert_eq!(offer_last_crash(&mut storage), None);
}

#[test]
fn no_report_offers_nothing_and_writes_nothing() {
    let mut storage = MemoryStorage::new();
    assert_eq!(offer_last_crash(&mut storage), None);
    assert_eq!(storage.list().unwrap(), Vec::<String>::new());
}

#[test]
fn a_report_that_does_not_read_is_marked_offered() {
    let mut storage = MemoryStorage::new();
    storage.write("crash-3", "not a report").unwrap();
    assert_eq!(offer_last_crash(&mut storage), None);
    let marked = storage.read(OFFERED_KEY).unwrap();
    assert_eq!(marked.as_deref(), Some("\"crash-3\""));
}

proptest! {
    #[test]
    fn parse_never_panics(text in ".{0,300}") {
        let _ = CrashReport::parse(&text);
    }

    #[test]
    fn a_report_written_and_read_back_is_equal(
        when: u64,
        message in ".{0,40}",
        location in proptest::option::of(".{0,20}"),
        frame: u32,
        log in proptest::collection::vec(".{0,20}", 0..4),
        record in proptest::option::of(".{0,40}"),
    ) {
        let mut report = CrashReport::new(when, message);
        report.location = location;
        report.frame = frame;
        report.log = log;
        report.record = record;
        prop_assert_eq!(CrashReport::parse(&report.to_text()), Ok(report));
    }
}

#[test]
fn a_frames_offset_is_written_as_a_number_and_another_modules_as_none() {
    let text = full().to_text();
    assert!(text.contains("offset: Some(6699)"), "{text}");
    assert!(text.contains("offset: None"), "{text}");
}
