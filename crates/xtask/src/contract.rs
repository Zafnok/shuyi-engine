//! The contract's version against the base's (ticket 2052): a PR that
//! changes `contract::VERSION` (`crates/cli/src/contract.rs`) raises it by
//! exactly one. Two PRs that each raised it from 37 to 38 both passed on
//! their own; on the merge ref, the second's is the base's again (its
//! schemas' test then asks for a raise) or one more than the base's plus
//! one. `cargo xtask api` runs it against its base.

use std::fs;
use std::path::Path;

use crate::api::Runner;

/// The file that holds the contract's version.
pub const FILE: &str = "crates/cli/src/contract.rs";

/// Checks the working tree's version against `base`'s. A side whose file
/// or line is missing is not compared: the schemas' own test covers that.
pub fn check(root: &Path, base: &str, runner: &dyn Runner) -> Result<(), String> {
    let head = fs::read_to_string(root.join(FILE))
        .ok()
        .and_then(|text| version_in(&text));
    let base_text = runner
        .run(root, "git", &["show", &format!("{base}:{FILE}")])
        .ok()
        .filter(|ran| ran.ok)
        .map(|ran| ran.stdout);
    let base = base_text.as_deref().and_then(version_in);
    match (base, head) {
        (Some(base), Some(head)) => compare(base, head),
        _ => Ok(()),
    }
}

/// The number of `pub const VERSION: u32 = N;` in `text`.
pub fn version_in(text: &str) -> Option<u32> {
    text.lines().find_map(|line| {
        line.trim()
            .strip_prefix("pub const VERSION: u32 = ")?
            .strip_suffix(';')?
            .trim()
            .parse()
            .ok()
    })
}

/// The rule: the same version, or the base's plus one.
pub fn compare(base: u32, head: u32) -> Result<(), String> {
    if head == base || Some(head) == base.checked_add(1) {
        return Ok(());
    }
    let next = base.saturating_add(1);
    Err(format!(
        "{FILE}: `VERSION` goes from {base} to {head}; a change raises it by one, to {next}. \
         Another PR may have raised it first: set it to {next}, move this PR's \
         `schemas/versions.json` entry to {next}, and fix the PR body and completion \
         notes that name {head}"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::Ran;

    #[test]
    fn the_same_version_or_one_more_passes() {
        assert_eq!(compare(41, 41), Ok(()));
        assert_eq!(compare(41, 42), Ok(()));
    }

    #[test]
    fn two_more_fails() {
        let error = compare(41, 43).unwrap_err();
        assert!(error.contains("from 41 to 43"), "{error}");
        assert!(error.contains("to 42"), "{error}");
    }

    #[test]
    fn a_lowered_version_fails() {
        assert!(compare(41, 40).is_err());
        assert!(compare(41, 0).is_err());
    }

    #[test]
    fn reads_the_constant() {
        let text = "/// The version.\npub const VERSION: u32 = 41;\n";
        assert_eq!(version_in(text), Some(41));
        assert_eq!(version_in("const VERSION: u32 = 41;"), None);
        assert_eq!(version_in("pub const VERSION: u32 = x;"), None);
    }

    #[test]
    fn real_contract_has_a_version() {
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("xtask is at <repo>/crates/xtask");
        let text = fs::read_to_string(repo_root.join(FILE)).unwrap();
        assert!(version_in(&text).is_some());
    }

    /// Answers `git show` with a fixed file.
    struct Base(Option<String>);

    impl Runner for Base {
        fn run(&self, _: &Path, _: &str, _: &[&str]) -> Result<Ran, String> {
            Ok(Ran {
                ok: self.0.is_some(),
                stdout: self.0.clone().unwrap_or_default(),
                stderr: String::new(),
            })
        }
    }

    #[test]
    fn checks_the_working_tree_against_the_base() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("xtask is at <repo>/crates/xtask");
        let head = version_in(&fs::read_to_string(root.join(FILE)).unwrap()).unwrap();
        let base = |version: u32| Base(Some(format!("pub const VERSION: u32 = {version};\n")));
        assert_eq!(check(root, "base", &base(head)), Ok(()));
        assert!(check(root, "base", &base(head - 2)).is_err());
        assert_eq!(check(root, "base", &Base(None)), Ok(()));
    }
}
