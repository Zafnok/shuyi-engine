//! Step 2's checks (`02-board`): the exercises of its page, *A level on the
//! board*, done in its playground.

#[path = "color.rs"]
mod color;
#[cfg(test)]
#[path = "edits.rs"]
mod edits;

use game::Data;
use game::level::Feature;
use shuyi::app::PageCheck;
use shuyi::prelude::*;

/// Every check of the step, by the name its page's exercises give.
pub const CHECKS: &[PageCheck<UiAction, game::stages::Own>] =
    &[("second-box", second_box), ("blue-walls", blue_walls)];

/// Level 1 has a second box (and, since it loaded, a target for it).
fn second_box(data: &Data) -> Result<String, String> {
    let boxes = data
        .own
        .level
        .features
        .values()
        .filter(|feature| **feature == Feature::Box)
        .count();
    match boxes {
        2 => Ok("Level 1 has two boxes and a target for each.".to_owned()),
        1 => Err(
            "Level 1 still has one box: add a second to its `features`, and an `o` for it to go on."
                .to_owned(),
        ),
        n => Err(format!(
            "Level 1 has {n} boxes, where the exercise asks for two."
        )),
    }
}

/// The walls' two colors, `wall_fg` and `wall_bg`, are both blue.
fn blue_walls(data: &Data) -> Result<String, String> {
    let palette = data.palette_def();
    for name in ["wall_fg", "wall_bg"] {
        let Some(color) = palette.get(name) else {
            return Err(format!("The palette has no `{name}`."));
        };
        if color::name(color) != Some("blue") {
            return Err(format!(
                "`{name}` is {}, not blue: make its last two digits, its blue, well above the four before them, its red and green.",
                color::hex(color)
            ));
        }
    }
    Ok("Both of the walls' colors are blue.".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_second_box_and_its_target_pass() {
        let own = second_box(&edits::own()).unwrap_err();
        assert!(own.starts_with("Level 1 still has one box"), "{own}");
        let data = edits::edited(
            "levels/1.map",
            &[
                ("(3, 2): Box }", "(3, 2): Box, (3, 3): Box }"),
                ("#.....#\n#######", "#....o#\n#######"),
            ],
        );
        assert_eq!(
            second_box(&data),
            Ok("Level 1 has two boxes and a target for each.".to_owned())
        );
    }

    #[test]
    fn blue_walls_pass() {
        let own = blue_walls(&edits::own()).unwrap_err();
        assert!(own.starts_with("`wall_fg` is #5a5a6e"), "{own}");
        let data = edits::edited(
            "palette.ron",
            &[
                (r##""wall_fg": "#5a5a6e""##, r##""wall_fg": "#5a7aff""##),
                (r##""wall_bg": "#26262e""##, r##""wall_bg": "#10206e""##),
            ],
        );
        assert_eq!(
            blue_walls(&data),
            Ok("Both of the walls' colors are blue.".to_owned())
        );
    }
}
