//! Step 5's checks (`05-undo`): the exercise of its page, *Undo*, done in
//! its playground.

#[cfg(test)]
#[path = "edits.rs"]
mod edits;

use game::Data;
use game::action::Action;
use shuyi::app::PageCheck;
use shuyi::prelude::*;

/// Every check of the step, by the name its page's exercises give.
pub const CHECKS: &[PageCheck<Action, game::stages::Own>] = &[("undo-two-keys", undo_two_keys)];

/// Undo has two keys, or more.
fn undo_two_keys(data: &Data) -> Result<String, String> {
    let keymap = data.keymap_def();
    let layout = keymap.first_layout().ok_or("The keymap has no layout.")?;
    let keys: Vec<String> = keymap
        .chords(layout, Action::Undo)
        .iter()
        .map(|key| format!("`{key}`"))
        .collect();
    if keys.len() >= 2 {
        Ok(format!(
            "Undo has {} keys: {}.",
            keys.len(),
            keys.join(", ")
        ))
    } else {
        Err(format!(
            "Undo has one key, {}: add a second to its list, one no other action has.",
            keys.join(", ")
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_second_key_for_undo_passes() {
        let own = undo_two_keys(&edits::own()).unwrap_err();
        assert!(own.starts_with("Undo has one key, `u`"), "{own}");
        let data = edits::edited(
            "keymap.ron",
            &[(r#""Undo": ["u"],"#, r#""Undo": ["u", "r"],"#)],
        );
        assert_eq!(
            undo_two_keys(&data),
            Ok("Undo has 2 keys: `u`, `r`.".to_owned())
        );
    }
}
