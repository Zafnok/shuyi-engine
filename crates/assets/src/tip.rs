//! One-time contextual tips: what triggers each and what it says. The
//! game's own type names its triggers; its screens decide when one has
//! happened and remember which tips were seen. A tip's text names keys with
//! placeholders (`{Confirm}`), filled in with the player's keys when shown.
use crate::prelude::*;

use alloc::collections::BTreeSet;
use core::fmt::Debug;

use serde::Deserialize;
use serde::de::DeserializeOwned;

use crate::error::ContentError;
use crate::lang::text_width;
use crate::ron::parse_ron;

/// How much text a tip's box holds, in cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TipLimits {
    /// Most text lines a tip may have.
    pub text_lines: usize,
    /// Longest text line, before placeholders are filled in.
    pub line_chars: usize,
    /// Longest tip title.
    pub title_chars: usize,
}

/// One tip; `T` is the game's triggers.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(bound = "T: DeserializeOwned")]
pub struct Tip<T> {
    /// Stable id (`snake_case`); what is remembered once the tip was seen.
    pub id: String,
    /// What makes it appear.
    pub trigger: T,
    /// Shown in the box's border.
    pub title: String,
    /// The body: lines separated by `\n`, with `{Action}` placeholders
    /// for key names.
    pub text: String,
    /// The expression of whoever speaks the tip while it shows, if it
    /// names one. Written `face: "sad"`.
    #[serde(default, deserialize_with = "some_name")]
    pub face: Option<String>,
}

/// A name written plainly (`face: "sad"`, not `Some("sad")`).
fn some_name<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
    String::deserialize(d).map(Some)
}

impl<T> Tip<T> {
    /// The expression while the tip shows: its own, else `default`.
    pub fn face_or<'a>(&'a self, default: &'a str) -> &'a str {
        self.face.as_deref().unwrap_or(default)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(bound = "T: DeserializeOwned")]
struct RawFile<T> {
    tips: Vec<Tip<T>>,
}

/// Every tip, in file order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TipTable<T> {
    /// The tips.
    pub tips: Vec<Tip<T>>,
}

impl<T> Default for TipTable<T> {
    fn default() -> Self {
        Self { tips: Vec::new() }
    }
}

impl<T: PartialEq + Copy> TipTable<T> {
    /// The tip that `trigger` shows, if the file has one.
    pub fn for_trigger(&self, trigger: T) -> Option<&Tip<T>> {
        self.tips.iter().find(|t| t.trigger == trigger)
    }
}

/// What [`from_source`] checks tips against besides their box: which
/// placeholders exist, and what is wrong with a tip's face.
pub struct TipRules<'a> {
    /// How much text the box holds.
    pub limits: TipLimits,
    /// Whether `{name}` is a placeholder the game fills in.
    pub placeholder: &'a dyn Fn(&str) -> bool,
    /// What is wrong with a tip showing the face `face` (its own, or the
    /// game's default), if anything.
    pub face_problem: &'a dyn Fn(&str) -> Option<String>,
    /// The face of a tip that names none.
    pub default_face: &'a str,
}

/// Parses and validates tips `source`, attributing errors to `file`: ids
/// are lowercase and used once, a trigger has one tip, the title and text
/// fit `rules`' box, every placeholder is one of the game's and every face
/// is fine by the game. Reports every problem found.
pub fn from_source<T: DeserializeOwned + Debug + Ord>(
    file: &str,
    source: &str,
    rules: &TipRules<'_>,
) -> Result<TipTable<T>, Vec<ContentError>> {
    let TipLimits {
        text_lines,
        line_chars,
        title_chars,
    } = rules.limits;
    let raw: RawFile<T> = parse_ron(file, source).map_err(|e| vec![e])?;
    let mut errors = Vec::new();
    let mut ids = BTreeSet::new();
    let mut triggers = BTreeSet::new();
    for tip in &raw.tips {
        let mut err = |message: String| {
            errors.push(ContentError::new(
                file,
                format!("tip \"{}\": {message}", tip.id),
            ));
        };
        let id_ok = !tip.id.is_empty()
            && tip
                .id
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_');
        if !id_ok {
            err("the id must be lowercase letters, digits and `_`".into());
        }
        if !ids.insert(tip.id.as_str()) {
            err("the id is used twice".into());
        }
        if !triggers.insert(&tip.trigger) {
            err(format!("{:?} already has a tip", tip.trigger));
        }
        if tip.title.is_empty() || text_width(&tip.title) > title_chars {
            err(format!("the title must be 1 to {title_chars} characters"));
        }
        let lines: Vec<&str> = tip.text.lines().collect();
        if lines.is_empty() || lines.len() > text_lines {
            err(format!("the text must have 1 to {text_lines} lines"));
        }
        for line in lines {
            if text_width(line) > line_chars {
                err(format!(
                    "a text line is over {line_chars} characters: {line:?}"
                ));
            }
        }
        for name in placeholders(&tip.text) {
            if !(rules.placeholder)(name) {
                err(format!("unknown placeholder {{{name}}}"));
            }
        }
        if let Some(problem) = (rules.face_problem)(tip.face_or(rules.default_face)) {
            err(problem);
        }
    }
    if errors.is_empty() {
        Ok(TipTable { tips: raw.tips })
    } else {
        Err(errors)
    }
}

/// The names between braces in `text`, in order (`{Confirm}` gives
/// `Confirm`). An unclosed `{` is ignored.
pub fn placeholders(text: &str) -> Vec<&str> {
    let mut names = Vec::new();
    let mut rest = text;
    while let Some((_, after)) = rest.split_once('{') {
        let Some((name, tail)) = after.split_once('}') else {
            break;
        };
        names.push(name);
        rest = tail;
    }
    names
}

#[cfg(test)]
mod tests;
