//! Key caps (ticket 3215, ADR-0124): a key's name in styled text drawn as
//! a key, in a small rounded box, bold, with an even margin round its
//! letters, and the room the box needs beside it.
//!
//! A game turns them on with [`KeyCapStyle`]
//! ([`GameData::key_caps`](crate::GameData::key_caps)); the engine makes
//! it a [`KeyCaps`] with the palette's colours and the font's ink, and the
//! frame's [`GlyphBuffer`](crate::frame::GlyphBuffer) draws every
//! [`TextRole::Key`](crate::text::TextRole::Key) span as one
//! ([`GlyphBuffer::print_styled`](crate::frame::GlyphBuffer::print_styled)).
//!
//! A style may also name a picture of the arrow cluster
//! ([`KeyCapStyle::arrows`], ticket 3217, ADR-0129): a
//! [`TextRole::Arrows`](crate::text::TextRole::Arrows) span is then drawn
//! as that picture, a [`Sprite`] in the cells it takes.

use crate::prelude::*;
use alloc::borrow::Cow;
use alloc::rc::Rc;

use shuyi_assets::font::{CellWidths, FontInk};
use shuyi_assets::image::ImageId;

use crate::frame::color::{KEY_BG, KEY_EDGE, Palette, Rgb};
use crate::frame::console::ConsoleSpec;
use crate::frame::glyph_buffer::{GlyphStyle, Layer, PxRect, Rect, Sprite};

/// How a game draws a key's name in text as a key cap: what its theme
/// turns on ([`GameData::key_caps`](crate::GameData::key_caps)). The box's
/// colours are the palette's: its fill [`KEY_BG`], its 1 px edge
/// [`KEY_EDGE`] (else the fill's), the name [`KEY`](crate::frame::color::KEY).
///
/// Kind 2 of ADR-0039: build it with [`KeyCapStyle::new`] and the `with_`
/// functions.
///
/// ```
/// use shuyi_ui::frame::key_cap::KeyCapStyle;
/// const CAPS: KeyCapStyle = KeyCapStyle::new().with_margin_px(1).with_bold(false);
/// assert!(CAPS.capitals && !CAPS.bold);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct KeyCapStyle {
    /// The name's glyphs in bold: from the font atlas's bold face, else
    /// each drawn twice, 1 px apart (ticket 3216).
    pub bold: bool,
    /// A name of one letter drawn as its capital (`f` as `F`); longer
    /// names as the keymap writes them. The text itself is unchanged.
    pub capitals: bool,
    /// Pixels between the glyphs' ink and the box's edge, on all four
    /// sides (the box is never taller than the line: a margin too big for
    /// it is less above and below).
    pub margin_px: u8,
    /// Whether caps are drawn on a one-bit screen (the Playdate) too. Off:
    /// a one-bit target draws keys as text, in the pattern map's colours.
    pub one_bit: bool,
    /// A picture of the four arrow keys together (an upside-down T of
    /// caps), drawn for the cursor's keys when they are the arrows
    /// ([`TextRole::Arrows`](crate::text::TextRole::Arrows)): an image and
    /// the part of it to show, in image pixels. It is scaled to one line's
    /// height, keeping its shape, and takes the cells its width needs.
    /// `None`: the arrow keys are drawn as a cap of their word.
    pub arrows: Option<(ImageId, PxRect)>,
}

impl KeyCapStyle {
    /// Bold, capitals, a 2 px margin, not on a one-bit screen.
    pub const fn new() -> Self {
        Self {
            bold: true,
            capitals: true,
            margin_px: 2,
            one_bit: false,
            arrows: None,
        }
    }

    /// The same style, bold or not ([`bold`](Self::bold)).
    #[must_use]
    pub const fn with_bold(mut self, bold: bool) -> Self {
        self.bold = bold;
        self
    }

    /// The same style, with capitals or not ([`capitals`](Self::capitals)).
    #[must_use]
    pub const fn with_capitals(mut self, capitals: bool) -> Self {
        self.capitals = capitals;
        self
    }

    /// The same style with a margin of `margin_px` ([`margin_px`](Self::margin_px)).
    #[must_use]
    pub const fn with_margin_px(mut self, margin_px: u8) -> Self {
        self.margin_px = margin_px;
        self
    }

    /// The same style, on a one-bit screen or not ([`one_bit`](Self::one_bit)).
    #[must_use]
    pub const fn with_one_bit(mut self, one_bit: bool) -> Self {
        self.one_bit = one_bit;
        self
    }

    /// The same style, drawing the arrow keys as the part `src` of `image`
    /// ([`arrows`](Self::arrows)).
    #[must_use]
    pub const fn with_arrows(mut self, image: ImageId, src: PxRect) -> Self {
        self.arrows = Some((image, src));
        self
    }
}

impl Default for KeyCapStyle {
    fn default() -> Self {
        Self::new()
    }
}

/// A [`KeyCapStyle`] ready to draw: with the palette's colours, the
/// font's ink and the cell's size. What a
/// [`GlyphBuffer`](crate::frame::GlyphBuffer) draws caps with
/// ([`set_key_caps`](crate::frame::GlyphBuffer::set_key_caps)).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KeyCaps {
    style: KeyCapStyle,
    fill: Option<Rgb>,
    edge: Option<Rgb>,
    /// Where the glyphs drawn have ink: the bold face's when it draws
    /// them, else the regular face's; `None` where the font doesn't say.
    ink: Option<Rc<FontInk>>,
    /// Bold drawn as two strikes, 1 px apart: the ink a column wider.
    struck: bool,
    cell_w: i32,
    cell_h: i32,
}

/// Where a cap goes, from its first cell's top-left corner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CapLayout {
    /// The cells it takes beyond the name's.
    pub extra: u16,
    /// The box, in console pixels.
    pub frame: PxRect,
    /// How the name's glyphs are drawn.
    pub style: GlyphStyle,
}

impl KeyCaps {
    /// `style` in `palette`'s colours, placed by the ink `widths` carry
    /// ([`CellWidths::ink`]) in cells of `spec`'s size.
    pub fn new(
        style: KeyCapStyle,
        palette: &Palette,
        widths: &CellWidths,
        spec: ConsoleSpec,
    ) -> Self {
        let fill = palette.lookup(KEY_BG);
        let edge = palette.lookup(KEY_EDGE).or(fill);
        Self::with_colors(style, (fill, edge), widths, spec)
    }

    /// `style` with box fill and edge colours `(fill, edge)` (`None`: not
    /// drawn), placed as [`new`](Self::new) places it. Bold is drawn
    /// from the atlas's bold face and placed by its ink
    /// ([`CellWidths::ink`]); without one it is the regular face struck
    /// twice, its ink a column wider. Where the font's ink isn't known,
    /// the whole cell counts as ink.
    pub fn with_colors(
        style: KeyCapStyle,
        (fill, edge): (Option<Rgb>, Option<Rgb>),
        widths: &CellWidths,
        spec: ConsoleSpec,
    ) -> Self {
        let face = widths.ink(true).filter(|_| style.bold);
        Self {
            style,
            fill,
            edge,
            ink: face.or_else(|| widths.ink(false)).cloned(),
            struck: style.bold && face.is_none(),
            cell_w: i32::from(spec.cell_w.max(1)),
            cell_h: i32::from(spec.cell_h.max(1)),
        }
    }

    /// The style it draws.
    pub fn style(&self) -> &KeyCapStyle {
        &self.style
    }

    /// The box's fill and edge colours; `None` for one not drawn.
    pub fn colors(&self) -> (Option<Rgb>, Option<Rgb>) {
        (self.fill, self.edge)
    }

    /// How many cells a cap takes beyond its name's: the same for every
    /// name, the room a glyph inked across its whole cell (and a column
    /// more, bold struck twice) needs for its margin and edge on both sides.
    pub fn extra_cells(&self) -> u16 {
        let widest = self.cell_w + i32::from(self.struck) + 2 * self.pad();
        u16::try_from((widest + self.cell_w - 1) / self.cell_w - 1).unwrap_or(0)
    }

    /// `name` as a cap draws it: one letter as its capital with
    /// [`capitals`](KeyCapStyle::capitals), else as it is.
    pub fn shown<'a>(&self, name: &'a str) -> Cow<'a, str> {
        let mut chars = name.chars();
        if let (true, Some(c), None) = (self.style.capitals, chars.next(), chars.next())
            && c.is_alphabetic()
        {
            let mut upper = c.to_uppercase();
            if let (Some(u), None) = (upper.next(), upper.next()) {
                return Cow::Owned(u.to_string());
            }
        }
        Cow::Borrowed(name)
    }

    /// How many cells the arrow cluster's picture takes
    /// ([`KeyCapStyle::arrows`]): its width scaled to one line's height,
    /// in whole cells. `None` without a picture, or with an empty one.
    pub fn arrows_cells(&self) -> Option<u16> {
        let (_, w) = self.arrows_size()?;
        u16::try_from((w + self.cell_w - 1) / self.cell_w).ok()
    }

    /// The arrow cluster's picture as a sprite, its cells' first one at
    /// `(x, y)`: one line tall, centred across its cells, over the glyphs.
    /// `None` without a picture.
    pub(crate) fn arrows_sprite(&self, x: i32, y: i32) -> Option<Sprite> {
        let (image, src) = self.style.arrows?;
        let (h, w) = self.arrows_size()?;
        let cells = i32::from(self.arrows_cells()?);
        let left = x * self.cell_w + (cells * self.cell_w - w) / 2;
        let dest = Rect::new(left, y * self.cell_h, w, h);
        Some(Sprite::new(image, src, dest, Layer::Over))
    }

    /// The arrow cluster's picture's height and width as drawn: one line
    /// tall, its width scaled as much and rounded to the nearest pixel.
    fn arrows_size(&self) -> Option<(i32, i32)> {
        let (_, src) = self.style.arrows?;
        if src.is_empty() {
            return None;
        }
        let h = self.cell_h;
        let w = ((2 * src.w * h + src.h) / (2 * src.h)).max(1);
        Some((h, w))
    }

    /// `widths` as a frame drawing with `caps` measures text: with a key
    /// cap's extra cells, and the arrow cluster's picture's cells.
    pub(crate) fn frame_widths(widths: CellWidths, caps: Option<&KeyCaps>) -> CellWidths {
        widths
            .with_key_cap_cells(caps.map(KeyCaps::extra_cells))
            .with_arrows_cells(caps.and_then(KeyCaps::arrows_cells))
    }

    /// The margin and the edge: how far the box is outside the ink.
    fn pad(&self) -> i32 {
        i32::from(self.style.margin_px) + 1
    }

    /// The leftmost and rightmost inked pixel of glyph `c`, `cells` cells
    /// wide, from its first cell's left edge, as drawn.
    fn columns(&self, c: char, cells: i32) -> (i32, i32) {
        let px = |v: u32| i32::try_from(v).unwrap_or(0);
        let (left, right) = match (&self.ink, cells) {
            (Some(ink), 1) => {
                let (l, r) = ink.columns_of(c);
                (px(l), px(r))
            }
            // A wide glyph: the capitals' margins, from its two cells.
            (Some(ink), _) => (
                px(ink.capitals.left),
                cells * self.cell_w - self.cell_w + px(ink.capitals.right),
            ),
            (None, _) => (0, cells * self.cell_w - 1),
        };
        (left, right + i32::from(self.struck))
    }

    /// Where the cap of `name` goes, its glyphs measured with `widths`.
    /// Its box is `margin + 1` px outside the ink across (the edge is
    /// 1 px), from the first glyph's leftmost ink to the last one's
    /// rightmost, and centred in the name's cells and the extra ones;
    /// down, it is as far outside the capitals' ink if the line has room,
    /// and centred in the line.
    pub(crate) fn layout(&self, name: &str, widths: &CellWidths) -> CapLayout {
        let (cw, ch) = (self.cell_w, self.cell_h);
        let pad = self.pad();
        let mut at = 0;
        let mut ink: Option<(i32, i32)> = None;
        for c in name.chars() {
            let cells = i32::try_from(widths.of_char(c)).unwrap_or(1);
            let (l, r) = self.columns(c, cells);
            // The first glyph starts the name: its left ink is the cap's.
            ink = Some((ink.map_or(l, |(first, _)| first), at * cw + r));
            at += cells;
        }
        let (left, right) = ink.unwrap_or((0, cw - 1));
        let extra = self.extra_cells();
        let box_w = right - left + 1 + 2 * pad;
        let box_x = ((at.max(1) + i32::from(extra)) * cw - box_w) / 2;
        let px = |v: u32| i32::try_from(v).unwrap_or(0);
        let (top, bottom) = self.ink.as_ref().map_or((0, ch - 1), |ink| {
            (px(ink.capitals.top), px(ink.capitals.bottom))
        });
        let ink_h = bottom - top + 1;
        let box_h = (ink_h + 2 * pad).min(ch);
        let box_y = (ch - box_h) / 2;
        // Rounded down, also when the ink is taller than the box.
        let gap_top = (box_h - 2 - ink_h).div_euclid(2);
        let nudge = |v: i32| i8::try_from(v.clamp(i8::MIN.into(), i8::MAX.into())).unwrap_or(0);
        CapLayout {
            extra,
            frame: Rect::new(box_x, box_y, box_w, box_h),
            style: GlyphStyle::new(
                nudge(box_x + pad - left),
                nudge(box_y + 1 + gap_top - top),
                self.style.bold,
            ),
        }
    }
}

#[cfg(test)]
mod tests;
