//! Key caps as the raster paints them in Terminus 8 × 16, the font the
//! game measured its mockups in (ticket 3215): the margin round a cap's
//! letter, the glyphs beside a cap, and bold from a bold face or struck
//! twice.

use std::collections::BTreeMap;
use std::path::Path;

use shuyi_assets::font::CellWidths;
use shuyi_ui::frame::raster::{Image, Painter};
use shuyi_ui::frame::{Cell, ConsoleSpec, GlyphBuffer, KeyCapStyle, KeyCaps, Rgb};
use shuyi_ui::text::{StyledText, TextRole};

use crate::codec::encode_png;
use crate::font_atlas::{Atlas, AtlasChars, BitmapFont, add_bold, build_atlas, read_font};

const BG: Rgb = Rgb::new(20, 18, 28);
const TEXT: Rgb = Rgb::new(208, 196, 168);
const KEY: Rgb = Rgb::new(250, 230, 190);
const FILL: Rgb = Rgb::new(74, 64, 46);
const EDGE: Rgb = Rgb::new(128, 112, 82);

/// Hello, Grid's Terminus 8 × 16.
fn terminus() -> BitmapFont {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    read_font(&root.join("examples/hello-grid/files-src/ter-u16n.bdf")).unwrap()
}

/// A bold face made of `font` (the engine ships none): each glyph with
/// itself 1 px to the right, the way Terminus Bold thickens its strokes,
/// or, with `moved_only`, just moved 1 px right, a face that draws
/// unlike two strikes.
fn bold_of(font: &BitmapFont, moved_only: bool) -> BitmapFont {
    let w = font.cell_w as usize;
    let glyphs = font.glyphs.iter().map(|(&c, pixels)| {
        let bold = (0..pixels.len())
            .map(|i| {
                let left = i % w > 0 && pixels[i - 1];
                left || (!moved_only && pixels[i])
            })
            .collect();
        (c, bold)
    });
    BitmapFont {
        glyphs: glyphs.collect(),
        ..font.clone()
    }
}

/// Terminus's atlas, with `bold` as its bold face.
fn atlas(bold: Option<&BitmapFont>) -> Atlas {
    let atlas = build_atlas(&terminus(), &AtlasChars::default());
    match bold {
        Some(face) => add_bold(atlas, face).unwrap(),
        None => atlas,
    }
}

/// `text` on a line `cols` cells wide, its keys as caps in `style`, or as
/// text with `None`, painted at scale `scale`.
fn draw(atlas: &Atlas, style: Option<KeyCapStyle>, text: &StyledText, cols: u16, scale: u32) -> Image {
    let widths = CellWidths::of(&atlas.def);
    let spec = ConsoleSpec::text(cols, 1);
    let caps = style.map(|style| KeyCaps::with_colors(style, (Some(FILL), Some(EDGE)), &widths, spec));
    let mut buf = GlyphBuffer::with_spec(spec, Cell::new(' ', TEXT, BG))
        .with_widths(widths)
        .with_key_caps(caps);
    buf.print_styled(0, 0, text, |role| match role {
        Some(TextRole::Key) => (KEY, None),
        _ => (TEXT, Some(BG)),
    });
    Painter::new(atlas.def.clone(), atlas.image.clone(), BTreeMap::new(), BG).render(&buf, scale)
}

fn key(name: &str) -> StyledText {
    let mut text = StyledText::new();
    text.push_key(name);
    text
}

/// The smallest box round the pixels of `image` in `color`:
/// `(left, top, right, bottom)`.
fn bounds(image: &Image, color: Rgb) -> (u32, u32, u32, u32) {
    let want = [color.r, color.g, color.b, 255];
    let mut found = (u32::MAX, u32::MAX, 0, 0);
    for y in 0..image.height {
        for x in 0..image.width {
            if image.pixel(x, y) == Some(want) {
                found = (found.0.min(x), found.1.min(y), found.2.max(x), found.3.max(y));
            }
        }
    }
    assert!(found.0 <= found.2, "no pixel in {color:?}");
    found
}

/// Pixels between the ink and the inside of the box's edge: left, top,
/// right, bottom.
fn margins(image: &Image) -> [u32; 4] {
    let (el, et, er, eb) = bounds(image, EDGE);
    let (il, it, ir, ib) = bounds(image, KEY);
    [il - el - 1, it - et - 1, er - ir - 1, eb - ib - 1]
}

#[test]
fn a_one_letter_cap_has_two_pixels_between_its_ink_and_its_box_on_every_side() {
    let atlas = atlas(None);
    for name in ["w", "d", "f", "e", "W", "D", "F", "E"] {
        let image = draw(&atlas, Some(KeyCapStyle::new()), &key(name), 4, 1);
        assert_eq!(margins(&image), [2; 4], "{name}");
        let plain = KeyCapStyle::new().with_bold(false);
        let image = draw(&atlas, Some(plain), &key(name), 4, 1);
        assert_eq!(margins(&image), [2; 4], "{name}, not bold");
    }
}

#[test]
fn a_full_stop_comma_or_colon_after_a_cap_is_drawn_as_without_it() {
    let atlas = atlas(None);
    for mark in [".", ",", ":", " "] {
        let mut capped = key("f");
        capped.push_plain(mark);
        let with = draw(&atlas, Some(KeyCapStyle::new()), &capped, 4, 1);
        // Without caps, the mark printed in the same cell, the third.
        let mut moved = StyledText::new();
        moved.push_plain("  ").push_plain(mark);
        let without = draw(&atlas, None, &moved, 4, 1);
        for y in 0..16 {
            for x in 16..24 {
                assert_eq!(with.pixel(x, y), without.pixel(x, y), "{mark:?} at {x},{y}");
            }
        }
    }
}

#[test]
fn bold_comes_from_the_bold_face_or_two_strikes() {
    let regular = draw(&atlas(None), Some(KeyCapStyle::new().with_bold(false)), &key("F"), 4, 1);
    let struck = draw(&atlas(None), Some(KeyCapStyle::new()), &key("F"), 4, 1);
    let moved = bold_of(&terminus(), true);
    let face = draw(&atlas(Some(&moved)), Some(KeyCapStyle::new()), &key("F"), 4, 1);
    let width = |image: &Image| {
        let (left, _, right, _) = bounds(image, KEY);
        right - left + 1
    };
    // F's ink is 6 px; struck twice 7; the face (moved, not thicker) 6.
    assert_eq!((width(&regular), width(&struck), width(&face)), (6, 7, 6));
    // Each is placed by its own ink: 2 px all round.
    assert_eq!(margins(&face), [2; 4]);
    assert_eq!(margins(&struck), [2; 4]);
}

#[test]
fn without_caps_a_key_is_text() {
    let mut line = key("f");
    line.push_plain(" select");
    let image = draw(&atlas(None), None, &line, 9, 1);
    let edge = [EDGE.r, EDGE.g, EDGE.b, 255];
    let any_edge = (0..image.height).any(|y| (0..image.width).any(|x| image.pixel(x, y) == Some(edge)));
    assert!(!any_edge);
    // The key's lowercase `f`, in its own cell on the grid.
    let (left, _, right, _) = bounds(&image, KEY);
    assert!(left < right && right < 8, "{left}..{right}");
}

/// What a test can't judge: the pictures a reviewer looks at, written to
/// `target/key-caps/` (`cargo test -p shuyi-cli key_caps_pictures --
/// --ignored`): a help line, a text box's corner and a line of prose,
/// without caps, with caps struck twice and with a bold face.
#[test]
#[ignore = "writes pictures for a reviewer"]
fn key_caps_pictures() {
    let line = |parts: &[(&str, bool)]| {
        let mut text = StyledText::new();
        for &(part, key) in parts {
            if key {
                text.push_key(part);
            } else {
                text.push_plain(part);
            }
        }
        text
    };
    let lines = [
        line(&[("f", true), (" select · ", false), ("Space", true), (" end turn · ", false)]),
        line(&[("Shift+Space", true), (" skip · ", false), ("b", true), (" log  ", false), ("f", true), (" ▼", false)]),
        line(&[("Press ", false), ("e", true), (" to talk, then ", false), ("Enter", true), (".", false)]),
    ];
    let thick = bold_of(&terminus(), false);
    let looks = [
        ("off", atlas(None), None),
        ("struck", atlas(None), Some(KeyCapStyle::new())),
        ("face", atlas(Some(&thick)), Some(KeyCapStyle::new())),
    ];
    let out = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/key-caps");
    std::fs::create_dir_all(&out).unwrap();
    for (name, atlas, style) in &looks {
        let images: Vec<Image> = lines.iter().map(|text| draw(atlas, *style, text, 44, 3)).collect();
        let width = images[0].width;
        let rgba: Vec<u8> = images.iter().flat_map(|image| image.rgba.iter().copied()).collect();
        let height = images.iter().map(|image| image.height).sum();
        let png = encode_png(width, height, &rgba).unwrap();
        std::fs::write(out.join(format!("{name}.png")), png).unwrap();
    }
}
