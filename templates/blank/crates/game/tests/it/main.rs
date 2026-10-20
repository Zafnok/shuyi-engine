//! The game played through the engine's Harness, and through its tools
//! (the tool contract). Key names here are the Harness's test-script
//! names, input to a test, not game code.

use game::title::Title;
use game::tools::{Tools, harness};
use shuyi_cli::GameTools;
use shuyi_cli::contract::bots::BotsInput;
use shuyi_cli::contract::check::{CheckInput, check};
use shuyi_cli::contract::run::{RunInput, keys, run};
use shuyi_cli::contract::session::Event;

#[test]
fn the_title_shows_the_games_name_and_cancel_leaves() {
    let mut h = harness();
    assert_eq!(h.screens(), ["title"]);
    let view = Title::view(h.ctx());
    assert!(h.snapshot().contains(&view.name), "{}", h.snapshot());
    assert!(!view.help.is_empty());
    h.keys("Backspace");
    assert!(h.quit_requested());
}

#[test]
fn check_finds_no_error_in_the_files() {
    let answer = check(&Tools, &CheckInput::default());
    assert!(answer.ok, "{answer:?}");
}

#[test]
fn run_says_the_game_quit_and_has_no_scene() {
    let input = RunInput {
        script: keys("Backspace"),
        frames: 3,
        ..RunInput::default()
    };
    let answer = run(&Tools, &input);
    assert!(answer.ok, "{answer:?}");
    let out = answer.result.as_ref().unwrap();
    assert_eq!(out.quit, Some(1));
    assert!(out.events.contains(&Event::Quit { frame: 1 }));
    assert!(out.scene.is_none());
    assert!(answer.limits.iter().any(|l| l.starts_with("no scene")));
    assert!(Tools.bots(&BotsInput::default()).is_err());
}
