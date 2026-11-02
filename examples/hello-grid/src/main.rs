//! Hello, Grid's binary: macroquad's entry point, which must be in the
//! binary, and what the engine's macroquad backend (`shuyi-app`) needs to
//! know about the game. How it starts is the library's (`hello_grid::start`).

// A release build on Windows is a GUI program: no console window.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use hello_grid::app_info::APP;
use hello_grid::{FONT_PNG, TITLE, sound};
use macroquad::window::Conf;
use shuyi::app::{AppConfig, Window};

/// The window, titled with the game's name: the console at twice its
/// size, and a small margin, with the game's icon (none yet).
const WINDOW: Window =
    Window::new(TITLE, 48 * 8 * 2 + 16, 24 * 16 * 2 + 16).with_icon(APP.window_icon);

/// What the macroquad backend needs to know about the game: its settings
/// and key bindings are kept under `APP.storage`.
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

#[macroquad::main(window_conf)]
async fn main() {
    let start = hello_grid::start_over(shuyi::app::watched_layer());
    shuyi::app::run(config(), start).await;
}
