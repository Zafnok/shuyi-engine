//! The puzzle played through the engine's Harness: tests of what happened
//! read the board's state. Key names here are the Harness's test-script
//! names, input to a test, not game code.

use game::board::Board;
use shuyi::platform::harness::Harness;

/// The game on its first level.
fn on_board() -> Harness<game::Data> {
    let mut h = game::game().harness();
    h.keys("Enter");
    assert_eq!(h.screens(), ["title", "board"]);
    h
}

fn board(h: &Harness<game::Data>) -> &Board {
    h.screen::<Board>()
        .unwrap_or_else(|| panic!("no board: {:?}", h.screens()))
}

#[test]
fn the_level_is_drawn_and_cancel_goes_back() {
    let mut h = on_board();
    assert!(h.snapshot().contains("First push"), "{}", h.snapshot());
    assert_eq!(board(&h).puzzle.boxes().len(), 1);
    h.keys("Backspace");
    assert!(h.screen::<Board>().is_none());
}
