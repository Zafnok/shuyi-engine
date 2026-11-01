//! The comparator on signals made here: what's right passes, and each fault
//! fails naming what is wrong. CI runs the same faults on real recordings.

use serde_json::json;

use super::compare::{Ch, Class, Fault, align, check, inject, repeat_difference};
use super::corpus::{MARKER_HEAD_MS, RATE, marker, sources};
use super::*;

fn tolerance() -> Tolerance {
    Tolerance {
        timing_ms: 40,
        level_db: 1.0,
        silent_peak: 8,
        silent_tone: 0.002,
        marker_correlation: 0.95,
        exact_peak: 1,
        decoded_peak: 4,
        repeat_db: 1.0,
    }
}

fn done() -> Log {
    Log {
        done: true,
        ..Log::default()
    }
}

/// `pad` frames of silence, then `samples`, then 200 ms of silence.
fn recording(pad: usize, samples: &[i16]) -> Recording {
    let mut all = vec![0; pad * 2];
    all.extend_from_slice(samples);
    all.extend(vec![0; RATE as usize / 5 * 2]);
    Recording::new(&all)
}

fn signal(name: &str) -> Vec<i16> {
    sources()
        .into_iter()
        .find(|(n, _)| *n == name)
        .map_or_else(|| panic!("no {name}"), |(_, s)| s.samples)
}

fn oracle(file: &str) -> Option<Vec<i16>> {
    sources()
        .into_iter()
        .find(|(n, _)| *n == file)
        .map(|(_, s)| s.samples)
}

fn tone_file() -> Vec<Expect> {
    vec![
        Expect::Waveform {
            file: "fmt_44k_s.wav".to_owned(),
            repeats: 1,
            class: Class::Exact,
        },
        Expect::Tone {
            from: 400,
            to: 1200,
            ch: Ch::Left,
            hz: 1000.0,
            amp: 0.5,
        },
        Expect::Tone {
            from: 400,
            to: 1200,
            ch: Ch::Right,
            hz: 500.0,
            amp: 0.5,
        },
        Expect::Silent {
            from: 1350,
            to: 1450,
        },
    ]
}

fn problems(recording: &Recording, expect: &[Expect]) -> Vec<String> {
    check(expect, 1450, recording, &done(), &oracle, &tolerance())
}

#[test]
fn the_right_signal_passes() {
    let good = recording(4000, &signal("fmt_44k_s.wav"));
    assert_eq!(problems(&good, &tone_file()), Vec::<String>::new());
}

#[test]
fn the_marker_places_the_recording_to_the_frame() {
    let good = recording(4321, &signal("fmt_44k_s.wav"));
    assert_eq!(align(&good, &tolerance()), Ok(4321));
}

#[test]
fn a_rounding_step_of_conversion_passes_and_a_gain_does_not() {
    let source = signal("fmt_44k_s.wav");
    let off_by_one: Vec<i16> = source.iter().map(|s| s.saturating_add(1)).collect();
    assert_eq!(
        problems(&recording(100, &off_by_one), &tone_file()),
        Vec::<String>::new()
    );
    let quieter: Vec<i16> = source.iter().map(|s| s / 2).collect();
    let found = problems(&recording(100, &quieter), &tone_file());
    assert!(found.iter().any(|p| p.contains("differs")), "{found:?}");
    assert!(found.iter().any(|p| p.contains("Hz")), "{found:?}");
}

/// Each fault of the gate's list, by what the comparator says of it.
#[test]
fn every_fault_fails_naming_what_is_wrong() {
    let good = recording(4000, &signal("fmt_44k_s.wav"));
    let cases = [
        (Fault::Silence, "silent recording"),
        (Fault::SwapChannels, "missing marker"),
        (Fault::WrongRate(1.06), "missing marker"),
        (Fault::Drop(400, 1300), "differs"),
        (Fault::Clip, "clipping"),
        (Fault::MissingMarker, "missing marker"),
    ];
    for (fault, says) in cases {
        let broken = inject(&good, fault, &tolerance());
        let found = problems(&broken, &tone_file());
        assert!(
            found.iter().any(|p| p.contains(says)),
            "{fault:?}: {found:?}"
        );
    }
}

#[test]
fn a_wrong_rate_or_channel_after_a_good_marker_fails_on_the_tone() {
    let (left, right) = marker(300);
    let mut samples: Vec<i16> = left
        .iter()
        .zip(&right)
        .flat_map(|(l, r)| [*l, *r])
        .collect();
    // The tone of the file, with its channels the wrong way round.
    let tone = signal("tone.wav");
    let swapped: Vec<i16> = tone.chunks(2).flat_map(|p| [p[1], p[0]]).collect();
    samples.extend_from_slice(&swapped);
    let expect = [Expect::Tone {
        from: 400,
        to: 1200,
        ch: Ch::Left,
        hz: 440.0,
        amp: 0.5,
    }];
    let found = problems(&recording(10, &samples), &expect);
    assert!(found.iter().any(|p| p.contains("440")), "{found:?}");
}

#[test]
fn a_missing_loop_pass_fails_and_whole_passes_pass() {
    let pass = signal("loop.wav");
    let mut three = pass.clone();
    three.extend_from_slice(&pass);
    three.extend_from_slice(&pass);
    let expect = [Expect::Waveform {
        file: "loop.wav".to_owned(),
        repeats: 3,
        class: Class::Exact,
    }];
    let check_it = |r: &Recording| check(&expect, 1500, r, &done(), &oracle, &tolerance());
    assert_eq!(check_it(&recording(50, &three)), Vec::<String>::new());
    // A gap of one frame at the first boundary shifts all that follows.
    let mut gap = pass.clone();
    gap.extend_from_slice(&[0, 0]);
    gap.extend_from_slice(&pass);
    gap.extend_from_slice(&pass);
    let found = check_it(&recording(50, &gap));
    assert!(found.iter().any(|p| p.contains("pass 2")), "{found:?}");
}

#[test]
fn a_silent_recording_is_a_failure_and_only_no_signal_expects_one() {
    let silent = Recording::new(&vec![0; RATE as usize]);
    let found = problems(&silent, &tone_file());
    assert!(found.iter().any(|p| p.contains("silent")), "{found:?}");
    assert_eq!(problems(&silent, &[Expect::NoSignal]), Vec::<String>::new());
    let good = recording(10, &signal("marker.wav"));
    assert_ne!(problems(&good, &[Expect::NoSignal]).len(), 0);
}

#[test]
fn a_probe_that_failed_or_never_finished_fails() {
    let good = recording(10, &signal("fmt_44k_s.wav"));
    let log = Log {
        failed: Some("no device".to_owned()),
        ..Log::default()
    };
    let found = check(&tone_file(), 1450, &good, &log, &oracle, &tolerance());
    assert!(found.iter().any(|p| p.contains("no device")), "{found:?}");
    assert!(found.iter().any(|p| p.contains("done")), "{found:?}");
}

#[test]
fn the_state_and_warnings_are_checked_against_the_log() {
    let good = recording(10, &signal("fmt_44k_s.wav"));
    let log = Log {
        steps: vec![(100, "music_a".to_owned(), 1.0)],
        warnings: vec!["this backend can't pause sounds".to_owned()],
        done: true,
        ..Log::default()
    };
    let state = |cue: &str, secs| Expect::State {
        at: 100,
        cue: cue.to_owned(),
        secs,
    };
    let run = |expect: &[Expect]| check(expect, 100, &good, &log, &oracle, &tolerance());
    assert_eq!(run(&[state("music_a", 1.01)]).len(), 0);
    assert_ne!(run(&[state("music_b", 1.0)]).len(), 0);
    assert_ne!(run(&[state("music_a", 1.5)]).len(), 0);
    assert_eq!(run(&[Expect::Warned("can't pause".to_owned())]).len(), 0);
    assert_ne!(run(&[Expect::Warned("something else".to_owned())]).len(), 0);
}

#[test]
fn two_good_runs_are_alike_and_a_dropped_stretch_is_not() {
    let good = recording(10, &signal("fmt_44k_s.wav"));
    let later = recording(777, &signal("fmt_44k_s.wav"));
    let same = repeat_difference(&good, &later, &[], 1300, &tolerance());
    assert_eq!(same, Ok(0.0));
    let broken = inject(&later, Fault::Drop(600, 900), &tolerance());
    let apart = repeat_difference(&good, &broken, &[], 1300, &tolerance());
    assert!(apart.is_ok_and(|db| db > 10.0));
}

#[test]
fn the_marker_head_is_what_alignment_matches() {
    assert_eq!(
        marker(MARKER_HEAD_MS).0.len(),
        (MARKER_HEAD_MS * RATE / 1000) as usize
    );
}

// -- the matrix --

const PINS: (&str, &str, &str) = ("rev", "corpus", "scenarios");

fn pins() -> Pins {
    Pins {
        revision: PINS.0.to_owned(),
        corpus: PINS.1.to_owned(),
        scenarios: PINS.2.to_owned(),
    }
}

fn meta(revision: &str, corpus: &str, scenarios: &str) -> Meta {
    Meta {
        revision: revision.to_owned(),
        backend: "quad-snd".to_owned(),
        corpus_sha256: corpus.to_owned(),
        scenarios_sha256: scenarios.to_owned(),
        environment: Value::Null,
    }
}

#[test]
fn stale_evidence_is_refused() {
    assert_eq!(stale(&meta(PINS.0, PINS.1, PINS.2), &pins()), Ok(()));
    for old in [
        meta("older", PINS.1, PINS.2),
        meta(PINS.0, "older", PINS.2),
        meta(PINS.0, PINS.1, "older"),
    ] {
        let why = stale(&old, &pins()).err().unwrap_or_default();
        assert!(why.contains("stale"), "{why}");
    }
}

#[test]
fn a_malformed_log_line_is_an_error() {
    assert!(parse_log("{\"event\":\"done\"}\n").is_ok_and(|log| log.done));
    assert!(parse_log("{\"event\":\"do").is_err());
}

fn matrix(cells: &[Value]) -> String {
    json!({"schema": SCHEMA, "cells": cells}).to_string()
}

#[test]
fn every_mandatory_cell_but_a_passed_one_fails_the_gate() {
    for status in ["failed", "inconclusive", "unsupported"] {
        let one = matrix(&[cell("p", "c", true, status, "why")]);
        assert_eq!(failures(&one).len(), 1, "{status}");
    }
    assert_eq!(
        failures(&matrix(&[cell("p", "c", true, "passed", "")])).len(),
        0
    );
    assert_eq!(
        failures(&matrix(&[cell("p", "c", false, "unsupported", "")])).len(),
        0
    );
}

#[test]
fn a_broken_matrix_fails_the_gate() {
    assert_ne!(failures("not json").len(), 0);
    assert_ne!(failures(&matrix(&[])).len(), 0);
    assert_ne!(
        failures(&json!({"schema": SCHEMA + 1, "cells": []}).to_string()).len(),
        0
    );
    assert_ne!(failures(&json!({"schema": SCHEMA}).to_string()).len(), 0);
}

#[test]
fn a_port_needs_real_evidence() {
    let blocked = matrix(&[cell("windows", "capture", false, "unsupported", "3031")]);
    assert!(port_ready(&blocked, "windows").is_err());
    assert!(port_ready(&blocked, "macos").is_err());
    let good = matrix(&[
        cell("windows", "capture", true, "passed", ""),
        cell("windows", "scenario:onset", true, "passed", ""),
    ]);
    assert_eq!(port_ready(&good, "windows"), Ok(()));
    let only_capture = matrix(&[cell("windows", "capture", true, "passed", "")]);
    assert!(port_ready(&only_capture, "windows").is_err());
    let bad = matrix(&[
        cell("windows", "scenario:onset", true, "passed", ""),
        cell("windows", "scenario:loop", true, "failed", ""),
    ]);
    assert!(port_ready(&bad, "windows").is_err());
    assert!(port_ready("not json", "windows").is_err());
}

// -- the performance verdict --

const LIMIT: Limit = Limit {
    max_ratio: 1.2,
    max_spread: 0.1,
};

#[test]
fn equal_work_passes_a_slowdown_fails_and_noise_is_inconclusive() {
    let before = [10.0; 9];
    let same = ratios(&before, &[10.0; 9]);
    assert_eq!(same.median, 1.0);
    assert_eq!(judge(&same, LIMIT), "passed");
    assert_eq!(judge(&ratios(&before, &[15.0; 9]), LIMIT), "failed");
    let noisy = [5.0, 15.0, 5.0, 15.0, 5.0, 15.0, 5.0, 15.0, 10.0];
    assert_eq!(judge(&ratios(&before, &noisy), LIMIT), "inconclusive");
}

#[test]
fn too_few_pairs_is_inconclusive() {
    let few = ratios(&[10.0; 8], &[10.0; 8]);
    assert_eq!(few.pairs, 8);
    assert_eq!(judge(&few, LIMIT), "inconclusive");
}

#[test]
fn a_gate_without_budget_or_samples_fails_closed() {
    let budget = Budget {
        version: 1,
        correctness: tolerance(),
        performance: BTreeMap::new(),
        platforms: BTreeMap::new(),
    };
    let dir = std::env::temp_dir();
    let cells = performance_cells("no-such-platform", &dir, &budget);
    assert_eq!(cells[0]["status"], "failed");
}

#[test]
fn the_files_in_the_repository_parse() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let tool = root.join(TOOL);
    let budget: Result<Budget, _> = read_ron(&tool.join("budget.ron"));
    let plan: Result<Plan, _> = read_ron(&tool.join("scenarios.ron"));
    let manifest: Result<Manifest, _> = read_ron(&tool.join("corpus.ron"));
    assert!(budget.is_ok(), "{:?}", budget.err());
    assert!(manifest.is_ok(), "{:?}", manifest.err());
    let plan = plan.unwrap_or_else(|e| panic!("{e}"));
    for (_, scenario, _) in FAULTS {
        assert!(
            plan.scenarios.iter().any(|s| s.name == scenario),
            "{scenario}"
        );
    }
}

#[test]
fn the_corpus_is_what_its_manifest_pins() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let manifest: Manifest =
        read_ron(&root.join(TOOL).join("corpus.ron")).unwrap_or_else(|e| panic!("{e}"));
    for (name, signal) in sources() {
        assert_eq!(
            manifest.wav.get(name),
            Some(&sha256(&corpus::wav(&signal))),
            "{name}"
        );
    }
}

/// ADR-0068's fence: going back stays possible. 3084 takes out the default's
/// check when it makes the new backend the default; 3079 takes out the rest.
#[test]
fn quad_snd_stays_and_stays_the_default() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let table = |path: &str| -> toml::Table {
        let text = fs::read_to_string(root.join(path)).unwrap_or_else(|e| panic!("{path}: {e}"));
        toml::from_str(&text).unwrap_or_else(|e| panic!("{path}: {e}"))
    };
    let app = table("crates/app/Cargo.toml");
    let quad_snd = app["dependencies"].get("quad-snd");
    assert!(
        quad_snd.is_some_and(|dep| dep.get("optional").is_none()),
        "only 3079 removes quad-snd or makes it optional (ADR-0068)"
    );
    let has = |list: Option<&toml::Value>, name: &str| {
        list.and_then(toml::Value::as_array)
            .is_some_and(|items| items.iter().any(|item| item.as_str() == Some(name)))
    };
    assert!(
        !has(app["features"].get("default"), "in-house-sound"),
        "only 3084 makes the new backend the default (ADR-0068)"
    );
    let workspace = table("Cargo.toml");
    let macroquad = &workspace["workspace"]["dependencies"]["macroquad"];
    assert!(
        has(macroquad.get("features"), "audio"),
        "only 3079 removes macroquad's audio (ADR-0068)"
    );
}

// -- giving up --

#[test]
fn giving_up_covers_all_the_chain_added() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    assert_eq!(give_up::run(&root, false), Vec::<String>::new());
}

#[test]
fn a_file_naming_the_chain_outside_the_footprint_is_caught() {
    let owned = ["tools/sound-parity".to_owned()];
    let files = |names: &[&str]| -> Vec<String> { names.iter().map(|n| (*n).to_owned()).collect() };
    let clean = files(&[
        "tools/sound-parity/a.ron",
        "tickets/open/1.md",
        "docs/adr/1.md",
    ]);
    let none = BTreeMap::new();
    assert_eq!(give_up::uncovered(&clean, &owned, &none).len(), 0);
    let stray = files(&["crates/new/Cargo.toml"]);
    assert_eq!(give_up::uncovered(&stray, &owned, &none).len(), 1);
    // An edit counts only if it leaves nothing of the chain behind.
    let mut edited = BTreeMap::new();
    edited.insert("crates/new/Cargo.toml".to_owned(), "clean".to_owned());
    assert_eq!(give_up::uncovered(&stray, &owned, &edited).len(), 0);
    edited.insert(
        "crates/new/Cargo.toml".to_owned(),
        "shuyi-sound = 1".to_owned(),
    );
    assert_eq!(give_up::uncovered(&stray, &owned, &edited).len(), 1);
}

#[test]
fn an_edit_that_no_longer_matches_is_an_error() {
    let replace = |find: &str| give_up::Edit::Replace {
        file: "f".to_owned(),
        find: find.to_owned(),
        with: "x".to_owned(),
    };
    assert_eq!(replace("b").apply("abc"), Ok("axc".to_owned()));
    assert!(replace("z").apply("abc").is_err());
    assert!(replace("a").apply("aa").is_err());
    let cut = |from: &str, to: &str| give_up::Edit::Cut {
        file: "f".to_owned(),
        from: from.to_owned(),
        to: to.to_owned(),
    };
    assert_eq!(cut("b", "d").apply("abcde"), Ok("ade".to_owned()));
    assert!(cut("d", "b").apply("abcde").is_err());
    assert!(cut("z", "b").apply("abcde").is_err());
}
