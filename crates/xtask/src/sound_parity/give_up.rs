//! `cargo xtask sound-parity give-up`: takes everything of the sound
//! replacement out of the repository, at any stage until quad-snd is removed
//! (ADR-0068). Only Nick decides to: the `give-up-sound-replacement` skill
//! says what else that session does.
//!
//! `tools/sound-parity/footprint.ron` lists what the chain added. Without
//! `--apply` this only checks that the list still covers everything, which a
//! test does on every run, and CI's `sound-give-up` job does it for real.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::Command;

use serde::Deserialize;

use super::read_ron;

/// The words only the chain's code uses.
const WORDS: &str = "sound-parity|sound_parity|sound_probe|sound-probe|SoundProbeBackend|in-house-sound|shuyi-sound|shuyi_sound";
/// What may keep naming the chain afterwards: the record of what happened.
const HISTORY: [&str; 5] = [
    "tickets/",
    "docs/adr/",
    "docs/research/",
    "docs/decisions.md",
    "docs/ROADMAP.md",
];

/// `tools/sound-parity/footprint.ron`.
#[derive(Debug, Deserialize)]
struct Footprint {
    owned: Vec<String>,
    edits: Vec<Edit>,
}

/// One change to a file the chain shares with the rest of the engine.
#[derive(Debug, Deserialize)]
pub(crate) enum Edit {
    Replace {
        file: String,
        find: String,
        with: String,
    },
    Cut {
        file: String,
        from: String,
        to: String,
    },
}

impl Edit {
    fn file(&self) -> &str {
        match self {
            Self::Replace { file, .. } | Self::Cut { file, .. } => file,
        }
    }

    /// `text` with this edit made, or why it can't be: what it looks for must
    /// be there exactly once, so a list that has drifted fails loudly.
    pub(crate) fn apply(&self, text: &str) -> Result<String, String> {
        let once = |wanted: &str| match text.matches(wanted).count() {
            1 => Ok(text.find(wanted).unwrap_or(0)),
            count => Err(format!("{wanted:?} is there {count} times, not once")),
        };
        match self {
            Self::Replace { find, with, .. } => {
                once(find)?;
                Ok(text.replacen(find.as_str(), with, 1))
            }
            Self::Cut { from, to, .. } => {
                let start = once(from)?;
                let end = once(to)?;
                if end < start {
                    return Err(format!("{to:?} comes before {from:?}"));
                }
                Ok(format!("{}{}", &text[..start], &text[end..]))
            }
        }
    }
}

/// Checks the footprint against the repository, and with `apply` takes it all
/// out: what is wrong, if anything. Nothing changes unless all of it is right.
pub(crate) fn run(root: &Path, apply: bool) -> Vec<String> {
    let footprint: Footprint = match read_ron(&root.join("tools/sound-parity/footprint.ron")) {
        Ok(footprint) => footprint,
        Err(e) => return vec![e],
    };
    let mut errors = Vec::new();
    let mut edited = BTreeMap::new();
    for edit in &footprint.edits {
        let file = edit.file().to_owned();
        let text = edited
            .remove(&file)
            .or_else(|| fs::read_to_string(root.join(&file)).ok());
        let Some(text) = text else {
            errors.push(format!("{file}: can't read it"));
            continue;
        };
        match edit.apply(&text) {
            Ok(after) => {
                edited.insert(file, after);
            }
            Err(e) => {
                errors.push(format!("{file}: {e}"));
                edited.insert(file, text);
            }
        }
    }
    for path in &footprint.owned {
        if !root.join(path).exists() {
            errors.push(format!("{path}: listed as the chain's, and not there"));
        }
    }
    match named(root) {
        Ok(files) => errors.extend(uncovered(&files, &footprint.owned, &edited)),
        Err(e) => errors.push(e),
    }
    if !apply || !errors.is_empty() {
        return errors;
    }
    for (file, text) in &edited {
        if let Err(e) = fs::write(root.join(file), text) {
            errors.push(format!("{file}: {e}"));
        }
    }
    for path in &footprint.owned {
        let full = root.join(path);
        let removed = if full.is_dir() {
            fs::remove_dir_all(&full)
        } else {
            fs::remove_file(&full)
        };
        if let Err(e) = removed {
            errors.push(format!("{path}: {e}"));
        }
    }
    errors
}

/// The files that name a word of the chain.
fn named(root: &Path) -> Result<Vec<String>, String> {
    let output = Command::new("git")
        .args(["grep", "-l", "-I", "--untracked", "-E", WORDS])
        .current_dir(root)
        .output()
        .map_err(|e| format!("git grep: {e}"))?;
    // 1 is `git grep`'s way of saying it found nothing.
    if !matches!(output.status.code(), Some(0 | 1)) {
        return Err(format!(
            "git grep: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let text = String::from_utf8_lossy(&output.stdout);
    Ok(text.lines().map(str::to_owned).collect())
}

/// Which of `files` would still name the chain once `owned` is gone and
/// `edited` is written, the record of what happened aside.
pub(crate) fn uncovered(
    files: &[String],
    owned: &[String],
    edited: &BTreeMap<String, String>,
) -> Vec<String> {
    let words: Vec<&str> = WORDS.split('|').collect();
    files
        .iter()
        .filter(|file| !HISTORY.iter().any(|kept| file.starts_with(kept)))
        .filter(|file| {
            !owned
                .iter()
                .any(|path| *file == path || file.starts_with(&format!("{path}/")))
        })
        .filter(|file| {
            edited
                .get(*file)
                .is_none_or(|text| words.iter().any(|word| text.contains(word)))
        })
        .map(|file| {
            format!(
                "{file} names the sound replacement and tools/sound-parity/footprint.ron doesn't \
                 cover it: add it, so giving up takes it out (ADR-0068)"
            )
        })
        .collect()
}
