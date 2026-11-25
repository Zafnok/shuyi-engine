//! Snake played through the engine's Harness and its tools (the tool
//! contract): a run through actions only, the same run at 30 and 60 frames
//! a second, its record replayed, its rules under random input, its
//! pictures, and the checks of its files and targets.

mod contract;
mod files;
mod frames;
mod playthrough;
mod props;
mod snapshot;

use std::fs;
use std::path::{Path, PathBuf};

use shuyi::core::Dir;
use shuyi::core::tick::TickRecord;
use shuyi::platform::harness::Harness;
use shuyi::prelude::UiAction;
use snake::Data;
use snake::screens::Stage;
use snake::tools::Tools;

/// The game's tools.
fn tools() -> Tools {
    Tools::new(snake::game())
}

/// The example's folder, which holds its `shuyi.ron`.
fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

/// A fresh empty folder under the system temp folder.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("snake-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display()));
    dir
}

/// The game at its title, its first run seeded `seed`.
fn at_title(seed: u64) -> Harness<Data> {
    tools().harness(seed).unwrap_or_else(|e| panic!("{e}"))
}

/// The run open in `h`.
fn stage(h: &Harness<Data>) -> &Stage {
    h.screen::<Stage>()
        .unwrap_or_else(|| panic!("no run is open: {:?}", h.screens()))
}

/// The action that turns the snake `dir`.
fn action(dir: Dir) -> UiAction {
    match dir {
        Dir::Up => UiAction::Up,
        Dir::Down => UiAction::Down,
        Dir::Left => UiAction::Left,
        Dir::Right => UiAction::Right,
    }
}

/// One frame of 1/60 s: one step of the rules.
const FRAME: f32 = 1.0 / 60.0;

/// Steers the run open in `h` to its first food and eats it, through
/// actions only: a turn whenever its next move needs one to head for the
/// food, else a frame's wait.
fn eat_first_food(h: &mut Harness<Data>) {
    for _ in 0..2_000 {
        let run = stage(h);
        if run.snake.score() == 1 {
            return;
        }
        let (head, food) = (
            run.snake.head(),
            run.snake.food().unwrap_or(run.snake.head()),
        );
        let heading = run.snake.heading();
        let across = matches!(heading, Dir::Left | Dir::Right);
        // Along a row once in the food's column, along a column once in its
        // row, and off a row whose way leads away from it.
        let turn = if across && head.x == food.x {
            Some(if food.y < head.y { Dir::Up } else { Dir::Down })
        } else if !across && head.y == food.y {
            Some(if food.x < head.x {
                Dir::Left
            } else {
                Dir::Right
            })
        } else if across && (food.x < head.x) == (heading == Dir::Right) {
            Some(if food.y < head.y { Dir::Up } else { Dir::Down })
        } else {
            None
        };
        match turn {
            Some(dir) if run.snake.turns().is_empty() && dir != heading => {
                h.act(action(dir));
            }
            _ => {
                h.wait(FRAME);
            }
        }
    }
    panic!("the snake never reached the food");
}

/// Seed 1 played from the title through actions only: Confirm, the snake
/// steered to its first food and eaten ([`eat_first_food`]), then left to
/// run into a wall. The Harness at the end of the run, and its record.
fn playthrough() -> (Harness<Data>, TickRecord) {
    let mut h = at_title(1);
    h.act(UiAction::Confirm);
    assert_eq!(h.screens(), ["title", "stage"]);
    eat_first_food(&mut h);
    for _ in 0..2_000 {
        if h.top_screen() == "over" {
            break;
        }
        h.wait(FRAME);
    }
    assert_eq!(h.screens(), ["title", "stage", "over"]);
    let record = stage(&h).record().clone();
    (h, record)
}
