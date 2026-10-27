//! The toy game's tools, as its tool program (the `toy` example) runs
//! them on `toy/shuyi.ron`, and `run --perf` on its fake clock.

use std::fs;
use std::path::Path;

use shuyi_cli::codec::decode_png;
use shuyi_cli::contract::perf;
use shuyi_cli::contract::run::{self as run_op, RunInput, RunOutput};
use shuyi_cli::toy::ToyTools;
use shuyi_cli::{Project, run};
use shuyi_ui::perf::PerfLine;

use crate::{args, scratch};

/// The toy's project, with its root moved to `root` (where pictures go).
fn toy_project(root: &Path) -> Project {
    let toy = Path::new(env!("CARGO_MANIFEST_DIR")).join("toy");
    let mut project = Project::load(&toy).unwrap_or_else(|e| panic!("{e}"));
    project.root = root.to_path_buf();
    project
}

#[test]
fn the_toys_project_names_its_tool_program() {
    let project = toy_project(Path::new("."));
    let game = project.config.game.unwrap();
    assert_eq!(
        game.cargo_args("frame-png", &[]).join(" "),
        "run --quiet -p shuyi-cli --example toy --features toy -- frame-png"
    );
}

#[test]
fn frame_png_renders_the_toy() {
    let dir = scratch("toy-frame");
    let project = toy_project(&dir);
    let picture = |extra: &[&str]| {
        let mut all = vec!["frame-png", "toy.png", "--scale", "1"];
        all.extend_from_slice(extra);
        assert_eq!(run(&project, &args(&all), Some(&ToyTools)), 0, "{extra:?}");
        decode_png(&fs::read(dir.join("toy.png")).unwrap()).unwrap()
    };
    let (width, height, home) = picture(&[]);
    assert!(width > 0 && height > 0);
    // Something is drawn on the clear colour: the toy's text, as boxes.
    let first = &home[..4];
    assert!(home.chunks(4).any(|p| p != first));
    // Its own option and a step change the picture.
    let (_, _, lobby) = picture(&["--lobby"]);
    assert_ne!(lobby, home);
    let (_, _, moved) = picture(&["--lobby", "--keys", "Down"]);
    assert_ne!(moved, lobby);
    // Help and bad arguments.
    assert_eq!(
        run(&project, &args(&["frame-png", "--help"]), Some(&ToyTools)),
        0
    );
    let bad = args(&["frame-png", "t.png", "--light", "dusk"]);
    assert_eq!(run(&project, &bad, Some(&ToyTools)), 2);
    // An unknown target exits as SH3003 does, as `check` and `bake` do.
    let moon = args(&["frame-png", "t.png", "--target", "moon"]);
    assert_eq!(
        run(&project, &moon, Some(&ToyTools)),
        shuyi_cli::contract::codes::exit_code(Some("SH3003"))
    );
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn lang_status_reports_the_toys_debug_pack() {
    let project = toy_project(Path::new("."));
    assert_eq!(
        run(&project, &args(&["lang-status", "dbg"]), Some(&ToyTools)),
        0
    );
    assert_eq!(
        run(&project, &args(&["lang-status", "zz"]), Some(&ToyTools)),
        2
    );
    assert_eq!(run(&project, &args(&["lang-status"]), Some(&ToyTools)), 2);
    let report = shuyi_cli::lang_status::report(
        &shuyi_cli::GameTools::lang(&ToyTools).unwrap().lang,
        &[] as &[&str],
        "dbg",
        &shuyi_cli::lang_status::Options::default(),
    )
    .unwrap();
    assert!(
        report.starts_with("lang-status dbg (Debug, machine-made): "),
        "{report}"
    );
}

/// `run` on the toy with `perf` on, `frames` idle frames, the trace
/// written to `perf_out` (from `dir`) if given.
fn run_with_perf(
    dir: &Path,
    frames: u32,
    perf_out: Option<&str>,
) -> shuyi_cli::contract::Answer<RunOutput> {
    let mut input = RunInput::default();
    input.frames = frames;
    input.perf = true;
    input.perf_out = perf_out.map(str::to_owned);
    run_op::run_at(&ToyTools, dir, &input)
}

#[test]
fn run_with_perf_on_the_toys_fake_clock_answers_the_same_summary_and_trace() {
    let dir = scratch("toy-perf");
    let answer = run_with_perf(&dir, 5, Some("traces/perf.jsonl"));
    assert!(answer.ok, "{answer:?}");
    let output = answer.result.clone().unwrap();
    let mut summary = output.perf.clone().unwrap();
    let file = summary.file.take().unwrap();
    let written = std::path::absolute(dir.join("traces/perf.jsonl")).unwrap();
    assert_eq!(Path::new(&file), written);
    insta::assert_snapshot!(
        "toy_perf_summary",
        serde_json::to_string_pretty(&summary).unwrap()
    );
    // The fake memory's snapshots: before the script (frame 0) and after
    // the last frame, `update` grown by 16 KB a frame.
    let growth = summary.heap_growth.as_ref().unwrap();
    assert_eq!((growth.from, growth.to, growth.live_bytes), (0, 5, 81_920));
    assert_eq!(summary.heap_tags.len(), 4);
    let trace = fs::read_to_string(&file).unwrap();
    let kinds: Vec<String> = trace
        .lines()
        .map(|line| match serde_json::from_str(line).unwrap() {
            PerfLine::Frame(record) => format!("frame {}", record.frame),
            PerfLine::Snapshot(snapshot) => format!("snapshot {}", snapshot.frame),
            PerfLine::Diff(diff) => format!("diff {} {}", diff.from, diff.to),
            other => panic!("{other:?}"),
        })
        .collect();
    assert_eq!(
        kinds,
        [
            "frame 1",
            "frame 2",
            "frame 3",
            "frame 4",
            "frame 5",
            "snapshot 0",
            "snapshot 5",
            "diff 0 5"
        ]
    );
    insta::assert_snapshot!("toy_perf_trace", trace);
    let text = run_op::text(&output);
    assert!(
        text.contains("heap grew 80.0 KB from frame 0 to 5: update +80.0 KB\n"),
        "{text}"
    );
    assert!(answer.limits.contains(&perf::TIMES.to_owned()));
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_plain_run_answers_no_perf_and_no_limit_of_it() {
    let dir = scratch("toy-plain");
    let mut input = RunInput::default();
    input.frames = 1;
    let plain = run_op::run_at(&ToyTools, &dir, &input);
    assert!(plain.ok, "{plain:?}");
    assert_eq!(plain.result.as_ref().unwrap().perf, None);
    let json = serde_json::to_value(&plain).unwrap();
    assert!(json["result"]["perf"].is_null(), "{json}");
    let measured = run_with_perf(&dir, 1, None);
    let mut limits = measured.limits.clone();
    limits.retain(|limit| limit != perf::TIMES);
    assert_eq!(limits, plain.limits);
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_run_of_700_frames_keeps_600_and_says_100_were_dropped() {
    let dir = scratch("toy-perf-700");
    let answer = run_with_perf(&dir, 700, None);
    let summary = answer.result.unwrap().perf.unwrap();
    assert_eq!((summary.frames, summary.dropped), (600, 100));
    assert!(
        answer.limits.contains(&perf::dropped_limit(100)),
        "{:?}",
        answer.limits
    );
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_trace_file_that_cant_be_written_fails_with_the_file_code() {
    let dir = scratch("toy-perf-unwritable");
    fs::write(dir.join("taken"), "a file, not a folder").unwrap();
    let answer = run_with_perf(&dir, 1, Some("taken/perf.jsonl"));
    assert!(!answer.ok);
    assert_eq!(answer.code.as_deref(), Some("SH3004"));
    assert!(
        answer
            .error
            .unwrap()
            .starts_with("--perf-out taken/perf.jsonl: "),
    );
    fs::remove_dir_all(&dir).unwrap();
}
