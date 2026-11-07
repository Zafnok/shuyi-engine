//! Filling in a text's placeholders: `{name}` by a value the caller gives
//! ([`fill_values`]), or by what the caller looks up for it, such as the
//! key the player has for an action ([`fill`],
//! [`Ctx::text_with`](crate::Ctx::text_with)).

use crate::prelude::*;
use alloc::collections::BTreeMap;
use shuyi_assets::ContentError;
use shuyi_assets::bundle::Bundle;
use shuyi_assets::error::codes;
use shuyi_assets::keymap::ActionSet;
use shuyi_assets::palette::line_of_key;
use shuyi_assets::text::NO_BREAK_SPACE;
use shuyi_assets::tip::placeholders;

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

/// Checks that every placeholder of screen text `text`, read from `path`
/// in `bundle`, can be filled: a capitalized `{Name}` names a key, so it
/// must be [`{Cursor}`](CURSOR) or an action of `A`, as [`key_text`]
/// fills them. A lowercase `{name}` is a value a screen passes
/// ([`Ctx::text_with`](crate::Ctx::text_with)'s `args`), which no file
/// can know, and passes. Each error names the file, the key's line and
/// the known actions, with code `SH2008` (`unknown-name`).
///
/// # Errors
///
/// Every placeholder that names no key, each its own error.
pub fn check_key_placeholders<A: ActionSet>(
    bundle: &Bundle,
    path: &str,
    text: &BTreeMap<String, String>,
) -> Result<(), Vec<ContentError>> {
    let source = bundle.file(path).unwrap_or_default();
    let file = bundle.display_path(path);
    let mut errors = Vec::new();
    for (key, value) in text {
        for name in placeholders(value) {
            let names_a_key = name.chars().next().is_some_and(char::is_uppercase);
            if !names_a_key || name == CURSOR || A::from_name(name).is_some() {
                continue;
            }
            let known: Vec<&str> = A::ALL.iter().map(|a| a.name()).collect();
            let message = format!(
                "\"{key}\": unknown action \"{{{name}}}\"; known actions: {}, and {{{CURSOR}}} \
                 for the cursor keys",
                known.join(", ")
            );
            let mut error = ContentError::new(&file, message).with_code(codes::UNKNOWN_NAME);
            if let Some(line) = line_of_key(source, key) {
                error = error.at(line, None);
            }
            errors.push(error);
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

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
///
/// What `lookup` gives (a key's name, `Left Shift`) is kept on one line:
/// its spaces become [`NO_BREAK_SPACE`]s, which
/// [`word_wrap`](crate::widgets::word_wrap) never breaks at and a glyph
/// buffer draws as spaces.
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
            out.extend(
                found
                    .chars()
                    .map(|c| if c == ' ' { NO_BREAK_SPACE } else { c }),
            );
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
    fn a_key_name_of_two_words_is_never_split_across_lines() {
        use shuyi_assets::font::CellWidths;
        let key = |name: &str| (name == "Run").then(|| "Left Shift".to_owned());
        let filled = fill("hold {Run} to run, {who}", &[("who", &"Ada Brook")], key);
        // The key's name is joined; a value given is not.
        assert_eq!(filled, "hold Left\u{a0}Shift to run, Ada Brook");
        for width in 10..20 {
            let lines = crate::widgets::word_wrap(&filled, width, &CellWidths::NARROW);
            assert!(
                lines.iter().any(|l| l.contains("Left\u{a0}Shift")),
                "{width}: {lines:?}"
            );
        }
    }

    /// `check_key_placeholders` on `en.ron` written `source`, in `files/`.
    fn checked(source: &'static str) -> Result<(), Vec<ContentError>> {
        use shuyi_assets::bundle::Layer;
        use shuyi_assets::keymap::UiAction;
        let files: &'static [(&str, &[u8])] = Box::leak(Box::new([("en.ron", source.as_bytes())]));
        let bundle = Bundle::new(vec![Layer::table("files", files)]);
        let text: shuyi_assets::lang::English = ron::from_str(source).unwrap();
        check_key_placeholders::<UiAction>(&bundle, "en.ron", &text.0)
    }

    #[test]
    fn actions_the_cursor_and_values_a_screen_passes_are_placeholders() {
        let good = "{\n  \"a\": \"{Confirm} plant {seeds} {Cursor} move\",\n  \"b\": \"{} {\",\n}";
        assert_eq!(checked(good), Ok(()));
    }

    #[test]
    fn a_misspelled_action_fails_with_its_file_line_and_the_known_actions() {
        let bad = "{\n  \"a\": \"{Confirm} ok\",\n  \"title.help\": \"{Confirn} plant\",\n}";
        let errors = checked(bad).unwrap_err();
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert_eq!(errors[0].code, Some(codes::UNKNOWN_NAME));
        assert_eq!(
            errors[0].to_string(),
            "files/en.ron:3: \"title.help\": unknown action \"{Confirn}\"; known actions: \
             Up, Down, Left, Right, Confirm, Cancel, and {Cursor} for the cursor keys"
        );
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
