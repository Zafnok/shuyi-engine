//! `cargo xtask compile-times`: what a game's developer waits for (ticket
//! 2009).
//!
//! It makes a game with `shuyi new grid` on this checkout of the engine, in
//! `target/compile-times/` (or `--dir`), with the engine's own
//! `Cargo.lock`, and builds it in its own target folder as its developer
//! would, with the setup the template gives it (`.cargo/config.toml`, the
//! profiles) and no `RUSTFLAGS` of the caller's. It times, from cargo's own
//! `Finished … in` line:
//!
//! - the clean `cargo build`;
//! - the rebuild after a one-line change in the game's `main.rs`, three
//!   times, and their median;
//! - the same for `cargo check`, which is what an editor runs;
//!
//! and counts the crates in a new game's dependency tree (`cargo metadata`
//! on a game from `shuyi new blank`). With `--budget`, the median rebuild
//! and the count are held to `tools/compile-budget.ron`, which CI's
//! `compile-times` job does on its runners.
//!
//! Every cargo call goes through [`Runner`], which the tests fake.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Deserialize;
use serde_json::{Value, json};

use crate::script::{Ran, Runner};
use crate::skills::make_game;

/// Where the games are made, from the repository's root.
const GAMES: &str = "target/compile-times";
/// How many times each rebuild is timed.
const REBUILDS: usize = 3;
/// The file of the game a rebuild changes.
const MAIN: &str = "crates/game/src/main.rs";
/// The variables that would replace the game's own `rustflags`
/// (`.cargo/config.toml`); a developer's shell rarely sets them, CI's does.
const RUSTFLAGS: &[&str] = &[
    "RUSTFLAGS",
    "CARGO_ENCODED_RUSTFLAGS",
    "CARGO_BUILD_RUSTFLAGS",
];

/// `compile-times`' options.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Options {
    /// `--json`: the answer as JSON, not a table.
    pub json: bool,
    /// `--budget <file>`: hold the numbers to this budget.
    pub budget: Option<PathBuf>,
    /// `--summary <file>`: append the table's Markdown to this file.
    pub summary: Option<PathBuf>,
    /// `--dir <dir>`: make the games here, not in `target/compile-times`.
    pub dir: Option<PathBuf>,
}

impl Options {
    /// The options `args` give, or `None` on anything else.
    pub fn parse(args: &[String]) -> Option<Self> {
        let mut options = Self::default();
        let mut rest = args.iter();
        while let Some(arg) = rest.next() {
            match arg.as_str() {
                "--json" => options.json = true,
                "--budget" => options.budget = Some(PathBuf::from(rest.next()?)),
                "--summary" => options.summary = Some(PathBuf::from(rest.next()?)),
                "--dir" => options.dir = Some(PathBuf::from(rest.next()?)),
                _ => return None,
            }
        }
        Some(options)
    }
}

/// What one run measured, in seconds.
#[derive(Debug, Clone, PartialEq)]
pub struct Times {
    /// The OS it ran on, as `std::env::consts::OS` names it.
    pub os: String,
    /// The clean `cargo build`.
    pub clean: f64,
    /// Each `cargo build` after a one-line change.
    pub rebuilds: Vec<f64>,
    /// The clean `cargo check`, after the builds.
    pub check_clean: f64,
    /// Each `cargo check` after a one-line change.
    pub check_rebuilds: Vec<f64>,
    /// The crates in a new game's dependency tree.
    pub dependencies: usize,
}

impl Times {
    /// The median rebuild.
    pub fn rebuild(&self) -> f64 {
        median(&self.rebuilds)
    }

    /// The median `check` rebuild.
    pub fn check_rebuild(&self) -> f64 {
        median(&self.check_rebuilds)
    }

    /// The numbers as JSON.
    pub fn json(&self) -> Value {
        json!({
            "os": self.os,
            "clean_build": self.clean,
            "rebuild": self.rebuild(),
            "rebuilds": self.rebuilds,
            "check_clean": self.check_clean,
            "check_rebuild": self.check_rebuild(),
            "check_rebuilds": self.check_rebuilds,
            "dependencies": self.dependencies,
        })
    }

    /// The numbers as a Markdown table, which reads as plain text too.
    pub fn table(&self) -> String {
        let all = |times: &[f64]| {
            times
                .iter()
                .map(|t| format!("{t:.2}"))
                .collect::<Vec<_>>()
                .join(", ")
        };
        let mut table = format!(
            "Compile times of a new `grid` game on {}\n\n\
             | What | Seconds |\n\
             | ---- | ------- |\n",
            self.os
        );
        // Writing to a `String` can't fail.
        let _ = writeln!(table, "| Clean build | {:.2} |", self.clean);
        let _ = writeln!(
            table,
            "| Rebuild after a one-line change (median of {}) | {:.2} |",
            all(&self.rebuilds),
            self.rebuild()
        );
        let _ = writeln!(table, "| Clean `check` | {:.2} |", self.check_clean);
        let _ = writeln!(
            table,
            "| `check` after a one-line change (median of {}) | {:.2} |",
            all(&self.check_rebuilds),
            self.check_rebuild()
        );
        let _ = writeln!(
            table,
            "\nA new `blank` game depends on {} crates.",
            self.dependencies
        );
        table
    }
}

/// The median of `times`; 0 for none.
pub fn median(times: &[f64]) -> f64 {
    let mut sorted = times.to_vec();
    sorted.sort_by(f64::total_cmp);
    match sorted.len() {
        0 => 0.0,
        n if n % 2 == 1 => sorted[n / 2],
        n => f64::midpoint(sorted[n / 2 - 1], sorted[n / 2]),
    }
}

/// `tools/compile-budget.ron`: what a game's compile may cost.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Budget {
    /// The most the median rebuild may take, in seconds, by OS.
    pub rebuild: BTreeMap<String, f64>,
    /// The most crates a new game's dependency tree may hold.
    pub dependencies: usize,
}

/// Reads the budget in `text`.
pub fn parse_budget(text: &str) -> Result<Budget, String> {
    ron::from_str(text).map_err(|e| e.to_string())
}

/// What a check against a budget found: the errors, and the warnings.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Verdict {
    /// What is over the budget.
    pub errors: Vec<String>,
    /// What is well under it, so the budget could come down.
    pub warnings: Vec<String>,
}

/// Holds `times` to `budget` (`file` names it in the messages).
pub fn check(times: &Times, budget: &Budget, file: &str) -> Verdict {
    let mut verdict = Verdict::default();
    match budget.rebuild.get(&times.os) {
        Some(&most) if times.rebuild() > most => verdict.errors.push(format!(
            "the median rebuild took {:.2} s on {}, over the {most:.2} s of {file}: \
             find what made it slower (`cargo build --timings`) before raising it",
            times.rebuild(),
            times.os
        )),
        Some(_) => {}
        None => verdict
            .errors
            .push(format!("{file} has no rebuild budget for {}", times.os)),
    }
    match times.dependencies.cmp(&budget.dependencies) {
        std::cmp::Ordering::Greater => verdict.errors.push(format!(
            "a new game depends on {} crates, more than the {} of {file}: a new dependency \
             is a choice, so raise the number there in the same change, or drop the crate",
            times.dependencies, budget.dependencies
        )),
        std::cmp::Ordering::Less => verdict.warnings.push(format!(
            "a new game depends on {} crates, fewer than the {} of {file}: lower it",
            times.dependencies, budget.dependencies
        )),
        std::cmp::Ordering::Equal => {}
    }
    verdict
}

/// The seconds of cargo's `Finished … in 2.34s` (or `in 1m 05s`) line in
/// `stderr`, the last if there are several.
pub fn finished_in(stderr: &str) -> Option<f64> {
    let line = stderr
        .lines()
        .rev()
        .find(|line| line.trim_start().starts_with("Finished"))?;
    let (_, time) = line.rsplit_once(" in ")?;
    let time = time.trim();
    let seconds = |s: &str| s.strip_suffix('s')?.parse::<f64>().ok();
    match time.split_once("m ") {
        Some((minutes, rest)) => Some(minutes.parse::<f64>().ok()? * 60.0 + seconds(rest)?),
        None => seconds(time),
    }
}

/// The crates in the dependency tree of `cargo metadata`'s JSON: every
/// package of the resolve but the workspace's own.
pub fn dependencies(metadata: &str) -> Result<usize, String> {
    let metadata: Value =
        serde_json::from_str(metadata).map_err(|e| format!("cargo metadata: {e}"))?;
    let members: Vec<&Value> = metadata["workspace_members"]
        .as_array()
        .map(|members| members.iter().collect())
        .unwrap_or_default();
    let nodes = metadata["resolve"]["nodes"]
        .as_array()
        .ok_or("cargo metadata: no resolve")?;
    Ok(nodes
        .iter()
        .filter(|node| !members.contains(&&node["id"]))
        .count())
}

/// The real cargo for a game: its own target folder, and none of the
/// caller's `RUSTFLAGS`.
pub struct Game {
    /// The game's target folder.
    pub target_dir: PathBuf,
}

impl Runner for Game {
    fn run(&self, dir: &Path, program: &str, args: &[String]) -> Result<Ran, String> {
        let mut command = Command::new(program);
        command
            .args(args)
            .current_dir(dir)
            .env("CARGO_TARGET_DIR", &self.target_dir);
        for name in RUSTFLAGS {
            command.env_remove(name);
        }
        let output = command
            .output()
            .map_err(|e| format!("`{program}` did not run ({e})"))?;
        Ok(Ran {
            code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }
}

/// Where the games are made: `dir`, or under the repository's root.
pub fn games_dir(root: &Path, dir: Option<&Path>) -> PathBuf {
    dir.map_or_else(|| root.join(GAMES), Path::to_path_buf)
}

/// Measures, with the engine at `root`, games made in `games`: `maker` runs
/// `shuyi new` in the checkout, `game` runs cargo in the games.
pub fn run(
    root: &Path,
    games: &Path,
    maker: &dyn Runner,
    game: &dyn Runner,
) -> Result<Times, String> {
    let timed = games.join("timed");
    let blank = games.join("blank");
    for (dir, template, name) in [(&timed, "grid", "timed"), (&blank, "blank", "blank")] {
        make_game(root, dir, template, name, maker)?;
        // The engine's own versions of every crate, so the count and the
        // times don't move when a crate releases.
        fs::copy(root.join("Cargo.lock"), dir.join("Cargo.lock"))
            .map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    let metadata = cargo(game, &blank, &["metadata", "--format-version", "1"])?;
    let dependencies = dependencies(&metadata.stdout)?;
    let clean = time(game, &timed, "build")?;
    let rebuilds = time_rebuilds(game, &timed, "build")?;
    let check_clean = time(game, &timed, "check")?;
    let check_rebuilds = time_rebuilds(game, &timed, "check")?;
    Ok(Times {
        os: std::env::consts::OS.to_owned(),
        clean,
        rebuilds,
        check_clean,
        check_rebuilds,
        dependencies,
    })
}

/// `cargo <command>` in `dir` after a new line in the game's `main.rs`,
/// [`REBUILDS`] times.
fn time_rebuilds(runner: &dyn Runner, dir: &Path, command: &str) -> Result<Vec<f64>, String> {
    let main = dir.join(MAIN);
    (1..=REBUILDS)
        .map(|n| {
            let mut text =
                fs::read_to_string(&main).map_err(|e| format!("{}: {e}", main.display()))?;
            let _ = writeln!(text, "// compile-times: {command} {n}");
            fs::write(&main, text).map_err(|e| format!("{}: {e}", main.display()))?;
            time(runner, dir, command)
        })
        .collect()
}

/// The seconds `cargo <command>` takes in `dir`.
fn time(runner: &dyn Runner, dir: &Path, command: &str) -> Result<f64, String> {
    let ran = cargo(runner, dir, &[command, "--color", "never"])?;
    finished_in(&ran.stderr).ok_or_else(|| {
        format!(
            "`cargo {command}` in {} said no `Finished … in` time",
            dir.display()
        )
    })
}

/// `cargo <args>` in `dir`; an error when it fails.
fn cargo(runner: &dyn Runner, dir: &Path, args: &[&str]) -> Result<Ran, String> {
    let args: Vec<String> = args.iter().map(|&a| a.to_owned()).collect();
    let ran = runner.run(dir, "cargo", &args)?;
    if ran.code == Some(0) {
        Ok(ran)
    } else {
        let tail: Vec<&str> = ran.stderr.lines().rev().take(20).collect();
        Err(format!(
            "`cargo {}` in {} failed:\n{}",
            args.join(" "),
            dir.display(),
            tail.into_iter().rev().collect::<Vec<_>>().join("\n")
        ))
    }
}

#[cfg(test)]
mod tests;
