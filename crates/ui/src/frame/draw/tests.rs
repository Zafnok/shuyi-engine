//! The drawing helpers' looks, read from cells.

use super::*;

const WHITE: Rgb = Rgb::new(255, 255, 255);
const RED: Rgb = Rgb::new(255, 0, 0);
const BLUE: Rgb = Rgb::new(0, 0, 255);
const BLANK: Cell = Cell::new('.', WHITE, BLUE);
const FULL: Cell = Cell::new('█', RED, BLUE);
const EMPTY: Cell = Cell::new('░', WHITE, BLUE);

/// The glyphs of row `y` of `buf`.
fn row(buf: &GlyphBuffer, y: i32) -> String {
    (0..i32::from(buf.width()))
        .map(|x| buf.get(x, y).unwrap().glyph)
        .collect()
}

/// A 10-cell meter showing `filled` of `of`.
fn meter(filled: u32, of: u32) -> String {
    let mut buf = GlyphBuffer::new(10, 1, BLANK);
    buf.meter(buf.bounds(), filled, of, FULL, EMPTY);
    row(&buf, 0)
}

#[test]
fn a_meter_at_none_one_half_all_but_one_and_all() {
    assert_eq!(meter(0, 40), "░░░░░░░░░░");
    assert_eq!(meter(1, 40), "█░░░░░░░░░"); // 0.25 cells: at least one
    assert_eq!(meter(20, 40), "█████░░░░░");
    assert_eq!(meter(39, 40), "█████████░"); // 9.75 cells: at least one empty
    assert_eq!(meter(40, 40), "██████████");
}

#[test]
fn a_meter_rounds_half_up_and_clamps() {
    assert_eq!(meter_fill(10, 1, 4), 3); // 2.5
    assert_eq!(meter_fill(10, 1, 3), 3); // 3.33
    assert_eq!(meter_fill(10, 99, 40), 10); // past `of` is full
    assert_eq!(meter_fill(10, 5, 0), 0); // of nothing: empty
    assert_eq!(meter_fill(1, 1, 3), 0); // one cell: rounding alone
    assert_eq!(meter_fill(1, 2, 3), 1);
    assert_eq!(meter_fill(0, 1, 2), 0);
    assert_eq!(meter_fill(u32::MAX, u32::MAX - 1, u32::MAX), u32::MAX - 1);
}

#[test]
fn a_meter_keeps_its_cells_colours_and_fills_every_row() {
    let mut buf = GlyphBuffer::new(4, 3, BLANK);
    buf.meter(Rect::new(0, 1, 4, 2), 1, 2, FULL, EMPTY);
    assert_eq!(row(&buf, 0), "....");
    assert_eq!(row(&buf, 1), "██░░");
    assert_eq!(row(&buf, 2), "██░░");
    assert_eq!(buf.get(0, 1), Some(&FULL));
    assert_eq!(buf.get(3, 2), Some(&EMPTY));
    buf.meter(Rect::new(0, 0, 0, 1), 1, 2, FULL, EMPTY);
    assert_eq!(row(&buf, 0), "....");
}

#[test]
fn a_panel_clears_frames_and_sets_its_title_in_the_top_border() {
    let mut buf = GlyphBuffer::new(12, 4, BLANK);
    let look = PanelLook::new(BoxStyle::Double, WHITE, BLUE).title(RED);
    buf.panel(buf.bounds(), &look, Some("Shop"));
    assert_eq!(row(&buf, 0), "╔═ Shop ═══╗");
    assert_eq!(row(&buf, 1), "║          ║");
    assert_eq!(row(&buf, 3), "╚══════════╝");
    assert_eq!(buf.get(3, 0), Some(&Cell::new('S', RED, BLUE)));
    assert_eq!(buf.get(1, 0), Some(&Cell::new('═', WHITE, BLUE)));
    assert_eq!(buf.get(5, 2), Some(&Cell::new(' ', WHITE, BLUE)));
    let look = look.border(RED);
    buf.panel(buf.bounds(), &look, None);
    assert_eq!(buf.get(0, 0), Some(&Cell::new('╔', RED, BLUE)));
    assert_eq!(buf.get(5, 2), Some(&Cell::new(' ', WHITE, BLUE)));
}

#[test]
fn a_title_longer_than_the_panel_is_cut() {
    let mut buf = GlyphBuffer::new(12, 3, BLANK);
    let look = PanelLook::new(BoxStyle::Single, WHITE, BLUE);
    buf.panel(buf.bounds(), &look, Some("Pause menu"));
    assert_eq!(row(&buf, 0), "┌─ Pause… ─┐");
    // Too narrow for a cell of title: the border alone.
    let mut buf = GlyphBuffer::new(6, 3, BLANK);
    buf.panel(buf.bounds(), &look, Some("Shop"));
    assert_eq!(row(&buf, 0), "┌────┐");
    buf.panel(buf.bounds(), &look, None);
    assert_eq!(row(&buf, 0), "┌────┐");
}

#[test]
fn centring_an_odd_and_an_even_width() {
    let mut buf = GlyphBuffer::new(7, 1, BLANK);
    buf.print_centred(buf.bounds(), "abc", RED);
    assert_eq!(row(&buf, 0), "..abc..");
    let mut buf = GlyphBuffer::new(7, 1, BLANK);
    buf.print_centred(buf.bounds(), "ab", RED);
    assert_eq!(row(&buf, 0), "..ab...");
    let mut buf = GlyphBuffer::new(8, 1, BLANK);
    buf.print_centred(buf.bounds(), "abc", RED);
    assert_eq!(row(&buf, 0), "..abc...");
    let mut buf = GlyphBuffer::new(8, 1, BLANK);
    buf.print_centred(buf.bounds(), "ab", RED);
    assert_eq!(row(&buf, 0), "...ab...");
}

#[test]
fn centring_in_a_rectangle_cuts_and_keeps_the_background() {
    let mut buf = GlyphBuffer::new(8, 4, BLANK);
    let written = buf.print_centred(Rect::new(2, 0, 4, 4), "abcdef", RED);
    assert_eq!(written, 4);
    assert_eq!(row(&buf, 1), "..abc…..");
    assert_eq!(buf.get(2, 1), Some(&Cell::new('a', RED, BLUE)));
    assert_eq!(buf.print_centred(Rect::new(0, 0, 0, 1), "a", RED), 0);
}

#[test]
fn printing_cut_to_a_width() {
    let mut buf = GlyphBuffer::new(8, 1, BLANK);
    assert_eq!(buf.print_cut(1, 0, 4, "Escape", RED), 4);
    assert_eq!(row(&buf, 0), ".Esc…...");
    assert_eq!(buf.get(1, 0), Some(&Cell::new('E', RED, BLUE)));
    buf.print_cut(0, 0, 8, "Enter", RED);
    assert_eq!(row(&buf, 0), "Enter...");
}
