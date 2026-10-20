//! `cargo xtask check-no-std`: no `std::` path in the code of the crates a
//! `no_std` target builds (ADR-0008 §5), outside `cfg(feature = "std")`.
//!
//! The `no_std` builds catch most of it already: CI builds these crates for
//! `thumbv7em-none-eabihf` and runs their tests without `std`. This also
//! catches what those builds never compile, such as code behind another
//! feature. A `std::` path is allowed:
//!
//! - on a line that names `cfg(feature = "std")` itself
//!   (`#[cfg(feature = "std")] use std::io;`);
//! - in a file that starts with `#![cfg(feature = "std")]`, which is where
//!   more `std`-only code goes.
//!
//! Comments are left out: documentation may show what a game with `std`
//! writes.

use std::fs;
use std::path::{Path, PathBuf};

/// The crates checked, by folder from the root.
pub const CRATES: &[&str] = &["crates/core", "crates/ui", "crates/platform"];

/// What lets a line, or a whole file, use `std`.
const STD_CFG: &str = "cfg(feature = \"std\")";

/// Checks the `src` of every crate in [`CRATES`] under `repo_root` and
/// returns every `std::` path found (empty means clean).
pub fn run(repo_root: &Path) -> Vec<String> {
    let mut errors = Vec::new();
    for krate in CRATES {
        let mut files = Vec::new();
        let src = repo_root.join(krate).join("src");
        if let Err(e) = rust_files(&src, &mut files) {
            errors.push(format!("{krate}/src: {e}"));
        }
        files.sort();
        for file in files {
            let display = file
                .strip_prefix(repo_root)
                .unwrap_or(&file)
                .to_string_lossy()
                .replace('\\', "/");
            match fs::read_to_string(&file) {
                Ok(text) => {
                    errors.extend(std_paths(&text).into_iter().map(|line| {
                        format!("{display}:{line}: a `std::` path outside `{STD_CFG}`")
                    }));
                }
                Err(e) => errors.push(format!("{display}: {e}")),
            }
        }
    }
    errors
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            rust_files(&path, out)?;
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
    Ok(())
}

/// The lines (from 1) of `text` with a `std::` path that isn't allowed.
fn std_paths(text: &str) -> Vec<usize> {
    if is_std_only(text) {
        return Vec::new();
    }
    text.lines()
        .enumerate()
        .filter(|(_, line)| {
            let code = code_of(line);
            !code.contains(STD_CFG) && has_std_path(code)
        })
        .map(|(i, _)| i + 1)
        .collect()
}

/// Whether the file's first line of code is `#![cfg(feature = "std")]`.
fn is_std_only(text: &str) -> bool {
    text.lines()
        .map(|line| code_of(line).trim())
        .find(|code| !code.is_empty())
        .is_some_and(|code| code == format!("#![{STD_CFG}]"))
}

/// A line without its `//` comment. A `//` inside a string ends the code
/// early too, which can only hide a path, never invent one.
fn code_of(line: &str) -> &str {
    line.find("//").map_or(line, |at| &line[..at])
}

/// Whether `code` has a path starting at `std`: `std::` that isn't the end
/// of a longer name (`no_std::` is not one; `::std::` is).
fn has_std_path(code: &str) -> bool {
    code.match_indices("std::").any(|(at, _)| {
        !code[..at]
            .chars()
            .next_back()
            .is_some_and(|c| c.is_alphanumeric() || c == '_')
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_std_path_in_code_is_found_on_its_line() {
        let text = "use alloc::vec::Vec;\nuse std::rc::Rc;\nfn f() { let _ = ::std::mem::size_of::<u8>(); }\n";
        assert_eq!(std_paths(text), [2, 3]);
    }

    #[test]
    fn core_alloc_and_longer_names_are_not_std() {
        let text =
            "use core::fmt;\nuse alloc::string::String;\nmod no_std::x;\nlet my_std::y = 1;\n";
        assert_eq!(std_paths(text), Vec::<usize>::new());
    }

    #[test]
    fn comments_and_doc_comments_are_left_out() {
        let text = "//! use std::rc::Rc;\n/// std::fs::read\nlet x = 1; // std::mem\n";
        assert_eq!(std_paths(text), Vec::<usize>::new());
    }

    #[test]
    fn a_line_with_the_std_cfg_may_use_std() {
        let text = "#[cfg(feature = \"std\")] use std::io;\n#[cfg(not(feature = \"std\"))] use std::io;\nuse std::io;\n";
        // `cfg(not(feature = "std"))` is the `no_std` build: no `std` there.
        assert_eq!(std_paths(text), [2, 3]);
    }

    #[test]
    fn a_file_behind_the_std_cfg_may_use_std() {
        let text = "//! Only with `std`.\n\n#![cfg(feature = \"std\")]\nuse std::io;\n";
        assert_eq!(std_paths(text), Vec::<usize>::new());
        let later = "use core::fmt;\n#![cfg(feature = \"std\")]\nuse std::io;\n";
        assert_eq!(std_paths(later), [3], "only as the first line of code");
    }

    #[test]
    fn the_checked_crates_are_clean() {
        let root = crate::repo_root();
        assert_eq!(run(&root), Vec::<String>::new());
    }

    #[test]
    fn only_rust_files_are_read_in_every_folder_of_src() {
        let root = std::env::temp_dir().join(format!("xtask-no-std-{}", std::process::id()));
        for krate in CRATES {
            fs::create_dir_all(root.join(krate).join("src/deep")).unwrap();
        }
        let core = root.join("crates/core/src");
        fs::write(core.join("deep/bad.rs"), "use std::io;\n").unwrap();
        fs::write(core.join("notes.md"), "use std::io;\n").unwrap();
        fs::write(core.join("snapshot.snap"), "std::fs\n").unwrap();
        let errors = run(&root);
        let _ = fs::remove_dir_all(&root);
        assert_eq!(
            errors,
            ["crates/core/src/deep/bad.rs:1: a `std::` path outside `cfg(feature = \"std\")`"]
        );
    }

    #[test]
    fn a_missing_crate_folder_is_an_error() {
        let errors = run(Path::new("no-such-repository"));
        assert_eq!(errors.len(), CRATES.len(), "{errors:?}");
        assert!(errors[0].starts_with("crates/core/src: "), "{errors:?}");
    }
}
