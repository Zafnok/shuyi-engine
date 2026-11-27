//~ `Mine` is not a game's data
//~ write `impl GameData for Mine`, or use the engine's `BasicData`
//~ `BasicData<UiAction>` is a whole one
//~ https://zafnok.github.io/shuyi-engine/learn/book/games-and-screens/
//! A game's code with its own data, its `impl GameData` missing (ticket 2510).

use shuyi::prelude::*;

/// The game's data.
#[derive(Debug)]
struct Mine;

fn draw(_ctx: &Ctx<Mine>) {}

fn main() {
    let _ = draw;
}
