//! `check-keys`: fails when game code or player text names a key directly
//! instead of going through an `Action` and the player's keymap.
//! Controller buttons are treated like keys.
//!
//! It scans the non-test Rust in the project's source folders
//! ([`CheckKeys::rust`]) for key and button types (`Key::`, `Chord::`,
//! `Button::`) and macroquad key reads (`KeyCode`, `is_key_down`, …)
//! outside the files that make up the key pipeline, and scans string
//! literals there for key names shown to the player (`"f select"`,
//! `"press f"`, `"[F]"`, `"Space"`, `"arrows"`, …). Text in the asset
//! folders (`.ron` strings: tips, item and skill descriptions; `.dlg`
//! dialogue) is prose, so there only an instruction to press a key
//! (`press f`, `hold Shift`), a `Shift+` chord, `WASD` or `[F]` counts;
//! "Escape!" or "fires arrows" as plain words are fine.
//!
//! Two lists of keys are read. [`KEY_WORDS`] holds the bare words a help bar
//! shows alone. After a press verb, any name the keymap knows (`Key` and
//! `Button` in `shuyi-assets`: `Tab`, `Start`, `PageUp`, `DpadUp`, …) counts
//! too, in both scans, so a key added to the keymap is found with no change
//! here.
//!
//! It is a line-based scanner, not a parser: comments are ignored, and a
//! `#[cfg(test)]` item (usually `mod tests { … }`) is skipped by bracket
//! counting from the attribute to the end of the item.

use std::fs;
use std::path::Path;

use shuyi_assets::keymap::{Button, Key};

use crate::project::CheckKeys;

/// The comment that allows the next item (or its own line) to name keys, for
/// drawing a picture of a keyboard. Only honoured in
/// [`CheckKeys::picture_files`].
pub const MARKER: &str = "check-keys: keyboard picture";

/// Key and controller-button types: only the key pipeline names them.
const KEY_TYPES: [&str; 3] = ["Key::", "Chord::", "Button::"];
/// macroquad's keyboard: only `app/src/keys.rs` reads it.
const PLATFORM_READS: [&str; 6] = [
    "KeyCode",
    "is_key_down",
    "is_key_pressed",
    "get_keys_pressed",
    "get_char_pressed",
    "get_last_key_pressed",
];
/// Key names that must never appear in player text as whole words: the bare
/// words a help bar shows alone. The other names the keymap knows (`Tab`,
/// `Start`, `Up`, …) are ordinary words in a label, so they count only after
/// a press verb ([`is_keymap_name`]).
const KEY_WORDS: [&str; 8] = [
    "Space", "Esc", "Escape", "Enter", "Shift", "arrows", "WASD", "wasd",
];

/// Runs the scan over the project at `root` and returns every error
/// (empty = clean).
pub fn run(rules: &CheckKeys, root: &Path) -> Vec<String> {
    let mut errors = Vec::new();
    for dir in &rules.rust {
        for file in files_under(&root.join(dir), &mut errors) {
            let rel = relative(root, &file);
            if has_ext(&file, "rs") && !is_test_file(&rel, &rules.test_support) {
                read_and(&file, &rel, &mut errors, |rel, source| {
                    scan_rust(rules, rel, source)
                });
            }
        }
    }
    for assets in &rules.assets {
        for file in files_under(&root.join(assets), &mut errors) {
            let rel = relative(root, &file);
            if has_ext(&file, "dlg") {
                read_and(&file, &rel, &mut errors, |rel, source| {
                    scan_dlg(&rules.hint, rel, source)
                });
            } else if has_ext(&file, "ron") && !rules.not_text.contains(&rel) {
                read_and(&file, &rel, &mut errors, |rel, source| {
                    scan_ron(&rules.hint, rel, source)
                });
            }
        }
    }
    errors
}

/// Whether `file` has extension `ext` (any case).
pub(crate) fn has_ext(file: &Path, ext: &str) -> bool {
    file.extension()
        .is_some_and(|e| e.eq_ignore_ascii_case(ext))
}

/// Reads `file` and appends what `scan` finds in it, or the read error.
fn read_and(
    file: &Path,
    rel: &str,
    errors: &mut Vec<String>,
    scan: impl Fn(&str, &str) -> Vec<String>,
) {
    match fs::read_to_string(file) {
        Ok(source) => errors.extend(scan(rel, &source)),
        Err(e) => errors.push(format!("{rel}: cannot read: {e}")),
    }
}

/// Every file under `dir`, recursively, sorted so output is stable.
pub fn files_under(dir: &Path, errors: &mut Vec<String>) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let entries = match fs::read_dir(&d) {
            Ok(entries) => entries,
            Err(e) => {
                errors.push(format!("{}: cannot read directory: {e}", d.display()));
                continue;
            }
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                out.push(path);
            }
        }
    }
    out.sort();
    out
}

/// `file` relative to `root`, with `/` separators.
pub fn relative(root: &Path, file: &Path) -> String {
    let rel = file.strip_prefix(root).unwrap_or(file);
    rel.to_string_lossy().replace('\\', "/")
}

/// Test-only Rust files: `tests.rs`, `*_tests.rs`, anything in a `tests/`
/// directory, and the files in `support` (compiled only for tests).
pub(crate) fn is_test_file(rel: &str, support: &[String]) -> bool {
    let name = rel.rsplit('/').next().unwrap_or(rel);
    name == "tests.rs"
        || name.ends_with("_tests.rs")
        || rel.split('/').any(|part| part == "tests")
        || support.iter().any(|file| file == rel)
}

/// Scans one Rust file (`rel` is its project-relative path, for messages
/// and the allow-lists).
pub fn scan_rust(rules: &CheckKeys, rel: &str, source: &str) -> Vec<String> {
    let lines = lex(source);
    let listed = |files: &[String]| files.iter().any(|file| file == rel);
    let key_file = listed(&rules.key_files);
    let picture_file = listed(&rules.picture_files);
    let hint = &rules.hint;
    let platform_file = &rules.platform_file;
    let exempt = exempt_lines(&lines, picture_file.then_some(MARKER));
    let mut errors = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        let n = i + 1;
        if line.comment.contains(MARKER) && !picture_file {
            errors.push(format!(
                "{rel}:{n}: `// {MARKER}` is only allowed in {}",
                rules.picture_files.join(", ")
            ));
        }
        if exempt.get(i).copied().unwrap_or(false) {
            continue;
        }
        if !key_file {
            for pattern in KEY_TYPES {
                if contains_token(&line.code, pattern) {
                    errors.push(format!(
                        "{rel}:{n}: names a key or button with `{pattern}` — {hint}"
                    ));
                }
            }
        }
        if rel != platform_file {
            for pattern in PLATFORM_READS {
                if contains_token(&line.code, pattern) {
                    errors.push(format!(
                        "{rel}:{n}: reads the keyboard with `{pattern}` outside {platform_file} — \
                         {hint}"
                    ));
                }
            }
        }
        if !key_file {
            for text in &line.strings {
                if let Some(why) = names_key(text) {
                    errors.push(format!("{rel}:{n}: text {why}: {text:?} — {hint}"));
                }
            }
        }
    }
    errors
}

/// Scans one RON data file's string literals with the `prose_names_key`
/// rule: they are player-facing prose (tips, item and skill descriptions),
/// where "fires arrows" or "Shift the odds" are just words.
pub fn scan_ron(hint: &str, rel: &str, source: &str) -> Vec<String> {
    let mut errors = Vec::new();
    for (i, line) in lex(source).iter().enumerate() {
        for text in &line.strings {
            if let Some(why) = prose_names_key(text) {
                errors.push(format!("{rel}:{}: text {why}: {text:?} — {hint}", i + 1));
            }
        }
    }
    errors
}

/// Scans one `.dlg` dialogue script: every line but `#` comments, with the
/// `prose_names_key` rule like `.ron` text.
pub fn scan_dlg(hint: &str, rel: &str, source: &str) -> Vec<String> {
    let mut errors = Vec::new();
    for (i, line) in source.lines().enumerate() {
        if line.trim_start().starts_with('#') {
            continue;
        }
        if let Some(why) = prose_names_key(line) {
            errors.push(format!("{rel}:{}: text {why}: {line:?} — {hint}", i + 1));
        }
    }
    errors
}

/// Verbs that, followed by a key, tell the player to press it.
const PRESS_VERBS: [&str; 4] = ["press", "hit", "tap", "hold"];

/// Why prose `text` names a key, if it does: only an instruction to press
/// one ("press f", "hold Shift", "hit Space."), a `Shift+` chord, `WASD`, or
/// a bracketed letter (`[F]`). Bare key words are allowed, and so is the
/// article in "Press a key…" and "hold a button".
fn prose_names_key(text: &str) -> Option<String> {
    let words: Vec<&str> = text.split_whitespace().collect();
    if let Some(why) = press_instruction(&words, true) {
        return Some(why);
    }
    if text.contains("Shift+") {
        return Some("names a Shift+ chord".to_owned());
    }
    if contains_word(text, "WASD") || contains_word(text, "wasd") {
        return Some("names the key \"WASD\"".to_owned());
    }
    bracketed_letter(text).then(|| "names a key in brackets".to_owned())
}

/// Whether `word` is a name the keymap knows, of two or more ASCII letters
/// and digits starting with a letter (`Tab`, `Start`, `PageUp`). It matches
/// exactly as the keymap writes it, so "up" and "on" stay prose; the
/// numpad's punctuation names and the single characters are left to the
/// callers.
fn is_keymap_name(word: &str) -> bool {
    word.len() >= 2
        && word.starts_with(|c: char| c.is_ascii_alphabetic())
        && word.bytes().all(|b| b.is_ascii_alphanumeric())
        && (Key::from_name(word).is_some() || Button::from_name(word).is_some())
}

/// Why `words` tell the player to press a key, if they do: a press verb
/// followed by a letter, a bare [`KEY_WORDS`] word, a `Shift+` chord, a
/// function key or a keymap name. A single letter counts only with
/// `letters`: Rust strings say "hold a brace", and [`names_key`] has its own
/// rule for "press f". The article in "Press a key…" and "hold a
/// button" is not the `a` key.
fn press_instruction(words: &[&str], letters: bool) -> Option<String> {
    for (i, pair) in words.windows(2).enumerate() {
        let [verb, key] = pair else { continue };
        let next = words.get(i + 2).copied().unwrap_or_default();
        let next = next.trim_matches(|c: char| !c.is_alphanumeric());
        let noun = ["key", "button"]
            .iter()
            .any(|n| n.eq_ignore_ascii_case(next));
        if *key == "a" && noun {
            continue;
        }
        let key = key.trim_end_matches(|c: char| c.is_ascii_punctuation() && c != '+');
        let is_key = letters && key.len() == 1 && key.bytes().all(|b| b.is_ascii_alphabetic())
            || KEY_WORDS.iter().any(|w| w.eq_ignore_ascii_case(key))
            || key.starts_with("Shift+")
            || is_function_key(key)
            || is_keymap_name(key);
        if PRESS_VERBS.iter().any(|v| v.eq_ignore_ascii_case(verb)) && is_key {
            return Some(format!(
                "tells the player to {} \"{key}\"",
                verb.to_lowercase()
            ));
        }
    }
    None
}

/// Whether `key` is `F1`..`F12` (either case).
fn is_function_key(key: &str) -> bool {
    let Some(n) = key.strip_prefix(['F', 'f']) else {
        return false;
    };
    n.parse::<u8>().is_ok_and(|n| (1..=12).contains(&n))
}

/// Whether `text` has a single letter in brackets, like `[F]`.
fn bracketed_letter(text: &str) -> bool {
    text.as_bytes()
        .windows(3)
        .any(|w| w[0] == b'[' && w[1].is_ascii_alphabetic() && w[2] == b']')
}

/// Why `text` names a key to the player, if it does.
fn names_key(text: &str) -> Option<String> {
    if let Some(word) = KEY_WORDS.iter().find(|w| contains_word(text, w)) {
        return Some(format!("names the key \"{word}\""));
    }
    let words: Vec<&str> = text
        .split(|c: char| c.is_whitespace() || c == '·')
        .collect();
    let letter = |w: &str| w.len() == 1 && w.bytes().all(|b| b.is_ascii_alphabetic());
    for pair in words.windows(2) {
        let [first, second] = pair else { continue };
        // "press f", "Press F".
        if first.eq_ignore_ascii_case("press") && letter(second) {
            return Some(format!("names the key \"{second}\""));
        }
        // Help-bar hints: "f select", "d back". Keys print as lowercase
        // letters; "a" is a word, and a capital is a name ("Theme B").
        let lower = |w: &str| w.bytes().next().is_some_and(|b| b.is_ascii_lowercase());
        if letter(first) && lower(first) && *first != "a" && lower(second) {
            return Some(format!("names the key \"{first}\""));
        }
    }
    // "Press Tab", "hold Start": a keymap name after a press verb.
    if let Some(why) = press_instruction(&words, false) {
        return Some(why);
    }
    // "[F]", "[f]".
    bracketed_letter(text).then(|| "names a key in brackets".to_owned())
}

/// Whether `text` contains `word` with no identifier character either side.
fn contains_word(text: &str, word: &str) -> bool {
    text.match_indices(word).any(|(at, _)| {
        let before = text[..at].chars().next_back();
        let after = text[at + word.len()..].chars().next();
        !before.is_some_and(is_ident) && !after.is_some_and(is_ident)
    })
}

/// Whether `code` contains `pattern` not glued to a longer identifier (so
/// `RawKey::` isn't `Key::`, and `is_key_down_x` isn't `is_key_down`).
fn contains_token(code: &str, pattern: &str) -> bool {
    code.match_indices(pattern).any(|(at, _)| {
        let before = code[..at].chars().next_back();
        let after = code[at + pattern.len()..].chars().next();
        let ends_in_ident = pattern.chars().next_back().is_some_and(is_ident);
        !(before.is_some_and(is_ident) || ends_in_ident && after.is_some_and(is_ident))
    })
}

pub(crate) fn is_ident(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// For each line, whether it is skipped: inside a `#[cfg(test)]` item, or
/// covered by a comment holding `marker` (if one is given).
pub(crate) fn exempt_lines(lines: &[Line], marker: Option<&str>) -> Vec<bool> {
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
pub(crate) fn mark_item(lines: &[Line], start: usize, exempt: &mut [bool]) {
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
pub(crate) struct Line {
    /// The code, with comments removed and string/char literal contents
    /// blanked (the quotes stay).
    pub(crate) code: String,
    /// The text of every comment on the line.
    pub(crate) comment: String,
    /// Contents of the string literals that start on this line.
    pub(crate) strings: Vec<String>,
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
pub(crate) fn lex(source: &str) -> Vec<Line> {
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
        self.lines.push(Line::default());
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A game's rules, laid out as Shuyi's first game's are.
    pub(crate) fn rules() -> CheckKeys {
        let strings = |items: &[&str]| items.iter().map(|s| (*s).to_owned()).collect();
        CheckKeys {
            rust: strings(&[
                "crates/ui/src",
                "crates/app/src",
                "crates/content/src",
                "crates/playdate/src",
            ]),
            assets: strings(&["story", "test-files"]),
            not_text: strings(&["story/data/keymap.ron"]),
            key_files: strings(&[
                "crates/content/src/keymap.rs",
                "crates/ui/src/input.rs",
                "crates/playdate/src/pads.rs",
            ]),
            platform_file: "crates/app/src/keys.rs".to_owned(),
            test_support: strings(&["crates/ui/src/harness.rs"]),
            picture_files: strings(&[
                "crates/ui/src/screens/layout_picker.rs",
                "crates/ui/src/screens/layout_picker/glyph.rs",
            ]),
            hint: "use an Action and `widgets::help::key_name`; see the keyboard-input skill"
                .to_owned(),
        }
    }

    fn rust(rel: &str, src: &str) -> Vec<String> {
        scan_rust(&rules(), rel, src)
    }

    const SCREEN: &str = "crates/ui/src/screens/foo.rs";

    const HINT: &str = "use an Action";

    #[test]
    fn key_in_screen_code_is_an_error_naming_file_line_and_fix() {
        let errs = rust(SCREEN, "fn f(k: Key) -> bool {\n    k == Key::F\n}\n");
        assert_eq!(errs.len(), 1, "{errs:?}");
        assert!(
            errs[0].starts_with("crates/ui/src/screens/foo.rs:2: "),
            "{}",
            errs[0]
        );
        assert!(errs[0].contains("`Key::`"), "{}", errs[0]);
        assert!(
            errs[0].contains(
                "use an Action and `widgets::help::key_name`; see the keyboard-input skill"
            ),
            "{}",
            errs[0]
        );
    }

    #[test]
    fn press_f_in_help_text_is_an_error() {
        let errs = rust(
            SCREEN,
            "fn help() -> &'static str {\n    \"press f to go on\"\n}\n",
        );
        assert_eq!(errs.len(), 1, "{errs:?}");
        assert!(errs[0].starts_with("crates/ui/src/screens/foo.rs:2: text names the key \"f\""));
    }

    #[test]
    fn chords_and_macroquad_reads_are_errors() {
        let src = "let a = Chord::plain(k);\nlet b = is_key_pressed(KeyCode::F);\n";
        let errs = rust("crates/app/src/main.rs", src);
        assert_eq!(errs.len(), 3, "{errs:?}");
        assert!(errs[0].contains("`Chord::`"));
        assert!(errs[1].contains("`KeyCode`"));
        assert!(errs[2].contains("`is_key_pressed`"));
    }

    #[test]
    fn key_pipeline_files_are_allowed() {
        let src = "let a = Chord::plain(Key::F);\nlet s = \"Space\";\n";
        assert_eq!(rust("crates/ui/src/input.rs", src), Vec::<String>::new());
        assert_eq!(
            rust("crates/content/src/keymap.rs", src),
            Vec::<String>::new()
        );
        // Only the engine's keys.rs may read macroquad's keyboard.
        let read = "let d = is_key_down(KeyCode::A);\n";
        assert_eq!(rust(&rules().platform_file, read), Vec::<String>::new());
        assert_eq!(rust("crates/ui/src/input.rs", read).len(), 2);
        let errs = rust("crates/app/src/main.rs", read);
        assert_eq!(errs.len(), 2, "{errs:?}");
        assert!(errs[1].contains("outside crates/app/src/keys.rs"));
    }

    #[test]
    fn controller_buttons_are_named_only_in_the_key_pipeline() {
        // Ticket 0219: buttons are treated like keys.
        let src = "if b == Button::South {\n    confirm();\n}\n";
        let errs = rust(SCREEN, src);
        assert_eq!(errs.len(), 1, "{errs:?}");
        assert!(
            errs[0].starts_with(
                "crates/ui/src/screens/foo.rs:1: names a key or button with `Button::`"
            ),
            "{}",
            errs[0]
        );
        assert_eq!(rust("crates/app/src/main.rs", src).len(), 1);
        // The pipeline, and where the Playdate reads its buttons.
        let pipeline = "let a = (gilrs::Button::South, Button::South, Key::F);\n";
        for file in [
            "crates/content/src/keymap.rs",
            "crates/ui/src/input.rs",
            "crates/playdate/src/pads.rs",
        ] {
            assert_eq!(rust(file, pipeline), Vec::<String>::new(), "{file}");
        }
        // Not just any file whose name starts the same way, nor the game's
        // binary.
        assert_eq!(rust("crates/app/src/pads.rs", pipeline).len(), 2);
        assert_eq!(rust("crates/app/src/pads/native.rs", pipeline).len(), 2);
        assert_eq!(rust("crates/playdate/src/pd/entry.rs", pipeline).len(), 2);
        // Other things called button are not controller buttons.
        let other = "let m = MouseButton::Left;\nlet b = Button;\nfn f(b: Button) {}\n";
        assert_eq!(rust(SCREEN, other), Vec::<String>::new());
    }

    #[test]
    fn test_files_are_skipped() {
        let support = rules().test_support;
        let is_test_file = |support: &[String], rel: &str| is_test_file(rel, support);
        assert!(is_test_file(
            &support,
            "crates/ui/src/screens/battle/tests.rs"
        ));
        assert!(is_test_file(
            &support,
            "crates/ui/src/screens/battle/mode_tests.rs"
        ));
        assert!(is_test_file(&support, "crates/ui/src/tests/helpers.rs"));
        assert!(is_test_file(&support, "crates/ui/src/harness.rs"));
        assert!(!is_test_file(
            &support,
            "crates/ui/src/screens/battle/mod.rs"
        ));
        assert!(!is_test_file(&support, "crates/ui/src/contests.rs"));
    }

    #[test]
    fn cfg_test_modules_are_skipped() {
        let src = "\
fn real() {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn f() {
        tap(Key::F);
        assert_eq!(help(), \"f select\");
    }
}

fn after() { let _ = Key::D; }
";
        let errs = rust(SCREEN, src);
        assert_eq!(errs.len(), 1, "{errs:?}");
        assert!(
            errs[0].starts_with("crates/ui/src/screens/foo.rs:14: "),
            "{}",
            errs[0]
        );
    }

    #[test]
    fn cfg_test_single_items_and_extra_attributes_are_skipped() {
        let src = "\
#[cfg(test)]
use game_content::Key;
#[cfg(test)]
#[allow(dead_code)]
const K: Key = Key::F;
#[cfg(test)]
fn helper(
    k: Key,
) -> Chord {
    Chord::plain(k)
}
const L: Key = Key::J;
";
        let errs = rust(SCREEN, src);
        assert_eq!(errs.len(), 1, "{errs:?}");
        assert!(
            errs[0].starts_with("crates/ui/src/screens/foo.rs:12: "),
            "{}",
            errs[0]
        );
    }

    #[test]
    fn marker_allows_the_next_item_in_the_picture_file_only() {
        let src = "\
// check-keys: keyboard picture
const ROW: [Key; 2] = [
    Key::Q,
    Key::W,
];
const UP: Key = Key::Up; // check-keys: keyboard picture
const DOWN: Key = Key::Down;
";
        for picture in &rules().picture_files {
            let errs = rust(picture, src);
            assert_eq!(errs.len(), 1, "{picture}: {errs:?}");
            assert!(
                errs[0].starts_with(&format!("{picture}:7: ")),
                "{}",
                errs[0]
            );
        }
        // Elsewhere the marker is itself an error and allows nothing.
        let errs = rust(SCREEN, src);
        let markers = errs
            .iter()
            .filter(|e| e.contains("is only allowed in"))
            .count();
        assert_eq!(markers, 2, "{errs:?}");
        assert_eq!(errs.len(), 2 + 4, "{errs:?}");
    }

    #[test]
    fn comments_and_similar_identifiers_are_not_hits() {
        let src = "\
/// Press the Confirm key (`Key::F` by default) to go on.
// Space is the End turn key.
/* Key::D, \"press d\" */
let a = RawKey::Down;
let b = my_is_key_down(x);
let c = LAYOUT_KEY;
let d = is_key_down_fast(x);
";
        assert_eq!(rust(SCREEN, src), Vec::<String>::new());
    }

    #[test]
    fn strings_that_name_keys() {
        for text in [
            "f select · d back",
            "arrows move",
            "wasd move",
            "Space end turn",
            "Press F to continue",
            "hit [F]",
            "Shift+Space auto-end",
            "Esc",
            "Enter to confirm",
            "Coming soon — press d to go back",
        ] {
            assert!(names_key(text).is_some(), "{text}");
        }
        for text in [
            "E ",
            "a spell",
            "Battle Theme B for RPG",
            "I think so",
            "Iron Sword",
            "Weapons",
            "{Confirm} select · {Cancel} back",
            "{Cursor} move",
            "spacesuit",
            "Entering the keep",
            "x",
        ] {
            assert_eq!(names_key(text), None, "{text}");
        }
    }

    #[test]
    fn ron_strings_and_dlg_lines_are_scanned() {
        let ron = "(\n  // press f here is a comment\n  text: \"Press {Confirm} to go on\",\n  bad: \"Press f to go on\",\n  bow: \"Fires arrows. Escape is harder.\",\n  art: \"Shift the odds: Enter a stance.\",\n)";
        let errs = scan_ron(HINT, "story/data/tips.ron", ron);
        // Only the instruction to press a key; key words as prose pass.
        assert_eq!(errs.len(), 1, "{errs:?}");
        assert!(
            errs[0].starts_with("story/data/tips.ron:4: "),
            "{}",
            errs[0]
        );

        // "a key" and "a button" are not the `a` key; `a` alone is, and so
        // is the A button.
        for ok in ["Press a key…", "Hold a button.", "PRESS a KEY…"] {
            assert_eq!(
                scan_ron(HINT, "x.ron", &format!("[\"{ok}\"]")),
                [""; 0],
                "{ok}"
            );
        }
        for bad in [
            "Press a",
            "press a to attack",
            "Press a.",
            "Press A button",
            "Press a keyhole",
            "Press b key",
        ] {
            let errs = scan_ron(HINT, "x.ron", &format!("[\"{bad}\"]"));
            assert_eq!(errs.len(), 1, "{bad}: {errs:?}");
        }

        let dlg =
            "# press f is a comment\n@scene s\nlord: Press Space to end your turn.\nlord: Fine.\n";
        let errs = scan_dlg(HINT, "story/dialogue/x.dlg", dlg);
        assert_eq!(errs.len(), 1, "{errs:?}");
        assert!(
            errs[0].starts_with("story/dialogue/x.dlg:3: "),
            "{}",
            errs[0]
        );
    }

    #[test]
    fn dialogue_prose_may_use_key_words_as_words() {
        for line in [
            "knight: Escape while you can!",
            "bowman: A rain of arrows, then we Enter the keep.",
            "> The wind began to Shift.",
            "lord: Space enough for all of us.",
            "lord: I think so. Hold fast!",
            "lord: Press on, press forward.",
            "lord: Plan B, then.",
            "test_hero[happy]: Better late than never.",
            "lord: Hit it with an F1 car? No.",
        ] {
            assert_eq!(prose_names_key(line), None, "{line}");
        }
    }

    #[test]
    fn dialogue_telling_the_player_to_press_a_key_is_an_error() {
        for line in [
            "lord: Press f to attack.",
            "lord: press F!",
            "lord: Hit Space to end your turn.",
            "lord: hold shift, then tap Enter.",
            "lord: Tap Esc to go back.",
            "lord: press Shift+Space.",
            "lord: Shift+Space ends turns for you.",
            "lord: Move with WASD.",
            "lord: Use [F] to confirm.",
            "lord: Press F2 for the debug view.",
            "lord: Hold arrows to scroll.",
        ] {
            assert!(prose_names_key(line).is_some(), "{line}");
        }
        assert_eq!(
            prose_names_key("lord: Hit Space.").as_deref(),
            Some("tells the player to hit \"Space\"")
        );
    }

    #[test]
    fn a_keymap_name_after_a_press_verb_is_a_key() {
        for line in [
            "lord: Press Tab to switch.",
            "lord: press Start!",
            "lord: Hold Backspace to clear.",
            "lord: Tap DpadUp.",
        ] {
            assert!(prose_names_key(line).is_some(), "{line}");
        }
        assert_eq!(
            prose_names_key("lord: Press Tab to switch.").as_deref(),
            Some("tells the player to press \"Tab\"")
        );
        assert!(names_key("Press Tab to switch").is_some());
        assert_eq!(
            names_key("Press Tab to switch").as_deref(),
            Some("tells the player to press \"Tab\"")
        );
    }

    #[test]
    fn keymap_names_as_plain_words_stay_prose() {
        for line in [
            "lord: Hold up!",
            "lord: Hold down the fort.",
            "lord: Press on, press forward.",
            "lord: We hold the North.",
        ] {
            assert_eq!(prose_names_key(line), None, "{line}");
        }
        for text in ["Start game", "Select a file", "End turn"] {
            assert_eq!(names_key(text), None, "{text}");
        }
    }

    #[test]
    fn every_keymap_name_is_found_after_a_press_verb() {
        let names = Key::ALL
            .iter()
            .map(|k| k.name())
            .chain(Button::ALL.iter().map(|b| b.name()))
            .filter(|n| {
                n.len() >= 2
                    && n.starts_with(|c: char| c.is_ascii_alphabetic())
                    && n.bytes().all(|b| b.is_ascii_alphanumeric())
            });
        let mut count = 0;
        for name in names {
            count += 1;
            let text = format!("press {name}");
            assert!(prose_names_key(&text).is_some(), "{text}");
            assert!(names_key(&text).is_some(), "{text}");
        }
        assert!(count > 40, "{count}");
    }

    #[test]
    fn dlg_press_start_is_one_hit_on_its_line() {
        let errs = scan_dlg(HINT, "x.dlg", "@scene s\nlord: Press Start to begin.\n");
        assert_eq!(errs.len(), 1, "{errs:?}");
        assert!(errs[0].starts_with("x.dlg:2: "), "{}", errs[0]);
        assert!(
            errs[0].contains("tells the player to press \"Start\""),
            "{}",
            errs[0]
        );
    }

    #[test]
    fn function_keys() {
        assert!(is_function_key("F1"));
        assert!(is_function_key("f12"));
        assert!(!is_function_key("F0"));
        assert!(!is_function_key("F13"));
        assert!(!is_function_key("Fx"));
        assert!(!is_function_key("G1"));
    }

    #[test]
    fn lexer_handles_escapes_raw_strings_chars_and_lifetimes() {
        let src = "let a: &'static str = \"a \\\"q\\\" b\";\nlet c = '\"';\nlet r = r#\"x \"y\" z\"#; // note\nlet n = '\\n';\nlet s = \"one \\\n    two\";\n";
        let lines = lex(src);
        assert_eq!(lines[0].strings, vec!["a \"q\" b".to_owned()]);
        assert_eq!(lines[0].code, "let a: &'static str = \"\";");
        assert_eq!(lines[1].strings, Vec::<String>::new());
        assert_eq!(lines[1].code, "let c = '';");
        assert_eq!(lines[2].strings, vec!["x \"y\" z".to_owned()]);
        assert_eq!(lines[2].comment, " note");
        assert_eq!(lines[3].code, "let n = '';");
        assert_eq!(lines[4].strings, vec!["one two".to_owned()]);
    }

    #[test]
    fn lexer_handles_tabs_byte_raw_strings_and_a_trailing_backslash() {
        let lines = lex(r#"let t = "a\tb"; let b = br"c\d"; let e = "x\"#);
        assert_eq!(lines[0].strings, vec!["a\tb".to_owned(), r"c\d".to_owned()]);
        // An unterminated string at end of input just stops.
        assert_eq!(lines.len(), 1);
    }

    #[test]
    fn lexer_keeps_newlines_in_strings_and_slashes_in_code() {
        let lines = lex("let s = \"a\nb\"; let d = a / b * c; let k = Key::F;");
        assert_eq!(lines[0].strings, vec!["a\nb".to_owned()]);
        assert_eq!(lines[1].code, "\"; let d = a / b * c; let k = Key::F;");
        let lines = lex("x = 'a'; y = \"s\";");
        assert_eq!(lines[0].code, "x = ''; y = \"\";");
        assert_eq!(lines[0].strings, vec!["s".to_owned()]);
    }

    #[test]
    fn r_ending_an_identifier_does_not_start_a_raw_string() {
        // `abr"…"` is identifier `abr` then an ordinary string, unlike `br"…"`.
        let lines = lex(r#"abr"a\"b"; br"c\";"#);
        assert_eq!(lines[0].strings, vec!["a\"b".to_owned(), r"c\".to_owned()]);
    }

    #[test]
    fn nested_block_comments_close() {
        let lines = lex("/* a /* b */ Key::F */ let x = Key::G;");
        assert_eq!(lines[0].code.trim(), "let x = Key::G;");
    }

    #[test]
    fn run_reports_hits_in_a_repo_tree() {
        let root = std::env::temp_dir().join(format!("shuyi-check-keys-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let screens = root.join("crates/ui/src/screens");
        fs::create_dir_all(&screens).unwrap();
        fs::create_dir_all(root.join("story/data")).unwrap();
        fs::write(screens.join("foo.rs"), "fn f() { g(Key::F); }\n").unwrap();
        fs::write(screens.join("tests.rs"), "fn f() { g(Key::F); }\n").unwrap();
        fs::write(root.join("story/data/tips.ron"), "[\"press f\"]\n").unwrap();
        // Neither the keymap file nor a file that isn't RON is player text.
        fs::write(root.join("story/data/keymap.ron"), "(\"press f\")\n").unwrap();
        fs::write(root.join("story/data/notes.md"), "\"press f\"\n").unwrap();
        fs::create_dir_all(root.join("test-files/dialogue")).unwrap();
        let script = "@scene s\nlord: Press Space to end your turn.\n";
        fs::write(root.join("test-files/dialogue/x.dlg"), script).unwrap();
        let rules = rules();
        let errs = run(&rules, &root);
        let _ = fs::remove_dir_all(&root);
        // Missing crates/{app,content,playdate}/src are reported too.
        let hits: Vec<&String> = errs.iter().filter(|e| e.contains(&rules.hint)).collect();
        assert_eq!(hits.len(), 3, "{errs:?}");
        assert!(
            hits[0].starts_with("crates/ui/src/screens/foo.rs:1: "),
            "{}",
            hits[0]
        );
        assert!(
            hits[1].starts_with("story/data/tips.ron:1: "),
            "{}",
            hits[1]
        );
        assert!(
            hits[2].starts_with("test-files/dialogue/x.dlg:2: "),
            "{}",
            hits[2]
        );
        assert_eq!(errs.len(), 6, "{errs:?}");
    }
}
