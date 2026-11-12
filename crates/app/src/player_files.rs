//! Files the player keeps (ticket 3309): what the backend does with a
//! screen's requests (`shuyi_ui::player_files`).
//!
//! - On a desktop ([`native`]), an offered file is written into the
//!   `exports` folder of the game's data folder, numbered beside a file of
//!   its name (`farm (2).ron`), never over it; a folder opens in the
//!   system's file manager (`explorer`, `open`, `xdg-open`). There is no
//!   picker: the player drops a file on the window instead
//!   ([`drops`](crate::drops)).
//! - On the web ([`web`]), an offered file is a download, and an ask opens
//!   the browser's file picker; the `shuyi_player_files` miniquad plugin in
//!   `crates/cli/web/player-files.js` (our own code, loaded by
//!   `crates/cli/web/index.html`) does both. The answer to an ask comes in
//!   a later frame's system events, once the player has chosen.

#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(target_arch = "wasm32")]
pub(crate) mod web;

#[cfg(not(target_arch = "wasm32"))]
pub(crate) use native::PlayerFiles;
#[cfg(target_arch = "wasm32")]
pub(crate) use web::PlayerFiles;

use shuyi_ui::{FilePicker, PlatformFacts};

use crate::storage;

/// `facts` with what this backend says of the player's files, for the game
/// whose storage is at `at`: where it is, and a file picker on the web
/// alone.
pub(crate) fn facts(facts: PlatformFacts, at: &storage::StorageAt) -> PlatformFacts {
    facts
        .with_storage_place(at.place())
        .with_file_picker(if cfg!(target_arch = "wasm32") {
            FilePicker::Opens
        } else {
            FilePicker::Absent
        })
}

#[cfg(test)]
mod tests {
    use shuyi_ui::StoragePlace;

    use super::*;

    #[test]
    fn a_desktop_says_its_storage_folder_and_has_no_picker() {
        let at = storage::StorageAt::find("some-game", storage::DesktopFolder::Data);
        let said = facts(PlatformFacts::default(), &at);
        assert!(!said.can_pick_files());
        let StoragePlace::Folder(folder) = &said.storage_place else {
            panic!("{:?}", said.storage_place);
        };
        assert!(folder.ends_with("some-game"), "{folder}");
    }
}
