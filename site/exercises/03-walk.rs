//! Step 3's checks (`03-walk`): the exercise of its page, *Walking*, done
//! in its playground.

#[cfg(test)]
#[path = "edits.rs"]
mod edits;

use game::Data;
use game::rules::{Command, Puzzle, Refusal};
use shuyi::app::PageCheck;
use shuyi::core::{Dir, TurnRules};
use shuyi::prelude::*;

/// Every check of the step, by the name its page's exercises give.
pub const CHECKS: &[PageCheck<UiAction, game::stages::Own>] = &[("wall-right", wall_right)];

/// A first step right, from the start of level 1, is refused by a wall:
/// played through the step's rules.
fn wall_right(data: &Data) -> Result<String, String> {
    let mut puzzle = Puzzle::new(&data.own.level)?;
    match puzzle.apply(&Command::Step(Dir::Right)) {
        Err(Refusal::Wall) => Ok("A first step right bumps into the wall.".to_owned()),
        Err(Refusal::Box) => Err(
            "A first step right bumps into the box, not a wall: move the box elsewhere in `features`, then put a `#` right of the start."
                .to_owned(),
        ),
        Ok(_) => Err(
            "A first step right walks onto floor: put a `#` in the row of the start, right beside it."
                .to_owned(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_wall_right_of_the_start_passes() {
        let own = wall_right(&edits::own()).unwrap_err();
        assert!(own.contains("bumps into the box"), "{own}");
        let data = edits::edited(
            "levels/1.map",
            &[("(3, 2): Box }", "(3, 3): Box }"), ("#....o#", "#..#.o#")],
        );
        assert_eq!(
            wall_right(&data),
            Ok("A first step right bumps into the wall.".to_owned())
        );
        let moved = edits::edited("levels/1.map", &[("(3, 2): Box }", "(3, 3): Box }")]);
        let floor = wall_right(&moved).unwrap_err();
        assert!(floor.contains("walks onto floor"), "{floor}");
    }
}
