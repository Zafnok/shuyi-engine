//! The glyph look of a [`Menu`](super::Menu): paints a [`MenuView`] as a
//! single-line box of text rows with the focused one as a bar and, when
//! its items have notes, the focused item's note under a rule. Everything
//! about the look lives here; nothing here decides what the menu shows.
//! Another look is another `paint` of the same view (game ADR-0054).

use super::view::{MenuItemView, MenuView, SuffixKind};
use crate::frame::color::{EngineColor, Palette};
use crate::frame::glyph_buffer::{BoxStyle, Cell, GlyphBuffer, Rect};

/// Cells the label and suffix of `item` take.
fn width(item: &MenuItemView) -> usize {
    let suffix = item
        .suffix
        .as_ref()
        .map_or(0, |(t, _)| t.chars().count() + 1);
    item.label.chars().count() + suffix
}

/// Cells the note of `item` takes.
fn note_width(item: &MenuItemView) -> usize {
    item.note.as_ref().map_or(0, |n| n.chars().count())
}

/// Whether the menu has a note row: some item has a note.
fn has_notes(view: &MenuView) -> bool {
    view.items.iter().any(|i| i.note.is_some())
}

/// Box size in cells: the widest label (or note, so the box keeps its
/// width as the focus moves) plus a space and a border on each side, by
/// one row per item plus the border, and a rule and a note row if any
/// item has a note.
pub fn size(view: &MenuView) -> (i32, i32) {
    let widest = view
        .items
        .iter()
        .map(|i| width(i).max(note_width(i)))
        .max()
        .unwrap_or(0);
    let w = i32::try_from(widest).unwrap_or(i32::MAX).saturating_add(4);
    let notes = if has_notes(view) { 2 } else { 0 };
    let h = i32::try_from(view.items.len())
        .unwrap_or(i32::MAX)
        .saturating_add(2 + notes);
    (w, h)
}

/// The colour of the text after a label: the warning colour for a
/// warning, dim for an aside.
pub const fn suffix_color(kind: SuffixKind) -> EngineColor {
    match kind {
        SuffixKind::Warning => EngineColor::Warning,
        SuffixKind::Aside => EngineColor::TextDim,
    }
}

/// Paints `view` with its top-left corner at `(x, y)`: a
/// [`Panel`](EngineColor::Panel) box with a [`Border`](EngineColor::Border)
/// single line, items in [`Text`](EngineColor::Text), disabled ones in
/// [`TextDim`](EngineColor::TextDim), and the focused one as a bar of panel
/// text on [`Focus`](EngineColor::Focus) (on the border's color for a
/// disabled one: a menu that browses disabled items). Under the items, if
/// any has a note: a border rule and the focused item's note in text.
pub fn paint(palette: &Palette, view: &MenuView, buf: &mut GlyphBuffer, x: i32, y: i32) {
    let color = |e| palette.engine(e);
    let bg = color(EngineColor::Panel);
    let (w, h) = size(view);
    let rect = Rect::new(x, y, w, h);
    buf.fill_rect(rect, Cell::new(' ', color(EngineColor::Text), bg));
    buf.draw_box(rect, BoxStyle::Single, color(EngineColor::Border), bg);
    for (row, item) in (y + 1..).zip(&view.items) {
        let (fg, row_bg) = match (item.focused, item.enabled) {
            (true, false) => (bg, color(EngineColor::Border)),
            (false, false) => (color(EngineColor::TextDim), bg),
            (true, true) => (bg, color(EngineColor::Focus)),
            (false, true) => (color(EngineColor::Text), bg),
        };
        buf.fill_rect(Rect::new(x + 1, row, w - 2, 1), Cell::new(' ', fg, row_bg));
        buf.print(x + 2, row, &item.label, fg, row_bg);
        if let Some((text, kind)) = &item.suffix {
            let at = x + 3 + i32::try_from(item.label.chars().count()).unwrap_or(0);
            // On the focus bar the colour would vanish: keep the bar's.
            let fg = if item.focused {
                fg
            } else {
                color(suffix_color(*kind))
            };
            buf.print(at, row, text, fg, row_bg);
        }
    }
    if has_notes(view) {
        let rule = y + h - 3;
        let border = color(EngineColor::Border);
        buf.fill_rect(Rect::new(x + 1, rule, w - 2, 1), Cell::new('─', border, bg));
        buf.set(x, rule, Cell::new('├', border, bg));
        buf.set(x + w - 1, rule, Cell::new('┤', border, bg));
        if let Some(note) = view.focused().and_then(|i| i.note.as_ref()) {
            buf.print(x + 2, rule + 1, note, color(EngineColor::Text), bg);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::color::tests::engine_palette;

    fn item(label: &str, enabled: bool, focused: bool) -> MenuItemView {
        MenuItemView {
            label: label.to_owned(),
            suffix: None,
            note: None,
            enabled,
            focused,
        }
    }

    fn painted(view: &MenuView) -> (GlyphBuffer, Palette) {
        let p = engine_palette();
        let (w, h) = size(view);
        let blank = Cell::new(
            ' ',
            p.engine(EngineColor::Text),
            p.engine(EngineColor::Background),
        );
        let mut buf = GlyphBuffer::new(
            u16::try_from(w + 2).unwrap(),
            u16::try_from(h + 2).unwrap(),
            blank,
        );
        paint(&p, view, &mut buf, 1, 1);
        (buf, p)
    }

    fn row(buf: &GlyphBuffer, y: i32) -> String {
        (0..i32::from(buf.width()))
            .map(|x| buf.get(x, y).map_or(' ', |c| c.glyph))
            .collect()
    }

    #[test]
    fn the_box_is_the_widest_item_plus_a_space_and_border_each_side() {
        assert_eq!(size(&MenuView { items: vec![] }), (4, 2));
        let view = MenuView {
            items: vec![item("a", true, true), item("bcd", false, false)],
        };
        assert_eq!(size(&view), (7, 4));
        // A suffix counts, with the space before it.
        let mut with = item("ab", true, true);
        with.suffix = Some(("(x)".to_owned(), SuffixKind::Warning));
        assert_eq!(size(&MenuView { items: vec![with] }), (10, 3));
    }

    #[test]
    fn items_are_painted_one_to_a_row_inside_the_border() {
        let view = MenuView {
            items: vec![item("Move", true, false), item("Wait", true, true)],
        };
        let (buf, _) = painted(&view);
        assert_eq!(row(&buf, 1).trim(), "┌──────┐");
        assert_eq!(row(&buf, 2).trim(), "│ Move │");
        assert_eq!(row(&buf, 3).trim(), "│ Wait │");
        assert_eq!(row(&buf, 4).trim(), "└──────┘");
    }

    #[test]
    fn the_focused_item_is_a_bar_and_a_disabled_one_is_dim() {
        let view = MenuView {
            items: vec![
                item("Move", true, false),
                item("Attack", false, false),
                item("Wait", true, true),
            ],
        };
        let (buf, p) = painted(&view);
        let bar = p.engine(EngineColor::Focus);
        let at = |x, y| *buf.get(x, y).unwrap();
        // The bar fills the row inside the border, in the panel's colour.
        assert_eq!(at(2, 4).bg, bar);
        assert_eq!(at(2, 4).fg, p.engine(EngineColor::Panel));
        assert_eq!(at(8, 4).bg, bar);
        assert_ne!(at(2, 2).bg, bar);
        assert_eq!(at(3, 2).fg, p.engine(EngineColor::Text));
        assert_eq!(at(3, 3).fg, p.engine(EngineColor::TextDim));
        assert_ne!(at(3, 3).bg, bar);
    }

    #[test]
    fn the_focused_items_note_shows_under_a_rule_at_the_widest_notes_width() {
        let mut brace = item("Brace", true, true);
        brace.note = Some("self: Def +5".to_owned());
        let mut push = item("Nudge", false, false);
        push.note = Some("push 1 tile, 5 on impact".to_owned());
        let view = MenuView {
            items: vec![brace, push],
        };
        // As wide as the longest note, whichever item has the focus.
        assert_eq!(size(&view), (28, 6));
        let (buf, p) = painted(&view);
        assert_eq!(row(&buf, 4).trim(), "├──────────────────────────┤");
        assert_eq!(row(&buf, 5).trim(), "│ self: Def +5             │");
        assert_eq!(row(&buf, 6).trim(), "└──────────────────────────┘");
        assert_eq!(buf.get(3, 5).unwrap().fg, p.engine(EngineColor::Text));
    }

    #[test]
    fn a_focused_disabled_item_is_a_muted_bar_with_its_note() {
        let mut brace = item("Brace", true, false);
        brace.note = Some("self".to_owned());
        let mut push = item("Nudge", false, true);
        push.note = Some("push".to_owned());
        push.suffix = Some(("no uses left".to_owned(), SuffixKind::Warning));
        let view = MenuView {
            items: vec![brace, push],
        };
        let (buf, p) = painted(&view);
        let at = |x, y| *buf.get(x, y).unwrap();
        assert_eq!(at(2, 3).bg, p.engine(EngineColor::Border));
        assert_eq!(at(3, 3).fg, p.engine(EngineColor::Panel));
        // The reason takes the bar's colour too.
        assert_eq!(at(9, 3).fg, p.engine(EngineColor::Panel));
        assert_eq!(row(&buf, 5).trim(), "│ push               │");
    }

    #[test]
    fn a_menu_without_notes_has_no_rule() {
        let view = MenuView {
            items: vec![item("Wait", true, true)],
        };
        assert_eq!(size(&view), (8, 3));
    }

    #[test]
    fn a_suffix_keeps_its_colour_except_on_the_focus_bar() {
        let mut broken = item("Sword", false, false);
        broken.suffix = Some(("(broken)".to_owned(), SuffixKind::Warning));
        let mut focused = item("Axe", true, true);
        focused.suffix = Some(("(worn)".to_owned(), SuffixKind::Warning));
        let view = MenuView {
            items: vec![broken, focused],
        };
        let (buf, p) = painted(&view);
        let at = |x, y| *buf.get(x, y).unwrap();
        // After the label and a space: column 3 + 5 on the first row.
        assert_eq!(row(&buf, 2).trim(), "│ Sword (broken) │");
        assert_eq!(at(9, 2).glyph, '(');
        assert_eq!(at(9, 2).fg, p.engine(EngineColor::Warning));
        assert_eq!(at(7, 3).glyph, '(');
        assert_eq!(at(7, 3).fg, p.engine(EngineColor::Panel));
    }
}
