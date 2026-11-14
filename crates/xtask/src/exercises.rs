//! `cargo xtask exercises [--copy] [<step>...]`: the book's exercises
//! (ticket 2828, ADR-0115), checked by the game of the step they are on.
//!
//! The checks are plain Rust in `site/exercises/`, never in a step's own
//! project, which stays as its page tells the reader to make it: one file a
//! step, named as its folder (`02-board.rs`), the binary every step runs
//! with them (`main.rs`), and the helpers they share. For each step this
//! makes a copy of it under `target/exercises/<step>/`, three folders down
//! like a step so its path dependencies on the engine still resolve, with
//! `main.rs` in place of the step's and the folder's files under
//! `crates/game/src/exercises/`, the step's as `checks.rs`. The copy keeps
//! the step's manifest and lock: the checks add no dependency.
//!
//! With no flag it checks them too: each step's page asks only for checks
//! its file has and its file has only checks a page asks for, and in each
//! copy `cargo fmt` leaves it as it is (the folder's files are no crate's,
//! so the workspace's `cargo fmt` never sees them), Clippy passes and the
//! tests do (each check passes with its exercise's edit and fails without). With
//! `--copy` it only makes the copies, for the site's web builds of the steps
//! (`pages.yml`, CI's `web-render` job).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::book;
use crate::tries::copy_tree;

#[cfg(test)]
mod tests;

/// The steps' projects, from the repository's root.
const STEPS: &str = "site/tutorial";
/// The checks, from the repository's root.
const EXERCISES: &str = "site/exercises";
/// The steps' pages, from the repository's root.
const PAGES: &str = "site/content/learn/book/box-pusher";
/// Where the copies are made, from the repository's root.
pub const COPIES: &str = "target/exercises";
/// The copies' one target folder, from the repository's root.
const BUILD: &str = "target/exercises-build";
/// The binary every copy runs, in [`EXERCISES`].
const MAIN: &str = "main.rs";
/// The folder of the copy the checks go in, and the step's file's name there.
const CHECKS_DIR: &str = "crates/game/src/exercises";
/// See [`CHECKS_DIR`].
const CHECKS_FILE: &str = "checks.rs";
/// The binary of a step's game, which the copy's checks are compiled into.
const BIN: &str = "box-pusher";

/// `cargo xtask exercises`' options.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Options {
    /// `--copy`: only make the copies.
    pub copy: bool,
    /// Only these steps; every one when empty.
    pub steps: Vec<String>,
}

impl Options {
    /// The options in `args`, or `None` on a flag it doesn't know.
    pub fn parse(args: &[String]) -> Option<Self> {
        let mut options = Self::default();
        for arg in args {
            match arg.as_str() {
                "--copy" => options.copy = true,
                flag if flag.starts_with('-') => return None,
                step => options.steps.push(step.to_owned()),
            }
        }
        Some(options)
    }
}

/// The names of the checks the step's file `text` holds: each `("name",`
/// of its `CHECKS` table.
pub fn names_in(text: &str) -> Vec<String> {
    let Some(table) = text
        .split_once("pub const CHECKS")
        .and_then(|(_, rest)| rest.split_once("];"))
        .map(|(table, _)| table)
    else {
        return Vec::new();
    };
    table
        .split("(\"")
        .skip(1)
        .filter_map(|after| after.split_once('"').map(|(name, _)| name.to_owned()))
        .collect()
}

/// What is wrong between the page of `step`, whose text is `page`, and its
/// checks' file, whose text is `checks` (none when there is no file): a
/// page with a playground and no file, an exercise whose check the file
/// lacks, a check no exercise asks for.
pub fn page_problems(step: &str, page: &str, checks: Option<&str>) -> Vec<String> {
    let shown = format!("{PAGES}/{step}.md");
    let file = format!("{EXERCISES}/{step}.rs");
    let with_files = !book::page_files(page).is_empty();
    let asked: Vec<String> = book::exercises(&shown, page)
        .into_iter()
        .map(|exercise| exercise.check)
        .collect();
    let Some(checks) = checks else {
        return if with_files {
            vec![format!(
                "{shown}: the step has a playground, so its exercises' checks are in {file}, which isn't there"
            )]
        } else {
            Vec::new()
        };
    };
    let held = names_in(checks);
    let mut errors = Vec::new();
    if held.is_empty() {
        errors.push(format!(
            "{file}: no `pub const CHECKS` naming its checks (ADR-0115)"
        ));
    }
    for name in asked.iter().filter(|name| !held.contains(name)) {
        errors.push(format!(
            "{shown}: an exercise asks for the check `{name}`, which {file}'s `CHECKS` doesn't hold"
        ));
    }
    for name in held.iter().filter(|name| !asked.contains(name)) {
        errors.push(format!(
            "{file}: the check `{name}` is asked for by no exercise of {shown}"
        ));
    }
    errors
}

/// Makes the copies of the steps `options` names, and without `--copy`
/// checks them; every problem.
pub fn run(root: &Path, options: &Options) -> Vec<String> {
    let steps = match steps(root) {
        Ok(steps) => steps,
        Err(e) => return vec![e],
    };
    let mut errors = Vec::new();
    for step in &options.steps {
        if !steps.contains(step) {
            errors.push(format!("{STEPS}: no step `{step}`"));
        }
    }
    let chosen: Vec<&String> = steps
        .iter()
        .filter(|step| options.steps.is_empty() || options.steps.contains(step))
        .collect();
    for step in &chosen {
        let page = fs::read_to_string(root.join(PAGES).join(format!("{step}.md")));
        let checks = fs::read_to_string(root.join(EXERCISES).join(format!("{step}.rs"))).ok();
        match page {
            Ok(page) => errors.extend(page_problems(step, &page, checks.as_deref())),
            Err(e) => errors.push(format!("{PAGES}/{step}.md: {e}")),
        }
    }
    if !errors.is_empty() {
        return errors;
    }
    for step in &chosen {
        if let Err(e) = copy(root, step) {
            return vec![e];
        }
    }
    if options.copy {
        return errors;
    }
    let build = root.join(BUILD);
    for step in chosen {
        let dir = root.join(COPIES).join(step);
        eprintln!("exercises: {step}");
        for args in [
            &["fmt", "--all", "--check"][..],
            &[
                "clippy", "--locked", "-q", "--bin", BIN, "--tests", "--", "-D", "warnings",
            ],
            &["test", "--locked", "-q", "--bin", BIN],
        ] {
            if let Err(e) = cargo(&dir, &build, args) {
                errors.push(format!("{step}: {e}"));
            }
        }
    }
    errors
}

/// Every step's folder name, in order.
fn steps(root: &Path) -> Result<Vec<String>, String> {
    let read = fs::read_dir(root.join(STEPS)).map_err(|e| format!("{STEPS}: {e}"))?;
    let mut steps = Vec::new();
    for entry in read {
        let entry = entry.map_err(|e| format!("{STEPS}: {e}"))?;
        if entry.path().is_dir() {
            steps.push(entry.file_name().to_string_lossy().into_owned());
        }
    }
    steps.sort();
    Ok(steps)
}

/// Makes the copy of `step`, with the checks spliced in.
fn copy(root: &Path, step: &str) -> Result<(), String> {
    let to = root.join(COPIES).join(step);
    if to.exists() {
        fs::remove_dir_all(&to).map_err(|e| format!("{}: {e}", to.display()))?;
    }
    copy_tree(&root.join(STEPS).join(step), &to).map_err(|e| format!("copying {step}: {e}"))?;
    let from = root.join(EXERCISES);
    let checks = to.join(CHECKS_DIR);
    fs::create_dir_all(&checks).map_err(|e| format!("{}: {e}", checks.display()))?;
    let put = |file: PathBuf, at: PathBuf| {
        fs::copy(&file, &at)
            .map(|_| ())
            .map_err(|e| format!("{} to {}: {e}", file.display(), at.display()))
    };
    put(from.join(MAIN), to.join("crates/game/src/main.rs"))?;
    put(from.join(format!("{step}.rs")), checks.join(CHECKS_FILE))?;
    for helper in helpers(root)? {
        put(from.join(&helper), checks.join(&helper))?;
    }
    Ok(())
}

/// The files of [`EXERCISES`] a step's checks may name: every `.rs` file
/// but [`MAIN`] and the steps' own.
fn helpers(root: &Path) -> Result<Vec<String>, String> {
    let steps = steps(root)?;
    let read = fs::read_dir(root.join(EXERCISES)).map_err(|e| format!("{EXERCISES}: {e}"))?;
    let mut found = Vec::new();
    for entry in read {
        let name = entry
            .map_err(|e| format!("{EXERCISES}: {e}"))?
            .file_name()
            .to_string_lossy()
            .into_owned();
        let Some(stem) = name.strip_suffix(".rs") else {
            continue;
        };
        if name != MAIN && !steps.iter().any(|step| step == stem) {
            found.push(name);
        }
    }
    found.sort();
    Ok(found)
}

/// Runs cargo with `args` in `dir`, building in `build`.
fn cargo(dir: &Path, build: &Path, args: &[&str]) -> Result<(), String> {
    let output = Command::new("cargo")
        .args(args)
        .current_dir(dir)
        .env("CARGO_TARGET_DIR", build)
        .output()
        .map_err(|e| format!("cargo did not run: {e}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "`cargo {}` failed:\n{}{}",
            args.join(" "),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}
