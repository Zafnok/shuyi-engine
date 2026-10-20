//! The game's binary: macroquad's entry point, which must be in the
//! binary, and what the engine's macroquad backend (`shuyi-app`) needs to
//! know about the game and how it starts (`shuyi-platform`'s `Start`).

// A release build on Windows is a GUI program: no console window.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use game::{Data, FONT_PNG};
use macroquad::window::Conf;
use shuyi_app::{AppConfig, Window};
use shuyi_assets::audio::{Bytes, Files};
use shuyi_platform::{Card, Look, Setup, Start};
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

/// What the macroquad backend needs to know about the game.
fn config() -> AppConfig {
    AppConfig {
        window: WINDOW,
        storage_name: STORAGE_NAME,
        files: game::file,
        font_png: FONT_PNG,
        sounds: Files {
            sound: &no_sound,
            sound_shown: &shown,
            sound_left_out: &left_out,
            music_dir: "music",
            music: None,
        },
        voices: None,
        text_input: None,
    }
}

/// What the game's frames are painted with.
fn look(data: &Data) -> Look<'_> {
    Look {
        font: &data.font,
        images: &data.images,
    }
}

/// The game on the platform's `setup`, at its title.
fn start(setup: Setup) -> Game<Data> {
    game::game(setup.ctx(game::ctx()))
}

fn window_conf() -> Conf {
    shuyi_app::window_conf(&WINDOW)
}

#[macroquad::main(window_conf)]
async fn main() {
    let start = Start::new(Card::text(WINDOW.title), look, |setup| Ok(start(setup)));
    shuyi_app::run(config(), start).await;
}
