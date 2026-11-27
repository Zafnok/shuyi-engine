//! A key cap printed into a [`GlyphBuffer`] (ADR-0124): its
//! cells, its glyphs' style and its box.

use crate::prelude::*;

use shuyi_assets::lang::text_width;

use super::{Cell, GlyphBuffer, Layer, Overlay, Rect};
use crate::frame::color::Rgb;
use crate::frame::key_cap::{CapLayout, KeyCaps};
use crate::text::TextRole;

impl GlyphBuffer {
    /// Prints `name` as a key cap from column `cx`, which it moves past
    /// the cap: the name in the key's colour on plain text's background,
    /// then the cap's extra cells as spaces; then, if the whole cap is on
    /// the buffer, the name's glyph style and the box. Returns how many
    /// cells were written.
    pub(super) fn print_cap(
        &mut self,
        cx: &mut i64,
        y: i32,
        name: &str,
        caps: &KeyCaps,
        colors: &impl Fn(Option<TextRole>) -> (Rgb, Option<Rgb>),
    ) -> u16 {
        let shown = caps.shown(name);
        let cells = u16::try_from(text_width(&shown, &self.widths)).unwrap_or(u16::MAX);
        let layout = caps.layout(&shown, &self.widths);
        let (fg, _) = colors(Some(TextRole::Key));
        let (_, bg) = colors(None);
        let paint = |c: &mut Cell, glyph: char| {
            c.glyph = glyph;
            c.fg = fg;
            if let Some(bg) = bg {
                c.bg = bg;
                c.see_through = false;
            }
        };
        let start = *cx;
        let mut written = self.print_from(cx, y, &shown, paint);
        let pad = " ".repeat(usize::from(layout.extra));
        written += self.print_from(cx, y, &pad, paint);
        let end = start + i64::from(cells) + i64::from(layout.extra);
        let on_buffer = (0..i32::from(self.spec.rows)).contains(&y)
            && start >= 0
            && end <= i64::from(self.spec.cols);
        if let (true, Ok(x)) = (on_buffer, i32::try_from(start)) {
            for at in x..x + i32::from(cells) {
                self.set_glyph_style(at, y, layout.style);
            }
            self.add_cap_box(x, y, &layout, caps);
        }
        written
    }

    /// Prints the arrow cluster's picture from column `cx`, which it moves
    /// past the picture's cells: spaces on plain text's background, and
    /// the picture over them, clipped to the buffer like any sprite.
    /// Returns how many cells were written.
    pub(super) fn print_arrows(
        &mut self,
        cx: &mut i64,
        y: i32,
        caps: &KeyCaps,
        colors: &impl Fn(Option<TextRole>) -> (Rgb, Option<Rgb>),
    ) -> u16 {
        let cells = caps.arrows_cells().unwrap_or(0);
        let (_, bg) = colors(None);
        let start = *cx;
        let blank = " ".repeat(usize::from(cells));
        let written = self.print_from(cx, y, &blank, |c, glyph| {
            c.glyph = glyph;
            if let Some(bg) = bg {
                c.bg = bg;
                c.see_through = false;
            }
        });
        let on_rows = (0..i32::from(self.spec.rows)).contains(&y);
        if let (true, Ok(x)) = (on_rows, i32::try_from(start))
            && let Some(sprite) = caps.arrows_sprite(x, y)
        {
            self.add_sprite(sprite);
        }
        written
    }

    /// The box of a cap whose first cell is `(x, y)`: its fill, and its
    /// edge 1 px wide with the four corner pixels left out, a rounded
    /// corner. Rectangles under the glyphs.
    fn add_cap_box(&mut self, x: i32, y: i32, layout: &CapLayout, caps: &KeyCaps) {
        let (cw, ch) = (i32::from(self.spec.cell_w), i32::from(self.spec.cell_h));
        let f = layout.frame;
        let (left, top) = (x * cw + f.x, y * ch + f.y);
        let (right, bottom) = (left + f.w - 1, top + f.h - 1);
        let (fill, edge) = caps.colors();
        let mut boxes = Vec::with_capacity(5);
        if let Some(fill) = fill {
            boxes.push((Rect::new(left + 1, top + 1, f.w - 2, f.h - 2), fill));
        }
        if let Some(edge) = edge {
            boxes.extend([
                (Rect::new(left + 1, top, f.w - 2, 1), edge),
                (Rect::new(left + 1, bottom, f.w - 2, 1), edge),
                (Rect::new(left, top + 1, 1, f.h - 2), edge),
                (Rect::new(right, top + 1, 1, f.h - 2), edge),
            ]);
        }
        for (rect, color) in boxes {
            self.add_overlay(Overlay::new(rect, color, Layer::Under));
        }
    }
}
