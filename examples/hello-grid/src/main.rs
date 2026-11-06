//! Hello, Grid's binary: macroquad's entry point, which must be in the
//! binary, and what the engine's macroquad backend (`shuyi-app`) needs to
//! know about the game. How it starts is the library's (`hello_grid::start`).

// A release build on Windows is a GUI program: no console window.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use hello_grid::app_info::APP;
use hello_grid::{FONT_PNG, TITLE, sound};
use macroquad::window::Conf;
use shuyi::app::page::{self, PageLimits};
use shuyi::app::{AppConfig, CrashReports, Window};
use shuyi::assets::bundle::Layer;

/// The window, titled with the game's name: the console at twice its
/// size, and a small margin, with the game's icon (none yet).
const WINDOW: Window =
    Window::new(TITLE, 48 * 8 * 2 + 16, 24 * 16 * 2 + 16).with_icon(APP.window_icon);

/// What the macroquad backend needs to know about the game: its settings
/// and key bindings are kept under `APP.storage`, and its crash reports
/// name `APP.version`.
fn config() -> AppConfig {
    let mut config = AppConfig::new(
        WINDOW,
        APP.storage,
        hello_grid::file,
        FONT_PNG,
        sound::files(),
    );
    config.crash = CrashReports::new(APP.version);
    config
}

fn window_conf() -> Conf {
    shuyi::app::window_conf(&WINDOW)
}

/// The files the page holds, laid over the game's on the web (the site's
/// playground): when the game's data loads with them. Else the page hears
/// every problem, and the game plays its own files. Off the web the page
/// holds none.
fn page_over() -> Option<Layer> {
    let page = match page::page_layer("files", PageLimits::default()) {
        Ok(page) => page?,
        Err(error) => {
            page::report(&error.to_string());
            return None;
        }
    };
    if let Some(problems) = hello_grid::problems_over(page) {
        page::report(&problems);
        return None;
    }
    page::report("");
    hello_grid::lay_over_files(page);
    Some(page)
}

#[macroquad::main(window_conf)]
async fn main() {
    let over = shuyi::app::watched_layer().or_else(page_over);
    let start = hello_grid::start_over(over);
    shuyi::app::run(config(), start).await;
}
