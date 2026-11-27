//! The screens as they draw and declare themselves: the title, the pause
//! menu and the end of a run as text, the board's cells, each screen's
//! actions, and what a frame's hash and a covered stage hold.

use shuyi::core::prelude::{Pos, StateHash, TickSim};
use shuyi::prelude::grid::MapUnit;
use shuyi::prelude::{EngineColor, UiAction};
use snake::screens::{Over, Part, Pause, Piece, Stage};

use crate::{at_title, playthrough, stage};

#[test]
fn the_title_pause_and_end_of_a_run_as_text() {
    let mut h = at_title(1);
    insta::assert_snapshot!("title", h.snapshot());
    h.act(UiAction::Confirm);
    h.act(UiAction::Cancel);
    insta::assert_snapshot!("pause", h.snapshot());
    let (h, _) = playthrough();
    insta::assert_snapshot!("over", h.snapshot());
}

#[test]
fn the_pause_menus_focus_is_a_bar_that_up_and_down_move() {
    let mut h = at_title(1);
    h.act(UiAction::Confirm);
    h.act(UiAction::Cancel);
    let focus = h.ctx().palette.engine(EngineColor::Focus);
    // The row of each item, and whether its first letter sits on the bar.
    let on_bar = |h: &shuyi::prelude::test::Harness<snake::Data>, word: &str| {
        let buf = h.game().buffer();
        (0..i32::from(buf.height())).find_map(|y| {
            let row: String = (0..i32::from(buf.width()))
                .filter_map(|x| buf.get(x, y).map(|c| c.glyph))
                .collect();
            let at = row.find(word)?;
            let x = i32::try_from(row[..at].chars().count()).unwrap_or(0);
            buf.get(x, y).map(|cell| cell.bg == focus)
        })
    };
    assert_eq!(on_bar(&h, "Resume"), Some(true));
    assert_eq!(on_bar(&h, "Back to the title"), Some(false));
    h.act(UiAction::Down);
    assert_eq!(h.screen::<Pause>().map(|p| p.focus), Some(1));
    assert_eq!(on_bar(&h, "Resume"), Some(false));
    assert_eq!(on_bar(&h, "Back to the title"), Some(true));
    // Down again goes back round to the first.
    h.act(UiAction::Down);
    assert_eq!(h.screen::<Pause>().map(|p| p.focus), Some(0));
}

#[test]
fn the_board_paints_each_part_on_both_cells_of_its_tile() {
    let mut h = at_title(1);
    h.act(UiAction::Confirm);
    let buf = h.game().buffer();
    let cells = |x: i32, y: i32| [x, x + 1].map(|x| buf.get(x, y).map_or(' ', |cell| cell.glyph));
    // The head on (6, 7): cells 12 and 13 of row 8 (the score line is row
    // 0); its body to the left, a wall at the board's edge.
    assert_eq!(cells(12, 8), ['►', '►']);
    assert_eq!(cells(10, 8), ['(', ')']);
    assert_eq!(cells(0, 8), ['▓', '▓']);
    let food = stage(&h).snake.food().unwrap_or_else(|| panic!("no food"));
    assert_eq!(cells(food.x * 2, food.y + 1), ['<', '>']);
    // A piece is where it says.
    let piece = Piece {
        part: Part::Food,
        pos: Pos::new(3, 4),
    };
    assert_eq!((piece.id(), piece.pos()), (Part::Food, Pos::new(3, 4)));
}

#[test]
fn every_screen_is_listed_and_declares_its_actions() {
    use UiAction::{Cancel, Confirm, Down, Left, Right, Up};

    let game = snake::game();
    let mut ctx = game.ctx().unwrap_or_else(|e| panic!("{e}"));
    let screens = snake::screens::every(&mut ctx);
    let declared: Vec<(&str, Option<Vec<UiAction>>)> =
        screens.iter().map(|s| (s.name(), s.actions())).collect();
    assert_eq!(
        declared,
        [
            ("title", Some(vec![Confirm, Cancel])),
            ("stage", Some(vec![Up, Down, Left, Right, Cancel])),
            ("pause", Some(vec![Up, Down, Confirm, Cancel])),
            ("over", Some(vec![Confirm, Cancel])),
        ]
    );
}

#[test]
fn a_frames_hash_is_the_runs_and_the_end_screen_holds_its_score() {
    let (h, _) = playthrough();
    let run = stage(&h);
    assert_eq!(h.game().state_hash(), Some(StateHash(run.snake.hash())));
    assert_ne!(h.game().state_hash(), Some(StateHash::default()));
    let over = h
        .screen::<Over>()
        .unwrap_or_else(|| panic!("no end screen"));
    assert_eq!((over.score, over.length), (1, 4));
}

#[test]
fn a_turn_pressed_before_a_step_ran_is_dropped_by_the_pause() {
    let mut h = at_title(1);
    h.act(UiAction::Confirm);
    // Up and Cancel in a frame with no step: the turn waits in the driver,
    // and the pause covers the stage.
    let game = h.app_mut().game_mut().unwrap_or_else(|| panic!("no game"));
    game.act(UiAction::Up);
    game.act(UiAction::Cancel);
    h.frame(&[], 0.0);
    assert_eq!(h.screens(), ["title", "stage", "pause"]);
    h.act(UiAction::Cancel);
    h.wait(0.5);
    let run = stage(&h);
    assert_eq!(
        run.snake.heading(),
        shuyi::core::prelude::Dir::Right,
        "{:?}",
        run.events
    );
    let _ = run.record();
    // A test may reach the run to change it (`Screen::as_any_mut`).
    let game = h.app_mut().game_mut().unwrap_or_else(|| panic!("no game"));
    let found = game.screen_mut::<Stage>().map(|stage| stage.seed);
    assert_eq!(found, Some(1));
}
