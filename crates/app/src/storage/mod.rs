//! Platform [`Storage`] backends (ticket 0207, game ADR-0009): files in the OS
//! data directory on native, `localStorage` on web. Which folder a desktop
//! keeps them in is the game's [`DesktopFolder`] (ticket 3310, ADR-0109).

#[cfg(not(target_arch = "wasm32"))]
mod desktop;
#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(target_arch = "wasm32")]
mod web;

#[cfg(not(target_arch = "wasm32"))]
pub use native::FileStorage;
#[cfg(target_arch = "wasm32")]
pub use web::WebStorage;

#[cfg(not(target_arch = "wasm32"))]
pub use desktop::desktop_dir;
#[cfg(not(target_arch = "wasm32"))]
pub use native::data_dir;

use std::path::PathBuf;

use shuyi_ui::StoragePlace;
use shuyi_ui::storage::Storage;

/// Which folder a desktop build keeps the storage in
/// ([`AppConfig::desktop_folder`](crate::AppConfig)). The web keeps it in
/// `localStorage` whatever this says.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum DesktopFolder {
    /// The storage's name in the OS's data folder ([`data_dir`]):
    /// `%APPDATA%` on Windows, `~/Library/Application Support` on macOS,
    /// `$XDG_DATA_HOME` (or `~/.local/share`) elsewhere.
    #[default]
    Data,
    /// On Windows, `<Documents>\My Games\<folder>`, where the player finds
    /// PC games' saves: Documents is the system's known folder, wherever it
    /// was moved (to `OneDrive`). Elsewhere, [`Data`](Self::Data): macOS and
    /// Linux have no such place. On the first launch whose folder holds no
    /// key yet, everything in the [`Data`](Self::Data) folder moves there;
    /// if the move fails, that launch keeps the old folder and logs why.
    MyGames(&'static str),
}

/// Where a game's storage is, found once as it starts
/// ([`find`](Self::find)): its name, and its folder on a desktop.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct StorageAt {
    /// The storage's name: the `localStorage` prefix on the web.
    name: &'static str,
    /// The folder on a desktop; `None` on the web, or where the system has
    /// no data folder.
    dir: Option<PathBuf>,
}

impl StorageAt {
    /// The storage of the game whose storage is called `name`, in the
    /// folder `desktop` says, its old storage moved there first on a
    /// desktop ([`DesktopFolder::MyGames`]).
    pub(crate) fn find(name: &'static str, desktop: DesktopFolder) -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        let dir = desktop::find(name, desktop);
        #[cfg(target_arch = "wasm32")]
        let dir = {
            let _ = desktop;
            None
        };
        Self { name, dir }
    }

    /// The folder on a desktop, if there is one.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn dir(&self) -> Option<&std::path::Path> {
        self.dir.as_deref()
    }

    /// The storage backend there ([`platform`]).
    pub(crate) fn open(&self) -> Box<dyn Storage> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Some(dir) = &self.dir {
                Box::new(FileStorage::in_folder(dir.clone()))
            } else {
                macroquad::prelude::error!(
                    "storage: could not determine an OS data directory; saves and settings will not persist"
                );
                Box::new(shuyi_ui::storage::MemoryStorage::new())
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            Box::new(WebStorage::new(self.name))
        }
    }

    /// Where it is, to tell the player ([`place`]).
    pub(crate) fn place(&self) -> StoragePlace {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = self.name;
            folder_place(self.dir.as_deref())
        }
        #[cfg(target_arch = "wasm32")]
        {
            browser_place(self.name)
        }
    }
}

/// The storage backend for this platform, for the game whose storage is
/// called `name` ([`AppConfig::storage_name`](crate::AppConfig)), in its
/// [`DesktopFolder::Data`] folder on a desktop. Native falls back to an
/// in-memory store (nothing persists) if the OS has no usable data
/// directory (the relevant environment variable is unset, which only
/// happens on unsupported or misconfigured systems); it logs an error when
/// that happens.
pub fn platform(name: &str) -> Box<dyn Storage> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        match FileStorage::new(name) {
            Ok(storage) => Box::new(storage),
            Err(e) => {
                macroquad::prelude::error!("storage: {e}; saves and settings will not persist");
                Box::new(shuyi_ui::storage::MemoryStorage::new())
            }
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        Box::new(WebStorage::new(name))
    }
}

/// The prefix of every key the web build keeps in `localStorage` for the
/// game whose storage is called `name`: `name/`.
pub fn web_prefix(name: &str) -> String {
    format!("{name}/")
}

/// Where the game whose storage is called `name` keeps it, to tell the
/// player (`shuyi_ui::PlatformFacts::storage_place`): its
/// [`DesktopFolder::Data`] folder on native ([`data_dir`]), unknown where
/// the system has none; the browser's
/// site storage under [`web_prefix`] on the web.
pub fn place(name: &str) -> StoragePlace {
    #[cfg(not(target_arch = "wasm32"))]
    {
        folder_place(data_dir(name).as_deref())
    }
    #[cfg(target_arch = "wasm32")]
    {
        browser_place(name)
    }
}

/// The place of storage in the folder `dir`, if there is one.
#[cfg(any(not(target_arch = "wasm32"), test))]
fn folder_place(dir: Option<&std::path::Path>) -> StoragePlace {
    dir.map_or(StoragePlace::Unknown, |dir| {
        StoragePlace::Folder(dir.display().to_string())
    })
}

/// The place of storage in the browser for the game whose storage is
/// called `name`.
#[cfg(any(target_arch = "wasm32", test))]
fn browser_place(name: &str) -> StoragePlace {
    StoragePlace::Browser {
        prefix: web_prefix(name),
    }
}

/// The key [`smoke_check`] writes and removes.
const SMOKE_TEST_KEY: &str = "smoke_test";

/// A startup check, which the backend runs in debug builds as it hands the
/// game its storage: writes and reads back a throwaway key and logs the
/// result, then removes it.
pub fn smoke_check(storage: &mut dyn Storage) {
    use macroquad::prelude::{error, info};

    match run_smoke_check(storage) {
        Ok(()) => {
            info!("storage: smoke check passed");
        }
        Err(e) => {
            error!("storage: smoke check failed: {e}");
        }
    }
}

fn run_smoke_check(storage: &mut dyn Storage) -> Result<(), shuyi_ui::storage::StorageError> {
    storage.write(SMOKE_TEST_KEY, "ok")?;
    let read_back = storage.read(SMOKE_TEST_KEY)?;
    storage.delete(SMOKE_TEST_KEY)?;
    if read_back.as_deref() == Some("ok") {
        Ok(())
    } else {
        Err(shuyi_ui::storage::StorageError::Backend(format!(
            "wrote \"ok\", read back {read_back:?}"
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_web_prefix_is_the_storage_name_and_a_slash() {
        assert_eq!(web_prefix("some-game"), "some-game/");
    }

    #[test]
    fn the_place_is_the_data_folder_on_a_desktop_and_the_prefix_on_the_web() {
        let dir = std::path::Path::new("saves").join("some-game");
        assert_eq!(
            folder_place(Some(&dir)),
            StoragePlace::Folder(dir.display().to_string())
        );
        assert_eq!(folder_place(None), StoragePlace::Unknown);
        assert_eq!(
            browser_place("some-game"),
            StoragePlace::Browser {
                prefix: "some-game/".to_owned()
            }
        );
        #[cfg(not(target_arch = "wasm32"))]
        assert_eq!(
            place("some-game"),
            folder_place(data_dir("some-game").as_deref())
        );
    }
}
