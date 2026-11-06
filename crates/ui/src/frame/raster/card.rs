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
//!   [`MISSING_COLOR`](super::MISSING_COLOR)); wide glyphs among narrow
//!   ones, each over its two cells, and one without its continuation
//!   (the fallback too);
//! - rectangles and sprites in both layers; sprites in every [`Paint`], at
//!   opacity 255 and 128, at brightness 255 and 102, mirrored, cut by
//!   their `clip`, and one whose image is missing;
//! - painted pictures (ADR-0078) of every kind, RGBA (some pixels clear
//!   or half clear), indexed and one-bit (with a mask and without), in
//!   both layers, at brightness 102, cut by a `clip`, part of one, and one
//!   the frame's table lacks;
//! - a backdrop at zoom 2, its scene offset by half a scene pixel, with
//!   a painted picture of its own;
//! - the debug view's gizmos (ADR-0084) over everything: lines into all
//!   eight octants, rectangles' outlines (one cut by a `clip`), crosses,
//!   circles' outlines (one of radius 0) and labels in the cells.
//!
//! Its pictures are made in code: the [test card](super::test_card) and a
//! small font atlas of boxes and bars, and a wide glyph, and the painted
//! pictures' pixels. A new `Item`, `Paint`, `Layer` or `PaintedKind`
//! variant goes on the card in the change that adds it: the tests don't
//! compile until it does.

use alloc::collections::BTreeMap;
use alloc::format;
use alloc::rc::Rc;

use shuyi_assets::font::{CellWidths, FALLBACK_GLYPH, FontAtlasDef};
use shuyi_assets::image::{ImageId, ImageInfo, ImageTable};
use shuyi_assets::lighting::Light;

use super::{Image, Painter, test_card};
use crate::frame::color::Rgb;
use crate::frame::gizmo::{GizmoItem, GizmoRole, GizmoShape, LabelText};
use crate::frame::glyph_buffer::{Cell, GlyphBuffer, Layer, Overlay, Paint, Rect, Sprite};
use crate::frame::painted::{Painted, PaintedId, PaintedSprite};

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
/// The atlas's wide glyph: a box with a bar down left of its middle, so
/// its two halves differ.
const WIDE_GLYPH: char = '田';
/// The sprites' gallery: two background colours, a cell each, in turn.
const STRIPES: [Rgb; 2] = [Rgb::new(70, 70, 80), Rgb::new(200, 190, 170)];

/// The painted RGBA picture.
pub const PAINTED_RGBA: PaintedId = PaintedId::new("card/rgba");
/// The painted indexed picture.
pub const PAINTED_INDEXED: PaintedId = PaintedId::new("card/indexed");
/// The painted one-bit picture with a mask.
pub const PAINTED_MASKED: PaintedId = PaintedId::new("card/masked");
/// The painted one-bit picture without a mask.
pub const PAINTED_ONE_BIT: PaintedId = PaintedId::new("card/one-bit");
/// The backdrop scene's painted picture.
pub const PAINTED_SCENE: PaintedId = PaintedId::new("card/scene");
/// A painted picture the frame's table lacks.
pub const PAINTED_MISSING: PaintedId = PaintedId::new("card/painted-missing");

/// One fixed frame with every kind of thing a frame draws, and what
/// paints it.
#[derive(Debug, Clone, PartialEq)]
pub struct RenderCard {
    /// The frame.
    pub buffer: GlyphBuffer,
    /// The font atlas: boxes and bars, the fallback glyph and a wide one.
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

/// A 64 × 16 atlas of eight 8 × 16 cells: the space, a box (`#`), a box's
/// outline (`o`), a bar across (`-`), a bar down (`|`), the fallback
/// glyph, a checkerboard, and the wide glyph over the last two. Glyph
/// pixels are white, the rest see-through.
fn atlas() -> (FontAtlasDef, Image) {
    const CELL: (u32, u32) = (8, 16);
    let order = [' ', '#', 'o', '-', '|', FALLBACK_GLYPH];
    let glyphs: BTreeMap<char, u32> = order.iter().copied().zip(0..).collect();
    let narrow = u32::try_from(order.len()).unwrap_or(0);
    let columns = narrow + 2;
    let (width, height) = (CELL.0 * columns, CELL.1);
    // Whether pixel `(x, y)` of glyph `glyph`'s cell (both cells of the
    // wide one) is set.
    let set = |glyph: char, x: u32, y: u32| match glyph {
        '#' => (1..7).contains(&x) && (2..14).contains(&y),
        'o' => {
            let (across, down) = ((1..7).contains(&x), (3..13).contains(&y));
            across && down && (x == 1 || x == 6 || y == 3 || y == 12)
        }
        '-' => (7..9).contains(&y),
        '|' => (3..5).contains(&x),
        FALLBACK_GLYPH => (1..7).contains(&x) && (2..14).contains(&y) && (x + y).is_multiple_of(2),
        WIDE_GLYPH => {
            let (across, down) = ((1..15).contains(&x), (2..14).contains(&y));
            across && down && (x == 1 || x == 14 || x == 5 || y == 2 || y == 13 || y == 7)
        }
        _ => false,
    };
    let rgba = (0..height)
        .flat_map(|y| (0..width).map(move |x| (x, y)))
        .flat_map(|(x, y)| {
            let cell = x / CELL.0;
            let on = match order.get(cell as usize) {
                Some(&glyph) => set(glyph, x % CELL.0, y),
                None => set(WIDE_GLYPH, x - narrow * CELL.0, y),
            };
            [if on { 255 } else { 0 }; 4]
        })
        .collect();
    let def = FontAtlasDef {
        cell_w: CELL.0,
        cell_h: CELL.1,
        columns,
        glyphs,
        wide: BTreeMap::from([(WIDE_GLYPH, narrow)]),
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
    let mut buf = GlyphBuffer::new(COLUMNS, ROWS, Cell::new(' ', FOREGROUNDS[0], CLEAR))
        .with_widths(CellWidths::wide([WIDE_GLYPH]));
    cells(&mut buf);
    rects(&mut buf);
    sprites(&mut buf, card, missing);
    painted(&mut buf);
    backdrop(&mut buf, card);
    gizmos(&mut buf);
    buf
}

/// The cells: a band of backgrounds and glyphs in every colour (row 0),
/// glyphs on the clear colour and the glyph the atlas lacks (row 1), the
/// sprites' gallery on stripes (rows 2 to 10, columns 0 to 19), glyphs
/// over the `Under` sprites (row 6), see-through cells where there is no
/// backdrop (row 11, columns 0 to 8), and wide glyphs among narrow ones
/// and one wide glyph without its continuation (row 11, from column 10).
fn cells(buf: &mut GlyphBuffer) {
    for x in 0..i32::from(COLUMNS) {
        let i = nth(x);
        let glyph = GLYPHS[i % GLYPHS.len()];
        let fg = FOREGROUNDS[i % FOREGROUNDS.len()];
        buf.set(x, 0, Cell::new(glyph, fg, BACKGROUNDS[i / 4 % 4]));
        let glyph = if i.is_multiple_of(3) {
            MISSING_GLYPH
        } else {
            glyph
        };
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
    let wide = format!("{WIDE_GLYPH}#{WIDE_GLYPH}{WIDE_GLYPH}o");
    buf.print(10, 11, &wide, FOREGROUNDS[1], CLEAR);
    buf.print(
        18,
        11,
        &format!("{WIDE_GLYPH}|"),
        FOREGROUNDS[3],
        BACKGROUNDS[0],
    );
    buf.set(22, 11, Cell::new(WIDE_GLYPH, FOREGROUNDS[2], CLEAR));
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

/// An 8 × 8 RGBA picture: red across, green down, a blue checker, the
/// top-left pixels clear and the bottom row half clear.
fn rgba_picture() -> Painted {
    let mut picture = Painted::rgba(8, 8);
    for (i, pixel) in picture
        .pixels_mut()
        .as_chunks_mut::<4>()
        .0
        .iter_mut()
        .enumerate()
    {
        let (x, y) = (i % 8, i / 8);
        let alpha = match (x, y) {
            (0..2, 0..2) => 0,
            (_, 7) => 128,
            _ => 255,
        };
        let level = |v: usize| u8::try_from(v * 36).unwrap_or(u8::MAX);
        let blue = if (x + y) % 2 == 0 { 200 } else { 40 };
        pixel.copy_from_slice(&[level(x), level(y), blue, alpha]);
    }
    picture
}

/// An 8 × 8 indexed picture of three of its four colours in diagonal
/// bands, a pixel wide.
fn indexed_picture() -> Painted {
    let palette = [
        BACKGROUNDS[1],
        FOREGROUNDS[1],
        FOREGROUNDS[2],
        BACKGROUNDS[2],
    ];
    let mut picture = Painted::indexed(8, 8, &palette);
    for (i, pixel) in picture.pixels_mut().iter_mut().enumerate() {
        *pixel = u8::try_from((i % 8 + i / 8) % 3).unwrap_or(0);
    }
    picture
}

/// A one-bit picture `width` × 8: a white frame round a black one, a
/// white cross in it; with a mask, its corners and its middle column
/// left out.
fn one_bit_picture(width: u16, masked: bool) -> Painted {
    let mut picture = Painted::one_bit(width, 8);
    if masked {
        picture = picture.with_mask();
    }
    let stride = picture.stride();
    let w = usize::from(width);
    let white =
        |x: usize, y: usize| x == 0 || y == 0 || x == w - 1 || y == 7 || x == y || x + y == w - 1;
    let pixels = picture.pixels_mut();
    for y in 0..8 {
        for x in 0..w {
            if white(x, y) {
                pixels[y * stride + x / 8] |= 0x80 >> (x % 8);
            }
        }
    }
    if let Some(mask) = picture.mask_mut() {
        for y in 0..8 {
            for x in 0..w {
                let corner = (x == 0 || x == w - 1) && (y == 0 || y == 7);
                if corner || x == w / 2 {
                    mask[y * stride + x / 8] &= !(0x80 >> (x % 8));
                }
            }
        }
    }
    picture
}

/// The painted pictures, from y 140 below the sprites' gallery, each
/// twice its size: the RGBA one `Under` and `Over` at brightness 102, the
/// indexed one whole and cut by a `clip` on odd pixels, the masked one,
/// the left half of the unmasked one, and one the table lacks, twice
/// (the second from y 158).
fn painted(buf: &mut GlyphBuffer) {
    let pictures = buf.pictures_mut();
    pictures.insert(PAINTED_RGBA, rgba_picture());
    pictures.insert(PAINTED_INDEXED, indexed_picture());
    pictures.insert(PAINTED_MASKED, one_bit_picture(8, true));
    pictures.insert(PAINTED_ONE_BIT, one_bit_picture(16, false));
    let eight = Rect::new(0, 0, 8, 8);
    let at = |x: i32| Rect::new(x, 140, 16, 16);
    let place = |id, x, layer| PaintedSprite::new(id, eight, at(x), layer);
    buf.add_painted(place(PAINTED_RGBA, 4, Layer::Under));
    let mut dim = place(PAINTED_RGBA, 26, Layer::Over);
    dim.brightness = 102;
    buf.add_painted(dim);
    buf.add_painted(place(PAINTED_INDEXED, 48, Layer::Over));
    let mut cut = place(PAINTED_INDEXED, 70, Layer::Under);
    cut.clip = Rect::new(73, 143, 9, 11);
    buf.add_painted(cut);
    buf.add_painted(place(PAINTED_MASKED, 92, Layer::Over));
    buf.add_painted(place(PAINTED_ONE_BIT, 114, Layer::Under));
    buf.add_painted(place(PAINTED_MISSING, 136, Layer::Over));
    let mut missing = place(PAINTED_MISSING, 4, Layer::Under);
    missing.dest.y = 158;
    missing.clip = missing.dest;
    buf.add_painted(missing);
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
            let glyph = if i.is_multiple_of(2) {
                GLYPHS[i % 4]
            } else {
                ' '
            };
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
    // Its own picture, which `set_backdrop` copies into the frame's table.
    scene
        .pictures_mut()
        .insert(PAINTED_SCENE, one_bit_picture(16, true));
    let whole = Rect::new(0, 0, 16, 8);
    let painted = PaintedSprite::new(PAINTED_SCENE, whole, Rect::new(30, 4, 16, 8), Layer::Over);
    scene.add_painted(painted);
    let clip = Rect::new(21, 2, 10, 9);
    for y in clip.y..clip.y + clip.h {
        for x in clip.x..clip.x + clip.w {
            let i = nth(x + y);
            let glyph = if i.is_multiple_of(3) {
                GLYPHS[i % 4]
            } else {
                ' '
            };
            buf.set(x, y, Cell::see_through(glyph, FOREGROUNDS[i % 4]));
        }
    }
    buf.set(23, 4, Cell::new('o', FOREGROUNDS[1], BACKGROUNDS[1]));
    buf.set(24, 4, Cell::new('#', FOREGROUNDS[2], CLEAR));
    buf.set(27, 8, Cell::new(' ', FOREGROUNDS[0], BACKGROUNDS[3]));
    buf.set_backdrop(Rc::new(scene), clip, (0.5, 0.5), 2);
}

/// The gizmos, last, over everything: eight lines from (208, 96) into
/// each octant, over the backdrop; two rectangles' outlines, the second
/// cut by a `clip`; two crosses and three circles' outlines below and
/// above it, one of radius 0; and two labels in row 11's cells.
fn gizmos(buf: &mut GlyphBuffer) {
    let ends = [
        (238, 104),
        (216, 126),
        (200, 126),
        (178, 104),
        (178, 88),
        (200, 66),
        (216, 66),
        (238, 88),
    ];
    let mut add = |role, shape, color| buf.add_gizmo(GizmoItem::new("card", role, shape, color));
    for (i, to) in ends.into_iter().enumerate() {
        let line = GizmoShape::Line {
            from: (208, 96),
            to,
        };
        add(GizmoRole::Path, line, FOREGROUNDS[i % FOREGROUNDS.len()]);
    }
    let rect = |x, y, w, h| GizmoShape::Rect {
        rect: Rect::new(x, y, w, h),
    };
    add(GizmoRole::Bounds, rect(170, 36, 24, 14), FOREGROUNDS[0]);
    let circle = |center, radius| GizmoShape::Circle { center, radius };
    add(GizmoRole::Trigger, circle((226, 150), 10), FOREGROUNDS[2]);
    add(GizmoRole::Bone, circle((250, 20), 3), FOREGROUNDS[3]);
    add(GizmoRole::Bone, circle((164, 186), 0), FOREGROUNDS[4]);
    let cross = |at, size| GizmoShape::Cross { at, size };
    add(GizmoRole::Note, cross((186, 150), 4), FOREGROUNDS[1]);
    add(GizmoRole::Note, cross((200, 150), 1), FOREGROUNDS[0]);
    let mut cut = GizmoItem::new(
        "card",
        GizmoRole::Solid,
        rect(226, 40, 18, 10),
        FOREGROUNDS[1],
    );
    cut.clip = Rect::new(230, 38, 20, 6);
    buf.add_gizmo(cut);
    // A label's glyphs are in the cells, and its item draws nothing.
    for (column, text) in [(24, "#o"), (29, "-|")] {
        buf.print_fg(column, 11, text, FOREGROUNDS[2]);
        let label = GizmoShape::Label {
            at: (column * 8, 180),
            text: LabelText::new(text),
        };
        let mut item = GizmoItem::new("card", GizmoRole::Note, label, FOREGROUNDS[2]);
        item.clip = Rect::new(column * 8, 176, 16, 16);
        buf.add_gizmo(item);
    }
}

/// `v` as an index into a list of colours or glyphs (0 if negative).
fn nth(v: i32) -> usize {
    usize::try_from(v).unwrap_or(0)
}

#[cfg(test)]
mod tests;
