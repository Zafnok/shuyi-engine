//! `--stale-hashes`: the older hashes of a crate in a profile folder's
//! `deps/` that no build read in days (ticket 2092, ADR-0121).
//!
//! cargo names each unit's output `<crate>-<16 hex digits><kind>`, and one
//! crate can sit there under several hashes: most live (features, triples,
//! Clippy), some left by a build that changed them. cargo never rewrites a
//! Fresh dependency's files, so only the last access tells the stale ones,
//! and only where the system updates it ([`last_access_updates`]).

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::SystemTime;

use super::is_link;

/// How many hex digits cargo's hash in a file name has.
const HASH_DIGITS: usize = 16;

/// `name`'s crate and kind, the hash between them taken out:
/// `libfoo-0123456789abcdef.rlib` is (`libfoo`, `.rlib`). The hash is the
/// first `-` followed by exactly 16 hex digits and then a `.` or the end,
/// so `my-crate-…` keeps its dash. `None` when there is none.
pub(super) fn split_hash(name: &str) -> Option<(&str, &str)> {
    let bytes = name.as_bytes();
    bytes
        .iter()
        .enumerate()
        .filter(|&(_, &b)| b == b'-')
        .find_map(|(dash, _)| {
            let start = dash + 1;
            let end = start + HASH_DIGITS;
            let digits = bytes.get(start..end)?;
            let after = bytes.get(end).copied();
            (digits.iter().all(u8::is_ascii_hexdigit) && matches!(after, None | Some(b'.')))
                .then(|| (&name[..dash], &name[end..]))
        })
}

/// A file of `deps/` the trim would delete, with its length.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Stale {
    pub path: PathBuf,
    pub len: u64,
}

/// The stale files of `deps`: in each group of files of one crate and
/// kind under more than one hash, those last accessed before `cutoff`,
/// never the most recently accessed one. Only files directly in `deps`,
/// never a folder or a link; a file whose last access can't be read is
/// kept.
pub(super) fn stale_files(deps: &Path, cutoff: SystemTime) -> Vec<Stale> {
    let Ok(entries) = fs::read_dir(deps) else {
        return Vec::new();
    };
    let mut groups: BTreeMap<(String, String), Vec<(SystemTime, Stale)>> = BTreeMap::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        let Some((krate, kind)) = split_hash(name) else {
            continue;
        };
        let Ok(meta) = fs::symlink_metadata(&path) else {
            continue;
        };
        if !meta.is_file() || is_link(&meta) {
            continue;
        }
        let Ok(accessed) = meta.accessed() else {
            continue;
        };
        let key = (krate.to_owned(), kind.to_owned());
        groups.entry(key).or_default().push((
            accessed,
            Stale {
                len: meta.len(),
                path,
            },
        ));
    }
    let mut stale = Vec::new();
    for mut group in groups.into_values().filter(|group| group.len() > 1) {
        group.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.path.cmp(&b.1.path)));
        stale.extend(
            group
                .into_iter()
                .skip(1)
                .filter(|(accessed, _)| *accessed < cutoff)
                .map(|(_, file)| file),
        );
    }
    stale.sort_by(|a, b| a.path.cmp(&b.path));
    stale
}

/// Whether the system updates files' last access, so that
/// [`stale_files`] can be trusted: `Ok(())`, or why not, for the run's
/// notes. On Windows, from `fsutil behavior query disablelastaccess`;
/// elsewhere assumed on (Linux's `relatime` updates it at least daily).
pub(super) fn last_access_updates() -> Result<(), String> {
    if !cfg!(windows) {
        return Ok(());
    }
    let output = Command::new("fsutil")
        .args(["behavior", "query", "disablelastaccess"])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .map_err(|e| format!("fsutil did not start ({e})"))?;
    if !output.status.success() {
        return Err(format!("fsutil failed ({})", output.status));
    }
    fsutil_answer(&String::from_utf8_lossy(&output.stdout))
}

/// What `fsutil behavior query disablelastaccess` said: its line reads
/// `DisableLastAccess = 2  (System Managed, Last Access Time Updates
/// ENABLED)`. The words are trusted when there; otherwise the value's low
/// bit (0 and 2 on, 1 and 3 off).
pub(super) fn fsutil_answer(text: &str) -> Result<(), String> {
    const OFF: &str = "last-access updates are off (fsutil behavior query disablelastaccess)";
    let upper = text.to_ascii_uppercase();
    if upper.contains("UPDATES DISABLED") {
        return Err(OFF.to_owned());
    }
    if upper.contains("UPDATES ENABLED") {
        return Ok(());
    }
    let value = text
        .lines()
        .find_map(|line| line.split_once("DisableLastAccess")?.1.split_once('='))
        .and_then(|(_, rest)| {
            let word = rest.split_whitespace().next()?;
            match word.strip_prefix("0x") {
                Some(hex) => u32::from_str_radix(hex, 16).ok(),
                None => word.parse().ok(),
            }
        });
    match value {
        Some(value) if value & 1 == 0 => Ok(()),
        Some(_) => Err(OFF.to_owned()),
        None => Err(format!(
            "fsutil's answer didn't say whether last-access updates are on ({})",
            text.trim()
        )),
    }
}

#[cfg(test)]
mod tests;
