//! The render card (ADR-0052 §6, ticket 2392): one fixed frame with every
//! kind of thing a frame draws, so a backend's window is held to the
//! raster for all of them, with no game behind it. `shuyi-app`'s
//! `render_card` example draws it in a window and compares the window
//! with [`RenderCard::painter`]'s picture, in CI's `render` job.
//!
//! The card holds:
//!
//! - cells: backgrounds in several colours, see-through cells, glyphs in
//!   several colours, and a glyph the atlas lacks (the fallback, in
//!   [`MISSING_COLOR`](super::MISSING_COLOR));
//! - rectangles and sprites in both layers; sprites in every [`Paint`], at
//!   opacity 255 and 128, at brightness 255 and 102, mirrored, cut by
//!   their `clip`, and one whose image is missing;
//! - a backdrop at zoom 2, its scene offset by half a scene pixel.
//!
//! Its pictures are made in code: the [test card](super::test_card) and a
//! small font atlas of boxes and bars. A new `Item`, `Paint` or `Layer`
//! variant goes on the card in the change that adds it: the tests don't
//! compile until it does.

use alloc::collections::BTreeMap;
use alloc::rc::Rc;

use shuyi_assets::font::{FALLBACK_GLYPH, FontAtlasDef};
use shuyi_assets::image::{ImageId, ImageInfo, ImageTable};
use shuyi_assets::lighting::Light;

use super::{Image, Painter, test_card};
use crate::frame::color::Rgb;
use crate::frame::glyph_buffer::{Cell, GlyphBuffer, Layer, Overlay, Paint, Rect, Sprite};

/// The card's size in cells (8 × 16 pixels each): 256 × 192 pixels.
pub const COLUMNS: u16 = 32;
/// See [`COLUMNS`].
pub const ROWS: u16 = 12;

/// The test card's path in [`RenderCard::table`].
pub const TEST_CARD_PATH: &str = "card/test_card.png";
/// The path of the picture that is missing: in [`RenderCard::table`], not
/// in [`RenderCard::images`].
pub const MISSING_PATH: &str = "card/missing.png";

/// The console's background.
const CLEAR: Rgb = Rgb::new(20, 18, 28);
/// Cell backgrounds.
const BACKGROUNDS: [Rgb; 4] = [
    Rgb::new(40, 60, 120),
    Rgb::new(120, 40, 50),
    Rgb::new(30, 100, 60),
    Rgb::new(150, 140, 40),
];
/// Glyph colours.
const FOREGROUNDS: [Rgb; 5] = [
    Rgb::new(255, 255, 255),
    Rgb::new(250, 200, 60),
    Rgb::new(90, 220, 240),
    Rgb::new(240, 90, 200),
    Rgb::new(10, 10, 10),
];
/// The glyphs the atlas has, but the space.
const GLYPHS: [char; 4] = ['#', 'o', '-', '|'];
/// A glyph the atlas lacks.
const MISSING_GLYPH: char = 'Z';
/// The sprites' gallery: two background colours, a cell each, in turn.
const STRIPES: [Rgb; 2] = [Rgb::new(70, 70, 80), Rgb::new(200, 190, 170)];

/// One fixed frame with every kind of thing a frame draws, and what
/// paints it.
#[derive(Debug, Clone, PartialEq)]
pub struct RenderCard {
    /// The frame.
    pub buffer: GlyphBuffer,
    /// The font atlas: boxes and bars, and the fallback glyph.
    pub atlas: FontAtlasDef,
    /// The atlas's picture.
    pub atlas_image: Image,
    /// The pictures' table: the test card and the missing picture.
    pub table: ImageTable,
    /// The pictures there are: the test card alone.
    pub images: BTreeMap<ImageId, Image>,
    /// The console's background.
    pub clear: Rgb,
}

impl RenderCard {
    /// The raster's painter of the card's frame.
    pub fn painter(&self) -> Painter {
        Painter::new(
            self.atlas.clone(),
            self.atlas_image.clone(),
            self.images.clone(),
            self.clear,
        )
    }
}

/// The render card.
pub fn build() -> RenderCard {
    let table = ImageTable::new(BTreeMap::from([
        (
            TEST_CARD_PATH,
            ImageInfo::new(test_card::SIZE, test_card::SIZE),
        ),
        (
            MISSING_PATH,
            ImageInfo::new(test_card::SIZE, test_card::SIZE),
        ),
    ]));
    let ids = table.id(TEST_CARD_PATH).zip(table.id(MISSING_PATH));
    let Some((card, missing)) = ids else {
        unreachable!("the table names both pictures");
    };
    let (atlas, atlas_image) = atlas();
    let card_image = Image {
        width: test_card::SIZE,
        height: test_card::SIZE,
        rgba: test_card::pixels(),
    };
    RenderCard {
        buffer: frame(card, missing),
        atlas,
        atlas_image,
        images: BTreeMap::from([(card, card_image)]),
        table,
        clear: CLEAR,
    }
}

/// A 48 × 16 atlas of six 8 × 16 cells: the space, a box (`#`), a box's
/// outline (`o`), a bar across (`-`), a bar down (`|`), and the fallback
/// glyph, a checkerboard. Glyph pixels are white, the rest see-through.
fn atlas() -> (FontAtlasDef, Image) {
    const CELL: (u32, u32) = (8, 16);
    let order = [' ', '#', 'o', '-', '|', FALLBACK_GLYPH];
    let glyphs: BTreeMap<char, u32> = order.iter().copied().zip(0..).collect();
    let columns = u32::try_from(order.len()).unwrap_or(0);
    let (width, height) = (CELL.0 * columns, CELL.1);
    // Whether pixel `(x, y)` of glyph `glyph`'s cell is set.
    let set = |glyph: char, x: u32, y: u32| match glyph {
        '#' => (1..7).contains(&x) && (2..14).contains(&y),
        'o' => {
            let (across, down) = ((1..7).contains(&x), (3..13).contains(&y));
            across && down && (x == 1 || x == 6 || y == 3 || y == 12)
        }
        '-' => (7..9).contains(&y),
        '|' => (3..5).contains(&x),
        FALLBACK_GLYPH => (1..7).contains(&x) && (2..14).contains(&y) && (x + y).is_multiple_of(2),
        _ => false,
    };
    let rgba = (0..height)
        .flat_map(|y| (0..width).map(move |x| (x, y)))
        .flat_map(|(x, y)| {
            let on = set(order[(x / CELL.0) as usize], x % CELL.0, y);
            [if on { 255 } else { 0 }; 4]
        })
        .collect();
    let def = FontAtlasDef {
        cell_w: CELL.0,
        cell_h: CELL.1,
        columns,
        glyphs,
    };
    (
        def,
        Image {
            width,
            height,
            rgba,
        },
    )
}

/// The card's frame, its sprites showing `card`, but one showing
/// `missing`.
fn frame(card: ImageId, missing: ImageId) -> GlyphBuffer {
    let mut buf = GlyphBuffer::new(COLUMNS, ROWS, Cell::new(' ', FOREGROUNDS[0], CLEAR));
    cells(&mut buf);
    rects(&mut buf);
    sprites(&mut buf, card, missing);
    backdrop(&mut buf, card);
    buf
}

/// The cells: a band of backgrounds and glyphs in every colour (row 0),
/// glyphs on the clear colour and the glyph the atlas lacks (row 1), the
/// sprites' gallery on stripes (rows 2 to 10, columns 0 to 19), glyphs
/// over the `Under` sprites (row 6), and see-through cells where there is
/// no backdrop (row 11).
fn cells(buf: &mut GlyphBuffer) {
    for x in 0..i32::from(COLUMNS) {
        let i = nth(x);
        let glyph = GLYPHS[i % GLYPHS.len()];
        let fg = FOREGROUNDS[i % FOREGROUNDS.len()];
        buf.set(x, 0, Cell::new(glyph, fg, BACKGROUNDS[i / 4 % 4]));
        let glyph = if i.is_multiple_of(3) { MISSING_GLYPH } else { glyph };
        buf.set(x, 1, Cell::new(glyph, fg, CLEAR));
    }
    for y in 2..11 {
        for x in 0..20 {
            let stripe = STRIPES[nth(x + y) % 2];
            buf.set(x, y, Cell::new(' ', FOREGROUNDS[0], stripe));
        }
    }
    for x in 0..8 {
        let i = nth(x);
        let stripe = STRIPES[nth(x + 6) % 2];
        buf.set(x, 6, Cell::new(GLYPHS[i % 4], FOREGROUNDS[i % 5], stripe));
    }
    for x in 0..8 {
        let glyph = if x % 2 == 0 { 'o' } else { ' ' };
        buf.set(x, 11, Cell::see_through(glyph, FOREGROUNDS[2]));
    }
    buf.set(8, 11, Cell::see_through(MISSING_GLYPH, FOREGROUNDS[1]));
}

/// The rectangles: an `Under` bar through row 1's glyphs, `Over` squares
/// on row 0's, a one-pixel `Over` line, and an `Over` square on a sprite.
fn rects(buf: &mut GlyphBuffer) {
    let red = Rgb::new(230, 40, 40);
    buf.add_overlay(Overlay::new(Rect::new(0, 23, 256, 3), red, Layer::Under));
    for (i, color) in BACKGROUNDS.iter().enumerate() {
        let x = i32::try_from(i).unwrap_or(0) * 32 + 5;
        buf.add_overlay(Overlay::new(Rect::new(x, 6, 5, 3), *color, Layer::Over));
    }
    let white = Rgb::new(255, 255, 255);
    buf.add_overlay(Overlay::new(Rect::new(161, 33, 1, 140), white, Layer::Over));
    let green = Rgb::new(40, 220, 90);
    buf.add_overlay(Overlay::new(Rect::new(30, 40, 4, 7), green, Layer::Over));
}

/// The sprites' gallery, the test card each time:
///
/// - y 36: once each in the image's colours `Under` and `Over`, `Solid`,
///   `Dimmed`, `Shaded`, mirrored, and the missing picture;
/// - y 60: the same, at opacity 128 or brightness 102, or both;
/// - y 98: twice the size, `Under` row 6's glyphs, then cut by a `clip`
///   on odd pixels, mirrored and cut, and `Solid` and cut.
fn sprites(buf: &mut GlyphBuffer, card: ImageId, missing: ImageId) {
    let full = Rect::new(0, 0, 16, 16);
    let dusk = Paint::Shaded(Light::new(0.4, [0.7, 0.75, 1.0]));
    let gold = Paint::Solid(Rgb::new(250, 200, 60));
    let at = |x: i32, y: i32, size: i32| Rect::new(x, y, size, size);
    // (picture, paint, layer, flip_x, opacity, brightness)
    let small = [
        (card, Paint::Image, Layer::Under, false, 255, 255),
        (card, Paint::Image, Layer::Over, false, 255, 255),
        (card, gold, Layer::Over, false, 255, 255),
        (card, Paint::Dimmed, Layer::Over, false, 255, 255),
        (card, dusk, Layer::Over, false, 255, 255),
        (card, Paint::Image, Layer::Over, true, 255, 255),
        (missing, Paint::Image, Layer::Over, false, 255, 255),
    ];
    let faded = [
        (card, Paint::Image, Layer::Over, false, 128, 255),
        (card, gold, Layer::Over, false, 128, 255),
        (card, Paint::Image, Layer::Over, false, 255, 102),
        (card, gold, Layer::Over, false, 128, 102),
        (card, Paint::Dimmed, Layer::Over, false, 255, 102),
        (card, Paint::Image, Layer::Under, true, 128, 255),
        (card, dusk, Layer::Over, false, 128, 102),
    ];
    for (y, row) in [(36, small), (60, faded)] {
        for (x, (image, paint, layer, flip_x, opacity, brightness)) in (4..).step_by(22).zip(row) {
            let mut sprite = Sprite::new(image, full, at(x, y, 16), layer).painted(paint);
            sprite.flip_x = flip_x;
            sprite.opacity = opacity;
            sprite.brightness = brightness;
            buf.add_sprite(sprite);
        }
    }
    // Twice the size, from y 98: the glyphs of row 6 (y 96 to 112) over
    // the first, and the others cut on odd pixels.
    let big = |x: i32, layer| Sprite::new(card, full, at(x, 98, 32), layer);
    buf.add_sprite(big(4, Layer::Under));
    let mut cut = big(42, Layer::Over);
    cut.clip = Rect::new(47, 101, 21, 23);
    buf.add_sprite(cut);
    let mut cut = big(80, Layer::Over);
    cut.clip = Rect::new(85, 101, 21, 23);
    cut.flip_x = true;
    buf.add_sprite(cut);
    let mut cut = big(118, Layer::Under).painted(Paint::Solid(Rgb::new(90, 220, 240)));
    cut.clip = Rect::new(121, 105, 25, 15);
    buf.add_sprite(cut);
}

/// The backdrop: a scene of 6 × 5 cells, zoom 2, behind cells 21 to 30
/// of rows 2 to 10, its scene pixel (0.5, 0.5) at the window's top-left.
/// The cells there are see-through, some with glyphs, but a few solid
/// ones: one in the clear colour, which hides the scene too.
fn backdrop(buf: &mut GlyphBuffer, card: ImageId) {
    let mut scene = GlyphBuffer::new(6, 5, Cell::new(' ', FOREGROUNDS[0], CLEAR));
    for y in 0..5 {
        for x in 0..6 {
            let i = nth(x + y * 6);
            if i % 5 == 4 {
                continue; // The clear colour.
            }
            let glyph = if i.is_multiple_of(2) { GLYPHS[i % 4] } else { ' ' };
            let cell = Cell::new(glyph, FOREGROUNDS[i % 5], BACKGROUNDS[i % 4]);
            scene.set(x, y, cell);
        }
    }
    let full = Rect::new(0, 0, 16, 16);
    scene.add_sprite(Sprite::new(
        card,
        full,
        Rect::new(12, 20, 16, 16),
        Layer::Over,
    ));
    let line = Overlay::new(Rect::new(3, 45, 30, 2), Rgb::new(230, 40, 40), Layer::Under);
    scene.add_overlay(line);
    let clip = Rect::new(21, 2, 10, 9);
    for y in clip.y..clip.y + clip.h {
        for x in clip.x..clip.x + clip.w {
            let i = nth(x + y);
            let glyph = if i.is_multiple_of(3) { GLYPHS[i % 4] } else { ' ' };
            buf.set(x, y, Cell::see_through(glyph, FOREGROUNDS[i % 4]));
        }
    }
    buf.set(23, 4, Cell::new('o', FOREGROUNDS[1], BACKGROUNDS[1]));
    buf.set(24, 4, Cell::new('#', FOREGROUNDS[2], CLEAR));
    buf.set(27, 8, Cell::new(' ', FOREGROUNDS[0], BACKGROUNDS[3]));
    buf.set_backdrop(Rc::new(scene), clip, (0.5, 0.5), 2);
}

/// `v` as an index into a list of colours or glyphs (0 if negative).
fn nth(v: i32) -> usize {
    usize::try_from(v).unwrap_or(0)
}

#[cfg(test)]
mod tests;
