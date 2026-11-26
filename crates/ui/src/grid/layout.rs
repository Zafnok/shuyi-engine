//! Where a scene's tiles go on the console, in pixels: what every grid
//! skin works out from its own tile size, and the shared drawing (the path,
//! the threat arrows) works from. The tiles are laid out by a
//! [`Projection`]: today top-down squares only (ADR-0003 §6; the
//! isometric one is ticket 1126).

use shuyi_core::Pos;

use crate::frame::console::ConsoleSpec;
use crate::frame::glyph_buffer::{PxRect, Rect};

/// A console cell in pixels, across × down. The grid skins lay a map out
/// in a text console's cells ([`ConsoleSpec::text`]): 8 × 16 pixels.
pub const CELL_PX: (i32, i32) = (
    ConsoleSpec::text(0, 0).cell_w as i32,
    ConsoleSpec::text(0, 0).cell_h as i32,
);

/// `cells` in console pixels.
pub fn px_rect(cells: Rect) -> PxRect {
    let (cw, ch) = CELL_PX;
    Rect::new(cells.x * cw, cells.y * ch, cells.w * cw, cells.h * ch)
}

/// How map tiles are laid on the screen: where a tile goes, from where it
/// is in the view. Every skin and every drawing across tiles asks it,
/// rather than assume squares in rows (ADR-0003 §6).
///
/// Kind 6 of ADR-0039: more projections may come, so a `match` on it has a `_` arm.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum Projection {
    /// Seen from above: tiles in rows and columns, each the tile's size
    /// right of and below the one before.
    #[default]
    TopDown,
}

impl Projection {
    /// The top-left pixel of the tile `dx` right of and `dy` below the
    /// view's first, whose top-left pixel is `corner`, on tiles of `tile`
    /// pixels.
    pub const fn place(self, corner: (i32, i32), tile: (i32, i32), dx: i32, dy: i32) -> (i32, i32) {
        match self {
            Projection::TopDown => (corner.0 + tile.0 * dx, corner.1 + tile.1 * dy),
        }
    }

    /// The inverse of [`place`](Self::place): the tile the pixel `px`
    /// lies on, as tiles right of and below the view's first, whose
    /// top-left pixel is `corner`, on tiles of `tile` pixels (a size
    /// below 1 counts as 1).
    pub fn tile_at(self, corner: (i32, i32), tile: (i32, i32), px: (i32, i32)) -> (i32, i32) {
        let (w, h) = (tile.0.max(1), tile.1.max(1));
        match self {
            Projection::TopDown => (
                (px.0 - corner.0).div_euclid(w),
                (px.1 - corner.1).div_euclid(h),
            ),
        }
    }
}

/// The tiles of a scene laid out on the console.
///
/// Kind 3 of ADR-0039: build it with [`Grid::top_down`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Grid {
    /// The map tile in the top-left corner.
    pub origin: Pos,
    /// The top-left pixel of that tile.
    pub corner: (i32, i32),
    /// A tile's size in pixels, across × down.
    pub tile: (i32, i32),
    /// Tiles drawn, across × down.
    pub tiles: (i32, i32),
    /// Console pixels per pixel of the art (the skin's zoom): 1 unless
    /// everything on the map is drawn bigger. `tile` already counts it.
    pub scale: i32,
    /// How the tiles are laid.
    pub projection: Projection,
}

impl Grid {
    /// One tile of `tile` pixels, the map tile `origin`, its top-left pixel
    /// the console's, laid top-down at zoom 1. Set the rest with
    /// [`with_corner`](Self::with_corner), [`with_tiles`](Self::with_tiles)
    /// and [`with_scale`](Self::with_scale): no constructor takes two pairs,
    /// which a call could swap (ADR-0151).
    pub const fn top_down(origin: Pos, tile: (i32, i32)) -> Self {
        Self {
            origin,
            corner: (0, 0),
            tile,
            tiles: (1, 1),
            scale: 1,
            projection: Projection::TopDown,
        }
    }

    /// The same grid, the origin's top-left pixel at `corner`.
    #[must_use]
    pub const fn with_corner(mut self, corner: (i32, i32)) -> Self {
        self.corner = corner;
        self
    }

    /// The same grid, `tiles` tiles drawn, across × down.
    #[must_use]
    pub const fn with_tiles(mut self, tiles: (i32, i32)) -> Self {
        self.tiles = tiles;
        self
    }

    /// The same grid, at the zoom `scale` (which `tile` already counts).
    #[must_use]
    pub const fn with_scale(mut self, scale: i32) -> Self {
        self.scale = scale;
        self
    }

    /// Where `tile` is, as tiles right of and below the origin, if drawn.
    pub fn offset(&self, tile: Pos) -> Option<(i32, i32)> {
        let dx = tile.x.checked_sub(self.origin.x)?;
        let dy = tile.y.checked_sub(self.origin.y)?;
        let inside = (0..self.tiles.0).contains(&dx) && (0..self.tiles.1).contains(&dy);
        inside.then_some((dx, dy))
    }

    /// The top-left pixel of `tile`, drawn or not (it may lie outside the
    /// grid).
    pub fn tile_px(&self, tile: Pos) -> (i32, i32) {
        let (dx, dy) = (tile.x - self.origin.x, tile.y - self.origin.y);
        self.projection.place(self.corner, self.tile, dx, dy)
    }

    /// The pixels of the tile `dx` right of and `dy` below the origin.
    pub fn rect_at(&self, dx: i32, dy: i32) -> PxRect {
        let (w, h) = self.tile;
        let (x, y) = self.projection.place(self.corner, self.tile, dx, dy);
        Rect::new(x, y, w, h)
    }

    /// The drawn tile the console pixel `px` lies on (what the pointer is
    /// over), or `None` outside the drawn tiles.
    pub fn tile_at_px(&self, px: (i32, i32)) -> Option<Pos> {
        let (dx, dy) = self.projection.tile_at(self.corner, self.tile, px);
        let inside = (0..self.tiles.0).contains(&dx) && (0..self.tiles.1).contains(&dy);
        if !inside {
            return None;
        }
        let x = self.origin.x.checked_add(dx)?;
        let y = self.origin.y.checked_add(dy)?;
        Some(Pos::new(x, y))
    }

    /// The pixels of `tile`, or `None` if it isn't drawn.
    pub fn rect(&self, tile: Pos) -> Option<PxRect> {
        let (dx, dy) = self.offset(tile)?;
        Some(self.rect_at(dx, dy))
    }

    /// The skin's zoom, at least 1.
    pub fn zoom(&self) -> i32 {
        self.scale.max(1)
    }

    /// A tile at the art's own size, its corner at (0, 0): what a mark's
    /// place on a tile is worked out on, whatever the zoom
    /// ([`zoomed`](Self::zoomed)).
    pub fn art_tile(&self) -> PxRect {
        let zoom = self.zoom();
        Rect::new(0, 0, self.tile.0 / zoom, self.tile.1 / zoom)
    }

    /// The same tiles at the art's own size, laid from (0, 0): what a
    /// drawing across tiles (the path) is worked out on, whatever the
    /// zoom, before [`zoomed`](Self::zoomed) from this grid's corner
    /// places it.
    #[must_use]
    pub fn at_art_size(&self) -> Self {
        let tile = self.art_tile();
        Self {
            corner: (0, 0),
            tile: (tile.w, tile.h),
            scale: 1,
            ..*self
        }
    }

    /// `art`, pixels worked out on [`art_tile`](Self::art_tile), on the
    /// tile whose top-left pixel is `corner`: every art pixel a square of
    /// the zoom's console pixels.
    pub fn zoomed(&self, corner: (i32, i32), art: PxRect) -> PxRect {
        let zoom = self.zoom();
        Rect::new(
            corner.0 + art.x * zoom,
            corner.1 + art.y * zoom,
            art.w * zoom,
            art.h * zoom,
        )
    }

    /// The pixels every drawn tile covers.
    pub fn bounds(&self) -> PxRect {
        let (w, h) = self.tile;
        let (x, y) = self.corner;
        Rect::new(x, y, w * self.tiles.0, h * self.tiles.1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: i32, y: i32) -> Pos {
        Pos::new(x, y)
    }

    #[test]
    fn tiles_are_laid_from_the_corner_in_their_size() {
        let g = Grid::top_down(p(5, -2), (24, 20))
            .with_corner((4, 8))
            .with_tiles((3, 2));
        assert_eq!(g.projection, Projection::TopDown);
        assert_eq!(g.offset(p(5, -2)), Some((0, 0)));
        assert_eq!(g.offset(p(7, -1)), Some((2, 1)));
        for out in [
            p(8, -2),
            p(4, -2),
            p(5, 0),
            p(5, -3),
            p(i32::MIN, 0),
            p(0, i32::MIN),
        ] {
            assert_eq!(g.offset(out), None, "{out:?}");
            assert_eq!(g.rect(out), None, "{out:?}");
        }
        assert_eq!(g.tile_px(p(5, -2)), (4, 8));
        assert_eq!(g.tile_px(p(7, -1)), (52, 28));
        // Outside the grid too.
        assert_eq!(g.tile_px(p(4, -3)), (-20, -12));
        assert_eq!(g.rect(p(6, -1)), Some(Rect::new(28, 28, 24, 20)));
        assert_eq!(g.rect_at(2, 0), Rect::new(52, 8, 24, 20));
        assert_eq!(g.bounds(), Rect::new(4, 8, 72, 40));
        // At the art's own size, a mark is where it was worked out.
        assert_eq!(g.art_tile(), Rect::new(0, 0, 24, 20));
        let mark = Rect::new(3, -2, 5, 7);
        assert_eq!(g.zoomed((28, 28), mark), Rect::new(31, 26, 5, 7));
    }

    #[test]
    fn a_cell_is_a_text_consoles_and_cells_scale_to_pixels() {
        assert_eq!(CELL_PX, (8, 16));
        assert_eq!(px_rect(Rect::new(3, 2, 5, 4)), Rect::new(24, 32, 40, 64));
        assert_eq!(px_rect(Rect::new(-1, -2, 0, 1)), Rect::new(-8, -32, 0, 16));
    }

    #[test]
    fn top_down_tiles_go_in_rows_and_columns() {
        let top = Projection::default();
        assert_eq!(top, Projection::TopDown);
        assert_eq!(top.place((4, 8), (24, 20), 0, 0), (4, 8));
        assert_eq!(top.place((4, 8), (24, 20), 2, 1), (52, 28));
        assert_eq!(top.place((4, 8), (24, 20), -1, -3), (-20, -52));
    }

    #[test]
    fn a_pixel_lies_on_the_tile_drawn_there() {
        let top = Projection::TopDown;
        assert_eq!(top.tile_at((4, 8), (24, 20), (4, 8)), (0, 0));
        assert_eq!(top.tile_at((4, 8), (24, 20), (27, 27)), (0, 0));
        assert_eq!(top.tile_at((4, 8), (24, 20), (28, 28)), (1, 1));
        assert_eq!(top.tile_at((4, 8), (24, 20), (3, 7)), (-1, -1));
        assert_eq!(top.tile_at((0, 0), (0, -3), (5, 6)), (5, 6), "at least 1");
        let g = Grid::top_down(p(5, -2), (24, 20))
            .with_corner((4, 8))
            .with_tiles((3, 2));
        assert_eq!(g.tile_at_px((4, 8)), Some(p(5, -2)));
        assert_eq!(g.tile_at_px((75, 47)), Some(p(7, -1)));
        for out in [(3, 8), (4, 7), (76, 8), (4, 48)] {
            assert_eq!(g.tile_at_px(out), None, "{out:?}");
        }
        let far = Grid::top_down(p(i32::MAX, 0), (8, 8)).with_tiles((2, 1));
        assert_eq!(far.tile_at_px((9, 0)), None, "past the last position");
    }

    #[test]
    fn a_zoomed_grid_draws_every_art_pixel_as_a_square() {
        let g = Grid::top_down(p(0, 0), (48, 40))
            .with_corner((8, 0))
            .with_tiles((3, 2))
            .with_scale(2);
        assert_eq!(g.art_tile(), Rect::new(0, 0, 24, 20));
        // From the tile's corner: twice as far, twice as big.
        let mark = Rect::new(3, -2, 5, 7);
        assert_eq!(g.zoomed((56, 40), mark), Rect::new(62, 36, 10, 14));
        assert_eq!(g.zoomed((56, 40), g.art_tile()), g.rect_at(1, 1));
        // The same tiles unzoomed, from (0, 0); already so at 1×.
        let art = Grid {
            corner: (0, 0),
            tile: (24, 20),
            scale: 1,
            ..g
        };
        assert_eq!(g.at_art_size(), art);
        assert_eq!(art.at_art_size(), art);
        // A scale below 1 counts as 1.
        let flat = Grid { scale: 0, ..g };
        assert_eq!(flat.art_tile(), Rect::new(0, 0, 48, 40));
        assert_eq!(flat.zoomed((1, 2), mark), Rect::new(4, 0, 5, 7));
    }

    #[test]
    fn the_constructor_builds_what_a_literal_built() {
        let built = Grid::top_down(p(5, -2), (24, 20))
            .with_corner((4, 8))
            .with_tiles((3, 2))
            .with_scale(2);
        let literal = Grid {
            origin: p(5, -2),
            corner: (4, 8),
            tile: (24, 20),
            tiles: (3, 2),
            scale: 2,
            projection: Projection::TopDown,
        };
        assert_eq!(built, literal);
        let plain = Grid::top_down(p(0, 0), (16, 16));
        assert_eq!(
            (plain.corner, plain.tiles, plain.scale),
            ((0, 0), (1, 1), 1)
        );
    }
}
