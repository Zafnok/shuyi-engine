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
fn a_cap_one_cell_short_of_its_room_has_no_box() {
    // `f` at the last column: its name fits, its extra cell doesn't.
    let mut buf = capped(4);
    buf.print_styled(3, 0, &hint(&[("f", true)]), colors);
    assert_eq!(glyphs(&buf, 0), "   F");
    assert_eq!(buf.items().len(), 0);
    assert_eq!(buf.glyph_style(3, 0), GlyphStyle::PLAIN);
    // At the column before, it fits.
    buf.print_styled(2, 1, &hint(&[("f", true)]), colors);
    assert_eq!(buf.overlays().len(), 5);
}

#[test]
fn a_shorter_box_sits_down_the_line_on_the_row_it_is_printed_on() {
    let buf = GlyphBuffer::new(4, 2, Cell::new(' ', FG, BG)).with_widths(terminus());
    let style = KeyCapStyle::new().with_margin_px(1);
    let caps = KeyCaps::with_colors(style, (Some(FILL), Some(EDGE)), &terminus(), buf.spec());
    let mut buf = buf.with_key_caps(Some(caps));
    buf.print_styled(1, 1, &hint(&[("f", true)]), colors);
    // The box is 11 × 14 px, 2 px into the cap's cells and 1 down: from
    // (10, 17), its top edge one pixel in from each corner.
    let under = |x, y, w, h, color| Overlay::new(Rect::new(x, y, w, h), color, Layer::Under);
    assert_eq!(buf.overlays()[1], under(11, 17, 9, 1, EDGE));
    assert_eq!(buf.overlays()[2], under(11, 30, 9, 1, EDGE));
}

#[test]
fn printing_over_a_cell_leaves_its_neighbours_styles() {
    let mut buf = GlyphBuffer::new(4, 1, Cell::new(' ', FG, BG));
    let bold = GlyphStyle::new(1, 0, true);
    for x in 0..4 {
        buf.set_glyph_style(x, 0, bold);
    }
    buf.print(1, 0, "x", FG, BG);
    let styled: Vec<i32> = buf.glyph_styles().map(|(x, _, _)| x).collect();
    assert_eq!(styled, [0, 2, 3]);
    buf.print(3, 0, "y", FG, BG);
    let styled: Vec<i32> = buf.glyph_styles().map(|(x, _, _)| x).collect();
    assert_eq!(styled, [0, 2]);
}

#[test]
fn a_cap_the_edge_cuts_has_no_box_and_its_glyphs_are_plain() {
    let mut buf = capped(4);
    let written = buf.print_styled(2, 0, &hint(&[("Space", true)]), colors);
    assert_eq!(written, 2);
    assert_eq!(glyphs(&buf, 0), "  Sp");
    assert_eq!(buf.items().len(), 0);
    assert!(buf.glyph_styles().next().is_none());
    let mut left = capped(4);
    left.print_styled(-1, 0, &hint(&[("f", true), ("x", false)]), colors);
    assert_eq!(glyphs(&left, 0), " x  ");
    assert_eq!(left.items().len(), 0);
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

/// A made-up 24 × 16 picture of the arrow cluster, in a 32 × 16 image.
fn arrows_image() -> (ImageId, PxRect) {
    use shuyi_assets::image::{ImageInfo, ImageTable};
    let table = ImageTable::new([("arrows.png", ImageInfo::new(32, 16))].into());
    (table.id("arrows.png").unwrap(), Rect::new(8, 0, 24, 16))
}

/// [`capped`], its caps with [`arrows_image`].
fn pictured(w: u16) -> GlyphBuffer {
    let (image, src) = arrows_image();
    let buf = GlyphBuffer::new(w, 2, Cell::new('.', FG, BG)).with_widths(terminus());
    let style = KeyCapStyle::new().with_arrows(image, src);
    let caps = KeyCaps::with_colors(style, (Some(FILL), Some(EDGE)), &terminus(), buf.spec());
    buf.with_key_caps(Some(caps))
}

fn arrows_hint() -> StyledText {
    let mut text = StyledText::new();
    text.push_arrows("arrows").push_plain(" move");
    text
}

#[test]
fn the_arrow_keys_are_the_picture_in_the_cells_it_takes() {
    let mut buf = pictured(12);
    assert_eq!(buf.widths().arrows_cells(), Some(3));
    let text = arrows_hint();
    assert_eq!(text.width(buf.widths()), 8);
    assert_eq!(buf.print_styled(1, 1, &text, colors), 8);
    assert_eq!(glyphs(&buf, 1), ".    move...");
    // Blank on the plain text's background, the picture over them.
    assert_eq!(*buf.get(1, 1).unwrap(), Cell::new(' ', FG, BG));
    let (image, src) = arrows_image();
    let dest = Rect::new(8, 16, 24, 16);
    assert_eq!(buf.sprites(), [Sprite::new(image, src, dest, Layer::Over)]);
    assert!(buf.overlays().is_empty() && buf.glyph_styles().next().is_none());
}

#[test]
fn a_picture_is_scaled_to_the_line_and_centred_in_its_cells() {
    let (image, _) = arrows_image();
    let spec = ConsoleSpec::text(10, 1);
    let style = KeyCapStyle::new().with_arrows(image, Rect::new(0, 0, 12, 8));
    let caps = KeyCaps::with_colors(style, (None, None), &terminus(), spec);
    // 12 × 8 to a line 16 tall: 24 × 16, three 8 px cells.
    assert_eq!(caps.arrows_cells(), Some(3));
    let odd = KeyCapStyle::new().with_arrows(image, Rect::new(0, 0, 10, 16));
    let caps = KeyCaps::with_colors(odd, (None, None), &terminus(), spec);
    assert_eq!(caps.arrows_cells(), Some(2));
    let sprite = caps.arrows_sprite(2, 0).unwrap();
    // 10 px in 16: 3 px either side.
    assert_eq!(sprite.dest, Rect::new(19, 0, 10, 16));
    let empty = KeyCapStyle::new().with_arrows(image, Rect::new(0, 0, 0, 16));
    let caps = KeyCaps::with_colors(empty, (None, None), &terminus(), spec);
    assert_eq!(
        (caps.arrows_cells(), caps.arrows_sprite(0, 0)),
        (None, None)
    );
}

#[test]
fn without_a_picture_the_arrows_are_a_cap_and_without_caps_text() {
    let mut buf = capped(12);
    assert_eq!(buf.widths().arrows_cells(), None);
    let text = arrows_hint();
    assert_eq!(buf.print_styled(0, 0, &text, colors), 12);
    assert_eq!(glyphs(&buf, 0), "arrows  move");
    assert!(buf.sprites().is_empty() && !buf.overlays().is_empty());
    let mut buf = pictured(12);
    buf.set_key_caps(None);
    assert_eq!(buf.widths().arrows_cells(), None);
    assert_eq!(buf.print_styled(0, 0, &text, colors), 11);
    assert_eq!(glyphs(&buf, 0), "arrows move.");
    assert_eq!(buf.items(), []);
}

#[test]
fn a_picture_the_edge_cuts_is_clipped_and_new_widths_keep_its_cells() {
    let mut buf = pictured(2);
    assert_eq!(buf.print_styled(0, 0, &arrows_hint(), colors), 2);
    let sprite = buf.sprites()[0];
    assert_eq!(sprite.clip, Rect::new(0, 0, 16, 16));
    // Off the rows, nothing.
    assert_eq!(buf.print_styled(0, 5, &arrows_hint(), colors), 0);
    assert_eq!(buf.sprites().len(), 1);
    buf.set_widths(CellWidths::NARROW);
    assert_eq!(buf.widths().arrows_cells(), Some(3));
}
