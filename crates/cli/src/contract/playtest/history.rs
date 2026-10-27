//! `playtest`'s history: every run kept as one JSON line, in a file per
//! stage, entrants and game options, so the next run can say what the
//! last one found.

use std::fs;
use std::io::{self, Write as _};
use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use shuyi_core::Made;

use super::PlaytestOutput;
use crate::game::GameArg;

/// One run of one stage, as a line of its history file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct Record {
    /// When it ran: `2026-10-22 14:05 UTC`.
    pub(crate) date: String,
    /// The short hash of the commit the project was on, or `unknown`.
    pub(crate) commit: String,
    /// The plan's title.
    pub(crate) title: String,
    /// The stage, if the game names its stages.
    pub(crate) stage: Option<String>,
    /// The entrants that played, in report order.
    pub(crate) entrants: Vec<String>,
    /// The game's options, as `flag=value`.
    pub(crate) options: Vec<String>,
    /// What made it: the build's stamp, if the game's tools give one
    /// (`GameTools::made`). Lines kept before stamps have none.
    #[serde(default)]
    pub(crate) made: Option<Made>,
    /// What it found.
    pub(crate) output: PlaytestOutput,
}

/// A game option as `flag=value` (or `flag`), without its dashes.
pub(crate) fn option(arg: &GameArg) -> String {
    let flag = arg.flag.trim_start_matches('-');
    match &arg.value {
        Some(value) => format!("{flag}={value}"),
        None => flag.to_owned(),
    }
}

/// The history file's name of a run of `stage` (`"stage"` if the game
/// names none) by `entrants` with the game's `options`:
/// `<stage>-<entrants joined by "+">[-<options>].jsonl`, anything outside
/// `[A-Za-z0-9_.=-]` in a name or an option turned into `_`.
pub(crate) fn file_name(stage: Option<&str>, entrants: &[String], options: &[String]) -> String {
    let mut name = clean(stage.unwrap_or("stage"));
    name.push('-');
    name.push_str(
        &entrants
            .iter()
            .map(|e| clean(e))
            .collect::<Vec<_>>()
            .join("+"),
    );
    for option in options {
        name.push('-');
        name.push_str(&clean(option));
    }
    name.push_str(".jsonl");
    name
}

/// `text` with anything outside `[A-Za-z0-9_.=-]` turned into `_`.
fn clean(text: &str) -> String {
    text.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || "_.=-".contains(c) {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// The last line of `file` that reads as a record, if any.
pub(crate) fn last(file: &Path) -> Option<Record> {
    let text = fs::read_to_string(file).ok()?;
    text.lines()
        .rev()
        .find_map(|line| serde_json::from_str(line).ok())
}

/// Adds `record` to `file` as a line, making it and its folder if needed.
pub(crate) fn append(file: &Path, record: &Record) -> io::Result<()> {
    if let Some(dir) = file.parent() {
        fs::create_dir_all(dir)?;
    }
    let line = serde_json::to_string(record)?;
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(file)?;
    writeln!(file, "{line}")
}

/// The short hash of the commit checked out in `root`, or `unknown`.
pub(crate) fn git_commit(root: &Path) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["rev-parse", "--short", "HEAD"])
        .output();
    output
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
        .filter(|hash| !hash.is_empty())
        .unwrap_or_else(|| "unknown".to_owned())
}

/// Seconds since 1970-01-01 00:00 UTC.
pub(crate) fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// `secs` since 1970-01-01 00:00 UTC as `2026-10-01 14:05 UTC`.
pub(crate) fn utc_stamp(secs: u64) -> String {
    let leap =
        |year: u64| year.is_multiple_of(4) && !year.is_multiple_of(100) || year.is_multiple_of(400);
    let mut days = secs / 86_400;
    let mut year = 1970;
    // Bounded, so a wrong count of days can't spin forever.
    for candidate in 1970..=9999 {
        year = candidate;
        let length = if leap(year) { 366 } else { 365 };
        if days < length {
            break;
        }
        days -= length;
    }
    let february = if leap(year) { 29 } else { 28 };
    let mut month = 1;
    for length in [31, february, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31] {
        if days < length {
            break;
        }
        days -= length;
        month += 1;
    }
    let minutes = secs % 86_400 / 60;
    format!(
        "{year}-{month:02}-{:02} {:02}:{:02} UTC",
        days + 1,
        minutes / 60,
        minutes % 60
    )
}
