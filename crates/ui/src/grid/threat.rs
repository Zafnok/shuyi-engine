//! The threat arrows, as every grid skin draws them (game ticket 0448): a
//! slim straight arrow from each enemy that could attack a tile to that
//! tile, and a `!` above each of those enemies. Over units and terrain, so
//! never hidden. Worked out from the skin's tile size, in the art's own
//! pixels.
//!
//! An arrow is a 1-pixel line from the enemy's tile (its middle, or its
//! edge for a skin whose units are letters: [`Start`]) to just short of the
//! middle of the tile it threatens, ending in two barbs, with a 1-pixel
//! dark edge all round. A line at any angle is a row of short
//! rectangles, one per step across it. An enemy outside the view has its
//! arrow cut at the view's edge and no mark.

use crate::prelude::*;
use shuyi_core::Pos;

use super::layout::Grid;
use super::scene::{MapScene, ThreatView};
use crate::frame::color::Rgb;
use crate::frame::glyph_buffer::{GlyphBuffer, Layer, Overlay, PxRect, Rect};

/// How far short of the tile's middle an arrow's tip stops, in pixels.
/// *Tunable.*
pub const TIP_SHORT: f32 = 3.0;

/// How long each of an arrowhead's two barbs is, in pixels. *Tunable.*
pub const BARB_LEN: f32 = 4.0;

/// The cosine and sine of the angle between a barb and the arrow's line
/// (about 35°). *Tunable.*
const BARB_TURN: (f32, f32) = (0.82, 0.57);

/// The mark's width, in pixels.
pub const MARK_W: i32 = 3;

/// The height of the mark's bar, of the gap under it and of its dot.
pub const MARK_BAR_H: i32 = 6;
const MARK_GAP_H: i32 = 1;
const MARK_DOT_H: i32 = 2;

/// Pixels between the mark's lowest pixel and the tile's top edge, so that
/// it clears the head of the unit standing there (Nick: lower, "it covers
/// the enemy's head and looks weird"). *Tunable.*
pub const MARK_ABOVE: i32 = 3;

/// The mark's height, bar to dot.
pub const MARK_H: i32 = MARK_BAR_H + MARK_GAP_H + MARK_DOT_H;

/// Where on its enemy's tile an arrow starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Start {
    /// In the tile's middle, over the unit's picture.
    Middle,
    /// Where the arrow's line leaves the tile, so that it never covers
    /// the unit's letters (as the path line starts at its unit's edge).
    Edge,
}

/// The colours of the arrows and marks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Colors {
    /// The arrow's line and barbs.
    pub arrow: Rgb,
    /// The mark over an enemy.
    pub mark: Rgb,
    /// The dark edge round both.
    pub edge: Rgb,
}

/// The pixels of the straight line from `a` to `b`, both included, as
/// rectangles one pixel thick: one for each run of pixels in a row (or in a
/// column, for a line more upright than flat), from `a`'s end.
fn line(a: (i32, i32), b: (i32, i32)) -> Vec<PxRect> {
    let (dx, dy) = ((b.0 - a.0).abs(), (b.1 - a.1).abs());
    let (sx, sy) = ((b.0 - a.0).signum(), (b.1 - a.1).signum());
    let flat = dx >= dy;
    let (long, short) = if flat { (dx, dy) } else { (dy, dx) };
    // A run from `from` to `to`, which share a row or a column.
    let run = |from: (i32, i32), to: (i32, i32)| {
        let (x, y) = (from.0.min(to.0), from.1.min(to.1));
        Rect::new(x, y, (to.0 - from.0).abs() + 1, (to.1 - from.1).abs() + 1)
    };
    let mut runs = Vec::new();
    let (mut from, mut at) = (a, a);
    let mut err = long / 2;
    for _ in 0..long {
        err -= short;
        let turn = err < 0;
        if turn {
            err += long;
            runs.push(run(from, at));
        }
        at = match (flat, turn) {
            (true, false) => (at.0 + sx, at.1),
            (false, false) => (at.0, at.1 + sy),
            (_, true) => (at.0 + sx, at.1 + sy),
        };
        if turn {
            from = at;
        }
    }
    runs.push(run(from, at));
    runs
}

/// `r` with one more pixel on every side.
const fn grown(r: PxRect) -> PxRect {
    Rect::new(r.x - 1, r.y - 1, r.w + 2, r.h + 2)
}

/// The furthest from pixel 0 an arrow's end may be, each way: far past any
/// console, and near enough that a [`line`] to it is quick to work out.
const FAR: f32 = 32_000.0;

/// The nearest whole pixel to `(x, y)`, no further than [`FAR`] out.
#[allow(clippy::cast_possible_truncation)] // within `FAR`
fn pixel(x: f32, y: f32) -> (i32, i32) {
    let whole = |v: f32| v.round().clamp(-FAR, FAR) as i32;
    (whole(x), whole(y))
}

/// The arrow from tile `from` (its middle or its edge: `start`) to just
/// short of the middle of tile `to`, on `art` (a grid at the art's size):
/// its line, then its two barbs, as [`line`]s. Nothing for an enemy on the
/// tile itself.
fn arrow(from: Pos, to: Pos, art: &Grid, start: Start) -> Vec<PxRect> {
    let middle = |tile: Pos| {
        let (x, y) = art.tile_px(tile);
        (x + art.tile.0 / 2, y + art.tile.1 / 2)
    };
    let (mid, end) = (middle(from), middle(to));
    #[allow(clippy::cast_precision_loss)] // console pixels
    let (dx, dy) = ((end.0 - mid.0) as f32, (end.1 - mid.1) as f32);
    if from == to {
        return Vec::new();
    }
    let len = dx.hypot(dy);
    // One pixel along the arrow.
    let (ux, uy) = (dx / len, dy / len);
    #[allow(clippy::cast_precision_loss)]
    let (tx, ty) = (end.0 as f32 - ux * TIP_SHORT, end.1 as f32 - uy * TIP_SHORT);
    let tip = pixel(tx, ty);
    let (cos, sin) = BARB_TURN;
    // Back along the arrow, turned to each side.
    let barb = |sin: f32| {
        let (bx, by) = (-ux * cos + uy * sin, -uy * cos - ux * sin);
        pixel(tx + bx * BARB_LEN, ty + by * BARB_LEN)
    };
    let first = match start {
        Start::Middle => mid,
        Start::Edge => {
            // The first pixel along the arrow that is off the enemy's tile.
            let (x, y) = art.tile_px(from);
            let tile = Rect::new(x, y, art.tile.0, art.tile.1);
            #[allow(clippy::cast_precision_loss)]
            let along = |k: i32| pixel(mid.0 as f32 + ux * k as f32, mid.1 as f32 + uy * k as f32);
            let reach = art.tile.0.max(art.tile.1);
            let out = (1..=reach).map(along).find(|&(x, y)| !tile.contains(x, y));
            out.unwrap_or(mid)
        }
    };
    let mut rects = line(first, tip);
    rects.extend(line(tip, barb(sin)));
    rects.extend(line(tip, barb(-sin)));
    rects
}

/// The mark over tile `tile` of `art` (a grid at the art's size, its view's
/// top at pixel 0): its bar and its dot, centred across the tile, the dot's
/// last row [`MARK_ABOVE`] pixels clear of the tile's top; moved down to
/// keep its dark edge inside the view on the view's top rows.
fn mark(tile: Pos, art: &Grid) -> [PxRect; 2] {
    let (x, y) = art.tile_px(tile);
    let x = x + (art.tile.0 - MARK_W) / 2;
    // Its top row; row 0 is the dark edge's on the view's top row.
    let top = (y - MARK_ABOVE - MARK_H).max(1);
    [
        Rect::new(x, top, MARK_W, MARK_BAR_H),
        Rect::new(x, top + MARK_BAR_H + MARK_GAP_H, MARK_W, MARK_DOT_H),
    ]
}

/// The overlays drawing `threat` on `grid`, all `Over` the glyphs: every
/// arrow's dark edge, then the arrows (each from its enemy's tile at
/// `start`); then every mark's dark edge and the marks. Clipped to the
/// tiles of `grid`. On a zoomed grid every pixel is a square of the zoom's.
pub fn threat_overlays(
    threat: &ThreatView,
    grid: &Grid,
    start: Start,
    colors: Colors,
) -> Vec<Overlay> {
    let view = grid.bounds();
    let art = grid.at_art_size();
    let mut out = Vec::new();
    let mut add = |rect: PxRect, color: Rgb| {
        if let Some(rect) = grid.zoomed(grid.corner, rect).intersect(&view) {
            out.push(Overlay::new(rect, color, Layer::Over));
        }
    };
    let arrows: Vec<PxRect> = threat
        .enemies
        .iter()
        .flat_map(|&enemy| arrow(enemy, threat.tile, &art, start))
        .collect();
    for &rect in &arrows {
        add(grown(rect), colors.edge);
    }
    for &rect in &arrows {
        add(rect, colors.arrow);
    }
    let shown = threat.enemies.iter().filter(|&&e| art.offset(e).is_some());
    let marks: Vec<[PxRect; 2]> = shown.map(|&enemy| mark(enemy, &art)).collect();
    for [bar, _] in &marks {
        // One dark box behind both.
        let both = Rect::new(bar.x, bar.y, MARK_W, MARK_H);
        add(grown(both), colors.edge);
    }
    for part in marks.iter().flatten() {
        add(*part, colors.mark);
    }
    out
}

/// Paints `scene`'s threat arrows and marks, if it has any, on `grid`, in
/// `colors` ([`Looks::threat`](super::Looks::threat)), each arrow from its
/// enemy's tile at `start`.
pub fn paint<U>(
    scene: &MapScene<U>,
    grid: &Grid,
    start: Start,
    colors: Colors,
    buf: &mut GlyphBuffer,
) {
    let Some(threat) = &scene.threat else {
        return;
    };
    for overlay in threat_overlays(threat, grid, start, colors) {
        buf.add_overlay(overlay);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const COLORS: Colors = Colors {
        arrow: Rgb::new(255, 0, 0),
        mark: Rgb::new(255, 255, 0),
        edge: Rgb::new(0, 0, 0),
    };

    fn p(x: i32, y: i32) -> Pos {
        Pos::new(x, y)
    }

    /// A 10 × 10 view from tile (0, 0) at pixel (0, 0), 16 px tiles.
    fn grid() -> Grid {
        Grid::top_down(p(0, 0), (0, 0), (16, 16), (10, 10), 1)
    }

    fn threat(tile: Pos, enemies: &[Pos]) -> ThreatView {
        ThreatView {
            tile,
            enemies: enemies.to_vec(),
        }
    }

    /// The overlays of `threat` on `grid` in `color`, as `(x, y, w, h)`.
    fn rects(threat: &ThreatView, grid: &Grid, color: Rgb) -> Vec<(i32, i32, i32, i32)> {
        let overlays = threat_overlays(threat, grid, Start::Middle, COLORS);
        assert!(overlays.iter().all(|o| o.layer == Layer::Over));
        let of = overlays.iter().filter(|o| o.color == color);
        of.map(|o| (o.rect.x, o.rect.y, o.rect.w, o.rect.h))
            .collect()
    }

    /// Every pixel the rectangles cover.
    fn pixels(rects: &[PxRect]) -> Vec<(i32, i32)> {
        let all = rects.iter().flat_map(|r| {
            (r.y..r.y + r.h).flat_map(move |y| (r.x..r.x + r.w).map(move |x| (x, y)))
        });
        all.collect()
    }

    #[test]
    fn a_line_is_one_rectangle_per_run_of_pixels() {
        // Flat and upright: one run, whichever way it is drawn.
        assert_eq!(line((2, 5), (9, 5)), [Rect::new(2, 5, 8, 1)]);
        assert_eq!(line((9, 5), (2, 5)), [Rect::new(2, 5, 8, 1)]);
        assert_eq!(line((4, 1), (4, 6)), [Rect::new(4, 1, 1, 6)]);
        assert_eq!(line((4, 6), (4, 1)), [Rect::new(4, 1, 1, 6)]);
        assert_eq!(line((3, 3), (3, 3)), [Rect::new(3, 3, 1, 1)]);
        // 9 across, 2 down: three runs, the middle one longest.
        assert_eq!(
            line((0, 0), (9, 2)),
            [
                Rect::new(0, 0, 3, 1),
                Rect::new(3, 1, 4, 1),
                Rect::new(7, 2, 3, 1)
            ]
        );
        // The same line the other way up and backwards.
        assert_eq!(
            line((9, 0), (0, 2)),
            [
                Rect::new(7, 0, 3, 1),
                Rect::new(3, 1, 4, 1),
                Rect::new(0, 2, 3, 1)
            ]
        );
        // More upright than flat: runs down columns.
        assert_eq!(
            line((0, 0), (1, 5)),
            [Rect::new(0, 0, 1, 3), Rect::new(1, 3, 1, 3)]
        );
        assert_eq!(
            line((1, 5), (0, 0)),
            [Rect::new(1, 3, 1, 3), Rect::new(0, 0, 1, 3)]
        );
        // The diagonal: a pixel a run.
        let diagonal = line((0, 0), (3, 3));
        assert_eq!(pixels(&diagonal), [(0, 0), (1, 1), (2, 2), (3, 3)]);
        assert_eq!(diagonal.len(), 4);
    }

    #[test]
    fn a_pixel_is_the_nearest_one_and_never_far_off_the_console() {
        assert_eq!(pixel(2.4, -2.6), (2, -3));
        assert_eq!(pixel(2.5, 7.0), (3, 7));
        assert_eq!(pixel(f32::INFINITY, f32::NEG_INFINITY), (32_000, -32_000));
        assert_eq!(pixel(1e9, -1e9), (32_000, -32_000));
        assert_eq!(pixel(f32::NAN, 31_999.6), (0, 32_000));
    }

    #[test]
    fn a_line_has_one_pixel_per_step_from_end_to_end() {
        for (a, b) in [
            ((0, 0), (40, 13)),
            ((40, 13), (0, 0)),
            ((5, 30), (9, -7)),
            ((-3, 2), (20, 2)),
            ((7, 7), (-9, 23)),
        ] {
            let runs = line(a, b);
            assert!(runs.iter().all(|r| r.w == 1 || r.h == 1), "{a:?} {b:?}");
            let px = pixels(&runs);
            let steps = (b.0 - a.0).abs().max((b.1 - a.1).abs());
            assert_eq!(px.len(), usize::try_from(steps).unwrap() + 1, "{a:?} {b:?}");
            assert!(px.contains(&a) && px.contains(&b), "{a:?} {b:?}");
            // Each run starts a pixel on, across and along, from the last.
            for pair in runs.windows(2) {
                let grown = grown(pair[0]);
                assert!(grown.intersect(&pair[1]).is_some(), "{pair:?}");
                assert_eq!(pair[0].intersect(&pair[1]), None, "{pair:?}");
            }
        }
    }

    #[test]
    fn an_arrow_runs_from_the_enemys_middle_to_just_short_of_the_tiles() {
        // The enemy three tiles east of the tile, on its row: tile (1, 1)
        // spans pixels 16..32, its middle is (24, 24); the enemy's, (72, 24).
        let t = threat(p(1, 1), &[p(4, 1)]);
        let red = rects(&t, &grid(), COLORS.arrow);
        // The line stops 3 px short of the middle; the barbs go back from
        // its tip, down and up: 4 px long, about 35° off the line.
        assert_eq!(red[0], (27, 24, 46, 1));
        assert_eq!(
            &red[1..],
            [(27, 24, 1, 1), (28, 25, 2, 1), (30, 26, 1, 1)]
                .into_iter()
                .chain([(27, 24, 1, 1), (28, 23, 2, 1), (30, 22, 1, 1)])
                .collect::<Vec<_>>()
        );
        // The dark edge: every rectangle a pixel bigger each way, under
        // all the red.
        let all = threat_overlays(&t, &grid(), Start::Middle, COLORS);
        let dark: Vec<_> = all.iter().take_while(|o| o.color == COLORS.edge).collect();
        assert_eq!(dark.len(), red.len());
        assert_eq!(dark[0].rect, Rect::new(26, 23, 48, 3));
        for (edge, line) in dark.iter().zip(&red) {
            let r = edge.rect;
            assert_eq!((r.x + 1, r.y + 1, r.w - 2, r.h - 2), *line);
        }
        let reds = &all[dark.len()..dark.len() + red.len()];
        assert!(reds.iter().all(|o| o.color == COLORS.arrow));
    }

    #[test]
    fn arrows_point_at_the_tile_from_any_side() {
        // The tip, where the line ends and both barbs start: 3 px from the
        // middle of tile (4, 4), (72, 72), towards the enemy.
        let tip = |enemy: Pos| {
            let px = pixels(&arrow(enemy, p(4, 4), &grid(), Start::Middle));
            // From the enemy's middle.
            assert!(px.contains(&(enemy.x * 16 + 8, enemy.y * 16 + 8)));
            let thrice = |q: &&(i32, i32)| px.iter().filter(|r| r == q).count() == 3;
            *px.iter().find(thrice).unwrap()
        };
        assert_eq!(tip(p(7, 4)), (75, 72));
        assert_eq!(tip(p(1, 4)), (69, 72));
        assert_eq!(tip(p(4, 1)), (72, 69));
        assert_eq!(tip(p(4, 8)), (72, 75));
        // Diagonally: 3 px back along the line, to the nearest pixel.
        assert_eq!(tip(p(6, 6)), (74, 74));
        assert_eq!(tip(p(2, 2)), (70, 70));
        assert_eq!(tip(p(6, 2)), (74, 70));
        // An enemy next to the tile still has its arrow.
        assert_eq!(tip(p(5, 4)), (75, 72));
        // One on the tile itself (it can't be) has none.
        for start in [Start::Middle, Start::Edge] {
            assert!(arrow(p(4, 4), p(4, 4), &grid(), start).is_empty());
        }
        let none = threat(p(4, 4), &[p(4, 4)]);
        assert!(rects(&none, &grid(), COLORS.arrow).is_empty());
    }

    #[test]
    fn barbs_are_four_pixels_back_from_the_tip_on_both_sides() {
        let t = threat(p(4, 4), &[p(4, 1)]);
        let red = rects(&t, &grid(), COLORS.arrow);
        // Down the column x 72 from the enemy's middle (y 24) to the tip.
        assert_eq!(red[0], (72, 24, 1, 46));
        // Each barb: from the tip (72, 69) up and out, to x 70 and x 74.
        let barb = |x: i32, end: i32| [(72, 69, 1, 1), (x, 67, 1, 2), (end, 66, 1, 1)];
        assert_eq!(red[1..4], barb(73, 74));
        assert_eq!(red[4..], barb(71, 70));
    }

    #[test]
    fn an_arrow_from_the_edge_starts_on_the_first_pixel_off_the_enemys_tile() {
        // The first run of the arrow from each side of tile (4, 4), which
        // spans pixels 64..80 both ways: the enemy's tile is left clear.
        let first = |enemy: Pos, start| {
            let all = arrow(enemy, p(4, 4), &grid(), start);
            let (x, y) = grid().tile_px(enemy);
            let tile = Rect::new(x, y, 16, 16);
            let clear = pixels(&all).iter().all(|&(x, y)| !tile.contains(x, y));
            assert_eq!(clear, start == Start::Edge, "{enemy:?}");
            all[0]
        };
        // East, on the row: from the middle of the enemy's tile (x 120)...
        assert_eq!(first(p(7, 4), Start::Middle), Rect::new(75, 72, 46, 1));
        // ...or from the pixel left of that tile (x 112..128).
        assert_eq!(first(p(7, 4), Start::Edge), Rect::new(75, 72, 37, 1));
        assert_eq!(first(p(1, 4), Start::Edge), Rect::new(32, 72, 38, 1));
        assert_eq!(first(p(4, 1), Start::Edge), Rect::new(72, 32, 1, 38));
        assert_eq!(first(p(4, 8), Start::Edge), Rect::new(72, 75, 1, 53));
        // Diagonally: out through the tile's corner.
        assert_eq!(first(p(6, 6), Start::Edge), Rect::new(95, 95, 1, 1));
        assert_eq!(first(p(2, 2), Start::Edge), Rect::new(48, 48, 1, 1));
        // Next to the tile, there are 5 px left of the arrow, and its barbs.
        assert_eq!(first(p(5, 4), Start::Edge), Rect::new(75, 72, 5, 1));
        let barbs = arrow(p(5, 4), p(4, 4), &grid(), Start::Edge);
        assert_eq!(barbs.len(), 7);
        // The rest of the arrow is the same from either start.
        let (edge, middle) = (
            arrow(p(7, 4), p(4, 4), &grid(), Start::Edge),
            arrow(p(7, 4), p(4, 4), &grid(), Start::Middle),
        );
        assert_eq!(edge[1..], middle[1..]);
    }

    #[test]
    fn a_mark_sits_above_each_enemy_clear_of_its_head() {
        // Tile (4, 3) spans x 64..80 from y 48: the mark is 3 px wide in
        // its middle; its dot ends 3 px clear of the tile's top.
        let t = threat(p(1, 5), &[p(4, 3), p(6, 5)]);
        let yellow = rects(&t, &grid(), COLORS.mark);
        assert_eq!(
            yellow,
            [
                (70, 36, 3, 6),
                (70, 43, 3, 2),
                (102, 68, 3, 6),
                (102, 75, 3, 2)
            ]
        );
        assert_eq!((MARK_H, 48 - MARK_ABOVE - MARK_H), (9, 36));
        // One dark box behind each, a pixel bigger each way; the boxes
        // before the marks, after every arrow.
        let all = threat_overlays(&t, &grid(), Start::Middle, COLORS);
        let n = all.len();
        let tail: Vec<_> = all[n - 6..].iter().map(|o| (o.rect, o.color)).collect();
        assert_eq!(tail[0], (Rect::new(69, 35, 5, 11), COLORS.edge));
        assert_eq!(tail[1], (Rect::new(101, 67, 5, 11), COLORS.edge));
        assert!(tail[2..].iter().all(|&(_, c)| c == COLORS.mark));
        assert!(all[..n - 6].iter().all(|o| o.color != COLORS.mark));
    }

    #[test]
    fn a_mark_on_the_views_top_row_moves_down_to_stay_inside() {
        let t = threat(p(1, 5), &[p(4, 0)]);
        let yellow = rects(&t, &grid(), COLORS.mark);
        assert_eq!(yellow, [(70, 1, 3, 6), (70, 8, 3, 2)]);
        let dark = rects(&t, &grid(), COLORS.edge);
        assert_eq!(dark[dark.len() - 1], (69, 0, 5, 11));
        // The second row has room for all but one row of it.
        let t = threat(p(1, 5), &[p(4, 1)]);
        let yellow = rects(&t, &grid(), COLORS.mark);
        assert_eq!(yellow, [(70, 4, 3, 6), (70, 11, 3, 2)]);
    }

    #[test]
    fn an_enemy_outside_the_view_has_its_arrow_cut_at_the_edge_and_no_mark() {
        // A 4 × 4 view of 16 px tiles from tile (2, 2), at pixel (8, 4).
        let view = Grid::top_down(p(2, 2), (8, 4), (16, 16), (4, 4), 1);
        // The enemy is on the tile's row, three tiles past the right edge.
        let t = threat(p(3, 3), &[p(8, 3)]);
        assert!(rects(&t, &view, COLORS.mark).is_empty());
        let red = rects(&t, &view, COLORS.arrow);
        // From the tip, 3 px right of the tile's middle (32, 28), to the
        // view's right edge at x 72.
        assert_eq!(red[0], (35, 28, 37, 1));
        let bounds = view.bounds();
        assert_eq!(bounds, Rect::new(8, 4, 64, 64));
        for o in threat_overlays(&t, &view, Start::Middle, COLORS) {
            assert_eq!(o.rect.intersect(&bounds), Some(o.rect), "{o:?}");
        }
        // The same enemy in a view that holds it: the whole arrow, marked.
        let wide = Grid {
            tiles: (8, 4),
            ..view
        };
        assert_eq!(rects(&t, &wide, COLORS.arrow)[0], (35, 28, 78, 1));
        assert_eq!(rects(&t, &wide, COLORS.mark).len(), 2);
        // The tile itself outside the view: what crosses the view is drawn.
        let t = threat(p(0, 3), &[p(4, 3)]);
        let red = rects(&t, &view, COLORS.arrow);
        assert_eq!(red, [(8, 28, 41, 1)]);
        assert_eq!(rects(&t, &view, COLORS.mark).len(), 2);
    }

    #[test]
    fn on_a_zoomed_grid_every_pixel_is_a_square_of_the_zooms() {
        let t = threat(p(1, 3), &[p(4, 1), p(3, 0)]);
        let once = threat_overlays(&t, &grid(), Start::Edge, COLORS);
        let zoomed = Grid {
            corner: (8, 16),
            tile: (32, 32),
            scale: 2,
            ..grid()
        };
        let twice = threat_overlays(&t, &zoomed, Start::Edge, COLORS);
        let doubled: Vec<Overlay> = once
            .iter()
            .map(|o| {
                let r = o.rect;
                let rect = Rect::new(8 + 2 * r.x, 16 + 2 * r.y, 2 * r.w, 2 * r.h);
                Overlay::new(rect, o.color, o.layer)
            })
            .collect();
        assert_eq!(twice, doubled);
        assert!(!twice.is_empty());
    }

    #[test]
    fn the_geometry_follows_the_tile_size() {
        // 24 × 20 tiles: tile (1, 1) spans 24..48 × 20..40, middle (36, 30).
        let big = Grid {
            tile: (24, 20),
            ..grid()
        };
        let t = threat(p(1, 1), &[p(3, 1)]);
        assert_eq!(rects(&t, &big, COLORS.arrow)[0], (39, 30, 46, 1));
        // The mark in the middle of tile (3, 1): x 72..96.
        assert_eq!(rects(&t, &big, COLORS.mark)[0], (82, 8, 3, 6));
    }

    #[test]
    fn paint_adds_the_scenes_threat_over_everything_and_nothing_without_one() {
        let mut scene = MapScene::<crate::toy::grid::Piece>::new(p(0, 0), (10, 10));
        let blank = || {
            let c = crate::frame::glyph_buffer::Cell::new('.', COLORS.edge, COLORS.edge);
            GlyphBuffer::new(20, 10, c)
        };
        let mut buf = blank();
        paint(&scene, &grid(), Start::Middle, COLORS, &mut buf);
        assert_eq!(buf, blank());
        let t = threat(p(1, 1), &[p(4, 2), p(0, 5)]);
        scene.threat = Some(t.clone());
        paint(&scene, &grid(), Start::Edge, COLORS, &mut buf);
        assert_eq!(
            buf.overlays(),
            threat_overlays(&t, &grid(), Start::Edge, COLORS)
        );
        assert!(!buf.overlays().is_empty());
    }
}
