//! Drawing helpers on [`GlyphBuffer`] that skins repeat: a [`panel`] with
//! a title set into its top border, a [`meter`] bar that shows how full
//! something is, a line [`print_centred`] in a rectangle and a line
//! [`print_cut`] to a width (with [`cut`], the text it prints). A panel
//! may wear a picture frame ([`PanelFrame`]) and a plate behind its title
//! ([`TitlePlate`]).
//!
//! Each counts cells as [`GlyphBuffer::print`] does: one a `char`, two a
//! wide one ([`GlyphBuffer::widths`]).
//!
//! [`panel`]: GlyphBuffer::panel
//! [`meter`]: GlyphBuffer::meter
//! [`print_centred`]: GlyphBuffer::print_centred
//! [`print_cut`]: GlyphBuffer::print_cut

use crate::prelude::*;

use shuyi_assets::font::CellWidths;
use shuyi_assets::lang::text_width;

use crate::frame::color::Rgb;
use crate::frame::glyph_buffer::{
    BoxStyle, Cell, GlyphBuffer, Layer, Rect, Sprite, cells_to_px, subtract,
};
use crate::frame::slice::{CentreFill, NineSlice, nine_slice_sprites};

/// Cells of a panel's top border left of its title: the corner and one
/// line.
const TITLE_DX: i32 = 2;

/// Cells of a panel's top border that a title never covers: a corner and
/// a line on each side ([`TITLE_DX`] each), and a space each side of the
/// title.
const TITLE_FRAME: i32 = 6;

/// `text` in at most `w` cells, measured with `widths`: cut, its last cell
/// a `…`, if it is longer. A wide character that would reach that cell
/// goes too.
///
/// ```
/// use shuyi_assets::font::CellWidths;
/// use shuyi_ui::frame::draw::cut;
/// let narrow = &CellWidths::NARROW;
/// assert_eq!(cut("Backspace", 5, narrow), "Back…");
/// assert_eq!(cut("Enter", 5, narrow), "Enter");
/// assert_eq!(cut("あいう", 5, &CellWidths::wide(['あ', 'い', 'う'])), "あい…");
/// ```
pub fn cut(text: &str, w: usize, widths: &CellWidths) -> String {
    if text_width(text, widths) <= w {
        return text.to_owned();
    }
    let room = w.saturating_sub(1);
    let mut used = 0;
    let kept: String = text
        .chars()
        .take_while(|&c| {
            used += widths.of_char(c);
            used <= room
        })
        .collect();
    if w == 0 { kept } else { kept + "…" }
}

/// How many of a meter's `cells` are full when it shows `filled` of `of`.
///
/// The rule every meter shares: `cells × filled / of` rounded half up, but
/// a meter that is not empty (`filled > 0`) shows at least one full cell
/// and one that is not full (`filled < of`) at least one empty cell, so
/// "almost dead" and "almost full" never look like "dead" and "full". With
/// one cell, the rounding alone decides. `filled` past `of` counts as
/// `of`; an `of` of 0 is an empty meter.
///
/// ```
/// use shuyi_ui::frame::draw::meter_fill;
/// assert_eq!(meter_fill(10, 1, 100), 1); // 0.1 would round to 0
/// assert_eq!(meter_fill(10, 99, 100), 9); // 9.9 would round to 10
/// assert_eq!(meter_fill(10, 25, 100), 3); // 2.5 rounds up
/// ```
pub fn meter_fill(cells: u32, filled: u32, of: u32) -> u32 {
    if of == 0 {
        return 0;
    }
    let (cells, filled, of) = (
        u128::from(cells),
        u128::from(filled.min(of)),
        u128::from(of),
    );
    let rounded = (2 * cells * filled + of) / (2 * of);
    let shown = if cells >= 2 && filled > 0 && filled < of {
        rounded.clamp(1, cells - 1)
    } else {
        rounded
    };
    // At most `cells`, which came from a u32.
    u32::try_from(shown).unwrap_or(u32::MAX)
}

/// A panel's frame drawn from a picture: a [`NineSlice`]
/// laid over the panel's border cells, reaching `outset` console pixels
/// past the panel on every side. A picture whose corners are wider than a
/// cell keeps them off the text that way: the frame moves out, the text
/// stays on its cells.
///
/// Kind 2 of ADR-0039: build it with [`PanelFrame::new`] and its `with_`
/// methods.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
#[non_exhaustive]
pub struct PanelFrame {
    /// The picture, in nine slices. Its middle is never drawn: the inside
    /// stays the panel's background cells.
    pub slice: NineSlice,
    /// How far the frame reaches past the panel on each side, in console
    /// pixels.
    pub outset: u16,
}

impl PanelFrame {
    /// A frame of `slice` over the panel's own edge.
    pub const fn new(slice: NineSlice) -> Self {
        Self { slice, outset: 0 }
    }

    /// This frame reaching `outset` console pixels past the panel.
    #[must_use]
    pub const fn with_outset(self, outset: u16) -> Self {
        Self { outset, ..self }
    }
}

/// A plate behind a panel's title: a picture with two end
/// caps (its [`NineSlice`]'s left and right borders) and a body stretched
/// to the title's width, `pad` console pixels wider on each side. The
/// title's row sits `text_top` picture pixels down from the plate's top,
/// so a plate taller than a cell reaches above and below it.
///
/// Kind 2 of ADR-0039: build it with [`TitlePlate::new`] and its `with_`
/// methods.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
#[non_exhaustive]
pub struct TitlePlate {
    /// The picture: its left and right borders are the caps; its top and
    /// bottom are usually 0.
    pub slice: NineSlice,
    /// Console pixels of body on each side of the title.
    pub pad: u16,
    /// Picture pixels from the plate's top to the top of the title's row.
    pub text_top: u16,
}

impl TitlePlate {
    /// A plate of `slice`, the title on its top row, with no padding.
    pub const fn new(slice: NineSlice) -> Self {
        Self {
            slice,
            pad: 0,
            text_top: 0,
        }
    }

    /// This plate with `pad` console pixels of body each side of the title.
    #[must_use]
    pub const fn with_pad(self, pad: u16) -> Self {
        Self { pad, ..self }
    }

    /// This plate with the title's row `text_top` picture pixels down.
    #[must_use]
    pub const fn with_text_top(self, text_top: u16) -> Self {
        Self { text_top, ..self }
    }
}

/// How a [`GlyphBuffer::panel`] looks: its border's style and colour, its
/// title's colour, and the text colour and background it is cleared to;
/// and, if set, a picture frame in place of the lines and a plate behind
/// the title.
///
/// Kind 2 of ADR-0039: build it with [`PanelLook::new`] and its `with_` methods.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[non_exhaustive]
pub struct PanelLook {
    /// The border's lines.
    pub style: BoxStyle,
    /// The border's colour.
    pub border: Rgb,
    /// The title's colour.
    pub title: Rgb,
    /// The colour the inside is cleared to, for text printed keeping it.
    pub text: Rgb,
    /// The background, inside and under the border and the title.
    pub bg: Rgb,
    /// A picture frame drawn in place of the lines, keeping its own
    /// colours (`border` is not applied to it).
    pub frame: Option<PanelFrame>,
    /// A plate drawn behind the title.
    pub plate: Option<TitlePlate>,
}

impl PanelLook {
    /// A panel in `style` with everything in `fg` on `bg`.
    pub const fn new(style: BoxStyle, fg: Rgb, bg: Rgb) -> Self {
        Self {
            style,
            border: fg,
            title: fg,
            text: fg,
            bg,
            frame: None,
            plate: None,
        }
    }

    /// This look with its border drawn from `frame`, or in lines if `None`.
    #[must_use]
    pub const fn with_frame(self, frame: Option<PanelFrame>) -> Self {
        Self { frame, ..self }
    }

    /// This look with `plate` behind its title, or none if `None`.
    #[must_use]
    pub const fn with_plate(self, plate: Option<TitlePlate>) -> Self {
        Self { plate, ..self }
    }

    /// This look with its border in `border`.
    #[must_use]
    pub const fn with_border(self, border: Rgb) -> Self {
        Self { border, ..self }
    }

    /// This look with its title in `title`.
    #[must_use]
    pub const fn with_title(self, title: Rgb) -> Self {
        Self { title, ..self }
    }
}

impl GlyphBuffer {
    /// A panel: clears `rect` to `look.text` on `look.bg`, frames it in
    /// `look.style` and `look.border`, and sets `title`, if any, into the
    /// top border in `look.title`, two cells from the left with a space
    /// each side of it. A title that doesn't fit is [`cut`]; the border
    /// keeps a line and its corner on each side.
    ///
    /// ```
    /// use shuyi_ui::frame::draw::PanelLook;
    /// use shuyi_ui::frame::{BoxStyle, Cell, GlyphBuffer, Rgb};
    /// let (white, black) = (Rgb::new(255, 255, 255), Rgb::new(0, 0, 0));
    /// let look = PanelLook::new(BoxStyle::Single, white, black);
    /// let mut buf = GlyphBuffer::new(12, 3, Cell::new(' ', white, black));
    /// buf.panel(buf.bounds(), &look, Some("Shop"));
    /// let top: String = (0..12).map(|x| buf.get(x, 0).unwrap().glyph).collect();
    /// assert_eq!(top, "┌─ Shop ───┐");
    /// ```
    ///
    /// With a [`PanelFrame`] the border is the picture
    /// ([`panel_border`](Self::panel_border)), and with a [`TitlePlate`]
    /// the plate is drawn behind the title's text: both are sprites under
    /// the glyphs, and the cells stay as they are without them.
    pub fn panel(&mut self, rect: Rect, look: &PanelLook, title: Option<&str>) {
        self.fill_rect(rect, Cell::new(' ', look.text, look.bg));
        self.panel_border(rect, look);
        let Some(title) = title else {
            return;
        };
        let room = usize::try_from(rect.w.saturating_sub(TITLE_FRAME)).unwrap_or(0);
        if room > 0 {
            let text = cut(title, room, self.widths());
            let shown = format!(" {text} ");
            let x = rect.x.saturating_add(TITLE_DX);
            self.print(x, rect.y, &shown, look.title, look.bg);
            if let Some(plate) = look.plate {
                let cells = self.text_cells(&text);
                self.title_plate(&plate, Rect::new(x.saturating_add(1), rect.y, cells, 1));
            }
        }
    }

    /// The border of a panel on `rect`: [`draw_box`](Self::draw_box) in
    /// `look.style` and `look.border`, or, with `look.frame`, the frame's
    /// picture.
    ///
    /// A picture frame sets the border cells to blanks in `look.text` on
    /// `look.bg`, then lays the frame's corners and edges (never its
    /// middle) over the panel's pixels widened by its
    /// [`outset`](PanelFrame::outset), but never past the buffer's edge (a
    /// panel at the edge keeps its corners whole), in [`Layer::Under`]. No part of it
    /// lies over the inside cells (`rect` less one cell on each side), so
    /// a highlight drawn there as cell backgrounds still shows. Like
    /// `draw_box`, it leaves the inside cells as they are.
    pub fn panel_border(&mut self, rect: Rect, look: &PanelLook) {
        let Some(frame) = look.frame else {
            self.draw_box(rect, look.style, look.border, look.bg);
            return;
        };
        if rect.is_empty() {
            return;
        }
        let blank = Cell::new(' ', look.text, look.bg);
        let (right, bottom) = (rect.x + rect.w - 1, rect.y + rect.h - 1);
        self.fill_rect(Rect::new(rect.x, rect.y, rect.w, 1), blank);
        self.fill_rect(Rect::new(rect.x, bottom, rect.w, 1), blank);
        self.fill_rect(Rect::new(rect.x, rect.y, 1, rect.h), blank);
        self.fill_rect(Rect::new(right, rect.y, 1, rect.h), blank);
        let spec = self.spec();
        let px = |r: Rect| cells_to_px(spec, r.x.into(), r.y.into(), r.w.into(), r.h.into());
        let (Some(outer), inside) = (
            px(rect),
            px(Rect::new(rect.x + 1, rect.y + 1, rect.w - 2, rect.h - 2)),
        ) else {
            return;
        };
        let out = i32::from(frame.outset);
        let widened = Rect::new(
            outer.x.saturating_sub(out),
            outer.y.saturating_sub(out),
            outer.w.saturating_add(2 * out),
            outer.h.saturating_add(2 * out),
        );
        // Never past the buffer: a panel at its edge keeps its corners.
        let on_screen = outer.intersect(&self.pixel_bounds()).is_some();
        let dest = match widened.intersect(&self.pixel_bounds()) {
            Some(cut) if on_screen => cut,
            _ => widened,
        };
        let slice = frame.slice.with_centre(CentreFill::Empty);
        for sprite in nine_slice_sprites(&slice, dest, self.pixel_bounds(), Layer::Under) {
            let parts = match inside.filter(|r| !r.is_empty()) {
                Some(hole) => subtract(sprite.clip, hole),
                None => vec![sprite.clip],
            };
            for clip in parts {
                self.add_sprite(Sprite { clip, ..sprite });
            }
        }
    }

    /// A [`TitlePlate`] behind the text on the cells `text` (its first
    /// row): its body `text`'s width plus the plate's `pad` on each side,
    /// its caps beyond, its top `text_top` picture pixels above the text's
    /// row, in [`Layer::Under`]. [`panel`](Self::panel) draws one behind
    /// its title; a skin that prints a title of its own (centred, say)
    /// puts one behind it with this.
    pub fn title_plate(&mut self, plate: &TitlePlate, text: Rect) {
        let spec = self.spec();
        let Some(at) = cells_to_px(spec, text.x.into(), text.y.into(), text.w.into(), 1) else {
            return;
        };
        let slice = &plate.slice;
        let zoom = i64::from(slice.scale());
        let caps = slice
            .borders
            .fitted(slice.picture.rect.w, slice.picture.rect.h);
        let pad = i64::from(plate.pad);
        let left = i64::from(at.x) - pad - i64::from(caps.left) * zoom;
        let wide = i64::from(at.w) + 2 * pad + i64::from(caps.left + caps.right) * zoom;
        let top = i64::from(at.y) - i64::from(plate.text_top) * zoom;
        let tall = i64::from(slice.picture.rect.h) * zoom;
        let fit = |v: i64| i32::try_from(v).ok();
        let dest = || Some(Rect::new(fit(left)?, fit(top)?, fit(wide)?, fit(tall)?));
        if let Some(dest) = dest() {
            self.nine_slice(slice, dest, Layer::Under);
        }
    }

    /// A meter: each row of `rect` is `rect.w` cells, the first
    /// [`meter_fill`] of them `full` and the rest `empty`, showing `filled`
    /// of `of`.
    ///
    /// ```
    /// use shuyi_ui::frame::{Cell, GlyphBuffer, Rect, Rgb};
    /// let (white, black) = (Rgb::new(255, 255, 255), Rgb::new(0, 0, 0));
    /// let mut buf = GlyphBuffer::new(10, 1, Cell::new(' ', white, black));
    /// let (full, empty) = (Cell::new('█', white, black), Cell::new('░', white, black));
    /// buf.meter(buf.bounds(), 12, 20, full, empty);
    /// let row: String = (0..10).map(|x| buf.get(x, 0).unwrap().glyph).collect();
    /// assert_eq!(row, "██████░░░░");
    /// ```
    pub fn meter(&mut self, rect: Rect, filled: u32, of: u32, full: Cell, empty: Cell) {
        if rect.is_empty() {
            return;
        }
        let cells = u32::try_from(rect.w).unwrap_or(0);
        let shown = i32::try_from(meter_fill(cells, filled, of)).unwrap_or(rect.w);
        self.fill_rect(Rect::new(rect.x, rect.y, shown, rect.h), full);
        let unfilled = Rect::new(rect.x.saturating_add(shown), rect.y, rect.w - shown, rect.h);
        self.fill_rect(unfilled, empty);
    }

    /// Prints `text` in `fg` on the middle row of `rect` (the upper of two
    /// middles), centred across it (a cell left of the middle when the
    /// cells left over are odd) and [`cut`] to its width, keeping each
    /// cell's background. Returns how many cells were written.
    ///
    /// ```
    /// use shuyi_ui::frame::{Cell, GlyphBuffer, Rect, Rgb};
    /// let (white, black) = (Rgb::new(255, 255, 255), Rgb::new(0, 0, 0));
    /// let mut buf = GlyphBuffer::new(9, 1, Cell::new('.', white, black));
    /// buf.print_centred(buf.bounds(), "Paused", white);
    /// let row: String = (0..9).map(|x| buf.get(x, 0).unwrap().glyph).collect();
    /// assert_eq!(row, ".Paused..");
    /// ```
    pub fn print_centred(&mut self, rect: Rect, text: &str, fg: Rgb) -> u16 {
        if rect.is_empty() {
            return 0;
        }
        let shown = cut(text, usize::try_from(rect.w).unwrap_or(0), self.widths());
        let len = self.text_cells(&shown).min(rect.w);
        let x = rect.x.saturating_add((rect.w - len) / 2);
        let y = rect.y.saturating_add((rect.h - 1) / 2);
        self.print_fg(x, y, &shown, fg)
    }

    /// Prints `text` in `fg` from `(x, y)` in at most `width` cells,
    /// [`cut`] if it is longer, keeping each cell's background: a
    /// [`print_fg`](Self::print_fg) of [`cut`]. Returns how many cells
    /// were written.
    ///
    /// ```
    /// use shuyi_ui::frame::{Cell, GlyphBuffer, Rgb};
    /// let (white, black) = (Rgb::new(255, 255, 255), Rgb::new(0, 0, 0));
    /// let mut buf = GlyphBuffer::new(8, 1, Cell::new('.', white, black));
    /// buf.print_cut(0, 0, 5, "Backspace", white);
    /// let row: String = (0..8).map(|x| buf.get(x, 0).unwrap().glyph).collect();
    /// assert_eq!(row, "Back…...");
    /// ```
    pub fn print_cut(&mut self, x: i32, y: i32, width: usize, text: &str, fg: Rgb) -> u16 {
        let shown = cut(text, width, self.widths());
        self.print_fg(x, y, &shown, fg)
    }
}

#[cfg(test)]
mod tests;
