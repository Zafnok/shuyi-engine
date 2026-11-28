//~ `Letters` doesn't paint units `Piece`
//~ write `impl UnitPainter<Piece> for Letters`, or use the engine's `NoUnits`
//~ https://zafnok.github.io/shuyi-engine/learn/book/frame-and-skins/
//! A game's code with its painter, its `impl UnitPainter` missing (ticket 2510).

use shuyi::core::Pos;
use shuyi::prelude::*;
use shuyi::ui::grid::{GlyphSkin, GridSkin, Looks, MapScene, MapUnit};

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

/// Paints each piece as a letter.
struct Letters;

fn draw(skin: &GlyphSkin, looks: &Looks, scene: &MapScene<Piece>, buf: &mut GlyphBuffer) {
    skin.paint(looks, scene, Rect::new(0, 0, 40, 20), &Letters, buf);
}

fn main() {
    let _ = (draw, Piece { pos: Pos::new(0, 0) });
}
