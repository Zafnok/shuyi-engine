//! Hello, Grid's binary: macroquad's entry point, which must be in the
//! binary, and what the engine's platform shell (`shuyi-app`) needs to
//! know about the game.

// A release build on Windows is a GUI program: no console window.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use hello_grid::{FONT_PNG, Hello, sound};
use macroquad::window::Conf;
use shuyi_app::{AppConfig, Platform, Window};
use shuyi_ui::Game;

/// The window: the console at twice its size, and a small margin.
const WINDOW: Window = Window {
    title: "Hello, Grid",
    width: 100 * 8 * 2 + 16,
    height: 32 * 16 * 2 + 16,
    icon: None,
};

/// Where the settings and key bindings are kept: never change it, or every
/// player's are lost.
const STORAGE_NAME: &str = "shuyi-hello-grid";

/// What the platform shell needs to know about the game.
fn config() -> AppConfig<Hello> {
    AppConfig {
        window: WINDOW,
        storage_name: STORAGE_NAME,
        files: hello_grid::file,
        font: |game| &game.font,
        font_png: FONT_PNG,
        images: |game| &game.images,
        sounds: sound::files(),
        voices: None,
    }
}

/// The game on `platform`, at its title. The stages' luck is seeded from
/// the clock (the platform's music seed), so each launch plays differently.
fn start(platform: &Platform) -> Game<Hello> {
    let mut ctx = platform.setup(hello_grid::ctx(0));
    ctx.game.seed = ctx.music_seed;
    hello_grid::game(ctx)
}

fn window_conf() -> Conf {
    shuyi_app::window_conf(&WINDOW)
}

#[macroquad::main(window_conf)]
async fn main() {
    shuyi_app::run(config(), |platform| Ok(start(platform))).await;
}
