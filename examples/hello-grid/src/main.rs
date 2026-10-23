//! Hello, Grid's binary: macroquad's entry point, which must be in the
//! binary, and what the engine's macroquad backend (`shuyi-app`) needs to
//! know about the game and how it starts (`shuyi-platform`'s `Start`).

// A release build on Windows is a GUI program: no console window.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use hello_grid::{FONT_PNG, Hello, sound};
use macroquad::window::Conf;
use shuyi_app::{AppConfig, Window};
use shuyi_platform::{Card, Look, Setup, Start};
use shuyi_ui::Game;

/// The window: the console at twice its size, and a small margin.
const WINDOW: Window = Window::new("Hello, Grid", 48 * 8 * 2 + 16, 24 * 16 * 2 + 16);

/// Where the settings and key bindings are kept: never change it, or every
/// player's are lost.
const STORAGE_NAME: &str = "shuyi-hello-grid";

/// What the macroquad backend needs to know about the game.
fn config() -> AppConfig {
    AppConfig::new(
        WINDOW,
        STORAGE_NAME,
        hello_grid::file,
        FONT_PNG,
        sound::files(),
    )
}

/// What the game's frames are painted with.
fn look(game: &Hello) -> Look<'_> {
    Look {
        font: &game.font,
        images: &game.images,
    }
}

/// The game on the platform's `setup`, at its title. The stages' luck is
/// seeded from the clock (the platform's music seed), so each launch plays
/// differently.
fn start(setup: Setup) -> Game<Hello> {
    let mut ctx = setup.ctx(hello_grid::ctx(0));
    ctx.game.seed = ctx.music_seed;
    hello_grid::game(ctx)
}

fn window_conf() -> Conf {
    shuyi_app::window_conf(&WINDOW)
}

#[macroquad::main(window_conf)]
async fn main() {
    let start = Start::new(Card::text(WINDOW.title), look, |setup| Ok(start(setup)));
    shuyi_app::run(config(), start).await;
}
