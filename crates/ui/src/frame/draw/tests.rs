//! The drawing helpers' looks, read from cells.

use super::*;
use crate::frame::slice::NineSlice;

const WHITE: Rgb = Rgb::new(255, 255, 255);
const RED: Rgb = Rgb::new(255, 0, 0);
const BLUE: Rgb = Rgb::new(0, 0, 255);
const BLANK: Cell = Cell::new('.', WHITE, BLUE);
const FULL: Cell = Cell::new('█', RED, BLUE);
const EMPTY: Cell = Cell::new('░', WHITE, BLUE);

/// A 10-cell meter showing `filled` of `of`.
fn meter(filled: u32, of: u32) -> String {
    let mut buf = GlyphBuffer::new(10, 1, BLANK);
    buf.meter(buf.bounds(), filled, of, FULL, EMPTY);
    buf.row_text(0)
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
    assert_eq!(buf.row_text(0), "....");
    assert_eq!(buf.row_text(1), "██░░");
    assert_eq!(buf.row_text(2), "██░░");
    assert_eq!(buf.get(0, 1), Some(&FULL));
    assert_eq!(buf.get(3, 2), Some(&EMPTY));
    // Narrower than its row: the rest of the row is untouched.
    buf.meter(Rect::new(1, 0, 2, 1), 1, 4, FULL, EMPTY);
    assert_eq!(buf.row_text(0), ".█░.");
    buf.meter(Rect::new(0, 0, 0, 1), 1, 2, FULL, EMPTY);
    assert_eq!(buf.row_text(0), ".█░.");
}

#[test]
fn a_panel_looks_new_and_with_methods_build_what_a_literal_did() {
    const LOOK: PanelLook = PanelLook::new(BoxStyle::Double, WHITE, BLUE)
        .with_border(RED)
        .with_title(BLUE);
    let literal = PanelLook {
        style: BoxStyle::Double,
        border: RED,
        title: BLUE,
        text: WHITE,
        bg: BLUE,
        frame: None,
        plate: None,
    };
    assert_eq!(LOOK, literal);
}

#[test]
fn a_panel_clears_frames_and_sets_its_title_in_the_top_border() {
    let mut buf = GlyphBuffer::new(12, 4, BLANK);
    let look = PanelLook::new(BoxStyle::Double, WHITE, BLUE).with_title(RED);
    buf.panel(buf.bounds(), &look, Some("Shop"));
    assert_eq!(buf.row_text(0), "╔═ Shop ═══╗");
    assert_eq!(buf.row_text(1), "║          ║");
    assert_eq!(buf.row_text(3), "╚══════════╝");
    assert_eq!(buf.get(3, 0), Some(&Cell::new('S', RED, BLUE)));
    assert_eq!(buf.get(1, 0), Some(&Cell::new('═', WHITE, BLUE)));
    assert_eq!(buf.get(5, 2), Some(&Cell::new(' ', WHITE, BLUE)));
    let look = look.with_border(RED);
    buf.panel(buf.bounds(), &look, None);
    assert_eq!(buf.get(0, 0), Some(&Cell::new('╔', RED, BLUE)));
    assert_eq!(buf.get(5, 2), Some(&Cell::new(' ', WHITE, BLUE)));
}

#[test]
fn a_title_longer_than_the_panel_is_cut() {
    let mut buf = GlyphBuffer::new(12, 3, BLANK);
    let look = PanelLook::new(BoxStyle::Single, WHITE, BLUE);
    buf.panel(buf.bounds(), &look, Some("Pause menu"));
    assert_eq!(buf.row_text(0), "┌─ Pause… ─┐");
    // Too narrow for a cell of title: the border alone.
    let mut buf = GlyphBuffer::new(6, 3, BLANK);
    buf.panel(buf.bounds(), &look, Some("Shop"));
    assert_eq!(buf.row_text(0), "┌────┐");
    buf.panel(buf.bounds(), &look, None);
    assert_eq!(buf.row_text(0), "┌────┐");
}

#[test]
fn centring_an_odd_and_an_even_width() {
    let mut buf = GlyphBuffer::new(7, 1, BLANK);
    buf.print_centred(buf.bounds(), "abc", RED);
    assert_eq!(buf.row_text(0), "..abc..");
    let mut buf = GlyphBuffer::new(7, 1, BLANK);
    buf.print_centred(buf.bounds(), "ab", RED);
    assert_eq!(buf.row_text(0), "..ab...");
    let mut buf = GlyphBuffer::new(8, 1, BLANK);
    buf.print_centred(buf.bounds(), "abc", RED);
    assert_eq!(buf.row_text(0), "..abc...");
    let mut buf = GlyphBuffer::new(8, 1, BLANK);
    buf.print_centred(buf.bounds(), "ab", RED);
    assert_eq!(buf.row_text(0), "...ab...");
}

#[test]
fn centring_in_a_rectangle_cuts_and_keeps_the_background() {
    let mut buf = GlyphBuffer::new(8, 4, BLANK);
    let written = buf.print_centred(Rect::new(2, 0, 4, 4), "abcdef", RED);
    assert_eq!(written, 4);
    assert_eq!(buf.row_text(1), "..abc…..");
    assert_eq!(buf.get(2, 1), Some(&Cell::new('a', RED, BLUE)));
    assert_eq!(buf.print_centred(Rect::new(0, 0, 0, 1), "a", RED), 0);
    // Five rows: the third; two: the first.
    let mut buf = GlyphBuffer::new(1, 5, BLANK);
    buf.print_centred(buf.bounds(), "a", RED);
    assert_eq!(buf.get(0, 2).unwrap().glyph, 'a');
    let mut buf = GlyphBuffer::new(1, 2, BLANK);
    buf.print_centred(buf.bounds(), "a", RED);
    assert_eq!(buf.get(0, 0).unwrap().glyph, 'a');
}

#[test]
fn wide_text_is_centred_and_cut_by_its_cells() {
    let widths = CellWidths::wide(['あ', 'い']);
    let mut buf = GlyphBuffer::new(8, 1, BLANK).with_widths(widths.clone());
    // Four cells, centred in eight: two each side.
    assert_eq!(buf.print_centred(buf.bounds(), "あい", RED), 4);
    assert_eq!(buf.get(2, 0).unwrap().glyph, 'あ');
    assert_eq!(buf.get(4, 0).unwrap().glyph, 'い');
    let mut buf = GlyphBuffer::new(8, 1, BLANK).with_widths(widths.clone());
    assert_eq!(buf.print_cut(0, 0, 4, "あいA", RED), 3);
    assert_eq!(buf.get(2, 0).unwrap().glyph, '…');
    assert_eq!(cut("aい", 2, &widths), "a…");
    assert_eq!(cut("いa", 3, &widths), "いa");
}

#[test]
fn printing_cut_to_a_width() {
    let mut buf = GlyphBuffer::new(8, 1, BLANK);
    assert_eq!(buf.print_cut(1, 0, 4, "Escape", RED), 4);
    assert_eq!(buf.row_text(0), ".Esc…...");
    assert_eq!(buf.get(1, 0), Some(&Cell::new('E', RED, BLUE)));
    buf.print_cut(0, 0, 8, "Enter", RED);
    assert_eq!(buf.row_text(0), "Enter...");
}

/// A 24×24 frame cut 10 on every side, and a 33×32 plate with 16-pixel
/// caps.
fn pictures() -> (NineSlice, NineSlice) {
    use shuyi_assets::image::{ImageInfo, ImageRect, ImageTable, Picture};
    use shuyi_assets::nine_slice::Borders;
    let table = ImageTable::new(
        [
            ("frame.png", ImageInfo::new(24, 24)),
            ("plate.png", ImageInfo::new(33, 32)),
        ]
        .into(),
    );
    let picture = |path: &str, w, h| Picture {
        image: table.id(path).unwrap(),
        rect: ImageRect { x: 0, y: 0, w, h },
    };
    let frame = NineSlice::new(picture("frame.png", 24, 24), Borders::uniform(10));
    let caps = Borders {
        left: 16,
        top: 0,
        right: 16,
        bottom: 0,
    };
    (frame, NineSlice::new(picture("plate.png", 33, 32), caps))
}

/// A 10×5 framed panel at (2, 3) in a 16×10 buffer of 8×16 cells.
fn framed(outset: u16, title: Option<&str>) -> GlyphBuffer {
    let (frame, plate) = pictures();
    let look = PanelLook::new(BoxStyle::Single, WHITE, BLUE)
        .with_title(RED)
        .with_frame(Some(PanelFrame::new(frame).with_outset(outset)))
        .with_plate(Some(TitlePlate::new(plate).with_pad(6).with_text_top(8)));
    let mut buf = GlyphBuffer::new(16, 10, BLANK);
    buf.panel(Rect::new(2, 3, 10, 5), &look, title);
    buf
}

#[test]
fn a_framed_panel_draws_no_lines_and_keeps_its_cells() {
    let buf = framed(0, Some("Shop"));
    assert_eq!(buf.row_text(3), "..   Shop   ....");
    assert_eq!(buf.row_text(4), "..          ....");
    assert_eq!(buf.row_text(7), "..          ....");
    assert_eq!(buf.get(2, 4), Some(&Cell::new(' ', WHITE, BLUE)));
    assert_eq!(buf.get(5, 3), Some(&Cell::new('S', RED, BLUE)));
}

#[test]
fn a_frame_lies_over_the_border_cells_and_never_the_inside() {
    // The panel's pixels: (16, 48) to (96, 128); inside (24, 64) to (88, 112).
    let inside = Rect::new(24, 64, 64, 48);
    for outset in [0, 5] {
        let buf = framed(outset, None);
        let sprites = buf.sprites();
        assert_ne!(sprites.len(), 0);
        let o = i32::from(outset);
        let outer = Rect::new(16 - o, 48 - o, 80 + 2 * o, 80 + 2 * o);
        for s in &sprites {
            assert_eq!(s.layer, Layer::Under);
            assert!(s.clip.intersect(&inside).is_none(), "{s:?} over the inside");
            assert!(outer.intersect(&s.clip) == Some(s.clip), "{s:?} outside");
        }
        // The top-left corner reaches the outset's edge.
        assert!(
            sprites
                .iter()
                .any(|s| s.dest == Rect::new(16 - o, 48 - o, 10, 10))
        );
        // No middle: nothing from the picture's (10, 10, 4, 4).
        assert!(sprites.iter().all(|s| s.src != Rect::new(10, 10, 4, 4)));
    }
}

#[test]
fn a_title_plate_hugs_the_title_with_its_pad_and_caps() {
    let buf = framed(5, Some("Shop"));
    let image = pictures().1.picture.image;
    let plates: Vec<_> = buf
        .sprites()
        .into_iter()
        .filter(|s| s.image == image)
        .collect();
    assert_eq!(plates.len(), 3);
    // The title "Shop" is on cells 5..9 of row 3: pixels (40, 48), 32 wide.
    // Its body is 32 + 2 × 6 wide, its caps 16 each, its top 8 above.
    let left = plates.iter().find(|s| s.src.x == 0).unwrap();
    assert_eq!(left.dest, Rect::new(40 - 6 - 16, 40, 16, 32));
    let body = plates.iter().find(|s| s.src.x == 16).unwrap();
    assert_eq!(body.dest, Rect::new(40 - 6, 40, 44, 32));
    let right = plates.iter().find(|s| s.src.x == 17).unwrap();
    assert_eq!(right.dest, Rect::new(40 + 32 + 6, 40, 16, 32));
    // Drawn after the frame, so over it.
    let items = buf.sprites();
    let last_frame = items.iter().rposition(|s| s.image != left.image).unwrap();
    let first_plate = items.iter().position(|s| s.image == left.image).unwrap();
    assert!(first_plate > last_frame);
}

#[test]
fn a_plate_without_a_frame_and_a_frame_without_a_title() {
    let (_, plate) = pictures();
    let look =
        PanelLook::new(BoxStyle::Single, WHITE, BLUE).with_plate(Some(TitlePlate::new(plate)));
    let mut buf = GlyphBuffer::new(12, 3, BLANK);
    buf.panel(buf.bounds(), &look, Some("Shop"));
    assert_eq!(buf.row_text(0), "┌─ Shop ───┐");
    assert_eq!(buf.sprites().len(), 3);
    buf.panel(buf.bounds(), &look, None);
    assert_eq!(buf.sprites().len(), 0, "the fill cut the old plate away");
}

#[test]
fn a_one_cell_panel_with_a_frame_draws_its_corners() {
    let (frame, _) = pictures();
    let look =
        PanelLook::new(BoxStyle::Single, WHITE, BLUE).with_frame(Some(PanelFrame::new(frame)));
    let mut buf = GlyphBuffer::new(4, 2, BLANK);
    buf.panel(Rect::new(1, 0, 1, 1), &look, Some("A long title"));
    assert_eq!(buf.row_text(0), ". ..");
    let sprites = buf.sprites();
    // An 8×16 cell: corners cut to 4 + 4 across and 8 + 8 down, no edges.
    assert_eq!(sprites.len(), 4, "{sprites:?}");
    let area: i32 = sprites.iter().map(|s| s.clip.w * s.clip.h).sum();
    assert_eq!(area, 8 * 16);
}

#[test]
fn a_frame_at_the_buffers_edge_keeps_its_corners_inside_it() {
    let (frame, _) = pictures();
    let look = PanelLook::new(BoxStyle::Single, WHITE, BLUE)
        .with_frame(Some(PanelFrame::new(frame).with_outset(5)));
    let mut buf = GlyphBuffer::new(16, 10, BLANK);
    // At the top right: the outset is kept on the left and below only.
    buf.panel(Rect::new(10, 0, 6, 4), &look, None);
    let sprites = buf.sprites();
    let corner = |x, y| sprites.iter().any(|s| s.dest == Rect::new(x, y, 10, 10));
    assert!(corner(80 - 5, 0), "the top-left corner at the top edge");
    assert!(corner(128 - 10, 0), "the top-right corner whole");
    assert!(
        corner(128 - 10, 64 + 5 - 10),
        "the bottom-right corner whole"
    );
}

#[test]
fn a_frames_border_blanks_only_the_border_cells() {
    let (frame, _) = pictures();
    let look =
        PanelLook::new(BoxStyle::Single, WHITE, BLUE).with_frame(Some(PanelFrame::new(frame)));
    let mut buf = GlyphBuffer::new(8, 6, BLANK);
    // Only the border: the inside keeps what was there.
    buf.panel_border(Rect::new(1, 1, 5, 4), &look);
    assert_eq!(buf.row_text(0), "........");
    assert_eq!(buf.row_text(1), ".     ..");
    assert_eq!(buf.row_text(2), ". ... ..");
    assert_eq!(buf.row_text(4), ".     ..");
    assert_eq!(buf.row_text(5), "........");
    // The frame shows over every border cell: the bottom-right one, and
    // the right and bottom ones beside the inside.
    let sprites = buf.sprites();
    let shown = |x, y| sprites.iter().any(|s| s.clip.contains(x, y));
    assert!(shown(5 * 8 + 4, 4 * 16 + 8), "the bottom-right corner");
    assert!(shown(5 * 8 + 4, 2 * 16 + 8), "the right edge");
    assert!(shown(3 * 8 + 4, 4 * 16 + 8), "the bottom edge");
}

#[test]
fn a_panel_off_the_buffer_keeps_its_frame_where_it_is() {
    let (frame, _) = pictures();
    let look = PanelLook::new(BoxStyle::Single, WHITE, BLUE)
        .with_frame(Some(PanelFrame::new(frame).with_outset(5)));
    let mut buf = GlyphBuffer::new(8, 6, BLANK);
    // Wholly left of the buffer: only its outset reaches in, and the frame
    // isn't moved onto the buffer.
    buf.panel_border(Rect::new(-2, 1, 2, 3), &look);
    let sprites = buf.sprites();
    assert_ne!(sprites.len(), 0);
    assert!(sprites.iter().all(|s| s.dest.x < 0), "{sprites:?}");
}

#[test]
fn a_plate_at_a_zoom_scales_its_caps_height_and_text_row() {
    let (_, plate) = pictures();
    let plate = TitlePlate::new(plate.with_zoom(2))
        .with_pad(3)
        .with_text_top(4);
    let mut buf = GlyphBuffer::new(20, 8, BLANK);
    buf.title_plate(&plate, Rect::new(8, 4, 2, 1));
    let body = buf.sprites().into_iter().find(|s| s.src.w == 1).unwrap();
    // The text at (64, 64), 16 wide: the caps 32 each, 8 rows of text_top.
    assert_eq!(body.dest, Rect::new(64 - 3, 64 - 8, 16 + 6, 64));
    let left = buf.sprites().into_iter().find(|s| s.src.x == 0).unwrap();
    assert_eq!(left.dest, Rect::new(64 - 3 - 32, 64 - 8, 32, 64));
}
