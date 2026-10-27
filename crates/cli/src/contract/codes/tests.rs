//! The codes' gate: every code well formed and once, every page a row's,
//! every row used.

use std::fs;
use std::path::Path;

use super::*;

/// The files that declare the codes, where a constant's name is no use
/// of it.
const DECLARING: [&str; 2] = [
    "crates/assets/src/error.rs",
    "crates/cli/src/contract/codes.rs",
];

/// `text` without its tests: none if `path` is a `tests.rs`, else the
/// text before `#[cfg(test)]`.
fn without_tests<'a>(path: &str, text: &'a str) -> &'a str {
    if path.ends_with("tests.rs") {
        return "";
    }
    text.find("#[cfg(test)]").map_or(text, |at| &text[..at])
}

/// Whether `text` writes the constant `codes::<name>` (and not a longer
/// name that starts with it).
fn uses(text: &str, name: &str) -> bool {
    let needle = format!("codes::{name}");
    text.match_indices(&needle).any(|(at, _)| {
        !text[at + needle.len()..].starts_with(|c: char| c.is_ascii_alphanumeric() || c == '_')
    })
}

/// What is wrong with `table`, the files of `crates/cli/errors/`
/// (`pages`: each name and text) and the engine's sources (`sources`:
/// each path from the repository's root, `/`-separated, and text).
fn gate(table: &[Entry], pages: &[(String, String)], sources: &[(String, String)]) -> Vec<String> {
    let mut wrong = Vec::new();
    for (i, entry) in table.iter().enumerate() {
        let code = entry.code.0;
        let digits = code.strip_prefix("SH").unwrap_or_default();
        if digits.len() != 4 || !digits.bytes().all(|b| b.is_ascii_digit()) {
            wrong.push(format!("{code}: not SH and four digits"));
        }
        if Class::of(code) != Some(entry.class) {
            wrong.push(format!(
                "{code}: its first digit isn't its class's ({})",
                entry.class.name()
            ));
        }
        if table[..i].iter().any(|e| e.code == entry.code) {
            wrong.push(format!("{code}: in the table twice"));
        }
        if table[..i].iter().any(|e| e.name == entry.name) {
            wrong.push(format!("{}: a name in the table twice", entry.name));
        }
        if !entry.page.starts_with(&format!("# {code}: ")) {
            wrong.push(format!("{code}: its page's first line doesn't name it"));
        }
        let constant = entry.name.to_uppercase().replace('-', "_");
        let used = sources.iter().any(|(path, text)| {
            !DECLARING.contains(&path.as_str()) && uses(without_tests(path, text), &constant)
        });
        if !entry.retired && !used {
            wrong.push(format!("{code}: no source uses codes::{constant}"));
        }
    }
    for (name, text) in pages {
        let code = name.strip_suffix(".md").unwrap_or(name);
        match table.iter().find(|e| e.code.0 == code) {
            Some(entry) if entry.page == text => {}
            Some(_) => wrong.push(format!("{name}: not its row's page")),
            None => wrong.push(format!("{name}: a page with no row")),
        }
    }
    wrong
}

/// The repository's root.
fn root() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
}

/// Every file under `dir`, at any depth, with `extension`: its path from
/// the repository's root and its text.
fn files_under(dir: &Path, extension: &str, out: &mut Vec<(String, String)>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            files_under(&path, extension, out);
        } else if path.extension().is_some_and(|e| e == extension) {
            let shown = path
                .strip_prefix(root())
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            out.push((shown, fs::read_to_string(&path).unwrap_or_default()));
        }
    }
}

#[test]
fn every_code_is_well_formed_has_its_page_and_is_used() {
    let mut pages = Vec::new();
    files_under(&root().join("crates/cli/errors"), "md", &mut pages);
    let pages: Vec<(String, String)> = pages
        .into_iter()
        .map(|(path, text)| (path.rsplit('/').next().unwrap_or("").to_owned(), text))
        .collect();
    assert_eq!(pages.len(), TABLE.len());
    let mut sources = Vec::new();
    for krate in fs::read_dir(root().join("crates")).unwrap().flatten() {
        files_under(&krate.path().join("src"), "rs", &mut sources);
    }
    assert!(sources.len() > 50, "{}", sources.len());
    assert_eq!(gate(TABLE, &pages, &sources), Vec::<String>::new());
}

/// A row of the fake tables.
const fn fake(code: &'static str, name: &'static str, class: Class) -> Entry {
    Entry {
        code: Code(code),
        name,
        class,
        title: "",
        help: "",
        page: "",
        retired: false,
    }
}

#[test]
fn the_gate_finds_a_code_twice_a_page_with_no_row_and_a_row_nothing_uses() {
    let page = |code: &str| (format!("{code}.md"), format!("# {code}: x\n"));
    let mut rows = [
        fake("SH0001", "wrong-arguments", Class::Usage),
        fake("SH2003", "missing-file", Class::Content),
    ];
    rows[0].page = "# SH0001: x\n";
    rows[1].page = "# SH2003: x\n";
    let source = |path: &str, text: &str| (path.to_owned(), text.to_owned());
    let sources = [
        source("crates/a/src/lib.rs", "f(codes::WRONG_ARGUMENTS);"),
        source("crates/b/src/lib.rs", "g(codes::MISSING_FILE)"),
    ];
    let pages = [page("SH0001"), page("SH2003")];
    assert_eq!(gate(&rows, &pages, &sources), Vec::<String>::new());
    // A page with no row.
    let extra = [page("SH0001"), page("SH2003"), page("SH2099")];
    assert_eq!(
        gate(&rows, &extra, &sources),
        ["SH2099.md: a page with no row"]
    );
    // A code twice, a name twice.
    let mut twice = rows;
    twice[1].code = Code("SH0001");
    twice[1].class = Class::Usage;
    twice[1].name = "wrong-arguments";
    twice[1].page = twice[0].page;
    let found = gate(&twice, &pages[..1], &sources[..1]);
    assert_eq!(
        found,
        [
            "SH0001: in the table twice",
            "wrong-arguments: a name in the table twice"
        ]
    );
    // A row nothing uses: in a test, in its declaring file, or as the
    // start of a longer name.
    let unused = [
        source("crates/a/src/lib.rs", "f(codes::WRONG_ARGUMENTS);"),
        source("crates/b/src/tests.rs", "codes::MISSING_FILE"),
        source(
            "crates/b/src/lib.rs",
            "codes::MISSING_FILES\n#[cfg(test)]\ncodes::MISSING_FILE",
        ),
        source("crates/assets/src/error.rs", "codes::MISSING_FILE"),
    ];
    assert_eq!(
        gate(&rows, &pages, &unused),
        ["SH2003: no source uses codes::MISSING_FILE"]
    );
    // A retired row needs no use.
    let mut retired = rows;
    retired[1].retired = true;
    assert_eq!(gate(&retired, &pages, &unused), Vec::<String>::new());
    // A malformed code, a class that isn't its digit's, a page that
    // doesn't name it.
    let mut bad = rows;
    bad[0].code = Code("SH01");
    bad[1].class = Class::Setup;
    bad[1].page = "# SH2004: x\n";
    let found = gate(&bad, &[page("SH2003")], &sources);
    assert_eq!(
        found,
        [
            "SH01: not SH and four digits",
            "SH01: its page's first line doesn't name it",
            "SH2003: its first digit isn't its class's (setup)",
            "SH2003: its page's first line doesn't name it",
            "SH2003.md: not its row's page",
        ]
    );
}

#[test]
fn a_class_sets_the_exit_code() {
    assert_eq!(Class::Usage.exit_code(), 2);
    assert_eq!(Class::Setup.exit_code(), 3);
    assert_eq!(Class::Content.exit_code(), 1);
    assert_eq!(Class::Operation.exit_code(), 1);
    assert_eq!(Class::of("SH1002"), Some(Class::Setup));
    assert_eq!(Class::of("SH9999"), None);
    assert_eq!(Class::of("E0277"), None);
    assert_eq!(exit_code(Some("SH0001")), 2);
    assert_eq!(exit_code(Some("SH1001")), 3);
    assert_eq!(exit_code(Some("SH3004")), 1);
    assert_eq!(exit_code(None), 1);
    for class in Class::ALL {
        assert_eq!(Class::of(&format!("SH{}000", class.digit())), Some(class));
    }
}

#[test]
fn a_row_is_found_by_its_code_with_its_help() {
    let row = entry("SH2003").unwrap();
    assert_eq!(row.name, "missing-file");
    assert_eq!(row.class, Class::Content);
    assert!(row.page.starts_with("# SH2003: A file isn't there\n"));
    assert_eq!(help_of("SH2003"), Some(row.help));
    assert_eq!(entry("SH9999"), None);
}

#[test]
fn an_error_for_people_says_its_code_help_and_page() {
    assert_eq!(
        said("check", "SH1002", "no game", Some("fix it\nthen this")),
        "check: error[SH1002]: no game\n  help: fix it\n        then this\n  see: shuyi \
         explain SH1002\n"
    );
    assert_eq!(
        said("x", "SH0001", "bad", None),
        "x: error[SH0001]: bad\n  see: shuyi explain SH0001\n"
    );
}

#[test]
fn problems_are_followed_by_one_help_line_per_code() {
    let problem = |code: Option<Code>| {
        let mut error = shuyi_assets::ContentError::new("a.ron", "bad");
        error.code = code;
        Problem::from(&error)
    };
    let problems = [
        problem(Some(content::PARSE)),
        problem(None),
        problem(Some(content::MISSING_FILE)),
        problem(Some(content::PARSE)),
    ];
    assert_eq!(
        help_lines(&problems),
        format!(
            "help[SH2001]: {}\nhelp[SH2003]: {}\n",
            help_of("SH2001").unwrap(),
            help_of("SH2003").unwrap()
        )
    );
    assert_eq!(help_lines(&problems[1..2]), "");
}

/// Whether `line` is one sentence run: no two spaces in a row and no
/// line break (a lost `\` continuation leaves a run of spaces).
fn is_plain(line: &str) -> bool {
    !line.contains("  ") && !line.contains('\n')
}

#[test]
fn no_help_line_or_page_title_holds_a_run_of_spaces() {
    for entry in TABLE {
        let title = entry.page.lines().next().unwrap_or_default();
        assert!(
            is_plain(entry.title),
            "{}: title: {}",
            entry.code,
            entry.title
        );
        assert!(is_plain(entry.help), "{}: help: {}", entry.code, entry.help);
        assert!(is_plain(title), "{}: page title: {title}", entry.code);
    }
}
