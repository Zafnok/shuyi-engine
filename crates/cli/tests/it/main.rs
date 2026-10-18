//! `shuyi` on projects: the binary itself on scratch projects, and the
//! toy game's tools (`frame-png`, `lang-status`) as its tool program runs
//! them.

mod binary;
mod toy;

use std::path::PathBuf;

/// A fresh empty folder under the system temp folder.
pub fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("shuyi-it-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display()));
    dir
}

/// `items` as owned arguments.
pub fn args(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| (*s).to_owned()).collect()
}
