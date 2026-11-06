//! `bench`: its scenarios, Callgrind's dumps, the budgets' comparison and
//! the report, and the toy's fixtures played in this program (without
//! Valgrind: CI's `bench` job counts their instructions).

use std::path::{Path, PathBuf};

use super::report::{self, Budget, FrameCounts, Over, Package, Peak, Spread, What};
use super::*;
use crate::toy::ToyTools;

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("shuyi-bench-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn args(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| (*s).to_owned()).collect()
}

/// The toy's project folder, with its fixtures under `bench/`.
fn toy() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("toy")
}

const SCENARIO: &str = r#"(
    format_version: 1,
    about: "the cursor sweeping the map",
    start: [(flag: "--stage"), (flag: "--seed", value: "3")],
    script: (steps: [keys("Right Right"), wait(0.5)]),
    frames: 30,
    measure: Frames(first: 10, last: 40),
)"#;

#[test]
fn a_scenario_reads_with_its_start_steps_and_frames() {
    let scenario = Scenario::read("a.bench.ron", SCENARIO).unwrap();
    assert_eq!(scenario.about, "the cursor sweeping the map");
    assert_eq!(scenario.start[1].value.as_deref(), Some("3"));
    assert_eq!(scenario.script.steps.len(), 2);
    assert_eq!(scenario.frames, 30);
    assert_eq!(
        scenario.measure,
        Measure::Frames {
            first: 10,
            last: 40
        }
    );
    assert!(!scenario.measure.has(9) && scenario.measure.has(10) && scenario.measure.has(40));
    let least = Scenario::read("b.bench.ron", "(format_version: 1)").unwrap();
    assert_eq!(least.measure, Measure::Every);
    assert!(least.measure.has(1) && least.script.steps.is_empty());
}

#[test]
fn a_scenario_that_doesnt_read_names_the_file_line_and_column() {
    let err = |text: &str| Scenario::read("bench/a.bench.ron", text).unwrap_err();
    let broken = err("(\n    format_version: 1,\n    frames: \"many\",\n)");
    assert!(broken.starts_with("bench/a.bench.ron:3:"), "{broken}");
    assert_eq!(
        err("(format_version: 2)"),
        "bench/a.bench.ron: format_version 2: this shuyi reads 1"
    );
    let unknown = err("(format_version: 1, seconds: 3)");
    assert!(unknown.starts_with("bench/a.bench.ron:1:"), "{unknown}");
    let key = err(r#"(format_version: 1, script: (steps: [keys("NoSuchKey")]))"#);
    assert!(
        key.starts_with("bench/a.bench.ron: script: step 1: keys:"),
        "{key}"
    );
    for range in ["Frames(first: 0, last: 3)", "Frames(first: 5, last: 4)"] {
        let text = format!("(format_version: 1, measure: {range})");
        assert!(err(&text).contains("measure: frames"), "{range}");
    }
}

#[test]
fn scenarios_are_found_by_name_outside_built_and_hidden_folders() {
    let dir = temp_dir("find");
    for file in [
        "idle.bench.ron",
        "bench/menu.bench.ron",
        "bench/deep/bot.bench.ron",
        "target/x.bench.ron",
        "dist/y.bench.ron",
        ".git/z.bench.ron",
        "bench/notes.ron",
    ] {
        let path = dir.join(file);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, "").unwrap();
    }
    assert_eq!(
        find(&dir),
        [
            "bench/deep/bot.bench.ron",
            "bench/menu.bench.ron",
            "idle.bench.ron"
        ]
    );
    assert_eq!(name_of("bench/deep/bot.bench.ron"), "bot");
    assert_eq!(name_of("idle.bench.ron"), "idle");
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn bench_reads_its_arguments() {
    let options = parse_args(&args(&[])).unwrap();
    assert_eq!(
        options,
        Options {
            target: None,
            scenario: None,
            out: "dist/bench".to_owned(),
            instructions: true,
        }
    );
    let given = args(&[
        "--target",
        "playdate",
        "--scenario",
        "idle",
        "--out",
        "o",
        "--no-instructions",
    ]);
    let options = parse_args(&given).unwrap();
    assert_eq!(options.target.as_deref(), Some("playdate"));
    assert_eq!(options.scenario.as_deref(), Some("idle"));
    assert_eq!((options.out.as_str(), options.instructions), ("o", false));
    assert_eq!(
        parse_args(&args(&["--target"])).unwrap_err(),
        "--target needs a value"
    );
    assert_eq!(
        parse_args(&args(&["--fast"])).unwrap_err(),
        "unknown argument: --fast"
    );
    assert!(usage("cargo shuyi").starts_with("usage: cargo shuyi bench"));
    let (file, target, out) = parse_play(&args(&["a.bench.ron", "--out", "p.json"])).unwrap();
    assert_eq!(
        (file.as_str(), target, out),
        ("a.bench.ron", None, "p.json".into())
    );
    assert!(parse_play(&args(&["a.bench.ron"])).is_err());
    assert!(parse_play(&args(&["a", "b", "--out", "p"])).is_err());
}

#[test]
fn the_program_built_is_cargos_last_executable() {
    let stdout = r#"{"reason":"compiler-artifact","executable":null,"target":{"name":"dep"}}
not json
{"reason":"compiler-artifact","executable":"/t/shuyi-bench/examples/tools","target":{"name":"tools"}}
{"reason":"build-finished","success":true}"#;
    assert_eq!(
        executable(stdout),
        Some(PathBuf::from("/t/shuyi-bench/examples/tools"))
    );
    assert_eq!(executable(r#"{"reason":"build-finished"}"#), None);
}

/// A Callgrind dump of `part`, taken at a frame's end or the program's,
/// with `instructions`.
fn dump(part: u32, at_exit: bool, instructions: u64) -> String {
    let trigger = if at_exit {
        "Program termination".to_owned()
    } else {
        format!("--dump-before={}", callgrind::MARKER)
    };
    format!(
        "version: 1\ncreator: callgrind-3.22.0\npid: 7\ncmd: tools bench-play\npart: {part}\n\n\
         desc: I1 cache:\ndesc: Timerange: Basic block 0 - 99\ndesc: Trigger: {trigger}\n\n\
         positions: line\nevents: Ir\nsummary: {instructions}\n\nfn=main\n0 {instructions}\n\n\
         totals: {instructions}\n"
    )
}

#[test]
fn callgrinds_dumps_are_read_in_order_without_the_programs_end() {
    let parsed = callgrind::parse("callgrind.out.2", &dump(2, false, 1_234)).unwrap();
    assert_eq!(
        parsed,
        callgrind::Dump {
            part: 2,
            at_exit: false,
            instructions: 1_234
        }
    );
    assert!(
        callgrind::parse("x", "part: 1\n")
            .unwrap_err()
            .contains("no instruction total")
    );
    let dir = temp_dir("dumps");
    for (part, at_exit, n) in [
        (3, false, 30),
        (1, false, 900),
        (4, true, 5),
        (2, false, 20),
    ] {
        fs::write(
            dir.join(format!("callgrind.out.{part}")),
            dump(part, at_exit, n),
        )
        .unwrap();
    }
    fs::write(dir.join("played.json"), "{}").unwrap();
    assert_eq!(callgrind::read(&dir).unwrap(), [900, 20, 30]);
    let empty = temp_dir("no-dumps");
    assert!(
        callgrind::read(&empty)
            .unwrap_err()
            .contains("process_heap")
    );
    let given: Vec<String> = callgrind::args(&dir)
        .iter()
        .map(|a| a.to_string_lossy().into_owned())
        .collect();
    assert!(given.contains(&"--tool=callgrind".to_owned()), "{given:?}");
    assert!(
        given.iter().any(|a| a.starts_with("--dump-before=*")),
        "{given:?}"
    );
    fs::remove_dir_all(&dir).unwrap();
    fs::remove_dir_all(&empty).unwrap();
}

fn played(frames: &[(u32, u64)]) -> Played {
    Played {
        frames: frames
            .iter()
            .map(|&(frame, peak)| PlayedFrame {
                frame,
                peak_heap_bytes: Some(peak),
            })
            .collect(),
        counts_heap: true,
        limits: vec!["a limit".to_owned()],
    }
}

#[test]
fn each_frame_gets_its_dump_after_the_starts() {
    let measured = combine("a", played(&[(1, 10), (2, 12)]), Some(vec![900, 20, 30])).unwrap();
    assert_eq!(measured.start_instructions, Some(900));
    assert_eq!(
        measured.frames,
        [
            FrameCounts {
                frame: 1,
                instructions: Some(20),
                peak_heap_bytes: Some(10)
            },
            FrameCounts {
                frame: 2,
                instructions: Some(30),
                peak_heap_bytes: Some(12)
            },
        ]
    );
    assert_eq!(measured.limits, ["a limit"]);
    let err = combine("a", played(&[(1, 10)]), Some(vec![900, 20, 30])).unwrap_err();
    assert_eq!(
        err,
        "a: Callgrind counted 2 frame(s), and the game recorded 1"
    );
    assert!(combine("a", played(&[]), Some(Vec::new())).is_err());
    let uncounted = combine("a", played(&[(1, 10)]), None).unwrap();
    assert_eq!(uncounted.start_instructions, None);
    assert_eq!(uncounted.frames[0].instructions, None);
}

fn frames(counts: &[(u32, u64, u64)]) -> Vec<FrameCounts> {
    counts
        .iter()
        .map(|&(frame, n, peak)| FrameCounts {
            frame,
            instructions: Some(n),
            peak_heap_bytes: Some(peak),
        })
        .collect()
}

#[test]
fn the_spread_is_the_most_its_first_frame_the_p99_and_the_mean() {
    let mut counts: Vec<(u32, u64, u64)> = (1..=100).map(|f| (f, u64::from(f), 0)).collect();
    counts[49].1 = 1_000;
    counts.push((101, 1_000, 0));
    let spread = report::spread(&frames(&counts)).unwrap();
    assert_eq!((spread.most, spread.frame), (1_000, 50));
    // 101 counts: the 100th smallest is the p99's.
    assert_eq!(spread.p99, 1_000);
    let sum: u64 = counts.iter().map(|c| c.1).sum();
    assert_eq!(spread.mean, sum / 101);
    let one = report::spread(&frames(&[(7, 42, 0)])).unwrap();
    assert_eq!((one.most, one.frame, one.p99, one.mean), (42, 7, 42, 42));
    let uncounted = [FrameCounts {
        frame: 1,
        instructions: None,
        peak_heap_bytes: None,
    }];
    assert_eq!(report::spread(&uncounted), None);
    assert_eq!(report::peak(&uncounted), None);
    let peak = report::peak(&frames(&[(1, 0, 5), (2, 0, 9), (3, 0, 9)])).unwrap();
    assert_eq!(peak, Peak { bytes: 9, frame: 2 });
}

fn budget(target: &str, instructions: u64, heap: u64, package: Option<u64>) -> Budget {
    Budget {
        target: target.to_owned(),
        frame_instructions: Some(instructions),
        peak_heap_bytes: Some(heap),
        package_bytes: package,
    }
}

fn run(scenario: &str, on: &[&str], most: (u32, u64), peak: (u32, u64)) -> Run {
    Run {
        scenario: scenario.to_owned(),
        file: format!("bench/{scenario}.bench.ron"),
        about: String::new(),
        on: on.iter().map(|s| (*s).to_owned()).collect(),
        one_bit: false,
        start_instructions: None,
        instructions: Some(Spread {
            most: most.1,
            frame: most.0,
            p99: most.1,
            mean: most.1,
        }),
        peak_heap: Some(Peak {
            bytes: peak.1,
            frame: peak.0,
        }),
        frames: Vec::new(),
        limits: Vec::new(),
    }
}

const MB: u64 = 1 << 20;

#[test]
fn a_number_over_a_budget_names_the_scenario_frame_target_and_both_numbers() {
    let budgets = [
        budget("desktop", 20_000_000, 512 * MB, None),
        budget("playdate", 2_000_000, 10 * MB, Some(40 * MB)),
    ];
    let runs = [
        run("holds", &["desktop"], (1, 1_000), (1, 11 * MB)),
        run("holds", &["playdate"], (1, 1_000), (1, 11 * MB)),
        run("burns", &["playdate"], (5, 3_000_000), (1, MB)),
        run("burns", &["desktop"], (5, 3_000_000), (1, MB)),
    ];
    let packages = [Package {
        target: "playdate".to_owned(),
        export: "playdate".to_owned(),
        file: "game.pdx.zip".to_owned(),
        bytes: 41 * MB,
    }];
    let over = report::over(&budgets, &runs, &packages);
    let texts: Vec<String> = over.iter().map(Over::text).collect();
    assert_eq!(
        texts,
        [
            "holds on playdate: the heap's peak reached 11.0 MB (11,534,336 bytes) by frame 1, \
             over the budget of 10.0 MB (10,485,760 bytes)",
            "burns on playdate: frame 5 took 3,000,000 instructions, over the budget of 2,000,000",
            "playdate: the package game.pdx.zip is 41.0 MB (42,991,616 bytes), over the budget \
             of 40.0 MB (41,943,040 bytes)",
        ]
    );
    assert_eq!(over[1].what, What::FrameInstructions);
    assert_eq!(
        (over[1].frame, over[1].value, over[1].budget),
        (Some(5), 3_000_000, 2_000_000)
    );
    // A run is never compared with a target it wasn't played for, and a
    // budget a target hasn't isn't one.
    let mut none = budget("switch", 1, 1, None);
    none.frame_instructions = None;
    none.peak_heap_bytes = None;
    assert_eq!(report::over(&[none], &runs, &packages), Vec::<Over>::new());
}

#[test]
fn the_table_has_the_runs_the_budgets_whats_over_and_whats_not_measured() {
    let budgets = vec![budget("playdate", 2_000_000, 10 * MB, Some(40 * MB))];
    let runs = vec![run("burns", &["playdate"], (5, 3_000_000), (2, MB))];
    let mut report = Report {
        format_version: 1,
        over: report::over(&budgets, &runs, &[]),
        targets: budgets,
        runs,
        packages: Vec::new(),
        limits: vec!["playdate: no package measured".to_owned()],
    };
    let table = report::markdown(&report);
    for row in [
        "## shuyi bench",
        "| burns | playdate | 0 | 3,000,000 (5) | 3,000,000 | 3,000,000 | 1.0 MB (2) |",
        "| playdate | 2,000,000 | 10.0 MB | 40.0 MB |",
        "**Over budget:**",
        "- burns on playdate: frame 5 took 3,000,000 instructions",
        "- playdate: no package measured",
    ] {
        assert!(table.contains(row), "{row}\n{table}");
    }
    report.over.clear();
    report.limits.clear();
    report.runs[0].instructions = None;
    let table = report::markdown(&report);
    assert!(
        table.contains("| burns | playdate | 0 | — | — | — |"),
        "{table}"
    );
    assert!(
        table.contains("Every number is within its budget."),
        "{table}"
    );
    assert!(!table.contains("Not measured"), "{table}");
    assert_eq!(report::grouped(0), "0");
    assert_eq!(report::grouped(999), "999");
    assert_eq!(report::grouped(1_000), "1,000");
    assert_eq!(report::grouped(20_000_000), "20,000,000");
    let json = serde_json::to_string(&report).unwrap();
    assert_eq!(serde_json::from_str::<Report>(&json).unwrap(), report);
}

/// Plays the toy's scenarios in this program, uncounted.
struct InProcess;

impl Player for InProcess {
    fn play(&self, file: &str, target: Option<&str>, _dir: &Path) -> Result<Measured, String> {
        let played = play(&toy(), file, target, &ToyTools).map_err(|(_, e)| e)?;
        combine(file, played, None)
    }
}

/// The toy's plan: `scenario` against `targets`' profiles.
fn toy_plan(scenario: &str, targets: &[&str]) -> Plan {
    let project = Project {
        root: toy(),
        config: crate::project::Config::default(),
    };
    let mut plan = Plan {
        targets: Vec::new(),
        scenarios: Vec::new(),
        limits: Vec::new(),
    };
    for id in targets {
        let options = Options {
            target: Some((*id).to_owned()),
            scenario: Some(scenario.to_owned()),
            out: OUT.to_owned(),
            instructions: false,
        };
        let one = super::plan(&project, &options).unwrap();
        plan.targets.extend(one.targets);
        plan.scenarios = one.scenarios;
    }
    plan
}

#[test]
fn the_toy_holding_11_mb_is_over_the_playdates_heap_and_within_a_desktops() {
    let plan = toy_plan("holds-11-mb", &["desktop", "playdate"]);
    assert_eq!(plan.scenarios.len(), 1);
    let runs = temp_dir("runs");
    let report = measure(&plan, &InProcess, &runs, &runs, "shuyi").unwrap();
    // Once for the desktop, once more on the Playdate's console.
    assert_eq!(report.runs.len(), 2);
    assert_eq!(report.runs[0].on, ["desktop"]);
    assert!(report.runs[1].one_bit && report.runs[1].limits.is_empty());
    for run in &report.runs {
        assert_eq!(run.frames.len(), 5, "{run:?}");
        let peak = run.peak_heap.unwrap();
        assert!(peak.bytes >= 11 * MB, "{peak:?}");
        assert_eq!(run.instructions, None);
    }
    let over: Vec<(&str, What)> = report
        .over
        .iter()
        .map(|o| (o.target.as_str(), o.what))
        .collect();
    assert_eq!(over, [("playdate", What::PeakHeap)]);
    assert!(
        report
            .limits
            .iter()
            .any(|l| l.starts_with("playdate: no package measured")),
        "{:?}",
        report.limits
    );
    fs::remove_dir_all(&runs).unwrap();
}

/// Plays the toy's burn fixture, its ten frames, whatever file it is
/// given.
struct Ten;

impl Player for Ten {
    fn play(&self, file: &str, _: Option<&str>, _: &Path) -> Result<Measured, String> {
        let played = play(&toy(), "bench/burns-in-frame-5.bench.ron", None, &ToyTools)
            .map_err(|(_, e)| e)?;
        combine(file, played, None)
    }
}

#[test]
fn a_scenario_plays_each_frame_with_the_heaps_peak_at_its_end() {
    let played = play(&toy(), "bench/burns-in-frame-5.bench.ron", None, &ToyTools).unwrap();
    let numbers: Vec<u32> = played.frames.iter().map(|f| f.frame).collect();
    assert_eq!(numbers, (1..=10).collect::<Vec<_>>());
    assert!(played.counts_heap);
    assert!(played.frames.iter().all(|f| f.peak_heap_bytes.is_some()));
    // A frame it doesn't play is an error, naming what it played.
    let plan = Plan {
        targets: vec![(budget("desktop", 1, 1, None), false)],
        scenarios: vec![(
            "bench/late.bench.ron".to_owned(),
            Scenario::read(
                "x",
                "(format_version: 1, measure: Frames(first: 50, last: 60))",
            )
            .unwrap(),
        )],
        limits: Vec::new(),
    };
    let runs = temp_dir("late");
    let (code, e) = measure(&plan, &Ten, &runs, &runs, "shuyi").unwrap_err();
    assert_eq!(code, codes::GAME_ERROR);
    assert_eq!(
        e,
        "bench/late.bench.ron: no frame it measures was played: it recorded frames 1 to 10"
    );
    fs::remove_dir_all(&runs).unwrap();
}

#[test]
fn a_package_is_read_from_its_export_report_by_its_profile() {
    let root = temp_dir("packages");
    let web = root.join("dist/export/web");
    fs::create_dir_all(&web).unwrap();
    let report = ExportOutput {
        target: "web".to_owned(),
        profile: "web".to_owned(),
        sizes: vec![crate::contract::export::FileSize {
            file: "game-0.1.0-web.zip".to_owned(),
            bytes: 30 * MB,
        }],
        ..ExportOutput::default()
    };
    fs::write(web.join(REPORT), serde_json::to_string(&report).unwrap()).unwrap();
    let budgets = [
        budget("web", 1, 1, Some(25 * MB)),
        budget("desktop", 1, 1, None),
        budget("playdate", 1, 1, Some(40 * MB)),
    ];
    let (found, limits) = packages(&root, &budgets, "cargo shuyi");
    assert_eq!(
        found,
        [Package {
            target: "web".to_owned(),
            export: "web".to_owned(),
            file: "game-0.1.0-web.zip".to_owned(),
            bytes: 30 * MB,
        }]
    );
    assert_eq!(
        limits,
        [
            "playdate: no package measured: no dist/export/*/export-report.json is for it \
          (`cargo shuyi export <target>` makes one)"
        ]
    );
    let over = report::over(&budgets, &[], &found);
    assert_eq!(over[0].what, What::Package);
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn a_target_the_game_lists_but_doesnt_support_yet_isnt_gated() {
    let project = Project {
        root: toy(),
        config: crate::project::Config::parse(
            "(targets: {\"desktop\": (frame: (8, 8), scale: integer, text_px: 8), \
             \"switch\": (frame: (8, 8), scale: integer, text_px: 8)})",
        )
        .unwrap(),
    };
    let options = parse_args(&[]).unwrap();
    let plan = plan(&project, &options).unwrap();
    let ids: Vec<&str> = plan
        .targets
        .iter()
        .map(|(b, _)| b.target.as_str())
        .collect();
    assert_eq!(ids, ["desktop"]);
    assert_eq!(plan.limits, ["switch isn't supported yet: not gated"]);
    assert_eq!(plan.scenarios.len(), 2);
    let none = Project {
        root: toy(),
        config: crate::project::Config::default(),
    };
    let (code, _) = super::plan(&none, &options).unwrap_err();
    assert_eq!(code, codes::PROJECT_FILE);
    let missing = Options {
        target: Some("desktop".to_owned()),
        scenario: Some("nope".to_owned()),
        ..options
    };
    let (code, e) = super::plan(&none, &missing).unwrap_err();
    assert_eq!(
        (code, e.as_str()),
        (
            codes::WRONG_ARGUMENTS,
            "no scenario nope.bench.ron in the game's folder"
        )
    );
}

#[test]
fn every_scenario_the_engine_ships_reads() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for folder in ["examples/hello-grid", "crates/cli/toy"] {
        let dir = root.join(folder);
        let files = find(&dir);
        assert!(!files.is_empty(), "{folder}");
        for file in files {
            let text = fs::read_to_string(dir.join(&file)).unwrap();
            Scenario::read(&file, &text).unwrap_or_else(|e| panic!("{folder}/{e}"));
        }
    }
}
