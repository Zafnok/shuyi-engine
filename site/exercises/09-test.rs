//! Step 9's checks (`09-test`): the exercise of its page, *Bots, replays and
//! a frame*, done in its playground.

#[cfg(test)]
#[path = "edits.rs"]
mod edits;
#[path = "solve.rs"]
mod solve;

use game::Data;
use game::action::Action;
use game::rules::Puzzle;
use shuyi::app::PageCheck;

use self::solve::Solved;

/// Every check of the step, by the name its page's exercises give.
pub const CHECKS: &[PageCheck<Action, game::stages::Own>] = &[("cant-win", cant_win)];

/// The first level loads, and no way of playing it wins: every one is
/// searched through the step's rules, as a bot would play them all.
fn cant_win(data: &Data) -> Result<String, String> {
    let level = data
        .own
        .levels
        .first()
        .ok_or("The list in levels.ron is empty.")?;
    let puzzle = Puzzle::new(level)?;
    match solve::solve(&puzzle, |p| (p.player(), p.boxes().clone())) {
        Solved::Never => Ok(format!(
            "No way of playing \"{}\" wins it, and the game still loads it.",
            level.name
        )),
        Solved::Won(moves) => Err(format!(
            "\"{}\" can still be won, in {} moves. Trap a box where no push reaches its target: in a corner, or behind walls.",
            level.name,
            moves.len()
        )),
        Solved::TooBig => Err(format!(
            "\"{}\" has too many ways to play to look through ({} positions): make it smaller.",
            level.name,
            solve::LIMIT
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_first_level_that_cant_be_won_passes() {
        let own = cant_win(&edits::own()).unwrap_err();
        assert!(
            own.starts_with("\"First push\" can still be won, in 2 moves"),
            "{own}"
        );
        // The box in the top right corner.
        let data = edits::edited("levels/1.map", &[("(3, 2): Box }", "(5, 1): Box }")]);
        assert_eq!(
            cant_win(&data),
            Ok("No way of playing \"First push\" wins it, and the game still loads it.".to_owned())
        );
    }
}
