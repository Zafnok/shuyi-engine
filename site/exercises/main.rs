//! The game's binary as the book's site plays it (ADR-0115): the step's
//! own `main.rs`, but with the checks its page's exercises ask for. `cargo
//! xtask exercises` puts this file in a copy of each step, and that step's
//! checks beside it in `src/exercises/`.

// A release build on Windows is a GUI program: no console window.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

#[path = "exercises/checks.rs"]
mod checks;

use shuyi::app::run_game_checked;

fn main() {
    run_game_checked(game::game(), checks::CHECKS);
}
