//! Pictures in ticket 2203's formats (JPEG, WebP, QOI, JPEG XL), painted as
//! a PNG of the same pixels is (ticket 2234).

use shuyi_assets::format::{AssetPath, DecodeCx, Formats};
use shuyi_assets::image::ImageInfo;
use shuyi_ui::frame::{Cell, Layer, Rect, Sprite};

use super::super::*;
use super::{CLEAR, tiny_atlas};

/// A fixture of ticket 2203's, by file name.
macro_rules! fixture {
    ($name:literal) => {
        include_bytes!(concat!("../../../../assets/tests/fixtures/formats/", $name)).as_slice()
    };
}

/// Every picture fixture but the PNG ones: 20 × 12 pixels each.
const FIXTURES: [(&str, &[u8]); 6] = [
    ("images/pic.jpg", fixture!("picture.jpg")),
    ("images/pic.webp", fixture!("picture.webp")),
    ("images/lossless.webp", fixture!("lossless.webp")),
    ("images/pic.qoi", fixture!("picture.qoi")),
    ("images/pic.jxl", fixture!("picture.jxl")),
    ("images/lossless.jxl", fixture!("lossless.jxl")),
];

/// The picture at `path` (20 × 12 px, its file `bytes`) painted whole on a
/// 3 × 1 console of 8 × 16 cells, half-opaque, and flipped, at scale 2.
fn painted(path: &'static str, bytes: &[u8]) -> Image {
    let (atlas, png) = tiny_atlas();
    let table = ImageTable::new([(path, ImageInfo::new(20, 12))].into());
    let read = |p: &str| (p == path).then_some(bytes);
    let painter = Painter::new(atlas, &png, &table, read, CLEAR).unwrap();
    let mut buf = GlyphBuffer::new(3, 1, Cell::new(' ', CLEAR, CLEAR));
    let id = table.id(path).unwrap();
    let mut sprite = Sprite::new(
        id,
        Rect::new(0, 0, 20, 12),
        Rect::new(2, 2, 20, 12),
        Layer::Over,
    );
    sprite.opacity = 200;
    sprite.flip_x = true;
    buf.add_sprite(sprite);
    painter.render(&buf, 2)
}

#[test]
fn each_format_paints_as_a_png_of_its_pixels() {
    let formats = Formats::standard();
    for (path, bytes) in FIXTURES {
        let pixels: Image = formats
            .decode(&AssetPath::new(path), bytes, &DecodeCx::new())
            .unwrap();
        assert_eq!(decode_image(bytes).as_ref(), Ok(&pixels), "{path}");
        let twin = encode_png(pixels.width, pixels.height, &pixels.rgba).unwrap();
        let drawn = painted(path, bytes);
        assert_eq!(drawn, painted("images/twin.png", &twin), "{path}");
        // Not the missing picture's magenta.
        assert_ne!(drawn.pixel(6, 6), Some([255, 0, 255, 255]), "{path}");
    }
}

#[test]
fn a_file_no_format_reads_draws_in_magenta() {
    let drawn = painted("images/pic.bmp", b"BM not a picture we read");
    assert_eq!(drawn.pixel(6, 6), Some([255, 0, 255, 255]));
    let why = decode_image(b"BM not a picture we read").unwrap_err();
    assert!(
        why.starts_with("no image pixels format knows its bytes"),
        "{why}"
    );
    assert!(why.contains("png, jpeg"), "it names the formats: {why}");
}
