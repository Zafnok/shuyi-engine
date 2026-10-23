//! The game played through the engine's Harness, and through its tools
//! (the tool contract). Key names here are the Harness's test-script
//! names, input to a test, not game code.

use std::path::Path;

use game::title::Title;
use shuyi_cli::contract::bots::BotsInput;
use shuyi_cli::contract::check::{CheckInput, check, targets};
use shuyi_cli::contract::run::{RunInput, keys, run};
use shuyi_cli::contract::session::Event;
use shuyi_cli::contract::target::engine_profiles;
use shuyi_cli::project::Project;
use shuyi_cli::{BasicTools, GameTools};

/// The game's tools, as its tool program has them.
fn tools() -> BasicTools<shuyi_platform::prelude::UiAction> {
    BasicTools::new(game::game())
}

#[test]
fn the_title_shows_the_games_name_and_cancel_leaves() {
    let mut h = game::game().harness();
    assert_eq!(h.screens(), ["title"]);
    let view = Title::view(h.ctx());
    assert!(h.snapshot().contains(&view.name), "{}", h.snapshot());
    assert!(!view.help.is_empty(), "{:?}", view.help);
    h.keys("Backspace");
    assert!(h.quit_requested());
}

#[test]
fn check_finds_no_error_in_the_files() {
    let answer = check(&tools(), &CheckInput::default());
    assert!(answer.ok, "{answer:?}");
}

#[test]
fn run_says_the_game_quit_and_has_no_scene() {
    let input = RunInput {
        script: keys("Backspace"),
        frames: 3,
        ..RunInput::default()
    };
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
    assert_eq!(names, ["title"]);
    assert!(screens.iter().all(|s| s.actions().is_some()));
    // The targets shuyi.ron lists, as `cargo shuyi check --target all`
    // checks them.
    let controls = tools()
        .controls()
        .unwrap()
        .unwrap_or_else(|e| panic!("{e}"));
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
    let answer = targets(
        &project.config,
        &project.root,
        &all_targets(),
        Some(&tools()),
    );
    assert!(answer.ok, "{answer:?}");
    assert!(
        !answer.limits.iter().any(|l| l.contains("controls aren't")),
        "{:?}",
        answer.limits
    );
}

/// `check --target all`'s input.
fn all_targets() -> CheckInput {
    CheckInput {
        target: Some("all".to_owned()),
        ..CheckInput::default()
    }
}
