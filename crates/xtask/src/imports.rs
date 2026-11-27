//! `cargo xtask check-imports` (ticket 3438, ADR-0161): the code a game is
//! made from or learns from (the template, the examples, the book's
//! snippets and the skill pack's worked examples) names an engine item only
//! through a prelude, or through a deeper path `tools/imports.ron` allows,
//! each with its reason and the files it is allowed in.
//!
//! An agent writes a game from these files and from memory: a path it
//! guesses is a prelude's, and every deeper path it would copy is one the
//! list names and says why. The check reads each `.rs` file's `use` trees
//! and the paths written in its code (`shuyi::app::run_app(…)`), with the
//! lexer `shuyi check-text` reads source with, so a path in a comment or a
//! string is not read. An engine crate is one under `crates/`, by its
//! package's name.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use serde::Deserialize;
use shuyi_cli::rust_lines;

/// The list, from the repository's root.
pub const IMPORTS: &str = "tools/imports.ron";
/// The folders read, from the repository's root.
pub const ROOTS: &[&str] = &["templates", "examples", "site/snippets", "skills"];

/// `tools/imports.ron`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rules {
    /// The preludes, as paths: a path under one passes.
    pub preludes: Vec<String>,
    /// The deeper paths allowed, each with its reason.
    pub allowed: Vec<Allowed>,
}

/// One deeper path allowed.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Allowed {
    /// The path: it and every path under it pass.
    pub path: String,
    /// The files it is allowed in, each a file or a folder from the
    /// repository's root; every file read, when empty.
    #[serde(default, rename = "in")]
    pub files: Vec<String>,
    /// Why no prelude holds it.
    pub why: String,
}

/// One engine path a file names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Named {
    /// Its line, from 1.
    pub line: usize,
    /// The path, its segments joined by `::` (a glob ends in `*`).
    pub path: String,
}

/// Checks the repository at `root` and returns every error (empty means
/// clean).
pub fn run(root: &Path) -> Vec<String> {
    let rules = match fs::read_to_string(root.join(IMPORTS)) {
        Ok(text) => match ron::from_str::<Rules>(&text) {
            Ok(rules) => rules,
            Err(e) => return vec![format!("{IMPORTS}: {e}")],
        },
        Err(e) => return vec![format!("{IMPORTS}: {e}")],
    };
    let crates = engine_crates(root);
    let mut sources = Vec::new();
    for dir in ROOTS {
        rust_files(root, &root.join(dir), &mut sources);
    }
    let mut files = Vec::new();
    let mut errors = Vec::new();
    for rel in sources {
        match fs::read_to_string(root.join(&rel)) {
            Ok(text) => files.push((rel, text)),
            Err(e) => errors.push(format!("{rel}: {e}")),
        }
    }
    errors.extend(check(&rules, &crates, &files, |rel| {
        root.join(rel).exists()
    }));
    errors
}

/// The errors of `files` (each its path from the root and its text) under
/// `rules`, with `crates` the engine's crates by the name code writes
/// (`shuyi_core`), and `exists` whether a path of an allowed entry's `in`
/// is there.
pub fn check(
    rules: &Rules,
    crates: &BTreeSet<String>,
    files: &[(String, String)],
    exists: impl Fn(&str) -> bool,
) -> Vec<String> {
    let mut errors = Vec::new();
    for (i, allowed) in rules.allowed.iter().enumerate() {
        if allowed.why.trim().is_empty() {
            errors.push(format!("{IMPORTS}: `{}` says no why", allowed.path));
        }
        if rules.allowed[..i].iter().any(|a| a.path == allowed.path) {
            errors.push(format!("{IMPORTS}: `{}` is listed twice", allowed.path));
        }
        for file in &allowed.files {
            if !exists(file) {
                errors.push(format!(
                    "{IMPORTS}: `{}` is allowed in {file}, which is not there",
                    allowed.path
                ));
            }
        }
    }
    let mut used = vec![false; rules.allowed.len()];
    for (rel, text) in files {
        for named in named_paths(text, crates) {
            if under_any(&named.path, &rules.preludes) {
                continue;
            }
            let found = rules.allowed.iter().position(|a| {
                under(&named.path, &a.path)
                    && (a.files.is_empty() || a.files.iter().any(|f| in_folder(rel, f)))
            });
            match found {
                Some(at) => used[at] = true,
                None => errors.push(format!(
                    "{rel}:{}: `{}` is past the preludes: name it through \
                     `shuyi::prelude` (or its `grid`, `text`, `audio` or `test`), \
                     `shuyi::core::prelude` or `shuyi_cli::prelude`, or allow it in \
                     {IMPORTS} with why",
                    named.line, named.path
                )),
            }
        }
    }
    for (allowed, used) in rules.allowed.iter().zip(used) {
        if !used {
            errors.push(format!(
                "{IMPORTS}: `{}` allows nothing any file names: take it out",
                allowed.path
            ));
        }
    }
    errors
}

/// Whether `path` is `prefix` or under it, segment by segment.
fn under(path: &str, prefix: &str) -> bool {
    path == prefix
        || path
            .strip_prefix(prefix)
            .is_some_and(|rest| rest.starts_with("::"))
}

fn under_any(path: &str, prefixes: &[String]) -> bool {
    prefixes.iter().any(|p| under(path, p))
}

/// Whether the file `rel` is `folder` or in it.
fn in_folder(rel: &str, folder: &str) -> bool {
    let folder = folder.trim_end_matches('/');
    rel == folder
        || rel
            .strip_prefix(folder)
            .is_some_and(|rest| rest.starts_with('/'))
}

/// The engine's crates by the name code writes: each package under
/// `crates/` whose name is `shuyi` or starts `shuyi-`, with `_` for `-`.
fn engine_crates(root: &Path) -> BTreeSet<String> {
    let mut crates = BTreeSet::new();
    let Ok(dirs) = fs::read_dir(root.join("crates")) else {
        return crates;
    };
    for dir in dirs.flatten() {
        let Ok(text) = fs::read_to_string(dir.path().join("Cargo.toml")) else {
            continue;
        };
        let Ok(manifest) = toml::from_str::<toml::Table>(&text) else {
            continue;
        };
        let name = manifest
            .get("package")
            .and_then(|p| p.get("name"))
            .and_then(toml::Value::as_str);
        if let Some(name) = name.filter(|n| *n == "shuyi" || n.starts_with("shuyi-")) {
            crates.insert(name.replace('-', "_"));
        }
    }
    crates
}

/// Every `.rs` file under `dir`, as a path from `root` with `/`, sorted; a
/// `target` folder is left out.
fn rust_files(root: &Path, dir: &Path, out: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<_> = entries.flatten().map(|e| e.path()).collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            if path.file_name().is_some_and(|n| n != "target") {
                rust_files(root, &path, out);
            }
        } else if path.extension().is_some_and(|e| e == "rs")
            && let Ok(rel) = path.strip_prefix(root)
        {
            out.push(rel.to_string_lossy().replace('\\', "/"));
        }
    }
}

/// A token of code.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Tok {
    Ident(String),
    /// `::`.
    Sep,
    Punct(char),
}

/// The tokens of `text`'s code, each with its line from 1: comments and
/// the contents of strings are not code.
fn tokens(text: &str) -> Vec<(Tok, usize)> {
    let mut out = Vec::new();
    for (n, line) in rust_lines::lex(text).iter().enumerate() {
        let chars: Vec<char> = line.code.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            let c = chars[i];
            if rust_lines::is_ident(c) {
                let start = i;
                while i < chars.len() && rust_lines::is_ident(chars[i]) {
                    i += 1;
                }
                out.push((Tok::Ident(chars[start..i].iter().collect()), n + 1));
                continue;
            }
            if c == ':' && chars.get(i + 1) == Some(&':') {
                out.push((Tok::Sep, n + 1));
                i += 2;
                continue;
            }
            if !c.is_whitespace() {
                out.push((Tok::Punct(c), n + 1));
            }
            i += 1;
        }
    }
    out
}

/// The engine paths `text` names: each leaf of its `use` trees, and each
/// path in its code that starts with an engine crate.
pub fn named_paths(text: &str, crates: &BTreeSet<String>) -> Vec<Named> {
    let toks = tokens(text);
    let mut found = Vec::new();
    let mut i = 0;
    while i < toks.len() {
        match &toks[i].0 {
            Tok::Ident(word) if word == "use" => {
                i = use_tree(&toks, i + 1, &[], &mut found);
            }
            Tok::Ident(word)
                if crates.contains(word)
                    && matches!(toks.get(i + 1), Some((Tok::Sep, _)))
                    && !after_path(&toks, i) =>
            {
                let line = toks[i].1;
                let mut segments = vec![word.clone()];
                i += 1;
                while let (Some((Tok::Sep, _)), Some((Tok::Ident(next), _))) =
                    (toks.get(i), toks.get(i + 1))
                {
                    segments.push(next.clone());
                    i += 2;
                }
                found.push(Named {
                    line,
                    path: segments.join("::"),
                });
            }
            _ => i += 1,
        }
    }
    found.retain(|n| {
        let mut segments = n.path.split("::");
        segments.next().is_some_and(|first| crates.contains(first)) && segments.next().is_some()
    });
    found
}

/// Whether the token at `i` follows `<ident>::`, so it is not a path's
/// first segment.
fn after_path(toks: &[(Tok, usize)], i: usize) -> bool {
    i >= 2 && toks[i - 1].0 == Tok::Sep && matches!(toks[i - 2].0, Tok::Ident(_))
}

/// Reads the use tree starting at `i` under `prefix`, pushing each leaf,
/// and returns the index past it (at the `,`, `}` or `;` that ends it).
fn use_tree(toks: &[(Tok, usize)], mut i: usize, prefix: &[String], out: &mut Vec<Named>) -> usize {
    let mut path = prefix.to_vec();
    loop {
        match toks.get(i) {
            Some((Tok::Sep, _)) => i += 1,
            Some((Tok::Ident(word), line)) => {
                if word != "self" {
                    path.push(word.clone());
                }
                i += 1;
                if !matches!(toks.get(i), Some((Tok::Sep, _))) {
                    out.push(Named {
                        line: *line,
                        path: path.join("::"),
                    });
                    // `as name`.
                    if matches!(toks.get(i), Some((Tok::Ident(w), _)) if w == "as") {
                        i += 2;
                    }
                    return i;
                }
            }
            Some((Tok::Punct('*'), line)) => {
                path.push("*".to_owned());
                out.push(Named {
                    line: *line,
                    path: path.join("::"),
                });
                return i + 1;
            }
            Some((Tok::Punct('{'), _)) => {
                i += 1;
                loop {
                    match toks.get(i) {
                        Some((Tok::Punct('}'), _)) => return i + 1,
                        Some((Tok::Punct(','), _)) => i += 1,
                        None => return i,
                        Some(_) => {
                            let next = use_tree(toks, i, &path, out);
                            // A tree that read nothing would loop forever.
                            i = if next == i { i + 1 } else { next };
                        }
                    }
                }
            }
            _ => return i,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn crates() -> BTreeSet<String> {
        ["shuyi", "shuyi_core", "shuyi_cli"]
            .map(str::to_owned)
            .into()
    }

    fn paths(text: &str) -> Vec<(usize, String)> {
        named_paths(text, &crates())
            .into_iter()
            .map(|n| (n.line, n.path))
            .collect()
    }

    fn rules(allowed: &[(&str, &[&str])]) -> Rules {
        Rules {
            preludes: [
                "shuyi::prelude",
                "shuyi::core::prelude",
                "shuyi_cli::prelude",
            ]
            .map(str::to_owned)
            .into(),
            allowed: allowed
                .iter()
                .map(|(path, files)| Allowed {
                    path: (*path).to_owned(),
                    files: files.iter().map(|f| (*f).to_owned()).collect(),
                    why: "a reason".to_owned(),
                })
                .collect(),
        }
    }

    fn errors(rules: &Rules, files: &[(&str, &str)]) -> Vec<String> {
        let files: Vec<(String, String)> = files
            .iter()
            .map(|(p, t)| ((*p).to_owned(), (*t).to_owned()))
            .collect();
        check(rules, &crates(), &files, |_| true)
    }

    #[test]
    fn a_use_tree_is_read_to_its_leaves() {
        let text = "use shuyi::{prelude::*, ui::grid::{self, Camera as Cam}};\n\
                    use std::fmt;\nuse shuyi_core::prelude::Pos;";
        assert_eq!(
            paths(text),
            [
                (1, "shuyi::prelude::*".to_owned()),
                (1, "shuyi::ui::grid".to_owned()),
                (1, "shuyi::ui::grid::Camera".to_owned()),
                (3, "shuyi_core::prelude::Pos".to_owned()),
            ]
        );
    }

    #[test]
    fn a_path_in_code_is_read_and_one_in_a_comment_or_a_string_is_not() {
        let text = "fn main() {\n    shuyi::app::run_app(x::shuyi::y);\n    \
                    // shuyi::ui::Hidden\n    let s = \"shuyi::ui::Quoted\";\n    \
                    shuyi::files!(\"f\");\n}";
        assert_eq!(
            paths(text),
            [
                (2, "shuyi::app::run_app".to_owned()),
                (5, "shuyi::files".to_owned()),
            ]
        );
    }

    #[test]
    fn a_game_crate_named_like_the_engine_is_not_the_engine() {
        assert_eq!(paths("use shuyi_gates_rules::Board;\nuse shuyi;"), []);
    }

    #[test]
    fn a_deep_import_with_no_reason_fails_naming_its_file_and_line() {
        let text = "use shuyi::prelude::*;\n\nuse shuyi::ui::frame::glyph_buffer::Sprite;\n";
        let errors = errors(&rules(&[]), &[("examples/a/src/lib.rs", text)]);
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(
            errors[0].starts_with(
                "examples/a/src/lib.rs:3: `shuyi::ui::frame::glyph_buffer::Sprite` is past the preludes"
            ),
            "{errors:?}"
        );
    }

    #[test]
    fn a_prelude_path_passes_and_an_allowed_one_passes_only_where_it_is_allowed() {
        let text = "use shuyi::prelude::grid::Camera;\nuse shuyi_cli::prelude::*;\n\
                    use shuyi::ui::dialogue::TextBox;\n";
        let rules = rules(&[("shuyi::ui::dialogue", &["skills/shuyi-dialogue"])]);
        assert_eq!(
            errors(&rules, &[("skills/shuyi-dialogue/example/talk.rs", text)]),
            Vec::<String>::new()
        );
        let elsewhere = errors(&rules, &[("examples/a/src/lib.rs", text)]);
        assert_eq!(elsewhere.len(), 2, "{elsewhere:?}");
        assert!(
            elsewhere[0].starts_with("examples/a/src/lib.rs:3:"),
            "{elsewhere:?}"
        );
        assert!(elsewhere[1].contains("allows nothing"), "{elsewhere:?}");
    }

    #[test]
    fn a_path_is_under_a_prefix_only_at_a_segment() {
        assert!(under("shuyi::ui::dialogue::TextBox", "shuyi::ui::dialogue"));
        assert!(under("shuyi::ui::dialogue", "shuyi::ui::dialogue"));
        assert!(!under("shuyi::ui::dialogues", "shuyi::ui::dialogue"));
        assert!(in_folder("skills/a/x.rs", "skills/a/"));
        assert!(!in_folder("skills/ab/x.rs", "skills/a"));
    }

    #[test]
    fn the_list_says_why_names_each_path_once_and_names_files_that_are_there() {
        let mut rules = rules(&[("shuyi::app", &["gone.rs"]), ("shuyi::app", &[])]);
        rules.allowed[1].why = " ".to_owned();
        let files = [("a.rs".to_owned(), "use shuyi::app::Window;".to_owned())];
        let errors = check(&rules, &crates(), &files, |f| f != "gone.rs");
        assert!(
            errors.iter().any(|e| e.contains("says no why")),
            "{errors:?}"
        );
        assert!(
            errors.iter().any(|e| e.contains("listed twice")),
            "{errors:?}"
        );
        assert!(errors.iter().any(|e| e.contains("not there")), "{errors:?}");
    }

    #[test]
    fn the_repository_passes() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        assert_eq!(run(&root), Vec::<String>::new());
    }
}
