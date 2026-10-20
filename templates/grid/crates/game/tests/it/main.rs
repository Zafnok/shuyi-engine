//! The game played through the engine's Harness, and through its tools
//! (the tool contract). Key names here are the Harness's test-script
//! names, input to a test, not game code.

use std::fs;

use game::stage::StageScreen;
use game::tools::{StageRecord, Tools, harness};
use rules::{Command, Outcome, Stage};
use shuyi_cli::GameTools;
use shuyi_cli::contract::bots::{BotsInput, bots};
use shuyi_cli::contract::check::{CheckInput, check};
use shuyi_cli::contract::import::{ImportInput, import};
use shuyi_cli::contract::replay::{ReplayInput, replay};
use shuyi_cli::contract::run::{RunInput, keys, run, scene};
use shuyi_core::{Dir, Pos, TurnRules};
use shuyi_ui::harness::Harness;

fn stage(h: &Harness<game::Data>) -> &StageScreen {
    h.screen::<StageScreen>()
        .unwrap_or_else(|| panic!("no stage: {:?}", h.screens()))
}

/// The key for a step `dir`.
fn key(dir: Dir) -> &'static str {
    match dir {
        Dir::Right => "Right",
        Dir::Down => "Down",
        Dir::Left => "Left",
        Dir::Up => "Up",
    }
}

#[test]
fn the_game_opens_on_the_stage_and_a_step_moves_your_piece() {
    let mut h = harness(0);
    assert_eq!(h.screens(), ["stage"]);
    let start = stage(&h).stage.hero();
    h.keys("Right");
    assert_eq!(stage(&h).stage.hero(), Pos::new(start.x + 1, start.y));
    assert_eq!(stage(&h).commands, [Command::Step(Dir::Right)]);
    // A wall is below: the step is refused, and nothing changes.
    h.keys("Down");
    assert_eq!(stage(&h).commands.len(), 1);
    assert!(h.snapshot().contains("Turn 2"), "{}", h.snapshot());
}

#[test]
fn a_stage_plays_to_its_end_and_confirm_starts_the_next() {
    let mut h = harness(5);
    for _ in 0..500 {
        let screen = stage(&h);
        if screen.stage.outcome().is_some() {
            break;
        }
        let step = screen.stage.legal_commands()[0];
        let Command::Step(dir) = step;
        h.keys(key(dir));
    }
    let screen = stage(&h);
    assert!(screen.stage.outcome().is_some(), "no end in 500 steps");
    assert_eq!(screen.seed, 5);
    h.keys("Enter");
    assert_eq!(stage(&h).seed, 6);
    assert!(stage(&h).commands.is_empty());
    h.keys("Backspace");
    assert!(h.quit_requested());
}

#[test]
fn check_finds_no_error_in_the_files() {
    let answer = check(&Tools, &CheckInput::default());
    assert!(answer.ok, "{answer:?}");
}

#[test]
fn run_and_scene_show_the_stage_after_a_script() {
    let input = RunInput {
        script: keys("Right Right"),
        frames: 10,
        ..RunInput::default()
    };
    let answer = run(&Tools, &input);
    assert!(answer.ok, "{answer:?}");
    let out = answer.result.unwrap();
    assert_eq!((out.top_screen.as_str(), out.frames), ("stage", 14));
    let shown = out.scene.unwrap();
    assert_eq!(shown.data["commands"], 2);
    assert!(shown.text.contains("@ (3,6)"), "{}", shown.text);
    // `scene` sees the same.
    let answer = scene(&Tools, &input);
    assert!(answer.ok, "{answer:?}");
    assert_eq!(answer.result.unwrap().scene.unwrap().text, shown.text);
}

#[test]
fn the_bots_play_to_an_end_and_their_records_replay() {
    let input = BotsInput {
        games: 20,
        ..BotsInput::default()
    };
    let answer = bots(&Tools, &input);
    assert!(answer.ok, "{answer:?}");
    let out = answer.result.unwrap();
    assert_eq!(out.games, 20);
    assert_eq!(out.outcomes.values().sum::<u32>(), 20);
    let dir = std::env::temp_dir().join(format!("game-replay-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let record = out.plays[0].record.clone().unwrap();
    fs::write(dir.join("play.ron"), &record).unwrap();
    let input = ReplayInput {
        record: "play.ron".to_owned(),
        other: None,
    };
    let answer = replay(&Tools, &dir, &input);
    assert!(answer.ok, "{answer:?}");
    let run = &answer.result.unwrap().runs[0];
    let parsed: StageRecord = ron::from_str(&record).unwrap();
    assert_eq!(run.steps.len(), parsed.commands.len() + 1);
    // The record replays to the outcome the bots saw.
    let mut stage = Stage::new(&game::Data::load().unwrap().stage_map, parsed.seed).unwrap();
    for command in &parsed.commands {
        stage.apply(command).unwrap();
    }
    assert_eq!(
        run.outcome,
        stage.outcome().map(|o: Outcome| format!("{o:?}"))
    );
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn import_reads_the_stages_map() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let input = ImportInput {
        file: "files/stage.map".to_owned(),
        kind: None,
        print: false,
    };
    let answer = import(&Tools.importers(), root, &input);
    assert!(answer.ok, "{answer:?}");
    let read = &answer.result.unwrap().read_as;
    assert_eq!(read[0].summary, "map: 12 tiles by 8, 2 piece(s)");
}
