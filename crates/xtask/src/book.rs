//! `cargo xtask check-book`: the marks saying who a passage of Learn is for
//! (ticket 2825, ADR-0090 §4 and §5), checked in every Markdown file under
//! `site/content/learn/`:
//!
//! - a passage is marked `{% newcomer(to="rust", topic="`match`") %}` …
//!   `{% end %}`, each on a line of its own at the line's start, with a
//!   blank line before and after;
//! - `to` is one of [`AUDIENCES`], `topic` is a few words, and nothing
//!   else is given;
//! - marks don't nest, and each is closed;
//! - a page marked whole (`[extra] newcomer`) names one of the audiences;
//! - no callout is left in the old hand-made form (`> **Rust:**`).
//!
//! [`parts`] splits a page at its marks for `llms.rs`, which writes each
//! passage in the twins as a blockquote under its label.

use std::fs;
use std::path::{Path, PathBuf};

use crate::llms;

#[cfg(test)]
mod tests;

/// Learn's pages, from the repository's root.
const LEARN: &str = "site/content/learn";

/// The four audiences a passage or a page may be marked for, and each one's
/// label (ADR-0090 §4). An unmarked passage is for everyone.
pub const AUDIENCES: [(&str, &str); 4] = [
    ("programming", "New to programming?"),
    ("games", "New to making games?"),
    ("rust", "New to Rust?"),
    ("shuyi", "New to Shuyi?"),
];

/// The label of the audience `to` ("New to Rust?"); none for an id that
/// isn't one.
pub fn label(to: &str) -> Option<&'static str> {
    AUDIENCES
        .iter()
        .find(|(id, _)| *id == to)
        .map(|(_, label)| *label)
}

/// The callouts written by hand before the marks, which the check fails:
/// a blockquote starting with one of these in bold.
const OLD_CALLOUTS: [&str; 3] = ["**Rust:**", "**Shuyi:**", "**Game idea:**"];

/// A passage's mark: who it is for and what it explains.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mark {
    /// One of [`AUDIENCES`]' ids.
    pub to: String,
    /// What the passage explains, in a few words of inline Markdown.
    pub topic: String,
}

/// A run of a page's body: plain text, or the text of a marked passage
/// without its two shortcode lines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Part<'a> {
    pub mark: Option<Mark>,
    pub text: &'a str,
    /// The number of its first line in the file.
    pub line: usize,
}

/// Every Learn page's marks, and its old callouts.
pub fn run(root: &Path) -> Vec<String> {
    let mut files = Vec::new();
    if let Err(e) = markdown_under(&root.join(LEARN), &mut files) {
        return vec![format!("{LEARN}: {e}")];
    }
    files.sort();
    let mut errors = Vec::new();
    for file in files {
        let shown = file
            .strip_prefix(root)
            .unwrap_or(&file)
            .to_string_lossy()
            .replace('\\', "/");
        match fs::read_to_string(&file) {
            Ok(text) => errors.extend(check(&shown, &text)),
            Err(e) => errors.push(format!("{shown}: {e}")),
        }
    }
    errors
}

/// Every `.md` file under `dir`.
fn markdown_under(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            markdown_under(&path, out)?;
        } else if path.extension().is_some_and(|ext| ext == "md") {
            out.push(path);
        }
    }
    Ok(())
}

/// What is wrong with the marks of the page `shown`, whose whole text is
/// `text`.
pub fn check(shown: &str, text: &str) -> Vec<String> {
    let (body, first_line, mut errors) = match front_matter(shown, text) {
        Ok(found) => found,
        Err(e) => return vec![e],
    };
    errors.extend(parts(shown, body, first_line).1);
    for (at, line) in body.lines().enumerate() {
        let quoted = line.trim_start().strip_prefix('>').map(str::trim_start);
        if let Some(old) = quoted.and_then(|q| OLD_CALLOUTS.iter().find(|old| q.starts_with(**old)))
        {
            errors.push(format!(
                "{shown}:{}: a callout of the old form, `> {old}`: mark the passage with \
                 {{% newcomer(to=\"…\", topic=\"…\") %}} … {{% end %}} (site/README.md, *Marking a passage*)",
                first_line + at
            ));
        }
    }
    errors
}

/// The body of the page `shown` after its `+++` front matter, the number of
/// its first line, and what is wrong with a page-level mark
/// (`newcomer = "…"`).
fn front_matter<'a>(shown: &str, text: &'a str) -> Result<(&'a str, usize, Vec<String>), String> {
    let mut lines = text.split_inclusive('\n');
    if lines.next().map(str::trim_end) != Some("+++") {
        return Err(format!("{shown}:1: no `+++` front matter"));
    }
    let mut errors = Vec::new();
    let mut at = text.find('\n').map_or(text.len(), |end| end + 1);
    for (number, line) in lines.enumerate() {
        let number = number + 2;
        at += line.len();
        let bare = line.trim();
        if bare == "+++" {
            return Ok((&text[at..], number + 1, errors));
        }
        if let Some(value) = bare
            .strip_prefix("newcomer")
            .map(str::trim_start)
            .and_then(|rest| rest.strip_prefix('='))
        {
            let to = value.trim().trim_matches('"');
            if label(to).is_none() {
                errors.push(format!(
                    "{shown}:{number}: newcomer = \"{to}\" isn't an audience: {}",
                    ids()
                ));
            }
        }
    }
    Err(format!("{shown}: its front matter has no closing `+++`"))
}

/// The audiences' ids, for an error.
fn ids() -> String {
    let ids: Vec<String> = AUDIENCES.iter().map(|(id, _)| format!("`{id}`")).collect();
    ids.join(", ")
}

/// What a line of a body is to the marks.
enum Line {
    Open(Result<Mark, String>),
    End,
    Other,
}

/// `body`, the text of `shown` from its line `first_line`, split at its
/// marks, and what is wrong with them. A mark that doesn't close leaves the
/// rest of the body plain.
pub fn parts<'a>(shown: &str, body: &'a str, first_line: usize) -> (Vec<Part<'a>>, Vec<String>) {
    let mut out = Vec::new();
    let mut errors = Vec::new();
    // The plain run's start, as a byte and a line.
    let (mut from, mut from_line) = (0, first_line);
    // The open mark, its text's first byte, and the opener's line.
    let mut open: Option<(Mark, usize, usize)> = None;
    let mut previous_blank = true;
    let mut next_must_be_blank: Option<usize> = None;
    let mut at = 0;
    for (index, raw) in body.split_inclusive('\n').enumerate() {
        let number = first_line + index;
        let line = raw.trim_end_matches(['\n', '\r']);
        let blank = line.trim().is_empty();
        if let Some(closer) = next_must_be_blank.take()
            && !blank
        {
            errors.push(format!(
                "{shown}:{closer}: a mark's line has no blank line after it"
            ));
        }
        match kind(line) {
            Line::Open(mark) => {
                if !previous_blank {
                    errors.push(format!(
                        "{shown}:{number}: a mark has no blank line before it"
                    ));
                }
                match (mark, &open) {
                    (_, Some((_, _, opened))) => errors.push(format!(
                        "{shown}:{number}: a mark inside the mark of line {opened}: marks don't nest"
                    )),
                    (Err(e), None) => errors.push(format!("{shown}:{number}: {e}")),
                    (Ok(mark), None) => {
                        out.push(Part {
                            mark: None,
                            text: &body[from..at],
                            line: from_line,
                        });
                        open = Some((mark, at + raw.len(), number));
                    }
                }
            }
            Line::End => {
                match open.take() {
                    Some((mark, start, opened)) => {
                        out.push(Part {
                            mark: Some(mark),
                            text: &body[start..at],
                            line: opened + 1,
                        });
                        from = at + raw.len();
                        from_line = number + 1;
                    }
                    None => errors.push(format!(
                        "{shown}:{number}: a `{{% end %}}` with no mark open"
                    )),
                }
                next_must_be_blank = Some(number);
            }
            Line::Other => {
                if let Some(e) = misplaced(line) {
                    errors.push(format!("{shown}:{number}: {e}"));
                }
            }
        }
        previous_blank = blank;
        at += raw.len();
    }
    if let Some((_, _, opened)) = open {
        errors.push(format!("{shown}:{opened}: a mark with no `{{% end %}}`"));
    }
    out.push(Part {
        mark: None,
        text: &body[from..],
        line: from_line,
    });
    out.retain(|part| part.mark.is_some() || !part.text.is_empty());
    (out, errors)
}

/// The shortcode after the `{%` at the start of `line`: its name and what
/// follows it, or none for any other line (an escaped `{%/*` too).
fn shortcode(line: &str) -> Option<(&str, &str)> {
    let after = line.strip_prefix("{%")?;
    if after.starts_with("/*") {
        return None;
    }
    let trimmed = after.trim_start();
    let name_len = trimmed
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .unwrap_or(trimmed.len());
    Some((&trimmed[..name_len], after))
}

/// What `line` is: a mark's opener, its closer, or another line.
fn kind(line: &str) -> Line {
    match shortcode(line) {
        Some(("newcomer", after)) => Line::Open(opener(after)),
        Some(("end", after)) if after.trim_start()["end".len()..].trim() == "%}" => Line::End,
        _ => Line::Other,
    }
}

/// The mark of an opener, `after` its `{%`.
fn opener(after: &str) -> Result<Mark, String> {
    let call = match llms::call(after, "%}") {
        Some(Ok(call)) => call,
        Some(Err(e)) => return Err(e),
        None => return Err("a `newcomer` mark is `{% newcomer(to=\"…\", topic=\"…\") %}`".into()),
    };
    if !after[call.len..].trim().is_empty() {
        return Err("a mark stands on a line of its own".into());
    }
    let (mut to, mut topic) = (None, None);
    for (key, value) in llms::arguments(call.args)? {
        match key {
            "to" => to = Some(value),
            "topic" => topic = Some(value),
            _ => {
                return Err(format!(
                    "a mark takes no argument `{key}` (it takes `to` and `topic`)"
                ));
            }
        }
    }
    let to = to.ok_or_else(|| format!("a mark with no `to`: {}", ids()))?;
    if label(to).is_none() {
        return Err(format!("to=\"{to}\" isn't an audience: {}", ids()));
    }
    let topic = topic
        .map(str::trim)
        .filter(|topic| !topic.is_empty())
        .ok_or("a mark with no `topic`: a few words saying what the passage explains")?;
    Ok(Mark {
        to: to.to_owned(),
        topic: topic.to_owned(),
    })
}

/// What is wrong with a line holding a mark's opener or closer anywhere but
/// at its start.
fn misplaced(line: &str) -> Option<&'static str> {
    let mut rest = line;
    let mut first = true;
    while let Some(found) = rest.find("{%") {
        let here = &rest[found..];
        if let Some(name @ ("newcomer" | "end")) = shortcode(here).map(|(name, _)| name) {
            return Some(if first && found > 0 && rest[..found].trim().is_empty() {
                "a mark's line starts at the line's start, with no indent"
            } else if name == "end" {
                "a `{% end %}` stands on a line of its own"
            } else {
                "a mark stands on a line of its own"
            });
        }
        first = false;
        rest = &here[2..];
    }
    None
}
