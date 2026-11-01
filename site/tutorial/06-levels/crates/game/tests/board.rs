//! The puzzle played through the engine's Harness: tests of what happened
//! read the board's state. Key names here are the Harness's test-script
//! names, input to a test, not game code.

use game::board::Board;
use game::levels::LevelMenu;
use shuyi::core::Pos;
use shuyi::core::TurnRules;
use shuyi::platform::harness::Harness;

// ANCHOR: on_board
/// The game on its first level.
fn on_board() -> Harness<game::Data> {
    let mut h = game::game().harness();
    h.keys("Enter Enter");
    assert_eq!(h.screens(), ["title", "levels", "board"]);
    h
}
// ANCHOR_END: on_board

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

// ANCHOR: test_walk
#[test]
fn the_player_walks_and_a_wall_stops_them() {
    let mut h = on_board();
    assert_eq!(board(&h).puzzle.player(), Pos::new(2, 2));
    h.keys("Up");
    assert_eq!(board(&h).puzzle.player(), Pos::new(2, 1));
    h.keys("Up");
    assert_eq!(board(&h).puzzle.player(), Pos::new(2, 1));
}
// ANCHOR_END: test_walk

// ANCHOR: test_push
#[test]
fn pushing_the_box_onto_the_target_wins() {
    let mut h = on_board();
    h.keys("Right");
    assert!(board(&h).puzzle.boxes().contains(&Pos::new(4, 2)));
    assert!(board(&h).puzzle.outcome().is_none());
    h.keys("Right");
    assert!(board(&h).puzzle.boxes().contains(&Pos::new(5, 2)));
    assert!(board(&h).puzzle.outcome().is_some());
}
// ANCHOR_END: test_push

#[test]
fn a_box_against_a_wall_stays_and_so_does_the_player() {
    let mut h = on_board();
    // Round the box, and push it up against the top wall.
    h.keys("Down Right Up");
    assert_eq!(board(&h).puzzle.player(), Pos::new(3, 2));
    assert!(board(&h).puzzle.boxes().contains(&Pos::new(3, 1)));
    h.keys("Up");
    assert_eq!(board(&h).puzzle.player(), Pos::new(3, 2));
}

// ANCHOR: test_undo
#[test]
fn undo_takes_back_a_push_and_nothing_before_the_start() {
    let mut h = on_board();
    let start = board(&h).puzzle.clone();
    h.keys("Right");
    assert_ne!(board(&h).puzzle, start);
    h.keys("u");
    assert_eq!(board(&h).puzzle, start);
    h.keys("u");
    assert_eq!(board(&h).puzzle, start);
}
// ANCHOR_END: test_undo

// ANCHOR: test_result
#[test]
fn a_won_level_hands_its_result_to_the_menu() {
    let mut h = on_board();
    h.keys("Right Right Enter");
    assert_eq!(h.screens(), ["title", "levels"]);
    let menu = h.screen::<LevelMenu>().unwrap_or_else(|| panic!("no menu"));
    assert_eq!(menu.focus, 1);
    h.keys("Enter");
    assert!(h.snapshot().contains("Two rows"), "{}", h.snapshot());
}
// ANCHOR_END: test_result

#[test]
fn confirm_does_nothing_on_a_level_not_won() {
    let mut h = on_board();
    h.keys("Enter");
    assert_eq!(h.screens(), ["title", "levels", "board"]);
}

#[test]
fn cancel_leaves_the_menu_for_the_title() {
    let mut h = game::game().harness();
    h.keys("Enter Backspace");
    assert_eq!(h.screens(), ["title"]);
}
