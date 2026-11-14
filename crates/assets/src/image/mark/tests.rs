use super::*;
use proptest::prelude::*;

/// A pixel-art picture of `colors`, each in a band of rows, and a
/// transparent row.
fn picture(colors: &[[u8; 4]]) -> Image {
    let width = 12;
    let mut rgba = Vec::new();
    for color in colors {
        for _ in 0..width * 2 {
            rgba.extend_from_slice(color);
        }
    }
    for _ in 0..width {
        rgba.extend_from_slice(&[0, 0, 0, 0]);
    }
    Image {
        width,
        height: u32::try_from(colors.len()).expect("few") * 2 + 1,
        rgba,
    }
}

/// Eight opaque colors, as a small game's tiles have (Hello, Grid's).
const EIGHT: [[u8; 4]; 8] = [
    [32, 32, 42, 255],
    [60, 60, 78, 255],
    [62, 52, 24, 255],
    [74, 74, 86, 255],
    [90, 200, 255, 255],
    [140, 140, 160, 255],
    [255, 90, 90, 255],
    [255, 215, 90, 255],
];

fn code(bits: u32) -> Code {
    Code::new(bits).expect("not zero")
}

/// The PNG of `image`, as a tool writes one.
fn png(image: &Image) -> Vec<u8> {
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, image.width, image.height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().expect("header");
    writer.write_image_data(&image.rgba).expect("pixels");
    writer.finish().expect("end");
    out
}

/// `bytes` decoded as the engine's PNG format reads a picture's pixels.
fn decoded(bytes: &[u8]) -> Image {
    let (width, height, rgba) = crate::bitmap::convert::png_pixels(bytes).expect("a PNG");
    Image {
        width,
        height,
        rgba,
    }
}

#[test]
fn a_marked_picture_names_its_code_and_keeps_its_palette() {
    let before = picture(&EIGHT);
    let mut marked = before.clone();
    mark(&mut marked, code(0x1234_5678)).expect("eight colors");
    assert_eq!(read(&marked), PictureRead::Code(code(0x1234_5678)));
    assert_eq!(palette_count(&marked), palette_count(&before));
    assert_eq!(palette_count(&before), 9);
    for (a, b) in before.rgba.iter().zip(&marked.rgba) {
        assert!(a.abs_diff(*b) <= 3, "{a} became {b}");
    }
    let transparent = before.rgba.len() - 12 * 4;
    assert_eq!(before.rgba[transparent..], marked.rgba[transparent..]);
}

#[test]
fn the_mark_survives_the_engines_png_round_trip() {
    let mut marked = picture(&EIGHT);
    mark(&mut marked, code(0x0bad_f00d)).expect("eight colors");
    let again = decoded(&png(&marked));
    assert_eq!(again, marked);
    assert_eq!(read(&again), PictureRead::Code(code(0x0bad_f00d)));
    assert_eq!(palette_count(&again), 9);
}

#[test]
fn an_unmarked_picture_with_clear_low_bits_names_no_code() {
    let clear: Vec<[u8; 4]> = EIGHT.iter().map(|&c| group(c)).collect();
    assert_eq!(read(&picture(&clear)), PictureRead::NoMark);
}

#[test]
fn one_carriers_bit_changed_names_no_code() {
    let mut marked = picture(&EIGHT);
    mark(&mut marked, code(0x1234_5678)).expect("eight colors");
    let first = &marked.rgba[..4].to_vec();
    let mut changed = marked.clone();
    for pixel in changed.rgba.as_chunks_mut::<4>().0 {
        if pixel[..] == first[..] {
            pixel[2] ^= 1;
        }
    }
    assert_eq!(read(&changed), PictureRead::NoMark);
}

#[test]
fn too_few_colors_are_refused_and_said() {
    let mut six = picture(&EIGHT[..6]);
    assert_eq!(
        mark(&mut six, code(1)),
        Err(MarkError::TooFewColors { found: 6 })
    );
    assert_eq!(read(&six), PictureRead::TooFewColors { found: 6 });
    assert_eq!(capacity(&six), 36);
}

#[test]
fn colors_that_differ_only_in_their_low_bits_carry_nothing_and_stay() {
    let mut colors = EIGHT.to_vec();
    colors.push([32, 32, 41, 255]);
    let before = picture(&colors);
    let mut marked = before.clone();
    mark(&mut marked, code(77)).expect("seven carriers left");
    assert_eq!(read(&marked), PictureRead::Code(code(77)));
    assert_eq!(palette_count(&marked), palette_count(&before));
    // The two of one group are the first band and the last.
    let band = 24 * 4;
    assert_eq!(before.rgba[..band], marked.rgba[..band]);
    assert_eq!(
        before.rgba[8 * band..9 * band],
        marked.rgba[8 * band..9 * band]
    );
}

#[test]
fn a_crop_that_keeps_every_color_keeps_the_mark_and_one_that_drops_one_doesnt() {
    let mut marked = picture(&EIGHT);
    mark(&mut marked, code(0x00ab_cdef)).expect("eight colors");
    let rows = |image: &Image, from: u32, to: u32| Image {
        width: image.width,
        height: to - from,
        rgba: image.rgba[(from * image.width * 4) as usize..(to * image.width * 4) as usize]
            .to_vec(),
    };
    let half_rows: Vec<u8> = (0..8usize)
        .flat_map(|band| {
            let start = band * 2 * 12 * 4;
            marked.rgba[start..start + 6 * 4].to_vec()
        })
        .collect();
    let narrow = Image {
        width: 6,
        height: 8,
        rgba: half_rows,
    };
    assert_eq!(read(&narrow), PictureRead::Code(code(0x00ab_cdef)));
    let without_first = rows(&marked, 2, 17);
    assert_ne!(read(&without_first), PictureRead::Code(code(0x00ab_cdef)));
}

proptest! {
    #[test]
    fn any_code_reads_back_through_a_png(bits in 1u32.., extra in 0usize..6) {
        let mut colors = EIGHT.to_vec();
        for at in 0..extra {
            let v = u8::try_from(at).expect("few") * 20 + 100;
            colors.push([v, 16, 200 - v, 255]);
        }
        let mut marked = picture(&colors);
        let before = palette_count(&marked);
        mark(&mut marked, code(bits)).expect("enough colors");
        let again = decoded(&png(&marked));
        prop_assert_eq!(read(&again), PictureRead::Code(code(bits)));
        prop_assert_eq!(palette_count(&again), before);
    }
}
