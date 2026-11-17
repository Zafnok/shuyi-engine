//! Step 1's checks (`01-new`): the exercises of its page, *A new game*,
//! done in its playground.

#[cfg(test)]
#[path = "edits.rs"]
mod edits;

use game::Data;
use shuyi::app::PageCheck;
use shuyi::assets::keymap::Chord;
use shuyi::prelude::*;

/// Every check of the step, by the name its page's exercises give.
pub const CHECKS: &[PageCheck<UiAction, game::stages::Own>] =
    &[("confirm-key", confirm_key), ("quit-word", quit_word)];

/// Confirm has the key `p` among its keys.
fn confirm_key(data: &Data) -> Result<String, String> {
    let keymap = data.keymap_def();
    let layout = keymap.first_layout().ok_or("The keymap has no layout.")?;
    let p = Chord::parse("p").map_err(|e| format!("`p` isn't a key: {e}"))?;
    let keys = keymap.chords(layout, UiAction::Confirm);
    if keys.contains(&p) {
        Ok("Confirm has `p` among its keys: pressing it starts the game.".to_owned())
    } else {
        let names: Vec<String> = keys.iter().map(|key| format!("`{key}`")).collect();
        Err(format!(
            "Confirm's keys are {}, without `p`: add `\"p\"` to its list in `keys`.",
            names.join(", ")
        ))
    }
}

/// The title's help line says "quit", and still names the key.
fn quit_word(data: &Data) -> Result<String, String> {
    let help = data
        .lang()
        .source_text("title.help")
        .ok_or("en.ron has no `title.help`.")?;
    if !help.contains("{Cancel}") {
        return Err(format!(
            "`title.help` is \"{help}\": put `{{Cancel}}` back in it, which shows the player's key for Cancel."
        ));
    }
    if !help.contains("quit") {
        return Err(format!(
            "`title.help` is \"{help}\": change its word `leave` to `quit`."
        ));
    }
    Ok(format!("The title's help line reads \"{help}\"."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn p_among_confirms_keys_passes() {
        let own = confirm_key(&edits::own()).unwrap_err();
        assert!(own.contains("without `p`"), "{own}");
        let data = edits::edited(
            "keymap.ron",
            &[(
                r#""Confirm": ["Enter", "Space", "z"],"#,
                r#""Confirm": ["Enter", "Space", "p"],"#,
            )],
        );
        assert!(confirm_key(&data).is_ok(), "{:?}", confirm_key(&data));
    }

    #[test]
    fn quit_in_place_of_leave_passes() {
        let own = quit_word(&edits::own()).unwrap_err();
        assert!(own.contains("change its word `leave`"), "{own}");
        let edited = |to: &str| {
            let to = format!(r#""title.help": "{to}","#);
            edits::edited("en.ron", &[(r#""title.help": "{Cancel} leave","#, &to)])
        };
        assert_eq!(
            quit_word(&edited("{Cancel} quit")),
            Ok("The title's help line reads \"{Cancel} quit\".".to_owned())
        );
        let lost = quit_word(&edited("quit")).unwrap_err();
        assert!(lost.contains("put `{Cancel}` back"), "{lost}");
    }
}
