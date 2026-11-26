//! Rust (and RON) source split into lines of code, comments and strings,
//! and the `#[cfg(test)]` items found by counting brackets on them: the
//! scanner `check-keys` and `check-text` read source with, and that
//! `cargo xtask template-lines` counts a game's lines with.
//!
//! It is a line-based scanner, not a parser: good enough to tell a `//`,
//! a `/*` or a brace inside a string or a char from one in code.

/// Whether `c` can be part of an identifier.
pub fn is_ident(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// For each line, whether it is skipped: inside a `#[cfg(test)]` item, or
/// covered by a comment holding `marker` (if one is given).
pub fn exempt_lines(lines: &[Line], marker: Option<&str>) -> Vec<bool> {
    let mut exempt = vec![false; lines.len()];
    for (i, line) in lines.iter().enumerate() {
        let allowed = marker.is_some_and(|m| line.comment.contains(m));
        if allowed || line.code.contains("#[cfg(test)]") {
            mark_item(lines, i, &mut exempt);
        }
    }
    exempt
}

/// Marks the item starting at line `start` through the line where its
/// brackets balance and it ends with `;`, `}` or `,`. A line that is only a
/// comment or `#[…]` attributes never ends an item, so an attribute or a
/// marker comment on its own line covers the item below it, and a marker
/// after code on a line covers that line's item.
pub fn mark_item(lines: &[Line], start: usize, exempt: &mut [bool]) {
    let mut depth: i64 = 0;
    for (i, line) in lines.iter().enumerate().skip(start) {
        if let Some(e) = exempt.get_mut(i) {
            *e = true;
        }
        let code = line.code.trim();
        for c in code.chars() {
            match c {
                '(' | '[' | '{' => depth += 1,
                ')' | ']' | '}' => depth -= 1,
                _ => {}
            }
        }
        if depth <= 0 && code.ends_with([';', '}', ',']) {
            return;
        }
    }
}

/// One source line split into its parts.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
// A view the lexer builds; it may learn more about a line (ADR-0039).
#[non_exhaustive]
pub struct Line {
    /// The code, with comments removed and string/char literal contents
    /// blanked (the quotes stay).
    pub code: String,
    /// The text of every comment on the line.
    pub comment: String,
    /// Contents of the string literals that start on this line.
    pub strings: Vec<String>,
    /// Whether the line begins inside a string literal that started on a
    /// line above, so its text is the string's, not code.
    pub in_string: bool,
}

/// Lexer state between characters.
#[derive(Clone, Copy)]
enum State {
    Code,
    LineComment,
    /// Nesting depth.
    BlockComment(u32),
    /// A `"…"` string, with escapes.
    Str,
    /// An `r#"…"#` string with this many `#`s.
    RawStr(usize),
}

/// Splits Rust (or RON) source into [`Line`]s. Good enough for this scan:
/// handles `//` and nested `/* */` comments, `"…"` strings with escapes,
/// raw strings, and tells char literals from lifetimes.
pub fn lex(source: &str) -> Vec<Line> {
    let mut lexer = Lexer {
        chars: source.chars().collect(),
        lines: vec![Line::default()],
        state: State::Code,
        literal: String::new(),
        literal_line: 0,
        skip_ws: false,
    };
    let mut i = 0;
    // Every step uses at least one char, so this bound is never reached; it
    // only stops a broken step from looping forever.
    for _ in 0..=lexer.chars.len() {
        let Some(c) = lexer.at(i) else { break };
        i += if c == '\n' {
            lexer.newline();
            1
        } else {
            match lexer.state {
                State::Code => lexer.code(i, c),
                State::LineComment => {
                    lexer.line().comment.push(c);
                    1
                }
                State::BlockComment(depth) => lexer.block_comment(i, c, depth),
                State::Str => lexer.string(i, c),
                State::RawStr(hashes) => lexer.raw_string(i, c, hashes),
            }
        };
    }
    lexer.lines
}

/// [`lex`]'s working state.
struct Lexer {
    chars: Vec<char>,
    /// Lines so far; the last is the current one.
    lines: Vec<Line>,
    state: State,
    /// The literal being read, and the line index it started on.
    literal: String,
    literal_line: usize,
    /// After a `\` line continuation in a string: skip leading whitespace.
    skip_ws: bool,
}

impl Lexer {
    fn at(&self, i: usize) -> Option<char> {
        self.chars.get(i).copied()
    }

    /// The current line (`lines` is never empty).
    fn line(&mut self) -> &mut Line {
        let last = self.lines.len() - 1;
        &mut self.lines[last]
    }

    fn newline(&mut self) {
        match self.state {
            State::LineComment => self.state = State::Code,
            State::Str | State::RawStr(_) if !self.skip_ws => self.literal.push('\n'),
            _ => {}
        }
        self.lines.push(Line {
            in_string: matches!(self.state, State::Str | State::RawStr(_)),
            ..Line::default()
        });
    }

    /// Starts a string literal in `state`.
    fn open(&mut self, state: State) {
        self.line().code.push('"');
        self.state = state;
        self.literal.clear();
        self.literal_line = self.lines.len() - 1;
    }

    /// Ends the string literal, filing it under the line it started on.
    fn close(&mut self) {
        self.line().code.push('"');
        self.state = State::Code;
        let text = std::mem::take(&mut self.literal);
        if let Some(line) = self.lines.get_mut(self.literal_line) {
            line.strings.push(text);
        }
    }

    /// Handles `c` at `i` in code; returns how many chars it used.
    fn code(&mut self, i: usize, c: char) -> usize {
        let next = self.at(i + 1);
        if c == '/' && next == Some('/') {
            self.state = State::LineComment;
            return 2;
        }
        if c == '/' && next == Some('*') {
            self.state = State::BlockComment(1);
            return 2;
        }
        if c == '"' {
            self.open(State::Str);
            return 1;
        }
        // r"…", r#"…"#, br"…" (the `b` is plain code before it).
        let before = |k: usize| i.checked_sub(k).and_then(|j| self.at(j));
        let raw_start = !before(1).is_some_and(is_ident)
            || before(1) == Some('b') && !before(2).is_some_and(is_ident);
        if c == 'r' && raw_start {
            let hashes = self
                .chars
                .get(i + 1..)
                .map_or(0, |rest| rest.iter().take_while(|&&h| h == '#').count());
            if self.at(i + 1 + hashes) == Some('"') {
                self.open(State::RawStr(hashes));
                return 2 + hashes;
            }
        }
        if c == '\'' {
            // '\n', '\'', '\u{…}' or 'x': a char literal. Otherwise a
            // lifetime or label.
            let end = if next == Some('\\') {
                (i + 3..self.chars.len()).find(|&j| self.at(j) == Some('\''))
            } else if self.at(i + 2) == Some('\'') {
                Some(i + 2)
            } else {
                None
            };
            if let Some(end) = end {
                self.line().code.push_str("''");
                return end + 1 - i;
            }
        }
        self.line().code.push(c);
        1
    }

    /// Handles `c` at `i` inside a block comment `depth` deep.
    fn block_comment(&mut self, i: usize, c: char, depth: u32) -> usize {
        let next = self.at(i + 1);
        if c == '*' && next == Some('/') {
            self.state = if depth == 1 {
                State::Code
            } else {
                State::BlockComment(depth - 1)
            };
            2
        } else if c == '/' && next == Some('*') {
            self.state = State::BlockComment(depth + 1);
            2
        } else {
            self.line().comment.push(c);
            1
        }
    }

    /// Handles `c` at `i` inside a `"…"` string.
    fn string(&mut self, i: usize, c: char) -> usize {
        if self.skip_ws && c.is_whitespace() {
            return 1;
        }
        self.skip_ws = false;
        match c {
            // Keep the escaped character; a `\` line continuation swallows
            // the newline and the next line's leading spaces, as in Rust.
            '\\' => match self.at(i + 1) {
                Some('\n') => {
                    self.skip_ws = true;
                    1
                }
                Some(e) => {
                    self.literal.push(match e {
                        'n' => '\n',
                        't' => '\t',
                        other => other,
                    });
                    2
                }
                None => 1,
            },
            '"' => {
                self.close();
                1
            }
            _ => {
                self.literal.push(c);
                1
            }
        }
    }

    /// Handles `c` at `i` inside a raw string closed by `"` and `hashes` `#`s.
    fn raw_string(&mut self, i: usize, c: char, hashes: usize) -> usize {
        if c == '"' && (1..=hashes).all(|k| self.at(i + k) == Some('#')) {
            self.close();
            1 + hashes
        } else {
            self.literal.push(c);
            1
        }
    }
}
