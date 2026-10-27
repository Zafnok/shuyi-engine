//! `cargo xtask check-links`: every relative link in the repository's
//! Markdown points at a file or folder that exists. Links to the web are not
//! fetched (no network); a link's `#anchor` is not checked.

use std::fs;
use std::path::{Component, Path, PathBuf};

use pulldown_cmark::{Event, Parser, Tag};

/// Folders never searched for Markdown, by name.
pub(crate) const SKIPPED_DIRS: &[&str] = &[".git", "target", "node_modules", "dist"];
/// Folders never searched for Markdown, by path from the root: the app's
/// worktrees are other checkouts of this repository.
const SKIPPED_PATHS: &[&str] = &[".claude/worktrees"];

/// Checks every `*.md` file under `repo_root` and returns every broken link
/// (empty means clean).
pub fn run(repo_root: &Path) -> Vec<String> {
    let mut files = Vec::new();
    let mut errors = Vec::new();
    if let Err(e) = markdown_files(repo_root, repo_root, &mut files) {
        errors.push(format!("{}: {e}", repo_root.display()));
    }
    files.sort();
    for file in files {
        let display = display_path(repo_root, &file);
        match fs::read_to_string(&file) {
            Ok(text) => {
                for (line, dest) in relative_links(&text) {
                    if !link_exists(repo_root, &file, &dest) {
                        errors.push(format!("{display}:{line}: broken link `{dest}`"));
                    }
                }
            }
            Err(e) => errors.push(format!("{display}: {e}")),
        }
    }
    errors
}

pub(crate) fn display_path(root: &Path, file: &Path) -> String {
    file.strip_prefix(root)
        .unwrap_or(file)
        .to_string_lossy()
        .replace('\\', "/")
}

/// Whether the folder `dir` is one never searched: by its name
/// ([`SKIPPED_DIRS`]) or its path from `root` ([`SKIPPED_PATHS`]).
pub(crate) fn is_skipped_dir(root: &Path, dir: &Path) -> bool {
    dir.file_name()
        .is_some_and(|name| SKIPPED_DIRS.iter().any(|d| name == *d))
        || SKIPPED_PATHS.iter().any(|p| display_path(root, dir) == *p)
}

pub(crate) fn markdown_files(
    root: &Path,
    dir: &Path,
    out: &mut Vec<PathBuf>,
) -> std::io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            if !is_skipped_dir(root, &path) {
                markdown_files(root, &path, out)?;
            }
        } else if path.extension().is_some_and(|ext| ext == "md") {
            out.push(path);
        }
    }
    Ok(())
}

/// The relative links and images of a Markdown text, with the line each one
/// starts on. Links in code are not links; web links, `mailto:`, links to
/// an anchor in the same file and the website's links to its own pages
/// (`@/…`, which `zola check` reads, ADR-0048) are left out.
pub(crate) fn relative_links(text: &str) -> Vec<(usize, String)> {
    let mut links = Vec::new();
    for (event, range) in Parser::new(text).into_offset_iter() {
        let Event::Start(Tag::Link { dest_url: dest, .. } | Tag::Image { dest_url: dest, .. }) =
            event
        else {
            continue;
        };
        if is_relative(&dest) {
            let line = text[..range.start].matches('\n').count() + 1;
            links.push((line, dest.to_string()));
        }
    }
    links
}

fn is_relative(dest: &str) -> bool {
    !(dest.is_empty()
        || dest.starts_with('#')
        || dest.starts_with("@/")
        || dest.contains("://")
        || dest.starts_with("mailto:"))
}

/// Whether `dest`, a link in `from`, names an existing file or folder. A
/// link starting with `/` is from the repository's root. The match is exact,
/// letter case included, as on GitHub.
fn link_exists(repo_root: &Path, from: &Path, dest: &str) -> bool {
    let path = dest.split(['#', '?']).next().unwrap_or(dest);
    let path = percent_decode(path);
    let (base, rel) = match path.strip_prefix('/') {
        Some(rel) => (repo_root.to_path_buf(), rel.to_string()),
        None => (
            from.parent().unwrap_or(repo_root).to_path_buf(),
            path.clone(),
        ),
    };
    let mut current = base;
    for component in Path::new(&rel).components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if !current.pop() {
                    return false;
                }
            }
            Component::Normal(name) => {
                let Ok(entries) = fs::read_dir(&current) else {
                    return false;
                };
                if !entries
                    .filter_map(Result::ok)
                    .any(|e| e.file_name() == name)
                {
                    return false;
                }
                current.push(name);
            }
            Component::RootDir | Component::Prefix(_) => return false,
        }
    }
    true
}

/// `%20` and the like, as GitHub reads them in a link; anything else is kept.
pub(crate) fn percent_decode(s: &str) -> String {
    let mut out = Vec::with_capacity(s.len());
    // Every step takes at least one byte off `rest`, so the loop ends
    // whatever a mutant does to it.
    let mut rest = s.as_bytes();
    while let Some((&first, tail)) = rest.split_first() {
        let hex = tail
            .get(..2)
            .and_then(|h| std::str::from_utf8(h).ok())
            .and_then(|h| u8::from_str_radix(h, 16).ok());
        if let (b'%', Some(byte)) = (first, hex) {
            out.push(byte);
            rest = &tail[2..];
        } else {
            out.push(first);
            rest = tail;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_repo(unique: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "xtask-check-links-test-{unique}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("docs/adr")).unwrap();
        fs::write(root.join("README.md"), "# Readme\n").unwrap();
        fs::write(root.join("docs/adr/0001-a b.md"), "# A\n").unwrap();
        root
    }

    #[test]
    fn finds_links_and_images_with_their_lines() {
        let text =
            "# T\n\nSee [a](a.md) and\n![pic](img/p.png).\n\n[ref]: docs/x.md\n\nUse [r][ref].\n";
        assert_eq!(
            relative_links(text),
            vec![
                (3, "a.md".to_string()),
                (4, "img/p.png".to_string()),
                (8, "docs/x.md".to_string()),
            ]
        );
    }

    #[test]
    fn skips_web_links_mail_anchors_and_code() {
        let text = "[w](https://example.com) [m](mailto:a@b.c) [h](#top) <https://x.y>\n\n`[c](nope.md)`\n\n```\n[c](nope.md)\n```\n";
        assert!(
            relative_links(text).is_empty(),
            "{:?}",
            relative_links(text)
        );
    }

    #[test]
    fn leaves_zolas_internal_links_to_zola() {
        let text =
            "[News](@/news/_index.md) and [a post](@/news/post.md#top), but [this](at/x.md).\n";
        assert_eq!(relative_links(text), vec![(1, "at/x.md".to_string())]);
    }

    #[test]
    fn resolves_relative_rooted_and_parent_links() {
        let root = temp_repo("resolve");
        let from = root.join("docs/adr/0001-a b.md");
        assert!(link_exists(&root, &from, "../../README.md"));
        assert!(link_exists(&root, &from, "/README.md"));
        assert!(link_exists(&root, &from, "./0001-a%20b.md#context"));
        assert!(link_exists(&root, &from, ".."));
        assert!(!link_exists(&root, &from, "../../MISSING.md"));
        assert!(!link_exists(&root, &from, "0002-b.md"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn letter_case_must_match() {
        let root = temp_repo("case");
        let from = root.join("docs/x.md");
        assert!(link_exists(&root, &from, "../README.md"));
        assert!(!link_exists(&root, &from, "../readme.md"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn decodes_percent_escapes() {
        assert_eq!(percent_decode("a%20b%2Fc"), "a b/c");
        assert_eq!(percent_decode("100%"), "100%");
        assert_eq!(percent_decode("%zz"), "%zz");
    }

    #[test]
    fn run_reports_broken_links_and_skips_worktrees_and_target() {
        let root = temp_repo("run");
        fs::write(
            root.join("docs/guide.md"),
            "# G\n\n[ok](../README.md)\n[bad](nope.md)\n",
        )
        .unwrap();
        for skipped in [".claude/worktrees/w", "target/doc"] {
            fs::create_dir_all(root.join(skipped)).unwrap();
            fs::write(root.join(skipped).join("x.md"), "[bad](nope.md)\n").unwrap();
        }
        let errors = run(&root);
        assert_eq!(
            errors,
            vec!["docs/guide.md:4: broken link `nope.md`".to_string()]
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn real_repo_links_pass() {
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("xtask is at <repo>/crates/xtask");
        let errors = run(repo_root);
        assert!(errors.is_empty(), "broken links: {errors:#?}");
    }
}
