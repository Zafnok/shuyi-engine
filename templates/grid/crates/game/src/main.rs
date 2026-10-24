//! The game's binary: macroquad's entry point, which must be in the
//! binary, and what the engine's macroquad backend (`shuyi-app`) needs to
//! know about the game and how it starts (`shuyi-platform`'s `Start`).

// A release build on Windows is a GUI program: no console window.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use game::{Data, FONT_PNG};
use macroquad::window::Conf;
use shuyi::app::{AppConfig, Window};
use shuyi::assets::audio::{Bytes, Files};
use shuyi::platform::{Card, Look, Setup, Start};
use shuyi::ui::Game;

/// The game's name, on its window.
const NAME: &str = "{{name}}";

/// The window: the console at twice its size, and a small margin.
const WINDOW: Window = Window::new(NAME, 48 * 8 * 2 + 16, 24 * 16 * 2 + 16);

/// Where the settings and key bindings are kept: never change it, or every
/// player's are lost.
const STORAGE_NAME: &str = "{{crate}}";

/// No sound file: the game has none yet.
fn no_sound(_: &str) -> Bytes {
    None
}

/// How errors name a sound file.
fn shown(name: &str) -> String {
    name.to_owned()
}

/// Whether a build leaves a sound file out on purpose: none does.
fn left_out(_: &str) -> bool {
    false
}

/// What the macroquad backend needs to know about the game.
fn config() -> AppConfig {
    let sounds = Files::new(&no_sound, &shown, &left_out, "music");
    AppConfig::new(WINDOW, STORAGE_NAME, game::file, FONT_PNG, sounds)
}

/// What the game's frames are painted with.
fn look(data: &Data) -> Look<'_> {
    Look::new(&data.font, &data.images)
}

/// The game on the platform's `setup`, on its stage, or the module that
/// can't be installed. The stages' luck is seeded from the clock (the
/// platform's music seed), so each launch plays differently.
fn start(setup: Setup) -> Result<Game<Data>, String> {
    let mut ctx = setup.ctx(game::ctx(0));
    ctx.game.seed = ctx.music_seed;
    game::game(ctx).map_err(|e| e.to_string())
}

fn window_conf() -> Conf {
    shuyi::app::window_conf(&WINDOW)
}

#[macroquad::main(window_conf)]
async fn main() {
    let start = Start::new(Card::text(WINDOW.title), look, start);
    shuyi::app::run(config(), start).await;
}
