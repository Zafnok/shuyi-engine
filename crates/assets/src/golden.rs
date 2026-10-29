//! Golden tables (ticket 2116): values a build gives, pinned in a file
//! beside the tests, so a change that moves one fails a test and is made
//! on purpose. CI checks them on x64 and ARM64 (the `test` job) and on
//! wasm32 under wasmtime (the `determinism` job): the same inputs give
//! the same values on every platform (ADR-0002 §2, ADR-0007 §5).
//!
//! A table is a RON map from a row's name to its value, both strings,
//! written by [`table_text`]:
//!
//! ```text
//! {
//!     "step 000": "1a2b3c4d5e6f7a8b",
//!     "step 001": "8b7a6f5e4d3c2b1a",
//! }
//! ```
//!
//! A test reads its table with [`golden!`](crate::golden!), checks each
//! row, and asserts the rest were all checked:
//!
//! ```ignore
//! let mut golden = shuyi_assets::golden!("tests/golden/stage.ron");
//! golden.check("outcome", &run.outcome);
//! golden.assert();
//! ```
//!
//! # What goes in a table
//!
//! - What a simulation, a seeded generator or a decoder gives: a state's
//!   hash after each step of a record, a decoded asset's `state_hash`.
//!   Never the pixels of a look (graphics are a skin, `AGENTS.md` rule 9:
//!   those are `frame-png` snapshots), and never bytes a compressor may
//!   write differently on another platform (a pack's own bytes, ADR-0023).
//! - Rows never depend on the target or the features: every platform
//!   checks every row.
//! - Row names sort as they are meant to read: numbers zero-padded
//!   (`step 007`).
//! - The inputs are fixtures frozen for the table, never a file edited
//!   for other reasons (a game's `targets/*.ron`, say): a row moves only
//!   when what makes it changes.
//! - The test that checks a table has `golden` in its name, so `cargo
//!   test -- golden` runs them all.
//!
//! # A new table, and blessing one
//!
//! [`golden!`](crate::golden!) reads its file with `include_str!` (a test
//! reads no file at run time, so it runs the same under wasmtime and on a
//! device), so the file must exist: a new table starts as a file holding
//! `{}`. Its test then fails with every row added, and blessing writes it.
//! `cargo xtask golden --bless` runs the golden tests, writes each table
//! a failed test printed between its markers (`--- new golden table:
//! <path> ---` and `--- end of golden table ---`), and runs them again.
//!
//! A game, or a new example, adds a table the same way: a dev-dependency
//! on `shuyi-assets` with the `golden` feature, a file holding `{}` under
//! its `tests/golden/`, and a test named `golden…` that checks its rows.
//! An example of this repository also adds its own step to CI's
//! `determinism` job, with its window feature off, as Hello, Grid's is.
//!
//! # A moved value is a break
//!
//! New rows are not a break. A changed or removed row is: this build no
//! longer gives what an earlier one gave, so a game's record replays
//! differently or its asset reads as another. The PR that blesses it adds
//! a `Breaking:` line naming the table to its ticket's completion notes
//! (the version policy counts it as a break; ticket 2035 makes CI's
//! `determinism` job check the line). An example's table whose example
//! changed its own files in the same PR moved with its own rules, not the
//! engine's: no line.
//!
//! No I/O and no `std`: a crate whose tests run without `std` uses it as
//! a dev-dependency with `default-features = false`.

use crate::prelude::*;

use alloc::collections::BTreeMap;
use core::fmt;

/// A golden table being checked ([module docs](self)).
#[derive(Debug, Clone)]
pub struct Golden {
    /// The table's file, as reports name it.
    path: &'static str,
    /// The table's rows, or why its text doesn't parse.
    table: Result<BTreeMap<String, String>, String>,
    /// The rows this run checked, by name.
    checked: BTreeMap<String, String>,
    /// The names checked more than once.
    twice: Vec<String>,
}

impl Golden {
    /// The table at `path` (what reports name it), from its `text`: a RON
    /// map from row name to value, both strings. A text that doesn't
    /// parse is reported by [`finish`](Self::finish), with its line.
    pub fn new(path: &'static str, text: &str) -> Self {
        let table = ::ron::from_str::<BTreeMap<String, String>>(text).map_err(|e| {
            let at = e.span.end;
            format!("{path}:{}:{}: {}", at.line, at.col, e.code)
        });
        Self {
            path,
            table,
            checked: BTreeMap::new(),
            twice: Vec::new(),
        }
    }

    /// Checks the row `name`: this run gives `value` for it. A name
    /// checked twice is a problem [`finish`](Self::finish) reports; the
    /// first value stands.
    pub fn check(&mut self, name: impl Into<String>, value: impl fmt::Display) {
        let name = name.into();
        if self.checked.contains_key(&name) {
            self.twice.push(name);
        } else {
            self.checked.insert(name, value.to_string());
        }
    }

    /// Passes when every row checked equals the table's and every row of
    /// the table was checked; otherwise the [`GoldenReport`] of what
    /// differs.
    pub fn finish(self) -> Result<(), GoldenReport> {
        let (table, unreadable) = match self.table {
            Ok(table) => (table, None),
            Err(why) => (BTreeMap::new(), Some(why)),
        };
        let mut changed = BTreeMap::new();
        let mut added = BTreeMap::new();
        for (name, new) in &self.checked {
            match table.get(name) {
                Some(old) if old == new => {}
                Some(old) => {
                    changed.insert(name.clone(), (old.clone(), new.clone()));
                }
                None => {
                    added.insert(name.clone(), new.clone());
                }
            }
        }
        let removed: BTreeMap<String, String> = table
            .into_iter()
            .filter(|(name, _)| !self.checked.contains_key(name))
            .collect();
        if changed.is_empty()
            && added.is_empty()
            && removed.is_empty()
            && self.twice.is_empty()
            && unreadable.is_none()
        {
            return Ok(());
        }
        Err(GoldenReport {
            path: self.path,
            changed,
            added,
            removed,
            twice: self.twice,
            unreadable,
            table: table_text(&self.checked),
        })
    }

    /// [`finish`](Self::finish), panicking with the report if it fails:
    /// what a test calls.
    pub fn assert(self) {
        if let Err(report) = self.finish() {
            panic!("{report}");
        }
    }
}

/// How a golden table differs from what this run gives ([`Golden::finish`]).
/// Its `Display` is for a person or an agent reading a failed test: the
/// rows, what to do, and the new table between the markers `cargo xtask
/// golden --bless` reads.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct GoldenReport {
    /// The table's file.
    pub path: &'static str,
    /// The rows whose value changed: the table's, then this run's.
    pub changed: BTreeMap<String, (String, String)>,
    /// The rows this run checked that the table lacks, with their values.
    pub added: BTreeMap<String, String>,
    /// The rows of the table this run didn't check, with their values.
    pub removed: BTreeMap<String, String>,
    /// The names checked more than once, in the order the repeats came.
    pub twice: Vec<String>,
    /// Why the table's text doesn't parse, if it doesn't (its rows then
    /// all count as added).
    pub unreadable: Option<String>,
    /// The table this run gives ([`table_text`] of the rows it checked).
    pub table: String,
}

/// The line before a report's new table.
pub const TABLE_START: &str = "--- new golden table: ";
/// The end of [`TABLE_START`]'s line.
pub const TABLE_START_END: &str = " ---";
/// The line after a report's new table.
pub const TABLE_END: &str = "--- end of golden table ---";

impl fmt::Display for GoldenReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "golden table {}: {} changed, {} added, {} removed",
            self.path,
            self.changed.len(),
            self.added.len(),
            self.removed.len()
        )?;
        if let Some(why) = &self.unreadable {
            writeln!(f, "  the table doesn't parse: {why}")?;
        }
        for (name, (old, new)) in &self.changed {
            writeln!(f, "  changed {name:?}: {old} -> {new}")?;
        }
        for (name, value) in &self.added {
            writeln!(f, "  added {name:?}: {value}")?;
        }
        for (name, value) in &self.removed {
            writeln!(f, "  removed {name:?}: {value}")?;
        }
        for name in &self.twice {
            writeln!(f, "  checked twice {name:?}")?;
        }
        writeln!(f)?;
        if !self.twice.is_empty() {
            writeln!(
                f,
                "A row's name is checked twice: give each check a name of its own."
            )?;
        }
        if !self.changed.is_empty() || !self.removed.is_empty() {
            writeln!(
                f,
                "This build no longer gives what an earlier one gave. If you meant it, bless \
                 the table with `cargo xtask golden --bless` and add a `Breaking:` line naming \
                 it to the ticket's completion notes. If not, find what moved it."
            )?;
        } else if self.unreadable.is_some() {
            writeln!(
                f,
                "Write the table again with `cargo xtask golden --bless`, or mend its text."
            )?;
        } else if !self.added.is_empty() {
            writeln!(
                f,
                "Bless the table with `cargo xtask golden --bless`; new rows are not a break."
            )?;
        }
        writeln!(f)?;
        writeln!(f, "{TABLE_START}{}{TABLE_START_END}", self.path)?;
        write!(f, "{}", self.table)?;
        write!(f, "{TABLE_END}")
    }
}

/// The text of a table of `rows`, the one way a table is written: `{`,
/// one `"name": "value",` line per row in name order, `}`, each line
/// ending in `\n`; names and values written as RON strings. No rows is
/// `{}`.
pub fn table_text(rows: &BTreeMap<String, String>) -> String {
    if rows.is_empty() {
        return "{}\n".to_owned();
    }
    let mut text = String::from("{\n");
    for (name, value) in rows {
        text.push_str("    ");
        text.push_str(&ron_string(name));
        text.push_str(": ");
        text.push_str(&ron_string(value));
        text.push_str(",\n");
    }
    text.push_str("}\n");
    text
}

/// `s` as a RON string, quoted and escaped.
fn ron_string(s: &str) -> String {
    // A `&str` always serializes.
    ::ron::to_string(s).unwrap_or_default()
}

/// The golden table at `$path`, under the calling crate's folder
/// (`CARGO_MANIFEST_DIR`), read at compile time with `include_str!`: a
/// [`Golden`](crate::golden::Golden) to check rows against. The file must
/// exist; a new table starts as `{}` ([module docs](crate::golden)).
///
/// ```ignore
/// let mut golden = shuyi_assets::golden!("tests/golden/formats.ron");
/// ```
#[macro_export]
macro_rules! golden {
    ($path:literal) => {
        $crate::golden::Golden::new(
            concat!(env!("CARGO_MANIFEST_DIR"), "/", $path),
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/", $path)),
        )
    };
}

#[cfg(test)]
mod tests;
