//! Drawing helpers on [`GlyphBuffer`] that skins repeat: a [`panel`] with
//! a title set into its top border, a [`meter`] bar that shows how full
//! something is, a line [`print_centred`] in a rectangle and a line
//! [`print_cut`] to a width (with [`cut`], the text it prints).
//!
//! Each counts one cell per `char`, as [`GlyphBuffer::print`] does.
//!
//! [`panel`]: GlyphBuffer::panel
//! [`meter`]: GlyphBuffer::meter
//! [`print_centred`]: GlyphBuffer::print_centred
//! [`print_cut`]: GlyphBuffer::print_cut

use crate::prelude::*;

use crate::frame::color::Rgb;
use crate::frame::glyph_buffer::{BoxStyle, Cell, GlyphBuffer, Rect};

/// Cells of a panel's top border left of its title: the corner and one
/// line.
const TITLE_DX: i32 = 2;

/// Cells of a panel's top border that a title never covers: a corner and
/// a line on each side ([`TITLE_DX`] each), and a space each side of the
/// title.
const TITLE_FRAME: i32 = 6;

/// `text` in at most `w` cells: cut, its last cell a `…`, if it is longer.
///
/// ```
/// use shuyi_ui::frame::draw::cut;
/// assert_eq!(cut("Backspace", 5), "Back…");
/// assert_eq!(cut("Enter", 5), "Enter");
/// ```
pub fn cut(text: &str, w: usize) -> String {
    if text.chars().count() <= w {
        return text.to_owned();
    }
    let kept: String = text.chars().take(w.saturating_sub(1)).collect();
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

/// How a [`GlyphBuffer::panel`] looks: its border's style and colour, its
/// title's colour, and the text colour and background it is cleared to.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
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
        }
    }

    /// This look with its border in `border`.
    #[must_use]
    pub const fn border(self, border: Rgb) -> Self {
        Self { border, ..self }
    }

    /// This look with its title in `title`.
    #[must_use]
    pub const fn title(self, title: Rgb) -> Self {
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
    pub fn panel(&mut self, rect: Rect, look: &PanelLook, title: Option<&str>) {
        self.fill_rect(rect, Cell::new(' ', look.text, look.bg));
        self.draw_box(rect, look.style, look.border, look.bg);
        let Some(title) = title else {
            return;
        };
        let room = usize::try_from(rect.w.saturating_sub(TITLE_FRAME)).unwrap_or(0);
        if room > 0 {
            let shown = format!(" {} ", cut(title, room));
            let x = rect.x.saturating_add(TITLE_DX);
            self.print(x, rect.y, &shown, look.title, look.bg);
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
        let shown = cut(text, usize::try_from(rect.w).unwrap_or(0));
        let len = i32::try_from(shown.chars().count()).unwrap_or(rect.w);
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
        self.print_fg(x, y, &cut(text, width), fg)
    }
}

#[cfg(test)]
mod tests;
