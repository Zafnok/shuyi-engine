//! `cargo xtask golden --bless` (ticket 2116): writes the golden tables
//! (`shuyi_assets::golden`) a build gives.
//!
//! It runs every test with `golden` in its name, natively, with
//! `SHUYI_BLESS=1` set (for a test that blesses itself), and reads its
//! output: each failed table test printed the table its run gives between
//! `--- new golden table: <path> ---` and `--- end of golden table ---`.
//! It writes each such table to its path (the last one printed for a path
//! wins), refusing a path outside the repository, prints what it wrote
//! with `git diff --stat`, then runs the tests again: they must pass.
//!
//! Every `git` and `cargo` call goes through [`Runner`], which the tests
//! fake.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Component, Path, PathBuf};

use crate::api::{Ran, Runner};

/// What runs the golden tests: every test of the workspace whose name has
/// `golden` in it.
pub const TEST_ARGS: &[&str] = &[
    "test",
    "--workspace",
    "--locked",
    "--no-fail-fast",
    "--",
    "golden",
];

/// The line before a new table: this, its path, then [`START_END`]
/// (`shuyi_assets::golden::TABLE_START`).
const START: &str = "--- new golden table: ";
/// The end of [`START`]'s line.
const START_END: &str = " ---";
/// The line after a new table (`shuyi_assets::golden::TABLE_END`).
const END: &str = "--- end of golden table ---";

/// The last lines of a failed run's output a message keeps.
const TAIL: usize = 40;

/// The tables between markers in `output`, by path: the last one printed
/// for a path wins. A table's text is its lines, each ending in `\n`, as
/// `shuyi_assets::golden::table_text` writes it.
pub fn tables(output: &str) -> BTreeMap<String, String> {
    let mut found = BTreeMap::new();
    let mut open: Option<(String, String)> = None;
    for line in output.lines() {
        if let Some(path) = line
            .strip_prefix(START)
            .and_then(|rest| rest.strip_suffix(START_END))
        {
            open = Some((path.to_string(), String::new()));
        } else if line == END {
            if let Some((path, text)) = open.take() {
                found.insert(path, text);
            }
        } else if let Some((_, text)) = &mut open {
            text.push_str(line);
            text.push('\n');
        }
    }
    found
}

/// `path` as a file inside `root`, or why it is refused.
pub fn inside(root: &Path, path: &str) -> Result<PathBuf, String> {
    let given = Path::new(path);
    let full = if given.is_absolute() {
        given.to_path_buf()
    } else {
        root.join(given)
    };
    let climbs = full.components().any(|c| matches!(c, Component::ParentDir));
    if climbs || !full.starts_with(root) {
        return Err(format!(
            "{path}: outside the repository ({}), so not written",
            root.display()
        ));
    }
    Ok(full)
}

/// What a bless did: the lines it printed, and what went wrong.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Outcome {
    pub lines: Vec<String>,
    pub errors: Vec<String>,
}

/// A run's output, stdout then stderr.
fn output(ran: &Ran) -> String {
    format!("{}\n{}", ran.stdout, ran.stderr)
}

/// The last [`TAIL`] lines of `ran`'s output.
fn tail(ran: &Ran) -> String {
    let all = output(ran);
    let lines: Vec<&str> = all.lines().collect();
    lines[lines.len().saturating_sub(TAIL)..].join("\n")
}

/// Runs the golden tests, writes the tables they print, and runs them
/// again ([module docs](self)).
pub fn bless(root: &Path, runner: &dyn Runner) -> Outcome {
    let mut outcome = Outcome::default();
    let first = match runner.run_with_env(root, "cargo", TEST_ARGS, &[("SHUYI_BLESS", "1")]) {
        Ok(ran) => ran,
        Err(e) => {
            outcome.errors.push(e);
            return outcome;
        }
    };
    let found = tables(&output(&first));
    if found.is_empty() {
        if first.ok {
            outcome
                .lines
                .push("every golden table matches: nothing to bless.".to_string());
        } else {
            outcome.errors.push(format!(
                "the golden tests failed without printing a table, so it isn't a table's \
                 value that moved. Their output ends:\n{}",
                tail(&first)
            ));
        }
        return outcome;
    }
    let mut written = Vec::new();
    for (path, text) in &found {
        let file = match inside(root, path) {
            Ok(file) => file,
            Err(e) => {
                outcome.errors.push(e);
                continue;
            }
        };
        match fs::write(&file, text) {
            Ok(()) => {
                outcome.lines.push(format!("wrote {path}"));
                written.push(file);
            }
            Err(e) => outcome.errors.push(format!("{path}: {e}")),
        }
    }
    if !written.is_empty() {
        let mut args = vec!["diff", "--stat", "--"];
        let shown: Vec<String> = written.iter().map(|f| f.display().to_string()).collect();
        args.extend(shown.iter().map(String::as_str));
        match runner.run(root, "git", &args) {
            Ok(ran) => outcome.lines.extend(ran.stdout.lines().map(str::to_string)),
            Err(e) => outcome.errors.push(e),
        }
    }
    if !outcome.errors.is_empty() {
        return outcome;
    }
    match runner.run(root, "cargo", TEST_ARGS) {
        Ok(ran) if ran.ok => outcome
            .lines
            .push("the golden tests pass with the tables written.".to_string()),
        Ok(ran) => outcome.errors.push(format!(
            "the golden tests still fail after blessing. Their output ends:\n{}",
            tail(&ran)
        )),
        Err(e) => outcome.errors.push(e),
    }
    outcome
}

#[cfg(test)]
mod tests;
