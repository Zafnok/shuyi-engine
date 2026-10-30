//! A one-bit target's bake (ADR-0008 §4, ADR-0007 §4): every picture
//! goes into the pack as a one-bit picture file at its PNG's path, made
//! by its own settings, and decoding it equals dithering the decoded
//! source with the same settings, on every fixture; a target's layer
//! replaces a picture; and a bad picture or plan is refused.
use super::*;
use crate::bitmap::dither::tests::{fixtures, png_of};
use crate::bitmap::dither::{Dither, OneBitSettings, dither};
use crate::bitmap::{Bitmap, OneBitFile, OneBitInfo, OneBitPicture};
use crate::bundle::{Bundle, Layer};
use crate::format::{AssetPath, DecodeCx, Format, Formats};
use crate::image::{Image, ImageInfo, ImageTable, Png};
use crate::palette::PaletteDef;

/// The stamp the tests bake with.
const STAMP: &str = "one-bit test 1";

/// The bake of a one-bit target: every picture a one-bit file.
fn one_bit() -> Bake {
    Bake::new(STAMP).under_as("", OneBitFile)
}

/// Leaks `files` into a table layer named `name`.
fn layer(name: &'static str, files: Vec<(String, Vec<u8>)>) -> Layer {
    let mut table: Vec<(&'static str, &'static [u8])> = files
        .into_iter()
        .map(|(path, bytes)| {
            let path: &'static str = Box::leak(path.into_boxed_str());
            (path, &*Box::leak(bytes.into_boxed_slice()))
        })
        .collect();
    table.sort_unstable_by_key(|&(path, _)| path);
    Layer::table(name, Box::leak(table.into_boxed_slice()))
}

/// The settings each fixture is baked with, and the file that says so.
fn settings_of(name: &str) -> (OneBitSettings, Option<&'static [u8]>) {
    match name {
        "gradient" => (
            OneBitSettings::new(Dither::Bayer),
            Some(b"(onebit: (dither: bayer))"),
        ),
        "noise" => {
            let mut settings = OneBitSettings::new(Dither::Threshold(100));
            settings.invert = true;
            let file: &[u8] = b"(onebit: (dither: threshold(100), invert: true))";
            (settings, Some(file))
        }
        "art" => (
            OneBitSettings::new(Dither::None),
            Some(b"(onebit: (dither: none))"),
        ),
        _ => (OneBitSettings::default(), None),
    }
}

/// A game of every fixture as a PNG under `images/`, each with its
/// settings, and a palette that isn't a picture.
fn game() -> Layer {
    let mut files = vec![("data/palette.ron".to_owned(), b"{}".to_vec())];
    for (name, image) in fixtures() {
        files.push((format!("images/{name}.png"), png_of(&image)));
        if let (_, Some(meta)) = settings_of(name) {
            files.push((format!("images/{name}.png.meta"), meta.to_vec()));
        }
    }
    layer("assets", files)
}

/// The pack `bake` makes of `bundle`, opened, and what went in.
fn baked(bake: &Bake, bundle: &Bundle) -> (Bundle, Baked) {
    let baked = bake
        .bake(&Formats::standard(), bundle, Some("playdate"))
        .unwrap_or_else(|e| panic!("{e:?}"));
    let bytes: &'static [u8] = baked.bytes.clone().leak();
    let pack = Pack::open(bytes, STAMP).unwrap_or_else(|e| panic!("{e}"));
    (Bundle::new(vec![pack.into_layer("pack")]), baked)
}

#[test]
fn decoding_a_baked_picture_equals_dithering_its_decoded_source() {
    let source = Bundle::new(vec![game()]);
    let bake = one_bit();
    let (pack, made) = baked(&bake, &source);
    for (name, _) in fixtures() {
        let path = format!("images/{name}.png");
        let file = AssetPath::new(&*path);
        let cx = DecodeCx::new();
        // The pack's file is a one-bit picture, read as it is…
        let in_pack = pack.bytes(&path).unwrap_or_default();
        assert!(Bitmap::is_bitmap(in_pack), "{path}");
        let decoded = OneBitFile.decode(&file, in_pack, &cx);
        // …and the source, decoded, then dithered by its settings, is it.
        let image: Image = Png
            .decode(&file, source.bytes(&path).unwrap_or_default(), &cx)
            .unwrap_or_else(|e| panic!("{e:?}"));
        let again = dither(&image, settings_of(name).0).map(OneBitPicture::new);
        assert_eq!(decoded.ok(), again.ok().and_then(Result::ok), "{path}");
    }
    // Reading every picture as the game would finds no error in the
    // sources nor in the pack, and a picture that doesn't read is one.
    let mut formats = Formats::standard();
    bake.register(&mut formats)
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(bake.load(&formats, &source), []);
    assert_eq!(bake.load(&formats, &pack), []);
    let broken = layer(
        "broken",
        vec![("images/ramp.png".to_owned(), b"\x89PNG".to_vec())],
    );
    let broken = Bundle::new(vec![broken, game()]);
    let errors = bake.load(&formats, &broken);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert_eq!(errors[0].file, "broken/images/ramp.png");
    // The check proves the same of every picture, and the rest is kept.
    assert_eq!(bake.check(&formats, &source, &pack), []);
    assert_eq!(pack.bytes("data/palette.ron"), Some(&b"{}"[..]));
    assert_eq!(
        pack.bytes("images/art.png.meta"),
        source.bytes("images/art.png.meta")
    );
    let kinds = |path: &str| {
        let file = made.files.iter().find(|f| f.path == path);
        file.map(|f| f.kinds.clone()).unwrap_or_default()
    };
    assert_eq!(kinds("images/ramp.png"), ["one-bit picture"]);
    assert_eq!(kinds("data/palette.ron"), [] as [&str; 0]);
}

#[test]
fn a_one_bit_pack_reads_as_its_sources_did_for_sizes_and_kinds() {
    let source = Bundle::new(vec![game()]);
    let (pack, _) = baked(&one_bit(), &source);
    let formats = Formats::standard();
    // The image table reads each picture's size from its one-bit file.
    let sizes = ImageTable::load_with(&source, &formats, &[]);
    assert_eq!(ImageTable::load_with(&pack, &formats, &[]), sizes);
    let info = formats.load::<ImageInfo>(&pack, "images/gradient.png");
    assert_eq!(info, Ok(ImageInfo::new(16, 16)));
    let shown = OneBitInfo.id();
    assert_eq!(formats.ids::<ImageInfo>(), ["png", shown]);
    // A palette is no picture: it went in as it was.
    let palette = formats.load::<PaletteDef>(&pack, "data/palette");
    assert!(palette.is_ok(), "{palette:?}");
}

#[test]
fn a_file_in_the_targets_layer_replaces_the_dithered_picture() {
    let (b, w) = ([0, 0, 0, 255], [255, 255, 255, 255]);
    let drawn = Image {
        width: 2,
        height: 1,
        rgba: [b, w].concat(),
    };
    let playdate = layer(
        "assets@playdate",
        vec![
            ("images/ramp.png".to_owned(), png_of(&drawn)),
            (
                "images/ramp.png.meta".to_owned(),
                b"(onebit: (dither: none))".to_vec(),
            ),
        ],
    );
    let source = Bundle::for_target(vec![game()], &[playdate], Some("playdate"));
    let (pack, made) = baked(&one_bit(), &source);
    let file = made.files.iter().find(|f| f.path == "images/ramp.png");
    let shown = file.map(|f| f.shown.as_str());
    assert_eq!(shown, Some("assets@playdate/images/ramp.png"));
    let other = made.files.iter().find(|f| f.path == "images/noise.png");
    assert_eq!(
        other.map(|f| f.shown.as_str()),
        Some("assets/images/noise.png")
    );
    let bitmap = Bitmap::parse(pack.bytes("images/ramp.png").unwrap_or_default());
    let bitmap = bitmap.unwrap_or_else(|e| panic!("{e}"));
    assert_eq!((bitmap.width(), bitmap.height()), (2, 1));
    assert_eq!(bitmap.style(0), Some(&[0b0100_0000][..]));
}

#[test]
fn a_grey_under_none_is_an_error_naming_the_file_and_no_pack() {
    let grey = layer(
        "assets@playdate",
        vec![(
            "images/ramp.png.meta".to_owned(),
            b"(onebit: (dither: none))".to_vec(),
        )],
    );
    let source = Bundle::for_target(vec![game()], &[grey], Some("playdate"));
    let errors = one_bit()
        .bake(&Formats::standard(), &source, Some("playdate"))
        .err()
        .unwrap_or_default();
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert_eq!(errors[0].file, "assets/images/ramp.png");
    assert!(
        errors[0].message.contains("`dither: none`"),
        "{}",
        errors[0].message
    );
}

#[test]
fn a_bad_section_is_refused_by_the_bake_with_its_file_and_line() {
    let typo = layer(
        "assets@playdate",
        vec![(
            "images/_folder.meta".to_owned(),
            b"(\n    onebit: (dithr: bayer),\n)".to_vec(),
        )],
    );
    let source = Bundle::for_target(vec![game()], &[typo], Some("playdate"));
    let errors = one_bit()
        .bake(&Formats::standard(), &source, Some("playdate"))
        .err()
        .unwrap_or_default();
    let shown: Vec<(&str, Option<u32>)> = errors
        .iter()
        .map(|e| (e.file.as_str(), e.line))
        .filter(|(file, _)| file.ends_with("_folder.meta"))
        .collect();
    assert!(
        shown.contains(&("assets@playdate/images/_folder.meta", Some(2))),
        "{errors:?}"
    );
}

#[test]
fn a_picture_kept_in_its_own_format_is_baked_as_no_other_kind() {
    let source = Bundle::new(vec![game()]);
    let errors = one_bit()
        .under::<ImageInfo>("images")
        .bake(&Formats::standard(), &source, None)
        .err()
        .unwrap_or_default();
    assert_eq!(errors.len(), fixtures().len(), "{errors:?}");
    assert!(
        errors[0].message.contains("as one-bit picture and image;"),
        "{}",
        errors[0].message
    );
}

#[test]
fn the_check_finds_a_baked_picture_that_differs_or_no_format_to_read_it() {
    let source = Bundle::new(vec![game()]);
    let bake = one_bit();
    let (pack, _) = baked(&bake, &source);
    // The pack baked from the ramp's picture with other settings.
    let mut other = OneBitSettings::new(Dither::Bayer);
    other.invert = true;
    let ramp = fixtures().remove(0).1;
    let changed = dither(&ramp, other).unwrap_or_default();
    let changed = layer("changed", vec![("images/ramp.png".to_owned(), changed)]);
    let pack = Bundle::new(vec![changed, pack.layers()[0]]);
    let formats = Formats::standard();
    let errors = bake.check(&formats, &source, &pack);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert_eq!(errors[0].file, "changed/images/ramp.png");
    assert_eq!(errors[0].code, Some(crate::error::codes::PACK_DIFFERS));
    // With no format to read a one-bit file, the check says so.
    let mut none = Formats::new();
    assert_eq!(none.register(Png), Ok(()));
    assert_eq!(none.register(crate::bitmap::dither::Dithered), Ok(()));
    let errors = bake.check(&none, &source, &pack);
    assert!(
        errors.iter().all(|e| e
            .message
            .starts_with("no format reads a baked one-bit picture")),
        "{errors:?}"
    );
    // Registering the bake's formats adds it.
    assert_eq!(bake.register(&mut none), Ok(()));
    assert_eq!(none.ids::<OneBitPicture>(), ["dithered", "one-bit"]);
}
