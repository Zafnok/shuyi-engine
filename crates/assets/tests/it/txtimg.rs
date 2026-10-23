//! A toy format plugin, `*.txtimg` (`docs/formats.md`): a picture written
//! as text, one character a pixel, made only for these tests. It holds
//! what a plugin crate holds: a format for each kind it reads, and an
//! encoder.
//!
//! ```text
//! txtimg 3 2
//! #.r
//! kgb
//! ```
//!
//! The first line is `txtimg`, the width and the height. Then come
//! `height` rows of `width` pixels: `.` see-through, `#` white, `k` black,
//! `r` red, `g` green, `b` blue.

use proptest::prelude::*;
use shuyi_assets::format::{AssetPath, DecodeCx, Encode, Format, Formats, TEXT_MAX_BYTES, text};
use shuyi_assets::image::{Image, ImageId, ImageInfo, ImageTable};
use shuyi_assets::{Bundle, ContentError, Layer};

/// The first word of every text image.
const MAGIC: &str = "txtimg";

/// Each pixel's character and colour.
const PIXELS: [(char, [u8; 4]); 6] = [
    ('.', [0, 0, 0, 0]),
    ('#', [255, 255, 255, 255]),
    ('k', [0, 0, 0, 255]),
    ('r', [255, 0, 0, 255]),
    ('g', [0, 255, 0, 255]),
    ('b', [0, 0, 255, 255]),
];

/// A text image, read for its pixels (`txtimg`, `.txtimg`).
struct TxtImg;

/// A text image, read for its size (`txtimg`, `.txtimg`): what the image
/// table reads.
struct TxtImgInfo;

impl Format for TxtImg {
    type Asset = Image;

    fn id(&self) -> &'static str {
        "txtimg"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["txtimg"]
    }

    fn sniff(&self, bytes: &[u8]) -> bool {
        bytes.starts_with(b"txtimg ")
    }

    fn max_bytes(&self) -> usize {
        TEXT_MAX_BYTES
    }

    fn decode(
        &self,
        file: &AssetPath,
        bytes: &[u8],
        _cx: &DecodeCx<'_>,
    ) -> Result<Image, Vec<ContentError>> {
        read(file, bytes)
    }
}

impl Format for TxtImgInfo {
    type Asset = ImageInfo;

    fn id(&self) -> &'static str {
        TxtImg.id()
    }

    fn extensions(&self) -> &'static [&'static str] {
        TxtImg.extensions()
    }

    fn sniff(&self, bytes: &[u8]) -> bool {
        TxtImg.sniff(bytes)
    }

    fn max_bytes(&self) -> usize {
        TxtImg.max_bytes()
    }

    fn decode(
        &self,
        file: &AssetPath,
        bytes: &[u8],
        _cx: &DecodeCx<'_>,
    ) -> Result<ImageInfo, Vec<ContentError>> {
        read(file, bytes).map(|image| image.info())
    }
}

impl Encode for TxtImg {
    /// An image whose pixels are all among the six; any other pixel comes
    /// back see-through.
    fn encode(&self, image: &Image) -> Vec<u8> {
        let mut out = format!("{MAGIC} {} {}\n", image.width, image.height);
        let mut pixels = image.rgba.chunks(4);
        for _ in 0..image.height {
            for _ in 0..image.width {
                let pixel = pixels.next().unwrap_or_default();
                let found = PIXELS.iter().find(|(_, colour)| colour[..] == *pixel);
                out.push(found.map_or('.', |&(c, _)| c));
            }
            out.push('\n');
        }
        out.into_bytes()
    }
}

/// The text image in `bytes` (the file `file`), or every problem with it,
/// each at its line.
fn read(file: &AssetPath, bytes: &[u8]) -> Result<Image, Vec<ContentError>> {
    let source = text(file, bytes)?;
    let at = |line: usize, message: String| {
        let line = u32::try_from(line).unwrap_or(u32::MAX);
        ContentError::new(file.shown(), message).at(line, None)
    };
    let mut lines = source.lines();
    let header: Vec<&str> = lines.next().unwrap_or_default().split(' ').collect();
    let size = match header[..] {
        [MAGIC, width, height] => width.parse().ok().zip(height.parse().ok()),
        _ => None,
    };
    let Some((width, height)) = size else {
        let message = format!("the first line must be \"{MAGIC} <width> <height>\"");
        return Err(vec![at(1, message)]);
    };
    let info = ImageInfo::checked(width, height).map_err(|message| vec![at(1, message)])?;
    let rows: Vec<&str> = lines.collect();
    let mut errors = Vec::new();
    if rows.len() != usize::try_from(height).unwrap_or(usize::MAX) {
        let message = format!("the picture is {height} rows; this one has {}", rows.len());
        errors.push(at(1, message));
    }
    let mut rgba = Vec::new();
    for (i, row) in rows.iter().enumerate() {
        let line = i + 2;
        let pixels = row.chars().count();
        if pixels != usize::try_from(width).unwrap_or(usize::MAX) {
            errors.push(at(
                line,
                format!("a row is {width} pixels; this one is {pixels}"),
            ));
        }
        for c in row.chars() {
            match PIXELS.iter().find(|&&(p, _)| p == c) {
                Some((_, colour)) => rgba.extend(colour),
                None => errors.push(at(line, format!("'{c}' is no pixel; they are . # k r g b"))),
            }
        }
    }
    if errors.is_empty() {
        Ok(Image {
            width: info.width,
            height: info.height,
            rgba,
        })
    } else {
        Err(errors)
    }
}

/// A bundle of `files`, shown under `base`.
fn bundle(files: &'static [(&'static str, &'static [u8])]) -> Bundle {
    Bundle::new(vec![Layer::table("base", files)])
}

/// A 2 × 1 PNG: a red pixel and a see-through one.
fn png_card() -> Vec<u8> {
    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, 2, 1);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap_or_else(|e| panic!("{e}"));
        writer
            .write_image_data(&RED_THEN_CLEAR)
            .unwrap_or_else(|e| panic!("{e}"));
    }
    out
}

/// A red pixel, then a see-through one.
const RED_THEN_CLEAR: [u8; 8] = [255, 0, 0, 255, 0, 0, 0, 0];

/// The same card as a text image.
const TEXT_CARD: &[u8] = b"txtimg 2 1\nr.\n";

/// The code that asks for a picture: it names it by id, never by its
/// file, and doesn't change whichever format the file is in.
fn card(formats: &Formats, bundle: &Bundle) -> Result<Image, Vec<ContentError>> {
    formats.load::<Image>(bundle, "images/card")
}

/// The code that asks for every image's size: the image table.
fn sizes(formats: &Formats, bundle: &Bundle) -> Vec<(&'static str, ImageInfo)> {
    let table = ImageTable::load_with(bundle, formats, &[]).unwrap_or_default();
    let size = |id: ImageId| table.info(id).map(|info| (id.path(), info));
    table.ids().filter_map(size).collect()
}

/// The toy's two formats, registered beside the engine's.
fn with_txtimg() -> Formats {
    let mut formats = Formats::standard();
    assert_eq!(formats.register(TxtImg), Ok(()));
    assert_eq!(formats.register(TxtImgInfo), Ok(()));
    formats
}

#[test]
fn a_registered_format_reads_an_image_with_no_change_to_the_code_that_asks_for_it() {
    let texts = bundle(&[("images/card.txtimg", TEXT_CARD)]);
    let pngs = bundle(Box::leak(Box::new([(
        "images/card.png",
        &*Box::leak(png_card().into_boxed_slice()),
    )])));
    let expected = Image {
        width: 2,
        height: 1,
        rgba: RED_THEN_CLEAR.to_vec(),
    };
    let size = ImageInfo::new(2, 1);
    // The engine's own formats read the PNG, and know no text image.
    let standard = Formats::standard();
    assert_eq!(card(&standard, &pngs), Ok(expected.clone()));
    let missing: Vec<String> = card(&standard, &texts)
        .err()
        .unwrap_or_default()
        .iter()
        .map(ToString::to_string)
        .collect();
    assert_eq!(
        missing,
        ["base/images/card: file not found in asset bundle, nor images/card.png"]
    );
    assert_eq!(sizes(&standard, &texts), []);
    // With the toy registered, the same code reads either.
    let formats = with_txtimg();
    assert_eq!(card(&formats, &texts), Ok(expected.clone()));
    assert_eq!(card(&formats, &pngs), Ok(expected));
    assert_eq!(sizes(&formats, &texts), [("images/card.txtimg", size)]);
    assert_eq!(sizes(&formats, &pngs), [("images/card.png", size)]);
}

#[test]
fn a_text_image_at_a_pngs_path_is_known_by_its_first_word() {
    let disguised = bundle(&[("images/card.png", TEXT_CARD)]);
    let read = with_txtimg().load::<Image>(&disguised, "images/card.png");
    assert_eq!(read.map(|image| image.rgba), Ok(RED_THEN_CLEAR.to_vec()));
}

#[test]
fn a_second_text_image_format_is_refused_unless_it_replaces_the_first() {
    let mut formats = with_txtimg();
    let error = formats.register(TxtImg).map_err(|e| e.to_string());
    assert_eq!(
        error,
        Err(
            "image pixels format \"txtimg\" is registered already; replace it to use the new one"
                .to_owned()
        )
    );
    assert_eq!(formats.replace(TxtImg), Ok(vec!["txtimg"]));
    assert_eq!(formats.ids::<Image>(), ["png", "txtimg"]);
}

/// What the toy reads `source` as, or its errors as text.
fn errors(source: &str) -> Vec<String> {
    let file = AssetPath::new("images/card.txtimg");
    let read = TxtImg.decode(&file, source.as_bytes(), &DecodeCx::new());
    read.err()
        .unwrap_or_default()
        .iter()
        .map(ToString::to_string)
        .collect()
}

#[test]
fn a_broken_text_image_names_the_line_of_each_problem() {
    let file = "images/card.txtimg";
    assert_eq!(
        errors("txtimg 2"),
        [format!(
            "{file}:1: the first line must be \"txtimg <width> <height>\""
        )]
    );
    assert_eq!(
        errors("txtimg 0 1\n"),
        [format!(
            "{file}:1: image is 0×1 px; both sides must be greater than 0"
        )]
    );
    assert_eq!(
        errors("txtimg 2 2\n#.\n#x\n."),
        [
            format!("{file}:1: the picture is 2 rows; this one has 3"),
            format!("{file}:3: '{}' is no pixel; they are . # k r g b", 'x'),
            format!("{file}:4: a row is 2 pixels; this one is 1"),
        ]
    );
    assert_eq!(
        errors("txtimg 1 1\n\u{ff}"),
        [format!(
            "{file}:2: '\u{ff}' is no pixel; they are . # k r g b"
        )]
    );
}

/// An image of `width × height` pixels among the six.
fn images() -> impl Strategy<Value = Image> {
    (1..8_u32, 1..8_u32).prop_flat_map(|(width, height)| {
        let count = usize::try_from(width * height).unwrap_or_default();
        let pixel = prop::sample::select(PIXELS.map(|(_, colour)| colour).to_vec());
        proptest::collection::vec(pixel, count).prop_map(move |pixels| Image {
            width,
            height,
            rgba: pixels.concat(),
        })
    })
}

proptest! {
    #[test]
    fn a_text_image_encoded_reads_back_the_same(image in images()) {
        let bytes = TxtImg.encode(&image);
        let file = AssetPath::new("a.txtimg");
        prop_assert_eq!(TxtImg.decode(&file, &bytes, &DecodeCx::new()), Ok(image));
    }

    #[test]
    fn no_bytes_panic_the_text_image_format(
        random in proptest::collection::vec(any::<u8>(), 0..200),
        shaped in "txtimg [0-9]{1,2} [0-9]{1,2}\n([.#krgbx]{0,9}\n){0,9}",
    ) {
        let file = AssetPath::new("a.txtimg");
        for bytes in [random.as_slice(), shaped.as_bytes()] {
            let _ = TxtImg.decode(&file, bytes, &DecodeCx::new());
            let _ = TxtImgInfo.decode(&file, bytes, &DecodeCx::new());
        }
    }
}
