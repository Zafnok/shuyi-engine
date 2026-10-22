//! `cargo xtask check-docs`: every Markdown file in the repository has a home
//! in the docs map, `docs/README.md` (ticket 2810). A file is covered by a
//! relative link in the map to the file itself or to a folder that holds it.
//! Links that don't resolve are `check-links`'s to report.

use std::fs;
use std::path::{Component, Path};

use crate::links::{display_path, markdown_files, percent_decode, relative_links};

/// The map, from the repository's root.
const MAP: &str = "docs/README.md";
/// Files never checked, by name: a `CLAUDE.md` only points to `AGENTS.md`.
const UNCHECKED_NAMES: &[&str] = &["CLAUDE.md"];
/// Folders never checked, by path from the root: `tickets/` has its own row.
const UNCHECKED_DIRS: &[&str] = &["tickets"];

/// Checks every `*.md` file under `repo_root` against the map and returns
/// every file the map doesn't cover (empty means clean).
pub fn run(repo_root: &Path) -> Vec<String> {
    let map = repo_root.join(MAP);
    let text = match fs::read_to_string(&map) {
        Ok(text) => text,
        Err(e) => return vec![format!("{MAP}: {e} (the docs map; the document skill)")],
    };
    let covered: Vec<Vec<String>> = relative_links(&text)
        .into_iter()
        .filter_map(|(_, dest)| resolve(&dest))
        .collect();

    let mut files = Vec::new();
    let mut errors = Vec::new();
    if let Err(e) = markdown_files(repo_root, repo_root, &mut files) {
        errors.push(format!("{}: {e}", repo_root.display()));
    }
    files.sort();
    for file in files {
        let display = display_path(repo_root, &file);
        let parts: Vec<String> = display.split('/').map(str::to_string).collect();
        if !is_checked(&display, &parts) {
            continue;
        }
        if !covered.iter().any(|link| parts.starts_with(link)) {
            errors.push(format!(
                "{display}: not in the docs map; add a row to {MAP} (the document skill)"
            ));
        }
    }
    errors
}

fn is_checked(display: &str, parts: &[String]) -> bool {
    display != MAP
        && !parts
            .last()
            .is_some_and(|name| UNCHECKED_NAMES.contains(&name.as_str()))
        && !parts
            .first()
            .is_some_and(|dir| UNCHECKED_DIRS.contains(&dir.as_str()))
}

/// The path from the repository's root that `dest`, a link in the map,
/// names, one entry per component; `None` when it climbs above the root.
/// The path is worked out from the text only: whether it exists is
/// `check-links`'s concern.
fn resolve(dest: &str) -> Option<Vec<String>> {
    let path = dest.split(['#', '?']).next().unwrap_or(dest);
    let path = percent_decode(path);
    let (mut parts, rel) = match path.strip_prefix('/') {
        Some(rel) => (Vec::new(), rel.to_string()),
        None => (vec!["docs".to_string()], path),
    };
    for component in Path::new(&rel).components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                parts.pop()?;
            }
            Component::Normal(name) => parts.push(name.to_string_lossy().into_owned()),
            Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    Some(parts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// A temporary repository with `files` written in it, and the map's text.
    fn temp_repo(unique: &str, map: Option<&str>, files: &[&str]) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "xtask-check-docs-test-{unique}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("docs")).unwrap();
        if let Some(map) = map {
            fs::write(root.join(MAP), map).unwrap();
        }
        for file in files {
            let path = root.join(file);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, "# X\n").unwrap();
        }
        root
    }

    fn uncovered(file: &str) -> String {
        format!("{file}: not in the docs map; add a row to docs/README.md (the document skill)")
    }

    #[test]
    fn a_file_linked_directly_is_covered() {
        let root = temp_repo(
            "direct",
            Some("[f](formats.md) [r](../README.md) [a](/AGENTS.md#rules)\n"),
            &["docs/formats.md", "README.md", "AGENTS.md"],
        );
        assert_eq!(run(&root), Vec::<String>::new());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_link_to_a_folder_covers_what_it_holds() {
        let root = temp_repo(
            "folder",
            Some("[adr](adr/) [t](../templates)\n"),
            &["docs/adr/0001-a.md", "templates/grid/AGENTS.md"],
        );
        assert_eq!(run(&root), Vec::<String>::new());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn an_uncovered_file_is_named() {
        let root = temp_repo(
            "uncovered",
            Some("[f](formats.md) and `templates/*/AGENTS.md`\n"),
            &["docs/formats.md", "docs/x.md", "templates/grid/AGENTS.md"],
        );
        assert_eq!(
            run(&root),
            vec![
                uncovered("docs/x.md"),
                uncovered("templates/grid/AGENTS.md")
            ]
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_sibling_folder_with_the_same_start_does_not_cover() {
        let root = temp_repo("sibling", Some("[ad](ad)\n"), &["docs/adr/x.md"]);
        assert_eq!(run(&root), vec![uncovered("docs/adr/x.md")]);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn claude_md_tickets_and_skipped_folders_are_not_checked() {
        let root = temp_repo(
            "unchecked",
            Some("# Map\n"),
            &[
                "CLAUDE.md",
                "templates/grid/CLAUDE.md",
                "tickets/open/2001-x.md",
                "target/doc/x.md",
                ".claude/worktrees/w/x.md",
            ],
        );
        assert_eq!(run(&root), Vec::<String>::new());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_missing_map_is_an_error() {
        let root = temp_repo("missing", None, &[]);
        let errors = run(&root);
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(errors[0].starts_with("docs/README.md: "), "{errors:?}");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn resolves_links_from_the_map_folder() {
        assert_eq!(
            resolve("formats.md"),
            Some(vec!["docs".into(), "formats.md".into()])
        );
        assert_eq!(resolve("../README.md#x"), Some(vec!["README.md".into()]));
        assert_eq!(
            resolve("./adr/a%20b.md"),
            Some(vec!["docs".into(), "adr".into(), "a b.md".into()])
        );
        assert_eq!(
            resolve("/crates/cli"),
            Some(vec!["crates".into(), "cli".into()])
        );
        assert_eq!(resolve("../../x.md"), None);
    }

    #[test]
    fn real_repo_is_covered() {
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("xtask is at <repo>/crates/xtask");
        let errors = run(repo_root);
        assert!(errors.is_empty(), "uncovered: {errors:#?}");
    }
}
