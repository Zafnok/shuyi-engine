//! Filling in a text's placeholders: `{name}` by a value the caller gives
//! ([`fill_values`]), or by what the caller looks up for it, such as the
//! key the player has for an action ([`fill`],
//! [`Ctx::text_with`](crate::Ctx::text_with)), a text being a message that
//! may choose its words by a number's plural or by a value (ADR-0127,
//! [`shuyi_assets::message`]); and [`StyledText`], a line
//! whose spans have roles a skin paints, such as a key's name
//! ([`fill_styled`], [`Ctx::text_styled`](crate::Ctx::text_styled),
//! ADR-0122).

use crate::prelude::*;
use alloc::collections::BTreeMap;
use shuyi_assets::ContentError;
use shuyi_assets::bundle::Bundle;
use shuyi_assets::error::codes;
use shuyi_assets::font::CellWidths;
use shuyi_assets::keymap::ActionSet;
use shuyi_assets::lang::text_width;
pub use shuyi_assets::message::{Arg, ToArg};
use shuyi_assets::message::{Message, Piece};
use shuyi_assets::palette::line_of_key;
use shuyi_assets::text::NO_BREAK_SPACE;
use shuyi_assets::tip::placeholders;
use shuyi_core::text::{Part, split_tokens};

use crate::frame::color::EngineColor;
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

/// The values of a text's placeholders, by name: what [`fill`] and
/// [`Ctx::text_with`](crate::Ctx::text_with) take (`&[("n", &3),
/// ("name", &"Mei")]`).
pub type Args<'a> = [(&'a str, &'a dyn ToArg)];

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
/// a list, a label). Anything else in braces is left alone. `lang` is the
/// language `text` is in, whose plural rules a message follows.
pub fn fill_values(text: &str, lang: &str, args: &Args<'_>) -> String {
    fill(text, lang, args, |_| None)
}

/// `text`, a message in language `lang` ([`Message`]), with each plural,
/// selectordinal and select choosing by its value in `args`, and each
/// `{name}` replaced by its value in `args`, else by what `lookup` gives
/// for it, else left as written. What is filled in is not looked at
/// again, so a value may hold braces. An integer is written in plain
/// digits.
///
/// A text that isn't a message (text from outside the language files,
/// which never panics) is filled as before messages: each `{name}` is
/// filled, and an unclosed `{` and what follows it are plain text.
///
/// What `lookup` gives (a key's name, `Left Shift`) is kept on one line:
/// its spaces become [`NO_BREAK_SPACE`]s, which
/// [`word_wrap`](crate::widgets::word_wrap) never breaks at and a glyph
/// buffer draws as spaces.
///
/// The plain text of [`fill_styled`], with no key names handed in as
/// values.
pub fn fill(
    text: &str,
    lang: &str,
    args: &Args<'_>,
    lookup: impl Fn(&str) -> Option<String>,
) -> String {
    fill_styled(text, lang, args, &[], lookup).plain()
}

/// [`fill`], with what names a key in a [`TextRole::Key`] span: each value
/// `lookup` gives, and each value of `key_args` (a key's name a screen
/// hands in as a value, such as a text box's fixed keys). The values of
/// `args` and the rest of the text are plain. A placeholder `args` names
/// is filled from `args` first, then from `key_args`, then by `lookup`; a
/// plural or select chooses by `args`, then `key_args`. A key's name, from
/// either, is kept on one line: its spaces become [`NO_BREAK_SPACE`]s.
///
/// ```
/// use shuyi_ui::text::{StyledText, TextRole, fill_styled};
/// let key = |name: &str| (name == "Confirm").then(|| "f".to_owned());
/// let text = "{Confirm} pick · {done} done";
/// let filled = fill_styled(text, "en", &[], &[("done", &"Enter")], key);
/// let mut expected = StyledText::new();
/// expected.push_key("f").push_plain(" pick · ").push_key("Enter").push_plain(" done");
/// assert_eq!(filled, expected);
/// assert_eq!(filled.plain(), "f pick · Enter done");
/// ```
pub fn fill_styled(
    text: &str,
    lang: &str,
    args: &Args<'_>,
    key_args: &Args<'_>,
    lookup: impl Fn(&str) -> Option<String>,
) -> StyledText {
    let mut out = StyledText::new();
    let value = |values: &Args<'_>, name: &str| {
        let found = values.iter().find(|(arg, _)| *arg == name);
        found.map(|(_, value)| value.to_arg())
    };
    let arg = |out: &mut StyledText, name: &str| {
        if let Some(plain) = value(args, name) {
            out.push_plain(&plain.to_string());
        } else if let Some(key) = value(key_args, name)
            .map(|key| key.to_string())
            .or_else(|| lookup(name))
        {
            out.push_key(&joined(&key));
        } else {
            out.push_plain(&format!("{{{name}}}"));
        }
    };
    let Ok(message) = Message::parse(text) else {
        for part in split_tokens(text) {
            match part {
                Part::Text(plain) | Part::Unclosed(plain) => {
                    out.push_plain(plain);
                }
                Part::Token(name) => arg(&mut out, name),
            }
        }
        return out;
    };
    let chosen_by = |name: &str| value(args, name).or_else(|| value(key_args, name));
    message.walk(lang, chosen_by, |piece| match piece {
        Piece::Text(plain) => {
            out.push_plain(plain);
        }
        Piece::Number(n) => {
            out.push_plain(&n.to_string());
        }
        Piece::Arg(name) => arg(&mut out, name),
        _ => {}
    });
    out
}

/// `name` with its spaces as [`NO_BREAK_SPACE`]s: a key's name on one line.
fn joined(name: &str) -> String {
    name.chars()
        .map(|c| if c == ' ' { NO_BREAK_SPACE } else { c })
        .collect()
}

/// What a span of a [`StyledText`] is, for the skin to paint it: never its
/// colour, which the palette gives
/// ([`Palette::text_colors`](crate::frame::color::Palette::text_colors)).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TextRole {
    /// A key's or a button's name: the palette's
    /// [`KEY`](crate::frame::color::KEY) and
    /// [`KEY_BG`](crate::frame::color::KEY_BG).
    Key,
    /// One of the engine's colours (a warning, a highlight).
    Engine(EngineColor),
    /// A colour of the game's palette, by its name
    /// ([`ColorSet::name`](crate::frame::color::ColorSet::name)).
    Palette(&'static str),
}

/// A run of text in one [`TextRole`], or in none (plain).
///
/// Kind 3 of ADR-0039: build it with [`TextSpan::new`]; a
/// [`StyledText`] builds its own.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct TextSpan {
    /// Its text.
    pub text: String,
    /// What it is; `None` for plain text.
    pub role: Option<TextRole>,
}

impl TextSpan {
    /// `text` in `role`.
    pub fn new(text: impl Into<String>, role: Option<TextRole>) -> Self {
        Self {
            text: text.into(),
            role,
        }
    }
}

/// A line of text in spans, each with an optional [`TextRole`] that a skin
/// paints from the palette
/// ([`GlyphBuffer::print_styled`](crate::frame::glyph_buffer::GlyphBuffer::print_styled)):
/// a hint whose keys stand out (ADR-0122). A role never changes the text;
/// only a key drawn as a cap takes more cells than its name (ADR-0124).
///
/// Its spans are kept joined: a push next to a span of the same role joins
/// it, and an empty push adds nothing, so two texts that read the same in
/// the same roles are equal.
///
/// ```
/// use shuyi_ui::text::{StyledText, TextRole};
/// let mut hint = StyledText::new();
/// hint.push_key("f").push_plain(" yes").push_plain(" / ");
/// hint.push_key("d").push_plain(" no");
/// assert_eq!(hint.plain(), "f yes / d no");
/// assert_eq!(hint.spans().len(), 4);
/// assert_eq!(hint.spans()[2].role, Some(TextRole::Key));
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct StyledText {
    spans: Vec<TextSpan>,
}

impl StyledText {
    /// No text.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds `text` in `role` at the end.
    pub fn push(&mut self, text: &str, role: Option<TextRole>) -> &mut Self {
        if text.is_empty() {
            return self;
        }
        match self.spans.last_mut() {
            Some(last) if last.role == role => last.text.push_str(text),
            _ => self.spans.push(TextSpan::new(text, role)),
        }
        self
    }

    /// Adds plain `text` at the end.
    pub fn push_plain(&mut self, text: &str) -> &mut Self {
        self.push(text, None)
    }

    /// Adds a key's name at the end, in [`TextRole::Key`].
    pub fn push_key(&mut self, name: &str) -> &mut Self {
        self.push(name, Some(TextRole::Key))
    }

    /// Adds every span of `other` at the end.
    pub fn append(&mut self, other: &StyledText) -> &mut Self {
        for span in &other.spans {
            self.push(&span.text, span.role);
        }
        self
    }

    /// Its spans, in order: none empty, and no two neighbours in the same
    /// role.
    pub fn spans(&self) -> &[TextSpan] {
        &self.spans
    }

    /// Its text, with no roles.
    pub fn plain(&self) -> String {
        self.spans.iter().map(|s| s.text.as_str()).collect()
    }

    /// Whether it has no text.
    pub fn is_empty(&self) -> bool {
        self.spans.is_empty()
    }

    /// How many cells it takes: [`text_width`] of its
    /// [`plain`](Self::plain) text, and where `widths` draw keys as caps
    /// ([`CellWidths::key_cap_cells`], a frame's
    /// [`widths`](crate::frame::GlyphBuffer::widths) with key caps on), each
    /// [`TextRole::Key`] span's extra cells (ADR-0124).
    pub fn width(&self, widths: &CellWidths) -> usize {
        self.spans.iter().map(|s| span_width(s, widths)).sum()
    }

    /// It in at most `w` cells: as [`cut`](crate::frame::draw::cut) cuts
    /// its plain text, each kept character in its role and the `…` in the
    /// role of the first character it stands for. Where `widths` draw
    /// keys as caps, a key is kept whole or not at all, and the `…` that
    /// stands for it is plain.
    #[must_use]
    pub fn cut(&self, w: usize, widths: &CellWidths) -> StyledText {
        if self.width(widths) <= w {
            return self.clone();
        }
        let room = w.saturating_sub(1);
        let mut used = 0;
        let mut out = StyledText::new();
        for span in &self.spans {
            if is_cap(span, widths) {
                used += span_width(span, widths);
                if used > room {
                    if w > 0 {
                        out.push_plain("…");
                    }
                    return out;
                }
                out.push(&span.text, span.role);
                continue;
            }
            for c in span.text.chars() {
                used += widths.of_char(c);
                if used > room {
                    if w > 0 {
                        out.push("…", span.role);
                    }
                    return out;
                }
                let mut buf = [0; 4];
                out.push(c.encode_utf8(&mut buf), span.role);
            }
        }
        out
    }
}

/// Whether `span` is drawn as a key cap with `widths`.
fn is_cap(span: &TextSpan, widths: &CellWidths) -> bool {
    span.role == Some(TextRole::Key) && widths.key_cap_cells().is_some()
}

/// How many cells `span` takes with `widths`: its text's, and a key cap's
/// extra cells.
fn span_width(span: &TextSpan, widths: &CellWidths) -> usize {
    let extra = if is_cap(span, widths) {
        widths.key_cap_cells().map_or(0, usize::from)
    } else {
        0
    };
    text_width(&span.text, widths) + extra
}

impl Display for StyledText {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.spans.iter().try_for_each(|s| f.write_str(&s.text))
    }
}

impl From<String> for StyledText {
    fn from(text: String) -> Self {
        let mut styled = Self::new();
        styled.push_plain(&text);
        styled
    }
}

impl From<&str> for StyledText {
    fn from(text: &str) -> Self {
        let mut styled = Self::new();
        styled.push_plain(text);
        styled
    }
}

/// Text a view holds before it is shown: a text key with the values of
/// its placeholders, put in the player's language by the skin when it
/// paints ([`Ctx::label`](crate::Ctx::label)), or text as it is. Logic that
/// doesn't know the player's language builds views of keys, and they need
/// no rebuilding when the language changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Label {
    /// The text for `key` in the player's language, with each `{name}` that
    /// `args` names filled by its value, and each plural and select
    /// choosing by it (and keys' names filled as
    /// [`Ctx::text_with`](crate::Ctx::text_with) fills them).
    Key {
        /// The text key.
        key: String,
        /// Each placeholder's name and its value: a number stays a number
        /// until the skin paints it, so a plural can choose by it
        /// (ADR-0127).
        args: Vec<(String, Arg)>,
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
    pub fn with_arg(mut self, name: impl Into<String>, value: impl ToArg) -> Self {
        if let Self::Key { args, .. } = &mut self {
            args.push((name.into(), value.to_arg()));
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
            fill(text, "en", &[("count", &3)], upper),
            "press K for 3 {things} {}"
        );
        // A value named like a looked-up placeholder wins.
        assert_eq!(
            fill(text, "en", &[("key", &"Z"), ("things", &"apples")], upper),
            "press Z for {count} apples {}"
        );
        assert_eq!(
            fill_values(text, "en", &[("key", &1)]),
            "press 1 for {count} {things} {}"
        );
    }

    #[test]
    fn a_filled_value_is_not_filled_again() {
        assert_eq!(fill("{a}{b}", "en", &[("a", &"{b}")], upper), "{b}{b}");
        assert_eq!(fill("{a}", "en", &[("a", &"{key}")], upper), "{key}");
    }

    #[test]
    fn a_key_name_of_two_words_is_never_split_across_lines() {
        use shuyi_assets::font::CellWidths;
        let key = |name: &str| (name == "Run").then(|| "Left Shift".to_owned());
        let filled = fill(
            "hold {Run} to run, {who}",
            "en",
            &[("who", &"Ada Brook")],
            key,
        );
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
        assert_eq!(fill("a {key", "en", &[], upper), "a {key");
        assert_eq!(fill("{key} {", "en", &[], upper), "K {");
        assert_eq!(fill("} {key}", "en", &[], upper), "} K");
        assert_eq!(fill("", "en", &[], upper), "");
        assert_eq!(fill("no braces", "en", &[], upper), "no braces");
    }

    #[test]
    fn a_misspelled_action_inside_a_plurals_branch_fails_too() {
        let bad = "{\n  \"a\": \"{n, plural, one {{Confirn} once} other {# times}}\",\n}";
        let errors = checked(bad).unwrap_err();
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(errors[0].to_string().contains("{Confirn}"), "{errors:?}");
        let good = "{\n  \"a\": \"{n, plural, one {{Confirm} once} other {# times}}\",\n}";
        assert_eq!(checked(good), Ok(()));
    }

    proptest::proptest! {
        #[test]
        fn fill_never_panics(text in "[{}#a-z ,=0-9]{0,40}", n in proptest::num::i64::ANY) {
            let args = [("n", &n as &dyn ToArg), ("a", &"x")];
            let _ = fill(&text, "en", &args, upper);
            let _ = fill_styled(&text, "ru", &[], &args, upper);
        }
    }
}

#[cfg(test)]
mod styled_tests;

#[cfg(test)]
mod hint_tests;
