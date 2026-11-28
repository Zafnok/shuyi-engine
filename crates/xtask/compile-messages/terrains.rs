//~ `Terrain` is not a game's terrains
//~ write `impl Terrains for Terrain`
//~ `name` and `glyphs` for each terrain id
//~ https://zafnok.github.io/shuyi-engine/learn/book/frame-and-skins/
//! A game's code with its terrains, their `impl Terrains` missing (ticket 2510).

use shuyi::assets::lighting::Lighting;
use shuyi::ui::frame::Palette;
use shuyi::ui::grid::Looks;

/// The game's terrains.
struct Terrain;

fn looks<'a>(palette: &'a Palette, lights: &'a Lighting) -> Looks<'a> {
    Looks::new(palette, &Terrain, lights)
}

fn main() {
    let _ = looks;
}
