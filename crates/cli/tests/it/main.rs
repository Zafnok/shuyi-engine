//! `shuyi` on projects: the binary itself on scratch projects, its MCP
//! server driven over stdio against Hello, Grid, the toy
//! game's tools (`frame-png`, `lang-status`) as its tool program runs
//! them, the contract's checked-in schemas, and `docs/contract.md` and the
//! README against the contract.

mod binary;
mod docs;
mod mcp;
mod modules;
mod schemas;
mod skills;
mod targets;
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
