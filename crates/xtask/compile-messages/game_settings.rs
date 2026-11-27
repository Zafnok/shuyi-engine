//~ `Own` is not a game's settings
//~ write `impl GameSettings for Own`, or use `()`
//~ a game with none of its own uses `()`
//~ https://zafnok.github.io/shuyi-engine/learn/book/saves-and-settings/
//! A game's code with its own settings, their `impl GameSettings` missing (ticket 2510).

use shuyi::ui::settings::Settings;

/// The settings the game adds.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct Own {
    hard: bool,
}

fn hard(settings: &Settings<Own>) -> bool {
    settings.game.hard
}

fn main() {
    let _ = hard;
}
