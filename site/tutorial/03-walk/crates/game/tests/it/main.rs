//! The game played through the engine's Harness, and through its tools
//! (the tool contract). Key names here are the Harness's test-script
//! names, input to a test, not game code.

use std::path::Path;

use game::title::Title;
use shuyi::prelude::UiAction;
use shuyi_cli::contract::bots::BotsInput;
use shuyi_cli::contract::check::{CheckInput, check, targets};
use shuyi_cli::contract::run::{RunInput, keys, run};
use shuyi_cli::contract::session::Event;
use shuyi_cli::contract::target::engine_profiles;
use shuyi_cli::project::Project;
use shuyi_cli::{BasicTools, GameTools};

/// The engine's tools for the game: `game::stages::tools` gives its
/// stages' own on top of them.
fn tools() -> BasicTools<UiAction, game::stages::Own> {
    BasicTools::new(game::game())
}

#[test]
fn the_title_shows_the_games_name_and_cancel_leaves() {
    let mut h = game::game().harness();
    assert_eq!(h.screens(), ["title"]);
    let view = Title::view(h.ctx());
    assert!(h.snapshot().contains(&view.title), "{}", h.snapshot());
    assert!(!view.help.is_empty(), "{:?}", view.help);
    assert_eq!(view.play.is_some(), !game::stages::names().is_empty());
    h.keys("Backspace");
    assert!(h.quit_requested());
}

#[test]
fn the_title_quits_only_where_the_game_can() {
    let profiles = engine_profiles().unwrap();
    let mut h = game::game().harness().as_target(&profiles["web"].profile);
    assert_eq!(Title::view(h.ctx()).help, "");
    h.keys("Backspace");
    assert!(!h.quit_requested());
    assert_eq!(h.screens(), ["title"]);

    let mut h = game::game()
        .harness()
        .as_target(&profiles["desktop"].profile);
    assert_ne!(Title::view(h.ctx()).help, "");
    h.keys("Backspace");
    assert!(h.quit_requested());
}

#[test]
fn confirm_starts_the_first_stage_if_there_is_one() {
    let mut h = game::game().harness();
    h.keys("Enter");
    match game::stages::names().first() {
        Some(_) => assert_eq!(h.screens().len(), 2, "{:?}", h.screens()),
        None => assert_eq!(h.screens(), ["title"]),
    }
}

#[test]
fn check_finds_no_error_in_the_files() {
    let answer = check(&tools(), &CheckInput::default());
    assert!(answer.ok, "{answer:?}");
    let answer = check(&game::stages::tools(), &CheckInput::default());
    assert!(answer.ok, "{answer:?}");
}

#[test]
fn run_says_the_game_quit_and_the_title_has_no_scene() {
    let mut input = RunInput::default();
    input.script = keys("Backspace");
    input.frames = 3;
    let answer = run(&tools(), &input);
    assert!(answer.ok, "{answer:?}");
    let out = answer.result.as_ref().unwrap();
    assert_eq!(out.quit, Some(1));
    assert!(out.events.contains(&Event::Quit { frame: 1 }));
    assert!(out.scene.is_none());
    assert!(answer.limits.iter().any(|l| l.starts_with("no scene")));
    assert!(tools().bots(&BotsInput::default()).is_err());
}

#[test]
fn every_screen_declares_its_actions_and_they_fit_every_target() {
    let game = game::game();
    let mut ctx = game.ctx().unwrap_or_else(|e| panic!("{e}"));
    let screens = game.every_screen(&mut ctx);
    let names: Vec<&str> = screens.iter().map(|s| s.name()).collect();
    assert_eq!(names.first(), Some(&"title"), "{names:?}");
    assert!(screens.iter().all(|s| s.actions().is_some()));
    // The targets shuyi.ron lists, as `cargo shuyi check --target all`
    // checks them.
    let tools = game::stages::tools();
    let controls = tools.controls().unwrap().unwrap_or_else(|e| panic!("{e}"));
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let project = Project::load(&root).unwrap_or_else(|e| panic!("{e}"));
    let profiles = engine_profiles().unwrap_or_else(|e| panic!("{e:?}"));
    assert!(
        !project.config.targets.is_empty(),
        "{:?}",
        project.config.targets
    );
    for id in project.config.targets.keys() {
        assert_eq!(controls.fit(&profiles[id].profile), [], "{id}");
    }
    let answer = targets(&project.config, &project.root, &all_targets(), Some(&tools));
    assert!(answer.ok, "{answer:?}");
    assert!(
        !answer.limits.iter().any(|l| l.contains("controls aren't")),
        "{:?}",
        answer.limits
    );
}

/// `check --target all`'s input.
fn all_targets() -> CheckInput {
    let mut input = CheckInput::default();
    input.target = Some("all".to_owned());
    input
}
