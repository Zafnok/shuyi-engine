use alloc::collections::BTreeMap;

use shuyi_assets::font::{FALLBACK_GLYPH, FontAtlasDef};
use shuyi_assets::format::{AssetPath, DecodeCx, Format};
use shuyi_assets::image::{ImageInfo, ImageTable, Png};
use shuyi_assets::parts::PartsDef;

use super::*;
use crate::frame::color::Rgb;
use crate::frame::glyph_buffer::{Cell, GlyphBuffer, Layer, PxRect};
use crate::frame::painted::{PaintedId, PaintedSprite};
use crate::frame::raster::Painter;

/// The part sets' fixture (`shuyi-assets`' tests): four slots of 8 × 8
/// parts, drawn for the tests, and two kinds.
const FACES: &str = include_str!("../../../../assets/tests/fixtures/parts/faces.ron");

/// Its picture, seven parts side by side.
const FACES_PNG: &[u8] = include_bytes!("../../../../assets/tests/fixtures/parts/faces.png");

/// The fixture's picture, its id, and the set.
fn faces() -> (Image, ImageId, PartSet) {
    let image = Png
        .decode(
            &AssetPath::new("parts/faces.png"),
            FACES_PNG,
            &DecodeCx::new(),
        )
        .unwrap();
    let table = ImageTable::new([("parts/faces.png", image.info())].into());
    let id = table.id("parts/faces.png").unwrap();
    let set = PartsDef::from_source("parts/faces.ron", FACES)
        .unwrap()
        .resolve("parts/faces.ron", "parts", &table)
        .unwrap();
    (image, id, set)
}

/// The key greys of the fixture.
const SKIN: u8 = 200;
const SHADOW: u8 = 150;
const HAIR: u8 = 120;

/// A look of the fixture: a round head, wide eyes and long hair, in
/// colours made up here.
fn look() -> Appearance {
    let parts = ["head_round", "eyes_wide", "hair_long"]
        .into_iter()
        .map(|p| Some(p.to_owned()))
        .chain([None])
        .collect();
    let colours = BTreeMap::from([
        (SKIN, [250, 200, 150]),
        (SHADOW, [200, 150, 100]),
        (HAIR, [60, 30, 10]),
    ]);
    Appearance::new("plain".into(), parts, colours)
}

/// Pixel `(x, y)` of `picture`.
fn at(picture: &Painted, x: u16, y: u16) -> [u8; 4] {
    picture.rgba_at(x, y)
}

#[test]
fn a_look_is_painted_by_its_keys_over_its_slots() {
    let (image, id, set) = faces();
    let mut out = Painted::rgba(8, 8);
    paint(&set, &look(), |i| (i == id).then_some(&image), &mut out).unwrap();
    assert_eq!(out.revision(), 1);
    // A key grey takes its colour, the shadow its own.
    assert_eq!(at(&out, 3, 3), [250, 200, 150, 255]);
    assert_eq!(at(&out, 6, 4), [200, 150, 100, 255]);
    // A red pixel stays red, the outline its own grey (not a key), the
    // eye whites white, and the eyes' key, which the look doesn't colour,
    // its grey.
    assert_eq!(at(&out, 3, 5), [220, 40, 40, 255]);
    assert_eq!(at(&out, 1, 6), [20, 20, 20, 255]);
    assert_eq!(at(&out, 1, 3), [255, 255, 255, 255]);
    assert_eq!(at(&out, 2, 2), [90, 90, 90, 255]);
    // The hair is drawn over the head (a later slot over an earlier one);
    // its pixels with alpha 100 leave the head's outline below them.
    assert_eq!(at(&out, 1, 1), [60, 30, 10, 255]);
    assert_eq!(at(&out, 0, 5), [20, 20, 20, 255]);
    // Where no part draws, the picture is clear.
    assert_eq!(at(&out, 0, 0), [0, 0, 0, 0]);
    // Painted again, the revision moves by one more.
    paint(&set, &look(), |_| Some(&image), &mut out).unwrap();
    assert_eq!(out.revision(), 2);
}

#[test]
fn painting_again_clears_what_was_painted() {
    let (image, _, set) = faces();
    let mut out = Painted::rgba(8, 8);
    paint(&set, &look(), |_| Some(&image), &mut out).unwrap();
    let mut bald = look();
    bald.parts[2] = None;
    paint(&set, &bald, |_| Some(&image), &mut out).unwrap();
    // The hair's pixel past the head is gone.
    assert_eq!(at(&out, 0, 3), [20, 20, 20, 255]);
    assert_eq!(at(&out, 2, 0), [20, 20, 20, 255]);
    assert_eq!(at(&out, 0, 0), [0, 0, 0, 0]);
}

#[test]
fn a_picture_of_the_wrong_size_or_kind_or_a_part_without_pixels_is_an_error() {
    let (image, _, set) = faces();
    let pixels = |_| Some(&image);
    let mut small = Painted::rgba(8, 7);
    assert_eq!(
        paint(&set, &look(), pixels, &mut small),
        Err(PaintError::WrongSize {
            want: (8, 8),
            got: (8, 7)
        })
    );
    let mut indexed = Painted::indexed(8, 8, &[Rgb::new(0, 0, 0)]);
    let wrong = paint(&set, &look(), pixels, &mut indexed).unwrap_err();
    assert_eq!(
        wrong.to_string(),
        "a part set paints an RGBA picture, not a indexed one"
    );
    let mut out = Painted::rgba(8, 8);
    let missing = paint(&set, &look(), |_| None, &mut out).unwrap_err();
    assert_eq!(
        missing.to_string(),
        "part \"head_round\": the pixels of parts/faces.png aren't there"
    );
    let cut = Image::filled(40, 8, [0; 4]);
    assert!(matches!(
        paint(&set, &look(), |_| Some(&cut), &mut out),
        Err(PaintError::NoPixels { .. })
    ));
    let mut other = look();
    other.parts[0] = Some("head_star".into());
    assert!(matches!(
        paint(&set, &other, pixels, &mut out),
        Err(PaintError::NotOfTheSet(_))
    ));
    other.parts = vec![None; 5];
    assert!(paint(&set, &other, pixels, &mut out).is_err());
    // A failure paints nothing.
    assert_eq!(out.revision(), 0);
    assert_eq!(small.revision(), 0);
}

/// `image` as text, a letter a colour in the order they first show, `.`
/// the clear colour, with a legend of the letters.
fn as_text(image: &Image, clear: Rgb) -> String {
    let mut letters: Vec<[u8; 4]> = Vec::new();
    let mut rows = String::new();
    for y in 0..image.height {
        for x in 0..image.width {
            let pixel = image.pixel(x, y).unwrap();
            if pixel == [clear.r, clear.g, clear.b, 255] {
                rows.push('.');
                continue;
            }
            let i = letters.iter().position(|&l| l == pixel).unwrap_or_else(|| {
                letters.push(pixel);
                letters.len() - 1
            });
            rows.push(char::from(b'a' + u8::try_from(i).unwrap()));
        }
        rows.push('\n');
    }
    let legend: Vec<String> = letters
        .iter()
        .enumerate()
        .map(|(i, [r, g, b, _])| {
            let letter = char::from(b'a' + u8::try_from(i).unwrap());
            format!("{letter} #{r:02x}{g:02x}{b:02x}")
        })
        .collect();
    format!("{rows}{}\n", legend.join("\n"))
}

#[test]
fn three_rolled_faces_side_by_side_look_as_they_did() {
    let (image, id, set) = faces();
    let clear = Rgb::new(16, 16, 16);
    let atlas = FontAtlasDef::new(8, 16, BTreeMap::from([(' ', 0), (FALLBACK_GLYPH, 1)]), 2);
    let atlas_image = Image::filled(16, 16, [0; 4]);
    let painter = Painter::new(atlas, atlas_image, BTreeMap::new(), clear);
    let mut buf = GlyphBuffer::new(4, 1, Cell::new(' ', clear, clear));
    let faces = [
        (PaintedId::new("face 1"), "plain", 1),
        (PaintedId::new("face 2"), "plain", 3),
        (PaintedId::new("face 3"), "capped", 6),
    ];
    let mut x = 1;
    for (face, kind, seed) in faces {
        let look = set.appearance(kind, seed, &[]).unwrap();
        let picture = buf
            .pictures_mut()
            .get_or_insert_with(face, || Painted::rgba(8, 8));
        paint(&set, &look, |i| (i == id).then_some(&image), picture).unwrap();
        let whole = PxRect::new(0, 0, 8, 8);
        buf.add_painted(PaintedSprite::new(
            face,
            whole,
            PxRect::new(x, 4, 8, 8),
            Layer::Over,
        ));
        x += 10;
    }
    let shot = painter.render(&buf, 1);
    let shown = as_text(&shot, clear);
    // Rows 4 to 11 hold the faces.
    let rows: Vec<&str> = shown.lines().collect();
    insta::assert_snapshot!(format!(
        "{}\n{}",
        rows[4..12].join("\n"),
        rows[16..].join("\n")
    ));
    // Three faces, not one face three times.
    let shot = &shot;
    let face = |x: u32| {
        (0..8)
            .flat_map(|y| (0..8).map(move |dx| shot.pixel(x + dx, 4 + y)))
            .collect::<Vec<_>>()
    };
    assert!(face(1) != face(11) && face(11) != face(21) && face(1) != face(21));
    let info = ImageInfo::new(32, 16);
    assert_eq!(shot.info(), info);
}
