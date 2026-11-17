//! Step 10's checks (`10-ship`): the exercise of its page, *Shipping it*,
//! done in its playground.

#[cfg(test)]
#[path = "edits.rs"]
mod edits;

use game::Data;
use game::action::Action;
use shuyi::app::PageCheck;
use shuyi::prelude::*;

/// Every check of the step, by the name its page's exercises give.
pub const CHECKS: &[PageCheck<Action, game::stages::Own>] = &[("made-by", made_by)];

/// The most characters the subtitle may have: the screen is 48 cells wide,
/// and its box leaves a margin.
const WIDEST: usize = 40;

/// The title's subtitle says who made the game: "by" and a name, short
/// enough to fit.
fn made_by(data: &Data) -> Result<String, String> {
    let subtitle = data
        .lang()
        .source_text("title.subtitle")
        .ok_or("en.ron has no `title.subtitle`.")?;
    let Some(name) = subtitle.strip_prefix("by ") else {
        return Err(format!(
            "`title.subtitle` is \"{subtitle}\": make it start with `by `, then your name."
        ));
    };
    if name.trim().is_empty() {
        return Err("`title.subtitle` says `by `, and no name after it.".to_owned());
    }
    let length = subtitle.chars().count();
    if length > WIDEST {
        return Err(format!(
            "`title.subtitle` is {length} characters: keep it to {WIDEST}, so it fits the title's screen."
        ));
    }
    Ok(format!("The title says \"{subtitle}\"."))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The data with the subtitle `to`.
    fn subtitled(to: &str) -> Data {
        let to = format!(r#""title.subtitle": "{to}","#);
        edits::edited(
            "en.ron",
            &[(r#""title.subtitle": "made with Shuyi Engine","#, &to)],
        )
    }

    #[test]
    fn by_and_a_name_passes() {
        let own = made_by(&edits::own()).unwrap_err();
        assert!(own.contains("make it start with `by `"), "{own}");
        assert_eq!(
            made_by(&subtitled("by Sam Rivera")),
            Ok("The title says \"by Sam Rivera\".".to_owned())
        );
        let none = made_by(&subtitled("by ")).unwrap_err();
        assert!(none.contains("no name after it"), "{none}");
        let long = made_by(&subtitled(&format!("by {}", "a".repeat(40)))).unwrap_err();
        assert!(long.contains("is 43 characters"), "{long}");
    }
}
