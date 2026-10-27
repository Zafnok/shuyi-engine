use alloc::collections::BTreeMap;
use alloc::rc::Rc;

use proptest::prelude::*;
use shuyi_assets::image::{ImageInfo, ImageTable};
use shuyi_assets::lighting::Light;

use super::*;
use crate::frame::{Cell, Overlay, Rect};

/// The test card's path in the image table.
const TEST_CARD_PATH: &str = "images/test_card.png";

/// The console background in these tests.
const CLEAR: Rgb = Rgb::new(16, 16, 16);

/// A 24×16 atlas of three 8×16 cells: ' ', then 'A' (a solid 4×4 block
/// in its top-left corner) and the fallback '?' (its left column), as
/// `frame-png`'s tests have it.
fn tiny_atlas() -> (FontAtlasDef, Image) {
    let def = FontAtlasDef {
        cell_w: 8,
        cell_h: 16,
        columns: 3,
        glyphs: BTreeMap::from([(' ', 0), ('A', 1), (FALLBACK_GLYPH, 2)]),
    };
    let mut rgba = vec![0; 24 * 16 * 4];
    let mut set = |x: usize, y: usize| rgba[(y * 24 + x) * 4..][..4].copy_from_slice(&[255; 4]);
    for y in 0..4 {
        for x in 8..12 {
            set(x, y);
        }
    }
    for y in 0..16 {
        set(16, y);
    }
    let image = Image {
        width: 24,
        height: 16,
        rgba,
    };
    (def, image)
}

/// A painter with the tiny atlas and the test card, if `with_card`, and
/// the table that names the card.
fn painter(with_card: bool) -> (Painter, ImageTable) {
    let (atlas, atlas_image) = tiny_atlas();
    let size = ImageInfo::new(test_card::SIZE, test_card::SIZE);
    let table = ImageTable::new([(TEST_CARD_PATH, size)].into());
    let card = Image {
        width: test_card::SIZE,
        height: test_card::SIZE,
        rgba: test_card::pixels(),
    };
    let id = table.id(TEST_CARD_PATH).unwrap();
    let images = if with_card {
        BTreeMap::from([(id, card)])
    } else {
        BTreeMap::new()
    };
    (Painter::new(atlas, atlas_image, images, CLEAR), table)
}

/// 2 × 1 cells: the test card over both, `Under`, and a red `Over`
/// rectangle at `(x, 0)`, 2 × 2.
fn card_frame(table: &ImageTable, x: i32) -> GlyphBuffer {
    let mut buf = GlyphBuffer::new(2, 1, Cell::new(' ', CLEAR, CLEAR));
    let card = table.id(TEST_CARD_PATH).unwrap();
    let full = Rect::new(0, 0, 16, 16);
    buf.add_sprite(Sprite::new(card, full, full, Layer::Under));
    let red = Overlay::new(Rect::new(x, 0, 2, 2), Rgb::new(255, 0, 0), Layer::Over);
    buf.add_overlay(red);
    buf
}

#[test]
fn a_rectangle_over_the_test_card_at_scale_2() {
    let (painter, table) = painter(true);
    let image = painter.render(&card_frame(&table, 0), 2);
    assert_eq!((image.width, image.height), (32, 32));
    let [red, green, blue, yellow] = test_card::QUADRANTS;
    for ((x, y), want) in [
        // The rectangle, over the card's border.
        ((0, 0), [255, 0, 0, 255]),
        ((3, 3), [255, 0, 0, 255]),
        // The border beside it, and each quadrant, each console pixel 2 × 2.
        ((4, 0), test_card::BORDER),
        ((4, 4), red),
        ((15, 15), red),
        ((16, 2), green),
        ((2, 16), blue),
        ((29, 29), yellow),
        ((31, 31), test_card::BORDER),
    ] {
        assert_eq!(image.pixel(x, y), Some(want), "({x}, {y})");
    }
}

#[test]
fn a_missing_glyph_and_a_missing_image_draw_in_the_missing_colour() {
    let (painter, table) = painter(false);
    let mut buf = card_frame(&table, 20);
    buf.set(1, 0, Cell::new('Z', Rgb::new(1, 2, 3), CLEAR));
    let image = painter.render(&buf, 1);
    let Rgb { r, g, b } = MISSING_COLOR;
    // The card isn't there, so its rectangle is; the fallback glyph's
    // column (cell 1's left) is in it too.
    assert_eq!(image.pixel(5, 5), Some([r, g, b, 255]));
    assert_eq!(image.pixel(8, 7), Some([r, g, b, 255]));
}

/// 3×2 cells: an 'A' on a blue cell, a white one on a green cell below
/// right, a glyph the atlas lacks, a green
/// `Under` line through the 'A', a red `Over` dot on it, a white `Over`
/// square, and the test card, flipped, half-opaque and clipped to its
/// middle-top 8×8, over the bottom row.
fn hand_checked_frame(table: &ImageTable) -> GlyphBuffer {
    let mut buf = GlyphBuffer::new(3, 2, Cell::new(' ', CLEAR, CLEAR));
    buf.set(
        0,
        0,
        Cell::new('A', Rgb::new(200, 100, 50), Rgb::new(0, 0, 128)),
    );
    buf.set(1, 0, Cell::new('Z', Rgb::new(1, 2, 3), CLEAR));
    buf.set(
        2,
        1,
        Cell::new('A', Rgb::new(255, 255, 255), Rgb::new(0, 128, 0)),
    );
    let under = Overlay::new(Rect::new(0, 2, 8, 1), Rgb::new(0, 255, 0), Layer::Under);
    buf.add_overlay(under);
    buf.add_overlay(Overlay::new(
        Rect::new(3, 3, 1, 1),
        Rgb::new(255, 0, 0),
        Layer::Over,
    ));
    buf.add_overlay(Overlay::new(
        Rect::new(20, 0, 2, 2),
        Rgb::new(255, 255, 255),
        Layer::Over,
    ));
    let card = table.id(TEST_CARD_PATH).unwrap();
    let dest = Rect::new(0, 16, 16, 16);
    let mut sprite = Sprite::new(card, Rect::new(0, 0, 16, 16), dest, Layer::Over);
    sprite.clip = Rect::new(4, 16, 8, 8);
    sprite.flip_x = true;
    sprite.opacity = 128;
    buf.add_sprite(sprite);
    buf
}

/// Pixels of the hand-checked frame at scale 1, worked out by hand.
const EXPECTED: [((u32, u32), [u8; 3]); 17] = [
    // The 'A': its block in fg, the rest of the cell its bg.
    ((0, 0), [200, 100, 50]),
    ((2, 1), [200, 100, 50]),
    ((5, 5), [0, 0, 128]),
    ((16, 16), [255, 255, 255]),
    ((21, 20), [0, 128, 0]),
    // The `Under` line shows only beside the glyph.
    ((0, 2), [200, 100, 50]),
    ((5, 2), [0, 255, 0]),
    // The `Over` dot covers the glyph.
    ((3, 3), [255, 0, 0]),
    ((20, 1), [255, 255, 255]),
    // The missing 'Z' draws the fallback glyph in magenta.
    ((8, 7), [255, 0, 255]),
    ((9, 7), [16, 16, 16]),
    // The sprite, flipped: its left shows the card's right (green), its
    // right the card's left (red); 128/255 over the clear colour.
    ((4, 17), [34, 93, 42]),
    ((11, 17), [110, 34, 34]),
    ((6, 16), [136, 136, 136]),
    // Clipped: nothing beside or below the clip.
    ((3, 17), [16, 16, 16]),
    ((12, 17), [16, 16, 16]),
    ((6, 24), [16, 16, 16]),
];

#[test]
fn matches_a_hand_checked_frame() {
    let (painter, table) = painter(true);
    let buf = hand_checked_frame(&table);
    for scale in [1, 2] {
        let image = painter.render(&buf, scale);
        assert_eq!((image.width, image.height), (24 * scale, 32 * scale));
        for ((x, y), [r, g, b]) in EXPECTED {
            // Every pixel of the console pixel's square.
            for (dx, dy) in [(0, 0), (scale - 1, scale - 1)] {
                let at = (x * scale + dx, y * scale + dy);
                assert_eq!(
                    image.pixel(at.0, at.1),
                    Some([r, g, b, 255]),
                    "({x}, {y}) at scale {scale}"
                );
            }
        }
    }
}

#[test]
fn an_unflipped_solid_sprite_shows_the_card_as_is() {
    let (painter, table) = painter(true);
    let mut buf = GlyphBuffer::new(2, 1, Cell::new(' ', CLEAR, CLEAR));
    let card = table.id(TEST_CARD_PATH).unwrap();
    let full = Rect::new(0, 0, 16, 16);
    buf.add_sprite(Sprite::new(card, full, full, Layer::Under));
    let image = painter.render(&buf, 1);
    for y in 0..16 {
        for x in 0..16 {
            let want = test_card::pixels()[((y * 16 + x) * 4) as usize..][..4].to_vec();
            assert_eq!(image.pixel(x, y).unwrap().to_vec(), want, "({x}, {y})");
        }
    }
}

#[test]
fn a_painted_sprite_is_one_colour_or_dimmed_where_the_image_is_solid() {
    let (painter, table) = painter(true);
    let card = table.id(TEST_CARD_PATH).unwrap();
    let full = Rect::new(0, 0, 16, 16);
    let render = |paint, opacity| {
        let mut buf = GlyphBuffer::new(2, 1, Cell::new(' ', CLEAR, CLEAR));
        let mut sprite = Sprite::new(card, full, full, Layer::Over).painted(paint);
        sprite.opacity = opacity;
        buf.add_sprite(sprite);
        painter.render(&buf, 1)
    };
    // The card: a white border, then a red quadrant from (1, 1).
    let [red, ..] = test_card::QUADRANTS;
    let red = Rgb::new(red[0], red[1], red[2]);
    let white = Rgb::new(255, 255, 255);
    let rgba = |c: Rgb| Some([c.r, c.g, c.b, 255]);
    // One colour everywhere, whatever the image's.
    let blue = Rgb::new(10, 20, 200);
    let solid = render(Paint::Solid(blue), 255);
    assert_eq!(solid.pixel(0, 0), rgba(blue));
    assert_eq!(solid.pixel(1, 1), rgba(blue));
    assert_eq!(solid.pixel(14, 14), rgba(blue));
    // Half see-through: half way from the clear colour (16) to it.
    let half = render(Paint::Solid(blue), 128);
    assert_eq!(half.pixel(1, 1), Some([13, 18, 108, 255]));
    // Dimmed: each pixel grey and darker, by its own colour.
    let dimmed = render(Paint::Dimmed, 255);
    assert_eq!(dimmed.pixel(0, 0), rgba(white.dimmed()));
    assert_eq!(dimmed.pixel(1, 1), rgba(red.dimmed()));
    assert_ne!(red.dimmed(), red);
    // Shaded: each pixel as it is in the light (game ADR-0058).
    let dusk = Light::new(0.35, [0.62, 0.56, 0.46]);
    let shaded = render(Paint::Shaded(dusk), 255);
    assert_eq!(shaded.pixel(0, 0), Some([158, 143, 117, 255]));
    assert_eq!(shaded.pixel(1, 1), rgba(Paint::Shaded(dusk).apply(red)));
    assert_ne!(Paint::Shaded(dusk).apply(red), red);
    // The image's own colours, as before.
    let plain = render(Paint::Image, 255);
    assert_eq!(plain.pixel(1, 1), rgba(red));
}

/// A darkened sprite (game ADR-0075) is its colours times its brightness, after
/// its paint, and as solid as before: nothing under it shows through.
#[test]
fn a_darkened_sprite_is_darker_not_see_through() {
    let (painter, table) = painter(true);
    let card = table.id(TEST_CARD_PATH).unwrap();
    let full = Rect::new(0, 0, 16, 16);
    let render = |paint, brightness| {
        let mut buf = GlyphBuffer::new(2, 1, Cell::new(' ', CLEAR, CLEAR));
        let mut sprite = Sprite::new(card, full, full, Layer::Over).painted(paint);
        sprite.brightness = brightness;
        buf.add_sprite(sprite);
        painter.render(&buf, 1)
    };
    let [red, ..] = test_card::QUADRANTS;
    let red = Rgb::new(red[0], red[1], red[2]);
    let rgba = |c: Rgb| Some([c.r, c.g, c.b, 255]);
    let dark = render(Paint::Image, 102);
    assert_eq!(dark.pixel(1, 1), rgba(red.scale(0.4)));
    assert_eq!(dark.pixel(0, 0), rgba(Rgb::new(102, 102, 102)));
    let blue = Rgb::new(10, 20, 200);
    let solid = render(Paint::Solid(blue), 102);
    assert_eq!(solid.pixel(1, 1), rgba(blue.scale(0.4)));
    // Black where it shows; full brightness as it is.
    assert_eq!(render(Paint::Image, 0).pixel(1, 1), rgba(Rgb::new(0, 0, 0)));
    assert_eq!(render(Paint::Image, 255).pixel(1, 1), rgba(red));
}

const RED: Rgb = Rgb::new(200, 0, 0);
const BLUE: Rgb = Rgb::new(0, 0, 128);
const GREEN: Rgb = Rgb::new(0, 128, 0);
const WHITE: Rgb = Rgb::new(255, 255, 255);

/// A 6×3-cell console over a 2×1-cell scene (a red 'A' on blue, then a
/// green cell with a white `Over` dot at scene pixel (9, 1)). The scene
/// shows at 2× from scene pixel `origin` through the window of cells
/// (1, 1) to (4, 2): console pixels 8..40 × 16..48. In the window, cell
/// (2, 1) holds a white 'A' with no background and cell (4, 2) is solid,
/// in the clear colour.
fn backdrop_frame(origin: (f32, f32)) -> GlyphBuffer {
    let mut scene = GlyphBuffer::new(2, 1, Cell::new(' ', CLEAR, GREEN));
    scene.set(0, 0, Cell::new('A', RED, BLUE));
    scene.add_overlay(Overlay::new(Rect::new(9, 1, 1, 1), WHITE, Layer::Over));
    let mut buf = GlyphBuffer::new(6, 3, Cell::new(' ', CLEAR, CLEAR));
    let window = Rect::new(1, 1, 4, 2);
    buf.fill_rect(window, Cell::see_through(' ', CLEAR));
    buf.set_backdrop(Rc::new(scene), window, origin, 2);
    buf.print_fg(2, 1, "A", WHITE);
    buf.set(4, 2, Cell::new(' ', CLEAR, CLEAR));
    buf
}

/// Pixels of [`backdrop_frame`] from origin (2, 1) at scale 1, worked out
/// by hand: scene pixel (sx, sy) is the 2×2 square at
/// (8 + (sx − 2) × 2, 16 + (sy − 1) × 2).
const BACKDROP_EXPECTED: [((u32, u32), Rgb); 16] = [
    // The scene's 'A' block (scene x and y under 4), then its cell's bg.
    ((8, 16), RED),
    ((11, 16), RED),
    ((12, 16), BLUE),
    ((10, 45), BLUE),
    // The console's own 'A' over the scene, and the scene beside it.
    ((16, 16), WHITE),
    ((19, 19), WHITE),
    ((20, 16), GREEN),
    // The scene's dot, scaled with it.
    ((22, 16), WHITE),
    ((23, 17), WHITE),
    ((22, 18), GREEN),
    // Clipped to the window: nothing left of or above it.
    ((7, 16), CLEAR),
    ((8, 15), CLEAR),
    // Past the scene's right and bottom edges: the clear colour.
    ((35, 16), GREEN),
    ((36, 16), CLEAR),
    ((10, 46), CLEAR),
    // The solid cell hides the scene, though it is in the clear colour.
    ((33, 33), CLEAR),
];

#[test]
fn a_backdrop_is_scaled_shifted_and_clipped_behind_the_see_through_cells() {
    let (painter, _) = painter(false);
    let buf = backdrop_frame((2.0, 1.0));
    for scale in [1, 2, 3] {
        let image = painter.render(&buf, scale);
        assert_eq!((image.width, image.height), (48 * scale, 48 * scale));
        for ((x, y), Rgb { r, g, b }) in BACKDROP_EXPECTED {
            for (dx, dy) in [(0, 0), (scale - 1, scale - 1)] {
                let at = (x * scale + dx, y * scale + dy);
                assert_eq!(
                    image.pixel(at.0, at.1),
                    Some([r, g, b, 255]),
                    "({x}, {y}) at scale {scale}"
                );
            }
        }
        // Beside the solid cell the scene still shows.
        let Rgb { r, g, b } = GREEN;
        assert_eq!(image.pixel(31 * scale, 33 * scale), Some([r, g, b, 255]));
    }
}

#[test]
fn half_a_scene_pixel_pans_by_one_window_pixel() {
    let (painter, _) = painter(false);
    let at = |origin, x| painter.render(&backdrop_frame(origin), 1).pixel(x, 16);
    let px = |Rgb { r, g, b }: Rgb| Some([r, g, b, 255]);
    // From (2, 1) the block ends before window pixel 12; from (2.5, 1),
    // one pixel sooner; from (3, 1), two.
    assert_eq!((at((2.0, 1.0), 10), at((2.0, 1.0), 11)), (px(RED), px(RED)));
    assert_eq!(
        (at((2.5, 1.0), 10), at((2.5, 1.0), 11)),
        (px(RED), px(BLUE))
    );
    assert_eq!((at((3.0, 1.0), 9), at((3.0, 1.0), 10)), (px(RED), px(BLUE)));
    // At 2 × 2 a quarter of a scene pixel is a window pixel.
    let at2 = |origin, x| painter.render(&backdrop_frame(origin), 2).pixel(x, 32);
    assert_eq!(
        (at2((2.0, 1.0), 23), at2((2.0, 1.0), 24)),
        (px(RED), px(BLUE))
    );
    assert_eq!(
        (at2((2.25, 1.0), 22), at2((2.25, 1.0), 23)),
        (px(RED), px(BLUE))
    );
    // A window that starts before the scene shows the clear colour first.
    assert_eq!(at((-1.0, 1.0), 9), px(CLEAR));
    assert_eq!(at((-1.0, 1.0), 10), px(RED));
}

#[test]
fn a_one_bit_frame_becomes_a_black_and_white_picture() {
    let mut frame = Box::new([0u8; onebit::FRAME_BYTES]);
    frame[0] = 0b0100_0000;
    let picture = one_bit_picture(&frame, 2);
    assert_eq!((picture.width, picture.height), (800, 480));
    assert_eq!(picture.pixel(0, 0), Some([0, 0, 0, 255]));
    assert_eq!(picture.pixel(2, 1), Some([255; 4]));
    assert_eq!(picture.pixel(3, 1), Some([255; 4]));
    assert_eq!(picture.pixel(4, 0), Some([0, 0, 0, 255]));
    assert_eq!(picture.pixel(2, 2), Some([0, 0, 0, 255]));
}

#[test]
fn a_filled_picture_is_one_colour_and_pixels_off_it_are_none() {
    let image = Image::filled(3, 2, Rgb::new(1, 2, 3));
    assert_eq!(image.rgba.len(), 3 * 2 * 4);
    for (x, y) in [(0, 0), (2, 1), (1, 0)] {
        assert_eq!(image.pixel(x, y), Some([1, 2, 3, 255]));
    }
    assert_eq!(image.pixel(3, 0), None);
    assert_eq!(image.pixel(0, 2), None);
    assert_eq!(Image::filled(0, 0, CLEAR).rgba, Vec::<u8>::new());
}

#[test]
fn empty_pictures_and_pixels_off_the_picture_are_harmless() {
    for (width, height) in [(0, 0), (0, 3), (3, 0)] {
        let empty = Image {
            width,
            height,
            rgba: Vec::new(),
        };
        assert_eq!(empty.texel(1, 1), [0; 4]);
    }
    let mut image = Image {
        width: 2,
        height: 2,
        rgba: vec![7; 16],
    };
    for (x, y) in [(-1, 0), (0, -1), (2, 0), (0, 2)] {
        image.blend(x, y, [1.0; 3], 1.0);
    }
    assert_eq!(image.rgba, vec![7; 16]);
    // Texels clamp to the picture's edge.
    image.blend(1, 1, [1.0; 3], 1.0);
    assert_eq!(image.texel(5, 5), [255, 255, 255, 255]);
    assert_eq!(image.texel(-5, -5), [7; 4]);
    assert_eq!(image.pixel(2, 0), None);
}

/// A 3 × 2 picture of a few colours, with alpha 255.
fn picture() -> Image {
    let mut image = Image::filled(3, 2, Rgb::new(100, 150, 200));
    image.rgba[4..8].copy_from_slice(&[0, 255, 10, 255]);
    image
}

/// `image` with channel `c` of pixel `(x, y)` moved up by `by`.
fn nudged(image: &Image, (x, y): (u32, u32), c: usize, by: u8) -> Image {
    let mut out = image.clone();
    out.rgba[(y * image.width + x) as usize * 4 + c] += by;
    out
}

#[test]
fn a_picture_compared_with_itself_passes() {
    let image = picture();
    let same = compare(&image, &image, Tolerance::default());
    assert!(same.same_size);
    assert_eq!((same.over, same.worst, same.first), (0, 0, None));
    assert!(same.passes());
    // The diff is the reference, darker by half.
    assert_eq!(same.diff.pixel(0, 0), Some([50, 75, 100, 255]));
    assert_eq!(same.diff.pixel(1, 0), Some([0, 127, 5, 255]));
}

#[test]
fn one_pixel_past_the_channel_tolerance_is_over() {
    let reference = picture();
    let tolerance = Tolerance::default();
    let window = nudged(&reference, (2, 1), 1, tolerance.channel + 1);
    let found = compare(&window, &reference, tolerance);
    assert_eq!(
        (found.over, found.worst, found.first),
        (1, tolerance.channel + 1, Some((2, 1)))
    );
    let Rgb { r, g, b } = MISSING_COLOR;
    assert_eq!(found.diff.pixel(2, 1), Some([r, g, b, 255]));
    assert_eq!(found.diff.pixel(1, 1), Some([50, 75, 100, 255]));
    assert!(!found.passes());
    assert!(compare(&window, &reference, Tolerance::new(tolerance.channel, 1)).passes());
}

#[test]
fn exactly_the_channel_tolerance_and_alpha_alone_are_not_over() {
    let reference = picture();
    let tolerance = Tolerance::new(4, 0);
    let window = nudged(&reference, (0, 0), 2, 4);
    let found = compare(&window, &reference, tolerance);
    assert_eq!((found.over, found.worst), (0, 4));
    assert!(found.passes());
    let mut window = reference.clone();
    window.rgba[3] = 0;
    let found = compare(&window, &reference, tolerance);
    assert_eq!((found.over, found.worst), (0, 0));
    assert!(found.passes());
}

#[test]
fn pictures_of_different_sizes_never_pass() {
    let reference = picture();
    let wider = Image::filled(4, 2, Rgb::new(100, 150, 200));
    let found = compare(&wider, &reference, Tolerance::new(255, u32::MAX));
    assert!(!found.same_size);
    assert!(!found.passes());
    // The shared area is compared: only pixel (1, 0) differs.
    assert_eq!((found.diff.width, found.diff.height), (3, 2));
    assert_eq!(
        compare(&wider, &reference, Tolerance::default()).first,
        Some((1, 0))
    );
    assert!(
        !compare(
            &Image::filled(0, 0, CLEAR),
            &reference,
            Tolerance::default()
        )
        .passes()
    );
}

#[test]
fn a_tolerance_defaults_and_reads_from_ron() {
    assert_eq!(Tolerance::default(), Tolerance::new(2, 0));
    let read: Tolerance = ron::from_str("(channel: 4)").unwrap();
    assert_eq!(read, Tolerance::new(4, 0));
    let read: Tolerance = ron::from_str("(pixels: 3)").unwrap();
    assert_eq!(read, Tolerance::new(2, 3));
    assert!(ron::from_str::<Tolerance>("(channel: 4, colour: 1)").is_err());
}

/// Two pictures of the same size, `w` × `h`.
fn two_pictures() -> impl Strategy<Value = (Image, Image)> {
    (1_u32..5, 1_u32..5).prop_flat_map(|(w, h)| {
        let len = (w * h * 4) as usize;
        let pixels = proptest::collection::vec(any::<u8>(), len);
        (pixels.clone(), pixels).prop_map(move |(a, b)| {
            let image = |rgba| Image {
                width: w,
                height: h,
                rgba,
            };
            (image(a), image(b))
        })
    })
}

proptest! {
    #[test]
    fn a_comparison_is_the_same_both_ways(
        (a, b) in two_pictures(),
        channel in any::<u8>(),
    ) {
        let tolerance = Tolerance::new(channel, 0);
        let (there, back) = (compare(&a, &b, tolerance), compare(&b, &a, tolerance));
        prop_assert_eq!(
            (there.over, there.worst, there.first),
            (back.over, back.worst, back.first)
        );
    }

    #[test]
    fn rendering_twice_gives_the_same_picture(scale in 1_u32..4, x in -2_i32..18) {
        let (painter, table) = painter(true);
        let buf = card_frame(&table, x);
        prop_assert_eq!(painter.render(&buf, scale), painter.render(&buf, scale));
    }
}
