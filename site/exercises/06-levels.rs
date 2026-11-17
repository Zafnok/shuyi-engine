//! Step 6's checks (`06-levels`): the exercise of its page, *Levels and a
//! menu*, done in its playground.

#[cfg(test)]
#[path = "edits.rs"]
mod edits;
#[path = "solve.rs"]
mod solve;

use game::Data;
use game::action::Action;
use game::level::Feature;
use game::rules::Puzzle;
use shuyi::app::PageCheck;

use self::solve::Solved;

/// Every check of the step, by the name its page's exercises give.
pub const CHECKS: &[PageCheck<Action, game::stages::Own>] = &[("three-boxes", three_boxes)];

/// The second level of the list has three boxes, and can still be won:
/// every way of playing it is searched through the step's rules.
fn three_boxes(data: &Data) -> Result<String, String> {
    let level = data
        .own
        .levels
        .get(1)
        .ok_or("The list in levels.ron has one level: level 2 is its second.")?;
    let boxes = level
        .features
        .values()
        .filter(|feature| **feature == Feature::Box)
        .count();
    if boxes != 3 {
        return Err(format!(
            "Level 2, \"{}\", has {boxes} boxes, where the exercise asks for three.",
            level.name
        ));
    }
    let puzzle = Puzzle::new(level)?;
    match solve::solve(&puzzle, |p| (p.player(), p.boxes().clone())) {
        Solved::Won(moves) => Ok(format!(
            "Level 2 has three boxes, and can be won in {} moves.",
            moves.len()
        )),
        Solved::Never => Err(
            "Level 2 has three boxes, but can't be won: a box can't reach a target. Leave room to push each one."
                .to_owned(),
        ),
        Solved::TooBig => Err(format!(
            "Level 2 has too many ways to play to look through ({} positions): make it smaller.",
            solve::LIMIT
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_third_box_that_can_be_pushed_home_passes() {
        let own = three_boxes(&edits::own()).unwrap_err();
        assert!(own.contains("has 2 boxes"), "{own}");
        let data = edits::edited(
            "levels/2.map",
            &[
                ("(3, 4): Box }", "(3, 4): Box, (3, 3): Box }"),
                (
                    "#......#\n#....o.#\n#......#\n#....o.#",
                    "#......#\n#....o.#\n#....o.#\n#....o.#",
                ),
            ],
        );
        let passed = three_boxes(&data);
        assert!(
            passed
                .as_ref()
                .is_ok_and(|text| text.starts_with("Level 2 has three boxes, and can be won in")),
            "{passed:?}"
        );
    }

    #[test]
    fn three_boxes_that_cant_be_won_fail() {
        // The third box in a corner, which no push gets out of.
        let data = edits::edited(
            "levels/2.map",
            &[
                ("(3, 4): Box }", "(3, 4): Box, (1, 1): Box }"),
                (
                    "#......#\n#....o.#\n#......#\n#....o.#",
                    "#......#\n#....o.#\n#....o.#\n#....o.#",
                ),
            ],
        );
        let failed = three_boxes(&data).unwrap_err();
        assert!(failed.contains("can't be won"), "{failed}");
    }
}
