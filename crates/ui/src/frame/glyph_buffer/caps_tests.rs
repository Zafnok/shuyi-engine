//! Key caps in a buffer (ticket 3215): their cells, glyph styles and box.

use proptest::prelude::*;
use shuyi_assets::font::{FontAtlasDef, FontInk, InkBox};

use super::*;
use crate::frame::color::tests::engine_palette;
use crate::frame::key_cap::KeyCapStyle;

const FG: Rgb = Rgb::new(200, 200, 200);
const BG: Rgb = Rgb::new(0, 0, 0);
const KEY: Rgb = Rgb::new(250, 200, 60);
const KEY_BG: Rgb = Rgb::new(80, 0, 0);
const FILL: Rgb = Rgb::new(74, 64, 46);
const EDGE: Rgb = Rgb::new(128, 112, 82);

/// Terminus 8 × 16's widths: its capitals' ink at columns 1 to 6, rows
/// 2 to 11.
fn terminus() -> CellWidths {
    CellWidths::of(&FontAtlasDef {
        cell_w: 8,
        cell_h: 16,
        ink: Some(FontInk::new(InkBox::new(1, 2, 6, 11))),
        ..FontAtlasDef::default()
    })
}

/// A `w × 2` buffer drawing key caps boxed in `FILL` and `EDGE`.
fn capped(w: u16) -> GlyphBuffer {
    let buf = GlyphBuffer::new(w, 2, Cell::new(' ', FG, BG)).with_widths(terminus());
    let caps = KeyCaps::with_colors(
        KeyCapStyle::new(),
        (Some(FILL), Some(EDGE)),
        &terminus(),
        buf.spec(),
    );
    buf.with_key_caps(Some(caps))
}

/// A key in `KEY` on `KEY_BG`, the rest in `FG` on `BG`.
fn colors(role: Option<TextRole>) -> (Rgb, Option<Rgb>) {
    match role {
        Some(TextRole::Key) => (KEY, Some(KEY_BG)),
        _ => (FG, Some(BG)),
    }
}

fn hint(parts: &[(&str, bool)]) -> StyledText {
    let mut text = StyledText::new();
    for &(part, key) in parts {
        if key {
            text.push_key(part);
        } else {
            text.push_plain(part);
        }
    }
    text
}

fn glyphs(buf: &GlyphBuffer, y: i32) -> String {
    (0..i32::from(buf.width()))
        .filter_map(|x| buf.get(x, y).map(|c| c.glyph))
        .collect()
}

#[test]
fn a_key_is_a_capital_in_bold_moved_into_its_box_with_a_cell_of_room() {
    let mut buf = capped(12);
    let text = hint(&[("f", true), (" select", false)]);
    assert_eq!(buf.widths().key_cap_cells(), Some(1));
    assert_eq!(text.width(buf.widths()), 9);
    assert_eq!(buf.print_styled(0, 0, &text, colors), 9);
    assert_eq!(glyphs(&buf, 0), "F  select   ");
    assert_eq!(text.plain(), "f select");
    // The key's colour, on the plain text's background: the box is drawn
    // under the glyphs, the cells keep the line's.
    assert_eq!(*buf.get(0, 0).unwrap(), Cell::new('F', KEY, BG));
    assert_eq!(*buf.get(1, 0).unwrap(), Cell::new(' ', KEY, BG));
    assert_eq!(*buf.get(2, 0).unwrap(), Cell::new(' ', FG, BG));
    assert_eq!(buf.glyph_style(0, 0), GlyphStyle::new(3, 1, true));
    assert_eq!(buf.glyph_style(1, 0), GlyphStyle::PLAIN);
    let under = |x, y, w, h, color| Overlay::new(Rect::new(x, y, w, h), color, Layer::Under);
    assert_eq!(
        buf.overlays(),
        [
            under(2, 1, 11, 14, FILL),
            under(2, 0, 11, 1, EDGE),
            under(2, 15, 11, 1, EDGE),
            under(1, 1, 1, 14, EDGE),
            under(13, 1, 1, 14, EDGE),
        ]
    );
}

#[test]
fn a_cap_followed_by_a_full_stop_or_a_comma_leaves_their_cells_alone() {
    for mark in [".", ",", ":"] {
        let mut buf = capped(6);
        buf.print_styled(0, 1, &hint(&[("Enter", true), (mark, false)]), colors);
        // Enter takes cells 0 to 5 with its room; the mark is cell 6, off
        // this buffer, so print it in a wider one.
        let mut wide = capped(8);
        wide.print_styled(0, 1, &hint(&[("Enter", true), (mark, false)]), colors);
        assert_eq!(glyphs(&wide, 1), format!("Enter {mark} "));
        let cell = PxRect::new(6 * 8, 16, 8, 16);
        for item in wide.items() {
            assert!(
                item.visible().intersect(&cell).is_none(),
                "{mark}: {item:?}"
            );
        }
        assert_eq!(wide.glyph_style(6, 1), GlyphStyle::PLAIN);
        assert!(buf.overlays().len() == 5, "Enter fits in six cells");
    }
}

#[test]
fn a_cap_the_edge_cuts_has_no_box_and_its_glyphs_are_plain() {
    let mut buf = capped(4);
    let written = buf.print_styled(2, 0, &hint(&[("Space", true)]), colors);
    assert_eq!(written, 2);
    assert_eq!(glyphs(&buf, 0), "  Sp");
    assert!(buf.items().is_empty());
    assert!(buf.glyph_styles().next().is_none());
    let mut left = capped(4);
    left.print_styled(-1, 0, &hint(&[("f", true), ("x", false)]), colors);
    assert_eq!(glyphs(&left, 0), " x  ");
    assert!(left.items().is_empty());
}

#[test]
fn replacing_a_caps_cells_makes_its_glyphs_plain_and_a_blit_moves_them() {
    let printed = || {
        let mut buf = capped(8);
        buf.print_styled(0, 0, &hint(&[("f", true)]), colors);
        buf
    };
    let mut over = printed();
    over.print(0, 0, "x", FG, BG);
    assert_eq!(over.glyph_style(0, 0), GlyphStyle::PLAIN);
    let mut set = printed();
    set.set(0, 0, Cell::new('y', FG, BG));
    assert_eq!(set.glyph_style(0, 0), GlyphStyle::PLAIN);
    let mut filled = printed();
    filled.fill_rect(Rect::new(0, 0, 1, 1), Cell::new(' ', FG, BG));
    assert!(filled.glyph_styles().next().is_none());
    assert!(filled.overlays().iter().all(|o| o.rect.x >= 8));
    let mut target = GlyphBuffer::new(10, 3, Cell::new(' ', FG, BG));
    target.blit(&printed(), 1, 1);
    let moved: Vec<_> = target.glyph_styles().collect();
    assert_eq!(moved, [(1, 1, GlyphStyle::new(3, 1, true))]);
    target.blit(&GlyphBuffer::new(3, 3, Cell::new(' ', FG, BG)), 0, 0);
    assert!(target.glyph_styles().next().is_none());
}

#[test]
fn a_glyph_style_set_by_hand_is_kept_until_its_cell_is_replaced() {
    let mut buf = GlyphBuffer::new(3, 1, Cell::new('a', FG, BG));
    buf.set_glyph_style(1, 0, GlyphStyle::new(-2, 0, false));
    buf.set_glyph_style(9, 0, GlyphStyle::new(1, 1, true));
    buf.dim(buf.bounds(), 0.5);
    assert_eq!(buf.glyph_style(1, 0), GlyphStyle::new(-2, 0, false));
    assert_eq!(buf.glyph_style(9, 0), GlyphStyle::PLAIN);
    buf.set_glyph_style(1, 0, GlyphStyle::PLAIN);
    assert!(buf.glyph_styles().next().is_none());
}

#[test]
fn without_caps_a_key_is_text_and_its_widths_count_no_room() {
    let mut buf = capped(10);
    buf.set_key_caps(None);
    assert_eq!(buf.widths().key_cap_cells(), None);
    let text = hint(&[("f", true), (" select", false)]);
    assert_eq!(buf.print_styled(0, 0, &text, colors), 8);
    assert_eq!(*buf.get(0, 0).unwrap(), Cell::new('f', KEY, KEY_BG));
    assert!(buf.items().is_empty() && buf.glyph_styles().next().is_none());
}

#[test]
fn new_widths_keep_the_caps_room() {
    let mut buf = capped(10);
    buf.set_widths(CellWidths::NARROW);
    assert_eq!(buf.widths().key_cap_cells(), Some(1));
}

#[test]
fn a_snapshot_lists_the_glyph_styles() {
    let mut buf = capped(6);
    buf.print_styled(0, 0, &hint(&[("f", true), (".", false)]), colors);
    let snapshot = buf.to_snapshot(&engine_palette());
    assert!(
        snapshot.ends_with("--- glyph styles ---\n0,0  +3,+1 bold\n"),
        "{snapshot}"
    );
    let plain = GlyphBuffer::new(2, 1, Cell::new(' ', FG, BG));
    assert!(
        !plain
            .to_snapshot(&engine_palette())
            .contains("glyph styles")
    );
}

proptest! {
    /// With caps on, a line advances by its width, the plain text's plus
    /// a cell of room a key, and its text is the same.
    #[test]
    fn a_line_with_caps_advances_by_its_width(
        parts in prop::collection::vec(("[a-zA-Z .,]{1,6}", any::<bool>()), 0..6),
    ) {
        let mut text = StyledText::new();
        for (part, key) in &parts {
            if *key { text.push_key(part); } else { text.push_plain(part); }
        }
        let mut buf = capped(200);
        let keys = text.spans().iter().filter(|s| s.role == Some(TextRole::Key)).count();
        let plain = text.plain();
        prop_assert_eq!(text.width(buf.widths()), plain.chars().count() + keys);
        let written = buf.print_styled(0, 0, &text, colors);
        prop_assert_eq!(usize::from(written), text.width(buf.widths()));
        prop_assert_eq!(text.plain(), plain);
    }
}
