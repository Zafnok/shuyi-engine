//! Step 7's checks (`07-save`): the exercise of its page, *Saving
//! progress*, done in its playground.

#[cfg(test)]
#[path = "edits.rs"]
mod edits;

use game::Data;
use game::action::Action;
use shuyi::app::PageCheck;

/// Every check of the step, by the name its page's exercises give.
pub const CHECKS: &[PageCheck<Action, game::stages::Own>] = &[("yard-first", yard_first)];

/// The name of the level the exercise puts first.
const YARD: &str = "The yard";

/// The yard is the first level, and the list still holds all five.
fn yard_first(data: &Data) -> Result<String, String> {
    let names: Vec<&str> = data
        .own
        .levels
        .iter()
        .map(|level| level.name.as_str())
        .collect();
    if names.len() != 5 {
        return Err(format!(
            "The list in levels.ron has {} levels: keep all five, in a new order.",
            names.len()
        ));
    }
    match names.iter().position(|name| *name == YARD) {
        Some(0) => Ok(format!(
            "The yard comes first, then {}.",
            names[1..].join(", ")
        )),
        Some(at) => Err(format!(
            "\"{YARD}\" (levels/5.map) is level {} of the list: move its line to the top.",
            at + 1
        )),
        None => Err(format!(
            "No level of the list is \"{YARD}\": put levels/5.map back, first."
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_yard_first_passes() {
        let own = yard_first(&edits::own()).unwrap_err();
        assert!(own.contains("is level 5 of the list"), "{own}");
        let data = edits::edited(
            "levels.ron",
            &[
                ("    \"levels/5.map\",\n", ""),
                ("[\n", "[\n    \"levels/5.map\",\n"),
            ],
        );
        assert_eq!(
            yard_first(&data),
            Ok(
                "The yard comes first, then First push, Two rows, Round the corner, Side door."
                    .to_owned()
            )
        );
    }

    #[test]
    fn a_list_without_all_five_fails() {
        let data = edits::edited("levels.ron", &[("    \"levels/5.map\",\n", "")]);
        let failed = yard_first(&data).unwrap_err();
        assert!(failed.contains("has 4 levels"), "{failed}");
    }
}
