//! The ditherers' tests: each on a 4 × 4 grey ramp checked by hand, the
//! mask and the cut, inverting, a flat grey under Bayer as the pattern
//! map's own dither, the format and its settings, a golden table of what
//! each gives on frozen fixtures, and random pictures never panicking.
use proptest::prelude::*;
use shuyi_core::state_hash;

use super::*;
use crate::bitmap::{Bitmap, OneBitPicture};
use crate::format::builtin_tests::bundle;
use crate::format::{AssetPath, DecodeCx, Formats};
use crate::golden::Golden;

/// An opaque grey pixel of brightness `v`.
fn grey(v: u8) -> [u8; 4] {
    [v, v, v, 255]
}

/// A picture `width` pixels wide of `pixels`, row by row.
fn image(width: u32, pixels: &[[u8; 4]]) -> Image {
    let height = u32::try_from(pixels.len()).unwrap_or(0) / width;
    Image {
        width,
        height,
        rgba: pixels.iter().flatten().copied().collect(),
    }
}

/// The 4 × 4 grey ramp: brightness 0, 17, 34 … 255, row by row.
fn ramp() -> Image {
    let pixels: Vec<[u8; 4]> = (0..16u8).map(|i| grey(i * 17)).collect();
    image(4, &pixels)
}

/// The rows of `image` made one-bit by `settings`: `#` black, `.` white,
/// ` ` see-through.
fn rows_of(image: &Image, settings: OneBitSettings) -> Vec<String> {
    let file = dither(image, settings).unwrap_or_else(|e| panic!("{e}"));
    let bitmap = Bitmap::parse(&file).unwrap_or_else(|e| panic!("{e}"));
    (0..bitmap.height())
        .map(|y| {
            (0..bitmap.width())
                .map(|x| match bitmap.pixel(0, x, y) {
                    Some(true) => '.',
                    Some(false) => '#',
                    None => ' ',
                })
                .collect()
        })
        .collect()
}

/// [`rows_of`] with the default settings but `dither`.
fn dithered(image: &Image, dither: Dither) -> Vec<String> {
    rows_of(image, OneBitSettings::new(dither))
}

#[test]
fn the_brightness_weighs_green_most_and_keeps_the_ends() {
    assert_eq!(brightness([0, 0, 0, 255]), 0);
    assert_eq!(brightness([255, 255, 255, 0]), 255);
    assert_eq!(brightness([255, 0, 0, 255]), 77);
    assert_eq!(brightness([0, 255, 0, 255]), 149);
    assert_eq!(brightness([0, 0, 255, 255]), 29);
    for v in 0..=255 {
        assert_eq!(brightness(grey(v)), v, "a grey is its own brightness");
    }
}

#[test]
fn threshold_cuts_the_ramp_at_its_level() {
    let cut = dithered(&ramp(), Dither::Threshold(128));
    assert_eq!(cut, ["####", "####", "....", "...."]);
    // 119 is black under 120, white under 119.
    assert_eq!(dithered(&ramp(), Dither::Threshold(120))[1], "####");
    assert_eq!(dithered(&ramp(), Dither::Threshold(119))[1], "###.");
    assert_eq!(dithered(&ramp(), Dither::Threshold(0)), ["...."; 4]);
}

#[test]
fn bayer_lights_each_grey_by_its_order() {
    // Row 1 is 68 to 119: dark 187 to 136 against orders 48, 16, 56, 24
    // (4 × order + 2 is 194, 66, 226, 98).
    let bayer = dithered(&ramp(), Dither::Bayer);
    assert_eq!(bayer, ["####", ".#.#", "#.#.", "...."]);
}

#[test]
fn atkinson_spreads_six_eighths_of_each_error() {
    // Worked by hand: 17 at the second pixel is black, so 17 / 8 = 2
    // goes to six pixels after it, and so on; 135 at row 1's third pixel
    // is the first white.
    let atkinson = dithered(&ramp(), Dither::Atkinson);
    assert_eq!(atkinson, ["####", "##.#", "....", "...."]);
    assert_eq!(
        OneBitSettings::default(),
        OneBitSettings::new(Dither::Atkinson)
    );
}

#[test]
fn none_takes_black_and_white_and_refuses_a_grey() {
    let (b, w) = (grey(0), grey(255));
    let art = image(3, &[b, w, b, w, w, b]);
    assert_eq!(dithered(&art, Dither::None), ["#.#", "..#"]);
    let error = dither(&ramp(), OneBitSettings::new(Dither::None)).err();
    assert_eq!(
        error.as_deref(),
        Some(
            "the picture isn't black and white, as `dither: none` needs: the pixel at column \
             1, row 0 is #111111"
        )
    );
    // A see-through grey isn't drawn, so it is no error.
    let clear = image(2, &[b, [9, 9, 9, 0]]);
    assert_eq!(dithered(&clear, Dither::None), ["# "]);
    // Nor is a colour that isn't grey black or white.
    let red = image(1, &[[255, 0, 0, 255]]);
    assert!(dither(&red, OneBitSettings::new(Dither::None)).is_err());
}

#[test]
fn see_through_pixels_are_masked_white_and_take_no_error() {
    let clear = [200, 10, 10, 127];
    let art = image(4, &[grey(100), clear, grey(100), grey(100)]);
    let file = dither(&art, OneBitSettings::new(Dither::Atkinson)).unwrap_or_default();
    let bitmap = Bitmap::parse(&file).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(bitmap.mask(), Some(&[0b1011_0000][..]));
    // White under the mask; 100's error skips it to the third pixel.
    assert_eq!(bitmap.style(0), Some(&[0b0100_0000][..]));
    assert_eq!(
        rows_of(&art, OneBitSettings::new(Dither::Atkinson)),
        ["# ##"]
    );
    // A cut of 0 draws every pixel: no mask.
    let mut all = OneBitSettings::new(Dither::Threshold(128));
    all.alpha_cut = 0;
    let file = dither(&art, all).unwrap_or_default();
    assert_eq!(Bitmap::parse(&file).map(|b| b.mask()), Ok(None));
    // A cut of 128 counts 128 drawn.
    let half = image(2, &[[0, 0, 0, 128], [0, 0, 0, 127]]);
    assert_eq!(dithered(&half, Dither::Bayer), ["# "]);
}

#[test]
fn inverting_turns_the_brightness_over() {
    let mut inverted = OneBitSettings::new(Dither::Threshold(128));
    inverted.invert = true;
    assert_eq!(rows_of(&ramp(), inverted), ["....", "....", "####", "####"]);
    inverted.dither = Dither::None;
    let art = image(2, &[grey(0), grey(255)]);
    assert_eq!(rows_of(&art, inverted), [".#"]);
}

#[test]
fn a_flat_grey_under_bayer_is_the_pattern_maps_dither() {
    for v in [0, 1, 40, 64, 127, 128, 191, 192, 254, 255] {
        let art = image(16, &[grey(v); 16 * 16]);
        let file = dither(&art, OneBitSettings::new(Dither::Bayer)).unwrap_or_default();
        let bitmap = Bitmap::parse(&file).unwrap_or_else(|e| panic!("{e}"));
        let pattern = crate::onebit::dither(255 - v);
        let style = bitmap.style(0).unwrap_or_default();
        for y in 0..16 {
            // Two bytes a row, each the pattern's row.
            assert_eq!(style[y * 2], pattern[y % 8], "grey {v}, row {y}");
            assert_eq!(style[y * 2 + 1], pattern[y % 8], "grey {v}, row {y}");
        }
    }
}

#[test]
fn the_file_has_the_pictures_size_as_its_source() {
    let file = dither(&ramp(), OneBitSettings::default()).unwrap_or_default();
    let bitmap = Bitmap::parse(&file).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!((bitmap.width(), bitmap.height()), (4, 4));
    assert_eq!(bitmap.source_size(), (4, 4));
    assert_eq!((bitmap.styles(), bitmap.mask()), (1, None));
}

#[test]
fn a_picture_too_big_or_short_of_pixels_is_refused() {
    let wide = Image {
        width: 70_000,
        height: 1,
        rgba: vec![0; 70_000 * 4],
    };
    let error = dither(&wide, OneBitSettings::default()).err();
    assert_eq!(
        error.as_deref(),
        Some("70000×1 px is too big for a one-bit picture")
    );
    let short = Image {
        width: 2,
        height: 2,
        rgba: vec![0; 12],
    };
    let error = dither(&short, OneBitSettings::default()).err();
    assert_eq!(
        error.as_deref(),
        Some("a 2×2 px picture needs 16 bytes of pixels, and has 12")
    );
    assert!(dither(&Image::default(), OneBitSettings::default()).is_err());
}

#[test]
fn the_section_reads_every_dither_and_refuses_unknown_fields() {
    let read = |text: &str| ::ron::from_str::<OneBitSettings>(text).map_err(|e| e.to_string());
    assert_eq!(read("()"), Ok(OneBitSettings::default()));
    let mut expected = OneBitSettings::new(Dither::Threshold(90));
    expected.alpha_cut = 1;
    expected.invert = true;
    assert_eq!(
        read("(dither: threshold(90), alpha_cut: 1, invert: true)"),
        Ok(expected)
    );
    assert_eq!(
        read("(dither: bayer)"),
        Ok(OneBitSettings::new(Dither::Bayer))
    );
    assert_eq!(
        read("(dither: none)"),
        Ok(OneBitSettings::new(Dither::None))
    );
    assert_eq!(
        read("(dither: atkinson)"),
        Ok(OneBitSettings::new(Dither::Atkinson))
    );
    assert!(read("(dither: floyd)").is_err());
    assert!(read("(dither: threshold(300))").is_err());
    assert!(read("(cut: 3)").is_err_and(|e| e.contains("cut")));
}

#[test]
fn the_standard_formats_check_the_section_with_no_bake() {
    let files = bundle(vec![
        ("images/hero.png", Vec::new()),
        (
            "images/hero.png.meta",
            b"(\n    onebit: (\n        dither: bayer,\n        dots: 2,\n    ),\n)".to_vec(),
        ),
    ]);
    let errors = Formats::standard().check_import_settings(files);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert_eq!(errors[0].file, "base/images/hero.png.meta");
    assert_eq!(errors[0].line, Some(4));
    assert!(errors[0].message.contains("dots"), "{}", errors[0].message);
}

/// The PNG file of `image`.
pub(crate) fn png_of(image: &Image) -> Vec<u8> {
    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, image.width, image.height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap_or_else(|e| panic!("{e}"));
        writer
            .write_image_data(&image.rgba)
            .unwrap_or_else(|e| panic!("{e}"));
    }
    out
}

/// What the standard formats read as a one-bit picture at `path` of
/// `files`.
fn picture(
    files: &'static crate::bundle::Bundle,
    path: &str,
) -> Result<OneBitPicture, Vec<crate::ContentError>> {
    Formats::standard().load::<OneBitPicture>(files, path)
}

#[test]
fn a_png_is_dithered_by_its_settings_or_its_folders() {
    let png = png_of(&ramp());
    let files = bundle(vec![
        ("a.png", png.clone()),
        ("images/_folder.meta", b"(onebit: (dither: bayer))".to_vec()),
        ("images/b.png", png.clone()),
        ("images/c.png", png.clone()),
        (
            "images/c.png.meta",
            b"(onebit: (dither: threshold(128)))".to_vec(),
        ),
    ]);
    let made = |path: &str, dither: Dither| {
        let expected = super::dither(&ramp(), OneBitSettings::new(dither)).unwrap_or_default();
        let read = picture(files, path).unwrap_or_else(|e| panic!("{e:?}"));
        assert_eq!(read.file(), expected, "{path}");
    };
    made("a.png", Dither::Atkinson);
    made("images/b.png", Dither::Bayer);
    made("images/c.png", Dither::Threshold(128));
}

#[test]
fn the_format_names_the_file_of_a_grey_under_none_and_reads_a_one_bit_file() {
    let file = super::dither(&ramp(), OneBitSettings::default()).unwrap_or_default();
    let files = bundle(vec![
        ("grey.png", png_of(&ramp())),
        ("grey.png.meta", b"(onebit: (dither: none))".to_vec()),
        ("drawn.png", file.clone()),
        ("bad.png", b"not a picture".to_vec()),
    ]);
    let errors = picture(files, "grey.png").err().unwrap_or_default();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].file, "base/grey.png");
    assert!(
        errors[0].message.contains("isn't black and white"),
        "{}",
        errors[0].message
    );
    // A one-bit picture file in a PNG's place is read as it is.
    let drawn = picture(files, "drawn.png").unwrap_or_else(|e| panic!("{e:?}"));
    assert_eq!(drawn.file(), file);
    assert!(picture(files, "bad.png").is_err());
    let formats = Formats::standard();
    assert_eq!(
        formats.ids::<OneBitPicture>(),
        ["one-bit", "dithered"],
        "a one-bit file is known by its bytes first"
    );
    let cx = DecodeCx::new();
    assert!(
        Dithered
            .decode(&AssetPath::new("a.png"), b"\x89PNG\r\n\x1a\n", &cx)
            .is_err()
    );
}

/// The golden table's fixtures, frozen for it: the ramp, a 16 × 16
/// gradient fading out to the right, colour noise from a fixed
/// sequence, and black-and-white art.
pub(crate) fn fixtures() -> Vec<(&'static str, Image)> {
    let gradient: Vec<[u8; 4]> = (0..16u8)
        .flat_map(|y| (0..16u8).map(move |x| [x * 16, y * 16, 255 - x * 8, 255 - x * 16]))
        .collect();
    let mut seed: u32 = 2_463_534_242;
    let noise: Vec<[u8; 4]> = (0..24 * 20)
        .map(|_| {
            // xorshift32: the same numbers everywhere.
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            seed.to_le_bytes()
        })
        .collect();
    let art: Vec<[u8; 4]> = (0..10u8 * 6)
        .map(|i| {
            if i % 3 == 0 || i % 7 == 0 {
                grey(0)
            } else {
                grey(255)
            }
        })
        .collect();
    vec![
        ("ramp", ramp()),
        ("gradient", image(16, &gradient)),
        ("noise", image(24, &noise)),
        ("art", image(10, &art)),
    ]
}

#[test]
fn every_dither_gives_its_golden_picture() {
    let mut golden: Golden = crate::golden!("tests/golden/dither.ron");
    let mut inverted = OneBitSettings::new(Dither::Atkinson);
    inverted.invert = true;
    let mut uncut = OneBitSettings::new(Dither::Bayer);
    uncut.alpha_cut = 0;
    let settings = [
        ("atkinson", OneBitSettings::new(Dither::Atkinson)),
        ("bayer", OneBitSettings::new(Dither::Bayer)),
        ("threshold 100", OneBitSettings::new(Dither::Threshold(100))),
        ("none", OneBitSettings::new(Dither::None)),
        ("atkinson inverted", inverted),
        ("bayer uncut", uncut),
    ];
    for (name, image) in fixtures() {
        for (how, settings) in &settings {
            // Only the black-and-white art takes `none`.
            let Ok(file) = dither(&image, *settings) else {
                assert!(name != "art" && *how == "none", "{name}: {how}");
                continue;
            };
            let hash = state_hash(&file).unwrap_or_else(|e| panic!("{e}"));
            golden.check(format!("{name}: {how}"), hash);
        }
    }
    golden.assert();
}

/// A random picture: up to 20 × 12 pixels of random bytes.
fn any_image() -> impl Strategy<Value = Image> {
    (1u32..20, 1u32..12).prop_flat_map(|(width, height)| {
        let size = usize::try_from(width * height * 4).unwrap_or(0);
        proptest::collection::vec(any::<u8>(), size).prop_map(move |rgba| Image {
            width,
            height,
            rgba,
        })
    })
}

/// Any of the dithers.
fn any_dither() -> impl Strategy<Value = Dither> {
    prop_oneof![
        Just(Dither::Atkinson),
        Just(Dither::Bayer),
        any::<u8>().prop_map(Dither::Threshold),
        Just(Dither::None),
    ]
}

proptest! {
    /// No picture makes a dither panic, and the file is the picture's
    /// size, with a mask only where a pixel is under the cut.
    #[test]
    fn random_pictures_never_panic_and_keep_their_size(
        image in any_image(),
        dither in any_dither(),
        alpha_cut: u8,
        invert: bool,
    ) {
        let mut settings = OneBitSettings::new(dither);
        settings.alpha_cut = alpha_cut;
        settings.invert = invert;
        let Ok(file) = super::dither(&image, settings) else {
            prop_assert_eq!(dither, Dither::None);
            return Ok(());
        };
        let bitmap = Bitmap::parse(&file).map_err(TestCaseError::fail)?;
        let size = (u32::from(bitmap.width()), u32::from(bitmap.height()));
        prop_assert_eq!(size, (image.width, image.height));
        prop_assert_eq!(bitmap.source_size(), (bitmap.width(), bitmap.height()));
        let cut = image.rgba.as_chunks::<4>().0.iter().any(|p| p[3] < alpha_cut);
        prop_assert_eq!(bitmap.mask().is_some(), cut);
    }

    /// Atkinson's dither of the inverted picture is the dither turned
    /// over, pixel for drawn pixel.
    #[test]
    fn atkinson_inverted_is_atkinson_turned_over(image in any_image()) {
        let mut settings = OneBitSettings::new(Dither::Atkinson);
        settings.alpha_cut = 0;
        let plain = super::dither(&image, settings).map_err(TestCaseError::fail)?;
        settings.invert = true;
        let turned = super::dither(&image, settings).map_err(TestCaseError::fail)?;
        let (plain, turned) = (Bitmap::parse(&plain), Bitmap::parse(&turned));
        let (plain, turned) = (plain.map_err(TestCaseError::fail)?, turned.map_err(TestCaseError::fail)?);
        for y in 0..plain.height() {
            for x in 0..plain.width() {
                let (a, b) = (plain.pixel(0, x, y), turned.pixel(0, x, y));
                prop_assert_eq!(a.map(|w| !w), b, "at {}, {}", x, y);
            }
        }
    }
}
