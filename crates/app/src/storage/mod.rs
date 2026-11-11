//! Platform [`Storage`] backends (ticket 0207, game ADR-0009): files in the OS
//! data directory on native, `localStorage` on web.

#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(target_arch = "wasm32")]
mod web;

#[cfg(not(target_arch = "wasm32"))]
pub use native::FileStorage;
#[cfg(target_arch = "wasm32")]
pub use web::WebStorage;

#[cfg(not(target_arch = "wasm32"))]
pub use native::data_dir;

use shuyi_ui::StoragePlace;
use shuyi_ui::storage::Storage;

/// The storage backend for this platform, for the game whose storage is
/// called `name` ([`AppConfig::storage_name`](crate::AppConfig)). Native
/// falls back to an in-memory store (nothing persists) if the OS has no
/// usable data directory (the relevant environment variable is unset,
/// which only happens on unsupported or misconfigured systems); it logs an
/// error when that happens.
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
/// player (`shuyi_ui::PlatformFacts::storage_place`): its data folder on
/// native ([`data_dir`]), unknown where the system has none; the browser's
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
