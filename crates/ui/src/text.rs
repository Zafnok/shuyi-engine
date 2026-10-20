//! Filling in a text's placeholders: `{name}` by a value the caller gives
//! ([`fill_values`]), or by what the caller looks up for it, such as the
//! key the player has for an action ([`fill`],
//! [`Ctx::text_with`](crate::Ctx::text_with)).

use crate::prelude::*;
use shuyi_assets::keymap::ActionSet;

use crate::input::help_keys::{HelpKeys, cursor_keys_name, key_name};

/// The placeholder for the keys that move the cursor (`{Cursor}`).
pub const CURSOR: &str = "Cursor";

/// What a `{name}` placeholder that names a key says: [`{Cursor}`](CURSOR)
/// the cursor keys, `{Confirm}` (an action's name) the key `keys` has for
/// that action. `None` for any other name. What a game's text fills in by
/// default ([`GameData::key_text`](crate::GameData::key_text)).
pub fn key_text<A: ActionSet>(name: &str, keys: HelpKeys<'_, A>) -> Option<String> {
    if name == CURSOR {
        Some(cursor_keys_name(keys))
    } else {
        A::from_name(name).map(|action| key_name(keys, action))
    }
}
use core::fmt::Display;

/// `text` with each `{name}` that `args` names replaced by its value, and
/// nothing else: for text built where there are no keys to name (a line of
/// a list, a label). Anything else in braces is left alone.
pub fn fill_values(text: &str, args: &[(&str, &dyn Display)]) -> String {
    fill(text, args, |_| None)
}

/// `text` with each `{name}` replaced by its value in `args`, else by what
/// `lookup` gives for it, else left as written. What is filled in is not
/// looked at again, so a value may hold braces. An unclosed `{` and what
/// follows it are plain text.
pub fn fill(
    text: &str,
    args: &[(&str, &dyn Display)],
    lookup: impl Fn(&str) -> Option<String>,
) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some((before, after)) = rest.split_once('{') {
        out.push_str(before);
        let Some((name, tail)) = after.split_once('}') else {
            // An unclosed `{`: the rest is plain text.
            out.push('{');
            rest = after;
            break;
        };
        if let Some((_, value)) = args.iter().find(|(arg, _)| *arg == name) {
            out.push_str(&value.to_string());
        } else if let Some(found) = lookup(name) {
            out.push_str(&found);
        } else {
            out.push('{');
            out.push_str(name);
            out.push('}');
        }
        rest = tail;
    }
    out.push_str(rest);
    out
}

/// Text a view holds before it is shown: a text key with the values of
/// its placeholders, put in the player's language by the skin when it
/// paints ([`Ctx::label`](crate::Ctx::label)), or text as it is. Logic that
/// doesn't know the player's language builds views of keys, and they need
/// no rebuilding when the language changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Label {
    /// The text for `key` in the player's language, with each `{name}` that
    /// `args` names filled by its value (and keys' names filled as
    /// [`Ctx::text_with`](crate::Ctx::text_with) fills them).
    Key {
        /// The text key.
        key: String,
        /// Each placeholder's name and its value, already text.
        args: Vec<(String, String)>,
    },
    /// Text shown as it is: a name the player typed, a number, text the
    /// caller has already translated.
    Text(String),
}

impl Label {
    /// The text for `key`, with no values.
    pub fn key(key: impl Into<String>) -> Self {
        Self::Key {
            key: key.into(),
            args: Vec::new(),
        }
    }

    /// The same label with the placeholder `{name}` filled by `value`. A
    /// [`Text`](Self::Text) label has no placeholders and stays as it is.
    #[must_use]
    pub fn with_arg(mut self, name: impl Into<String>, value: impl Display) -> Self {
        if let Self::Key { args, .. } = &mut self {
            args.push((name.into(), value.to_string()));
        }
        self
    }
}

impl From<String> for Label {
    fn from(text: String) -> Self {
        Self::Text(text)
    }
}

impl From<&str> for Label {
    fn from(text: &str) -> Self {
        Self::Text(text.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn upper(name: &str) -> Option<String> {
        (name == "key").then(|| "K".to_owned())
    }

    #[test]
    fn values_come_first_then_the_lookup_and_the_rest_stays() {
        let text = "press {key} for {count} {things} {}";
        assert_eq!(
            fill(text, &[("count", &3)], upper),
            "press K for 3 {things} {}"
        );
        // A value named like a looked-up placeholder wins.
        assert_eq!(
            fill(text, &[("key", &"Z"), ("things", &"apples")], upper),
            "press Z for {count} apples {}"
        );
        assert_eq!(
            fill_values(text, &[("key", &1)]),
            "press 1 for {count} {things} {}"
        );
    }

    #[test]
    fn a_filled_value_is_not_filled_again() {
        assert_eq!(fill("{a}{b}", &[("a", &"{b}")], upper), "{b}{b}");
        assert_eq!(fill("{a}", &[("a", &"{key}")], upper), "{key}");
    }

    #[test]
    fn an_unclosed_brace_is_plain_text() {
        assert_eq!(fill("a {key", &[], upper), "a {key");
        assert_eq!(fill("{key} {", &[], upper), "K {");
        assert_eq!(fill("} {key}", &[], upper), "} K");
        assert_eq!(fill("", &[], upper), "");
        assert_eq!(fill("no braces", &[], upper), "no braces");
    }
}
