//! The book's terms and its glossary (ticket 2852): a word of Rust or of the
//! engine is bolded where a page defines it, and its row goes in the one
//! glossary, `book/glossary.md` (`site/README.md`, *The rules*).
//!
//! A bold in a page of the book is a term, unless it ends with `.`, `:`,
//! `?` or `!`: then it is a run-in heading (`**A bad key.**`). Emphasis
//! that is neither is written in italics. Each term must be a word of a
//! glossary row, case and a plural aside: the row `Update, draw` covers
//! **update**, **draw** and **draws**. The rows' links are `check-links`'s.

#[cfg(test)]
mod tests;

/// The book's pages, from Learn's folder: every Markdown page here and in
/// the project's folder but the front pages and the glossary.
pub const FOLDERS: [&str; 2] = ["book", "book/box-pusher"];
/// The glossary, from Learn's folder.
pub const GLOSSARY: &str = "book/glossary.md";

/// The characters that end a run-in heading rather than a term.
const HEADING_ENDS: [char; 4] = ['.', ':', '?', '!'];

/// A bolded span of a page: its text and the line it starts on (from the
/// page's own first line, 1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bold {
    pub text: String,
    pub line: usize,
}

/// Whether the page `shown` (a path from the repository's root) is one of
/// the book's whose terms are checked; `run` gives only Markdown pages.
pub fn is_book_page(shown: &str) -> bool {
    let Some(rest) = shown
        .strip_prefix(super::LEARN)
        .and_then(|r| r.strip_prefix('/'))
    else {
        return false;
    };
    let Some((folder, file)) = rest.rsplit_once('/') else {
        return false;
    };
    FOLDERS.contains(&folder) && file != "_index.md" && rest != GLOSSARY
}

/// Every word the glossary's rows define, as [`key`]s: each row's first
/// cell, split at its commas.
pub fn glossary_words(glossary: &str) -> Vec<String> {
    let mut words = Vec::new();
    for line in glossary.lines() {
        let Some(row) = line.trim().strip_prefix('|') else {
            continue;
        };
        let Some(first) = row.split('|').next().map(str::trim) else {
            continue;
        };
        if first.is_empty() || first == "Word" || first.starts_with('-') {
            continue;
        }
        words.extend(first.split(", ").map(key));
    }
    words
}

/// A word as the check compares it: without backticks, lower case, its
/// spaces single, and singular (`Layers` and `layer` are one).
pub fn key(word: &str) -> String {
    let plain: String = word.chars().filter(|c| *c != '`').collect();
    let plain = plain.split_whitespace().collect::<Vec<_>>().join(" ");
    singular(&plain.to_lowercase())
}

/// `word` without the plural ending of its last word: `-ies` to `-y`,
/// `-es` after `s`, `x`, `ch` or `sh`, and a lone `-s`.
fn singular(word: &str) -> String {
    if word.len() > 4
        && let Some(stem) = word.strip_suffix("ies")
    {
        return format!("{stem}y");
    }
    if let Some(stem) = word.strip_suffix("es")
        && ["s", "x", "ch", "sh"].iter().any(|end| stem.ends_with(end))
    {
        return stem.to_string();
    }
    match word.strip_suffix('s') {
        Some(stem) if word.len() > 3 && !stem.ends_with('s') => stem.to_string(),
        _ => word.to_string(),
    }
}

/// Whether the bold `text` is a run-in heading, not a term.
pub fn is_heading(text: &str) -> bool {
    text.trim_end().ends_with(HEADING_ENDS)
}

/// The bolds of `body`, whose first line is `first_line`: outside fenced
/// blocks and code spans, each from its `**` to the next, within a
/// paragraph. A bold left open at a paragraph's end is dropped.
pub fn bolds(body: &str, first_line: usize) -> Vec<Bold> {
    let mut found = Vec::new();
    let mut fence: Option<&str> = None;
    let mut open: Option<Bold> = None;
    for (at, line) in body.lines().enumerate() {
        let number = first_line + at;
        let trimmed = line.trim_start();
        if let Some(mark) = fence {
            if trimmed.starts_with(mark) {
                fence = None;
            }
            continue;
        }
        if let Some(mark) = ["```", "~~~"].into_iter().find(|m| trimmed.starts_with(m)) {
            fence = Some(mark);
            open = None;
            continue;
        }
        if trimmed.is_empty() {
            open = None;
            continue;
        }
        if let Some(bold) = open.as_mut() {
            bold.text.push(' ');
        }
        scan_line(line, number, &mut open, &mut found);
    }
    found
}

/// One line of a paragraph: its code spans kept whole, and each `**`
/// opening or closing a bold.
fn scan_line(line: &str, number: usize, open: &mut Option<Bold>, found: &mut Vec<Bold>) {
    let mut rest = line;
    while !rest.is_empty() {
        if rest.starts_with('`') {
            let ticks = rest.len() - rest.trim_start_matches('`').len();
            let fence = &rest[..ticks];
            let after = &rest[ticks..];
            let span = match after.find(fence) {
                Some(end) => ticks + end + ticks,
                None => rest.len(),
            };
            if let Some(bold) = open.as_mut() {
                bold.text.push_str(&rest[..span]);
            }
            rest = &rest[span..];
        } else if let Some(after) = rest.strip_prefix("**") {
            match open.take() {
                Some(bold) => found.push(bold),
                None => {
                    *open = Some(Bold {
                        text: String::new(),
                        line: number,
                    });
                }
            }
            rest = after;
        } else {
            let next = rest.chars().next().map_or(1, char::len_utf8);
            if let Some(bold) = open.as_mut() {
                bold.text.push_str(&rest[..next]);
            }
            rest = &rest[next..];
        }
    }
}

/// What is wrong with the terms of the page `shown`, whose body is `body`
/// from line `first_line`: each term no word of the glossary covers.
pub fn check(shown: &str, body: &str, first_line: usize, words: &[String]) -> Vec<String> {
    let mut errors = Vec::new();
    for bold in bolds(body, first_line) {
        if is_heading(&bold.text) || words.contains(&key(&bold.text)) {
            continue;
        }
        errors.push(format!(
            "{shown}:{}: the term **{}** has no row in the glossary (`site/content/learn/{GLOSSARY}`): \
             add its row, or write it in italics if it is only emphasis (site/README.md, *The rules*)",
            bold.line, bold.text
        ));
    }
    errors
}
