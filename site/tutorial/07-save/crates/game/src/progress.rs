//! The player's progress, kept in their storage: the furthest level
//! reached, with the version of its format.

// ANCHOR: uses
use serde::{Deserialize, Serialize};
use shuyi::core::{SaveFormat, SaveHeader};
use shuyi::ui::storage::{Storage, StorageError};
// ANCHOR_END: uses

// ANCHOR: progress
/// What is kept: the version of the format, and the furthest level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Progress {
    /// The version of the format that wrote it.
    pub version: u32,
    /// The furthest level reached, counted from 0.
    pub furthest: usize,
}

impl SaveFormat for Progress {
    const NAME: &'static str = "Progress";
    /// Raised whenever this type changes shape or meaning.
    const VERSION: u32 = 1;
}

/// Where it is kept, in the player's profile.
// check-text: not player text, a storage key
pub const KEY: &str = "progress";

/// The furthest level reached: the first, if nothing readable is kept.
pub fn furthest(storage: &dyn Storage) -> usize {
    let Ok(Some(text)) = storage.read(KEY) else {
        return 0;
    };
    match ron::from_str::<SaveHeader<Progress>>(&text) {
        Ok(header) if header.is_current() => {
            ron::from_str::<Progress>(&text).map_or(0, |progress| progress.furthest)
        }
        _ => 0,
    }
}

/// Keeps `level` as the furthest reached, if it is further.
pub fn reached(storage: &mut dyn Storage, level: usize) -> Result<(), StorageError> {
    if level <= furthest(storage) {
        return Ok(());
    }
    let progress = Progress {
        version: Progress::VERSION,
        furthest: level,
    };
    let text = ron::to_string(&progress).map_err(|e| StorageError::Backend(e.to_string()))?;
    storage.write(KEY, &text)
}
// ANCHOR_END: progress
