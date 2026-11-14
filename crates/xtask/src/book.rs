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
//! And the quick checks (ticket 2827, ADR-0090 §8), `{% check(to="rust") %}`
//! … `{% end %}`, placed as a mark is and never inside one:
//!
//! - `to` is optional, and one of [`AUDIENCES`];
//! - the question comes first, then two to four answers, one line each,
//!   `- [x] ` for a right one and `- [ ] ` for a wrong one, each with
//!   ` — ` and why after it, and at least one right;
//! - every concept chapter (`[extra] chapter`) and project step (`[extra]
//!   step`) ends with a `## Check yourself` section holding two or more
//!   (no most: Nick, 2026-11-13), and no other page has one;
//! - across the book, no one place (first, second…) holds the right answer
//!   of more than half of the checks with one right answer (ticket 2833):
//!   the page shuffles its buttons, but the `<details>` list and the twins
//!   show the written order.
//!
//! [`parts`] splits a page at its marks and checks for `llms.rs`, which
//! writes each passage in the twins as a blockquote under its label, and
//! each check as text.

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

/// The topic of a check's mark, as the shortcode's label says it.
pub const CHECK_TOPIC: &str = "a quick check";

/// The heading of the section a chapter or a step ends with, holding its
/// checks.
pub const CHECK_SECTION: &str = "## Check yourself";

/// A quick check: its question and its answers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Check {
    /// The text before the answers, Markdown.
    pub question: String,
    pub answers: Vec<Answer>,
}

/// One answer of a check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Answer {
    pub right: bool,
    /// The answer, inline Markdown.
    pub text: String,
    /// Why it is right or wrong, inline Markdown.
    pub why: String,
}

/// A run of a page's body: plain text, or the text of a marked passage or
/// a check without its two shortcode lines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Part<'a> {
    /// A passage's mark, or a check's audience under [`CHECK_TOPIC`].
    pub mark: Option<Mark>,
    /// The check this part is, read from `text`; none for a passage, plain
    /// text, or a check that doesn't read.
    pub check: Option<Check>,
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
    let mut pages = Vec::new();
    for file in files {
        let shown = file
            .strip_prefix(root)
            .unwrap_or(&file)
            .to_string_lossy()
            .replace('\\', "/");
        match fs::read_to_string(&file) {
            Ok(text) => pages.push((shown, text)),
            Err(e) => errors.push(format!("{shown}: {e}")),
        }
    }
    errors.extend(check_pages(&pages));
    errors
}

/// What is wrong with `pages`, each its shown path and its whole text: each
/// page's own problems, then the spread of the right answers across them.
pub fn check_pages(pages: &[(String, String)]) -> Vec<String> {
    let mut errors = Vec::new();
    let mut places = Vec::new();
    for (shown, text) in pages {
        errors.extend(check(shown, text));
        places.extend(right_places(shown, text));
    }
    errors.extend(spread(&places));
    errors
}

/// The place (0-based) of the right answer in each check of the page that
/// has exactly one; a check that doesn't read counts for nothing.
fn right_places(shown: &str, text: &str) -> Vec<usize> {
    let Ok((body, first_line, _, _)) = front_matter(shown, text) else {
        return Vec::new();
    };
    parts(shown, body, first_line)
        .0
        .iter()
        .filter_map(|part| part.check.as_ref())
        .filter_map(|check| {
            let mut rights = check.answers.iter().enumerate().filter(|(_, a)| a.right);
            match (rights.next(), rights.next()) {
                (Some((place, _)), None) => Some(place),
                _ => None,
            }
        })
        .collect()
}

/// The places' names in an error, first to fourth: a check has two to four
/// answers.
const PLACES: [&str; 4] = ["first", "second", "third", "fourth"];

/// What is wrong with where the right answers are (ticket 2833): no one
/// place holds more than half of the checks with one right answer, so a
/// reader with no JavaScript, or of the twins, can't learn to pick a place
/// without reading.
fn spread(places: &[usize]) -> Option<String> {
    let mut counts = [0usize; PLACES.len()];
    for place in places {
        if let Some(count) = counts.get_mut(*place) {
            *count += 1;
        }
    }
    let total = places.len();
    let (most, &held) = counts.iter().enumerate().max_by_key(|(_, count)| **count)?;
    if held * 2 <= total {
        return None;
    }
    let others: Vec<String> = counts
        .iter()
        .enumerate()
        .filter(|(place, count)| *place != most && **count > 0)
        .map(|(place, count)| format!(", {} in {count}", PLACES[place]))
        .collect();
    let others = others.concat();
    Some(format!(
        "{LEARN}: the right answer is {} in {held} of {total} quick checks{others}: spread them (2833)",
        PLACES[most]
    ))
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
    let (body, first_line, mut errors, kind) = match front_matter(shown, text) {
        Ok(found) => found,
        Err(e) => return vec![e],
    };
    errors.extend(parts(shown, body, first_line).1);
    errors.extend(check_section(shown, body, first_line, kind));
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

/// What a page is to the checks: a concept chapter or a project step,
/// which ends with its checks, or another page, which has none.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PageKind {
    Chapter,
    Step,
    Other,
}

/// What [`front_matter`] finds: the body, its first line's number, what is
/// wrong with a page-level mark, and what the page is.
type FrontMatter<'a> = (&'a str, usize, Vec<String>, PageKind);

/// The body of the page `shown` after its `+++` front matter, the number of
/// its first line, what is wrong with a page-level mark
/// (`newcomer = "…"`), and what the page is (`chapter = …`, `step = …`).
fn front_matter<'a>(shown: &str, text: &'a str) -> Result<FrontMatter<'a>, String> {
    let mut lines = text.split_inclusive('\n');
    if lines.next().map(str::trim_end) != Some("+++") {
        return Err(format!("{shown}:1: no `+++` front matter"));
    }
    let mut errors = Vec::new();
    let mut kind = PageKind::Other;
    let mut at = text.find('\n').map_or(text.len(), |end| end + 1);
    for (number, line) in lines.enumerate() {
        let number = number + 2;
        at += line.len();
        let bare = line.trim();
        if bare == "+++" {
            return Ok((&text[at..], number + 1, errors, kind));
        }
        match bare.split('=').next().map(str::trim) {
            Some("chapter") => kind = PageKind::Chapter,
            Some("step") => kind = PageKind::Step,
            _ => {}
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
    Open(Result<Opened, String>),
    End,
    Other,
}

/// What an opener opens: a passage's mark, or a check and its audience.
enum Opened {
    Mark(Mark),
    Check(Option<String>),
}

impl Opened {
    /// What it is called in an error.
    fn name(&self) -> &'static str {
        match self {
            Opened::Mark(_) => "mark",
            Opened::Check(_) => "check",
        }
    }
}

/// `body`, the text of `shown` from its line `first_line`, split at its
/// marks, and what is wrong with them. A mark that doesn't close leaves the
/// rest of the body plain.
pub fn parts<'a>(shown: &str, body: &'a str, first_line: usize) -> (Vec<Part<'a>>, Vec<String>) {
    let mut out = Vec::new();
    let mut errors = Vec::new();
    // The plain run's start, as a byte and a line.
    let (mut from, mut from_line) = (0, first_line);
    // The open mark or check, its text's first byte, and the opener's line.
    let mut open: Option<(Opened, usize, usize)> = None;
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
                    (_, Some((what, _, opened))) => errors.push(format!(
                        "{shown}:{number}: a mark inside the {} of line {opened}: marks don't nest",
                        what.name()
                    )),
                    (Err(e), None) => errors.push(format!("{shown}:{number}: {e}")),
                    (Ok(mark), None) => {
                        out.push(Part {
                            mark: None,
                            check: None,
                            text: &body[from..at],
                            line: from_line,
                        });
                        open = Some((mark, at + raw.len(), number));
                    }
                }
            }
            Line::End => {
                match open.take() {
                    Some((what, start, opened)) => {
                        let text = &body[start..at];
                        let (mark, check) = match what {
                            Opened::Mark(mark) => (Some(mark), None),
                            Opened::Check(to) => {
                                let check = match read_check(text, opened + 1) {
                                    Ok(check) => Some(check),
                                    Err(found) => {
                                        errors.extend(
                                            found.into_iter().map(|e| format!("{shown}:{e}")),
                                        );
                                        None
                                    }
                                };
                                let mark = to.map(|to| Mark {
                                    to,
                                    topic: CHECK_TOPIC.to_owned(),
                                });
                                (mark, check)
                            }
                        };
                        out.push(Part {
                            mark,
                            check,
                            text,
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
        check: None,
        text: &body[from..],
        line: from_line,
    });
    out.retain(|part| part.mark.is_some() || part.check.is_some() || !part.text.is_empty());
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
        Some(("newcomer", after)) => Line::Open(opener(after).map(Opened::Mark)),
        Some(("check", after)) => Line::Open(check_opener(after).map(Opened::Check)),
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
        if let Some(name @ ("newcomer" | "check" | "end")) = shortcode(here).map(|(name, _)| name) {
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

/// A check's audience, from its opener `after` its `{%`: none when it has
/// no `to`.
fn check_opener(after: &str) -> Result<Option<String>, String> {
    let call = match llms::call(after, "%}") {
        Some(Ok(call)) => call,
        Some(Err(e)) => return Err(e),
        None => return Err("a check is `{% check() %}` or `{% check(to=\"…\") %}`".into()),
    };
    if !after[call.len..].trim().is_empty() {
        return Err("a check stands on a line of its own".into());
    }
    let mut to = None;
    for (key, value) in llms::arguments(call.args)? {
        if key == "to" {
            to = Some(value);
        } else {
            return Err(format!(
                "a check takes no argument `{key}` (it takes `to`, or nothing)"
            ));
        }
    }
    match to {
        Some(to) if label(to).is_none() => Err(format!("to=\"{to}\" isn't an audience: {}", ids())),
        to => Ok(to.map(str::to_owned)),
    }
}

/// The check written in `text`, whose first line is `first_line`, the line
/// after its opener, or what is wrong with it, each error starting with its
/// line's number. What is wrong with the whole check is on its opener.
pub fn read_check(text: &str, first_line: usize) -> Result<Check, Vec<String>> {
    let opener = first_line.saturating_sub(1);
    let mut errors = Vec::new();
    let mut question: Vec<&str> = Vec::new();
    let mut answers = Vec::new();
    // Answers that don't read, counted with the rest.
    let mut unread = 0;
    for (index, line) in text.lines().enumerate() {
        let number = first_line + index;
        let line = line.trim_end();
        let item = if let Some(rest) = line.strip_prefix("- [x] ") {
            Some((true, rest))
        } else {
            line.strip_prefix("- [ ] ").map(|rest| (false, rest))
        };
        match item {
            Some((right, rest)) => match read_answer(right, rest) {
                Ok(answer) => answers.push(answer),
                Err(e) => {
                    unread += 1;
                    errors.push(format!("{number}: {e}"));
                }
            },
            None if line.trim().is_empty() => {}
            None if line.trim_start().starts_with("- [") => {
                unread += 1;
                errors.push(format!(
                    "{number}: an answer starts `- [x] ` (right) or `- [ ] ` (wrong), at the line's start"
                ));
            }
            None if !answers.is_empty() || unread > 0 => errors.push(format!(
                "{number}: a check's answers are one line each, and nothing comes after them"
            )),
            None if line.contains("{{") || line.contains("{%") => errors.push(format!(
                "{number}: a check holds no shortcode: quote the line it asks about in `code`"
            )),
            None => question.push(line),
        }
    }
    let question = question.join("\n").trim().to_owned();
    if question.is_empty() {
        errors.push(format!(
            "{first_line}: a check with no question before its answers"
        ));
    }
    let count = answers.len() + unread;
    if !(2..=4).contains(&count) {
        errors.push(format!(
            "{opener}: a check with {count} answers: it has two to four"
        ));
    }
    if unread == 0 && !answers.iter().any(|answer: &Answer| answer.right) {
        errors.push(format!(
            "{opener}: a check with no right answer: mark one `- [x] `"
        ));
    }
    if errors.is_empty() {
        Ok(Check { question, answers })
    } else {
        Err(errors)
    }
}

/// One answer, from the text after its `- [x] ` or `- [ ] `.
fn read_answer(right: bool, rest: &str) -> Result<Answer, String> {
    let (text, why) = rest
        .split_once(" — ")
        .map(|(text, why)| (text.trim(), why.trim()))
        .filter(|(text, why)| !text.is_empty() && !why.is_empty())
        .ok_or("an answer with no reason: write the answer, ` — `, then why")?;
    if text.contains("](") {
        return Err(
            "an answer holds no link (the page makes it a button): put the link in why".into(),
        );
    }
    Ok(Answer {
        right,
        text: text.to_owned(),
        why: why.to_owned(),
    })
}

/// What is wrong with where the checks of `shown` are: a chapter or a step
/// ends with [`CHECK_SECTION`] holding two or more, and every check is in
/// it; another page has none.
fn check_section(shown: &str, body: &str, first_line: usize, kind: PageKind) -> Vec<String> {
    let mut errors = Vec::new();
    // The `## ` headings outside fenced code, and the checks' openers, by
    // line.
    let mut headings = Vec::new();
    let mut checks = Vec::new();
    let mut fenced = false;
    for (index, line) in body.lines().enumerate() {
        let number = first_line + index;
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            fenced = !fenced;
        } else if fenced {
        } else if line.starts_with("## ") {
            headings.push((number, line.trim_end()));
        } else if matches!(shortcode(line), Some(("check", _))) {
            checks.push(number);
        }
    }
    let section = headings
        .iter()
        .find(|(_, heading)| *heading == CHECK_SECTION)
        .map(|(line, _)| *line);
    if kind == PageKind::Other {
        if let Some(line) = section {
            errors.push(format!(
                "{shown}:{line}: `{CHECK_SECTION}` is for a concept chapter or a project step"
            ));
        }
        if let Some(line) = checks.first() {
            errors.push(format!(
                "{shown}:{line}: a check is for a concept chapter or a project step"
            ));
        }
        return errors;
    }
    let what = if kind == PageKind::Chapter {
        "chapter"
    } else {
        "step"
    };
    let Some(section) = section else {
        errors.push(format!(
            "{shown}: a {what} ends with `{CHECK_SECTION}`, holding two checks or more"
        ));
        return errors;
    };
    if let Some((line, _)) = headings.iter().find(|(line, _)| *line > section) {
        errors.push(format!(
            "{shown}:{line}: a heading after `{CHECK_SECTION}`: it is the {what}'s last section"
        ));
    }
    for line in checks.iter().filter(|line| **line < section) {
        errors.push(format!(
            "{shown}:{line}: a check before `{CHECK_SECTION}`: the checks go in it"
        ));
    }
    let count = checks.iter().filter(|line| **line > section).count();
    if count < 2 {
        errors.push(format!(
            "{shown}:{section}: `{CHECK_SECTION}` holds {count} checks: it holds two or more"
        ));
    }
    errors
}
