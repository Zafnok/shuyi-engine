//! The grid stage played through the engine's Harness, and through the
//! game's tools (the tool contract). Key names here are the Harness's
//! test-script names, input to a test, not game code.

use std::fs;
use std::path::Path;

use game::stages::top_down_grid::{StageRecord, StageScreen, Tools};
use rules::{Command, Outcome, Stage};
use shuyi::core::{Dir, Pos, TurnRules};
use shuyi::platform::harness::Harness;
use shuyi_cli::GameTools;
use shuyi_cli::contract::bots::{BotsInput, bots};
use shuyi_cli::contract::check::{CheckInput, check, targets};
use shuyi_cli::contract::import::{ImportInput, import};
use shuyi_cli::contract::replay::{ReplayInput, replay};
use shuyi_cli::contract::run::{RunInput, keys, run, scene};
use shuyi_cli::contract::target::engine_profiles;
use shuyi_cli::project::Project;

/// The game's tools.
fn tools() -> Tools {
    game::stages::tools()
}

/// The game in the Harness, on its stage seeded `seed`.
fn on_stage(seed: u64) -> Harness<game::Data> {
    let mut h = tools().harness(seed).unwrap_or_else(|e| panic!("{e}"));
    h.keys("Enter");
    h
}

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
fn confirm_on_the_title_opens_the_stage_and_a_step_moves_your_piece() {
    let mut h = on_stage(0);
    assert_eq!(h.screens(), ["title", "stage"]);
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
    let mut h = on_stage(5);
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
    let found = &stage(&h).commands;
    assert!(found.is_empty(), "{found:?}");
    // Cancel goes back to the title, and there leaves the game.
    h.keys("Backspace");
    assert_eq!(h.screens(), ["title"]);
    h.keys("Backspace");
    assert!(h.quit_requested());
}

#[test]
fn check_finds_no_error_in_the_files() {
    let answer = check(&tools(), &CheckInput::default());
    assert!(answer.ok, "{answer:?}");
}

#[test]
fn run_and_scene_show_the_stage_after_a_script() {
    let mut input = RunInput::default();
    input.script = keys("Enter Right Right");
    input.frames = 10;
    let answer = run(&tools(), &input);
    assert!(answer.ok, "{answer:?}");
    let out = answer.result.unwrap();
    assert_eq!((out.top_screen.as_str(), out.frames), ("stage", 16));
    let shown = out.scene.unwrap();
    assert_eq!(shown.data["commands"], 2);
    assert!(shown.text.contains("@ (3,6)"), "{}", shown.text);
    // `scene` sees the same.
    let answer = scene(&tools(), &input);
    assert!(answer.ok, "{answer:?}");
    assert_eq!(answer.result.unwrap().scene.unwrap().text, shown.text);
}

#[test]
fn the_bots_play_to_an_end_and_their_records_replay() {
    let mut input = BotsInput::default();
    input.games = 20;
    let answer = bots(&tools(), &input);
    assert!(answer.ok, "{answer:?}");
    let out = answer.result.unwrap();
    assert_eq!(out.games, 20);
    assert_eq!(out.outcomes.values().sum::<u32>(), 20);
    let dir = std::env::temp_dir().join(format!("game-replay-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let record = out.plays[0].record.clone().unwrap();
    fs::write(dir.join("play.ron"), &record).unwrap();
    let input = ReplayInput::new("play.ron");
    let answer = replay(&tools(), &dir, &input);
    assert!(answer.ok, "{answer:?}");
    let run = &answer.result.unwrap().runs[0];
    let parsed: StageRecord = ron::from_str(&record).unwrap();
    // The bots stamp their records with this build, and replay runs them.
    assert_eq!(parsed.made, Some(game::made()));
    assert_eq!(run.made, parsed.made);
    assert_eq!(run.steps.len(), parsed.commands.len() + 1);
    // The record replays to the outcome the bots saw.
    let map = game::game().data().unwrap().own.map;
    let mut stage = Stage::new(&map, parsed.seed).unwrap();
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
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let input = ImportInput::new("files/stage.map");
    let answer = import(&tools().importers(), root, &input);
    assert!(answer.ok, "{answer:?}");
    let read = &answer.result.unwrap().read_as;
    assert_eq!(read[0].summary, "map: 12 tiles by 8, 2 piece(s)");
}

#[test]
fn the_title_and_the_stage_declare_their_actions_and_they_fit_every_target() {
    let controls = tools()
        .controls()
        .unwrap()
        .unwrap_or_else(|e| panic!("{e}"));
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let project = Project::load(&root).unwrap_or_else(|e| panic!("{e}"));
    let profiles = engine_profiles().unwrap_or_else(|e| panic!("{e:?}"));
    for id in project.config.targets.keys() {
        assert_eq!(controls.fit(&profiles[id].profile), [], "{id}");
    }
    let game = game::game();
    let mut ctx = game.ctx().unwrap_or_else(|e| panic!("{e}"));
    let names: Vec<&str> = game
        .every_screen(&mut ctx)
        .iter()
        .map(|s| s.name())
        .collect();
    assert_eq!(names, ["title", "stage"]);
    let mut input = CheckInput::default();
    input.target = Some("all".to_owned());
    let answer = targets(&project.config, &project.root, &input, Some(&tools()));
    assert!(answer.ok, "{answer:?}");
}
