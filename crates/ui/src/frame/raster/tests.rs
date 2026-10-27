use alloc::collections::BTreeMap;

use proptest::prelude::*;
use shuyi_assets::image::{ImageInfo, ImageTable};

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

/// A painter with the tiny atlas and the test card, and the table that
/// names the card.
fn painter() -> (Painter, ImageTable) {
    let (atlas, atlas_image) = tiny_atlas();
    let size = ImageInfo::new(test_card::SIZE, test_card::SIZE);
    let table = ImageTable::new([(TEST_CARD_PATH, size)].into());
    let card = Image {
        width: test_card::SIZE,
        height: test_card::SIZE,
        rgba: test_card::pixels(),
    };
    let id = table.id(TEST_CARD_PATH).unwrap();
    let images = BTreeMap::from([(id, card)]);
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
    let (painter, table) = painter();
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
    let (atlas, atlas_image) = tiny_atlas();
    let painter = Painter::new(atlas, atlas_image, BTreeMap::new(), CLEAR);
    let (_, table) = self::painter();
    let mut buf = card_frame(&table, 20);
    buf.set(1, 0, Cell::new('Z', Rgb::new(1, 2, 3), CLEAR));
    let image = painter.render(&buf, 1);
    let Rgb { r, g, b } = MISSING_COLOR;
    // The card isn't there, so its rectangle is; the fallback glyph's
    // column (cell 1's left) is in it too.
    assert_eq!(image.pixel(5, 5), Some([r, g, b, 255]));
    assert_eq!(image.pixel(8, 7), Some([r, g, b, 255]));
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
        let (painter, table) = painter();
        let buf = card_frame(&table, x);
        prop_assert_eq!(painter.render(&buf, scale), painter.render(&buf, scale));
    }
}
