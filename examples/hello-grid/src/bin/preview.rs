//! Hello, Grid's dialogue preview, the program a writer is handed: the
//! scenes of the `.dlg` files in a folder, played in the game's own text
//! box, each read again a second after it is saved
//! ([`hello_grid::preview`]). The folder is the first argument, else the
//! game's `files/`. The web has no folders: there it plays the game's own
//! scene file, and reads nothing again.

// A release build on Windows is a GUI program: no console window.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use hello_grid::app_info::APP;
use hello_grid::{FONT_PNG, TITLE, sound};
use macroquad::window::Conf;
use shuyi::app::{AppConfig, Window};
use shuyi::assets::bundle::Layer;

/// The window: the game's.
const WINDOW: Window =
    Window::new(TITLE, 48 * 8 * 2 + 16, 24 * 16 * 2 + 16).with_icon(APP.window_icon);

/// What the macroquad backend needs to know: the game's, its settings and
/// key bindings shared with it.
fn config() -> AppConfig {
    AppConfig::new(
        WINDOW,
        APP.storage,
        hello_grid::file,
        FONT_PNG,
        sound::files(),
    )
}

fn window_conf() -> Conf {
    shuyi::app::window_conf(&WINDOW)
}

/// The folder named on the command line (else `files/`), read to be
/// watched in every build, or why it can't be read.
#[cfg(not(target_arch = "wasm32"))]
fn folder() -> Result<Layer, String> {
    use std::path::PathBuf;

    let folder = std::env::args_os().nth(1).map_or_else(
        || PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/files")),
        PathBuf::from,
    );
    let limit = u64::try_from(shuyi::assets::format::DEFAULT_MAX_BYTES).unwrap_or(u64::MAX);
    shuyi::app::read_watched_folder(&folder, "files", limit).map_err(|e| e.to_string())
}

/// On the web, which has no folders: the game's own scene file.
#[cfg(target_arch = "wasm32")]
fn folder() -> Result<Layer, String> {
    let scene = hello_grid::file(hello_grid::DIALOGUE).unwrap_or_default();
    let files = [(hello_grid::DIALOGUE.to_owned(), scene.to_vec())];
    Layer::owned("files".to_owned(), files.into_iter().collect()).map_err(|e| e.to_string())
}

#[macroquad::main(window_conf)]
async fn main() {
    match folder() {
        Ok(layer) => shuyi::app::run(config(), hello_grid::preview::start(layer)).await,
        Err(error) => {
            macroquad::logging::error!("{TITLE}: {error}");
        }
    }
}
