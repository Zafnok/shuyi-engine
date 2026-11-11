//! Saving a session on the web, which has no files: a download, through the
//! player's files' plugin ([`player_files::web`](crate::player_files::web)).

use macroquad::miniquad::date;
use shuyi_platform::Millis;

/// The page's clock now, in ms since the Unix epoch.
pub(super) fn unix_ms() -> u64 {
    Millis::from_secs_f64(date::now()).0
}

/// Offers the player `text` as a file called `name`.
pub(super) fn download(name: &str, text: &str) {
    if !crate::player_files::web::download(name, text) {
        macroquad::prelude::warn!("the browser refused the download of {}", name);
    }
}
