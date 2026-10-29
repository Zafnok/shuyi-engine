//! Chapter 7, saves and settings: a garden saved through the player's
//! storage with the version of its format, read back only by a build of
//! that version, and the player's settings changed and kept.

use serde::{Deserialize, Serialize};
use shuyi::core::{SaveFormat, SaveHeader};
use shuyi::ui::storage::{Storage, StorageError};

use crate::rules::Garden;

// ANCHOR: save
/// A save: the version of its format, and the garden.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Save {
    /// The version of the format that wrote it.
    pub version: u32,
    /// The game in progress.
    pub garden: Garden,
}

impl SaveFormat for Save {
    const NAME: &'static str = "Save";
    /// Raised whenever a saved type changes shape or meaning.
    const VERSION: u32 = 1;
}

/// Where the save is kept, in the player's profile.
// check-text: not player text, a storage key
pub const SAVE_KEY: &str = "garden";

/// Why a save didn't load.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadError {
    /// The storage failed.
    Storage(StorageError),
    /// Another version of the format wrote it.
    Version(u32),
    /// It doesn't read.
    Damaged(String),
}

/// Writes the garden under [`SAVE_KEY`].
pub fn save(storage: &mut dyn Storage, garden: &Garden) -> Result<(), StorageError> {
    let save = Save {
        version: Save::VERSION,
        garden: garden.clone(),
    };
    let text = ron::to_string(&save).map_err(|e| StorageError::Backend(e.to_string()))?;
    storage.write(SAVE_KEY, &text)
}

/// The saved garden, if there is one this build can read.
pub fn load(storage: &dyn Storage) -> Result<Option<Garden>, LoadError> {
    let Some(text) = storage.read(SAVE_KEY).map_err(LoadError::Storage)? else {
        return Ok(None);
    };
    // The version alone first, so another version is told from damage.
    let header: SaveHeader<Save> =
        ron::from_str(&text).map_err(|e| LoadError::Damaged(e.to_string()))?;
    if !header.is_current() {
        return Err(LoadError::Version(header.version));
    }
    let save: Save = ron::from_str(&text).map_err(|e| LoadError::Damaged(e.to_string()))?;
    Ok(Some(save.garden))
}
// ANCHOR_END: save

#[cfg(test)]
mod tests {
    use shuyi::ui::settings::SETTINGS_KEY;

    use super::*;
    use crate::assets::Plan;
    use crate::game;

    // ANCHOR: test_save
    #[test]
    fn a_save_comes_back_from_the_players_storage() {
        let mut ctx = game().ctx().unwrap();
        let garden = Garden::new(&Plan::new(6, 9, 75), 3);
        save(&mut ctx.storage, &garden).unwrap();
        assert_eq!(load(&ctx.storage), Ok(Some(garden)));
    }
    // ANCHOR_END: test_save

    #[test]
    fn a_save_of_another_version_is_refused_not_read() {
        let mut ctx = game().ctx().unwrap();
        ctx.storage
            .write(SAVE_KEY, "(version: 9, garden: ())")
            .unwrap();
        assert_eq!(load(&ctx.storage), Err(LoadError::Version(9)));
        ctx.storage.write(SAVE_KEY, "(garden").unwrap();
        assert!(matches!(load(&ctx.storage), Err(LoadError::Damaged(_))));
        ctx.storage.delete(SAVE_KEY).unwrap();
        assert_eq!(load(&ctx.storage), Ok(None));
    }

    // ANCHOR: settings
    #[test]
    fn a_changed_setting_is_kept_in_the_players_profile() {
        let mut ctx = game().ctx().unwrap();
        ctx.change_settings(|settings| settings.music_volume = 40)
            .unwrap();
        assert_eq!(ctx.settings().music_volume, 40);
        assert!(ctx.storage.read(SETTINGS_KEY).unwrap().is_some());
    }
    // ANCHOR_END: settings
}
