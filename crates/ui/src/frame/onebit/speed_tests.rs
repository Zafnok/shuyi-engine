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

mod timing;

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

/// `N` pseudo-random bytes, the same every build.
const fn noise_bytes<const N: usize>(seed: u32) -> [u8; N] {
    let mut bytes = [0; N];
    let (mut state, mut i) = (seed | 1, 0);
    while i < N {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        bytes[i] = state.to_be_bytes()[0];
        i += 1;
    }
    bytes
}

/// A wide test picture, 45 × 7 (rows of 6 bytes, the last in part): many
/// bytes a row, at every alignment, its mask see-through here and there.
const WIDE: [u8; 42] = noise_bytes(3);
const WIDE_MASK: [u8; 42] = {
    let (mut mask, more) = (noise_bytes::<42>(5), noise_bytes::<42>(9));
    let mut i = 0;
    while i < mask.len() {
        mask[i] |= more[i];
        i += 1;
    }
    mask
};

/// A width and a height.
type Size = (i32, i32);

/// Each test picture: its name, its own size and the size of the image
/// it stands for. `q.png` is `p.png` enlarged 1.5 times, and `n.png` is
/// `w.png` without its mask.
const PICTURES: [(&str, Size, Size); 4] = [
    ("p.png", (8, 4), (8, 4)),
    ("q.png", (12, 6), (8, 4)),
    ("w.png", (45, 7), (45, 7)),
    ("n.png", (45, 7), (45, 7)),
];

/// The id of test picture `i` of [`PICTURES`].
#[allow(clippy::cast_sign_loss)] // sizes
fn picture_of(i: usize) -> ImageId {
    let sizes = PICTURES.map(|(name, _, (w, h))| {
        let size = shuyi_assets::image::ImageInfo::new(w as u32, h as u32);
        (name, size)
    });
    let table = shuyi_assets::image::ImageTable::new(sizes.into());
    table.id(PICTURES[i].0).unwrap()
}

/// The test picture's id: `q.png` (the enlarged picture) if `big`, else
/// `p.png`; both stand for an 8 × 4 image.
fn picture_id(big: bool) -> ImageId {
    picture_of(usize::from(big))
}

struct TestImages;

impl Images for TestImages {
    fn image(&self, id: ImageId) -> Option<Image<'_>> {
        if id == picture_of(1) {
            let big = Image::new(12, 6, &BIG_PICTURE, Some(&BIG_MASK))?;
            return Some(big.with_source(8, 4));
        }
        if id == picture_of(2) {
            return Image::new(45, 7, &WIDE, Some(&WIDE_MASK));
        }
        if id == picture_of(3) {
            return Image::new(45, 7, &WIDE, None);
        }
        (id == picture_of(0))
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
    /// in a colour of [`colors`] after it), the layer, and which of
    /// [`PICTURES`] it is.
    Sprite(Rect, Rect, Rect, bool, u8, u8, usize, bool, usize),
}

fn px_rect(w: i32, h: i32) -> impl Strategy<Value = Rect> {
    (-12..w, -12..h, 0..w / 2, 0..h / 2).prop_map(|(x, y, w, h)| Rect::new(x, y, w, h))
}

/// A part of test picture `picture`, at times reaching up to 4 pixels
/// past its edges, and where it goes: anywhere at any size, or half the
/// time shown pixel for pixel (its picture's size, which an odd part of
/// `q.png` misses by a fraction).
fn src_and_dest(picture: usize) -> impl Strategy<Value = (Rect, Rect)> {
    let (_, (pw, ph), (sw, sh)) = PICTURES[picture];
    let side = |size: i32| {
        prop_oneof![
            3 => (0..size).prop_flat_map(move |at| (Just(at), 1..=size - at)),
            1 => (-4..size).prop_flat_map(move |at| (Just(at), 1..=size + 4 - at)),
        ]
    };
    let src = (side(sw), side(sh)).prop_map(|((x, w), (y, h))| Rect::new(x, y, w, h));
    (src, px_rect(420, 260), any::<bool>()).prop_map(move |(src, dest, exact)| {
        let dest = if exact {
            Rect::new(dest.x, dest.y, src.w * pw / sw, src.h * ph / sh)
        } else {
            dest
        };
        (src, dest)
    })
}

fn thing() -> impl Strategy<Value = Thing> {
    let n = colors().len();
    prop_oneof![
        (px_rect(420, 260), 0..n, any::<bool>()).prop_map(|(r, c, l)| Thing::Rect(r, c, l)),
        sprite(),
    ]
}

/// A [`Thing::Sprite`].
fn sprite() -> impl Strategy<Value = Thing> {
    let n = colors().len();
    (0..PICTURES.len())
        .prop_flat_map(move |picture| {
            (
                src_and_dest(picture),
                px_rect(420, 260),
                any::<bool>(),
                prop_oneof![Just(u8::MAX), any::<u8>()],
                prop_oneof![Just(u8::MAX), any::<u8>()],
                0..n + 2,
                any::<bool>(),
                Just(picture),
            )
        })
        .prop_map(|((s, d), c, f, o, b, p, l, picture)| {
            Thing::Sprite(s, d, c, f, o, b, p, l, picture)
        })
}

impl Thing {
    fn put(&self, buf: &mut GlyphBuffer) {
        let colors = colors();
        let layer = |over: bool| if over { Layer::Over } else { Layer::Under };
        match *self {
            Thing::Rect(rect, color, over) => {
                buf.add_overlay(Overlay::new(rect, colors[color], layer(over)));
            }
            Thing::Sprite(src, dest, clip, flip_x, opacity, brightness, paint, over, picture) => {
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
                    ..Sprite::new(picture_of(picture), src, dest, layer(over))
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

/// Pseudo-random bytes, the same every run.
fn noise(len: usize, seed: u32) -> Vec<u8> {
    let mut state = seed | 1;
    (0..len)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            state.to_be_bytes()[0]
        })
        .collect()
}

/// The pictures of a fight like the game's (ticket 3056): a 400 × 110
/// backdrop enlarged from a 336-pixel-wide PNG, and two 160 × 160
/// fighters enlarged twice from 80 × 80 ones, masked.
struct Fight {
    backdrop: Vec<u8>,
    fighter: Vec<u8>,
    mask: Vec<u8>,
}

impl Fight {
    fn new() -> Self {
        // The mask: a disc, see-through outside it.
        let mut mask = vec![0u8; Image::stride(160) * 160];
        for y in 0..160usize {
            for x in 0..160usize {
                if x.abs_diff(80).pow(2) + y.abs_diff(80).pow(2) < 78 * 78 {
                    mask[y * 20 + x / 8] |= 0x80 >> (x & 7);
                }
            }
        }
        Self {
            backdrop: noise(Image::stride(400) * 110, 7),
            fighter: noise(Image::stride(160) * 160, 11),
            mask,
        }
    }

    fn table() -> shuyi_assets::image::ImageTable {
        use shuyi_assets::image::{ImageInfo, ImageTable};
        ImageTable::new(
            [
                ("backdrop.png", ImageInfo::new(336, 92)),
                ("fighter.png", ImageInfo::new(80, 80)),
            ]
            .into(),
        )
    }

    /// The frame: the backdrop across the screen, the two fighters over
    /// it at `x` and `x + 176`, the second flipped, neither on a byte.
    fn frame(x: i32) -> GlyphBuffer {
        let table = Self::table();
        let black = Rgb::from(Ink::Black.code());
        let mut buf =
            GlyphBuffer::with_spec(ConsoleSpec::text(50, 15), Cell::see_through(' ', black));
        let backdrop = table.id("backdrop.png").unwrap();
        let fighter = table.id("fighter.png").unwrap();
        buf.add_sprite(Sprite::new(
            backdrop,
            Rect::new(0, 0, 336, 92),
            Rect::new(0, 20, 400, 110),
            Layer::Under,
        ));
        for (dx, flip_x) in [(0, false), (176, true)] {
            buf.add_sprite(Sprite {
                flip_x,
                ..Sprite::new(
                    fighter,
                    Rect::new(0, 0, 80, 80),
                    Rect::new(x + dx, 60, 160, 160),
                    Layer::Under,
                )
            });
        }
        buf
    }
}

impl Images for Fight {
    fn image(&self, id: ImageId) -> Option<Image<'_>> {
        let table = Self::table();
        if id == table.id("backdrop.png")? {
            return Some(Image::new(400, 110, &self.backdrop, None)?.with_source(336, 92));
        }
        let fighter = Image::new(160, 160, &self.fighter, Some(&self.mask))?;
        (id == table.id("fighter.png")?).then(|| fighter.with_source(80, 80))
    }
}

/// The fight's frame, as the fighters move a pixel at a time, draws as
/// the reference draws it.
#[test]
fn a_fight_draws_as_the_reference() {
    let (atlas, bits) = test_font();
    let font = Font::new(&atlas, &bits).unwrap();
    let fight = Fight::new();
    let (mut fast, mut slow) = (Box::new([0u8; FRAME_BYTES]), Box::new([0u8; FRAME_BYTES]));
    for x in 0..9 {
        let buf = Fight::frame(x * 3 + 5);
        render(&buf, &font, &fight, &mut fast);
        reference::render(&buf, &font, &fight, &mut slow);
        assert!(
            fast == slow,
            "at {x}: first differs at {:?}",
            first_difference(&fast, &slow)
        );
    }
}

/// A buffer of grey cells and only sprites, half the time see-through
/// over a backdrop's scene of sprites, at any zoom.
fn sprites_buffer() -> impl Strategy<Value = GlyphBuffer> {
    let things = || prop::collection::vec(sprite(), 1..10);
    let window = (0..52, 0..16, 1..52, 1..16).prop_map(|(x, y, w, h)| Rect::new(x, y, w, h));
    let backdrop = (things(), window, (-80.0f32..80.0, -40.0f32..40.0), 1..=4u8);
    (things(), prop::option::of(backdrop)).prop_map(|(things, backdrop)| {
        let grey = Rgb::from(Ink::Grey.code());
        let fill = |see_through| {
            if see_through {
                Cell::see_through(' ', grey)
            } else {
                Cell::new(' ', grey, grey)
            }
        };
        let mut buf = GlyphBuffer::with_spec(ConsoleSpec::text(50, 15), fill(backdrop.is_some()));
        for thing in &things {
            thing.put(&mut buf);
        }
        if let Some((scene_things, window, origin, zoom)) = backdrop {
            let mut scene = GlyphBuffer::with_spec(ConsoleSpec::text(24, 8), fill(false));
            for thing in &scene_things {
                thing.put(&mut scene);
            }
            buf.set_backdrop(Rc::new(scene), window, origin, zoom);
        }
        buf
    })
}

proptest! {
    /// Sprites anywhere (on a byte or not, clipped, flipped, masked, at
    /// any opacity and brightness, in their own pixels, dimmed or solid,
    /// pixel for pixel or not, at any scale) draw as the reference draws
    /// them (ticket 3056).
    #[test]
    fn sprites_draw_as_the_reference_does(buf in sprites_buffer()) {
        let (fast, slow) = (fast(&buf), slow(&buf));
        prop_assert!(fast == slow, "first differs at {:?}", first_difference(&fast, &slow));
        let (fast, slow) = (fast_with_roles(&buf), slow_with_roles(&buf));
        prop_assert!(fast == slow, "with roles, first differs at {:?}", first_difference(&fast, &slow));
    }
}

/// Whether `sprite`, drawn from `image` at `at` over screen columns
/// `x0..x1`, is found pixel for pixel, and its `k` if so.
fn run_of(sprite: &Sprite, image: &Image<'_>, at: Place, (x0, x1): (i32, i32)) -> Option<i32> {
    let shown = Clip {
        x0,
        y0: 0,
        x1,
        y1: 1,
    };
    match Columns::of_sprite(sprite, image, at, shown)? {
        Columns::Run { k, .. } => Some(k),
        Columns::Table { .. } => None,
    }
}

/// A sprite is pixel for pixel when its part of the picture is the
/// picture's size on the screen: not one pixel of picture or of screen
/// more or less, and not at a scale that only lands its ends right.
#[test]
fn pixel_for_pixel_is_found_exactly() {
    let data = [0u8; 21 * 4];
    // A picture `width` wide standing for an 80-pixel-wide image.
    let image = |width: u16| {
        Image::new(width, 4, &data, None)
            .unwrap()
            .with_source(80, 4)
    };
    // All 80 pixels of it, `w` pixels wide from x = 3.
    let sprite = |w: i32, flip_x: bool| Sprite {
        flip_x,
        ..Sprite::new(
            picture_of(0),
            Rect::new(0, 0, 80, 4),
            Rect::new(3, 0, w, 4),
            Layer::Over,
        )
    };
    let at = Place::ORIGIN;
    assert_eq!(
        run_of(&sprite(160, false), &image(160), at, (3, 163)),
        Some(-3)
    );
    assert_eq!(
        run_of(&sprite(160, true), &image(160), at, (3, 163)),
        Some(162)
    );
    for flip_x in [false, true] {
        let all = (3, 163);
        assert_eq!(run_of(&sprite(160, flip_x), &image(161), at, all), None);
        assert_eq!(run_of(&sprite(160, flip_x), &image(159), at, all), None);
        assert_eq!(
            run_of(&sprite(161, flip_x), &image(160), at, (3, 164)),
            None
        );
        assert_eq!(
            run_of(&sprite(159, flip_x), &image(160), at, (3, 162)),
            None
        );
    }
    // Ten columns of a sprite a pixel too wide still step by one each.
    assert_eq!(
        run_of(&sprite(161, false), &image(160), at, (3, 13)),
        Some(-3)
    );
    // Off the picture altogether: nothing to draw.
    let past = Sprite {
        src: Rect::new(100, 0, 80, 4),
        ..sprite(160, false)
    };
    assert!(Columns::of_sprite(&past, &image(160), at, Clip::SCREEN).is_none());
    // At scale 2, columns 1, 3, 3 of an 8-pixel picture shrunk to 4 end
    // two apart over three screen columns, but they don't step by one.
    let eight = Image::new(8, 4, &data, None).unwrap();
    let shrunk = Sprite::new(
        picture_of(0),
        Rect::new(0, 0, 8, 4),
        Rect::new(0, 0, 4, 4),
        Layer::Over,
    );
    let twice = Place {
        x: 0,
        y: 0,
        scale: 2,
    };
    assert_eq!(run_of(&shrunk, &eight, twice, (1, 4)), None);
    let once = Place { scale: 1, ..twice };
    let halved = Sprite {
        dest: Rect::new(0, 0, 8, 4),
        ..shrunk
    };
    assert_eq!(run_of(&halved, &eight, once, (1, 4)), Some(0));
    // From a column left of the picture, columns −0.375, 0.875 and 2.125
    // round towards 0 as 0, 0 and 2: two apart over three screen columns,
    // but no run. Only a `src` from a column of 0 or more is taken for one.
    let left = Sprite {
        src: Rect::new(-1, 0, 5, 4),
        dest: Rect::new(0, 0, 4, 4),
        ..halved
    };
    assert_eq!(run_of(&left, &eight, once, (0, 3)), None);
    // A flipped run cut short at its right: picture columns 159 to 63.
    assert_eq!(
        run_of(&sprite(160, true), &image(160), at, (3, 100)),
        Some(162)
    );
}

/// A sprite from a column left of its picture, which rounding towards 0
/// samples as 0, 0 and 2 over three screen columns, draws as the
/// reference draws it, not as a run of 0, 1 and 2.
#[test]
fn a_sprite_from_left_of_its_picture_draws_as_the_reference() {
    let black = Rgb::from(Ink::Black.code());
    for (picture, dest_x) in [(0, 0), (0, 13), (2, 5), (3, 21)] {
        let mut buf = GlyphBuffer::with_spec(
            ConsoleSpec::text(50, 15),
            Cell::new(' ', black, Rgb::from(Ink::Grey.code())),
        );
        buf.add_sprite(Sprite::new(
            picture_of(picture),
            Rect::new(-1, 0, 5, 4),
            Rect::new(dest_x, 0, 4, 4),
            Layer::Over,
        ));
        let (fast, slow) = (fast(&buf), slow(&buf));
        assert!(
            fast == slow,
            "{picture}: first differs at {:?}",
            first_difference(&fast, &slow)
        );
    }
}

/// The wide test picture's part `src` drawn at `dest` over white, maybe
/// flipped: every pixel of rows 0..7 is the picture's under it (where
/// its mask draws it) or white, and the reference draws the same.
fn check_wide(src: Rect, dest: Rect, flip_x: bool) {
    let black = Rgb::from(Ink::Black.code());
    let white = Rgb::from(Ink::White.code());
    let mut buf = GlyphBuffer::with_spec(ConsoleSpec::text(50, 15), Cell::new(' ', black, white));
    buf.add_sprite(Sprite {
        flip_x,
        ..Sprite::new(picture_of(2), src, dest, Layer::Over)
    });
    let frame = fast(&buf);
    let slow = slow(&buf);
    assert!(
        frame == slow,
        "first differs at {:?}",
        first_difference(&frame, &slow)
    );
    let image = TestImages.image(picture_of(2)).unwrap();
    for y in 0..7 {
        for x in 0..64 {
            let d = x - dest.x;
            let expected = if (0..dest.w).contains(&d) && (0..dest.h).contains(&(y - dest.y)) {
                let u = if flip_x { dest.w - 1 - d } else { d } + src.x;
                image.pixel(u.into(), (y + src.y - dest.y).into())
            } else {
                None
            };
            assert_eq!(
                is_white(&frame, x, y),
                expected.unwrap_or(true),
                "({x}, {y})"
            );
        }
    }
}

/// A flipped sprite shown pixel for pixel, from inside a byte to inside
/// another: its first byte holds the picture's last columns, its last
/// byte the first.
#[test]
fn a_flipped_sprite_ends_in_its_pictures_first_column() {
    check_wide(Rect::new(0, 0, 45, 7), Rect::new(5, 0, 45, 7), true);
    check_wide(Rect::new(0, 0, 45, 7), Rect::new(5, 0, 45, 7), false);
}

/// A sprite that starts and ends inside one byte touches only its own
/// pixels of it, flipped or not.
#[test]
fn a_sprite_inside_a_byte_draws_only_there() {
    for flip_x in [false, true] {
        check_wide(Rect::new(10, 0, 4, 7), Rect::new(10, 0, 4, 7), flip_x);
        check_wide(Rect::new(37, 2, 3, 5), Rect::new(17, 0, 3, 5), flip_x);
    }
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
