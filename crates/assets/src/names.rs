//! A names table: every display name of a story (characters, places,
//! factions, terms) by stable id, so a rename is one line. Text refers to a
//! name with a token, `{n:<id>}` (or `{N:<id>}` for a capital first
//! letter), filled in by [`Names::substitute`] when it is shown.
use crate::prelude::*;

use alloc::borrow::Cow;
use alloc::collections::BTreeMap;

use shuyi_core::text::{Part, split_tokens};

use crate::dialogue::char_problem;
use crate::error::{ContentError, line_of};
use crate::lang::text_width;
use crate::ron::parse_ron;

/// Every display name, by name id.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Names {
    /// Display names by id.
    pub names: BTreeMap<String, String>,
}

/// A name token: the inside of `{n:<id>}` or `{N:<id>}`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NameToken<'a> {
    /// The name id.
    pub id: &'a str,
    /// Whether the name's first letter is capitalised (`{N:...}`).
    pub capital: bool,
}

/// The name token that `token` (the text between the braces) is, if it
/// starts with `n:` or `N:`. The id may be anything; see [`is_name_id`].
pub fn name_token(token: &str) -> Option<NameToken<'_>> {
    if let Some(id) = token.strip_prefix("n:") {
        return Some(NameToken { id, capital: false });
    }
    token
        .strip_prefix("N:")
        .map(|id| NameToken { id, capital: true })
}

/// Whether `id` is a valid name id: lowercase ASCII letters, digits, `_`
/// and `.`, not empty.
pub fn is_name_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'.')
}

impl Names {
    /// The display name with id `id`.
    pub fn get(&self, id: &str) -> Option<&str> {
        self.names.get(id).map(String::as_str)
    }

    /// Cells of the longest display name (0 for an empty table): what
    /// every name token counts as when measuring text, so a rename can't
    /// push a line past its limit.
    pub fn longest(&self) -> usize {
        self.names
            .values()
            .map(|n| text_width(n))
            .max()
            .unwrap_or(0)
    }

    /// What name token `token` becomes, if it is one with a known id.
    fn token(&self, token: &str) -> Option<Cow<'_, str>> {
        let t = name_token(token)?;
        let name = self.get(t.id)?;
        Some(if t.capital {
            Cow::Owned(capitalise(name))
        } else {
            Cow::Borrowed(name)
        })
    }

    /// `text` with every name token replaced by its display name. Other
    /// tokens (the game's) and unknown ids are left as written (the
    /// dialogue validator rejects unknown ones).
    pub fn substitute<'t>(&self, text: &'t str) -> Cow<'t, str> {
        if !text.contains('{') {
            return Cow::Borrowed(text);
        }
        let mut out = String::with_capacity(text.len());
        for part in split_tokens(text) {
            match part {
                Part::Token(t) => {
                    if let Some(name) = self.token(t) {
                        out.push_str(&name);
                    } else {
                        out.push('{');
                        out.push_str(t);
                        out.push('}');
                    }
                }
                Part::Text(t) | Part::Unclosed(t) => out.push_str(t),
            }
        }
        Cow::Owned(out)
    }

    /// The longest display name written literally in `text` (outside
    /// tokens), with its id; of equally long ones, the first by id. The
    /// longest, so that a full name is reported as itself and not as one
    /// of its short forms (`Ada Brook` is `smith`, not `family.brook`).
    /// See [`literal_form`] for what counts.
    pub fn literal_in(&self, text: &str) -> Option<(&str, &str)> {
        let plain: Vec<&str> = split_tokens(text)
            .filter_map(|part| match part {
                Part::Text(t) | Part::Unclosed(t) => Some(t),
                Part::Token(_) => None,
            })
            .collect();
        let mut longest: Option<(&str, &str)> = None;
        for (id, name) in &self.names {
            let Some(form) = literal_form(name) else {
                continue;
            };
            if longest.is_none_or(|(_, l)| form.len() > l.len())
                && plain.iter().any(|t| contains_word(t, form))
            {
                longest = Some((id.as_str(), form));
            }
        }
        longest
    }
}

/// What of display name `name` the literal-name check looks for: the name
/// without a leading `the `, `a ` or `an ` (so `the Old Mill` is also
/// found as `Old Mill`), or `None` for a name with no capital letter
/// (`breath`, `a vow`: ordinary words the check would trip on).
pub fn literal_form(name: &str) -> Option<&str> {
    if !name.bytes().any(|b| b.is_ascii_uppercase()) {
        return None;
    }
    let form = ["the ", "a ", "an "]
        .iter()
        .find_map(|article| name.strip_prefix(article))
        .unwrap_or(name);
    Some(form)
}

/// Whether `text` contains `word` as whole words, case-sensitively: not
/// preceded or followed by a letter or digit.
fn contains_word(text: &str, word: &str) -> bool {
    let is_word = |c: char| c.is_ascii_alphanumeric();
    text.match_indices(word).any(|(i, _)| {
        let before = text[..i].chars().next_back();
        let after = text[i + word.len()..].chars().next();
        !before.is_some_and(is_word) && !after.is_some_and(is_word)
    })
}

/// Whether `c` is a letter of the Latin script: ASCII, Latin-1 or Latin
/// Extended-A/B.
fn is_latin_letter(c: char) -> bool {
    c.is_ascii_alphabetic() || (('\u{c0}'..='\u{24f}').contains(&c) && c.is_alphabetic())
}

/// `name` with its first letter capitalised, if it starts with a Latin
/// letter; as it is otherwise (a name in a script without capitals).
fn capitalise(name: &str) -> String {
    let mut chars = name.chars();
    match chars.next() {
        Some(first) if is_latin_letter(first) => first.to_uppercase().chain(chars).collect(),
        _ => name.to_owned(),
    }
}

/// `names` (the table of file `story`) with the names of `source` added
/// (a second file, the test files' names, say). `source` is checked like a names
/// file of its own, attributing errors to `file`; an id or a name that
/// `names` has too is an error.
pub fn with_part(
    mut names: Names,
    story: &str,
    file: &str,
    source: &str,
) -> Result<Names, Vec<ContentError>> {
    let part = from_source(file, source)?;
    let mut errors = Vec::new();
    for (id, name) in part.names {
        let mut err = |message: String| {
            let e = ContentError::new(file, message);
            errors.push(match line_of(source, &format!("\"{id}\":")) {
                Some(l) => e.at(l, None),
                None => e,
            });
        };
        if names.names.contains_key(&id) {
            err(format!("name id \"{id}\" is already in {story}"));
        } else if let Some((other, _)) = names.names.iter().find(|(_, n)| **n == name) {
            err(format!(
                "\"{name}\" is the name of \"{other}\" in {story} too; names must be unique"
            ));
        } else {
            names.names.insert(id, name);
        }
    }
    if errors.is_empty() {
        Ok(names)
    } else {
        Err(errors)
    }
}

/// Parses and validates names `source`, attributing errors to `file`:
/// valid ids, each once; values non-empty plain ASCII without braces, each
/// used once. Reports every problem found.
pub fn from_source(file: &str, source: &str) -> Result<Names, Vec<ContentError>> {
    let names: BTreeMap<String, String> = parse_ron(file, source).map_err(|e| vec![e])?;
    let mut errors = Vec::new();
    let mut err = |id: &str, message: String| {
        let e = ContentError::new(file, message);
        errors.push(match line_of(source, &format!("\"{id}\":")) {
            Some(l) => e.at(l, None),
            None => e,
        });
    };
    let mut first_with: BTreeMap<&str, &str> = BTreeMap::new();
    for (id, name) in &names {
        if !is_name_id(id) {
            err(
                id,
                format!("\"{id}\" is not a valid name id; use lowercase letters, digits, _ and ."),
            );
        }
        let key = format!("\"{id}\":");
        if source
            .lines()
            .filter(|l| l.trim_start().starts_with(&key))
            .count()
            > 1
        {
            err(id, format!("name id \"{id}\" is listed more than once"));
        }
        if name.trim().is_empty() {
            err(id, format!("name \"{id}\" is empty"));
        }
        if name.contains(['{', '}']) {
            err(
                id,
                format!("name \"{id}\" contains a brace; names can't hold tokens"),
            );
        }
        if let Some(problem) = name.chars().find_map(char_problem) {
            err(id, format!("name \"{id}\": {problem}"));
        }
        if let Some(other) = first_with.insert(name, id) {
            err(
                id,
                format!(
                    "\"{name}\" is the name of both \"{other}\" and \"{id}\"; names must be unique"
                ),
            );
        }
    }
    if errors.is_empty() {
        Ok(Names { names })
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests;
