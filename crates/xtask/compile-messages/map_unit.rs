//~ `Piece` is not a unit on a map
//~ write `impl MapUnit for Piece`
//~ `pos` (its tile)
//~ https://zafnok.github.io/shuyi-engine/learn/book/frame-and-skins/
//! A game's code with its piece, its `impl MapUnit` missing (ticket 2510).

use shuyi::ui::debug::gizmo::Gizmos;
use shuyi::ui::grid::debug::layout_gizmos;
use shuyi::ui::grid::{Grid, MapScene};

/// A piece on the map.
struct Piece;

fn gizmos(scene: &MapScene<Piece>, grid: &Grid, out: &mut Gizmos) {
    layout_gizmos(scene, grid, out);
}

fn main() {
    let _ = (gizmos, Piece);
}
