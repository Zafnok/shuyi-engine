//! `check-text`: counts the player-facing text still written as string
//! literals in the screens' source ([`CheckText::dir`]), instead of asked
//! for by key with `ctx.text("screen.thing")` from the game's language
//! file. It fails when the count is above [`CheckText::max_literals`].
//!
//! Like `check-keys` it is a line-based scanner, not a parser, built on
//! the same lexer. A string literal counts when it holds a word (letters
//! outside `{…}` placeholders) and is
//!
//! - an argument of a call that puts text on the screen (`print…`,
//!   `draw…`, `MenuItem::new`, `MenuItem::disabled`, `with_suffix`,
//!   `help_line`), directly or inside a `format!` there; or
//! - prose, wherever it stands: it has a capital letter or a space
//!   (`format!("Turn {n}")` kept in a variable, `"out of range".to_owned()`,
//!   a `&str` constant). Cue names, storage keys and screen names
//!   (`"menu_tick"`) don't; or
//! - one lowercase word (`"acted"`, `"info"`) that is the value of a
//!   `let`, a `match` arm or an `if`, or stands in a tuple or an array: a
//!   label. As a constant, a call's own argument (`Some("heal")`), a
//!   pattern (`"foot" =>`) or a value's name (`("count", &n)`) it is an id.
//!
//! The arguments of `text(…)`, `text_with(…)`, `MenuItem::keyed(…)` and
//! `Label::key(…)` are keys and value names, never text; those of an assertion, a panic or `expect` are never shown,
//! and neither is an attribute's (`#[error("…")]`). Tests, the test
//! harness, the snapshots' text form and the debug screens are skipped.
//! A literal the rules get wrong is let through with a
//! `// check-text: not player text` comment, with the reason, on its line
//! or the line above its item.
//!
//! It also fails on a screen that reads a `name` field itself: the data's
//! names are English, and a screen shows one through the game's helpers
//! ([`CheckText::words_file`]), which give it in the player's language. Reading `.name` (not calling `.name()`) on anything
//! but `self` counts; a view's own field is let through with a
//! `// check-text: not a data name` comment.

use std::cmp::Ordering;
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use crate::check_keys::{
    Line, exempt_lines, files_under, has_ext, is_ident, is_test_file, lex, mark_item, relative,
};
use crate::project::CheckText;

/// The comment that lets the next item (or its own line) through.
pub const MARKER: &str = "check-text: not player text";

/// The comment that lets a `.name` read on its line (or the item below
/// it) through.
pub const NAME_MARKER: &str = "check-text: not a data name";

/// One literal found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hit {
    /// Repo-relative path of its file.
    pub file: String,
    /// Its 1-based line.
    pub line: usize,
    /// The literal's text.
    pub text: String,
}

/// Scans the project at `root` and returns every literal found, by file
/// then line, and every file that couldn't be read.
pub fn run(rules: &CheckText, root: &Path) -> (Vec<Hit>, Vec<String>) {
    let mut errors = Vec::new();
    let mut hits = Vec::new();
    for file in files_under(&root.join(&rules.dir), &mut errors) {
        let rel = relative(root, &file);
        if !has_ext(&file, "rs") || is_skipped(rules, &rel) {
            continue;
        }
        match fs::read_to_string(&file) {
            Ok(source) => hits.extend(scan(&rel, &source)),
            Err(e) => errors.push(format!("{rel}: cannot read: {e}")),
        }
    }
    (hits, errors)
}

/// Scans the project at `root` and returns what is wrong, one line each:
/// every `.name` field read found ([`scan_names`]), by file then line, and
/// every file that couldn't be read. Empty when the screens read none.
pub fn run_names(rules: &CheckText, root: &Path) -> Vec<String> {
    let mut errors = Vec::new();
    let mut found = Vec::new();
    for file in files_under(&root.join(&rules.dir), &mut errors) {
        let rel = relative(root, &file);
        if !has_ext(&file, "rs") || is_skipped(rules, &rel) || rel == rules.words_file {
            continue;
        }
        let error = |hit: &Hit| name_error(&rules.name_hint, hit);
        match fs::read_to_string(&file) {
            Ok(source) => found.extend(scan_names(&rel, &source).iter().map(error)),
            Err(e) => errors.push(format!("{rel}: cannot read: {e}")),
        }
    }
    found.extend(errors);
    found
}

/// What the `.name` read `hit` is told.
fn name_error(told: &str, hit: &Hit) -> String {
    format!(
        "{}:{}: `{}` is read here — {told}",
        hit.file, hit.line, hit.text
    )
}

/// The `.name` field reads of one Rust file (`rel` is its repo-relative
/// path, for the report): `.name` not followed by `(`, on anything but
/// `self`, outside tests and lines let through by [`NAME_MARKER`]. A hit's
/// text is what was read (`unit.name`).
pub fn scan_names(rel: &str, source: &str) -> Vec<Hit> {
    const FIELD: &str = ".name";
    let lines = lex(source);
    let exempt = exempt_lines(&lines, Some(NAME_MARKER));
    let mut hits = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        if exempt.get(i).copied().unwrap_or(false) {
            continue;
        }
        for (at, _) in line.code.match_indices(FIELD) {
            let after = line.code[at + FIELD.len()..].chars().next();
            if after.is_some_and(|c| is_ident(c) || c == '(') {
                continue;
            }
            let before = &line.code[..at];
            let start = before
                .char_indices()
                .rev()
                .take_while(|&(_, c)| is_ident(c))
                .last()
                .map_or(at, |(start, _)| start);
            let receiver = &before[start..];
            if receiver != "self" {
                hits.push(Hit {
                    file: rel.to_owned(),
                    line: i + 1,
                    text: format!("{receiver}{FIELD}"),
                });
            }
        }
    }
    hits
}

/// Whether the file `rel` is left out: tests and the files the rules
/// skip (a path ending in `/` skips a folder).
fn is_skipped(rules: &CheckText, rel: &str) -> bool {
    is_test_file(rel, &rules.test_support)
        || rules.skip.iter().any(|skip| {
            if skip.ends_with('/') {
                rel.starts_with(skip.as_str())
            } else {
                rel == skip
            }
        })
}

/// What an open bracket belongs to.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Bracket {
    /// A call that puts its text on the screen.
    Sink,
    /// `text(…)` or `text_with(…)`: keys and value names.
    Key,
    /// A call whose text no player reads: an assertion, a panic, a log.
    Quiet,
    /// Any other call or macro (`format!(…)`, `Some(…)`).
    Call,
    /// Anything else: a tuple, an array, a block.
    Other,
}

impl Bracket {
    /// Whether it says what the literals inside it are, however deep.
    fn decides(self) -> bool {
        matches!(self, Bracket::Sink | Bracket::Key | Bracket::Quiet)
    }
}

/// The calls and macros whose text no player reads, by name; anything
/// starting with `assert` or `debug_assert` is one too.
const QUIET: [&str; 15] = [
    "panic",
    "unreachable",
    "unimplemented",
    "todo",
    "expect",
    "expect_err",
    "println",
    "eprintln",
    "dbg",
    // `Ctx::log(…)`: the log is English, never translated.
    "log",
    // Attributes: `#[allow(…, reason = "…")]`.
    "allow",
    "warn",
    "deny",
    "cfg",
    "cfg_attr",
];

/// What the call `path(` is (`path` as written: `MenuItem::new`, `print`).
fn bracket_of(path: &str) -> Bracket {
    let name = path.rsplit("::").next().unwrap_or(path);
    if name == "text"
        || name == "text_with"
        || path.ends_with("MenuItem::keyed")
        || path.ends_with("Label::key")
    {
        Bracket::Key
    } else if name.starts_with("print") && name != "println"
        || name.starts_with("draw")
        || name == "help_line"
        || name == "with_suffix"
        || path.ends_with("MenuItem::new")
        || path.ends_with("MenuItem::disabled")
    {
        Bracket::Sink
    } else if QUIET.contains(&name)
        || name.starts_with("assert")
        || name.starts_with("debug_assert")
    {
        Bracket::Quiet
    } else if name.is_empty() {
        Bracket::Other
    } else {
        Bracket::Call
    }
}

/// Whether `text` holds a word: a letter outside `{…}` placeholders.
fn has_word(text: &str) -> bool {
    let mut depth = 0_u32;
    for c in text.chars() {
        match c {
            '{' => depth += 1,
            '}' => depth = depth.saturating_sub(1),
            c if depth == 0 && c.is_alphabetic() => return true,
            _ => {}
        }
    }
    false
}

/// Whether `text` looks like something a player reads rather than an id:
/// it has a capital letter or a space.
fn is_prose(text: &str) -> bool {
    text.chars().any(|c| c.is_uppercase() || c == ' ')
}

/// Whether `text` is one plain lowercase word (`acted`, `info`): a label
/// as likely as an id, told apart by where it stands.
fn is_bare_word(text: &str) -> bool {
    !text.is_empty() && text.chars().all(char::is_lowercase)
}

/// Whether the literal that opens at byte `open` of `code` names
/// something rather than says it, by what follows it: a pattern
/// (`"foot" => …`, `"a" | "b"`), or the name of a value in a list made
/// for `text_with` (`("count", &n)`).
fn is_name(code: &str, open: usize) -> bool {
    let rest = code.get(open + 1..).unwrap_or("");
    let after = rest.find('"').and_then(|close| rest.get(close + 1..));
    after.is_some_and(|after| {
        let after = after.trim_start();
        after.starts_with("=>") || after.starts_with('|') || after.starts_with(", &")
    })
}

/// For each line, whether it is part of a `const` or `static` item of
/// `&str`s.
fn const_lines(lines: &[Line]) -> Vec<bool> {
    let mut inside = vec![false; lines.len()];
    for (i, line) in lines.iter().enumerate() {
        let code = line.code.trim_start();
        let code = code.strip_prefix("pub ").unwrap_or(code);
        let code = match code.strip_prefix("pub(") {
            Some(rest) => rest.split_once(") ").map_or(code, |(_, rest)| rest),
            None => code,
        };
        let item = code.starts_with("const ") || code.starts_with("static ");
        let of_str = code.contains("&str") || code.contains("&'static str");
        if item && of_str {
            mark_item(lines, i, &mut inside);
        }
    }
    inside
}

/// Whether the literal `text` counts where it stands: `open` are the
/// brackets around it (innermost last), `in_const` whether it is in a
/// `&str` constant, `name` whether it is a pattern or a value's name
/// ([`is_name`]).
fn counts(text: &str, open: &[Bracket], in_const: bool, name: bool) -> bool {
    if !has_word(text) {
        return false;
    }
    // The innermost call that says what its arguments are.
    match open.iter().rev().find(|b| b.decides()) {
        Some(Bracket::Sink) => return true,
        Some(_) => return false,
        None => {}
    }
    if is_prose(text) {
        return true;
    }
    // One lowercase word is an id as a constant, a call's argument, a
    // pattern or a value's name (`Some("heal")`, `"foot" =>`), and a
    // label anywhere else: a `let`, a `match` arm, a tuple.
    let argument = open.last() == Some(&Bracket::Call);
    is_bare_word(text) && !in_const && !argument && !name
}

/// Scans one Rust file (`rel` is its repo-relative path, for the report).
pub fn scan(rel: &str, source: &str) -> Vec<Hit> {
    let lines = lex(source);
    let exempt = exempt_lines(&lines, Some(MARKER));
    let consts = const_lines(&lines);
    let mut hits = Vec::new();
    let mut open: Vec<Bracket> = Vec::new();
    let mut in_string = false;
    for (i, line) in lines.iter().enumerate() {
        let skipped = exempt.get(i).copied().unwrap_or(false);
        let in_const = consts.get(i).copied().unwrap_or(false);
        // An attribute's text is for a tool: `#[error("…")]`, `#[serde(…)]`.
        let skipped = skipped || line.code.trim_start().starts_with("#[");
        let mut strings = line.strings.iter();
        let mut path = String::new();
        for (at, c) in line.code.char_indices() {
            if c == '"' {
                in_string = !in_string;
                path.clear();
                if !in_string {
                    continue;
                }
                let Some(text) = strings.next() else {
                    continue;
                };
                let name = is_name(&line.code, at);
                if !skipped && counts(text, &open, in_const, name) {
                    hits.push(Hit {
                        file: rel.to_owned(),
                        line: i + 1,
                        text: text.clone(),
                    });
                }
                continue;
            }
            match c {
                '(' => open.push(bracket_of(&path)),
                '[' | '{' => open.push(Bracket::Other),
                ')' | ']' | '}' => {
                    open.pop();
                }
                _ => {}
            }
            // A macro's `!` stays part of its name's place: `format!(`.
            if is_ident(c) || c == ':' {
                path.push(c);
            } else if c != '!' {
                path.clear();
            }
        }
    }
    hits
}

/// The report: the count in the folder `dir` against `max`, then the
/// literals by file.
pub fn report(dir: &str, hits: &[Hit], max: usize) -> String {
    let mut out = format!(
        "check-text: {} player-facing string literal(s) in {dir} (at most {max} allowed)\n",
        hits.len()
    );
    let mut file = "";
    for hit in hits {
        if hit.file != file {
            file = &hit.file;
            let count = hits.iter().filter(|h| h.file == file).count();
            // Writing to a `String` can't fail.
            let _ = writeln!(out, "{file}: {count}");
        }
        let _ = writeln!(out, "  {}: {:?}", hit.line, hit.text);
    }
    out
}

/// What the count means against `max`: an error when above it, a note
/// when below it (the number in the project file should come down too).
/// `hint` says what to do about a literal.
pub fn verdict(hint: &str, count: usize, max: usize) -> Result<Option<String>, String> {
    match count.cmp(&max) {
        Ordering::Greater => Err(format!(
            "check-text: {count} literals, over the {max} allowed — {hint}"
        )),
        Ordering::Less => Ok(Some(format!(
            "check-text: {count} literals, under the {max} allowed: lower \
             `check_text.max_literals` in shuyi.ron to {count}"
        ))),
        Ordering::Equal => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCREEN: &str = "crates/ui/src/screens/foo.rs";

    const HINT: &str = "put the text in the language file";
    const NAME_HINT: &str = "show the name through the words helpers";

    /// A game's rules, laid out as Shuyi's first game's are.
    fn rules() -> CheckText {
        let ui = |file: &str| format!("crates/ui/src/{file}");
        CheckText {
            dir: "crates/ui/src".to_owned(),
            max_literals: 0,
            words_file: ui("words.rs"),
            skip: vec![ui("debug.rs"), ui("debug/"), ui("snapshot.rs")],
            test_support: vec![ui("harness.rs")],
            hint: HINT.to_owned(),
            name_hint: NAME_HINT.to_owned(),
        }
    }

    /// The `.name` reads found in `source`, as `line: text`.
    fn names(source: &str) -> Vec<String> {
        scan_names(SCREEN, source)
            .iter()
            .map(|h| format!("{}: {}", h.line, h.text))
            .collect()
    }

    #[test]
    fn a_name_field_read_is_found_and_a_call_is_not() {
        let source = "fn draw(&self, unit: &Unit, class: &ClassDef) {
    print(&unit.name);
    let n = (class.name.len(), state.unit(id).name.clone());
    let fine = (screen.name(), self.name.len(), unit.names, unit.name_w, \"x.name\");
    let label = def.name; // check-text: not a data name
    // check-text: not a data name
    let row = Row {
        text: page.name.clone(),
    };
    print(table.items[0].name)
}

#[cfg(test)]
mod tests {
    fn t() { unit.name = String::new(); }
}
";
        assert_eq!(
            names(source),
            ["2: unit.name", "3: class.name", "3: .name", "10: .name"]
        );
        let hits = scan_names(SCREEN, source);
        assert_eq!((hits[0].file.as_str(), hits[0].line), (SCREEN, 2));
    }

    #[test]
    fn a_name_read_is_told_where_it_is_and_what_to_do() {
        let hits = scan_names(SCREEN, "fn f() { a.name }\n");
        assert_eq!(
            name_error(NAME_HINT, &hits[0]),
            format!("{SCREEN}:1: `a.name` is read here — {NAME_HINT}")
        );
    }

    #[test]
    fn name_reads_are_looked_for_in_screens_but_not_tests_debug_or_the_helpers() {
        let root = std::env::temp_dir().join(format!("shuyi-check-names-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let src = root.join("crates/ui/src");
        fs::create_dir_all(src.join("screens")).unwrap();
        fs::create_dir_all(src.join("debug")).unwrap();
        let read = "fn f(u: &Unit) -> &str { &u.name }\n";
        for file in [
            "screens/foo.rs",
            "screens/tests.rs",
            "debug.rs",
            "debug/menu.rs",
            "harness.rs",
            "words.rs",
            "screens/notes.txt",
        ] {
            fs::write(src.join(file), read).unwrap();
        }
        let found = run_names(&rules(), &root);
        let _ = fs::remove_dir_all(&root);
        assert_eq!(
            found,
            [format!(
                "crates/ui/src/screens/foo.rs:1: `u.name` is read here — {NAME_HINT}"
            )]
        );
        // A missing tree is reported, not passed over.
        assert_eq!(run_names(&rules(), &root).len(), 1);
    }

    /// The literals found in `source`, as `line: text`.
    fn found(source: &str) -> Vec<String> {
        scan(SCREEN, source)
            .iter()
            .map(|h| format!("{}: {}", h.line, h.text))
            .collect()
    }

    #[test]
    fn literals_passed_to_print_and_the_text_widgets_count() {
        let source = "\
fn draw(&self, ctx: &Ctx, buf: &mut GlyphBuffer) {
    buf.print(1, 2, \"Hello\", fg, bg);
    buf.print_fg(1, 3, \"again\", fg);
    print_centred(buf, 4, &format!(\"Turn {n} of {max}\"), fg, bg);
    let items = vec![MenuItem::new(\"Retry\"), MenuItem::disabled(\"quit\")];
    let item = MenuItem::new(label).with_suffix(\"(broken)\", UiColor::HpLow);
    let help = help_line(&[(Some(key), \"select\")]);
    draw_dialog(buf, \"Notes\", &lines);
}
";
        assert_eq!(
            found(source),
            [
                "2: Hello",
                "3: again",
                "4: Turn {n} of {max}",
                "5: Retry",
                "5: quit",
                "6: (broken)",
                "7: select",
                "8: Notes",
            ]
        );
        let hits = scan(SCREEN, source);
        assert_eq!(hits[0].file, SCREEN);
        assert_eq!(hits[0].line, 2);
    }

    /// Square brackets and blocks inside a call don't end it, and those
    /// after it don't reopen it.
    #[test]
    fn brackets_and_blocks_are_matched() {
        let source = "\
fn draw(&self, buf: &mut GlyphBuffer) {
    buf.print(xs[0], rows[1], \"After an index\", fg, bg);
    draw_rows(buf, |b| { b.height() }, \"After a block\");
    let label = [other(\"not_shown\")];
    if wide { buf.print(0, 0, \"In a block\", fg, bg); }
    let after = other(\"nor_this\");
}
";
        assert_eq!(
            found(source),
            ["2: After an index", "3: After a block", "5: In a block"]
        );
    }

    #[test]
    fn text_by_key_and_other_calls_do_not_count() {
        let source = "\
fn draw(&self, ctx: &Ctx, buf: &mut GlyphBuffer) {
    print_centred(buf, 4, ctx.text(\"title.new_game\"), fg, bg);
    buf.print(1, 2, &ctx.text_with(\"battle.turn\", &[(\"count\", &n)]), fg, bg);
    let label = ctx.text(\"title.quit\");
    ctx.audio.play_sound(\"menu_tick\");
    let s = format!(\"{a}/{b} {c:>3}\");
    buf.print(1, 2, &format!(\"{}/{}\", a, b), fg, bg);
    buf.print(1, 2, \" · \", fg, bg);
    buf.print(1, 2, \"\", fg, bg);
    let x = ctx.text_with(\"save.slot\", &[(\"slot\", &format!(\"No {n}\"))]);
}
";
        assert_eq!(found(source), Vec::<String>::new());
    }

    /// Ticket 0243: text made first and printed later.
    #[test]
    fn a_format_kept_or_returned_counts() {
        let source = "\
fn turn_text(turn: u32, limit: Option<u32>) -> String {
    match limit {
        Some(limit) => format!(\"Turn {turn}/{limit}\"),
        None => format!(\"Turn {turn}\"),
    }
}
fn draw(&self, buf: &mut GlyphBuffer) {
    let ask = format!(\"End turn with {ready} units ready?\");
    let hold = Some(format!(\"hold {}\", key));
    let cost = format!(\"{n} dur\");
    lines.push((format!(\"{who} waited\"), color));
}
";
        assert_eq!(
            found(source),
            [
                "3: Turn {turn}/{limit}",
                "4: Turn {turn}",
                "8: End turn with {ready} units ready?",
                "9: hold {}",
                "10: {n} dur",
                "11: {who} waited",
            ]
        );
    }

    /// Ticket 0243: prose counts wherever it stands.
    #[test]
    fn prose_outside_a_call_counts() {
        let source = "\
fn reason(error: &Error) -> String {
    match error {
        Error::Range => \"out of range\".to_owned(),
        Error::Broken => String::from(\"Weapon broken\"),
    }
}
fn label(self) -> &'static str {
    if self.mounted { \"Mounted\" } else { \"On foot\" }
}
";
        assert_eq!(
            found(source),
            [
                "3: out of range",
                "4: Weapon broken",
                "8: Mounted",
                "8: On foot"
            ]
        );
    }

    /// Ticket 0243: a help bar's labels kept in a variable, and `match`
    /// arms and `if`s that give a word.
    #[test]
    fn a_lowercase_word_is_a_label_unless_it_is_an_argument_a_pattern_or_a_name() {
        let source = "\
fn help(&self, km: HelpKeys) -> String {
    let info = (Some(key), \"info\");
    let label = \"attack\";
    let status = if unit.acted { \"acted\" } else { \"ready\" };
    let verb = match stat {
        StatKind::Mov => \"pins\",
        _ => \"lowers\",
    };
    let flags = [(mods.double, \"double\"), (mods.single, \"single\")];
    help_line(&[info])
}
fn ids(&self, movement: &str) -> Option<&'static str> {
    let cue = Some(\"heal\");
    let id = ItemId::new(\"potion\");
    let step = match movement {
        \"foot\" => \"step_walk\",
        \"horse\" | \"wyvern\" => \"step_ride\",
        _ => return None,
    };
    let values = [(\"count\", &n as &dyn Display), (\"name\", &name)];
    let line = ctx.text_with(\"battle.line\", &values);
    cue
}
const SCREEN: &str = \"battle\";
";
        assert_eq!(
            found(source),
            [
                "2: info",
                "3: attack",
                "4: acted",
                "4: ready",
                "6: pins",
                "7: lowers",
                "9: double",
                "9: single",
            ]
        );
    }

    #[test]
    fn asserts_panics_and_attributes_are_never_shown() {
        let source = "\
#[derive(thiserror::Error)]
pub enum BindError {
    #[error(\"{0} is a fixed key\")]
    Reserved(Chord),
}
#[serde(rename = \"PlayerKeys\")]
struct File;
fn f(x: Option<u8>) -> u8 {
    assert!(x.is_some(), \"No value here\");
    debug_assert_eq!(a, b, \"They differ\");
    let y = x.expect(\"There is one\");
    if y > 9 { panic!(\"Too big: {y}\"); }
    if y > 8 { unreachable!(\"Never eight\"); }
    eprintln!(\"Loaded {y}\");
    #[allow(clippy::x, reason = \"A reason\")]
    let z = format!(\"Shown {y}\");
    y
}
";
        assert_eq!(found(source), ["16: Shown {y}"]);
    }

    #[test]
    fn a_line_of_the_log_is_never_shown() {
        assert!(QUIET.contains(&"log"));
        let source = "fn update(&mut self, ctx: &mut Ctx) {
    ctx.log(LogLevel::Warn, LogCategory::Game, \"Lost the save\");
    self.log(LogLevel::Info, LogCategory::Game, format!(\"Moved to {x}\"));
    let shown = format!(\"Shown {x}\");
}
";
        assert_eq!(found(source), ["4: Shown {x}"]);
    }

    #[test]
    fn what_follows_a_literal_tells_a_name() {
        for code in [
            "    \"foot\" => 1,",
            "    \"a\" | \"b\" => 2,",
            "[(\"count\", &n)]",
            "(\"count\" , &n)",
        ] {
            let open = code.find('"').unwrap();
            assert!(is_name(code, open), "{code}");
        }
        for code in [
            "(key, \"info\")",
            "(\"info\", key)",
            "let a = \"x\";",
            "f(\"x\")",
            "\"never closed",
            "\"last\"",
        ] {
            let open = code.find('"').unwrap();
            assert!(!is_name(code, open), "{code}");
        }
        // The second literal of a line is looked at from its own quote.
        let code = "(\"count\", &n, \"word\")";
        assert!(!is_name(code, code.rfind("\"word").unwrap()));
        assert!(is_bare_word("acted"));
        for text in ["", "Acted", "two words", "menu_tick", "a1", "x.y"] {
            assert!(!is_bare_word(text), "{text}");
        }
    }

    #[test]
    fn prose_constants_count_and_ids_do_not() {
        let source = "\
pub const TITLE: &str = \"Visions of Shuyi\";
const QUIT: &str = \"Quit\";
pub(crate) const NOTE: &'static str = \"no saves yet\";
static NAMES: [&str; 2] = [
    \"New Game\",
    \"load\",
];
const CUE: &str = \"menu_tick\";
pub const LAYOUT_KEY: &str = \"layout\";
const ROW: i32 = 9;
const SEPARATOR: &str = \" · \";
let local = \"Not A Const\";
";
        assert_eq!(
            found(source),
            [
                "1: Visions of Shuyi",
                "2: Quit",
                "3: no saves yet",
                "5: New Game",
                "12: Not A Const",
            ]
        );
    }

    #[test]
    fn a_literal_over_several_lines_counts_once_on_its_first_line() {
        let source = "\
fn f(buf: &mut GlyphBuffer) {
    buf.print(
        1,
        2,
        \"one \\
         two\",
        fg,
    );
    buf.print(1, 2, \"after\", fg, bg);
}
";
        assert_eq!(found(source), ["5: one two", "9: after"]);
    }

    #[test]
    fn tests_and_marked_items_are_skipped() {
        let source = "\
// check-text: not player text
const LOG: &str = \"Could not save\";
const SHOWN: &str = \"Could not load\";
fn f(buf: &mut GlyphBuffer) {
    buf.print(1, 2, \"dev only\", fg, bg); // check-text: not player text
}

#[cfg(test)]
mod tests {
    const WANT: &str = \"New Game\";

    #[test]
    fn t() {
        buf.print(1, 2, \"Hello\", fg, bg);
    }
}
";
        assert_eq!(found(source), ["3: Could not load"]);
    }

    #[test]
    fn words_are_letters_outside_placeholders() {
        for text in ["a", "{n} left", "é", "Turn {n}"] {
            assert!(has_word(text), "{text}");
        }
        for text in ["", " · ", "{}/{}", "{name}", "{a}{b} 12", "}{x}"] {
            assert!(!has_word(text), "{text}");
        }
        assert!(is_prose("New"));
        assert!(is_prose("no saves"));
        assert!(!is_prose("menu_tick"));
    }

    #[test]
    fn calls_are_told_apart_by_name() {
        for sink in [
            "print",
            "print_fg",
            "print_centred",
            "draw_tip",
            "map_menu::draw_dialog_at",
            "help_line",
            "with_suffix",
            "MenuItem::new",
            "widgets::MenuItem::disabled",
        ] {
            assert!(bracket_of(sink) == Bracket::Sink, "{sink}");
        }
        for key in [
            "text",
            "text_with",
            "Ctx::text",
            "MenuItem::keyed",
            "text::Label::key",
        ] {
            assert!(bracket_of(key) == Bracket::Key, "{key}");
        }
        for quiet in [
            "panic",
            "unreachable",
            "todo",
            "unimplemented",
            "expect",
            "expect_err",
            "println",
            "eprintln",
            "dbg",
            "allow",
            "warn",
            "deny",
            "cfg",
            "cfg_attr",
            "assert",
            "assert_eq",
            "debug_assert",
            "debug_assert_ne",
            "std::panic",
        ] {
            assert!(bracket_of(quiet) == Bracket::Quiet, "{quiet}");
        }
        for call in ["new", "Menu::new", "format", "context", "reprint", "Some"] {
            assert!(bracket_of(call) == Bracket::Call, "{call}");
        }
        assert!(bracket_of("") == Bracket::Other);
        assert!(Bracket::Sink.decides() && Bracket::Key.decides() && Bracket::Quiet.decides());
        assert!(!Bracket::Call.decides() && !Bracket::Other.decides());
    }

    #[test]
    fn a_skipped_file_is_that_file_and_a_skipped_folder_all_below_it() {
        let rules = rules();
        for skipped in [
            "crates/ui/src/debug.rs",
            "crates/ui/src/debug/menu.rs",
            "crates/ui/src/snapshot.rs",
            "crates/ui/src/harness.rs",
            "crates/ui/src/screens/tests.rs",
        ] {
            assert!(is_skipped(&rules, skipped), "{skipped}");
        }
        for scanned in [
            "crates/ui/src/debug.rs.rs",
            "crates/ui/src/debugger.rs",
            "crates/ui/src/snapshot.rs/view.rs",
            "crates/ui/src/screens/title.rs",
        ] {
            assert!(!is_skipped(&rules, scanned), "{scanned}");
        }
    }

    #[test]
    fn the_report_lists_the_literals_by_file() {
        let hit = |file: &str, line, text: &str| Hit {
            file: file.to_owned(),
            line,
            text: text.to_owned(),
        };
        let hits = [
            hit("a.rs", 3, "One"),
            hit("a.rs", 9, "Two"),
            hit("b.rs", 1, "x y"),
        ];
        assert_eq!(
            report("crates/ui/src", &hits, 5),
            "check-text: 3 player-facing string literal(s) in crates/ui/src (at most 5 allowed)\n\
             a.rs: 2\n  3: \"One\"\n  9: \"Two\"\nb.rs: 1\n  1: \"x y\"\n"
        );
    }

    #[test]
    fn one_literal_too_many_fails() {
        let before = "fn f(b: &mut GlyphBuffer) {\n    b.print(0, 0, \"Old\", fg, bg);\n}\n";
        let max = scan(SCREEN, before).len();
        assert_eq!(max, 1);
        assert_eq!(verdict(HINT, max, max), Ok(None));
        let after = format!("{before}const NEW: &str = \"New Game\";\n");
        let count = scan(SCREEN, &after).len();
        assert_eq!(count, 2);
        let error = verdict(HINT, count, max).unwrap_err();
        assert!(error.contains("2 literals, over the 1 allowed"), "{error}");
        assert!(error.contains(HINT), "{error}");
        // Fewer than allowed passes, with a note to lower the number.
        let note = verdict(HINT, 0, max).unwrap().unwrap();
        assert!(note.contains("lower `check_text.max_literals`"), "{note}");
        assert!(note.contains("to 0"), "{note}");
    }

    #[test]
    fn run_scans_ui_sources_but_not_tests_or_debug_screens() {
        let root = std::env::temp_dir().join(format!("shuyi-check-text-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let src = root.join("crates/ui/src");
        fs::create_dir_all(src.join("screens")).unwrap();
        fs::create_dir_all(src.join("debug")).unwrap();
        let literal = "const A: &str = \"Some Text\";\n";
        for file in [
            "screens/foo.rs",
            "screens/tests.rs",
            "debug.rs",
            "debug/menu.rs",
            "harness.rs",
            "snapshot.rs",
            "screens/notes.txt",
        ] {
            fs::write(src.join(file), literal).unwrap();
        }
        let (hits, errors) = run(&rules(), &root);
        let _ = fs::remove_dir_all(&root);
        assert_eq!(errors, Vec::<String>::new());
        assert_eq!(hits.len(), 1, "{hits:?}");
        assert_eq!(hits[0].file, "crates/ui/src/screens/foo.rs");
        // A missing tree is reported, not passed over.
        let (hits, errors) = run(&rules(), &root);
        assert!(hits.is_empty(), "{hits:?}");
        assert_eq!(errors.len(), 1, "{errors:?}");
    }
}
