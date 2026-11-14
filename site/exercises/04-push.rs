//! Step 4's checks (`04-push`): the exercise of its page, *Pushing*, done
//! in its playground.

#[cfg(test)]
#[path = "edits.rs"]
mod edits;
#[path = "solve.rs"]
mod solve;

use game::Data;
use game::rules::{Command, Puzzle};
use shuyi::app::PageCheck;
use shuyi::core::Dir;
use shuyi::prelude::*;

use self::solve::Solved;

/// Every check of the step, by the name its page's exercises give.
pub const CHECKS: &[PageCheck<UiAction, game::stages::Own>] = &[("eight-moves", eight_moves)];

/// The fewest moves that win the exercise's level.
const FEWEST: usize = 8;

/// Level 1 can be won, and takes at least [`FEWEST`] moves: every way of
/// playing it is searched through the step's rules.
fn eight_moves(data: &Data) -> Result<String, String> {
    let puzzle = Puzzle::new(&data.own.level)?;
    match solve::solve(&puzzle, |p| (p.player(), p.boxes().clone())) {
        Solved::Won(moves) if moves.len() >= FEWEST => Ok(format!(
            "Level 1 takes {} moves at the fewest.",
            moves.len()
        )),
        Solved::Won(moves) => Err(format!(
            "Level 1 can be won in {} moves: {}. Move the box or its target further apart, or put walls in the way.",
            moves.len(),
            spoken(&moves)
        )),
        Solved::Never => {
            Err("Level 1 can't be won at all: the box can never reach its target.".to_owned())
        }
        Solved::TooBig => Err(format!(
            "Level 1 has too many ways to play to look through ({} positions): make it smaller.",
            solve::LIMIT
        )),
    }
}

/// `moves` as words: "right, right".
fn spoken(moves: &[Command]) -> String {
    let words: Vec<&str> = moves
        .iter()
        .map(|Command::Step(dir)| match dir {
            Dir::Up => "up",
            Dir::Down => "down",
            Dir::Left => "left",
            Dir::Right => "right",
        })
        .collect();
    words.join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_level_of_eight_moves_or_more_passes() {
        assert_eq!(
            eight_moves(&edits::own()),
            Err(
                "Level 1 can be won in 2 moves: right, right. Move the box or its target further apart, or put walls in the way."
                    .to_owned()
            )
        );
        // The target in the top left corner, a wall beside it: the box
        // comes up from below.
        let data = edits::edited(
            "levels/1.map",
            &[
                ("#######\n#.....#", "#######\n#o#...#"),
                ("#....o#", "#.....#"),
            ],
        );
        assert_eq!(
            eight_moves(&data),
            Ok("Level 1 takes 9 moves at the fewest.".to_owned())
        );
    }

    #[test]
    fn a_level_that_cant_be_won_fails() {
        // The target walled in.
        let data = edits::edited("levels/1.map", &[("#....o#\n#.....#", "#...#o#\n#....##")]);
        assert_eq!(
            eight_moves(&data),
            Err("Level 1 can't be won at all: the box can never reach its target.".to_owned())
        );
    }
}
