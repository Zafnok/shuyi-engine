use proptest::prelude::*;
use shuyi_assets::font::FontAtlasDef;
use shuyi_assets::onebit::{Base, Pattern, RolePatterns, Rows};

use super::*;
use crate::frame::console::ConsoleSpec;
use crate::frame::glyph_buffer::{Cell, Layer, Overlay, PxRect};

const PAPER: Rgb = Rgb::new(250, 248, 240);
const INK: Rgb = Rgb::new(20, 20, 30);
const PLAYER: Rgb = Rgb::new(60, 120, 255);
const ENEMY: Rgb = Rgb::new(230, 40, 40);
const CURSOR: Rgb = Rgb::new(255, 215, 90);
/// The same colour as `ENEMY`, another role.
const DANGER: Rgb = ENEMY;
/// A colour of no role.
const LOOSE: Rgb = Rgb::new(128, 128, 128);

/// A skin's palette by role.
fn palette() -> Vec<(&'static str, Rgb)> {
    vec![
        ("cursor", CURSOR),
        ("danger", DANGER),
        ("paper", PAPER),
        ("team.enemy", ENEMY),
        ("team.player", PLAYER),
        ("text", INK),
    ]
}

fn pattern(base: Base) -> Pattern {
    Pattern::new(base)
}

/// Its pattern map: no patterns for `danger`.
fn def() -> PatternMapDef {
    let roles = [
        ("cursor", RolePatterns::new(Pattern::BLACK, Pattern::BLACK)),
        ("paper", RolePatterns::new(Pattern::WHITE, Pattern::WHITE)),
        (
            "team.enemy",
            RolePatterns::new(pattern(Base::Dither50), pattern(Base::DiagonalDown)),
        ),
        (
            "team.player",
            RolePatterns {
                fg: pattern(Base::HLines).inverted(),
                bg: None,
            },
        ),
        ("text", RolePatterns::new(Pattern::BLACK, Pattern::WHITE)),
    ];
    PatternMapDef {
        roles: roles.map(|(n, p)| (n.to_owned(), p)).into(),
        ..PatternMapDef::default()
    }
}

fn map() -> PatternMap {
    PatternMap::new(palette(), &def())
}

/// A font of 8 × 16 cells: `#` every pixel, `|` its left column, `?`
/// (the fallback) nothing.
fn font_parts() -> (FontAtlasDef, Vec<u8>) {
    let atlas = FontAtlasDef {
        cell_w: 8,
        cell_h: 16,
        columns: 32,
        glyphs: [('#', 0), ('|', 1), ('?', 2)].into(),
    };
    let mut bits = vec![0u8; 3 * 16];
    bits[..16].fill(u8::MAX);
    bits[16..32].fill(0x80);
    (atlas, bits)
}

/// `buf` drawn with `map`.
fn drawn(buf: &GlyphBuffer, map: &PatternMap) -> OneBitFrame {
    let (atlas, bits) = font_parts();
    let font = Font::new(&atlas, &bits).unwrap();
    to_one_bit(buf, map, &font, &())
}

/// The display's 50 × 15 cells on paper.
fn screen() -> GlyphBuffer {
    GlyphBuffer::new(50, 15, Cell::new(' ', INK, PAPER))
}

#[test]
fn luminance_weighs_red_green_and_blue_as_rec_601_in_integers() {
    assert_eq!(luminance(Rgb::new(0, 0, 0)), 0);
    assert_eq!(luminance(Rgb::new(255, 255, 255)), 255);
    assert_eq!(luminance(Rgb::new(255, 0, 0)), 76);
    assert_eq!(luminance(Rgb::new(0, 255, 0)), 149);
    assert_eq!(luminance(Rgb::new(0, 0, 255)), 28);
    assert_eq!(luminance(LOOSE), 128);
    // (77 × 10 + 150 × 200 + 29 × 30) >> 8 = 31640 >> 8.
    assert_eq!(luminance(Rgb::new(10, 200, 30)), 123);
}

#[test]
fn a_colour_is_drawn_in_its_roles_patterns_or_else_by_its_luminance() {
    let map = map();
    assert_eq!(map.role_of(PLAYER), Some("team.player"));
    assert_eq!(map.role_of(INK), Some("text"));
    // `danger` has no patterns, so its colour is the enemy's alone.
    assert_eq!(map.role_of(ENEMY), Some("team.enemy"));
    assert_eq!(map.role_of(LOOSE), None);
    let shade = |p: Pattern| Shade::from(p);
    assert_eq!(map.shape(ENEMY), shade(pattern(Base::Dither50)));
    assert_eq!(map.glyph(ENEMY), shade(pattern(Base::Dither50)));
    assert_eq!(map.ground(ENEMY), shade(pattern(Base::DiagonalDown)));
    // No background: the foreground.
    let player = shade(pattern(Base::HLines).inverted());
    assert_eq!(map.ground(PLAYER), player);
    assert_eq!(map.glyph(PLAYER), player);
    // No role: dithered by luminance, a glyph solid.
    assert_eq!(map.ground(LOOSE), shade(pattern(Base::Dither50)));
    assert_eq!(map.shape(LOOSE), shade(pattern(Base::Dither50)));
    assert_eq!(map.glyph(LOOSE), shade(Pattern::WHITE));
    assert_eq!(map.glyph(Rgb::new(127, 127, 127)), shade(Pattern::BLACK));
    assert_eq!(map.ground(Rgb::new(0, 0, 0)), shade(Pattern::BLACK));
    assert_eq!(map.shape(Rgb::new(255, 255, 255)), shade(Pattern::WHITE));
    let quarter = Rgb::new(191, 191, 191);
    assert_eq!(map.shape(quarter), shade(pattern(Base::Dither25)));
    // Nothing here is edged or flips: those are the game's inks.
    assert!(!map.glyph(INK).edged && !map.shape(CURSOR).flips);
    // An empty map: every colour by luminance.
    let empty = PatternMap::new(palette(), &PatternMapDef::default());
    assert_eq!(empty.role_of(INK), None);
    assert_eq!(empty, PatternMap::default());
}

#[test]
fn of_roles_sharing_a_colour_the_first_with_patterns_is_its_role() {
    let mut both = def();
    both.roles.insert(
        "danger".to_owned(),
        RolePatterns::new(Pattern::BLACK, Pattern::BLACK),
    );
    let map = PatternMap::new(palette(), &both);
    assert_eq!(map.role_of(ENEMY), Some("danger"));
    assert_eq!(map.shape(ENEMY), Shade::from(Pattern::BLACK));
    let reversed = PatternMap::new(palette().into_iter().rev(), &both);
    assert_eq!(reversed.role_of(ENEMY), Some("team.enemy"));
}

#[test]
fn cells_rectangles_and_glyphs_are_drawn_in_their_roles_patterns() {
    let mut buf = screen();
    // An enemy's cell: its background, and a full glyph in its foreground.
    buf.set(2, 1, Cell::new('#', ENEMY, ENEMY));
    buf.set(3, 1, Cell::new(' ', ENEMY, ENEMY));
    // Text: a column of black on white.
    buf.set(5, 1, Cell::new('|', INK, PAPER));
    // A glyph of no role's colour, on a background of none.
    buf.set(7, 1, Cell::new('#', LOOSE, Rgb::new(0, 0, 0)));
    buf.add_overlay(Overlay::new(
        PxRect::new(80, 64, 13, 9),
        PLAYER,
        Layer::Over,
    ));
    let frame = drawn(&buf, &map());
    let fg = pattern(Base::Dither50).rows();
    let bg = pattern(Base::DiagonalDown).rows();
    let white = |rows: Rows, x: i32, y: i32| Shade::rows(rows).is_white(x, y);
    for y in 16..32 {
        for x in 16..24 {
            assert_eq!(frame.is_white(x, y), white(fg, x, y), "glyph ({x}, {y})");
        }
        for x in 24..32 {
            assert_eq!(frame.is_white(x, y), white(bg, x, y), "ground ({x}, {y})");
        }
        // Black, then white.
        assert!(!frame.is_white(40, y));
        assert!((41..48).all(|x| frame.is_white(x, y)));
        // Solid white: the grey is light enough.
        assert!((56..64).all(|x| frame.is_white(x, y)));
    }
    let lines = pattern(Base::HLines).inverted().rows();
    for y in 64..73 {
        for x in 80..93 {
            assert_eq!(frame.is_white(x, y), white(lines, x, y), "rect ({x}, {y})");
        }
        assert!(frame.is_white(79, y) && frame.is_white(93, y));
    }
}

#[test]
fn patterns_are_fixed_to_the_screen_so_they_dont_swim() {
    // The same rectangle a pixel apart: where both cover a pixel, it is
    // the same.
    let rect = |x| {
        let mut buf = screen();
        let at = PxRect::new(x, 30, 20, 20);
        buf.add_overlay(Overlay::new(at, ENEMY, Layer::Under));
        drawn(&buf, &map())
    };
    let (a, b) = (rect(10), rect(11));
    for y in 30..50 {
        for x in 11..30 {
            assert_eq!(a.is_white(x, y), b.is_white(x, y), "({x}, {y})");
        }
    }
    // And it is the pattern's pixel there, not the rectangle's.
    let rows = pattern(Base::Dither50).rows();
    assert_eq!(b.is_white(11, 30), Shade::rows(rows).is_white(11, 30));
}

#[test]
fn moving_the_cursor_one_cell_changes_only_that_cells_rows() {
    let cursor_at = |x: i32| {
        let mut buf = screen();
        buf.print(0, 0, "| || |", INK, PAPER);
        buf.set(x, 4, Cell::new(' ', INK, CURSOR));
        drawn(&buf, &map())
    };
    let (before, after) = (cursor_at(10), cursor_at(11));
    let changed = after.changed_rows(&before);
    // Cell row 4 of 16 pixels: rows 64 to 79, and no other.
    assert_eq!(
        changed.rows().collect::<Vec<_>>(),
        (64..80).collect::<Vec<_>>()
    );
    assert_eq!(changed.count(), 16);
    assert!(after.changed_rows(&after).is_empty());
    // Down a cell: its old row and its new one.
    let mut down = screen();
    down.print(0, 0, "| || |", INK, PAPER);
    down.set(10, 5, Cell::new(' ', INK, CURSOR));
    let changed = drawn(&down, &map()).changed_rows(&before);
    assert_eq!(
        changed.rows().collect::<Vec<_>>(),
        (64..96).collect::<Vec<_>>()
    );
    assert_eq!(
        changed,
        changed_rows(before.bytes(), drawn(&down, &map()).bytes())
    );
}

#[test]
fn a_row_mask_holds_the_displays_rows_only() {
    let mut mask = RowMask::default();
    assert!(mask.is_empty());
    for y in [-1, 0, 31, 32, 239, 240, 1000] {
        mask.insert(y);
    }
    assert_eq!(mask.rows().collect::<Vec<_>>(), [0, 31, 32, 239]);
    assert_eq!(mask.count(), 4);
    assert!(mask.contains(239) && !mask.contains(240) && !mask.contains(-1));
    assert!(!mask.contains(1));
}

#[test]
fn the_frame_is_the_display_rows_padded_to_32_bits() {
    let frame = OneBitFrame::white();
    assert_eq!((frame.width(), frame.height()), (400, 240));
    const { assert!(ROW_BYTES.is_multiple_of(4) && ROW_BYTES * 8 >= 400) };
    assert_eq!(frame.bytes().len(), ROW_BYTES * 240);
    assert!(frame.is_white(399, 239) && !frame.is_white(400, 0));
    assert_eq!(format!("{frame:?}"), "OneBitFrame(400 × 240)");
}

/// FNV-1a, 64 bits: the same on every platform.
fn fnv(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, &b| {
        (hash ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// The fixture: text, the teams' cells, a rectangle of each, colours of
/// no role, and a buffer of other cells reaching past the display.
fn fixture() -> GlyphBuffer {
    let spec = ConsoleSpec {
        cols: 70,
        rows: 20,
        cell_w: 6,
        cell_h: 12,
    };
    let mut buf = GlyphBuffer::with_spec(spec, Cell::new(' ', INK, PAPER));
    buf.print(1, 1, "#| |# ?x", INK, PAPER);
    for (x, colour) in (0..).zip([PLAYER, ENEMY, CURSOR, LOOSE, Rgb::new(90, 30, 200)]) {
        buf.set(2 + x * 3, 3, Cell::new('#', colour, PAPER));
        buf.set(3 + x * 3, 3, Cell::new('|', INK, colour));
        let rect = PxRect::new(5 + x * 37, 60 + x * 9, 31, 23);
        buf.add_overlay(Overlay::new(rect, colour, Layer::Over));
    }
    buf.add_overlay(Overlay::new(
        PxRect::new(380, 220, 50, 50),
        ENEMY,
        Layer::Under,
    ));
    buf
}

/// The fixture's bits are pinned: integers only, so web and native draw
/// them alike, and a change to how a frame is drawn shows here.
#[test]
fn the_fixture_converts_to_the_same_bits_everywhere() {
    let frame = drawn(&fixture(), &map());
    assert_eq!(format!("{:016x}", fnv(frame.bytes())), "9313e06e1f2ff927");
}

/// Any buffer: any size and cells, the display's colours and others.
fn any_buffer() -> impl Strategy<Value = GlyphBuffer> {
    let colour = prop_oneof![
        prop::sample::select(palette().into_iter().map(|(_, c)| c).collect::<Vec<_>>()),
        any::<(u8, u8, u8)>().prop_map(|(r, g, b)| Rgb::new(r, g, b)),
    ];
    let cell = (
        prop::sample::select(vec![' ', '#', '|', '?', 'é']),
        colour.clone(),
        colour.clone(),
        any::<bool>(),
    )
        .prop_map(|(glyph, fg, bg, see_through)| Cell {
            glyph,
            fg,
            bg,
            see_through,
        });
    let rect = (-50..450i32, -50..300i32, 0..200i32, 0..200i32, colour)
        .prop_map(|(x, y, w, h, c)| Overlay::new(PxRect::new(x, y, w, h), c, Layer::Over));
    (
        1..90u16,
        1..40u16,
        1..12u16,
        1..20u16,
        prop::collection::vec(cell, 1..64),
        prop::collection::vec(rect, 0..6),
    )
        .prop_map(|(cols, rows, cell_w, cell_h, cells, rects)| {
            let spec = ConsoleSpec {
                cols,
                rows,
                cell_w,
                cell_h,
            };
            let mut buf = GlyphBuffer::with_spec(spec, cells[0]);
            for (i, cell) in (0i32..).zip(&cells) {
                buf.set(i * 7 % i32::from(cols), i % i32::from(rows), *cell);
            }
            for rect in rects {
                buf.add_overlay(rect);
            }
            buf
        })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn to_one_bit_never_panics_and_is_always_the_displays_size(buf in any_buffer()) {
        let frame = drawn(&buf, &map());
        prop_assert_eq!((frame.width(), frame.height()), (WIDTH, HEIGHT));
        prop_assert_eq!(frame.bytes().len(), FRAME_BYTES);
        // The rows' padding stays white.
        for row in frame.bytes().as_chunks::<ROW_BYTES>().0 {
            prop_assert_eq!(&row[50..], &[u8::MAX; ROW_BYTES - 50][..]);
        }
    }
}
