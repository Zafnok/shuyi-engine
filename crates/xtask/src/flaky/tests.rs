use super::*;

/// A judge of cells, and the job that records its `windows*` ones.
const JUDGE: &str = "gate / judge";
const RECORDER: &str = "gate / record";

fn ledger(entries: &str) -> Ledger {
    parse(&format!(
        r#"(
            matrices: [(job: "{JUDGE}", artifact: "gate", recorded: [(platforms: "windows", job: "{RECORDER}")])],
            entries: [{entries}],
        )"#
    ))
    .unwrap()
}

const SOUND_CPU: &str = r#"(
    cell: Matrix(platform: "browser-chromium", check: "perf:cpu_ms"),
    ticket: "2068", seen: "2026-11-08", rate: (failed: 1, runs: 20), runs: ["CI run 1"],
)"#;
const WINDOWS_CPU: &str = r#"(
    cell: Matrix(platform: "windows-in-house", check: "perf:cpu_ms"),
    ticket: "2068", seen: "2026-11-12", rate: (failed: 1, runs: 20), runs: ["CI run 2"],
)"#;
const LABEL: &str = r#"(
    cell: Job("api-label"),
    ticket: "2999", seen: "2026-11-01", rate: (failed: 2, runs: 30), runs: ["CI run 3"],
)"#;
const TEST: &str = r#"(
    cell: Test(job: "test", name: "audio::tests::starts"),
    ticket: "2998", seen: "2026-11-01", rate: (failed: 1, runs: 10), runs: ["CI run 4"],
)"#;

fn job(name: &str, id: u64, failed: bool) -> Job {
    Job {
        name: name.to_string(),
        id,
        failed,
        failed_tests: Vec::new(),
        matrix_cells: None,
    }
}

fn judged(cells: &[(&str, &str)]) -> Job {
    Job {
        matrix_cells: Some(
            cells
                .iter()
                .map(|(p, c)| ((*p).to_string(), (*c).to_string()))
                .collect(),
        ),
        ..job(JUDGE, 10, true)
    }
}

#[test]
fn the_checked_in_ledger_parses_and_names_open_tickets() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let text = std::fs::read_to_string(root.join(LEDGER)).unwrap();
    // An empty ledger is a valid one (as giving up the sound replacement
    // leaves it).
    let ledger = parse(&text).unwrap();
    let open: Vec<String> = std::fs::read_dir(root.join("tickets/open"))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy()[..4].to_string())
        .collect();
    let open: HashSet<&str> = open.iter().map(String::as_str).collect();
    assert_eq!(
        check_tickets(&ledger, &open, &HashSet::new()),
        Vec::<String>::new()
    );
}

#[test]
fn each_cell_kind_parses() {
    let ledger = ledger(&[SOUND_CPU, LABEL, TEST].join(","));
    assert_eq!(
        ledger.entries[0].cell,
        Cell::Matrix {
            platform: "browser-chromium".into(),
            check: "perf:cpu_ms".into(),
        }
    );
    assert_eq!(ledger.entries[1].cell, Cell::Job("api-label".into()));
    assert_eq!(
        ledger.entries[2].cell,
        Cell::Test {
            job: "test".into(),
            name: "audio::tests::starts".into(),
        }
    );
    assert_eq!(
        ledger.entries[0].to_string(),
        "cell `browser-chromium perf:cpu_ms` (ticket 2068, 1 in 20, seen 2026-11-08)"
    );
}

#[test]
fn a_bad_entry_is_refused_with_each_reason() {
    let errors = parse(
        r#"(entries: [
            (cell: Job("a"), ticket: "68", seen: "8 Nov", rate: (failed: 0, runs: 20), runs: []),
            (cell: Job("a"), ticket: "2068", seen: "2026-11-08", rate: (failed: 3, runs: 2), runs: ["x"]),
        ])"#,
    )
    .unwrap_err();
    let all = errors.join("\n");
    assert!(all.contains("not a four-digit id"), "{all}");
    assert!(all.contains("not YYYY-MM-DD"), "{all}");
    assert!(all.contains("0 in 20"), "{all}");
    assert!(all.contains("3 in 2"), "{all}");
    assert!(all.contains("no runs named"), "{all}");
    assert!(all.contains("listed twice"), "{all}");
    assert_eq!(errors.len(), 6);
}

#[test]
fn an_unknown_field_is_refused() {
    assert!(parse(r#"(entries: [(cell: Job("a"), ticket: "2068", seen: "2026-11-08", rate: (failed: 1, runs: 2), runs: ["x"], note: "")])"#).is_err());
}

#[test]
fn an_entry_whose_ticket_is_done_fails_and_an_open_one_passes() {
    let ledger = ledger(&[SOUND_CPU, LABEL].join(","));
    let open = HashSet::from(["2999"]);
    let done = HashSet::from(["2068"]);
    let errors = check_tickets(&ledger, &open, &done);
    assert_eq!(errors.len(), 1);
    assert!(
        errors[0].contains("ticket 2068, which is done"),
        "{}",
        errors[0]
    );

    let errors = check_tickets(&ledger, &HashSet::from(["2068"]), &HashSet::new());
    assert_eq!(errors.len(), 1);
    assert!(
        errors[0].contains("2999, which doesn't exist"),
        "{}",
        errors[0]
    );
}

#[test]
fn failed_tests_are_read_from_a_timestamped_log() {
    let log = "2026-11-08T17:51:33.1Z test a::b ... ok\n\
               2026-11-08T17:51:33.2Z test a::c ... FAILED\n\
               2026-11-08T17:51:33.3Z test a::c ... FAILED\n\
               test d ... FAILED\r\n\
               failures:\n";
    assert_eq!(failed_tests(log), vec!["a::c", "d"]);
}

#[test]
fn matrix_cells_are_the_mandatory_ones_that_did_not_pass() {
    let matrix = r#"{"cells": [
        {"platform": "a", "check": "x", "mandatory": true, "status": "passed"},
        {"platform": "a", "check": "y", "mandatory": true, "status": "failed"},
        {"platform": "b", "check": "z", "mandatory": false, "status": "failed"},
        {"platform": "c", "check": "w", "mandatory": true, "status": "inconclusive"}
    ]}"#;
    assert_eq!(
        matrix_cells(matrix).unwrap(),
        vec![("a".into(), "y".into()), ("c".into(), "w".into())]
    );
}

#[test]
fn a_listed_matrix_cell_reruns_the_failed_jobs_once() {
    let ledger = ledger(SOUND_CPU);
    let jobs = [
        judged(&[("browser-chromium", "perf:cpu_ms")]),
        job(RECORDER, 11, false),
        job("ci-result", 12, true),
    ];
    let (decision, notes) = rerun(&ledger, &jobs, 1);
    assert_eq!(decision, Rerun::Failed);
    assert!(
        notes[0].contains("browser-chromium perf:cpu_ms` (ticket 2068"),
        "{notes:?}"
    );
    // The re-run isn't re-run again.
    assert_eq!(rerun(&ledger, &jobs, 2).0, Rerun::None);
}

#[test]
fn a_cell_not_listed_gets_no_rerun() {
    let ledger = ledger(SOUND_CPU);
    let jobs = [judged(&[
        ("browser-chromium", "perf:cpu_ms"),
        ("linux-native", "signal:tone"),
    ])];
    let (decision, notes) = rerun(&ledger, &jobs, 1);
    assert_eq!(decision, Rerun::None);
    assert!(
        notes[0].contains("`linux-native signal:tone` not listed"),
        "{notes:?}"
    );

    // A listed cell with another job failing on something not listed.
    let jobs = [
        judged(&[("browser-chromium", "perf:cpu_ms")]),
        job("clippy", 3, true),
    ];
    let (decision, notes) = rerun(&ledger, &jobs, 1);
    assert_eq!(decision, Rerun::None);
    assert!(
        notes[0].contains("`clippy` failed with no failed test"),
        "{notes:?}"
    );

    // The gate failed before it judged a cell.
    let (decision, _) = rerun(&ledger, &[job(JUDGE, 1, true)], 1);
    assert_eq!(decision, Rerun::None);
    let (decision, _) = rerun(&ledger, &[judged(&[])], 1);
    assert_eq!(decision, Rerun::None);
}

#[test]
fn nothing_failed_is_no_rerun() {
    let ledger = ledger(SOUND_CPU);
    assert_eq!(
        rerun(&ledger, &[job("fmt", 1, false)], 1),
        (Rerun::None, Vec::new())
    );
}

#[test]
fn a_windows_cell_reruns_the_job_that_records_windows() {
    let ledger = ledger(&[SOUND_CPU, WINDOWS_CPU, LABEL].join(","));
    let jobs = [
        judged(&[
            ("windows-in-house", "perf:cpu_ms"),
            ("browser-chromium", "perf:cpu_ms"),
        ]),
        job(RECORDER, 11, false),
    ];
    assert_eq!(rerun(&ledger, &jobs, 1).0, Rerun::Job(11));

    // With another listed job failing too, one call can't re-run both.
    let jobs = [
        judged(&[("windows-in-house", "perf:cpu_ms")]),
        job(RECORDER, 11, false),
        job("api-label", 12, true),
    ];
    let (decision, notes) = rerun(&ledger, &jobs, 1);
    assert_eq!(decision, Rerun::None);
    assert!(notes[0].contains("other jobs failed too"), "{notes:?}");
}

#[test]
fn a_listed_job_or_test_matches() {
    let ledger = ledger(&[LABEL, TEST].join(","));
    let mut test = job("test", 2, true);
    test.failed_tests = vec!["audio::tests::starts".into()];
    let (decision, notes) = rerun(&ledger, &[job("api-label", 1, true), test.clone()], 1);
    assert_eq!(decision, Rerun::Failed);
    assert!(
        notes[0].contains("`api-label` on job `api-label`"),
        "{notes:?}"
    );
    assert!(
        notes[0].contains("`test` on test `audio::tests::starts` of `test`"),
        "{notes:?}"
    );

    // The same test name in another job isn't listed.
    let mut other = test.clone();
    other.name = "determinism".into();
    assert_eq!(rerun(&ledger, &[other], 1).0, Rerun::None);
    // Nor is a second failed test beside it.
    test.failed_tests.push("audio::tests::stops".into());
    assert_eq!(rerun(&ledger, &[test], 1).0, Rerun::None);
}

#[test]
fn the_report_names_listed_and_unlisted_failures() {
    let ledger = ledger(SOUND_CPU);
    let jobs = [
        judged(&[("browser-chromium", "perf:cpu_ms")]),
        job("fmt", 2, true),
    ];
    let warnings = report(&ledger, &jobs, 1);
    assert_eq!(warnings.len(), 2);
    assert!(
        warnings[0].contains("attempt 1 failed on listed flakes"),
        "{warnings:?}"
    );
    assert!(warnings[0].contains("ticket 2068"), "{warnings:?}");
    assert!(warnings[1].contains("`fmt` failed"), "{warnings:?}");
    assert!(warnings[1].contains("second sighting"), "{warnings:?}");
}

#[test]
fn jobs_are_loaded_with_their_logs_and_matrix() {
    let dir = std::env::temp_dir().join(format!("xtask-flaky-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("t.log"), "Z test x ... FAILED\n").unwrap();
    std::fs::write(
        dir.join("m.json"),
        r#"{"cells": [{"platform": "a", "check": "b", "mandatory": true, "status": "failed"}]}"#,
    )
    .unwrap();
    let failures = dir.join("f.json");
    std::fs::write(
        &failures,
        serde_json::json!([
            {"name": "test", "id": 1, "conclusion": "failure", "log": dir.join("t.log")},
            {"name": JUDGE, "id": 2, "conclusion": "failure", "matrix": dir.join("m.json")},
            {"name": "fmt", "id": 3, "conclusion": "success"},
            {"name": "site", "id": 4, "conclusion": null},
        ])
        .to_string(),
    )
    .unwrap();
    let jobs = load_jobs(&failures).unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
    assert_eq!(jobs[0].failed_tests, vec!["x"]);
    assert_eq!(jobs[1].matrix_cells, Some(vec![("a".into(), "b".into())]));
    assert!(jobs[0].failed && jobs[1].failed && !jobs[2].failed && !jobs[3].failed);
}

#[test]
fn a_calibration_table_counts_each_run_and_cell() {
    let dir = std::env::temp_dir().join(format!("xtask-calibration-{}", std::process::id()));
    for (n, status) in [(1, "passed"), (2, "failed"), (3, "failed")] {
        let run = dir.join(format!("gate-run-{n}"));
        std::fs::create_dir_all(&run).unwrap();
        std::fs::write(
            run.join("matrix.json"),
            format!(r#"{{"cells": [{{"platform": "a", "check": "b", "mandatory": true, "status": "{status}"}}]}}"#),
        )
        .unwrap();
    }
    let jobs: Vec<CalibrationJob> = serde_json::from_value(serde_json::json!([
        {"name": "runs", "conclusion": "success"},
        {"name": "gate (1) / record", "conclusion": "success"},
        {"name": "gate (1) / judge", "conclusion": "success"},
        {"name": "gate (2) / record", "conclusion": "success"},
        {"name": "gate (2) / judge", "conclusion": "failure"},
        {"name": "gate (3) / record", "conclusion": "success"},
        {"name": "gate (3) / judge", "conclusion": "failure"},
        {"name": "gate (4) / record", "conclusion": "failure"},
        {"name": "gate (4) / judge", "conclusion": "skipped"},
    ]))
    .unwrap();
    let table = calibration("gate", &jobs, &dir, "abc").unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
    assert!(table.contains("4 runs of abc"), "{table}");
    assert!(
        table.contains("| 1 | success | success | none |"),
        "{table}"
    );
    assert!(
        table.contains("| 2 | failure | success | `a b` |"),
        "{table}"
    );
    assert!(
        table.contains("| 4 | skipped | failure | no matrix |"),
        "{table}"
    );
    assert!(table.contains("1 of 4 runs passed every job."), "{table}");
    assert!(table.contains("| `a b` | 2 in 4 | 2, 3 |"), "{table}");
    assert!(calibration("bench", &jobs, Path::new("."), "abc").is_err());
}
