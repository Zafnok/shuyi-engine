//! The report's numbers and lines.

use core::cell::Cell;

use alloc::vec::Vec;

use super::*;

#[test]
fn numbers_are_grouped_by_thousands() {
    assert_eq!(grouped(0), "0");
    assert_eq!(grouped(999), "999");
    assert_eq!(grouped(1_000), "1,000");
    assert_eq!(grouped(65_536), "65,536");
    assert_eq!(grouped(1_234_567), "1,234,567");
    assert_eq!(ms_tenths(12_449), "12.4");
    assert_eq!(ms_tenths(12_450), "12.5");
    assert_eq!(ms_tenths(1_234_000), "1,234.0");
    assert_eq!(ms(1_234_499), "1,234");
}

#[test]
fn a_timing_keeps_the_fewest_the_most_and_the_average() {
    let mut t = Timing::new();
    assert_eq!(t.line("Draw"), "Draw: none");
    assert_eq!((t.count(), t.avg_us()), (0, 0));
    t.add(2_000);
    assert_eq!(
        t.line("Draw"),
        "Draw: min 2.0 / avg 2.0 / max 2.0 ms, 1 times"
    );
    t.add(4_000);
    t.add(500);
    assert_eq!(t.count(), 3);
    assert_eq!((t.min_us(), t.max_us(), t.avg_us()), (500, 4_000, 2_166));
    assert_eq!(
        t.line("Draw"),
        "Draw: min 0.5 / avg 2.2 / max 4.0 ms, 3 times"
    );
}

#[test]
fn the_report_is_built_from_its_defaults() {
    let report = RunReport::new();
    assert_eq!(report, RunReport::default());
    assert_eq!(
        (report.every_seconds, report.slowest, report.phases),
        (60, 10, false)
    );
    assert_eq!(report.file, None);
    let report = report
        .file("run.txt")
        .every_seconds(0)
        .slowest(3)
        .phases(true);
    assert_eq!(report.file.as_deref(), Some("run.txt"));
    assert_eq!(
        (report.every_seconds, report.slowest, report.phases),
        (1, 3, true)
    );
}

std::thread_local! {
    /// What [`at`] reads next.
    static NOW: Cell<u64> = const { Cell::new(0) };
}

/// A timer the test sets.
fn at() -> u64 {
    NOW.get()
}

/// Notes a read of `path` at `ms` milliseconds.
fn read(reads: &mut Reads, path: &str, found: bool, ms: u64) {
    NOW.set(ms * 1000);
    reads.add(path, found);
}

/// A file's time runs to the next read, the last one's to the end; a file
/// read twice sums both; a step's time is its own.
#[test]
fn each_file_takes_the_time_to_the_next_read() {
    let mut reads = Reads::new(at);
    read(&mut reads, "data/palette.ron", true, 0);
    read(&mut reads, "data/terrain.ron", true, 10);
    read(&mut reads, "data/palette.ron", true, 40);
    read(&mut reads, "lang/fr/ui.ron", false, 42);
    read(&mut reads, "dialogue/ch01.dlg", true, 43);
    NOW.set(113_000);
    reads.mark("the title");
    assert_eq!(
        reads.summary(143_000, 10),
        [
            "Bundle: 5 reads of 4 files, 1 not in the bundle",
            "Not in the bundle: lang/fr/ui.ron",
            "Slowest reads, ms to the next read: dialogue/ch01.dlg 70, \
             (the title) 30, data/terrain.ron 30, data/palette.ron 12, lang/fr/ui.ron 1",
        ]
    );
    assert_eq!(
        reads.each(143_000),
        [
            "  dialogue/ch01.dlg: 70.0 ms",
            "  (the title): 30.0 ms",
            "  data/terrain.ron: 30.0 ms",
            "  data/palette.ron: 12.0 ms, 2 reads",
            "  lang/fr/ui.ron: 1.0 ms, not in the bundle",
        ]
    );
}

/// Equal times are in the order of their paths, and only the slowest are
/// named in the summary.
#[test]
fn the_summary_names_only_the_slowest_and_ties_go_by_path() {
    let mut reads = Reads::new(at);
    for (i, path) in ["c", "a", "b"].into_iter().enumerate() {
        read(&mut reads, path, true, 5 * i as u64);
    }
    assert_eq!(
        reads.summary(15_000, 2),
        [
            "Bundle: 3 reads of 3 files, 0 not in the bundle",
            "Slowest reads, ms to the next read: a 5, b 5",
        ]
    );
    assert_eq!(reads.each(15_000).len(), 3);
    assert_eq!(
        Reads::new(at).summary(0, 10),
        ["Bundle: 0 reads of 0 files, 0 not in the bundle"]
    );
}

#[test]
fn the_stack_line_says_where_it_is_not_measured() {
    assert_eq!(
        stack_line(Some(15_256), Some(65_536)),
        "Game stack used: 15,256 of 65,536 bytes"
    );
    assert_eq!(stack_line(Some(800), None), "Game stack used: 800 bytes");
    assert_eq!(
        stack_line(None, Some(65_536)),
        "Game stack: measured on the device only"
    );
}

#[test]
fn the_running_lines_name_the_screen_the_rate_the_times_and_the_music() {
    let mut stretch = Stretch::default();
    for _ in 0..1_788 {
        stretch.frame.add(1_000);
    }
    let mut measured = Measured::default();
    measured.draw.add(12_000);
    let now = Now {
        screen: Some("lake"),
        ms: 60_000,
        stack_used: Some(20_000),
        music: Some("theme"),
        underran: true,
    };
    let report = RunReport::new().stack_bytes(65_536);
    assert_eq!(
        stretch.lines(&now, &measured, &report),
        [
            "On lake for 60 seconds: 29.8 frames a second",
            "App::frame: min 1.0 / avg 1.0 / max 1.0 ms, 1,788 times",
            "Draw and flush: min 12.0 / avg 12.0 / max 12.0 ms, 1 times",
            "Game stack used: 20,000 of 65,536 bytes",
            "Music: theme, and a file player ran out of data",
        ]
    );
    let quiet = Now {
        screen: None,
        music: None,
        underran: false,
        ..now
    };
    let lines = Stretch::default().lines(&quiet, &Measured::default(), &report);
    assert_eq!(lines[0], "On no screen for 60 seconds: 0.0 frames a second");
    assert_eq!(lines.last().map(String::as_str), Some("Music: none"));
}

#[test]
fn the_phases_line_names_every_phase() {
    let mut phases = Phases::default();
    for us in [1_000, 3_000] {
        phases.add(Phase::Ground, us);
        phases.add(Phase::Glyphs, us * 2);
    }
    assert_eq!(
        phases.line(),
        "Draw by phase, avg / max ms: ground 2.0 / 3.0, backdrop 0.0 / 0.0, \
         backgrounds 0.0 / 0.0, under items 0.0 / 0.0, edges 0.0 / 0.0, \
         glyphs 4.0 / 6.0, over items 0.0 / 0.0, invert 0.0 / 0.0"
    );
    let measured = Measured {
        phases: Some(phases),
        ..Measured::default()
    };
    let now = Now {
        screen: Some("home"),
        ms: 1_000,
        stack_used: None,
        music: None,
        underran: false,
    };
    let lines: Vec<String> = Stretch::default().lines(&now, &measured, &RunReport::new());
    assert!(
        lines.iter().any(|l| l.starts_with("Draw by phase")),
        "{lines:?}"
    );
}

#[test]
fn a_mark_or_a_read_with_no_report_on_notes_nothing() {
    with_reads(|reads| *reads = None);
    mark("nothing");
    noted("a.ron", true);
    assert!(with_reads(|reads| reads.is_none()));
    with_reads(|reads| *reads = Some(Reads::new(at)));
    NOW.set(0);
    mark("step");
    noted("a.ron", false);
    let reads = with_reads(Option::take).unwrap_or_else(|| panic!("noted"));
    assert_eq!(
        reads.summary(0, 10)[..2],
        [
            "Bundle: 1 reads of 1 files, 1 not in the bundle",
            "Not in the bundle: a.ron"
        ]
    );
}
