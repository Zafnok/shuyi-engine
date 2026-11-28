//~ `Pictures` is not a grid skin of `Piece`
//~ write `impl GridSkin<Piece> for Pictures`
//~ `GlyphSkin` and `SpriteSkin` are two
//~ https://zafnok.github.io/shuyi-engine/learn/book/frame-and-skins/
//! A game's code with its own skin, its `impl GridSkin` missing (ticket 2510).

use shuyi::core::Pos;
use shuyi::ui::grid::{GridSkin, MapUnit};

/// A piece on the map.
struct Piece {
    pos: Pos,
}

impl MapUnit for Piece {
    type Id = u8;

    fn id(&self) -> u8 {
        0
    }

    fn pos(&self) -> Pos {
        self.pos
    }

    fn text(&self) -> String {
        String::from("piece")
    }
}

/// The game's own skin.
#[derive(Debug)]
struct Pictures;

fn skin() -> Box<dyn GridSkin<Piece>> {
    Box::new(Pictures)
}

fn main() {
    let _ = (skin, Piece { pos: Pos::new(0, 0) });
}
