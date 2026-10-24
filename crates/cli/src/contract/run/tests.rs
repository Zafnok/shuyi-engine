//! `run` and `scene` on a made-up game.

use super::*;
use crate::contract::fake::{Fake, FakeSession, OPTIONS};

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
    assert!(bad.check().unwrap_err().starts_with("keys: "));
    let answer = run(&Fake::default(), &too_many);
    assert_eq!(
        answer.error.as_deref(),
        Some("frames: at most 100000, not 100001")
    );
    assert!(scene(&Fake::default(), &bad).error.is_some());
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
            "--script", "Enter", "--keys", "Up", "--pad", "South", "--wait", "0.5", "--frames",
            "30", "--seed", "4",
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
        log: Vec::new(),
        modules: Vec::new(),
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
        }),
        log: Vec::new(),
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
