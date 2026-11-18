use shuyi_assets::font::InkBox;

use super::*;
use crate::frame::color::tests::engine_def;
use crate::frame::color::{EngineColor, KEY};

/// Terminus 8 × 16's ink: its capitals' columns 1 to 6, rows 2 to 11,
/// and its `W` inked from column 0.
fn terminus_ink() -> FontInk {
    let mut ink = FontInk::new(InkBox::new(1, 2, 6, 11));
    ink.columns.insert('W', [0, 6]);
    ink
}

fn terminus() -> CellWidths {
    let mut atlas =
        shuyi_assets::font::FontAtlasDef::new(8, 16, alloc::collections::BTreeMap::new(), 0);
    atlas.ink = Some(terminus_ink());
    CellWidths::of(&atlas)
}

fn caps(style: KeyCapStyle, widths: &CellWidths) -> KeyCaps {
    KeyCaps::with_colors(style, (None, None), widths, ConsoleSpec::text(40, 4))
}

#[test]
fn the_style_is_bold_capitals_and_a_two_pixel_margin_off_a_one_bit_screen() {
    let style = KeyCapStyle::default();
    assert_eq!(style, KeyCapStyle::new());
    assert!(style.bold && style.capitals && !style.one_bit);
    assert_eq!(style.margin_px, 2);
    let changed = style
        .with_bold(false)
        .with_capitals(false)
        .with_margin_px(3)
        .with_one_bit(true);
    assert!(!changed.bold && !changed.capitals && changed.one_bit);
    assert_eq!(changed.margin_px, 3);
}

#[test]
fn a_bold_letter_struck_twice_sits_two_pixels_inside_its_box_on_every_side() {
    // Bold, struck twice: the ink is columns 1 to 7. The box is
    // 3 px outside it (2 of margin, 1 of edge): 13 px, in two cells.
    let layout = caps(KeyCapStyle::new(), &terminus()).layout("F", &terminus());
    assert_eq!(layout.extra, 1);
    assert_eq!(layout.frame, Rect::new(1, 0, 13, 16));
    assert_eq!(layout.style, GlyphStyle::new(3, 1, true));
    // Ink at columns 4 to 10 and rows 3 to 12: 2 px from the edge's
    // inside, at 2 and 13 across and 1 and 14 down.
    let ink_left = i32::from(layout.style.dx) + 1;
    let ink_right = i32::from(layout.style.dx) + 7;
    let ink_top = i32::from(layout.style.dy) + 2;
    let ink_bottom = i32::from(layout.style.dy) + 11;
    let f = layout.frame;
    assert_eq!(ink_left - (f.x + 1), 2);
    assert_eq!((f.x + f.w - 2) - ink_right, 2);
    assert_eq!(ink_top - (f.y + 1), 2);
    assert_eq!((f.y + f.h - 2) - ink_bottom, 2);
}

#[test]
fn a_longer_name_takes_one_more_cell_and_the_same_placement() {
    let caps = caps(KeyCapStyle::new(), &terminus());
    let space = caps.layout("Space", &terminus());
    assert_eq!(space.extra, 1);
    assert_eq!(space.frame, Rect::new(1, 0, 45, 16));
    assert_eq!(space.style, GlyphStyle::new(3, 1, true));
    assert_eq!(caps.extra_cells(), 1);
}

#[test]
fn a_glyph_inked_wider_gets_a_wider_box_and_the_same_margin() {
    // W's ink is columns 0 to 6, 0 to 7 struck twice: a 14 px box.
    let caps = caps(KeyCapStyle::new(), &terminus());
    let w = caps.layout("W", &terminus());
    assert_eq!(w.frame, Rect::new(1, 0, 14, 16));
    assert_eq!(w.style, GlyphStyle::new(4, 1, true));
    // A name ending in W reaches its last column.
    let fw = caps.layout("FW", &terminus());
    assert_eq!(fw.frame, Rect::new(1, 0, 21, 16));
    assert_eq!(fw.style.dx, 3);
}

#[test]
fn a_wide_glyph_is_placed_by_the_capitals_margins_in_its_two_cells() {
    let widths = terminus();
    let wide = CellWidths::wide(['あ']);
    let caps = caps(KeyCapStyle::new().with_bold(false), &widths);
    // Columns 1 to 14 of its two cells: a 20 px box in three cells.
    let layout = caps.layout("あ", &wide);
    assert_eq!(layout.frame, Rect::new(2, 0, 20, 16));
    assert_eq!(layout.style.dx, 4);
}

#[test]
fn a_name_not_bold_is_placed_by_its_own_ink() {
    let plain = caps(KeyCapStyle::new().with_bold(false), &terminus()).layout("F", &terminus());
    // 6 + 6 = 12 px, centred in two cells.
    assert_eq!(plain.frame, Rect::new(2, 0, 12, 16));
    assert_eq!(plain.style, GlyphStyle::new(4, 1, false));
}

#[test]
fn a_smaller_margin_makes_a_shorter_box_centred_in_the_line() {
    let layout = caps(KeyCapStyle::new().with_margin_px(1), &terminus()).layout("F", &terminus());
    // 7 + 2 × 2 = 11 px across, 10 + 4 = 14 down.
    assert_eq!(layout.frame, Rect::new(2, 1, 11, 14));
    assert_eq!(layout.style, GlyphStyle::new(3, 1, true));
}

#[test]
fn a_margin_too_big_for_the_line_keeps_the_box_the_lines_height() {
    let layout = caps(KeyCapStyle::new().with_margin_px(5), &terminus()).layout("F", &terminus());
    // 7 + 12 = 19 px across, in three cells: room for a glyph inked
    // across its whole cell and struck twice, 8 + 1 + 12.
    assert_eq!(layout.extra, 2);
    assert_eq!(layout.frame, Rect::new(2, 0, 19, 16));
    assert_eq!(layout.style.dy, 1);
}

#[test]
fn the_extra_cells_are_the_room_of_a_glyph_inked_across_its_cell_and_bold() {
    // 8 + 1 + 2 × 4 = 17 px: three cells, two more than the name.
    let wide_margin = caps(KeyCapStyle::new().with_margin_px(3), &terminus());
    assert_eq!(wide_margin.extra_cells(), 2);
    // In 7 px cells, 7 + 1 + 6 = 14 px: two cells, one more.
    let narrow = KeyCaps::with_colors(
        KeyCapStyle::new(),
        (None, None),
        &CellWidths::NARROW,
        ConsoleSpec {
            cell_w: 7,
            ..ConsoleSpec::text(10, 2)
        },
    );
    assert_eq!(narrow.extra_cells(), 1);
}

#[test]
fn an_empty_name_gets_the_box_of_a_glyph_inked_across_its_cell() {
    let caps = caps(KeyCapStyle::new().with_bold(false), &terminus());
    let layout = caps.layout("", &terminus());
    assert_eq!(layout.frame, Rect::new(1, 0, 14, 16));
}

#[test]
fn a_font_whose_ink_is_unknown_counts_the_whole_cell() {
    let layout = caps(KeyCapStyle::new().with_bold(false), &CellWidths::NARROW)
        .layout("F", &CellWidths::NARROW);
    // 8 + 6 = 14 px across; down, the box is the line's.
    assert_eq!(layout.extra, 1);
    assert_eq!(layout.frame, Rect::new(1, 0, 14, 16));
    assert_eq!(layout.style, GlyphStyle::new(4, 0, false));
}

#[test]
fn one_letter_is_shown_as_its_capital_and_longer_names_as_written() {
    let on = caps(KeyCapStyle::new(), &CellWidths::NARROW);
    assert_eq!(on.shown("f"), "F");
    assert_eq!(on.shown("é"), "É");
    for name in ["Space", "Shift+Space", "1", ",", "ß", "F"] {
        assert_eq!(on.shown(name), name, "{name}");
    }
    let off = caps(KeyCapStyle::new().with_capitals(false), &CellWidths::NARROW);
    assert_eq!(off.shown("f"), "f");
}

#[test]
fn the_box_takes_the_palettes_key_colours_its_edge_the_fill_without_one() {
    let mut palette_file = engine_def();
    palette_file.colors.insert(KEY.to_owned(), [250, 200, 60]);
    palette_file.colors.insert(KEY_BG.to_owned(), [74, 64, 46]);
    let spec = ConsoleSpec::text(10, 2);
    let filled = Palette::new::<EngineColor>(&palette_file).unwrap();
    let caps = KeyCaps::new(KeyCapStyle::new(), &filled, &CellWidths::NARROW, spec);
    let fill = Some(Rgb::new(74, 64, 46));
    assert_eq!(caps.colors(), (fill, fill));
    palette_file
        .colors
        .insert(KEY_EDGE.to_owned(), [128, 112, 82]);
    let edged = Palette::new::<EngineColor>(&palette_file).unwrap();
    let caps = KeyCaps::new(KeyCapStyle::new(), &edged, &CellWidths::NARROW, spec);
    assert_eq!(caps.colors(), (fill, Some(Rgb::new(128, 112, 82))));
    let thin = KeyCapStyle::new().with_margin_px(1);
    let caps = KeyCaps::new(thin, &edged, &CellWidths::NARROW, spec);
    assert_eq!(caps.style(), &thin);
}

/// [`terminus`] with a bold face (ticket 3216): Terminus Bold's ink, its
/// capitals' columns 1 to 6 and rows 2 to 11, `W` from column 0, `E`
/// and `F` to column 7.
fn terminus_bold() -> CellWidths {
    let mut bold = FontInk::new(InkBox::new(1, 2, 6, 11));
    bold.columns.insert('W', [0, 6]);
    bold.columns.insert('E', [1, 7]);
    bold.columns.insert('F', [1, 7]);
    let mut atlas =
        shuyi_assets::font::FontAtlasDef::new(8, 16, alloc::collections::BTreeMap::new(), 0);
    atlas.ink = Some(terminus_ink());
    atlas.bold_ink = Some(bold);
    CellWidths::of(&atlas)
}

#[test]
fn a_letter_from_a_bold_face_sits_two_pixels_inside_its_box_on_every_side() {
    let widths = terminus_bold();
    let caps = caps(KeyCapStyle::new(), &widths);
    for (name, [left, right]) in [("W", [0, 6]), ("D", [1, 6]), ("F", [1, 7]), ("E", [1, 7])] {
        let layout = caps.layout(name, &widths);
        assert!(layout.style.bold, "{name}");
        let f = layout.frame;
        let (dx, dy) = (i32::from(layout.style.dx), i32::from(layout.style.dy));
        assert_eq!(dx + left - (f.x + 1), 2, "{name} left");
        assert_eq!((f.x + f.w - 2) - (dx + right), 2, "{name} right");
        assert_eq!(dy + 2 - (f.y + 1), 2, "{name} top");
        assert_eq!((f.y + f.h - 2) - (dy + 11), 2, "{name} bottom");
    }
    // D's bold ink is the capitals', columns 1 to 6, and no column wider:
    // a 12 px box, not the 13 px of the regular face struck twice.
    assert_eq!(caps.layout("D", &widths).frame.w, 12);
    let struck = self::caps(KeyCapStyle::new(), &terminus());
    assert_eq!(struck.layout("D", &terminus()).frame.w, 13);
}

#[test]
fn a_bold_face_counts_no_extra_column_and_plain_names_keep_the_regular_ink() {
    // 8 + 2 × 4 = 16 px: two cells, one more than the name; struck twice
    // it is 17, three cells.
    let margin = KeyCapStyle::new().with_margin_px(3);
    assert_eq!(caps(margin, &terminus_bold()).extra_cells(), 1);
    assert_eq!(caps(margin, &terminus()).extra_cells(), 2);
    let widths = terminus_bold();
    let plain = caps(KeyCapStyle::new().with_bold(false), &widths).layout("F", &widths);
    assert_eq!(plain.frame, Rect::new(2, 0, 12, 16));
    assert_eq!(plain.style, GlyphStyle::new(4, 1, false));
}
