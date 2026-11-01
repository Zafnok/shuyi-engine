//! The game through its tools (the tool contract): a bot that solves level
//! 1, a replay, what a program reads of the scene and a frame kept in the
//! repository. Key names here are the Harness's test-script names, input
//! to a test, not game code.

use std::path::Path;

use game::level;
use game::rules::{Command, Puzzle};
use game::tools::{LevelRecord, LineBot};
use shuyi::core::Dir;
use shuyi::core::bots::play;
use shuyi_cli::contract::bots::{BotsInput, bots};
use shuyi_cli::contract::replay::{ReplayInput, replay};
use shuyi_cli::contract::run::{RunInput, keys, scene};

// ANCHOR: test_bot
#[test]
fn every_levels_line_wins_it() {
    let bundle = game::game().bundle();
    for (n, file) in level::read_all(&bundle).unwrap().iter().enumerate() {
        let mut puzzle = Puzzle::new(file).unwrap();
        let mut bot = LineBot::new(&bundle, n).unwrap();
        let report = play(&mut puzzle, &mut [((), &mut bot)], 500).unwrap();
        assert!(report.outcome.is_some(), "level {}", n + 1);
    }
}

#[test]
fn the_bots_tool_wins_level_one() {
    let mut input = BotsInput::default();
    input.games = 3;
    let answer = bots(&game::stages::tools(), &input);
    assert!(answer.ok, "{answer:?}");
    let out = answer.result.as_ref().unwrap();
    assert_eq!(out.outcomes.get("won"), Some(&3), "{out:?}");
}
// ANCHOR_END: test_bot

// ANCHOR: test_replay
#[test]
fn a_record_replays_with_a_hash_at_every_step() {
    let dir = std::env::temp_dir().join(format!("box-pusher-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let record = LevelRecord {
        level: 0,
        commands: vec![Command::Step(Dir::Right), Command::Step(Dir::Right)],
        made: Some(game::made()),
    };
    std::fs::write(dir.join("one.ron"), ron::to_string(&record).unwrap()).unwrap();
    let answer = replay(&game::stages::tools(), &dir, &ReplayInput::new("one.ron"));
    std::fs::remove_dir_all(&dir).unwrap();
    assert!(answer.ok, "{answer:?}");
}
// ANCHOR_END: test_replay

// ANCHOR: test_scene
#[test]
fn the_scene_says_the_moves_and_whether_it_is_won() {
    let tools = game::stages::tools();
    let mut input = RunInput::default();
    input.script = keys("Enter Enter Right");
    let answer = scene(&tools, &input);
    let data = &answer.result.as_ref().unwrap().scene.as_ref().unwrap().data;
    assert_eq!(data["moves"], 1, "{data}");
    assert_eq!(data["won"], false, "{data}");
    input.script = keys("Enter Enter Right Right");
    let answer = scene(&tools, &input);
    let data = &answer.result.as_ref().unwrap().scene.as_ref().unwrap().data;
    assert_eq!(data["moves"], 2, "{data}");
    assert_eq!(data["won"], true, "{data}");
}
// ANCHOR_END: test_scene

// ANCHOR: test_frame
/// Compares the first level's frame with the file kept beside this test;
/// `UPDATE_FRAMES=1` writes it, to read before keeping it.
#[test]
fn the_first_level_looks_as_it_did() {
    let mut h = game::game().harness();
    h.keys("Enter Enter");
    let frame = h.snapshot();
    let file = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/frames/level1.txt");
    if std::env::var_os("UPDATE_FRAMES").is_some() {
        std::fs::write(&file, &frame).unwrap();
    }
    let kept = std::fs::read_to_string(&file).unwrap().replace('\r', "");
    assert_eq!(frame, kept);
}
// ANCHOR_END: test_frame
