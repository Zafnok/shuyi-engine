//! The game's binary: macroquad's entry point, which must be in the
//! binary, and what the engine's platform shell (`shuyi-app`) needs to
//! know about the game.

// A release build on Windows is a GUI program: no console window.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use game::{Data, FONT_PNG};
use macroquad::window::Conf;
use shuyi_app::{AppConfig, Platform, Window};
use shuyi_assets::audio::{Bytes, Files};
use shuyi_ui::Game;

/// The window: the console at twice its size, and a small margin.
const WINDOW: Window = Window {
    title: "{{name}}",
    width: 48 * 8 * 2 + 16,
    height: 24 * 16 * 2 + 16,
    icon: None,
};

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

/// What the platform shell needs to know about the game.
fn config() -> AppConfig<Data> {
    AppConfig {
        window: WINDOW,
        storage_name: STORAGE_NAME,
        files: game::file,
        font: |data| &data.font,
        font_png: FONT_PNG,
        images: |data| &data.images,
        sounds: Files {
            sound: &no_sound,
            sound_shown: &shown,
            sound_left_out: &left_out,
            music_dir: "music",
            music: None,
        },
        voices: None,
    }
}

/// The game on `platform`, on its stage. The stages' luck is seeded from
/// the clock (the platform's music seed), so each launch plays differently.
fn start(platform: &Platform) -> Game<Data> {
    let mut ctx = platform.setup(game::ctx(0));
    ctx.game.seed = ctx.music_seed;
    game::game(ctx)
}

fn window_conf() -> Conf {
    shuyi_app::window_conf(&WINDOW)
}

#[macroquad::main(window_conf)]
async fn main() {
    shuyi_app::run(config(), |platform| Ok(start(platform))).await;
}
