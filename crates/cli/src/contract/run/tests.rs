//! `run` and `scene` on a made-up game.

use super::*;
use crate::contract::fake::{Fake, FakeSession, OPTIONS, stamp};
use crate::game::GameArg;
use schemars::JsonSchema;
use serde::Serialize;
use shuyi_core::ContentHash;
use shuyi_platform::session::SessionRecord;
use shuyi_ui::perf::{HeapCounts, PerfLine, TagClass, TagRow};

fn args(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| (*s).to_owned()).collect()
}

#[test]
fn an_input_the_harness_cant_play_is_refused() {
    assert!(RunInput::default().check().is_ok());
    let most = RunInput {
        frames: MAX_IDLE_FRAMES,
        ..RunInput::default()
    };
    assert!(most.check().is_ok());
    let too_many = RunInput {
        frames: MAX_IDLE_FRAMES + 1,
        ..RunInput::default()
    };
    assert_eq!(
        too_many.check().unwrap_err(),
        "frames: at most 100000, not 100001"
    );
    let bad = RunInput {
        script: keys("NoSuchKey"),
        ..RunInput::default()
    };
    assert!(bad.check().unwrap_err().starts_with("step 1: keys: "));
    let answer = run(&Fake::default(), &too_many);
    assert_eq!(
        answer.error.as_deref(),
        Some("frames: at most 100000, not 100001")
    );
    assert!(scene(&Fake::default(), &bad).error.is_some());
}

#[test]
fn an_action_the_game_cant_press_is_refused_before_anything_plays() {
    let input = |name: &str| RunInput {
        script: Script::new(vec![Step::Keys("Enter".into()), Step::Act(name.to_owned())]),
        ..RunInput::default()
    };
    let ran = run(&Fake::default(), &input("Confirm"));
    assert!(ran.ok, "{ran:?}");
    // The key's two frames and the action's.
    assert_eq!(ran.result.unwrap().frames, 4);
    let said = "step 2: act: no action `Jump` with a key or a button; the game's are Up, Down, \
                Left, Right, Confirm, Cancel";
    let refused = run(&Fake::default(), &input("Jump"));
    assert_eq!(refused.code.as_deref(), Some(codes::WRONG_ARGUMENTS.0));
    assert_eq!(refused.error.as_deref(), Some(said));
    let refused = scene(&Fake::default(), &input("Jump"));
    assert_eq!(refused.error.as_deref(), Some(said));
}

#[test]
fn the_limits_say_what_a_run_could_not_see() {
    let found = limits_of(10, None, true, None);
    assert!(found.is_empty(), "{found:?}");
    assert_eq!(
        limits_of(10, Some(4), true, None),
        ["the game asked to quit on frame 4: the 6 frame(s) after it changed nothing"]
    );
    assert_eq!(
        limits_of(4, Some(4), true, None),
        ["the game asked to quit on frame 4, the last"]
    );
    assert_eq!(
        limits_of(4, None, false, Some("no stage")),
        [
            "no screen is open: the frame is the last one drawn",
            "no scene: no stage"
        ]
    );
}

#[test]
fn run_reports_the_session_after_its_script_and_frames() {
    let input = RunInput {
        script: keys("Enter Up"),
        frames: 3,
        ..RunInput::default()
    };
    let answer = run(&Fake::default(), &input);
    assert!(answer.ok, "{answer:?}");
    assert_eq!(answer.op, Op::Run);
    let out = answer.result.unwrap();
    assert_eq!(out.frames, 7);
    assert_eq!((out.top_screen.as_str(), out.screens.len()), ("stage", 1));
    assert_eq!(out.quit, None);
    assert_eq!(out.scene.unwrap().text, "map");
    assert!(out.frame.starts_with("AB\n"));
    assert_eq!(out.events.len(), 1);
    assert_eq!(out.sounds[0].cue, "click");
    assert!(answer.limits.is_empty(), "{:?}", answer.limits);
}

#[test]
fn a_game_with_no_session_cant_run() {
    let game = Fake {
        session: Err("no session".to_owned()),
        ..Fake::default()
    };
    assert_eq!(
        run(&game, &RunInput::default()).error.as_deref(),
        Some("no session")
    );
    assert_eq!(
        scene(&game, &RunInput::default()).error.as_deref(),
        Some("no session")
    );
}

#[test]
fn scene_passes_only_with_a_scene() {
    let answer = scene(&Fake::default(), &RunInput::default());
    assert!(answer.ok);
    assert_eq!(answer.op, Op::Scene);
    let out = answer.result.unwrap();
    assert_eq!((out.frames, out.top_screen.as_str()), (0, "stage"));
    assert_eq!(out.scene.unwrap().text, "map");
    let mut empty = FakeSession::on("title", "");
    empty.scene = Err("no stage".to_owned());
    empty.screens.clear();
    empty.quit = Some(0);
    let game = Fake {
        session: Ok(empty),
        ..Fake::default()
    };
    let answer = scene(&game, &RunInput::default());
    assert!(!answer.ok);
    let out = answer.result.as_ref().unwrap();
    assert_eq!(out.top_screen, "");
    assert!(out.scene.is_none());
    assert_eq!(answer.limits.len(), 3, "{:?}", answer.limits);
    let ran = run(&game, &RunInput::default());
    assert!(ran.ok);
    assert_eq!(ran.result.unwrap().top_screen, "");
}

#[test]
fn the_arguments_are_steps_frames_and_the_games_options() {
    let input = parse_args(
        &args(&[
            "--script", "Enter", "--keys", "Up", "--pad", "South", "--act", "Confirm", "--wait",
            "0.5", "--frames", "30", "--seed", "4",
        ]),
        &OPTIONS,
    )
    .unwrap();
    assert_eq!(
        input.script.steps,
        [
            Step::Keys("Enter".into()),
            Step::Keys("Up".into()),
            Step::Pad("South".into()),
            Step::Act("Confirm".into()),
            Step::Wait(0.5),
        ]
    );
    assert_eq!(input.frames, 30);
    assert_eq!(
        input.start,
        [GameArg {
            flag: "--seed".into(),
            value: Some("4".into())
        }]
    );
    let flag = [GameOption {
        flag: "--hard",
        value: None,
        help: "",
    }];
    let plain = parse_args(&args(&["--hard"]), &flag).unwrap();
    assert_eq!(plain.start[0].value, None);
    assert_eq!(parse_args(&[], &[]).unwrap(), RunInput::default());
    assert_eq!(
        parse_args(&args(&["--frames", "100000"]), &[])
            .unwrap()
            .frames,
        MAX_IDLE_FRAMES
    );
    for bad in [
        &["--script"][..],
        &["--keys", "Nope"],
        &["--wait", "soon"],
        &["--act", ""],
        &["--act"],
        &["--frames"],
        &["--frames", "100001"],
        &["--frames", "x"],
        &["--seed"],
        &["--bogus"],
        &["Enter"],
    ] {
        assert!(parse_args(&args(bad), &OPTIONS).is_err(), "{bad:?}");
    }
    assert_eq!(
        parse_args(&args(&["--bogus"]), &OPTIONS).unwrap_err(),
        "unknown option: --bogus"
    );
}

#[test]
fn the_usage_lists_the_steps_and_the_games_options() {
    let usage = usage("tools", Op::Scene, &OPTIONS);
    assert!(usage.starts_with("usage: tools scene [steps] [--frames N] [options] [--json]\n"));
    assert!(usage.contains("--script \"Enter Up\""));
    assert!(usage.contains("--act Confirm"), "{usage}");
    assert!(
        usage.contains("\nthe game's options:\n  --seed N             seed it\n"),
        "{usage}"
    );
    assert!(!super::usage("t", Op::Run, &[]).contains("the game's options"));
}

#[test]
fn every_event_reads_as_a_line() {
    let cases = [
        (
            Event::Opened {
                frame: 1,
                screen: "a".into(),
            },
            "frame 1: opened a",
        ),
        (
            Event::Closed {
                frame: 2,
                screen: "a".into(),
            },
            "frame 2: closed a",
        ),
        (
            Event::Music {
                frame: 3,
                command: "start".into(),
                cue: "theme".into(),
            },
            "frame 3: music start theme",
        ),
        (
            Event::Voice {
                frame: 4,
                line: "intro.1".into(),
            },
            "frame 4: voice intro.1",
        ),
        (Event::Quit { frame: 5 }, "frame 5: quit"),
        (
            Event::Game {
                text: "Moved".into(),
            },
            "game: Moved",
        ),
    ];
    for (event, line) in cases {
        assert_eq!(event_text(&event), line);
    }
}

#[test]
fn what_run_and_scene_saw_reads_for_people() {
    let input = RunInput {
        script: keys("Enter"),
        ..RunInput::default()
    };
    let out = run(&Fake::default(), &input).result.unwrap();
    let said = text(&out);
    assert!(
        said.starts_with("run: 2 frame(s); top screen: stage (stage)\n"),
        "{said}"
    );
    assert!(
        said.contains("events:\n  frame 1: opened stage\n"),
        "{said}"
    );
    assert!(said.contains("sounds: click (frame 1)\n"), "{said}");
    assert!(
        said.contains("scene (grid map, on stage):\nmap\n{\"turn\":1}\n"),
        "{said}"
    );
    assert!(
        said.ends_with("frame (its colours with --json):\nAB\n"),
        "{said}"
    );
    let bare = RunOutput {
        frames: 0,
        top_screen: String::new(),
        screens: Vec::new(),
        quit: None,
        scene: None,
        frame: "XY".into(),
        events: Vec::new(),
        sounds: Vec::new(),
        sounds_added: Vec::new(),
        log: Vec::new(),
        modules: Vec::new(),
        perf: None,
        session: None,
    };
    assert_eq!(
        text(&bare),
        "run: 0 frame(s); top screen: none ()\nframe (its colours with --json):\nXY\n"
    );
    let shown = SceneOutput {
        frames: 3,
        top_screen: "stage".into(),
        scene: Some(Scene {
            screen: "stage".into(),
            kind: "grid map".into(),
            text: "map\n".into(),
            data: serde_json::Value::Null,
            shape: None,
        }),
        log: Vec::new(),
        session: None,
        value: None,
        shape: None,
    };
    assert_eq!(
        scene_output_text(&shown),
        "scene: after 3 frame(s), top screen stage\nscene (grid map, on stage):\nmap\n"
    );
}

fn entry(frame: u32, level: &str, text: &str) -> LogEntry {
    LogEntry {
        frame,
        level: level.into(),
        category: "storage".into(),
        text: text.into(),
    }
}

/// A game whose session logged a warning and an info line, and dropped
/// `dropped` older ones.
fn logged(dropped: u32) -> Fake {
    let mut session = FakeSession::on("stage", "map");
    session.log = vec![
        entry(0, "warn", "settings: unreadable"),
        entry(2, "info", "hi"),
    ];
    session.log_dropped = dropped;
    Fake {
        session: Ok(session),
        ..Fake::default()
    }
}

#[test]
fn run_and_scene_answer_with_the_log() {
    let game = logged(0);
    let ran = run(&game, &RunInput::default());
    assert_eq!(ran.result.as_ref().unwrap().log.len(), 2);
    assert!(ran.limits.is_empty(), "{:?}", ran.limits);
    let json = serde_json::to_string(&ran).unwrap();
    assert_eq!(
        serde_json::from_str::<Answer<RunOutput>>(&json).unwrap(),
        ran
    );
    let seen = scene(&game, &RunInput::default());
    assert_eq!(seen.result.as_ref().unwrap().log.len(), 2);
    let json = serde_json::to_string(&seen).unwrap();
    assert_eq!(
        serde_json::from_str::<Answer<SceneOutput>>(&json).unwrap(),
        seen
    );
}

#[test]
fn dropped_lines_are_a_limit() {
    let limit = "the log keeps the last 500 lines: 3 earlier line(s) were dropped";
    assert_eq!(run(&logged(3), &RunInput::default()).limits, [limit]);
    assert_eq!(scene(&logged(3), &RunInput::default()).limits, [limit]);
}

#[test]
fn the_text_shows_the_logs_warnings_and_errors() {
    let game = logged(0);
    let warn = "log (warnings and errors; every line with --json):\n  \
                frame 0: warn storage: settings: unreadable\n";
    let said = text(&run(&game, &RunInput::default()).result.unwrap());
    assert!(
        said.contains(&format!("frame 1: opened stage\n{warn}")),
        "{said}"
    );
    assert!(!said.contains("hi"), "{said}");
    let said = scene_output_text(&scene(&game, &RunInput::default()).result.unwrap());
    assert!(
        said.starts_with(&format!(
            "scene: after 0 frame(s), top screen stage\n{warn}"
        )),
        "{said}"
    );
}

#[test]
fn run_lists_the_games_modules_with_their_hooks() {
    use shuyi_ui::module::{Hook, InstalledModule};

    let mut clock = InstalledModule::new("clock");
    clock.hooks = vec![Hook::BeforeUpdate, Hook::DrawOver];
    clock.states = vec!["clock::Ticks"];
    clock.debug_entries = vec!["Reset clock".into()];
    let mut session = FakeSession::on("stage", "map");
    session.modules = vec![ModuleEntry::from(&clock)];
    let game = Fake {
        session: Ok(session),
        ..Fake::default()
    };
    let out = run(&game, &RunInput::default()).result.unwrap();
    let json = serde_json::to_value(&out).unwrap();
    assert_eq!(
        json["modules"],
        serde_json::json!([{
            "name": "clock",
            "hooks": ["before_update", "draw_over"],
            "states": ["clock::Ticks"],
            "debug_entries": ["Reset clock"],
        }])
    );
    let said = text(&out);
    assert!(
        said.contains("modules: clock (before_update, draw_over)\n"),
        "{said}"
    );
    let none = run(&Fake::default(), &RunInput::default()).result.unwrap();
    assert!(none.modules.is_empty(), "{:?}", none.modules);
    assert!(!text(&none).contains("modules:"));
}

#[test]
fn perf_and_perf_out_are_read_and_perf_out_implies_perf() {
    let input = parse_args(&args(&["--perf"]), &OPTIONS).unwrap();
    assert!(input.perf && input.perf_out.is_none());
    let input = parse_args(&args(&["--perf-out", "perf.jsonl"]), &OPTIONS).unwrap();
    assert!(input.perf);
    assert_eq!(input.perf_out.as_deref(), Some("perf.jsonl"));
    assert!(input.measures());
    assert!(!RunInput::default().measures());
    let missing = parse_args(&args(&["--perf-out"]), &OPTIONS).unwrap_err();
    assert_eq!(missing, "--perf-out needs a value");
    assert!(usage("shuyi", Op::Run, &[]).contains("--perf-out <file>"));
    assert!(!usage("shuyi", Op::Scene, &[]).contains("--perf"));
}

#[test]
fn perf_outs_path_is_from_the_projects_folder() {
    let root = std::env::temp_dir().join("game");
    let path = perf_out_path(&root, "traces/perf.jsonl");
    assert!(path.is_absolute());
    assert_eq!(path, root.join("traces").join("perf.jsonl"));
    let elsewhere = std::env::temp_dir().join("elsewhere.jsonl");
    let text = elsewhere.display().to_string();
    assert_eq!(perf_out_path(&root, &text), elsewhere);
}

/// A fake game whose session measured `frames` frames of 1.2 ms.
fn measured(frames: u32) -> Fake {
    let mut session = FakeSession::on("stage", "map");
    session.perf = (1..=frames)
        .map(|frame| {
            let mut record = FrameRecord::new(frame);
            record.times.frame_us = 1200;
            record
        })
        .collect();
    Fake {
        session: Ok(session),
        ..Fake::default()
    }
}

#[test]
fn run_with_perf_sums_up_the_heap_where_it_was_counted() {
    let mut game = measured(2);
    if let Ok(session) = &mut game.session {
        for (n, record) in (1..).zip(&mut session.perf) {
            record.heap = Some(HeapCounts::new(1000 * n, 3_500_000, 4));
        }
    }
    let input = RunInput {
        perf: true,
        ..RunInput::default()
    };
    let answer = run(&game, &input);
    let output = answer.result.unwrap();
    let heap = output.perf.as_ref().unwrap().heap.as_ref().unwrap();
    assert_eq!((heap.peak_bytes, heap.live_bytes), (3_500_000, 2000));
    // A fake's counts: no limit of the heap's counts, but none by tag.
    assert_eq!(answer.limits, [perf::TIMES, perf::HEAP_UNTAGGED]);
    let text = text(&output);
    assert!(text.contains("worst 1.2 ms; heap peak 3.3 MB\n"), "{text}");
}

/// A snapshot of `frame` with `rows` (class, name, bytes, blocks), its
/// live bytes their sum.
fn snapshot(frame: u32, rows: &[(TagClass, &'static str, u64, u32)]) -> HeapSnapshot {
    let rows: Vec<TagRow> = rows
        .iter()
        .map(|&(class, name, bytes, blocks)| TagRow::new(class, name, bytes, blocks))
        .collect();
    let live = rows.iter().map(|r| r.live_bytes).sum();
    let mut snapshot = HeapSnapshot::new(live, live, rows);
    snapshot.frame = frame;
    snapshot
}

#[test]
fn run_with_perf_answers_what_holds_the_heap_and_what_grew() {
    let mut game = measured(2);
    let first = snapshot(
        0,
        &[
            (TagClass::Phase, "load", 500_000, 40),
            (TagClass::Other, "other", 4096, 1),
        ],
    );
    let last = snapshot(
        600,
        &[
            (TagClass::Phase, "load", 500_000, 40),
            (TagClass::Phase, "update", 98_304, 6),
            (TagClass::Asset, "image", 24_576, 3),
            (TagClass::Span, "ai", 4096, 1),
            (TagClass::Other, "other", 4096, 1),
        ],
    );
    if let Ok(session) = &mut game.session {
        session.snapshots = vec![first.clone(), last.clone()];
        for record in &mut session.perf {
            record.heap = Some(HeapCounts::new(504_096, 631_072, 4));
        }
    }
    let dir = std::env::temp_dir().join(format!("shuyi-run-heap-{}", std::process::id()));
    let input = RunInput {
        perf_out: Some("perf.jsonl".to_owned()),
        ..RunInput::default()
    };
    let answer = run_at(&game, &dir, &input);
    assert_eq!(answer.limits, [perf::TIMES], "{:?}", answer.limits);
    let output = answer.result.unwrap();
    let summary = output.perf.as_ref().unwrap();
    let tags: Vec<&str> = summary.heap_tags.iter().map(|r| &*r.name).collect();
    // `other` and `ai` tie: `other`'s class comes first.
    assert_eq!(tags, ["load", "update", "image", "other", "ai"]);
    assert_eq!(summary.heap_growth, Some(first.diff(&last)));
    let text = text(&output);
    assert!(
        text.contains(
            "heap grew 124.0 KB from frame 0 to 600: update +96.0 KB, image +24.0 KB, ai +4.0 KB\n"
        ),
        "{text}"
    );
    let trace = fs::read_to_string(dir.join("perf.jsonl")).unwrap();
    let lines: Vec<PerfLine> = trace
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(lines.len(), 5);
    assert_eq!(
        lines[2..],
        [
            PerfLine::Snapshot(first.clone()),
            PerfLine::Snapshot(last.clone()),
            PerfLine::Diff(first.diff(&last)),
        ]
    );
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn run_with_perf_answers_the_summary_of_what_the_session_kept() {
    let input = RunInput {
        perf: true,
        ..RunInput::default()
    };
    let answer = run(&measured(3), &input);
    let output = answer.result.unwrap();
    let summary = output.perf.as_ref().unwrap();
    assert_eq!((summary.frames, summary.frame.p50), (3, 1200));
    assert_eq!(summary.heap, None);
    assert_eq!(answer.limits, [perf::TIMES, perf::HEAP_UNCOUNTED]);
    assert!(
        text(&output).contains("perf: 3 frames, frame p50 1.2 ms p99 1.2 ms worst 1.2 ms\n"),
        "{}",
        text(&output)
    );
    // Without perf, none.
    let plain = run(&measured(3), &RunInput::default());
    assert_eq!(plain.result.unwrap().perf, None);
    assert!(plain.limits.is_empty(), "{:?}", plain.limits);
}

#[test]
fn run_with_perf_on_a_session_that_measured_nothing_says_why() {
    let input = RunInput {
        perf: true,
        frames: 4,
        ..RunInput::default()
    };
    let answer = run(&measured(0), &input);
    assert_eq!(answer.result.unwrap().perf, None);
    assert_eq!(
        answer.limits,
        [
            perf::TIMES,
            "nothing was measured: the game's tool program was built without debug tools"
        ]
    );
}

#[test]
fn a_start_option_the_game_doesnt_offer_is_refused_before_its_session() {
    let unknown = RunInput {
        start: vec![GameArg {
            flag: "--seeed".to_owned(),
            value: None,
        }],
        ..RunInput::default()
    };
    let no_value = RunInput {
        start: vec![GameArg {
            flag: "--seed".to_owned(),
            value: None,
        }],
        ..RunInput::default()
    };
    // The game's session would fail, so only the check can answer SH0001.
    let game = Fake {
        session: Err("no session".to_owned()),
        ..Fake::default()
    };
    for input in [&unknown, &no_value] {
        for answer in [run(&game, input).code, scene(&game, input).code] {
            assert_eq!(answer.as_deref(), Some("SH0001"));
        }
    }
    assert_eq!(
        run(&game, &no_value).error.as_deref(),
        Some("--seed needs a value")
    );
}

/// `run`'s input for the record `file`.
fn from_session(file: &str) -> RunInput {
    RunInput {
        session: Some(SessionInput::new(file)),
        ..RunInput::default()
    }
}

/// A fresh folder holding `record.ron` and `broken.ron`.
fn records(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("shuyi-run-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("record.ron"), SessionRecord::new().to_ron()).unwrap();
    fs::write(dir.join("broken.ron"), "not a record").unwrap();
    dir
}

#[test]
fn start_options_with_a_session_are_refused() {
    let mut input = from_session("record.ron");
    input.start = vec![GameArg {
        flag: "--seed".to_owned(),
        value: Some("1".to_owned()),
    }];
    let answer = run_at(&Fake::default(), Path::new("."), &input);
    assert_eq!(answer.code.as_deref(), Some("SH0001"));
    assert!(answer.error.unwrap().contains("no start options"));
}

#[test]
fn a_file_that_isnt_a_record_is_refused_naming_the_file() {
    let dir = records("broken");
    for file in ["broken.ron", "missing.ron"] {
        let answer = run_at(&Fake::default(), &dir, &from_session(file));
        assert_eq!(answer.code.as_deref(), Some("SH3004"), "{file}");
        let error = answer.error.unwrap();
        assert!(error.starts_with(&format!("{file}: ")), "{error}");
    }
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_game_with_no_session_from_says_so() {
    let dir = records("unoffered");
    for answer in [
        run_at(&Fake::default(), &dir, &from_session("record.ron")).code,
        scene_at(&Fake::default(), &dir, &from_session("record.ron")).code,
    ] {
        assert_eq!(answer.as_deref(), Some("SH3001"));
    }
    let answer = run_at(&Fake::default(), &dir, &from_session("record.ron"));
    assert_eq!(
        answer.error.as_deref(),
        Some(
            "the game's tools don't offer run --session and scene --session \
             (GameTools::session_from)"
        )
    );
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_record_stamped_by_another_build_is_refused_before_the_game_is_asked() {
    let dir = records("stamp");
    let mut record = SessionRecord::new();
    record.made = Some(stamp("0.1.0", "other", ContentHash(1)));
    fs::write(dir.join("record.ron"), record.to_ron()).unwrap();
    let game = Fake {
        made: Some(stamp("0.1.0", "demo", ContentHash(2))),
        ..Fake::default()
    };
    let answer = run_at(&game, &dir, &from_session("record.ron"));
    assert_eq!(answer.code.as_deref(), Some("SH3007"));
    let mut input = from_session("record.ron");
    input.session.as_mut().unwrap().any_stamp = true;
    // Past the stamp, to the game, which offers none.
    assert_eq!(run_at(&game, &dir, &input).code.as_deref(), Some("SH3001"));
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn session_and_any_stamp_read_from_the_command_line() {
    let read = parse_args(&args(&["--session", "a.ron", "--any-stamp"]), &[]).unwrap();
    assert_eq!(
        read.session.as_ref().map(|s| s.file.as_str()),
        Some("a.ron")
    );
    assert!(read.session.unwrap().any_stamp);
    let read = parse_args(&args(&["--session", "a.ron"]), &[]).unwrap();
    assert!(!read.session.unwrap().any_stamp);
    assert_eq!(
        parse_args(&args(&["--any-stamp"]), &[]).unwrap_err(),
        "--any-stamp needs --session"
    );
    assert!(parse_args(&args(&["--session"]), &[]).is_err());
    assert!(usage("shuyi", Op::Scene, &[]).contains("--session <file>"));
}

/// A game whose scene's data is a hero at (2, 1) on turn 1, its shape
/// given or not.
fn with_data(shape: bool) -> Fake {
    #[derive(Serialize, JsonSchema)]
    struct Hero {
        x: i32,
        y: i32,
    }
    #[derive(Serialize, JsonSchema)]
    struct Data {
        turn: u32,
        hero: Hero,
    }
    let mut session = FakeSession::on("stage", "map");
    let data = Data {
        turn: 1,
        hero: Hero { x: 2, y: 1 },
    };
    let mut scene = Scene::new("stage", "grid map", "map")
        .with_data(&data)
        .unwrap();
    if !shape {
        scene.shape = None;
    }
    session.scene = Ok(scene);
    Fake {
        session: Ok(session),
        ..Fake::default()
    }
}

#[test]
fn scene_with_a_path_answers_that_value_in_place_of_the_scene() {
    let input = RunInput {
        path: Some("/hero/x".to_owned()),
        ..RunInput::default()
    };
    let answer = scene(&with_data(true), &input);
    assert!(answer.ok);
    let out = answer.result.unwrap();
    assert_eq!(out.value, Some(serde_json::json!(2)));
    assert!(out.scene.is_none() && out.shape.is_none());
    assert_eq!(
        scene_output_text(&out),
        "scene: after 0 frame(s), top screen stage\nvalue: 2\n"
    );
}

#[test]
fn scene_with_a_path_that_names_nothing_fails_with_its_code() {
    let input = RunInput {
        path: Some("/hero/z".to_owned()),
        ..RunInput::default()
    };
    let answer = scene(&with_data(true), &input);
    assert!(!answer.ok);
    assert_eq!(answer.code.as_deref(), Some(codes::NO_VALUE.0));
    assert_eq!(
        answer.error.as_deref(),
        Some("no value at `/hero/z`: /hero has the keys x, y")
    );
    assert_eq!(answer.exit_code(), 2);
}

#[test]
fn scene_with_shape_answers_the_schema_of_its_data_or_says_why_not() {
    let input = RunInput {
        shape: true,
        ..RunInput::default()
    };
    let answer = scene(&with_data(true), &input);
    assert!(answer.ok);
    let shape = answer.result.unwrap().shape.unwrap();
    assert!(shape["properties"]["hero"].is_object(), "{shape}");
    let answer = scene(&with_data(false), &input);
    assert!(answer.ok);
    assert!(answer.result.unwrap().shape.is_none());
    assert_eq!(answer.limits, [NO_SHAPE]);
}

#[test]
fn run_takes_no_path_and_no_shape() {
    let input = RunInput {
        shape: true,
        ..RunInput::default()
    };
    let answer = run(&with_data(true), &input);
    assert_eq!(answer.code.as_deref(), Some(codes::WRONG_ARGUMENTS.0));
}

#[test]
fn path_and_shape_read_from_the_command_line() {
    let input = parse_args(&args(&["--path", "/hero/x", "--shape"]), &OPTIONS).unwrap();
    assert_eq!(input.path.as_deref(), Some("/hero/x"));
    assert!(input.shape);
    assert!(usage("shuyi", Op::Scene, &[]).contains("--path <pointer>"));
    assert!(!usage("shuyi", Op::Run, &[]).contains("--path"));
}
