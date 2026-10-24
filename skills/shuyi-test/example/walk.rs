//! The grid stage played through the engine's Harness: keys or actions
//! in, frames run, what happened read from the stage's state. Key names
//! here are the Harness's test-script names (the template's default
//! keys), input to a test, not game code.

use game::stages::top_down_grid::StageScreen;
use shuyi::core::{Pos, Sides, TurnRules};
use shuyi::platform::harness::Harness;
use shuyi::prelude::UiAction;

/// The stage on top of `h`.
fn stage(h: &Harness<game::Data>) -> &StageScreen {
    h.screen::<StageScreen>()
        .unwrap_or_else(|| panic!("no stage: {:?}", h.screens()))
}

#[test]
fn confirm_opens_the_stage_and_right_steps_your_piece() {
    let mut h = game::game().harness();
    assert_eq!(h.top_screen(), "title");
    h.keys("Enter");
    assert_eq!(h.screens(), ["title", "stage"]);
    let start = stage(&h).stage.hero();
    h.keys("Right");
    let screen = stage(&h);
    assert_eq!(screen.stage.hero(), Pos::new(start.x + 1, start.y));
    assert_eq!(screen.stage.turn(), 2);
    assert_eq!(screen.stage.outcome(), None);
}

#[test]
fn an_action_presses_the_key_the_player_bound_to_it() {
    let mut h = game::game().harness();
    h.act(UiAction::Confirm).act(UiAction::Right);
    assert_eq!(stage(&h).commands.len(), 1);
}

#[test]
fn cancel_goes_back_to_the_title_then_leaves_the_game() {
    let mut h = game::game().harness();
    h.keys("Enter Backspace");
    assert_eq!(h.screens(), ["title"]);
    h.keys("Backspace");
    assert!(h.quit_requested());
}
