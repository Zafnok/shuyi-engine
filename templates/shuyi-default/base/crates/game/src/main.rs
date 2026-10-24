//! The game's binary: the engine's macroquad backend runs the game in a
//! window.

// A release build on Windows is a GUI program: no console window.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use shuyi::prelude::run_game;

fn main() {
    run_game(game::game());
}
