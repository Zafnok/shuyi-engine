//~ `Wall` is not a set of colors
//~ write `impl ColorSet for Wall`, or use the engine's `EngineColor`
//~ https://zafnok.github.io/shuyi-engine/learn/book/frame-and-skins/
//! A game's code with its colors, their `impl ColorSet` missing (ticket 2510).

use shuyi::prelude::*;

type Data = BasicData<UiAction>;

/// The colors the game draws walls in.
#[derive(Clone, Copy)]
enum Wall {
    Stone,
}

fn draw(ctx: &Ctx<Data>) -> Rgb {
    ctx.palette.get(Wall::Stone)
}

fn main() {
    let _ = draw;
}
