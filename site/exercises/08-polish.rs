//! Step 8's checks (`08-polish`): the exercise of its page, *Polish*, done
//! in its playground.

#[path = "color.rs"]
mod color;
#[cfg(test)]
#[path = "edits.rs"]
mod edits;

use game::Data;
use game::action::Action;
use shuyi::app::PageCheck;
use shuyi::prelude::*;

/// Every check of the step, by the name its page's exercises give.
pub const CHECKS: &[PageCheck<Action, game::stages::Own>] = &[("yellow-player", yellow_player)];

/// The player is drawn yellow.
fn yellow_player(data: &Data) -> Result<String, String> {
    let player = data
        .palette_def()
        .get("player")
        .ok_or("The palette has no `player`.")?;
    if color::name(player) == Some("yellow") {
        Ok(format!("The player is {}, a yellow.", color::hex(player)))
    } else {
        Err(format!(
            "`player` is {}, not yellow: make its first two pairs, red and green, both high, and its last, blue, low.",
            color::hex(player)
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_yellow_player_passes() {
        let own = yellow_player(&edits::own()).unwrap_err();
        assert!(own.starts_with("`player` is #ffffff, not yellow"), "{own}");
        let data = edits::edited(
            "palette.ron",
            &[(r##""player": "#ffffff","##, r##""player": "#ffe040","##)],
        );
        assert_eq!(
            yellow_player(&data),
            Ok("The player is #ffe040, a yellow.".to_owned())
        );
    }
}
