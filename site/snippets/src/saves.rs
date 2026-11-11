//! Chapter 7, saves and settings: a garden saved through the player's
//! storage with the version of its format, read back only by a build of
//! that version, the player's settings changed and kept, and a save handed
//! to the player as a file they keep, and read back from one they pick,
//! and the saves packed in a bundle and read back with a verdict each.

use serde::{Deserialize, Serialize};
use shuyi::core::made::Made;
use shuyi::core::{SaveFormat, SaveHeader};
use shuyi::platform::{ChosenFile, SystemEvent};
use shuyi::ui::storage::bundle::{
    Applied, BundleError, FormatCheck, ImportPlan, apply, plan_import, write_bundle,
};
use shuyi::ui::storage::{Storage, StorageError};
use shuyi::ui::{Ctx, GameData};

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
    let text = save_text(garden).map_err(StorageError::Backend)?;
    storage.write(SAVE_KEY, &text)
}

/// The garden as a save's text.
pub fn save_text(garden: &Garden) -> Result<String, String> {
    let save = Save {
        version: Save::VERSION,
        garden: garden.clone(),
    };
    ron::to_string(&save).map_err(|e| e.to_string())
}

/// The saved garden, if there is one this build can read.
pub fn load(storage: &dyn Storage) -> Result<Option<Garden>, LoadError> {
    let Some(text) = storage.read(SAVE_KEY).map_err(LoadError::Storage)? else {
        return Ok(None);
    };
    load_text(&text).map(Some)
}

/// The garden in a save's text, if this build can read it.
pub fn load_text(text: &str) -> Result<Garden, LoadError> {
    // The version alone first, so another version is told from damage.
    let header: SaveHeader<Save> =
        ron::from_str(text).map_err(|e| LoadError::Damaged(e.to_string()))?;
    if !header.is_current() {
        return Err(LoadError::Version(header.version));
    }
    let save: Save = ron::from_str(text).map_err(|e| LoadError::Damaged(e.to_string()))?;
    Ok(save.garden)
}
// ANCHOR_END: save

// ANCHOR: files
/// Hands the player their garden as a file they keep, after this frame: a
/// download on the web, a file in the game's `exports` folder on a desktop.
pub fn back_up<G: GameData>(ctx: &mut Ctx<G>, garden: &Garden) -> Result<(), String> {
    ctx.offer_file("garden.ron", save_text(garden)?);
    Ok(())
}

/// The garden in the file the player picked, when the game's system hook
/// hears one: `None` for any other event.
pub fn restore(event: &SystemEvent) -> Option<Result<Garden, LoadError>> {
    let SystemEvent::FileChosen(ChosenFile::Picked { text, .. }) = event else {
        return None;
    };
    Some(load_text(text))
}
// ANCHOR_END: files

// ANCHOR: bundle
/// The player's saves as one file's text, stamped with what made them.
pub fn export_saves(storage: &dyn Storage, made: &Made) -> Result<String, StorageError> {
    write_bundle(storage, made, |key| key == SAVE_KEY)
}

/// What importing the file's `text` would do, before anything is written:
/// each save with its verdict, and whether it replaces one.
pub fn plan_saves(
    text: &str,
    storage: &dyn Storage,
    now: &Made,
) -> Result<ImportPlan, BundleError> {
    let check = FormatCheck::<Save>::new(|key| key == SAVE_KEY);
    plan_import(text, storage, &check, now)
}

/// Writes every save of `plan` this build reads, all or nothing.
pub fn import_saves(plan: &ImportPlan, storage: &mut dyn Storage) -> Result<Applied, StorageError> {
    apply(plan, storage, |_| true)
}
// ANCHOR_END: bundle

#[cfg(test)]
mod tests {
    use shuyi::platform::harness::{FRAME_DT, Harness};
    use shuyi::ui::audio::groups::MUSIC;
    use shuyi::ui::settings::SETTINGS_KEY;
    use shuyi::ui::storage::bundle::Verdict;

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

    // ANCHOR: test_files
    #[test]
    fn a_backed_up_garden_comes_back_from_the_file() {
        let mut h = Harness::start(game().start()).unwrap();
        let garden = Garden::new(&Plan::new(6, 9, 75), 3);
        back_up(h.ctx_mut(), &garden).unwrap();
        h.wait(FRAME_DT);
        let [(name, text)] = h.offered() else {
            panic!("one file offered");
        };
        assert_eq!(name, "garden.ron");
        // The player picks that file again later.
        let picked = SystemEvent::FileChosen(ChosenFile::picked(name, text.as_str()));
        assert_eq!(restore(&picked), Some(Ok(garden)));
    }
    // ANCHOR_END: test_files

    // ANCHOR: test_bundle
    #[test]
    fn saves_come_back_from_a_bundle_with_a_verdict_each() {
        let mut ctx = game().ctx().unwrap();
        let garden = Garden::new(&Plan::new(6, 9, 75), 3);
        save(&mut ctx.storage, &garden).unwrap();
        let text = export_saves(&ctx.storage, &game().made()).unwrap();

        // On another computer, with no save yet.
        let mut other = game().ctx().unwrap();
        let plan = plan_saves(&text, &other.storage, &game().made()).unwrap();
        let entry = plan.entry(SAVE_KEY).unwrap();
        assert_eq!(entry.verdict, Verdict::Ready);
        assert!(!entry.replaces);
        assert_eq!(plan.mismatch, None);
        import_saves(&plan, &mut other.storage).unwrap();
        assert_eq!(load(&other.storage), Ok(Some(garden)));
    }
    // ANCHOR_END: test_bundle

    #[test]
    fn a_bundles_save_of_another_version_is_not_imported() {
        let mut ctx = game().ctx().unwrap();
        ctx.storage
            .write(SAVE_KEY, "(version: 9, garden: ())")
            .unwrap();
        let text = export_saves(&ctx.storage, &game().made()).unwrap();
        let mut other = game().ctx().unwrap();
        let plan = plan_saves(&text, &other.storage, &game().made()).unwrap();
        assert_eq!(
            plan.entry(SAVE_KEY).unwrap().verdict,
            Verdict::Incompatible { found: 9, wants: 1 }
        );
        let applied = import_saves(&plan, &mut other.storage).unwrap();
        assert_eq!(applied.written, Vec::<String>::new());
        assert_eq!(load(&other.storage), Ok(None));
    }

    // ANCHOR: settings
    #[test]
    fn a_changed_setting_is_kept_in_the_players_profile() {
        let mut ctx = game().ctx().unwrap();
        ctx.change_settings(|settings| settings.volumes.set(MUSIC, 40))
            .unwrap();
        assert_eq!(ctx.volume(MUSIC), 40);
        assert!(ctx.storage.read(SETTINGS_KEY).unwrap().is_some());
    }
    // ANCHOR_END: settings
}
