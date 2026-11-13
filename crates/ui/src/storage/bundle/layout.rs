//! The text of a bundle's RON entries: [`compact`] gives an entry back as
//! a game wrote it with `ron::to_string`, and [`layout`] writes it over
//! lines for a player to read. The only code of the bundle that walks RON
//! text, so it knows every token whose inside isn't RON: strings (with
//! their escapes), raw strings (`r#"…"#`), chars, and line and block
//! comments (nested, as RON nests them).

use crate::prelude::*;

/// Four spaces to a level.
const INDENT: &str = "    ";

/// One piece of RON text, outside whitespace and comments.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Token<'a> {
    /// A space between two [`Token::Atom`]s, which whitespace or a comment
    /// parted: kept, as dropping it would join them.
    Space,
    /// One of `( ) [ ] { } , :`.
    Punct(char),
    /// Anything else, as it is: a string, a char, a number, a name.
    Atom(&'a str),
}

/// RON's whitespace (`ron::parse::is_whitespace_char`).
fn is_space(c: char) -> bool {
    matches!(
        c,
        ' ' | '\t'
            | '\n'
            | '\r'
            | '\x0B'
            | '\x0C'
            | '\u{85}'
            | '\u{200E}'
            | '\u{200F}'
            | '\u{2028}'
            | '\u{2029}'
    )
}

fn is_punct(c: char) -> bool {
    matches!(c, '(' | ')' | '[' | ']' | '{' | '}' | ',' | ':')
}

/// The closing bracket of `open`, if it opens one.
fn closing(open: char) -> Option<char> {
    match open {
        '(' => Some(')'),
        '[' => Some(']'),
        '{' => Some('}'),
        _ => None,
    }
}

/// The length of the whitespace or comment `rest` starts with, if it does.
fn gap_len(rest: &str) -> Option<usize> {
    if rest.starts_with("//") {
        return Some(rest.find('\n').unwrap_or(rest.len()));
    }
    if rest.starts_with("/*") {
        return Some(block_comment_len(rest));
    }
    let len: usize = rest
        .chars()
        .take_while(|c| is_space(*c))
        .map(char::len_utf8)
        .sum();
    (len > 0).then_some(len)
}

/// The length of the block comment `rest` starts with, those inside it
/// too; all of `rest` if it isn't closed.
fn block_comment_len(rest: &str) -> usize {
    let mut depth = 0_usize;
    let mut chars = rest.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        match (c, chars.peek().map(|(_, next)| *next)) {
            ('/', Some('*')) => {
                chars.next();
                depth += 1;
            }
            ('*', Some('/')) => {
                chars.next();
                depth -= 1;
                if depth == 0 {
                    return i + 2;
                }
            }
            _ => {}
        }
    }
    rest.len()
}

/// The length of the atom `rest` starts with: a string, raw string or
/// char (each perhaps a byte one, `b"…"`), or else a word up to the next
/// whitespace, comment, bracket, `,`, `:` or quote.
fn atom_len(rest: &str) -> usize {
    let prefix = usize::from(rest.starts_with('b'));
    let after = &rest[prefix..];
    if let Some(len) = raw_string_len(after) {
        return prefix + len;
    }
    match after.chars().next() {
        Some(quote @ ('"' | '\'')) => prefix + quoted_len(after, quote),
        _ => word_len(rest),
    }
}

/// The length of the raw string `rest` starts with (`r"…"`, `r#"…"#`), if
/// it starts with one; all of `rest` if it isn't closed.
fn raw_string_len(rest: &str) -> Option<usize> {
    let hashes = rest
        .strip_prefix('r')?
        .bytes()
        .take_while(|b| *b == b'#')
        .count();
    let open = 1 + hashes;
    if rest.as_bytes().get(open) != Some(&b'"') {
        return None;
    }
    let body = open + 1;
    let close = format!("\"{}", "#".repeat(hashes));
    Some(
        rest[body..]
            .find(&close)
            .map_or(rest.len(), |at| body + at + close.len()),
    )
}

/// The length of the string or char `rest` starts with, quoted by
/// `quote`, its escapes skipped; all of `rest` if it isn't closed.
fn quoted_len(rest: &str, quote: char) -> usize {
    let mut chars = rest.char_indices().skip(1);
    while let Some((i, c)) = chars.next() {
        if c == '\\' {
            chars.next();
        } else if c == quote {
            return i + c.len_utf8();
        }
    }
    rest.len()
}

/// The length of the word `rest` starts with: at least its first char.
fn word_len(rest: &str) -> usize {
    rest.char_indices()
        .skip(1)
        .find(|(i, c)| is_punct(*c) || matches!(c, '"' | '\'') || gap_len(&rest[*i..]).is_some())
        .map_or(rest.len(), |(i, _)| i)
}

/// The tokens of `text`: whitespace and comments dropped, but for a
/// [`Token::Space`] between two atoms they parted.
fn tokens(text: &str) -> Vec<Token<'_>> {
    let mut tokens = Vec::new();
    let mut parted = false;
    let mut rest = text;
    while let Some(c) = rest.chars().next() {
        let len = if let Some(len) = gap_len(rest) {
            parted = true;
            len
        } else if is_punct(c) {
            tokens.push(Token::Punct(c));
            parted = false;
            1
        } else {
            let len = atom_len(rest);
            if parted && matches!(tokens.last(), Some(Token::Atom(_))) {
                tokens.push(Token::Space);
            }
            tokens.push(Token::Atom(&rest[..len]));
            parted = false;
            len
        };
        // Every piece is at least its first char: the walk always moves
        // on, whatever a length says.
        rest = &rest[len.max(c.len_utf8())..];
    }
    tokens
}

/// `text` without whitespace or comments outside its strings and chars
/// (one space kept between two words they parted): what `ron::to_string`
/// writes, so a value it wrote comes back byte for byte.
pub(super) fn compact(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for token in tokens(text) {
        match token {
            Token::Space => out.push(' '),
            Token::Punct(c) => out.push(c),
            Token::Atom(atom) => out.push_str(atom),
        }
    }
    out
}

/// `text` laid out over lines, as `level` levels in: a line opened after
/// `(`, `[`, `{` and each `,`, one closed before `)`, `]` and `}`, four
/// spaces to a level, and `: ` after a field name or a map's key. A
/// bracket with nothing inside stays on one line (`()`). Laying out text
/// already laid out changes nothing, and [`compact`] undoes it.
pub(super) fn layout(text: &str, level: usize) -> String {
    let mut tokens = tokens(text).into_iter().peekable();
    let mut out = String::with_capacity(text.len() * 2);
    let mut depth = level;
    // A new line is owed before the next token.
    let mut owed = false;
    while let Some(token) = tokens.next() {
        let opens = match token {
            Token::Punct(c) => closing(c),
            _ => None,
        };
        if let Token::Punct(')' | ']' | '}') = token {
            depth = depth.saturating_sub(1);
            owed = !out.is_empty();
        }
        if owed {
            out.push('\n');
            for _ in 0..depth {
                out.push_str(INDENT);
            }
            owed = false;
        }
        match token {
            Token::Space => out.push(' '),
            Token::Punct(':') => out.push_str(": "),
            Token::Punct(c) => out.push(c),
            Token::Atom(atom) => out.push_str(atom),
        }
        if let Some(close) = opens {
            if tokens.next_if_eq(&Token::Punct(close)).is_some() {
                out.push(close);
            } else {
                depth += 1;
                owed = true;
            }
        }
        if token == Token::Punct(',') {
            owed = true;
        }
    }
    out
}

#[cfg(test)]
mod tests;
