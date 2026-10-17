//! Platform [`Storage`] backends (ticket 0207, ADR-0009): files in the OS
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

/// The key the debug-only [`smoke_check`] writes and removes.
#[cfg(debug_assertions)]
const SMOKE_TEST_KEY: &str = "smoke_test";

/// Debug-only startup check: writes and reads back a throwaway key and logs
/// the result, then removes it.
///
/// A `#[cfg(debug_assertions)]` island: in a release build, none of this —
/// including its `macroquad::prelude::{error, info}` imports — is compiled,
/// so it can't trip `unused_imports`/`dead_code` under `-D warnings` there.
#[cfg(debug_assertions)]
pub fn smoke_check(storage: &mut dyn Storage) {
    use macroquad::prelude::{error, info};

    match run_smoke_check(storage) {
        Ok(()) => info!("storage: smoke check passed"),
        Err(e) => error!("storage: smoke check failed: {e}"),
    }
}

#[cfg(debug_assertions)]
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
}
