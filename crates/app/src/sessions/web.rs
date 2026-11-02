//! Saving a session on the web, which has no files: a download, through the
//! `shuyi_download` miniquad plugin in `crates/cli/web/download.js` (our own
//! code, loaded by `crates/cli/web/index.html`).

use macroquad::miniquad::date;
use shuyi_platform::Millis;

/// The plugin's version, which the JS loader compares with the `version`
/// in `crates/cli/web/download.js` (a `shuyi-cli` test keeps them equal). Bump both when the
/// function below changes.
const PLUGIN_VERSION: u32 = 1;

// Declaring the function `crates/cli/web/download.js` provides.
#[allow(unsafe_code)]
unsafe extern "C" {
    /// Offers the player a file called the UTF-8 text at `name_ptr`, holding
    /// the bytes at `text_ptr`. Reads both before it returns.
    fn shuyi_download(name_ptr: *const u8, name_len: usize, text_ptr: *const u8, text_len: usize);
}

/// What the JS loader calls to check the plugin's version.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn shuyi_download_crate_version() -> u32 {
    PLUGIN_VERSION
}

/// The page's clock now, in ms since the Unix epoch.
pub(super) fn unix_ms() -> u64 {
    Millis::from_secs_f64(date::now()).0
}

/// Offers the player `text` as a file called `name`.
pub(super) fn download(name: &str, text: &str) {
    // SAFETY: each pointer and length is a live `&str`'s, and the plugin
    // only reads them, before it returns.
    #[allow(unsafe_code)]
    unsafe {
        shuyi_download(name.as_ptr(), name.len(), text.as_ptr(), text.len());
    }
}
