//! `bench --long`: the long scenario's file, the flatness and determinism
//! rules on samples made by hand, the report's table, a Harness without
//! logs, and the toy played in this program. CI's `bench` job plays the
//! toy's fixtures in processes of their own (`toy/long/`).

use std::path::PathBuf;

use shuyi_alloc_count::{global, tags};
use shuyi_platform::harness::{FRAME_DT, Harness};
use shuyi_ui::perf::TagClass;
use shuyi_ui::toy;

use super::*;
use crate::bench::{LongPlayer, measure_long};
use crate::toy::ToyTools;

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("shuyi-long-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

const GOOD: &str = r#"(
    format_version: 1,
    about: "the toy idling",
    start: [(flag: "--leak-bytes", value: "0")],
    script: (steps: [keys("Down"), wait(0.5)]),
    repeat: 40,
    idle: 100,
    warm_up: 500,
    sample_every: 100,
    slack: (sounds: 1),
)"#;

#[test]
fn a_long_scenario_reads_with_its_defaults() {
    let read = LongScenario::read("a.long.ron", GOOD).unwrap();
    assert_eq!(read.about, "the toy idling");
    assert_eq!(read.start.len(), 1);
    assert_eq!(
        (read.repeat, read.idle, read.warm_up, read.sample_every),
        (40, 100, 500, 100)
    );
    // A slack given in part keeps the other half's default.
    assert_eq!(
        read.slack,
        Slack {
            heap_bytes: 65_536,
            sounds: 1
        }
    );
    let least = "(format_version: 1, repeat: 1, sample_every: 1)";
    let least = LongScenario::read("b.long.ron", least).unwrap();
    assert_eq!(least.slack, Slack::default());
    assert_eq!((least.idle, least.warm_up), (0, 0));
}

#[test]
fn a_bad_long_scenario_is_refused_naming_what_is_wrong() {
    let bad = |text: &str| LongScenario::read("x.long.ron", text).unwrap_err();
    let e = bad("(format_version: 1, repeat: 1, sample_every: 1, speed: 2)");
    assert!(e.starts_with("x.long.ron:") && e.contains("speed"), "{e}");
    let e = bad("(format_version: 1, repeat: 1, sample_every: 1, slack: (bytes: 2))");
    assert!(e.contains("bytes"), "{e}");
    assert_eq!(
        bad("(format_version: 2, repeat: 1, sample_every: 1)"),
        "x.long.ron: format_version 2: this shuyi reads 1"
    );
    assert_eq!(
        bad("(format_version: 1, repeat: 0, sample_every: 1)"),
        "x.long.ron: repeat: 1 or more"
    );
    assert_eq!(
        bad("(format_version: 1, repeat: 1, sample_every: 0)"),
        "x.long.ron: sample_every: 1 or more"
    );
    let e = bad("(format_version: 1, sample_every: 1)");
    assert!(e.contains("repeat"), "{e}");
    assert_eq!(name_of("long/stage-loop.long.ron"), "stage-loop");
}

/// Samples at frames `every`, `2 × every`, … up to `last`, with the live
/// heap, a tag's bytes and the sounds alive of each frame from `at`.
fn samples(every: u32, last: u32, at: impl Fn(u32) -> (u64, u64, u32)) -> Vec<Sample> {
    (1..=last / every)
        .map(|i| {
            let frame = i * every;
            let (heap, tag, sounds) = at(frame);
            Sample {
                frame,
                live_heap_bytes: heap,
                tags: vec![("update".to_owned(), 4096), ("grows".to_owned(), tag)],
                sounds_alive: sounds,
                hash: Some(u64::from(frame)),
            }
        })
        .collect()
}

#[test]
fn a_flat_run_passes_and_a_rise_within_the_slack_too() {
    let slack = Slack::default();
    let flat = samples(100, 4000, |_| (1_000_000, 10, 2));
    let verdict = judge(&flat, 500, slack).unwrap();
    assert!(!verdict.heap_grew && !verdict.sounds_grew, "{verdict:?}");
    assert_eq!((verdict.judged, verdict.heap), (35, (1_000_000, 1_000_000)));
    assert_eq!(verdict.grew_most, []);
    // Up to the slack exactly passes; one byte more fails.
    let edge = |extra: u64| {
        samples(100, 4000, move |frame| {
            (1_000_000 + if frame > 3000 { extra } else { 0 }, 0, 0)
        })
    };
    assert!(!judge(&edge(65_536), 0, slack).unwrap().heap_grew);
    let over = judge(&edge(65_537), 0, slack).unwrap();
    assert!(over.heap_grew);
    assert_eq!(over.heap, (1_000_000, 1_065_537));
}

#[test]
fn a_rise_only_inside_the_warm_up_passes() {
    // 1 MB allocated once at frame 500 and kept, judged after 1,000 frames.
    let once = samples(100, 10_000, |frame| {
        (if frame >= 500 { 2_048_576 } else { 1_000_000 }, 0, 0)
    });
    let verdict = judge(&once, 1000, Slack::default()).unwrap();
    assert!(!verdict.heap_grew, "{verdict:?}");
    // The same megabyte at frame 9,000, past the warm-up, is a rise.
    let late = samples(100, 10_000, |frame| {
        (if frame >= 9000 { 2_048_576 } else { 1_000_000 }, 0, 0)
    });
    assert!(judge(&late, 1000, Slack::default()).unwrap().heap_grew);
}

#[test]
fn a_leak_names_the_tags_that_grew_most() {
    let leak = samples(100, 4000, |frame| {
        let kept = 1024 * u64::from(frame);
        (1_000_000 + kept, kept, 0)
    });
    let verdict = judge(&leak, 500, Slack::default()).unwrap();
    assert!(verdict.heap_grew);
    // 35 judged, 8 a quarter: frames 600–1,300 and 3,300–4,000.
    assert_eq!(
        verdict.heap,
        (1_000_000 + 1024 * 1300, 1_000_000 + 1024 * 4000)
    );
    assert_eq!(verdict.grew_most, [("grows".to_owned(), 1024 * 2700)]);
}

#[test]
fn sounds_alive_that_rise_fail_and_the_quarters_round_down() {
    // One more every 1,000 frames.
    let rising = samples(250, 20_000, |frame| (1_000_000, 0, frame / 1000));
    let verdict = judge(&rising, 1000, Slack::default()).unwrap();
    assert!(verdict.sounds_grew && !verdict.heap_grew);
    // 76 judged, 19 a quarter: frames 1,250–5,750 and 15,500–20,000.
    assert_eq!(verdict.judged, 76);
    assert_eq!(verdict.sounds, (5, 20));
    assert!(
        !judge(
            &rising,
            1000,
            Slack {
                heap_bytes: 0,
                sounds: 15
            }
        )
        .unwrap()
        .sounds_grew
    );
    // Seven samples: quarters of one, the first and the last.
    let seven = samples(1, 7, |frame| (u64::from(frame), 0, 0));
    let verdict = judge(
        &seven,
        0,
        Slack {
            heap_bytes: 0,
            sounds: 0,
        },
    )
    .unwrap();
    assert_eq!(verdict.heap, (1, 7));
    // Too few judged is said so.
    let e = judge(&seven, 4, Slack::default()).unwrap_err();
    assert!(
        e.starts_with("3 sample(s) after the warm-up of 4 frames"),
        "{e}"
    );
}

#[test]
fn the_first_difference_is_the_first_sample_whose_hash_or_frame_differs() {
    let a = samples(100, 1000, |_| (0, 0, 0));
    assert_eq!(first_difference(&a, &a), None);
    let mut b = a.clone();
    b[3].hash = Some(7);
    b[5].hash = Some(7);
    assert_eq!(first_difference(&a, &b), Some(400));
    let mut c = a.clone();
    c[2].frame = 301;
    assert_eq!(first_difference(&a, &c), Some(300));
    // One run ending first differs where the other goes on.
    assert_eq!(first_difference(&a[..6], &a), Some(700));
    assert_eq!(first_difference(&a, &a[..6]), Some(700));
}

/// A long scenario with `warm_up` 500 and the default slack.
fn scenario() -> LongScenario {
    LongScenario::read(
        "s",
        "(format_version: 1, repeat: 40, idle: 100, warm_up: 500, sample_every: 100)",
    )
    .unwrap()
}

/// What a run of 4,000 frames sampled.
fn played(samples: Vec<Sample>) -> LongPlayed {
    LongPlayed {
        samples,
        frames: 4000,
        counts_heap: true,
        limits: Vec::new(),
    }
}

#[test]
fn the_table_says_each_runs_quarters_and_why_it_fails() {
    let flat = played(samples(100, 4000, |_| (177_783, 10, 0)));
    let leaks = played(samples(100, 4000, |frame| {
        let kept = 1024 * u64::from(frame);
        (1_000_000 + kept, kept, 0)
    }));
    let report = LongReport {
        format_version: FORMAT_VERSION,
        runs: vec![
            run("long/flat.long.ron", &scenario(), flat.clone(), Some(&flat)),
            run("long/leaks.long.ron", &scenario(), leaks, None),
        ],
    };
    assert!(report.failed());
    assert_eq!(report.runs[0].failures, [] as [String; 0]);
    assert_eq!(
        markdown(&report),
        "## Long runs\n\n\
         | Scenario | Frames | Samples | Live heap, first quarter | Live heap, last quarter \
         | Sounds alive | Second run | Result |\n\
         |---|---:|---:|---:|---:|---:|---|---|\n\
         | flat | 4,000 | 35 | 177,783 | 177,783 | 0 → 0 | same hashes | flat |\n\
         | leaks | 4,000 | 35 | 2,331,200 | 5,096,000 | 0 → 0 | not played | **fails** |\n\
         \n### Failures\n\n\
         - leaks: the live heap grew from 2,331,200 bytes (the most of the first quarter) to \
         5,096,000 (the most of the last), over the slack of 65,536: grew most under grows \
         +2,764,800\n"
    );
    let json = serde_json::to_string(&report).unwrap();
    assert_eq!(serde_json::from_str::<LongReport>(&json).unwrap(), report);
}

#[test]
fn a_second_run_that_differs_or_isnt_hashed_fails_naming_it() {
    let first = played(samples(100, 4000, |_| (1, 0, 0)));
    let mut second = first.clone();
    second.samples[0].hash = Some(99);
    let differs = run(
        "long/pid.long.ron",
        &scenario(),
        first.clone(),
        Some(&second),
    );
    assert_eq!((differs.twice, differs.differs_at), (true, Some(100)));
    assert_eq!(
        differs.failures,
        [
            "pid: the second run differs from the first at frame 100: the game isn't \
          deterministic over a long run"
        ]
    );
    let mut unhashed = first;
    unhashed.samples.iter_mut().for_each(|s| s.hash = None);
    let not = run(
        "long/pid.long.ron",
        &scenario(),
        unhashed.clone(),
        Some(&unhashed),
    );
    assert_eq!(
        not.failures,
        ["pid: its frames weren't hashed, so the runs can't be compared"]
    );
}

/// The live bytes under the memory tag `name` now.
fn tag_bytes(name: &str) -> u64 {
    global::rows()
        .unwrap_or_default()
        .iter()
        .filter(|row| row.name == name)
        .map(|row| row.live_bytes)
        .sum()
}

/// The live bytes this thread holds under a tag of its own after 1,000
/// idle frames of the toy, and after 10,000, its logs kept or not.
fn held(keep: bool, name: &'static str) -> (u64, u64) {
    let before = tags::enter(tags::tag(TagClass::Span as u8, name));
    let mut h = Harness::new(toy::game_on(toy::ctx()));
    h.keep_logs(keep);
    for _ in 0..1000 {
        h.frame(&[], FRAME_DT);
    }
    let after_1000 = tag_bytes(name);
    for _ in 0..9000 {
        h.frame(&[], FRAME_DT);
    }
    let after_10_000 = tag_bytes(name);
    drop(h);
    tags::leave(before);
    (after_1000, after_10_000)
}

#[test]
fn a_harness_without_logs_holds_the_same_after_10_000_frames_as_after_1000() {
    if !global::COUNTING {
        return;
    }
    // Under a tag of this thread's own: the other tests' threads allocate
    // under theirs.
    let (at_1000, at_10_000) = held(false, "keep-logs-off");
    assert!(
        at_10_000.abs_diff(at_1000) <= 4096,
        "{at_1000} bytes after 1,000 frames, {at_10_000} after 10,000"
    );
    // With its logs kept, it grows: the test above would see it.
    let (at_1000, at_10_000) = held(true, "keep-logs-on");
    assert!(at_10_000 > at_1000 + 4096, "{at_1000}, then {at_10_000}");
}

/// Plays long scenarios from a folder in this program.
struct InProcess(PathBuf);

impl LongPlayer for InProcess {
    fn play_long(&self, file: &str, dir: &Path) -> Result<LongPlayed, String> {
        fs::create_dir_all(dir).unwrap();
        let out = dir.join("samples.jsonl");
        play(&self.0, file, &ToyTools, &out).map_err(|(_, e)| e)?;
        read_played(&out)
    }
}

/// A folder holding long scenario `name` with `text`, and that scenario
/// read.
fn one(name: &str, text: &str) -> (PathBuf, Vec<(String, LongScenario)>) {
    let root = temp_dir(name);
    let file = format!("{name}{SUFFIX}");
    fs::write(root.join(&file), text).unwrap();
    let scenarios = crate::bench::long_plan(&root, Some(name)).unwrap();
    (root, scenarios)
}

#[test]
fn the_toy_idling_on_its_fake_heap_is_flat_and_plays_the_same_twice() {
    let (root, scenarios) = one(
        "idles",
        "(format_version: 1, repeat: 4, idle: 120, sample_every: 30)",
    );
    let report = measure_long(
        &scenarios,
        &InProcess(root.clone()),
        &root.join("runs"),
        true,
    )
    .unwrap();
    let run = &report.runs[0];
    assert!(run.failures.is_empty(), "{:?}", run.failures);
    assert_eq!((run.frames, run.samples.len()), (480, 16));
    assert_eq!((run.twice, run.differs_at), (true, None));
    assert!(run.samples.iter().all(|s| s.hash.is_some()));
    assert_eq!(
        run.limits,
        ["the heap read is the game's own (a fake, or none): not this program's"]
    );
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn the_toy_starting_loops_fails_naming_the_sounds_alive() {
    let (root, scenarios) = one(
        "hums",
        r#"(format_version: 1, start: [(flag: "--hum-every", value: "50")], repeat: 8,
            idle: 50, sample_every: 30)"#,
    );
    let report = measure_long(
        &scenarios,
        &InProcess(root.clone()),
        &root.join("runs"),
        false,
    )
    .unwrap();
    let run = &report.runs[0];
    assert!(!run.twice);
    let verdict = run.verdict.as_ref().unwrap();
    assert_eq!((verdict.judged, verdict.sounds), (13, (1, 7)));
    assert_eq!(
        run.failures,
        [
            "hums: the sounds alive grew from 1 (the most of the first quarter) to 7 (the most \
          of the last), over the slack of 0"
        ]
    );
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn the_toy_keeping_bytes_every_frame_fails_naming_its_tag() {
    if !global::COUNTING {
        return;
    }
    // 64 KiB a frame, far over what the other tests' threads move.
    let (root, scenarios) = one(
        "leaks",
        r#"(format_version: 1, start: [(flag: "--leak-bytes", value: "65536")], repeat: 4,
            idle: 100, sample_every: 25)"#,
    );
    let report = measure_long(
        &scenarios,
        &InProcess(root.clone()),
        &root.join("runs"),
        false,
    )
    .unwrap();
    let run = &report.runs[0];
    let leaked = |s: &Sample| {
        s.tags
            .iter()
            .find(|(name, _)| name == "toy-leak")
            .map_or(0, |(_, bytes)| *bytes)
    };
    let (first, last) = (&run.samples[0], &run.samples[run.samples.len() - 1]);
    assert!(
        leaked(last) - leaked(first) >= 65_536 * u64::from(last.frame - first.frame),
        "{first:?} {last:?}"
    );
    // The whole program's heap here moves with the other tests' threads
    // (CI's `bench` job judges it in a process of its own): judged on the
    // toy's tag alone, the run fails naming it.
    let own: Vec<Sample> = run
        .samples
        .iter()
        .map(|s| Sample {
            live_heap_bytes: leaked(s),
            ..s.clone()
        })
        .collect();
    let played = LongPlayed {
        samples: own,
        frames: run.frames,
        counts_heap: true,
        limits: Vec::new(),
    };
    let judged = super::run("leaks.long.ron", &scenarios[0].1, played, None);
    let said = judged.failures.join("\n");
    assert!(
        said.starts_with("leaks: the live heap grew from ") && said.contains("toy-leak +"),
        "{said}"
    );
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn a_long_scenario_named_and_missing_is_a_wrong_argument() {
    let root = temp_dir("none");
    let (code, e) = crate::bench::long_plan(&root, None).unwrap_err();
    assert_eq!(
        (code, e.as_str()),
        (
            codes::FILE,
            "no *.long.ron long scenario in the game's folder"
        )
    );
    let (code, _) = crate::bench::long_plan(&root, Some("nope")).unwrap_err();
    assert_eq!(code, codes::WRONG_ARGUMENTS);
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn every_long_scenario_the_engine_ships_reads() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for folder in ["crates/cli/toy", "examples/hello-grid"] {
        let found = crate::bench::long_plan(&repo.join(folder), None).unwrap();
        assert!(!found.is_empty(), "{folder}");
    }
}
