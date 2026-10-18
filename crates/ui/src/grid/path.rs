//! The movement path arrow, as every grid skin draws it (game ADR-0018): a
//! 3-px line through tile centres, under the glyphs, from the edge of the
//! unit's tile, ending in a single arrowhead over the destination tile.
//! Worked out from the skin's tile size: on the glyph skin's 16 px tiles
//! the line is pixels 7..=9.

use crate::prelude::*;
use shuyi_core::Pos;

use super::layout::Grid;
use crate::frame::color::Rgb;
use crate::frame::glyph_buffer::{Layer, Overlay, PxRect, Rect};

/// The path line's thickness, in pixels.
pub const LINE_W: i32 = 3;

/// Pixels of the line on each side of its middle one.
const LINE_HALF: i32 = 1;

/// The arrowhead's columns (or rows), from its base (11 px across) to its
/// tip (1 px).
const ARROW_LEN: i32 = 6;

/// The line's middle pixel from the edge of a tile `len` pixels long;
/// arrowheads are centred on it.
const fn line_mid(len: i32) -> i32 {
    len / 2
}

/// Offset of the line from the edge of a tile `len` pixels long.
const fn line_offset(len: i32) -> i32 {
    line_mid(len) - LINE_HALF
}

/// Offset of the arrowhead's base from the edge behind it, on a tile `len`
/// pixels long: the arrowhead is centred along the tile.
const fn arrow_base(len: i32) -> i32 {
    (len - ARROW_LEN) / 2
}

/// The line between the centres of adjacent tiles `a` and `b`; with
/// `from_edge`, only the part outside `a`'s tile.
fn segment(a: Pos, b: Pos, grid: &Grid, from_edge: bool) -> PxRect {
    let ((ax, ay), (bx, by)) = (grid.tile_px(a), grid.tile_px(b));
    let (w, h) = grid.tile;
    let (ox, oy) = (line_offset(w), line_offset(h));
    let (mut x0, mut y0) = (ax.min(bx) + ox, ay.min(by) + oy);
    let (mut x1, mut y1) = (ax.max(bx) + ox + LINE_W, ay.max(by) + oy + LINE_W);
    if from_edge {
        match (b.x - a.x, b.y - a.y) {
            (1, _) => x0 = ax + w,
            (-1, _) => x1 = ax,
            (_, 1) => y0 = ay + h,
            _ => y1 = ay,
        }
    }
    Rect::new(x0, y0, x1 - x0, y1 - y0)
}

/// The arrowhead on tile `to`, pointing away from the adjacent tile `from`:
/// [`ARROW_LEN`] stacked 1-px rects, 11 px across at the base down to 1 at
/// the tip, centred on the line.
fn arrowhead(from: Pos, to: Pos, grid: &Grid) -> Vec<PxRect> {
    let (px, py) = grid.tile_px(to);
    let (w, h) = grid.tile;
    let (mid_x, mid_y) = (line_mid(w), line_mid(h));
    (0..ARROW_LEN)
        .map(|i| {
            let half = ARROW_LEN - 1 - i;
            let across = 2 * half + 1;
            let near = |len| arrow_base(len) + i;
            let far = |len| len - 1 - arrow_base(len) - i;
            match (to.x - from.x, to.y - from.y) {
                (1, _) => Rect::new(px + near(w), py + mid_y - half, 1, across),
                (-1, _) => Rect::new(px + far(w), py + mid_y - half, 1, across),
                (_, 1) => Rect::new(px + mid_x - half, py + near(h), across, 1),
                _ => Rect::new(px + mid_x - half, py + far(h), across, 1),
            }
        })
        .collect()
}

/// The overlays drawing `path` (the unit's tile first) in `color`: the
/// line `Under` the glyphs, then the arrowhead `Over` them; nothing for a
/// path of one tile. Clipped to the tiles of `grid`. On a zoomed grid
/// the line and the arrowhead are the art-sized tile's, every pixel a
/// square of the zoom's.
pub fn path_overlays(path: &[Pos], grid: &Grid, color: Rgb) -> Vec<Overlay> {
    let view = grid.bounds();
    let art = grid.at_art_size();
    let mut out = Vec::new();
    let mut add = |rect: PxRect, layer| {
        if let Some(rect) = grid.zoomed(grid.corner, rect).intersect(&view) {
            out.push(Overlay::new(rect, color, layer));
        }
    };
    for (i, pair) in path.windows(2).enumerate() {
        add(segment(pair[0], pair[1], &art, i == 0), Layer::Under);
    }
    if let [.., from, to] = path {
        for rect in arrowhead(*from, *to, &art) {
            add(rect, Layer::Over);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const RED: Rgb = Rgb::new(255, 0, 0);

    fn p(x: i32, y: i32) -> Pos {
        Pos::new(x, y)
    }

    /// The glyph skin's 16 px tiles in a 70 × 30-cell map area (560 ×
    /// 480 px): 35 × 30 tiles from the map's corner.
    fn view() -> Grid {
        Grid::top_down(p(0, 0), (0, 0), (16, 16), (35, 30), 1)
    }

    /// The overlays of `path` with the camera at the origin, as
    /// `(layer, x, y, w, h)`.
    fn rects(path: &[Pos]) -> Vec<(Layer, i32, i32, i32, i32)> {
        path_overlays(path, &view(), RED)
            .iter()
            .map(|o| (o.layer, o.rect.x, o.rect.y, o.rect.w, o.rect.h))
            .collect()
    }

    #[test]
    fn a_one_tile_path_draws_nothing() {
        assert!(rects(&[p(2, 2)]).is_empty());
    }

    #[test]
    fn the_line_starts_at_the_unit_tile_edge_and_ends_in_one_arrowhead() {
        // Right from (1, 1) (pixels 16..32 × 16..32) to (2, 1), then down.
        let r = rects(&[p(1, 1), p(2, 1), p(2, 2)]);
        let under: Vec<_> = r.iter().filter(|o| o.0 == Layer::Under).collect();
        assert_eq!(
            under,
            [
                // From the tile's right edge (x 32) to (2, 1)'s centre band.
                &(Layer::Under, 32, 23, 10, 3),
                // (2, 1)'s centre band down to (2, 2)'s.
                &(Layer::Under, 39, 23, 3, 19),
            ]
        );
        // The arrowhead points down on (2, 2) (pixels 32..48 × 32..48):
        // rows 37..=42, 11 px wide down to 1, centred on x 40.
        let over: Vec<_> = r.iter().filter(|o| o.0 == Layer::Over).copied().collect();
        let expect: Vec<_> = (0..6)
            .map(|i| (Layer::Over, 40 - (5 - i), 37 + i, 2 * (5 - i) + 1, 1))
            .collect();
        assert_eq!(over, expect);
    }

    #[test]
    fn arrowheads_point_the_way_of_the_last_step() {
        let tips = |path: &[Pos]| {
            let over: Vec<_> = rects(path)
                .into_iter()
                .filter(|o| o.0 == Layer::Over)
                .collect();
            // The 1-px tip is the last rect.
            let (_, x, y, w, h) = over[over.len() - 1];
            assert_eq!((w, h), (1, 1));
            (x, y)
        };
        // Tile (1, 1) spans pixels 16..32 on both axes; its centre is 24.
        assert_eq!(tips(&[p(0, 1), p(1, 1)]), (16 + 10, 24));
        assert_eq!(tips(&[p(2, 1), p(1, 1)]), (16 + 5, 24));
        assert_eq!(tips(&[p(1, 0), p(1, 1)]), (24, 16 + 10));
        assert_eq!(tips(&[p(1, 2), p(1, 1)]), (24, 16 + 5));
        // Each line starts at the unit tile's edge facing the move.
        let first = |path: &[Pos]| rects(path)[0];
        assert_eq!(first(&[p(1, 1), p(0, 1)]), (Layer::Under, 7, 23, 9, 3));
        assert_eq!(first(&[p(1, 1), p(1, 0)]), (Layer::Under, 23, 7, 3, 9));
        assert_eq!(first(&[p(1, 1), p(1, 2)]), (Layer::Under, 23, 32, 3, 10));
        // Every arrowhead's 11-px base is centred on the line's middle
        // pixel (24 on tile (1, 1)), across the direction of travel.
        let base = |path: &[Pos]| {
            let over: Vec<_> = rects(path)
                .into_iter()
                .filter(|o| o.0 == Layer::Over)
                .collect();
            over[0]
        };
        assert_eq!(base(&[p(0, 1), p(1, 1)]), (Layer::Over, 21, 19, 1, 11));
        assert_eq!(base(&[p(2, 1), p(1, 1)]), (Layer::Over, 26, 19, 1, 11));
        assert_eq!(base(&[p(1, 0), p(1, 1)]), (Layer::Over, 19, 21, 11, 1));
        assert_eq!(base(&[p(1, 2), p(1, 1)]), (Layer::Over, 19, 26, 11, 1));
        assert_eq!(2 * LINE_HALF + 1, LINE_W);
        assert_eq!(line_offset(16) + LINE_HALF, line_mid(16));
        assert_eq!((line_offset(16), line_mid(16), arrow_base(16)), (7, 8, 5));
    }

    #[test]
    fn the_geometry_follows_the_tile_size() {
        // 24 × 20 tiles: tile (1, 1) spans pixels 24..48 × 20..40.
        let grid = Grid::top_down(p(0, 0), (0, 0), (24, 20), (10, 10), 1);
        let r: Vec<_> = path_overlays(&[p(1, 1), p(2, 1), p(2, 2)], &grid, RED)
            .iter()
            .map(|o| (o.layer, o.rect.x, o.rect.y, o.rect.w, o.rect.h))
            .collect();
        // The line: from x 48 (the unit tile's right edge) along the centre
        // band of (2, 1) (pixels 59..=61 across, 29..=31 down), then down
        // into (2, 2)'s.
        assert_eq!(r[0], (Layer::Under, 48, 29, 14, 3));
        assert_eq!(r[1], (Layer::Under, 59, 29, 3, 23));
        // The arrowhead points down on (2, 2) (pixels 48..72 × 40..60): its
        // base 7 px into the tile, centred on x 60; its tip 5 rows on.
        assert_eq!(r[2], (Layer::Over, 55, 47, 11, 1));
        assert_eq!(r[7], (Layer::Over, 60, 52, 1, 1));
        assert_eq!(r.len(), 8);
        // Pointing left on a 24 px tile: columns 14 down to 9 of it.
        let left = path_overlays(&[p(3, 1), p(2, 1)], &grid, RED);
        let xs: Vec<i32> = left
            .iter()
            .filter(|o| o.layer == Layer::Over)
            .map(|o| o.rect.x)
            .collect();
        assert_eq!(xs, [62, 61, 60, 59, 58, 57]);
        assert_eq!((line_offset(24), line_mid(24), arrow_base(24)), (11, 12, 9));
        assert_eq!(arrow_base(20), 7);
        // The same tiles at twice the size, from (8, 16): every rectangle
        // twice as far from the corner and twice as big.
        let path = [p(1, 1), p(2, 1), p(2, 2)];
        let zoomed = Grid {
            corner: (8, 16),
            tile: (48, 40),
            scale: 2,
            ..grid
        };
        let twice: Vec<_> = path_overlays(&path, &zoomed, RED)
            .iter()
            .map(|o| (o.layer, o.rect.x, o.rect.y, o.rect.w, o.rect.h))
            .collect();
        let doubled: Vec<_> = r
            .iter()
            .map(|&(layer, x, y, w, h)| (layer, 8 + 2 * x, 16 + 2 * y, 2 * w, 2 * h))
            .collect();
        assert_eq!(twice, doubled);
        assert_eq!(twice[0], (Layer::Under, 104, 74, 28, 6));
        // Clipped to the zoomed tiles: a path that leaves a two-tile view.
        let two = Grid {
            tiles: (2, 1),
            ..zoomed
        };
        let cut = path_overlays(&[p(0, 0), p(1, 0), p(2, 0)], &two, RED);
        let right = two.bounds().x + two.bounds().w;
        assert!(cut.iter().all(|o| o.rect.x + o.rect.w <= right), "{cut:?}");
        assert_eq!(cut.len(), 2);
    }

    #[test]
    fn overlays_are_clipped_to_the_map_view() {
        // The view is 35 tiles wide: tile 35 is past its edge.
        let r = rects(&[p(33, 0), p(34, 0), p(35, 0)]);
        let right = view().bounds().w;
        assert_eq!(right, 560);
        assert!(r.iter().all(|o| o.1 + o.3 <= right), "{r:?}");
        assert_eq!(r.len(), 2, "no arrowhead off the view: {r:?}");
    }
}
