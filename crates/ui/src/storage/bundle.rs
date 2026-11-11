//! Save bundles: some of a [`Storage`]'s entries written as one text file
//! the player keeps, and read back with a verdict per entry before
//! anything is written.
//!
//! - [`write_bundle`] packs the keys a filter keeps into a [`SaveBundle`]'s
//!   text: RON, its entries in key order, so the same storage gives the
//!   same text.
//! - [`plan_import`] reads that text back into an [`ImportPlan`]: each
//!   entry with the [`Verdict`] the game's [`EntryCheck`] gives it
//!   ([`FormatCheck`] for a save of one [`SaveFormat`], [`Checks`] for
//!   several), whether it would replace a key the storage holds, and how
//!   the bundle's stamp differs from this build's ([`Made::check`]).
//! - [`apply`] writes the [`Verdict::Ready`] entries the game takes, all
//!   or nothing: on an error, and after a power cut once [`recover`] has
//!   run, the storage holds every old value or every new one.
//!
//! Getting the text to the player and back is the platform's
//! (`Ctx::offer_file`, `Ctx::ask_for_file`); which keys a game bundles,
//! and what it shows the player, are the game's. As a format (ADR-0007):
//! [`SaveBundleRon`].
//!
//! Keys starting with [`RESERVED_PREFIX`] are the bundle's own (the
//! journal [`apply`] keeps while it writes, and the old values it backs
//! up): [`write_bundle`] leaves them out, and a bundle that names one is
//! refused.

use crate::prelude::*;
use alloc::collections::{BTreeMap, BTreeSet};
use core::fmt;
use core::marker::PhantomData;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use shuyi_assets::error::codes;
use shuyi_assets::format::{AssetKind, AssetPath, DecodeCx, Encode, Format, TEXT_MAX_BYTES, text};
use shuyi_assets::{ContentError, line_of};
use shuyi_core::made::{Made, Mismatch};
use shuyi_core::save::{SaveFormat, SaveHeader};

use super::{Storage, StorageError, is_valid_key};

/// The version of the bundle format this build reads and writes.
pub const SAVE_BUNDLE_VERSION: u32 = 1;

/// Keys starting with it are the bundle's own, never a game's: see the
/// [module docs](self).
pub const RESERVED_PREFIX: &str = "_bundle-";

/// The journal [`apply`] keeps while it writes.
const JOURNAL_KEY: &str = "_bundle-journal";

/// The old values [`apply`] backs up start with it.
const BACKUP_PREFIX: &str = "_bundle-backup-";

/// Whether `key` is the bundle's own ([`RESERVED_PREFIX`]).
fn is_reserved(key: &str) -> bool {
    key.starts_with(RESERVED_PREFIX)
}

/// A save bundle: some of a storage's entries, the bundle format's
/// version, and the stamp of the build that wrote it. Written by
/// [`write_bundle`] (or [`to_text`](Self::to_text)), read by
/// [`plan_import`] (or [`read_bundle`]).
///
/// ```ron
/// SaveBundle(
///     version: 1,
///     made: Made(
///         engine: "0.1.0",
///         game: "demo 1.0",
///         content: "1a2b3c4d5e6f7a8b",
///     ),
///     entries: {
///         "garden": "(version:1,garden:(rows:6))",
///     },
/// )
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
// A file's contents: built with `SaveBundle::new`, it may grow (ADR-0039).
#[non_exhaustive]
pub struct SaveBundle {
    /// The bundle format's version ([`SAVE_BUNDLE_VERSION`]).
    pub version: u32,
    /// What made it.
    pub made: Made,
    /// The entries, by storage key.
    pub entries: BTreeMap<String, String>,
}

impl SaveBundle {
    /// A bundle of this version holding `entries`, made by `made`.
    pub fn new(made: Made, entries: BTreeMap<String, String>) -> Self {
        Self {
            version: SAVE_BUNDLE_VERSION,
            made,
            entries,
        }
    }

    /// Its text: RON, with its type's names, four spaces to a level, `\n`
    /// line ends, its entries in key order. The same bundle always gives
    /// the same text.
    pub fn to_text(&self) -> String {
        let config = ron::ser::PrettyConfig::new()
            .struct_names(true)
            .new_line("\n")
            .indentor("    ");
        // Strings, a number and a map of strings always serialize.
        let mut text = ron::ser::to_string_pretty(self, config).unwrap_or_default();
        text.push('\n');
        text
    }
}

impl SaveFormat for SaveBundle {
    const NAME: &'static str = "SaveBundle";
    const VERSION: u32 = SAVE_BUNDLE_VERSION;
}

impl AssetKind for SaveBundle {
    const NAME: &'static str = "save bundle";
}

/// Why a text isn't a bundle this build reads ([`read_bundle`],
/// [`plan_import`]).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum BundleError {
    /// The text isn't a save bundle: its syntax, or a field of the
    /// bundle, is wrong.
    NotABundle {
        /// The 1-based line of the problem.
        line: u32,
        /// The 1-based column of the problem.
        column: u32,
        /// What is wrong.
        message: String,
    },
    /// A bundle of another version of the bundle format.
    Version {
        /// The version the bundle names.
        found: u32,
        /// The version this build reads ([`SAVE_BUNDLE_VERSION`]).
        reads: u32,
    },
    /// An entry's key isn't a valid storage key
    /// ([`is_valid_key`]), or is one of the bundle's
    /// own ([`RESERVED_PREFIX`]).
    BadKey(String),
    /// The storage failed while the import was planned.
    Storage(StorageError),
}

// check-text: not player text (for the log)
impl fmt::Display for BundleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotABundle {
                line,
                column,
                message,
            } => write!(f, "not a save bundle: {line}:{column}: {message}"),
            Self::Version { found, reads } => write!(
                f,
                "a save bundle of version {found}; this build reads version {reads}"
            ),
            Self::BadKey(key) => write!(f, "a save bundle's entry has the bad key {key:?}"),
            Self::Storage(e) => write!(f, "{e}"),
        }
    }
}

impl From<StorageError> for BundleError {
    fn from(e: StorageError) -> Self {
        Self::Storage(e)
    }
}

/// `e`, from RON, as [`BundleError::NotABundle`].
fn not_a_bundle(e: &ron::error::SpannedError) -> BundleError {
    let at = e.span.end;
    BundleError::NotABundle {
        line: u32::try_from(at.line).unwrap_or(u32::MAX),
        column: u32::try_from(at.col).unwrap_or(u32::MAX),
        message: e.code.to_string(),
    }
}

/// The bundle in `text`: its version read first, so a bundle of another
/// version is told from a broken one, then the rest, then each key
/// checked.
pub fn read_bundle(text: &str) -> Result<SaveBundle, BundleError> {
    let header: SaveHeader<SaveBundle> = ron::from_str(text).map_err(|e| not_a_bundle(&e))?;
    if !header.is_current() {
        return Err(BundleError::Version {
            found: header.version,
            reads: SAVE_BUNDLE_VERSION,
        });
    }
    let bundle: SaveBundle = ron::from_str(text).map_err(|e| not_a_bundle(&e))?;
    if let Some(key) = bundle
        .entries
        .keys()
        .find(|key| !is_valid_key(key) || is_reserved(key))
    {
        return Err(BundleError::BadKey(key.clone()));
    }
    Ok(bundle)
}

/// The text of a bundle of every key of `storage` that `keep` keeps (and
/// none of the bundle's own), stamped `made`: [`SaveBundle::to_text`], so
/// the same storage gives the same text. A key listed but gone by the
/// time it is read is left out.
pub fn write_bundle(
    storage: &dyn Storage,
    made: &Made,
    keep: impl Fn(&str) -> bool,
) -> Result<String, StorageError> {
    let mut entries = BTreeMap::new();
    for key in storage.list()? {
        if is_reserved(&key) || !keep(&key) {
            continue;
        }
        if let Some(value) = storage.read(&key)? {
            entries.insert(key, value);
        }
    }
    Ok(SaveBundle::new(made.clone(), entries).to_text())
}

/// What this build can do with one entry of a bundle ([`EntryCheck`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Verdict {
    /// This build reads it: [`apply`] may write it.
    Ready,
    /// A save of another version of its format.
    Incompatible {
        /// The version the save names.
        found: u32,
        /// The version this build reads.
        wants: u32,
    },
    /// A save of a format the game knows, which doesn't read.
    Unreadable,
    /// A key the game doesn't know.
    Unknown,
}

/// Gives each entry of a bundle its [`Verdict`]: the game's knowledge of
/// its own keys and formats. [`Verdict::Unknown`] for a key it doesn't
/// know. A closure `|key, text| …` is one.
pub trait EntryCheck {
    /// The verdict on the entry `key`, holding `text`.
    fn check(&self, key: &str, text: &str) -> Verdict;
}

impl<F: Fn(&str, &str) -> Verdict> EntryCheck for F {
    fn check(&self, key: &str, text: &str) -> Verdict {
        self(key, text)
    }
}

/// An [`EntryCheck`] for the keys a matcher picks, each a save of format
/// `F`: [`Verdict::Unknown`] for another key; [`Verdict::Unreadable`] for
/// a text whose version doesn't read ([`SaveHeader`]);
/// [`Verdict::Incompatible`] for another version than
/// [`F::VERSION`](SaveFormat::VERSION); [`Verdict::Unreadable`] for one
/// of that version that doesn't read as `F`; else [`Verdict::Ready`].
pub struct FormatCheck<F> {
    matches: Box<dyn Fn(&str) -> bool>,
    format: PhantomData<fn() -> F>,
}

impl<F: SaveFormat + DeserializeOwned> FormatCheck<F> {
    /// The check of the keys `matches` picks, as saves of format `F`.
    pub fn new(matches: impl Fn(&str) -> bool + 'static) -> Self {
        Self {
            matches: Box::new(matches),
            format: PhantomData,
        }
    }
}

impl<F> fmt::Debug for FormatCheck<F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FormatCheck")
            .field("format", &core::any::type_name::<F>())
            .finish_non_exhaustive()
    }
}

impl<F: SaveFormat + DeserializeOwned> EntryCheck for FormatCheck<F> {
    fn check(&self, key: &str, text: &str) -> Verdict {
        if !(self.matches)(key) {
            return Verdict::Unknown;
        }
        let Ok(header) = ron::from_str::<SaveHeader<F>>(text) else {
            return Verdict::Unreadable;
        };
        if !header.is_current() {
            return Verdict::Incompatible {
                found: header.version,
                wants: F::VERSION,
            };
        }
        if ron::from_str::<F>(text).is_ok() {
            Verdict::Ready
        } else {
            Verdict::Unreadable
        }
    }
}

/// Several [`EntryCheck`]s as one: an entry's verdict is the first that
/// isn't [`Verdict::Unknown`], and [`Verdict::Unknown`] if all are.
#[derive(Default)]
pub struct Checks {
    checks: Vec<Box<dyn EntryCheck>>,
}

impl Checks {
    /// No checks yet: every entry is [`Verdict::Unknown`].
    pub fn new() -> Self {
        Self::default()
    }

    /// These checks, then `check`.
    #[must_use]
    pub fn with(mut self, check: impl EntryCheck + 'static) -> Self {
        self.checks.push(Box::new(check));
        self
    }
}

impl fmt::Debug for Checks {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Checks")
            .field("checks", &self.checks.len())
            .finish()
    }
}

impl EntryCheck for Checks {
    fn check(&self, key: &str, text: &str) -> Verdict {
        self.checks
            .iter()
            .map(|check| check.check(key, text))
            .find(|verdict| *verdict != Verdict::Unknown)
            .unwrap_or(Verdict::Unknown)
    }
}

/// One entry of an [`ImportPlan`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct PlanEntry {
    /// Its storage key.
    pub key: String,
    /// What this build can do with it.
    pub verdict: Verdict,
    /// Whether the storage held the key when the import was planned.
    pub replaces: bool,
    /// Its text, as the bundle holds it.
    pub text: String,
}

/// What importing a bundle would do ([`plan_import`]), before anything is
/// written: [`apply`] writes it.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ImportPlan {
    /// What made the bundle.
    pub made: Made,
    /// How that differs from this build, if it does. The verdicts decide
    /// what can be written; what to tell the player is the game's.
    pub mismatch: Option<Mismatch>,
    /// Every entry, in key order.
    pub entries: Vec<PlanEntry>,
}

impl ImportPlan {
    /// The entry of `key`, if the bundle has one.
    pub fn entry(&self, key: &str) -> Option<&PlanEntry> {
        self.entries.iter().find(|entry| entry.key == key)
    }

    /// The entries [`apply`] can write: those [`Verdict::Ready`].
    pub fn ready(&self) -> impl Iterator<Item = &PlanEntry> {
        self.entries
            .iter()
            .filter(|entry| entry.verdict == Verdict::Ready)
    }
}

/// What importing the bundle in `text` into `storage` would do: each
/// entry with `check`'s verdict and whether `storage` holds its key, and
/// how the bundle's stamp differs from `now`, this build's. Writes
/// nothing.
pub fn plan_import(
    text: &str,
    storage: &dyn Storage,
    check: &dyn EntryCheck,
    now: &Made,
) -> Result<ImportPlan, BundleError> {
    let bundle = read_bundle(text)?;
    let held: BTreeSet<String> = storage.list()?.into_iter().collect();
    let entries = bundle
        .entries
        .into_iter()
        .map(|(key, text)| PlanEntry {
            verdict: check.check(&key, &text),
            replaces: held.contains(&key),
            key,
            text,
        })
        .collect();
    Ok(ImportPlan {
        mismatch: bundle.made.check(now).err(),
        made: bundle.made,
        entries,
    })
}

/// What [`apply`] wrote.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct Applied {
    /// The keys written, in key order.
    pub written: Vec<String>,
    /// Those of them that held a value before.
    pub replaced: Vec<String>,
}

/// Writes the entries of `plan` that are [`Verdict::Ready`] and that
/// `take` takes, all or nothing.
///
/// It first finishes an import a power cut interrupted ([`recover`]).
/// Then it backs up the old value of each key it will write, writes a
/// journal of them, writes the new values, and deletes the journal, the
/// moment the import is done; then the backups. On an error it puts the
/// old values back and gives the error; after a power cut, [`recover`]
/// does. Either way the storage holds every old value or every new one.
pub fn apply(
    plan: &ImportPlan,
    storage: &mut dyn Storage,
    take: impl Fn(&PlanEntry) -> bool,
) -> Result<Applied, StorageError> {
    recover(storage)?;
    let chosen: Vec<&PlanEntry> = plan.ready().filter(|entry| take(entry)).collect();
    if chosen.is_empty() {
        return Ok(Applied::default());
    }
    match write_all(storage, &chosen) {
        Ok(applied) => Ok(applied),
        Err(e) => {
            // The next start's `recover` finishes it if this fails too.
            let _ = recover(storage);
            Err(e)
        }
    }
}

/// [`apply`]'s writes: the backups, the journal, the new values, then the
/// journal deleted. The backups are deleted after, at best: once the
/// journal is gone, the new values stand, and [`recover`] deletes any
/// backup left.
fn write_all(storage: &mut dyn Storage, chosen: &[&PlanEntry]) -> Result<Applied, StorageError> {
    let mut journal = Vec::with_capacity(chosen.len());
    for (i, entry) in chosen.iter().enumerate() {
        let backup = match storage.read(&entry.key)? {
            Some(old) => {
                let backup = format!("{BACKUP_PREFIX}{i}");
                storage.write(&backup, &old)?;
                Some(backup)
            }
            None => None,
        };
        journal.push((entry.key.clone(), backup));
    }
    // A list of strings always serializes.
    let text = ron::to_string(&journal).unwrap_or_default();
    storage.write(JOURNAL_KEY, &text)?;
    for entry in chosen {
        storage.write(&entry.key, &entry.text)?;
    }
    storage.delete(JOURNAL_KEY)?;
    let mut applied = Applied::default();
    for (key, backup) in journal {
        if let Some(backup) = backup {
            let _ = storage.delete(&backup);
            applied.replaced.push(key.clone());
        }
        applied.written.push(key);
    }
    Ok(applied)
}

/// Finishes an [`apply`] that a power cut (or a storage that failed, then
/// failed again) interrupted: if its journal is there, puts each key's
/// old value back (or deletes a key that had none), then deletes the
/// journal and every backup left. `Ok(true)` if it put old values back.
///
/// A game calls it at start, before it reads its saves, on each storage it
/// imports into; [`apply`] calls it first too.
pub fn recover(storage: &mut dyn Storage) -> Result<bool, StorageError> {
    let undone = match storage.read(JOURNAL_KEY)? {
        Some(text) => {
            // A storage that keeps the power-cut suite's promises never
            // tears the journal; one that did can't be undone.
            let journal: Vec<(String, Option<String>)> = ron::from_str(&text).unwrap_or_default();
            for (key, backup) in journal {
                let old = match &backup {
                    Some(backup) => storage.read(backup)?,
                    None => None,
                };
                match (backup, old) {
                    (None, _) => storage.delete(&key)?,
                    (Some(_), Some(old)) => storage.write(&key, &old)?,
                    // A backup the journal names is deleted only after the
                    // journal: its loss leaves the key as it is.
                    (Some(_), None) => {}
                }
            }
            storage.delete(JOURNAL_KEY)?;
            true
        }
        None => false,
    };
    for key in storage.list()? {
        if key.starts_with(BACKUP_PREFIX) {
            storage.delete(&key)?;
        }
    }
    Ok(undone)
}

/// A save bundle file (`save-bundle`, `.ron`), read as [`read_bundle`]
/// says, its errors with their line where it is known.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SaveBundleRon;

impl Format for SaveBundleRon {
    type Asset = SaveBundle;

    fn id(&self) -> &'static str {
        "save-bundle"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["ron"]
    }

    fn max_bytes(&self) -> usize {
        TEXT_MAX_BYTES
    }

    fn decode(
        &self,
        file: &AssetPath,
        bytes: &[u8],
        _cx: &DecodeCx<'_>,
    ) -> Result<SaveBundle, Vec<ContentError>> {
        let source = text(file, bytes)?;
        read_bundle(source).map_err(|e| vec![content_error(file.shown(), source, &e)])
    }
}

/// `e`, of the bundle file `shown` holding `source`, as a content error:
/// with its line and column, or the line of what it names.
fn content_error(shown: &str, source: &str, e: &BundleError) -> ContentError {
    let (needle, code) = match e {
        BundleError::NotABundle {
            line,
            column,
            message,
        } => {
            return ContentError::new(shown, message.clone())
                .at(*line, Some(*column))
                .with_code(codes::PARSE);
        }
        BundleError::Version { .. } => ("version".to_owned(), codes::OUT_OF_RANGE),
        BundleError::BadKey(key) => (ron::to_string(key).unwrap_or_default(), codes::UNKNOWN_NAME),
        // `read_bundle` reads no storage.
        BundleError::Storage(_) => (String::new(), codes::PARSE),
    };
    let error = ContentError::new(shown, e.to_string()).with_code(code);
    match line_of(source, &needle) {
        Some(line) if !needle.is_empty() => error.at(line, None),
        _ => error,
    }
}

impl Encode for SaveBundleRon {
    /// [`SaveBundle::to_text`].
    fn encode(&self, bundle: &SaveBundle) -> Vec<u8> {
        bundle.to_text().into_bytes()
    }
}

#[cfg(test)]
mod tests;
