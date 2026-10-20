//! The one-bit renderer, pixel by pixel. Pictures are text, a row a line:
//! `#` black, `.` white.

use alloc::rc::Rc;
use alloc::string::String;

use shuyi_assets::font::FontAtlasDef;
use shuyi_assets::image::ImageId;
use shuyi_assets::ink::Ink;

use super::*;
use crate::frame::console::ConsoleSpec;
use crate::frame::glyph_buffer::{Cell, Layer, Overlay, Rect, Sprite};
use crate::prelude::*;

/// The colour reserved for `ink`.
fn c(ink: Ink) -> Rgb {
    Rgb::from(ink.code())
}

/// A test font of 8 × 16 cells: `X` a 2 × 2 block (columns 3–4, rows
/// 6–7), `?` the two top corners, `[` its left column, `]` its right one,
/// `"` two dots one pixel apart (columns 2 and 4 of row 8).
fn test_font() -> (FontAtlasDef, Vec<u8>) {
    let atlas = FontAtlasDef {
        cell_w: 8,
        cell_h: 16,
        columns: 32,
        glyphs: [('X', 0), ('?', 1), ('[', 2), (']', 3), ('"', 4)].into(),
    };
    let mut bits = vec![0u8; 5 * 16];
    bits[6] = 0b0001_1000;
    bits[7] = 0b0001_1000;
    bits[16] = 0b1000_0001;
    bits[32..48].fill(0b1000_0000);
    bits[48..64].fill(0b0000_0001);
    bits[72] = 0b0010_1000;
    (atlas, bits)
}

/// A buffer of `w × h` Playdate cells, blank on white.
fn buf(w: u16, h: u16) -> GlyphBuffer {
    let blank = Cell::new(' ', c(Ink::Black), c(Ink::White));
    GlyphBuffer::with_spec(ConsoleSpec::text(50, 15).sized(w, h), blank)
}

/// `buf` rendered with the test font and `images`.
fn draw_with(buf: &GlyphBuffer, images: &impl Images) -> Box<Frame> {
    let (atlas, bits) = test_font();
    let font = Font::new(&atlas, &bits).unwrap();
    let mut frame = Box::new([0u8; FRAME_BYTES]);
    render(buf, &font, images, &mut frame);
    frame
}

fn draw(buf: &GlyphBuffer) -> Box<Frame> {
    draw_with(buf, &())
}

/// The pixels of `w × h` from `(x, y)`.
fn text(frame: &Frame, x: i32, y: i32, w: i32, h: i32) -> Vec<String> {
    (y..y + h)
        .map(|y| {
            (x..x + w)
                .map(|x| if is_white(frame, x, y) { '.' } else { '#' })
                .collect()
        })
        .collect()
}

/// The top-left 8 × 4 pixels of a cell filled in `ink`.
fn fill_of(ink: Ink) -> Vec<String> {
    let mut b = buf(2, 1);
    b.fill_rect(b.bounds(), Cell::new(' ', c(Ink::Black), c(ink)));
    text(&draw(&b), 0, 0, 8, 4)
}

#[test]
fn each_ink_fills_its_pattern() {
    assert_eq!(fill_of(Ink::White), ["........"; 4]);
    assert_eq!(fill_of(Ink::Black), ["########"; 4]);
    assert_eq!(fill_of(Ink::BlackEdged), ["########"; 4]);
    assert_eq!(fill_of(Ink::Invert), ["########"; 4]);
    assert_eq!(
        fill_of(Ink::Grey),
        ["#.#.#.#.", ".#.#.#.#", "#.#.#.#.", ".#.#.#.#"]
    );
    assert_eq!(
        fill_of(Ink::Dots),
        ["#...#...", "........", "..#...#.", "........"]
    );
    assert_eq!(
        fill_of(Ink::Diagonals),
        ["#...#...", ".#...#..", "..#...#.", "...#...#"]
    );
    assert_eq!(
        fill_of(Ink::Dashes),
        ["##..##..", "##..##..", "..##..##", "..##..##"]
    );
}

/// The patterns are the screen's, not the cell's: they run on across
/// cells and repeat every 4 rows.
#[test]
fn patterns_line_up_across_cells() {
    for ink in Ink::ALL {
        let mut b = buf(3, 2);
        b.fill_rect(b.bounds(), Cell::new(' ', c(Ink::Black), c(ink)));
        let frame = draw(&b);
        for y in 0..32 {
            for x in 0..24 {
                assert_eq!(
                    is_white(&frame, x, y),
                    ink_is_white(ink, x, y),
                    "{ink:?} ({x}, {y})"
                );
                assert_eq!(
                    ink_is_white(ink, x, y),
                    ink_is_white(ink, x + 8, y + 4),
                    "{ink:?} repeats"
                );
            }
        }
    }
}

#[test]
fn a_colour_without_an_ink_is_white_or_black_by_its_brightness() {
    for ink in Ink::ALL {
        assert_eq!(ink_of(c(ink)), ink);
    }
    // 299 × 128 + 587 × 128 + 114 × 127 is just under half of 255 × 1000.
    assert_eq!(ink_of(Rgb::new(128, 128, 127)), Ink::Black);
    assert_eq!(ink_of(Rgb::new(128, 128, 129)), Ink::White);
    // Exactly half: 299 × 1 + 587 × 189 + 114 × 147 = 128,000.
    assert_eq!(ink_of(Rgb::new(1, 189, 147)), Ink::White);
    assert_eq!(ink_of(Rgb::new(1, 189, 146)), Ink::Black);
    assert_eq!(ink_of(Rgb::new(0, 0, 255)), Ink::Black);
    assert_eq!(ink_of(Rgb::new(0, 255, 0)), Ink::White);
    assert_eq!(ink_of(Rgb::new(230, 0, 0)), Ink::Black);
    assert_eq!(ink_of(Rgb::new(255, 255, 254)), Ink::White);
}

#[test]
fn glyphs_draw_black_on_white_and_white_on_black() {
    let mut b = buf(2, 1);
    b.print(0, 0, "X", c(Ink::Black), c(Ink::White));
    b.print(1, 0, "X", c(Ink::White), c(Ink::Black));
    let frame = draw(&b);
    assert_eq!(
        text(&frame, 0, 5, 16, 4),
        [
            "........########",
            "...##...###..###",
            "...##...###..###",
            "........########"
        ]
    );
    assert_eq!(text(&frame, 8, 0, 8, 1), ["########"]);
}

/// An edged glyph (enemies' letters over grey) keeps a white edge one
/// pixel wide, corners too, which may reach into the next cell; the edge
/// of one glyph never covers another glyph.
#[test]
fn edged_glyphs_keep_a_white_edge() {
    let mut b = buf(2, 3);
    b.fill_rect(b.bounds(), Cell::new(' ', c(Ink::Black), c(Ink::Grey)));
    b.print_fg(0, 1, "][", c(Ink::BlackEdged));
    let frame = draw(&b);
    // x 4 to 11, from the row above the glyphs: `]` is x 7, `[` x 8.
    assert_eq!(
        text(&frame, 4, 15, 8, 3),
        [".#.....#", "#..##.#.", ".#.##..#"]
    );
    // And the row below them.
    assert_eq!(
        text(&frame, 4, 31, 8, 3),
        [".#.##..#", "#.....#.", ".#.#.#.#"]
    );
    // Without the edge, only the grey.
    let mut plain = b.clone();
    plain.print_fg(0, 1, "][", c(Ink::Black));
    assert_eq!(
        text(&draw(&plain), 4, 15, 8, 3),
        [".#.#.#.#", "#.###.#.", ".#.###.#"]
    );
    // An X's edge: the block and a ring round it.
    let mut b = buf(1, 1);
    b.fill_rect(
        b.bounds(),
        Cell::new('X', c(Ink::BlackEdged), c(Ink::Black)),
    );
    assert_eq!(
        text(&draw(&b), 0, 4, 8, 6),
        [
            "########", "##....##", "##.##.##", "##.##.##", "##....##", "########"
        ]
    );
}

/// The edge fills a one-pixel gap inside a glyph too.
#[test]
fn the_white_edge_fills_a_gap_between_two_pixels() {
    let mut b = buf(1, 1);
    b.fill_rect(
        b.bounds(),
        Cell::new('"', c(Ink::BlackEdged), c(Ink::Black)),
    );
    assert_eq!(
        text(&draw(&b), 0, 7, 8, 3),
        ["#.....##", "#.#.#.##", "#.....##"]
    );
}

#[test]
fn a_glyph_can_be_drawn_in_a_pattern() {
    let mut b = buf(1, 1);
    b.print(0, 0, "[", c(Ink::Grey), c(Ink::White));
    let frame = draw(&b);
    assert_eq!(text(&frame, 0, 0, 2, 4), ["#.", "..", "#.", ".."]);
}

#[test]
fn a_missing_glyph_is_the_fallback_and_a_space_is_nothing() {
    let mut b = buf(2, 1);
    b.print(0, 0, "é ", c(Ink::Black), c(Ink::White));
    let frame = draw(&b);
    assert_eq!(
        text(&frame, 0, 0, 16, 2),
        ["#......#........", "................"]
    );
    // A font without the fallback draws nothing for a glyph it lacks.
    let atlas = FontAtlasDef {
        glyphs: [('X', 0)].into(),
        ..test_font().0
    };
    let bits = test_font().1;
    let font = Font::new(&atlas, &bits).unwrap();
    let mut frame = Box::new([0u8; FRAME_BYTES]);
    render(&b, &font, &(), &mut frame);
    assert_eq!(text(&frame, 0, 0, 8, 1), ["........"]);
}

/// Cells, then `Under` items, then glyphs, then `Over` items; inverts
/// last of all.
#[test]
fn items_draw_under_and_over_the_glyphs() {
    let mut b = buf(1, 1);
    b.print(0, 0, "[", c(Ink::White), c(Ink::White));
    // Under: black behind the white glyph, which shows over it.
    b.add_overlay(Overlay::new(
        Rect::new(0, 0, 4, 2),
        c(Ink::Black),
        Layer::Under,
    ));
    // Over: black covering the glyph's second row.
    b.add_overlay(Overlay::new(
        Rect::new(0, 1, 1, 1),
        c(Ink::Black),
        Layer::Over,
    ));
    let frame = draw(&b);
    assert_eq!(text(&frame, 0, 0, 5, 3), [".###.", "####.", "....."]);
}

#[test]
fn an_invert_rectangle_flips_white_grey_and_black() {
    let mut b = buf(3, 1);
    b.set(1, 0, Cell::new(' ', c(Ink::Black), c(Ink::Grey)));
    b.set(2, 0, Cell::new('X', c(Ink::White), c(Ink::Black)));
    let invert = Overlay::new(Rect::new(4, 0, 18, 8), c(Ink::Invert), Layer::Under);
    b.add_overlay(invert);
    // Drawn after the invert, it is still flipped.
    b.add_overlay(Overlay::new(
        Rect::new(0, 0, 6, 1),
        c(Ink::Black),
        Layer::Over,
    ));
    let frame = draw(&b);
    assert_eq!(
        text(&frame, 0, 0, 24, 8),
        [
            "####..##.#.#.#.#......##",
            "....#####.#.#.#.......##",
            "....####.#.#.#.#......##",
            "....#####.#.#.#.......##",
            "....####.#.#.#.#......##",
            "....#####.#.#.#.......##",
            "....####.#.#.#.#...##.##",
            "....#####.#.#.#....##.##",
        ]
    );
}

/// A test picture 4 × 2: white, black, white, black over black, its
/// bottom row see-through but for its first pixel.
const PICTURE: [u8; 2] = [0b1010_0000, 0];
const PICTURE_MASK: [u8; 2] = [0b1111_0000, 0b1000_0000];

/// A made-up image's id.
fn picture_id() -> ImageId {
    let size = shuyi_assets::image::ImageInfo {
        width: 4,
        height: 2,
    };
    let table = shuyi_assets::image::ImageTable {
        images: [("p.png", size)].into(),
    };
    table.id("p.png").unwrap()
}

/// The test picture as [`picture_id`], or no picture at all if `none`.
struct TestImages {
    none: bool,
}

impl Images for TestImages {
    fn image(&self, id: ImageId) -> Option<Image<'_>> {
        (!self.none && id == picture_id())
            .then(|| Image::new(4, 2, &PICTURE, Some(&PICTURE_MASK)))
            .flatten()
    }
}

/// The test picture twice its size at the top-left of a grey cell, as
/// `edit` makes it.
fn sprite_frame(edit: impl Fn(Sprite) -> Sprite, none: bool) -> Vec<String> {
    let mut b = buf(1, 1);
    b.fill_rect(b.bounds(), Cell::new(' ', c(Ink::Black), c(Ink::Grey)));
    let sprite = Sprite::new(
        picture_id(),
        Rect::new(0, 0, 4, 2),
        Rect::new(0, 0, 8, 4),
        Layer::Over,
    );
    b.add_sprite(edit(sprite));
    text(&draw_with(&b, &TestImages { none }), 0, 0, 8, 4)
}

#[test]
fn a_sprite_draws_its_one_bit_picture() {
    let grey = ["#.#.#.#.", ".#.#.#.#", "#.#.#.#.", ".#.#.#.#"];
    // Each picture pixel 2 × 2; see-through pixels show the grey.
    assert_eq!(
        sprite_frame(|s| s, false),
        ["..##..##", "..##..##", "###.#.#.", "##.#.#.#"]
    );
    assert_eq!(
        sprite_frame(|s| Sprite { flip_x: true, ..s }, false),
        ["##..##..", "##..##..", "#.#.#.##", ".#.#.###"]
    );
    // Only its clip.
    let left = |s: Sprite| Sprite {
        clip: Rect::new(0, 0, 3, 4),
        ..s
    };
    assert_eq!(
        sprite_frame(left, false),
        ["..#.#.#.", "..##.#.#", "###.#.#.", "##.#.#.#"]
    );
    // A silhouette in one ink, a pattern fixed to the screen.
    let solid = |ink: Ink| move |s: Sprite| s.painted(Paint::Solid(c(ink)));
    assert_eq!(
        sprite_frame(solid(Ink::Black), false),
        ["########", "########", "###.#.#.", "##.#.#.#"]
    );
    assert_eq!(
        sprite_frame(solid(Ink::Dots), false),
        ["#...#...", "........", "..#.#.#.", "...#.#.#"]
    );
    // Dimmed: every other pixel white. Shaded: as it is.
    assert_eq!(
        sprite_frame(|s| s.painted(Paint::Dimmed), false),
        ["..#...#.", "...#...#", "#.#.#.#.", ".#.#.#.#"]
    );
    let dusk = shuyi_assets::lighting::Light::default();
    assert_eq!(
        sprite_frame(|s| s.painted(Paint::Shaded(dusk)), false),
        sprite_frame(|s| s, false)
    );
    // Darkened to black (game ADR-0075): where it shows, black, as a black
    // silhouette is; at full brightness, as it is.
    let dark = |brightness| move |s: Sprite| Sprite { brightness, ..s };
    assert_eq!(
        sprite_frame(dark(0), false),
        sprite_frame(solid(Ink::Black), false)
    );
    assert_eq!(sprite_frame(dark(255), false), sprite_frame(|s| s, false));
    // Faded out: none of it; with no picture, a grey rectangle.
    let faded = |opacity| move |s: Sprite| Sprite { opacity, ..s };
    assert_eq!(sprite_frame(faded(0), false), grey);
    let mut b = buf(1, 1);
    b.fill_rect(b.bounds(), Cell::new(' ', c(Ink::Black), c(Ink::White)));
    b.add_sprite(Sprite::new(
        picture_id(),
        Rect::new(0, 0, 4, 2),
        Rect::new(1, 1, 3, 2),
        Layer::Under,
    ));
    let frame = draw_with(&b, &TestImages { none: true });
    assert_eq!(
        text(&frame, 0, 0, 5, 4),
        [".....", ".#.#.", "..#..", "....."]
    );
}

/// Any image's id: [`One`] gives every id its picture.
struct One<'a>(Image<'a>);

impl Images for One<'_> {
    fn image(&self, _: ImageId) -> Option<Image<'_>> {
        Some(self.0)
    }
}

/// `image` drawn as an image of `source` size would be: `src` of it
/// stretched to `dest`, over a grey cell.
fn drawn(image: Image<'_>, src: Rect, dest: Rect, flip_x: bool) -> Vec<String> {
    let mut b = buf(1, 1);
    b.fill_rect(b.bounds(), Cell::new(' ', c(Ink::Black), c(Ink::Grey)));
    let sprite = Sprite::new(picture_id(), src, dest, Layer::Over);
    b.add_sprite(Sprite { flip_x, ..sprite });
    text(&draw_with(&b, &One(image)), 0, 0, 8, 4)
}

/// An 8 × 4 picture made from a 4 × 2 image enlarged twice before it was
/// dithered (game ADR-0085): drawn at the image's double size, each of its
/// pixels is one screen pixel, not a doubled one.
#[test]
fn an_enlarged_picture_draws_a_screen_pixel_a_pixel() {
    const BIG: [u8; 4] = [0b1100_0000, 0b0110_0000, 0b0011_0000, 0b0001_1000];
    let big = Image::new(8, 4, &BIG, None).unwrap().with_source(4, 2);
    let whole = Rect::new(0, 0, 4, 2);
    let rows = ["..######", "#..#####", "##..####", "###..###"];
    assert_eq!(drawn(big, whole, Rect::new(0, 0, 8, 4), false), rows);
    let flipped = ["######..", "#####..#", "####..##", "###..###"];
    assert_eq!(drawn(big, whole, Rect::new(0, 0, 8, 4), true), flipped);
    // The image's right half: the picture's right half, stretched.
    let right = drawn(big, Rect::new(2, 0, 2, 2), Rect::new(0, 0, 8, 4), false);
    assert_eq!(right, ["########", "########", "########", "..######"]);
    // At the image's own size: every other pixel of the picture, on the
    // grey cell.
    let small = drawn(big, whole, Rect::new(0, 0, 4, 2), false);
    assert_eq!(small, [".####.#.", "#.##.#.#", "#.#.#.#.", ".#.#.#.#"]);
    // A side of 0 leaves the source size as it was.
    assert_eq!(big.with_source(0, 0), big);
    assert_eq!(
        Image::new(8, 4, &BIG, None)
            .unwrap()
            .with_source(0, 0)
            .source_width,
        8
    );
}

/// With the picture its source's size, the sampling is plain nearest
/// sampling of the source (1032's).
#[test]
fn a_picture_its_sources_size_samples_as_before() {
    for start in 0..3 {
        for len in 1..6 {
            for dest in 1..9 {
                for d in 0..dest {
                    let plain = start + (2 * d + 1) * len / (2 * dest);
                    for size in [1u16, 7, 40] {
                        assert_eq!(Image::sample(d, start, len, dest, size, size), plain);
                    }
                }
            }
        }
    }
    // A source side of 0 is taken as 1, never a division by nothing.
    assert_eq!(Image::sample(0, 0, 1, 1, 4, 0), 2);
}

/// A picture the converter made (`shuyi_assets::bitmap::convert`), read
/// from its file, draws pixel for pixel as converted, see-through where
/// the image was. The converter is `std`-only (ADR-0008 §5), so this test
/// is too.
#[cfg(feature = "std")]
#[test]
fn a_converted_picture_draws_as_converted() {
    use shuyi_assets::bitmap::Bitmap;
    use shuyi_assets::bitmap::convert::{Method, Rgba, convert};
    let (black, white, clear) = ([0, 0, 0, 255], [255; 4], [0; 4]);
    let pixels = [white, black, clear, white, black, white, white, black];
    let rgba: alloc::vec::Vec<u8> = pixels.concat();
    let image = Rgba::new(4, 2, &rgba).unwrap();
    let file = convert(&image, (2, 1), &[Method::BAYER, Method::BAYER]).unwrap();
    let bitmap = Bitmap::parse(&file).unwrap();
    let picture = Image::from_bitmap(&bitmap, 1).unwrap();
    assert_eq!(Image::from_bitmap(&bitmap, 2), None);
    let frame = drawn(picture, Rect::new(0, 0, 4, 2), Rect::new(0, 0, 8, 4), false);
    for (y, row) in (0u16..).zip(&frame) {
        for (x, pixel) in (0u16..).zip(row.chars()) {
            let grey = (x + y) % 2 == 1;
            let expected = bitmap.pixel(1, x, y).unwrap_or(grey);
            assert_eq!(pixel == '.', expected, "({x}, {y})");
        }
    }
    // The see-through pixels (columns 4 and 5 of the top rows) show the
    // grey cell under the picture.
    assert_eq!(frame, ["..###...", "..##.#..", "##....##", "##....##"]);
}

/// A sprite away from the buffer's corner: its picture starts at its own
/// top-left.
#[test]
fn a_sprite_draws_where_it_is_placed() {
    let mut b = buf(1, 1);
    b.add_sprite(Sprite::new(
        picture_id(),
        Rect::new(0, 0, 4, 2),
        Rect::new(3, 2, 4, 2),
        Layer::Over,
    ));
    let frame = draw_with(&b, &TestImages { none: false });
    assert_eq!(
        text(&frame, 2, 1, 6, 4),
        ["......", "..#.#.", ".#....", "......"]
    );
}

/// A sprite at a scale that isn't whole takes the picture pixel under
/// each screen pixel's centre; and only its `src`.
#[test]
fn a_sprite_samples_pixel_centres_and_its_src() {
    let draw_one = |src: Rect, dest: Rect| {
        let mut b = buf(1, 1);
        b.add_sprite(Sprite::new(picture_id(), src, dest, Layer::Over));
        let frame = draw_with(&b, &TestImages { none: false });
        text(&frame, 0, 0, 6, 3)
    };
    // 4 × 2 at 1.5 ×: picture columns 0, 1, 1, 2, 3, 3 and rows 0, 1, 1.
    assert_eq!(
        draw_one(Rect::new(0, 0, 4, 2), Rect::new(0, 0, 6, 3)),
        [".##.##", "#.....", "#....."]
    );
    // Picture pixels 1 and 2 of the top row: black, white.
    assert_eq!(
        draw_one(Rect::new(1, 0, 2, 1), Rect::new(0, 0, 2, 1)),
        ["#.....", "......", "......"]
    );
    // The bottom row's first pixel, alone.
    assert_eq!(
        draw_one(Rect::new(0, 1, 1, 1), Rect::new(0, 0, 1, 1)),
        ["#.....", "......", "......"]
    );
}

/// A sprite fading out loses its pixels in a Bayer 4 × 4 order.
#[test]
fn a_fading_sprite_dithers() {
    let count = |opacity: u8| {
        (0..4)
            .flat_map(|y| (0..4).map(move |x| (x, y)))
            .filter(|&(x, y)| shows(opacity, x, y))
            .count()
    };
    assert_eq!(count(255), 16);
    assert_eq!(count(0), 0);
    assert_eq!(count(128), 8);
    assert_eq!(count(127), 8);
    assert_eq!(count(112), 7);
    // The last rank, 15, shows from 248: 247 × 16 is just 15 × 255 + 127.
    assert_eq!(count(247), 15);
    assert_eq!(count(248), 16);
    assert_eq!(count(64), 4);
    assert!(!shows(1, 0, 0) && shows(8, 0, 0));
    // The ranks are 0 to 15, each once.
    let mut ranks: Vec<u32> = BAYER.iter().flatten().copied().collect();
    ranks.sort_unstable();
    assert_eq!(ranks, (0..16).collect::<Vec<u32>>());
}

#[test]
fn image_sizes_are_checked() {
    assert_eq!(Image::stride(0), 0);
    assert_eq!(Image::stride(1), 1);
    assert_eq!(Image::stride(8), 1);
    assert_eq!(Image::stride(9), 2);
    assert!(Image::new(9, 2, &[0; 4], None).is_some());
    assert!(Image::new(9, 2, &[0; 3], None).is_none());
    assert!(Image::new(9, 2, &[0; 4], Some(&[0; 3])).is_none());
    assert!(Image::new(9, 2, &[0; 4], Some(&[0; 4])).is_some());
    let image = Image::new(9, 2, &[0x80, 0x80, 0, 0], None).unwrap();
    assert_eq!(image.pixel(0, 0), Some(true));
    assert_eq!(image.pixel(8, 0), Some(true));
    assert_eq!(image.pixel(1, 0), Some(false));
    assert_eq!(image.pixel(0, 1), Some(false));
    assert_eq!(image.pixel(9, 0), None);
    assert_eq!(image.pixel(-1, 0), None);
    assert_eq!(image.pixel(0, 2), None);
    assert_eq!(image.pixel(0, -1), None);
}

#[test]
fn a_font_must_fit_the_one_bit_atlas() {
    let (atlas, bits) = test_font();
    assert!(Font::new(&atlas, &bits).is_ok());
    assert!(Font::new(&atlas, &bits[..79]).is_err());
    let wide = FontAtlasDef {
        cell_w: 9,
        ..atlas.clone()
    };
    assert!(Font::new(&wide, &bits).is_err());
    let none = FontAtlasDef::default();
    assert!(Font::new(&none, &[]).is_ok());
}

/// A backdrop's scene shows through the see-through cells of its window,
/// at its zoom; solid cells cover it.
#[test]
fn a_backdrop_shows_behind_see_through_cells() {
    let mut scene = buf(2, 1);
    scene.print(0, 0, "X", c(Ink::Black), c(Ink::White));
    let scene = Rc::new(scene);
    let mut b = buf(3, 2);
    b.fill_rect(Rect::new(0, 0, 2, 2), Cell::see_through(' ', c(Ink::Black)));
    b.set_backdrop(Rc::clone(&scene), Rect::new(0, 0, 3, 2), (0.0, 0.0), 2);
    // The X is 4 × 4 at (6, 12); the solid third column covers the scene.
    let frame = draw(&b);
    assert_eq!(
        text(&frame, 4, 10, 6, 8),
        [
            "......", "......", "..####", "..####", "..####", "..####", "......", "......"
        ]
    );
    assert!(is_white(&frame, 16, 12));
    // Cut to the first cell: the X's right half is gone.
    b.set_backdrop(Rc::clone(&scene), Rect::new(0, 0, 1, 1), (0.0, 0.0), 2);
    assert_eq!(text(&draw(&b), 4, 12, 8, 1), ["..##...."]);
    // Panned by 3 scene pixels.
    b.set_backdrop(scene, Rect::new(0, 0, 1, 1), (3.0, 0.0), 2);
    let frame = draw(&b);
    assert_eq!(
        text(&frame, 0, 11, 8, 6)[1..],
        ["####....", "####....", "####....", "####....", "........"]
    );
    assert_eq!(text(&frame, 0, 11, 8, 1), ["........"]);
    // A window lower down, the scene panned down a scene pixel: the X's
    // rows 6 and 7 are 5 and 6 below the window's top, at zoom 2.
    b.set_backdrop(
        Rc::clone(&scene_of(&b)),
        Rect::new(0, 1, 1, 1),
        (0.0, 1.0),
        2,
    );
    assert_eq!(
        text(&draw(&b), 4, 25, 4, 6),
        ["....", "..##", "..##", "..##", "..##", "...."]
    );
}

/// The scene of `b`'s backdrop.
fn scene_of(b: &GlyphBuffer) -> Rc<GlyphBuffer> {
    Rc::new(b.backdrop().unwrap().scene().clone())
}

/// A scene's cells, glyphs and sprites away from its corner, in a window
/// away from the screen's, all at zoom 2.
#[test]
fn a_zoomed_scene_draws_everything_in_place() {
    let mut scene = buf(2, 2);
    scene.print(0, 0, "[", c(Ink::Black), c(Ink::White));
    scene.set(1, 1, Cell::new(' ', c(Ink::Black), c(Ink::Black)));
    scene.add_sprite(Sprite::new(
        picture_id(),
        Rect::new(0, 0, 4, 2),
        Rect::new(8, 0, 4, 2),
        Layer::Over,
    ));
    let mut b = buf(5, 6);
    b.fill_rect(b.bounds(), Cell::see_through(' ', c(Ink::Black)));
    // The window's top-left is screen pixel (8, 16).
    b.set_backdrop(Rc::new(scene), Rect::new(1, 1, 4, 5), (0.0, 0.0), 2);
    let frame = draw_with(&b, &TestImages { none: false });
    let black = |x, y| !is_white(&frame, x, y);
    // `[`: two pixels wide, all 32 rows tall.
    assert!(black(8, 16) && black(9, 47));
    assert!(!black(10, 30) && !black(8, 48) && !black(7, 30));
    // Scene cell (1, 1): screen 24..40 × 48..80.
    assert!(black(24, 48) && black(39, 79));
    assert!(!black(23, 60) && !black(24, 47) && !black(40, 60) && !black(30, 80));
    // The picture at scene pixel (8, 0), each pixel 2 × 2.
    assert_eq!(
        text(&frame, 24, 16, 8, 5),
        ["..##..##", "..##..##", "##......", "##......", "........"]
    );
}

#[test]
fn what_lies_past_the_screen_is_cut_off() {
    let mut b = GlyphBuffer::with_spec(
        ConsoleSpec::text(100, 32),
        Cell::new(' ', c(Ink::Black), c(Ink::Black)),
    );
    b.add_overlay(Overlay::new(
        Rect::new(396, 236, 10, 10),
        c(Ink::Invert),
        Layer::Over,
    ));
    let frame = draw(&b);
    assert_eq!(text(&frame, 394, 234, 6, 6)[5], "##....");
    assert!(!is_white(&frame, 0, 0));
    // The two unused bytes of each row stay white.
    assert!(frame.chunks(ROW_BYTES).all(|row| row[50..] == [u8::MAX; 2]));
    assert!(!is_white(&frame, 400, 0) && !is_white(&frame, 0, 240));
    assert!(!is_white(&frame, -1, 0) && !is_white(&frame, 0, -1));
}

/// A run of pixels not on a byte boundary, across bytes, clipped at both
/// ends.
#[test]
fn runs_cross_bytes_and_clip() {
    let mut frame = Box::new([u8::MAX; FRAME_BYTES]);
    let mut target = Target {
        frame: &mut frame,
        clip: Clip {
            x0: 5,
            y0: 1,
            x1: 20,
            y1: 3,
        },
    };
    let bits = 0b1011u64 << 60 | 1;
    target.run(1, 3, bits, Pen::Ink(Ink::Black));
    target.run(2, 3, u64::MAX, Pen::Flip);
    target.run(0, 3, u64::MAX, Pen::Ink(Ink::Black));
    target.run(3, 3, u64::MAX, Pen::Ink(Ink::Black));
    target.run(2, -60, u64::MAX, Pen::Flip);
    assert_eq!(
        text(&frame, 0, 0, 24, 4),
        [
            "........................",
            ".....##.................",
            ".....###############....",
            "........................"
        ]
    );
    assert_eq!(widen(0b1000_0001, 1), 0x81 << 56);
    assert_eq!(widen(0b1000_0001, 2), 0xc003 << 48);
    assert_eq!(widen(0b0100_0000, 4), 0x0f << 56);
}
