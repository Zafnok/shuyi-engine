//! `cargo xtask check-paths`: no tracked path is longer than [`MAX_PATH_LEN`]
//! characters (ticket 2071).
//!
//! A game depends on the engine by git, and Cargo checks the repository out
//! under `C:\Users\<user>\.cargo\git\checkouts\shuyi-engine-<16 hex>\<7 hex>\`
//! on Windows: about 80 characters on a GitHub runner (`runneradmin`), more
//! for a longer user name. libgit2 refuses a path over Windows' 260 unless
//! `core.longpaths` is set, so one long file name in this repository fails
//! every Windows build of every game on it (the game's CI at `ab593bb9`:
//! a 196-character ticket file, 276 on the runner). 150 leaves 109
//! characters for the prefix.

use std::path::Path;
use std::process::Command;

/// The longest path a tracked file may have, from the repository's root.
pub const MAX_PATH_LEN: usize = 150;

/// Checks every path `git ls-files` lists under `repo_root` and returns one
/// error per path over the limit (empty means clean).
pub fn run(repo_root: &Path) -> Vec<String> {
    let output = match Command::new("git")
        .args(["ls-files", "-z"])
        .current_dir(repo_root)
        .output()
    {
        Ok(output) if output.status.success() => output,
        Ok(output) => {
            return vec![format!(
                "`git ls-files` failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            )];
        }
        Err(e) => return vec![format!("git doesn't run: {e}")],
    };
    let listing = String::from_utf8_lossy(&output.stdout);
    too_long(listing.split('\0').filter(|p| !p.is_empty()))
}

/// One error for each of `paths` over [`MAX_PATH_LEN`] characters.
fn too_long<'a>(paths: impl Iterator<Item = &'a str>) -> Vec<String> {
    paths.filter_map(path_error).collect()
}

/// The error for one repository path, if it is over the limit.
pub fn path_error(path: &str) -> Option<String> {
    let len = path.chars().count();
    (len > MAX_PATH_LEN).then(|| {
        format!(
            "{path}: {len} characters, over {MAX_PATH_LEN}; shorten the file name \
             (Cargo's checkout on Windows adds about 80, and libgit2 stops at 260)"
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_path_at_the_limit_passes_and_one_over_fails() {
        let at = "a".repeat(MAX_PATH_LEN);
        let over = "a".repeat(MAX_PATH_LEN + 1);
        assert_eq!(path_error(&at), None);
        let error = path_error(&over).expect("over the limit");
        assert!(error.contains("151 characters, over 150"), "{error}");
    }

    #[test]
    fn characters_are_counted_not_bytes() {
        // 150 two-byte characters are 300 bytes but 150 characters.
        assert_eq!(path_error(&"é".repeat(MAX_PATH_LEN)), None);
    }

    #[test]
    fn only_the_long_paths_are_reported() {
        let long = format!("tickets/open/{}.md", "x".repeat(MAX_PATH_LEN));
        let errors = too_long(["README.md", long.as_str(), "docs/adr/README.md"].into_iter());
        assert_eq!(errors.len(), 1);
        assert!(errors[0].starts_with(&long), "{}", errors[0]);
    }

    #[test]
    fn this_repository_has_no_path_over_the_limit() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        assert_eq!(run(&root), Vec::<String>::new());
    }
}
