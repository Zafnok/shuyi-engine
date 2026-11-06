//! Painted pictures on the one-bit target (ADR-0078): a one-bit picture
//! drawn as a sprite, an RGBA or indexed one dithered through the 8 × 8
//! Bayer order at the screen's pixels. Pictures are text, a row a line:
//! `#` black, `.` white.

use alloc::rc::Rc;
use alloc::string::String;

use proptest::prelude::*;
use shuyi_assets::fnv::Hasher;
use shuyi_assets::ink::Ink;

use super::*;
use crate::frame::console::ConsoleSpec;
use crate::frame::glyph_buffer::{Cell, Layer, Rect};
use crate::frame::painted::{Painted, PaintedId, PaintedSprite};
use crate::prelude::*;

const ID: PaintedId = PaintedId::new("picture");

/// A Playdate frame of black cells with `picture` placed from `src` to
/// `dest`.
fn frame_of(picture: Option<Painted>, src: Rect, dest: Rect) -> GlyphBuffer {
    let black = Rgb::from(Ink::Black.code());
    let mut buf = GlyphBuffer::with_spec(ConsoleSpec::text(50, 15), Cell::new(' ', black, black));
    if let Some(picture) = picture {
        buf.pictures_mut().insert(ID, picture);
    }
    buf.add_painted(PaintedSprite::new(ID, src, dest, Layer::Over));
    buf
}

/// `buf` drawn by the fast renderer, checked equal to the reference's.
fn draw(buf: &GlyphBuffer) -> Box<Frame> {
    let atlas = shuyi_assets::font::FontAtlasDef {
        cell_w: 8,
        cell_h: 16,
        columns: 1,
        glyphs: alloc::collections::BTreeMap::default(),
        wide: alloc::collections::BTreeMap::default(),
        wide_sets: alloc::collections::BTreeMap::new(),
    };
    let font = Font::new(&atlas, &[]).unwrap();
    let mut fast = Box::new([0u8; FRAME_BYTES]);
    render(buf, &font, &(), &mut fast);
    let mut slow = Box::new([0u8; FRAME_BYTES]);
    reference::render(buf, &font, &(), &mut slow);
    assert!(fast == slow, "the fast renderer and the reference differ");
    fast
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

/// A `w × h` RGBA picture of grey `level`, opaque.
fn grey(w: u16, h: u16, level: u8) -> Painted {
    let mut picture = Painted::rgba(w, h);
    for pixel in picture.pixels_mut().as_chunks_mut::<4>().0 {
        pixel.copy_from_slice(&[level, level, level, u8::MAX]);
    }
    picture
}

#[test]
fn dither_row_follows_the_bayer_order_by_hand() {
    // Row 0 of the order is 0 32 8 40 2 34 10 42: at a brightness of 128
    // (a darkness of 127) a pixel is white where 4 × order + 2 ≥ 127,
    // so where the order is 32 or more.
    assert_eq!(dither_row([128; 8], 0), 0b0101_0101);
    // Row 1, 48 16 56 24 50 18 58 26.
    assert_eq!(dither_row([128; 8], 1), 0b1010_1010);
    // Black is black and white white on every row.
    for y in 0..8 {
        assert_eq!(dither_row([0; 8], y), 0);
        assert_eq!(dither_row([u8::MAX; 8], y), u8::MAX);
    }
    // Brightness 64 (darkness 191) needs an order of 48: none on row 0,
    // half of row 1.
    assert_eq!(dither_row([64; 8], 0), 0);
    assert_eq!(dither_row([64; 8], 1), 0b1010_1010);
    // Each pixel its own: white, black, white, black, 128 at order 2
    // (black), 128 at 34 (white), black, white.
    assert_eq!(
        dither_row([255, 0, 255, 0, 128, 128, 0, 255], 0),
        0b1010_0101
    );
    // The order repeats every 8 rows, below 0 too: row −1 is row 7, 63 31
    // 55 23 61 29 53 21.
    assert_eq!(dither_row([128; 8], 8), dither_row([128; 8], 0));
    assert_eq!(dither_row([128; 8], -1), 0b1010_1010);
}

#[test]
fn a_grey_picture_dithers_at_the_screens_pixels() {
    // 4 × 2 pixels at twice the size: 8 × 4 on the screen, each screen
    // pixel against the order at its own place.
    let one = |dest: Rect| frame_of(Some(grey(4, 2, 128)), Rect::new(0, 0, 4, 2), dest);
    let frame = draw(&one(Rect::new(0, 0, 8, 4)));
    assert_eq!(
        text(&frame, 0, 0, 8, 4),
        ["#.#.#.#.", ".#.#.#.#", "#.#.#.#.", ".#.#.#.#"]
    );
    // Moved one pixel right, the pattern stays where it was on the screen.
    let moved = draw(&one(Rect::new(1, 0, 8, 4)));
    assert_eq!(text(&moved, 1, 0, 7, 2), [".#.#.#.", "#.#.#.#"]);
}

/// A `w × h` RGBA picture, opaque, its grey rising to the right and
/// down: no two neighbours alike.
fn ramp(w: u16, h: u16) -> Painted {
    let mut picture = Painted::rgba(w, h);
    let pixels = picture.pixels_mut().as_chunks_mut::<4>().0;
    for (i, pixel) in (0u32..).zip(pixels) {
        let (x, y) = (i % u32::from(w), i / u32::from(w));
        let level = u8::try_from((x * 37 + y * 101) % 256).unwrap_or(0);
        *pixel = [level, level, level, u8::MAX];
    }
    picture
}

#[test]
fn a_picture_on_white_dithers_as_on_black() {
    let white = Rgb::from(Ink::White.code());
    let mut buf = GlyphBuffer::with_spec(ConsoleSpec::text(50, 15), Cell::new(' ', white, white));
    buf.pictures_mut().insert(ID, grey(4, 2, 128));
    let src = Rect::new(0, 0, 4, 2);
    buf.add_painted(PaintedSprite::new(
        ID,
        src,
        Rect::new(0, 0, 8, 4),
        Layer::Over,
    ));
    let on_black = draw(&frame_of(Some(grey(4, 2, 128)), src, Rect::new(0, 0, 8, 4)));
    assert_eq!(text(&draw(&buf), 0, 0, 8, 4), text(&on_black, 0, 0, 8, 4));
}

#[test]
fn a_source_past_the_pictures_edges_draws_nothing_there() {
    // Two pixels past the right edge, then two past the bottom: the black
    // cells show where there is no picture.
    for src in [Rect::new(0, 0, 6, 4), Rect::new(0, 0, 4, 6)] {
        let dest = Rect::new(0, 0, src.w * 2, src.h * 2);
        let frame = draw(&frame_of(Some(grey(4, 4, u8::MAX)), src, dest));
        assert_eq!(text(&frame, 0, 0, 8, 8), vec![".".repeat(8); 8]);
        let past = if src.w > 4 {
            text(&frame, 8, 0, 4, 8)
        } else {
            text(&frame, 0, 8, 8, 4)
        };
        assert!(past.iter().all(|row| !row.contains('.')), "{past:?}");
    }
}

#[test]
fn a_picture_in_a_zoomed_scene_away_from_the_corner_draws_as_the_reference() {
    let black = Rgb::from(Ink::Black.code());
    let ground = Cell::new(' ', black, black);
    let mut scene = GlyphBuffer::with_spec(ConsoleSpec::text(50, 15).sized(6, 4), ground);
    scene.pictures_mut().insert(ID, ramp(12, 9));
    let src = Rect::new(0, 0, 12, 9);
    scene.add_painted(PaintedSprite::new(
        ID,
        src,
        Rect::new(5, 3, 24, 27),
        Layer::Over,
    ));
    let mut buf = GlyphBuffer::with_spec(ConsoleSpec::text(50, 15), Cell::see_through(' ', black));
    buf.set_backdrop(Rc::new(scene), Rect::new(3, 2, 20, 8), (1.5, 2.5), 2);
    // `draw` holds the fast renderer to the reference; and something of
    // the picture shows.
    let frame = draw(&buf);
    assert!(text(&frame, 0, 0, 400, 240).concat().contains('.'));
}

#[test]
fn half_clear_pixels_go_and_brightness_darkens() {
    let mut picture = grey(2, 1, u8::MAX);
    // The second pixel just under half opaque: not drawn, the black cells
    // show.
    picture.pixels_mut()[7] = 127;
    let buf = frame_of(
        Some(picture.clone()),
        Rect::new(0, 0, 2, 1),
        Rect::new(0, 0, 16, 8),
    );
    assert_eq!(text(&draw(&buf), 6, 0, 4, 1), ["..##"]);
    picture.pixels_mut()[7] = 128;
    let mut buf = frame_of(Some(picture), Rect::new(0, 0, 2, 1), Rect::new(0, 0, 16, 8));
    assert_eq!(text(&draw(&buf), 6, 0, 4, 1), ["...."]);
    // At brightness 128, white is a 50 % grey.
    buf.fade(128.0 / 255.0);
    assert_eq!(text(&draw(&buf), 0, 0, 8, 2), ["#.#.#.#.", ".#.#.#.#"]);
}

#[test]
fn an_indexed_picture_dithers_its_colours_and_a_palette_change_shows() {
    let mut picture = Painted::indexed(2, 1, &[Rgb::new(0, 0, 0), Rgb::new(255, 255, 255)]);
    picture.pixels_mut()[1] = 1;
    let mut buf = frame_of(Some(picture), Rect::new(0, 0, 2, 1), Rect::new(0, 0, 8, 1));
    assert_eq!(text(&draw(&buf), 0, 0, 8, 1), ["####...."]);
    let picture = buf.pictures_mut().get_mut(ID).unwrap();
    picture.palette_mut()[0] = Rgb::new(255, 255, 255);
    assert_eq!(text(&draw(&buf), 0, 0, 8, 1), ["........"]);
}

#[test]
fn a_one_bit_picture_is_drawn_as_it_is_and_a_missing_one_grey() {
    let mut picture = Painted::one_bit(8, 1).with_mask();
    picture.pixels_mut()[0] = 0b1100_1100;
    picture.mask_mut().unwrap()[0] = 0b1111_0000;
    let mut buf = frame_of(Some(picture), Rect::new(0, 0, 8, 1), Rect::new(0, 0, 8, 1));
    // The half its mask leaves out shows the black cells under it.
    assert_eq!(text(&draw(&buf), 0, 0, 8, 1), ["..######"]);
    buf.pictures_mut().remove(ID);
    let grey = Shade::of_ink(Ink::Grey);
    let row: String = (0..8)
        .map(|x| if grey.is_white(x, 0) { '.' } else { '#' })
        .collect();
    assert_eq!(text(&draw(&buf), 0, 0, 8, 1), [row]);
}

#[test]
fn a_repainted_picture_is_drawn_again_where_it_changed() {
    let old = frame_of(
        Some(grey(4, 4, 0)),
        Rect::new(0, 0, 4, 4),
        Rect::new(40, 40, 4, 4),
    );
    let mut new = old.clone();
    new.pictures_mut()
        .get_mut(ID)
        .unwrap()
        .pixels_mut()
        .fill(u8::MAX);
    let atlas = shuyi_assets::font::FontAtlasDef {
        cell_w: 8,
        cell_h: 16,
        columns: 1,
        glyphs: alloc::collections::BTreeMap::default(),
        wide: alloc::collections::BTreeMap::default(),
        wide_sets: alloc::collections::BTreeMap::new(),
    };
    let font = Font::new(&atlas, &[]).unwrap();
    let mut frame = draw(&old);
    assert_eq!(text(&frame, 40, 40, 4, 1), ["####"]);
    render_changed(&old, &new, &font, &(), &mut frame);
    assert_eq!(text(&frame, 40, 40, 4, 1), ["...."]);
    assert_eq!(frame, draw(&new));
}

/// A `w × h` picture of only black and white pixels from `bits` (a bit a
/// pixel, row-major), as RGBA and as the one-bit picture of its threshold.
fn black_and_white(w: u16, h: u16, bits: &[bool]) -> (Painted, Painted) {
    let mut rgba = Painted::rgba(w, h);
    let mut one_bit = Painted::one_bit(w, h);
    let stride = one_bit.stride();
    let (colour, ones) = (rgba.pixels_mut(), one_bit.pixels_mut());
    for (i, &white) in bits.iter().enumerate() {
        let level = if white { u8::MAX } else { 0 };
        colour[i * 4..][..4].copy_from_slice(&[level, level, level, u8::MAX]);
        if white {
            let (x, y) = (i % usize::from(w), i / usize::from(w));
            ones[y * stride + x / 8] |= 0x80 >> (x % 8);
        }
    }
    (rgba, one_bit)
}

proptest! {
    /// ADR-0078 §7: dithering a picture of only black and white gives its
    /// threshold, wherever and at whatever size it is drawn.
    #[test]
    fn a_black_and_white_picture_dithers_to_its_threshold(
        (w, h, bits) in (1_u16..40, 1_u16..30).prop_flat_map(|(w, h)| {
            (Just(w), Just(h), prop::collection::vec(any::<bool>(), usize::from(w) * usize::from(h)))
        }),
        (x, y, dw, dh) in (-20_i32..400, -20_i32..240, 1_i32..120, 1_i32..90),
    ) {
        let (rgba, one_bit) = black_and_white(w, h, &bits);
        let (src, dest) = (Rect::new(0, 0, w.into(), h.into()), Rect::new(x, y, dw, dh));
        let dithered = draw(&frame_of(Some(rgba), src, dest));
        let threshold = draw(&frame_of(Some(one_bit), src, dest));
        prop_assert!(dithered == threshold);
    }
}

/// A plasma `w × h` of integers only: each channel a triangle wave of
/// `x`, `y` and `x + y` at its own period.
fn plasma(w: u16, h: u16, step: u32) -> Painted {
    let wave = |v: u32, period: u32| {
        let at = v % period;
        let half = period / 2;
        u8::try_from((if at < half { at } else { period - at }) * 255 / half).unwrap_or(u8::MAX)
    };
    let mut picture = Painted::rgba(w, h);
    let pixels = picture.pixels_mut();
    for y in 0..u32::from(h) {
        for x in 0..u32::from(w) {
            let at = usize::try_from((y * u32::from(w) + x) * 4).unwrap_or(0);
            let pixel = [
                wave(x + step, 24),
                wave(y + 2 * step, 18),
                wave(x + y + step, 30),
                255,
            ];
            pixels[at..at + 4].copy_from_slice(&pixel);
        }
    }
    picture
}

/// The FNV hash of `frame`, as a golden row holds it.
fn hash(frame: &Frame) -> String {
    let mut hasher = Hasher::new();
    hasher.bytes(frame);
    format!("{:016x}", hasher.finish())
}

/// ADR-0078 §7: an RGBA and an indexed picture dither the same on every
/// platform: the same frames on native and on wasm32 (CI's `determinism`
/// job runs this there).
#[test]
fn golden_painted_pictures_dither_the_same_everywhere() {
    let mut golden = shuyi_assets::golden!("tests/golden/onebit.ron");
    for (name, src, dest) in [
        (
            "plasma 64x48 at 1x",
            Rect::new(0, 0, 64, 48),
            Rect::new(3, 5, 64, 48),
        ),
        (
            "plasma 64x48 at 3x",
            Rect::new(0, 0, 64, 48),
            Rect::new(0, 0, 192, 144),
        ),
        (
            "plasma 64x48 stretched",
            Rect::new(8, 4, 40, 30),
            Rect::new(17, 9, 371, 222),
        ),
    ] {
        let frame = draw(&frame_of(Some(plasma(64, 48, 7)), src, dest));
        golden.check(format!("rgba: {name}"), hash(&frame));
    }
    let ramp: Vec<Rgb> = (0..=255).map(|v| Rgb::new(v, 255 - v, v / 2)).collect();
    let mut indexed = Painted::indexed(320, 200, &ramp);
    for (i, pixel) in indexed.pixels_mut().iter_mut().enumerate() {
        *pixel = u8::try_from((i % 320 + i / 320) % 256).unwrap_or(0);
    }
    let whole = Rect::new(0, 0, 320, 200);
    let frame = draw(&frame_of(Some(indexed), whole, Rect::new(40, 20, 320, 200)));
    golden.check("indexed: 320x200 ramp", hash(&frame));
    golden.assert();
}
