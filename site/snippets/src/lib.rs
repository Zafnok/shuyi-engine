//! The book's code: Moon Garden, a small game on Shuyi Engine, with each
//! chapter's part in a module of its own. The book
//! (`site/content/learn/book/`) shows these lines through its `snippet`
//! shortcode, so CI builds and tests every one of them, and a change to
//! the engine that breaks them breaks the book's build.
//!
//! Each piece the book shows sits between `// ANCHOR: <name>` and
//! `// ANCHOR_END: <name>` lines, which mark whole items at the file's
//! own indentation: the shortcode shows the lines as they are written.
//! The `files/` RON files carry anchors too.
//!
//! - [`quick_start`]: the game on the short path, as `shuyi new` lays one
//!   out (chapter 1);
//! - [`screens`]: its screens and the screen stack (chapter 2);
//! - [`actions`]: actions, never keys (chapter 3);
//! - [`frame`]: views and the skin that paints them (chapter 4);
//! - [`assets`]: its own kind of file, read through the registry
//!   (chapter 5);
//! - [`rules`]: its rules as commands (chapter 6);
//! - [`saves`]: a save and the player's settings (chapter 7);
//! - [`testing`]: bots and replays over its rules (chapter 8);
//! - [`targets`]: its project file's targets and web build (chapter 9);
//! - [`tools`]: the tool contract on it (chapter 10).

pub mod actions;
pub mod assets;
pub mod frame;
pub mod quick_start;
pub mod rules;
pub mod saves;
pub mod screens;
pub mod targets;
pub mod testing;
pub mod tools;

pub use quick_start::{Data, game};

#[cfg(test)]
mod anchors {
    use std::collections::BTreeSet;
    use std::fs;
    use std::path::{Path, PathBuf};

    /// Every file under `dir` whose name ends in one of `extensions`.
    fn files(dir: &Path, extensions: &[&str], out: &mut Vec<PathBuf>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                files(&path, extensions, out);
            } else if extensions
                .iter()
                .any(|e| path.extension().is_some_and(|x| x == *e))
            {
                out.push(path);
            }
        }
    }

    /// The problems with the anchors of one file's `text`: an anchor
    /// opened twice, closed before it opens or never closed.
    fn problems(text: &str) -> Vec<String> {
        let mut open = BTreeSet::new();
        let mut seen = BTreeSet::new();
        let mut problems = Vec::new();
        for (n, line) in text.lines().enumerate() {
            let line = line.trim();
            if let Some(name) = line.strip_prefix("// ANCHOR: ") {
                if !seen.insert(name.to_owned()) {
                    problems.push(format!("line {}: `{name}` again", n + 1));
                }
                open.insert(name.to_owned());
            } else if let Some(name) = line.strip_prefix("// ANCHOR_END: ")
                && !open.remove(name)
            {
                problems.push(format!("line {}: `{name}` ends but isn't open", n + 1));
            }
        }
        problems.extend(open.iter().map(|name| format!("`{name}` never ends")));
        problems
    }

    #[test]
    fn every_anchor_opens_once_and_ends() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut all = vec![root.join("build.rs")];
        for dir in ["src", "files", "actions", "project"] {
            files(&root.join(dir), &["rs", "ron"], &mut all);
        }
        assert!(all.len() > 10, "{all:?}");
        for file in all {
            let text = fs::read_to_string(&file).unwrap();
            assert_eq!(problems(&text), Vec::<String>::new(), "{}", file.display());
        }
        assert_eq!(
            problems("// ANCHOR: a\n// ANCHOR: a\n// ANCHOR_END: b\n"),
            [
                "line 2: `a` again",
                "line 3: `b` ends but isn't open",
                "`a` never ends"
            ]
        );
    }
}
