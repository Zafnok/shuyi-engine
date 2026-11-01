//! Hello, Grid's binary: macroquad's entry point, which must be in the
//! binary, and what the engine's macroquad backend (`shuyi-app`) needs to
//! know about the game and how it starts (`shuyi-platform`'s `Start`).

// A release build on Windows is a GUI program: no console window.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use hello_grid::app_info::APP;
use hello_grid::hot_palette::HotPalette;
use hello_grid::{FONT_PNG, Hello, sound};
use macroquad::window::Conf;
use shuyi::app::{AppConfig, Window};
use shuyi::assets::ContentError;
use shuyi::platform::{Card, Look, Setup, Start};
use shuyi::ui::Game;

/// The window, titled with the game's name: the console at twice its
/// size, and a small margin, with the game's icon (none yet).
const WINDOW: Window =
    Window::new(APP.name, 48 * 8 * 2 + 16, 24 * 16 * 2 + 16).with_icon(APP.window_icon);

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

/// What the game's frames are painted with.
fn look(game: &Hello) -> Look<'_> {
    game.look()
}

/// The game on the platform's `setup`, at its title, with its files as
/// the target the platform names has them. The stages' luck is seeded
/// from the clock (the platform's music seed), so each launch plays
/// differently. Under `cargo shuyi run --watch`, its files on disk are laid
/// over the embedded ones, and its palette reloads as its file changes.
fn start(setup: Setup) -> Result<Game<Hello>, String> {
    let target = setup.platform.target.clone();
    let watched = shuyi::app::watched_layer();
    let bundle = hello_grid::bundle_over(watched, &target);
    let mut ctx = setup.ctx(hello_grid::ctx_from(&bundle, &target, 0));
    ctx.game.seed = ctx.music_seed;
    let game = hello_grid::game(ctx);
    if watched.is_none() {
        return Ok(game);
    }
    let lines = |errors: Vec<ContentError>| {
        let lines: Vec<String> = errors.iter().map(ToString::to_string).collect();
        lines.join("\n")
    };
    let palette = HotPalette::new(&bundle).map_err(lines)?;
    game.with_module(palette).map_err(|e| e.to_string())
}

fn window_conf() -> Conf {
    shuyi::app::window_conf(&WINDOW)
}

#[macroquad::main(window_conf)]
async fn main() {
    let start = Start::new(Card::text(APP.name), look, start);
    shuyi::app::run(config(), start).await;
}
