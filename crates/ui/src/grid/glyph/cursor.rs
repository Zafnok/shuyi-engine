//! How the glyph skin draws the cursor (game ADR-0024): thin corner marks
//! around the tile, or a glow on it.

use super::{CELL_PX, Layout, TILE_W_CELLS};
use crate::frame::color::Rgb;
use crate::frame::glyph_buffer::{GlyphBuffer, Layer, Overlay, Rect};
use crate::grid::layout::px_rect;
use crate::grid::scene::{CursorStyle, CursorView};

/// How strongly the tile glow tints the tile at full brightness (`0` = not
/// at all, `1` = solid `cursor` colour). *Tunable.*
pub const GLOW_MAX: f32 = 0.3;

/// How long each arm of the corner marks is, in pixels.
pub const CORNER_ARM: i32 = 3;

/// How long each arm of the large corner marks is, in pixels.
pub const LARGE_CORNER_ARM: i32 = 4;

/// How the cursor looks: its color, the color it fades toward as it
/// pulses (the console's background, which a light theme makes light), and
/// how many pixels at a tile's foot its lower arms keep clear of (a game's
/// HP bar).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CursorInk {
    /// At full brightness.
    pub color: Rgb,
    /// What it fades toward.
    pub dark: Rgb,
    /// The pixels left clear at the foot of the tile.
    pub foot_px: i32,
}

/// Draws `cursor` in its style on the tile whose left cell is `(x, y)`,
/// clipped to the tiles of `layout`.
///
/// Corner marks are 1 px `Over` overlays that stay clear of every letter
/// (game ADR-0024): their vertical arms sit in the pixel column just left
/// of the tile and in the tile's last pixel column, which the font never
/// inks; their horizontal arms sit on the tile's top row and on the row
/// just above the tile's foot ([`CursorInk::foot_px`]), outside the
/// letters' rows. So a neighbour's letters always show.
pub fn draw_cursor(
    buf: &mut GlyphBuffer,
    ink: CursorInk,
    cursor: &CursorView,
    layout: &Layout,
    (x, y): (i32, i32),
) {
    let color = ink.color;
    let view = layout.cells();
    let arm = match cursor.style {
        CursorStyle::Corners => CORNER_ARM,
        CursorStyle::LargeCorners => LARGE_CORNER_ARM,
        CursorStyle::TileGlow => {
            let tile = Rect::new(x, y, TILE_W_CELLS, 1);
            if let Some(tile) = tile.intersect(&view) {
                buf.blend_bg(tile, color, GLOW_MAX * cursor.brightness);
            }
            return;
        }
    };
    // Fades toward the console background, which a light theme makes light.
    let fg = color.lerp(ink.dark, 1.0 - cursor.brightness);
    let view = px_rect(view);
    for r in corner_arms(x, y, arm, ink.foot_px) {
        if let Some(r) = r.intersect(&view) {
            buf.add_overlay(Overlay::new(r, fg, Layer::Over));
        }
    }
}

/// The eight 1 px arms of the corner marks around the tile whose left cell
/// is `(x, y)`, each `arm` px long, in console pixels, the lower ones just
/// above the `foot_px` pixels at the tile's foot.
pub fn corner_arms(x: i32, y: i32, arm: i32, foot_px: i32) -> [Rect; 8] {
    let (cw, ch) = CELL_PX;
    // Left arms in the left neighbour's last (always blank) pixel column,
    // right arms in this tile's last one; top arms on the tile's first row,
    // bottom arms on the row above the tile's foot.
    let left = x * cw - 1;
    let right = (x + TILE_W_CELLS) * cw - 1;
    let top = y * ch;
    let bottom = top + ch - foot_px - 1;
    let across = |x0, y0| Rect::new(x0, y0, arm, 1);
    let down = |x0, y0| Rect::new(x0, y0, 1, arm);
    [
        across(left, top),
        down(left, top),
        across(right - arm + 1, top),
        down(right, top),
        across(left, bottom),
        down(left, bottom - arm + 1),
        across(right - arm + 1, bottom),
        down(right, bottom - arm + 1),
    ]
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use shuyi_core::Pos;

    use super::*;
    use crate::frame::glyph_buffer::Cell;
    use crate::grid::cursor::Cursor;
    use crate::grid::glyph::tests::{AREA, layout_at};
    use crate::prelude::*;

    /// The cursor's color, and what it fades toward.
    const CURSOR: Rgb = Rgb::new(250, 240, 90);
    const DARK: Rgb = Rgb::new(0, 0, 0);

    /// Its look over a 2 px HP bar.
    const INK: CursorInk = CursorInk {
        color: CURSOR,
        dark: DARK,
        foot_px: 2,
    };

    /// Draws `cursor` in `style` on the tile whose left cell is `(x, y)` of
    /// a 70 × 30-cell map area.
    fn draw(buf: &mut GlyphBuffer, cursor: &Cursor, style: CursorStyle, x: i32, y: i32) {
        let view = CursorView {
            pos: cursor.pos,
            brightness: cursor.brightness(),
            style,
        };
        draw_cursor(buf, INK, &view, &layout_at(Pos::new(0, 0)), (x, y));
    }

    /// A 100 × 32 buffer of `.` on a dark background.
    fn dots() -> GlyphBuffer {
        GlyphBuffer::new(
            100,
            32,
            Cell::new('.', Rgb::new(1, 1, 1), Rgb::new(9, 9, 9)),
        )
    }

    fn rects(buf: &GlyphBuffer) -> Vec<Rect> {
        buf.overlays().iter().map(|o| o.rect).collect()
    }

    #[test]
    fn corner_marks_frame_the_tile_outside_the_letters() {
        let c = Cursor::new(Pos::new(0, 0));
        let mut buf = dots();
        let before = buf.clone();
        // Tile at cells (10, 4)-(11, 4): pixels x 80..96, y 64..80.
        draw(&mut buf, &c, CursorStyle::Corners, 10, 4);
        let r = Rect::new;
        assert_eq!(
            rects(&buf),
            [
                r(79, 64, 3, 1),
                r(79, 64, 1, 3),
                r(93, 64, 3, 1),
                r(95, 64, 1, 3),
                r(79, 77, 3, 1),
                r(79, 75, 1, 3),
                r(93, 77, 3, 1),
                r(95, 75, 1, 3),
            ]
        );
        for o in buf.overlays() {
            assert_eq!((o.color, o.layer), (CURSOR, Layer::Over));
        }
        for (cx, cy) in (0..100).flat_map(|cx| (0..32).map(move |cy| (cx, cy))) {
            assert_eq!(buf.get(cx, cy), before.get(cx, cy), "cell ({cx}, {cy})");
        }
    }

    #[test]
    fn the_lower_arms_sit_just_above_the_tiles_foot() {
        // With nothing at the foot: on the tile's last row.
        let arms = corner_arms(10, 4, 3, 0);
        assert_eq!(arms[4], Rect::new(79, 79, 3, 1));
        assert_eq!(arms[5], Rect::new(79, 77, 1, 3));
        // Above a 4 px bar.
        let arms = corner_arms(10, 4, 3, 4);
        assert_eq!(arms[6], Rect::new(93, 75, 3, 1));
        assert_eq!(arms[7], Rect::new(95, 73, 1, 3));
        // The upper arms never move.
        assert_eq!(corner_arms(10, 4, 3, 4)[..4], corner_arms(10, 4, 3, 0)[..4]);
    }

    #[test]
    fn large_corners_have_longer_arms() {
        let mut buf = dots();
        draw(
            &mut buf,
            &Cursor::new(Pos::new(0, 0)),
            CursorStyle::LargeCorners,
            10,
            4,
        );
        let r = Rect::new;
        assert_eq!(
            rects(&buf),
            [
                r(79, 64, 4, 1),
                r(79, 64, 1, 4),
                r(92, 64, 4, 1),
                r(95, 64, 1, 4),
                r(79, 77, 4, 1),
                r(79, 74, 1, 4),
                r(92, 77, 4, 1),
                r(95, 74, 1, 4),
            ]
        );
    }

    #[test]
    fn corner_marks_pulse_toward_the_background() {
        let mut c = Cursor::new(Pos::new(0, 0));
        c.tick(0.5);
        let mut buf = dots();
        draw(&mut buf, &c, CursorStyle::Corners, 10, 4);
        let dim = CURSOR.lerp(DARK, 0.5);
        assert!(buf.overlays().iter().all(|o| o.color == dim));
        // Toward the background, whatever it is.
        let light = CursorInk {
            dark: Rgb::new(255, 255, 255),
            ..INK
        };
        let view = CursorView {
            pos: c.pos,
            brightness: c.brightness(),
            style: CursorStyle::Corners,
        };
        let mut buf = dots();
        draw_cursor(&mut buf, light, &view, &layout_at(Pos::new(0, 0)), (10, 4));
        let pale = CURSOR.lerp(Rgb::new(255, 255, 255), 0.5);
        assert!(buf.overlays().iter().all(|o| o.color == pale));
    }

    #[test]
    fn tile_glow_tints_only_the_tile_and_pulses() {
        let mut c = Cursor::new(Pos::new(0, 0));
        let mut buf = dots();
        let bg = Rgb::new(9, 9, 9);
        draw(&mut buf, &c, CursorStyle::TileGlow, 10, 4);
        let bright = bg.lerp(CURSOR, GLOW_MAX);
        let cell = |b: &GlyphBuffer, x| *b.get(x, 4).unwrap();
        assert_eq!(cell(&buf, 10), Cell::new('.', Rgb::new(1, 1, 1), bright));
        assert_eq!(cell(&buf, 11), Cell::new('.', Rgb::new(1, 1, 1), bright));
        assert_eq!(cell(&buf, 9).bg, bg);
        assert_eq!(cell(&buf, 12).bg, bg);
        let found = buf.overlays();
        assert!(found.is_empty(), "{found:?}");
        c.tick(0.5);
        let mut dim = dots();
        draw(&mut dim, &c, CursorStyle::TileGlow, 10, 4);
        assert_eq!(cell(&dim, 10).bg, bg.lerp(CURSOR, GLOW_MAX * 0.5));
    }

    #[test]
    fn nothing_is_drawn_outside_the_map_area() {
        let c = Cursor::new(Pos::new(0, 0));
        let view = px_rect(AREA);
        for style in [CursorStyle::Corners, CursorStyle::LargeCorners] {
            for (x, y) in [(0, 0), (68, 29), (0, 29), (68, 0)] {
                let mut buf = dots();
                draw(&mut buf, &c, style, x, y);
                let found = buf.overlays();
                assert!(!found.is_empty(), "{found:?}");
                for r in rects(&buf) {
                    assert_eq!(
                        r.intersect(&view),
                        Some(r),
                        "{style:?} at ({x}, {y}): {r:?}"
                    );
                }
            }
        }
        // At the left edge the left arms' column is off the area: the
        // vertical ones go, the horizontal ones lose a pixel.
        let mut buf = dots();
        draw(&mut buf, &c, CursorStyle::Corners, 0, 0);
        assert_eq!(
            rects(&buf)[..2],
            [Rect::new(0, 0, 2, 1), Rect::new(13, 0, 3, 1)]
        );
        // Off the area entirely (rows below it): nothing at all.
        for style in [CursorStyle::Corners, CursorStyle::TileGlow] {
            let mut buf = dots();
            let before = buf.clone();
            draw(&mut buf, &c, style, 10, 30);
            assert_eq!(buf, before);
        }
    }

    proptest! {
        /// Whatever the style, pulse and position, the cursor changes no
        /// glyph or glyph color, and its overlays stay out of the letters:
        /// in the tile's rows 2..=11 (where letters are inked) only the
        /// pixel column left of the tile and the tile's last one are used.
        #[test]
        fn never_covers_letters(
            style in prop::sample::select(vec![
                CursorStyle::Corners,
                CursorStyle::LargeCorners,
                CursorStyle::TileGlow,
            ]),
            t in 0.0f32..1.0,
            tx in 0i32..35,
            ty in 0i32..30,
        ) {
            let mut c = Cursor::new(Pos::new(0, 0));
            c.tick(t);
            let mut buf = dots();
            let before = buf.clone();
            let (x, y) = (tx * TILE_W_CELLS, ty);
            draw(&mut buf, &c, style, x, y);
            for cy in 0..32 {
                for cx in 0..100 {
                    let (a, b) = (before.get(cx, cy).unwrap(), buf.get(cx, cy).unwrap());
                    prop_assert_eq!((a.glyph, a.fg), (b.glyph, b.fg));
                }
            }
            let (cw, ch) = CELL_PX;
            for r in rects(&buf) {
                for py in r.y..r.y + r.h {
                    for px in r.x..r.x + r.w {
                        let (rx, ry) = (px - x * cw, py - y * ch);
                        prop_assert!((-1..=15).contains(&rx) && (0..=13).contains(&ry));
                        prop_assert!(
                            !(2..=11).contains(&ry) || rx == -1 || rx == 15,
                            "pixel ({}, {}) of the tile", rx, ry
                        );
                    }
                }
            }
        }
    }
}
