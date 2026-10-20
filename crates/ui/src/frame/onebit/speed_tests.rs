//! The fast renderer (ticket 1055) against [`reference`], 1032's plain
//! one: any buffer draws the same, and redrawing only what changed gives
//! the full render of the new buffer.

use alloc::rc::Rc;

use proptest::prelude::*;
use shuyi_assets::font::FontAtlasDef;
use shuyi_assets::image::ImageId;
use shuyi_assets::ink::Ink;

use super::*;
use crate::frame::console::ConsoleSpec;
use crate::frame::glyph_buffer::{Cell, Overlay, Rect};
use crate::prelude::*;

/// A font of 8 × 16 cells: `X` a block, `?` (the fallback) two corners,
/// `[` and `]` a column at either side, `"` two dots, `#` every pixel.
fn test_font() -> (FontAtlasDef, Vec<u8>) {
    let atlas = FontAtlasDef {
        cell_w: 8,
        cell_h: 16,
        columns: 32,
        glyphs: [('X', 0), ('?', 1), ('[', 2), (']', 3), ('"', 4), ('#', 5)].into(),
    };
    let mut bits = vec![0u8; 6 * 16];
    bits[6] = 0b0001_1000;
    bits[7] = 0b0001_1000;
    bits[16] = 0b1000_0001;
    bits[32..48].fill(0b1000_0000);
    bits[48..64].fill(0b0000_0001);
    bits[72] = 0b0010_1000;
    bits[80..96].fill(u8::MAX);
    (atlas, bits)
}

/// The glyphs drawn: some of the font, a space, and two it lacks (one
/// ASCII, one not), drawn as the fallback.
const GLYPHS: [char; 9] = [' ', ' ', 'X', '?', '[', ']', '"', '#', 'Z'];
/// Every ink's code, and colours white or black by brightness, one each
/// side of the half.
fn colors() -> Vec<Rgb> {
    let mut colors: Vec<Rgb> = Ink::ALL.iter().map(|ink| Rgb::from(ink.code())).collect();
    colors.extend([
        Rgb::new(250, 250, 10),
        Rgb::new(200, 30, 30),
        Rgb::new(128, 128, 127),
        Rgb::new(128, 128, 128),
    ]);
    colors
}

/// A test picture 8 × 4, its mask see-through in its middle.
const PICTURE: [u8; 4] = [0b1010_0110, 0b0101_1001, 0b1111_0000, 0b0000_1111];
const PICTURE_MASK: [u8; 4] = [u8::MAX, 0b1110_0111, 0b1100_0011, u8::MAX];

/// The same 8 × 4 image as a picture enlarged 1.5 times before it was
/// dithered (12 × 6), as the converter makes it (game ADR-0085): rows of 2
/// bytes, its mask see-through at two corners.
const BIG_PICTURE: [u8; 12] = [
    0b1100_1010,
    0b0111_0000,
    0b0011_0101,
    0b1000_0000,
    0b1111_0000,
    0b1111_0000, //
    0b0000_1111,
    0b0000_0000,
    0b1010_1010,
    0b1010_0000,
    0b0101_0101,
    0b0101_0000,
];
const BIG_MASK: [u8; 12] = [
    0b0111_1111,
    0b1111_0000,
    0b1111_1111,
    0b1111_0000,
    0b1111_1111,
    0b1111_0000, //
    0b1111_1111,
    0b1111_0000,
    0b1111_1111,
    0b1111_0000,
    0b1111_1111,
    0b1110_0000,
];

/// The test picture's id: `q.png` (the enlarged picture) if `big`, else
/// `p.png`; both stand for an 8 × 4 image.
fn picture_id(big: bool) -> ImageId {
    let size = shuyi_assets::image::ImageInfo {
        width: 8,
        height: 4,
    };
    let table = shuyi_assets::image::ImageTable {
        images: [("p.png", size), ("q.png", size)].into(),
    };
    table.id(if big { "q.png" } else { "p.png" }).unwrap()
}

struct TestImages;

impl Images for TestImages {
    fn image(&self, id: ImageId) -> Option<Image<'_>> {
        if id == picture_id(true) {
            let big = Image::new(12, 6, &BIG_PICTURE, Some(&BIG_MASK))?;
            return Some(big.with_source(8, 4));
        }
        (id == picture_id(false))
            .then(|| Image::new(8, 4, &PICTURE, Some(&PICTURE_MASK)))
            .flatten()
    }
}

/// A cell: a glyph, fg and bg (indices into [`GLYPHS`] and
/// [`colors`]) and whether it is see-through.
fn cell() -> impl Strategy<Value = Cell> {
    cell_see_through(0.2)
}

/// [`cell`], see-through that often.
fn cell_see_through(odds: f64) -> impl Strategy<Value = Cell> {
    let colors = colors();
    let n = colors.len();
    (0..GLYPHS.len(), 0..n, 0..n, prop::bool::weighted(odds)).prop_map(
        move |(glyph, fg, bg, see_through)| Cell {
            glyph: GLYPHS[glyph],
            fg: colors[fg],
            bg: colors[bg],
            see_through,
        },
    )
}

/// What may be put in a buffer.
#[derive(Clone, Debug)]
enum Thing {
    /// A rectangle in a colour of [`colors`].
    Rect(Rect, usize, bool),
    /// A test picture: `src` (inside the picture), `dest`, `clip`,
    /// flipped, opacity, brightness, paint (0 the image, 1 dimmed, 2
    /// in a colour of [`colors`] after it), the layer, and whether it is
    /// the enlarged picture.
    Sprite(Rect, Rect, Rect, bool, u8, u8, usize, bool, bool),
}

fn px_rect(w: i32, h: i32) -> impl Strategy<Value = Rect> {
    (-12..w, -12..h, 0..w / 2, 0..h / 2).prop_map(|(x, y, w, h)| Rect::new(x, y, w, h))
}

fn thing() -> impl Strategy<Value = Thing> {
    let n = colors().len();
    let src = (0..8, 0..4).prop_flat_map(|(x, y)| {
        (1..=8 - x, 1..=4 - y).prop_map(move |(w, h)| Rect::new(x, y, w, h))
    });
    prop_oneof![
        (px_rect(420, 260), 0..n, any::<bool>()).prop_map(|(r, c, l)| Thing::Rect(r, c, l)),
        (
            src,
            px_rect(420, 260),
            px_rect(420, 260),
            any::<bool>(),
            prop_oneof![Just(u8::MAX), any::<u8>()],
            prop_oneof![Just(u8::MAX), any::<u8>()],
            0..n + 2,
            any::<bool>(),
            any::<bool>(),
        )
            .prop_map(|(s, d, c, f, o, b, p, l, big)| {
                Thing::Sprite(s, d, c, f, o, b, p, l, big)
            }),
    ]
}

impl Thing {
    fn put(&self, buf: &mut GlyphBuffer) {
        let colors = colors();
        let layer = |over: bool| if over { Layer::Over } else { Layer::Under };
        match *self {
            Thing::Rect(rect, color, over) => {
                buf.add_overlay(Overlay::new(rect, colors[color], layer(over)));
            }
            Thing::Sprite(src, dest, clip, flip_x, opacity, brightness, paint, over, big) => {
                let paint = match paint {
                    0 => Paint::Image,
                    1 => Paint::Dimmed,
                    i => Paint::Solid(colors[i - 2]),
                };
                buf.add_sprite(Sprite {
                    clip,
                    flip_x,
                    opacity,
                    brightness,
                    paint,
                    ..Sprite::new(picture_id(big), src, dest, layer(over))
                });
            }
        }
    }
}

/// The cells' sizes: the Playdate's, and smaller than the font's glyphs
/// (8 × 16), which then reach into the cells right of and below theirs.
const CELL_SIZES: [(u16, u16); 3] = [(8, 16), (6, 8), (8, 12)];

/// A buffer of up to `max_w × max_h` cells of one of [`CELL_SIZES`],
/// sometimes past the screen, with its cells (see-through that often) and
/// things.
fn plain_buffer(max_w: u16, max_h: u16, odds: f64) -> impl Strategy<Value = GlyphBuffer> {
    (1..=max_w, 1..=max_h, 0..CELL_SIZES.len())
        .prop_flat_map(move |(w, h, size)| {
            let n = usize::from(w) * usize::from(h);
            let cells = prop::collection::vec(cell_see_through(odds), n);
            (
                Just((w, h, size)),
                cells,
                prop::collection::vec(thing(), 0..6),
            )
        })
        .prop_map(|((w, h, size), cells, things)| {
            let (cell_w, cell_h) = CELL_SIZES[size];
            let spec = ConsoleSpec {
                cols: w,
                rows: h,
                cell_w,
                cell_h,
            };
            let mut buf = GlyphBuffer::with_spec(spec, cells[0]);
            for (i, cell) in (0i32..).zip(cells) {
                buf.set(i % i32::from(w), i / i32::from(w), cell);
            }
            for thing in &things {
                thing.put(&mut buf);
            }
            buf
        })
}

/// A backdrop: a scene, the window's cells, the origin and the zoom.
type Backdrop = (GlyphBuffer, Rect, (f32, f32), u8);

fn backdrop() -> impl Strategy<Value = Backdrop> {
    (
        plain_buffer(24, 8, 0.2),
        (0..52, 0..16, 1..30, 1..10).prop_map(|(x, y, w, h)| Rect::new(x, y, w, h)),
        (-80.0f32..80.0, -40.0f32..40.0),
        1..=4u8,
    )
}

fn with_backdrop(mut buf: GlyphBuffer, backdrop: Option<Backdrop>) -> GlyphBuffer {
    if let Some((scene, clip, origin, zoom)) = backdrop {
        buf.set_backdrop(Rc::new(scene), clip, origin, zoom);
    }
    buf
}

/// Any buffer: a plain one, or one mostly see-through over a backdrop.
fn buffer() -> impl Strategy<Value = GlyphBuffer> {
    prop_oneof![
        plain_buffer(52, 16, 0.2),
        (plain_buffer(52, 16, 0.8), backdrop())
            .prop_map(|(buf, backdrop)| with_backdrop(buf, Some(backdrop))),
    ]
}

/// A change to a buffer.
#[derive(Clone, Debug)]
enum Edit {
    Set(i32, i32, Cell),
    Put(Thing),
    /// Fills cells, which cuts the items over them.
    Fill(Rect, Cell),
    Backdrop(Option<Backdrop>),
}

fn edit() -> impl Strategy<Value = Edit> {
    prop_oneof![
        6 => (0..52, 0..16, cell()).prop_map(|(x, y, c)| Edit::Set(x, y, c)),
        2 => thing().prop_map(Edit::Put),
        1 => (0..52, 0..16, 1..12, 1..6, cell())
            .prop_map(|(x, y, w, h, c)| Edit::Fill(Rect::new(x, y, w, h), c)),
        1 => prop::option::of(backdrop()).prop_map(Edit::Backdrop),
    ]
}

impl Edit {
    fn apply(&self, buf: GlyphBuffer) -> GlyphBuffer {
        let mut buf = buf;
        match self {
            Edit::Set(x, y, cell) => buf.set(*x, *y, *cell),
            Edit::Put(thing) => thing.put(&mut buf),
            Edit::Fill(rect, cell) => buf.fill_rect(*rect, *cell),
            Edit::Backdrop(None) => buf.clear_backdrop(),
            Edit::Backdrop(Some(b)) => return with_backdrop(buf, Some(b.clone())),
        }
        buf
    }
}

fn fast(buf: &GlyphBuffer) -> Box<Frame> {
    let (atlas, bits) = test_font();
    let font = Font::new(&atlas, &bits).unwrap();
    let mut frame = Box::new([0u8; FRAME_BYTES]);
    render(buf, &font, &TestImages, &mut frame);
    frame
}

fn slow(buf: &GlyphBuffer) -> Box<Frame> {
    let (atlas, bits) = test_font();
    let font = Font::new(&atlas, &bits).unwrap();
    let mut frame = Box::new([0u8; FRAME_BYTES]);
    reference::render(buf, &font, &TestImages, &mut frame);
    frame
}

/// Where two frames first differ, for a failure's message.
fn first_difference(a: &Frame, b: &Frame) -> Option<(usize, usize)> {
    let i = a.iter().zip(b.iter()).position(|(a, b)| a != b)?;
    Some((i % ROW_BYTES * 8, i / ROW_BYTES))
}

proptest! {
    #[test]
    fn the_fast_renderer_draws_as_the_reference_does(buf in buffer()) {
        let (fast, slow) = (fast(&buf), slow(&buf));
        prop_assert!(fast == slow, "first differs at {:?}", first_difference(&fast, &slow));
    }

    #[test]
    fn redrawing_what_changed_gives_the_full_render(
        old in buffer(),
        edits in prop::collection::vec(edit(), 0..8),
    ) {
        let new = edits.iter().fold(old.clone(), |buf, edit| edit.apply(buf));
        let (atlas, bits) = test_font();
        let font = Font::new(&atlas, &bits).unwrap();
        let mut frame = fast(&old);
        render_changed(&old, &new, &font, &TestImages, &mut frame);
        let full = fast(&new);
        prop_assert!(*frame == *full, "first differs at {:?}", first_difference(&frame, &full));
    }

    #[test]
    fn with_roles_the_fast_renderer_draws_as_the_reference_does(buf in buffer()) {
        let (fast, slow) = (fast_with_roles(&buf), slow_with_roles(&buf));
        prop_assert!(fast == slow, "first differs at {:?}", first_difference(&fast, &slow));
    }

    #[test]
    fn with_roles_redrawing_what_changed_gives_the_full_render(
        old in buffer(),
        edits in prop::collection::vec(edit(), 0..8),
    ) {
        let new = edits.iter().fold(old.clone(), |buf, edit| edit.apply(buf));
        let (atlas, bits) = test_font();
        let font = Font::new(&atlas, &bits).unwrap();
        let mut frame = fast_with_roles(&old);
        render_changed_with(&old, &new, &font, &TestImages, &roles(), &mut frame);
        let full = fast_with_roles(&new);
        prop_assert!(*frame == *full, "first differs at {:?}", first_difference(&frame, &full));
    }
}

/// A pattern map of [`colors`]: the first six are roles, each of its own
/// patterns, foreground and background apart; the rest are of no role.
fn roles() -> PatternMap {
    let patterns: Vec<Pattern> = Base::ALL
        .iter()
        .flat_map(|&base| [Pattern::new(base), Pattern::new(base).inverted()])
        .collect();
    let names: Vec<String> = (0..6).map(|i| format!("role.{i}")).collect();
    let def = PatternMapDef {
        roles: (0..)
            .zip(&names)
            .map(|(i, name)| {
                let fg = patterns[(2 * i + 3) % patterns.len()];
                let bg = patterns[(5 * i + 1) % patterns.len()];
                (name.clone(), RolePatterns::new(fg, bg))
            })
            .collect(),
        ..PatternMapDef::default()
    };
    PatternMap::new(names.iter().map(String::as_str).zip(colors()), &def)
}

fn fast_with_roles(buf: &GlyphBuffer) -> Box<Frame> {
    let (atlas, bits) = test_font();
    let font = Font::new(&atlas, &bits).unwrap();
    let mut frame = Box::new([0u8; FRAME_BYTES]);
    render_with(buf, &font, &TestImages, &roles(), &mut frame);
    frame
}

fn slow_with_roles(buf: &GlyphBuffer) -> Box<Frame> {
    let (atlas, bits) = test_font();
    let font = Font::new(&atlas, &bits).unwrap();
    let mut frame = Box::new([0u8; FRAME_BYTES]);
    reference::render_with(buf, &font, &TestImages, &roles(), &mut frame);
    frame
}

/// Every ink's looked-up byte is the pattern's pixels, on every row and
/// all along it.
#[test]
fn pattern_bytes_are_the_inks_pixels() {
    for ink in Ink::ALL {
        for y in 0..HEIGHT {
            let byte = InkShades::of(ink).row(y);
            for x in 0..WIDTH {
                let white = byte >> (7 - (x & 7)) & 1 == 1;
                assert_eq!(white, ink_is_white(ink, x, y), "{ink:?} at ({x}, {y})");
            }
        }
    }
}

/// The ASCII table finds the rows the glyph map does, the fallback's for
/// glyphs the font lacks.
#[test]
fn ascii_glyphs_are_looked_up_as_the_glyph_map_has_them() {
    let (atlas, bits) = test_font();
    let font = Font::new(&atlas, &bits).unwrap();
    let from_map = |glyph: char| {
        let start = rows_start(&atlas, glyph)?;
        bits.get(start..start + 16)
    };
    for glyph in (0..=0x2fff).filter_map(char::from_u32) {
        assert_eq!(font.rows(glyph), from_map(glyph), "{glyph:?}");
    }
    assert_eq!(font.rows('Z'), font.rows('?'));
    assert_eq!(font.rows('é'), font.rows('?'));
    // Without a fallback, a glyph it lacks has no rows.
    let mut bare = atlas.clone();
    bare.glyphs.remove(&'?');
    let font = Font::new(&bare, &bits).unwrap();
    assert_eq!(font.rows('Z'), None);
    assert_eq!(font.rows('é'), None);
    assert_eq!(font.rows('X'), Some(&bits[..16]));
}

/// A run's one-byte path: a whole cell's row, inside the clip, at its
/// edges and cut by them.
#[test]
fn a_byte_at_a_whole_cell_is_written_whole_only_inside_the_clip() {
    let mut frame = Box::new([u8::MAX; FRAME_BYTES]);
    let mut target = Target {
        frame: &mut frame,
        clip: Clip {
            x0: 8,
            y0: 0,
            x1: 20,
            y1: 1,
        },
    };
    let bits = u64::from(0b1000_0001u8) << 56;
    target.run(0, 0, bits, Pen::ink(Ink::Black));
    target.run(0, 8, bits, Pen::ink(Ink::Black));
    target.run(0, 16, bits, Pen::ink(Ink::Black));
    target.run(0, 16, bits >> 1, Pen::Flip);
    let row: String = (0..24)
        .map(|x| if is_white(&frame, x, 0) { '.' } else { '#' })
        .collect();
    assert_eq!(row, "........#......###......");
}

/// Changed things mark the tiles they touch, which come back as runs.
#[test]
fn tiles_gather_into_runs_along_their_rows() {
    let mut tiles = Tiles::default();
    tiles.add(Clip {
        x0: 7,
        y0: 15,
        x1: 17,
        y1: 17,
    });
    tiles.add(Clip {
        x0: 392,
        y0: 230,
        x1: 900,
        y1: 900,
    });
    tiles.add(Clip {
        x0: -20,
        y0: -20,
        x1: 0,
        y1: 3,
    });
    // Two runs apart on one row: a single tile, then two.
    let clip = |x0, y0, x1, y1| Clip { x0, y0, x1, y1 };
    tiles.add(clip(40, 48, 48, 64));
    tiles.add(clip(64, 50, 80, 60));
    // At most a run per tile, so a wrong walk can't go on for ever.
    let regions: Vec<Clip> = tiles.regions().take(50 * 15).collect();
    assert_eq!(
        regions,
        [
            clip(0, 0, 24, 16),
            clip(0, 16, 24, 32),
            clip(40, 48, 48, 64),
            clip(64, 48, 80, 64),
            clip(392, 224, 400, 240)
        ]
    );
    assert_eq!(Tiles::default().regions().take(1).count(), 0);
}

/// Every phase is marked once, in order.
#[test]
fn a_render_marks_each_phase_in_order() {
    let (atlas, bits) = test_font();
    let font = Font::new(&atlas, &bits).unwrap();
    let blank = Cell::new(' ', Rgb::new(0, 0, 0), Rgb::new(255, 255, 255));
    let buf = GlyphBuffer::with_spec(ConsoleSpec::text(50, 15), blank);
    let mut frame = Box::new([0u8; FRAME_BYTES]);
    for slow in [false, true] {
        let mut marked = Vec::new();
        let mut mark = |phase| marked.push(phase);
        if slow {
            reference::render_marked(&buf, &font, &(), &mut frame, &mut mark);
        } else {
            render_marked(&buf, &font, &(), &mut frame, &mut mark);
        }
        assert_eq!(marked, Phase::ALL);
    }
    let names: Vec<&str> = Phase::ALL.iter().map(|p| p.name()).collect();
    assert_eq!(
        names,
        [
            "ground",
            "backdrop",
            "backgrounds",
            "under items",
            "edges",
            "glyphs",
            "over items",
            "invert"
        ]
    );
}

/// The one-byte path takes only a whole byte, at a byte's start, inside
/// the clip; anything else goes the long way.
#[test]
fn only_a_whole_byte_inside_the_clip_is_written_straight() {
    let mut frame = Box::new([u8::MAX; FRAME_BYTES]);
    let target = Target {
        frame: &mut frame,
        clip: Clip {
            x0: 8,
            y0: 0,
            x1: 24,
            y1: 1,
        },
    };
    let byte = u64::from(0b1000_0001u8) << 56;
    assert_eq!(target.whole_byte(8, byte), Some(0b1000_0001));
    assert_eq!(target.whole_byte(16, byte), Some(0b1000_0001));
    // Wider than a byte, off a byte's start, or past the clip either side.
    assert_eq!(target.whole_byte(8, byte | 1 << 55), None);
    assert_eq!(target.whole_byte(12, byte), None);
    assert_eq!(target.whole_byte(0, byte), None);
    assert_eq!(target.whole_byte(24, byte), None);
    let narrow = Target {
        frame: &mut frame,
        clip: Clip {
            x0: 0,
            y0: 0,
            x1: 5,
            y1: 1,
        },
    };
    assert_eq!(narrow.whole_byte(0, byte), None);
}

/// A buffer of another console size is drawn whole.
#[test]
fn a_new_console_size_is_drawn_whole() {
    let (atlas, bits) = test_font();
    let font = Font::new(&atlas, &bits).unwrap();
    let cell = Cell::new(
        '#',
        Rgb::from(Ink::Black.code()),
        Rgb::from(Ink::Grey.code()),
    );
    let old = GlyphBuffer::with_spec(ConsoleSpec::text(4, 2), cell);
    let small = ConsoleSpec {
        cols: 4,
        rows: 2,
        cell_w: 6,
        cell_h: 8,
    };
    let new = GlyphBuffer::with_spec(small, cell);
    let mut frame = fast(&old);
    render_changed(&old, &new, &font, &TestImages, &mut frame);
    let whole = fast(&new);
    assert!(
        *frame == *whole,
        "first differs at {:?}",
        first_difference(&frame, &whole)
    );
}

/// A scene zoomed and placed away from the screen's corner: its edged
/// glyphs and its sprites (a picture flipped, dimmed, faded) draw as the
/// reference draws them.
#[test]
fn a_zoomed_scene_away_from_the_corner_draws_as_the_reference() {
    let black = Rgb::from(Ink::Black.code());
    let edged = Rgb::from(Ink::BlackEdged.code());
    let grey = Rgb::from(Ink::Grey.code());
    for zoom in 2..=4 {
        let mut scene =
            GlyphBuffer::with_spec(ConsoleSpec::text(6, 3), Cell::new(' ', black, grey));
        scene.print(1, 1, "X#[", edged, grey);
        scene.print(0, 2, "]\"X", black, grey);
        for (x, paint) in [(4, Paint::Image), (20, Paint::Dimmed)] {
            scene.add_sprite(Sprite {
                flip_x: x == 20,
                opacity: 200,
                paint,
                ..Sprite::new(
                    picture_id(false),
                    Rect::new(0, 0, 8, 4),
                    Rect::new(x, 3, 13, 7),
                    Layer::Over,
                )
            });
        }
        let mut buf =
            GlyphBuffer::with_spec(ConsoleSpec::text(50, 15), Cell::see_through(' ', black));
        buf.set_backdrop(Rc::new(scene), Rect::new(3, 2, 40, 10), (5.0, 3.0), zoom);
        let (fast, slow) = (fast(&buf), slow(&buf));
        assert!(
            fast == slow,
            "zoom {zoom}: first differs at {:?}",
            first_difference(&fast, &slow)
        );
        // Something of the scene shows.
        assert!(fast.iter().any(|&byte| byte != u8::MAX), "zoom {zoom}");
    }
}

/// How far a cell may draw: its size, or the font's glyph where that is
/// bigger, and a pixel all round; and which cells may reach a stretch of
/// pixels.
#[test]
fn a_cell_reaches_its_glyph_and_a_pixel_more() {
    let (atlas, bits) = test_font();
    let font = Font::new(&atlas, &bits).unwrap();
    let blank = Cell::new(' ', Rgb::new(0, 0, 0), Rgb::new(255, 255, 255));
    let sized = |cell_w, cell_h| ConsoleSpec {
        cols: 10,
        rows: 4,
        cell_w,
        cell_h,
    };
    let reach = |cell_w, cell_h, scale| {
        let buf = GlyphBuffer::with_spec(sized(cell_w, cell_h), blank);
        let r = Reach::of(&buf, &font, scale);
        (r.cell_w, r.cell_h, r.right, r.below)
    };
    // The font's glyphs are 8 × 16.
    assert_eq!(reach(8, 16, 1), (8, 16, 9, 17));
    assert_eq!(reach(6, 8, 1), (6, 8, 9, 17));
    assert_eq!(reach(6, 8, 3), (18, 24, 25, 49));
    assert_eq!(reach(12, 20, 2), (24, 40, 25, 41));
    assert_eq!(font.height(3), 48);
    // Cells of 10 px from x = 5, each reaching 13 px past its start and
    // a pixel before it: pixels 30..40 are reached by cells 1 to 3.
    assert_eq!(Reach::span(30, 40, 5, 10, 13, 50), 1..4);
    assert_eq!(Reach::span(30, 40, 5, 10, 13, 3), 1..3);
    assert_eq!(Reach::span(0, 400, -100, 10, 13, 50), 8..50);
    assert_eq!(Reach::span(0, 10, 0, 0, 13, 50), 0..0);
    // The edge pixel before a cell: pixel 34 is cell 3's (from 35).
    assert_eq!(Reach::span(34, 35, 5, 10, 11, 50).end, 4);
    let at = Place {
        x: 7,
        y: -3,
        scale: 3,
    };
    assert_eq!(
        at.screen(Rect::new(2, 5, 4, 6)),
        Clip {
            x0: 13,
            y0: 12,
            x1: 25,
            y1: 30,
        }
    );
    let around = Reach::of(&GlyphBuffer::with_spec(sized(6, 8), blank), &font, 1);
    assert_eq!(
        around.around(Place::ORIGIN, 2, 3),
        Clip {
            x0: 11,
            y0: 23,
            x1: 21,
            y1: 41,
        }
    );
}

/// The dither's threshold, where a pixel just shows or just doesn't
/// (opacity 247 is the edge for the last pixel of the 4 × 4 order).
#[test]
fn a_sprite_at_the_dithers_edge_draws_as_the_reference() {
    let grey = Rgb::from(Ink::Grey.code());
    for level in [246, 247, 248] {
        let mut buf = GlyphBuffer::with_spec(ConsoleSpec::text(4, 2), Cell::new(' ', grey, grey));
        buf.add_sprite(Sprite {
            opacity: level,
            brightness: level,
            ..Sprite::new(
                picture_id(false),
                Rect::new(0, 0, 8, 4),
                Rect::new(0, 0, 32, 32),
                Layer::Over,
            )
        });
        let (fast, slow) = (fast(&buf), slow(&buf));
        assert!(
            fast == slow,
            "{level}: first differs at {:?}",
            first_difference(&fast, &slow)
        );
    }
    assert!(!shows(247, 0, 3) && shows(248, 0, 3) && shows(247, 1, 0));
}

/// A window that ends inside a byte (cells 6 pixels wide) cuts the
/// scene's glyphs there, as the reference does.
#[test]
fn a_window_ending_inside_a_byte_cuts_the_scene_there() {
    let black = Rgb::from(Ink::Black.code());
    let white = Rgb::from(Ink::White.code());
    let mut scene = GlyphBuffer::with_spec(ConsoleSpec::text(8, 2), Cell::new(' ', black, white));
    scene.print(0, 0, "########", black, white);
    let narrow = ConsoleSpec {
        cols: 10,
        rows: 4,
        cell_w: 6,
        cell_h: 8,
    };
    let mut buf = GlyphBuffer::with_spec(narrow, Cell::see_through(' ', black));
    buf.set_backdrop(Rc::new(scene), Rect::new(0, 0, 3, 2), (0.0, 0.0), 1);
    let (fast, slow) = (fast(&buf), slow(&buf));
    assert!(
        fast == slow,
        "first differs at {:?}",
        first_difference(&fast, &slow)
    );
    // Black up to the window's edge at x = 18, white past it.
    assert!(!is_white(&fast, 17, 0) && is_white(&fast, 18, 0));
}

/// Shades that count how often each is asked.
#[derive(Default)]
struct Counted {
    grounds: core::cell::Cell<u32>,
    glyphs: core::cell::Cell<u32>,
}

impl Shades for Counted {
    fn ground(&self, rgb: Rgb) -> Shade {
        self.grounds.set(self.grounds.get() + 1);
        InkShades.ground(rgb)
    }

    fn glyph(&self, rgb: Rgb) -> Shade {
        self.glyphs.set(self.glyphs.get() + 1);
        InkShades.glyph(rgb)
    }

    fn shape(&self, rgb: Rgb) -> Shade {
        InkShades.shape(rgb)
    }
}

/// Neighbouring cells mostly share their colours, so a colour's shade is
/// asked once for a run of cells of it, not once a cell.
#[test]
fn a_run_of_one_colour_asks_its_shade_once() {
    let (atlas, bits) = test_font();
    let font = Font::new(&atlas, &bits).unwrap();
    let (a, b) = (Rgb::new(10, 10, 10), Rgb::new(200, 200, 200));
    let mut buf = GlyphBuffer::new(50, 15, Cell::new('X', a, b));
    let mut frame = Box::new([0u8; FRAME_BYTES]);
    let counted = Counted::default();
    render_with(&buf, &font, &TestImages, &counted, &mut frame);
    assert_eq!((counted.grounds.get(), counted.glyphs.get()), (1, 1));
    // Another colour in the middle: asked again after it, and again back.
    buf.set(10, 3, Cell::new('X', b, a));
    let counted = Counted::default();
    render_with(&buf, &font, &TestImages, &counted, &mut frame);
    assert_eq!((counted.grounds.get(), counted.glyphs.get()), (3, 5));
}
