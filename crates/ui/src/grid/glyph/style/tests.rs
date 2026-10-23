use insta::assert_snapshot;
use proptest::prelude::*;
use shuyi_core::Pos;

use super::*;
use crate::grid::glyph::{GlyphSkin, cursor::CursorInk};
use crate::grid::scene::ThreatView;
use crate::grid::skin::{GridSkin, NoUnits};
use crate::grid::testing::{any_area, any_scene};
use crate::toy::grid::{COLORS, GLYPH, GRASS, Kit, Letters, ONE_BIT, Piece, WATER};

fn p(x: i32, y: i32) -> Pos {
    Pos::new(x, y)
}

const MOVE: RangeName = RangeName("move");
const ATTACK: RangeName = RangeName("attack");
const DANGER: RangeName = RangeName("danger");

/// The one-bit example's style.
const ONE: GlyphStyle = ONE_BIT.style;

/// A buffer of 16 × 4 cells, every cell `blank()`.
fn console() -> GlyphBuffer {
    GlyphBuffer::new(16, 4, blank())
}

fn blank() -> Cell {
    Cell::new('x', Rgb::new(1, 2, 3), Rgb::new(4, 5, 6))
}

/// A `w × h` view from (0, 0), all grass.
fn grass(w: i32, h: i32) -> MapScene<Piece> {
    let mut scene = MapScene::new(p(0, 0), (w, h));
    for tile in &mut scene.tiles {
        tile.terrain = Some(GRASS);
    }
    scene
}

/// `scene` painted by `skin` into [`console`]'s first 12 × 3 cells, its
/// pieces as letters.
fn paint(kit: &Kit, skin: &GlyphSkin, scene: &MapScene<Piece>) -> GlyphBuffer {
    let mut buf = console();
    skin.paint(
        &kit.looks(),
        scene,
        Rect::new(0, 0, 12, 3),
        &Letters,
        &mut buf,
    );
    buf
}

#[test]
fn the_plain_style_is_the_default_and_the_glyph_skins_own() {
    assert_eq!(GlyphStyle::default(), GlyphStyle::PLAIN);
    assert_eq!(GLYPH.style, GlyphStyle::PLAIN);
    let skin = GlyphSkin {
        colors: COLORS,
        walk: GLYPH.walk,
        foot_px: 2,
        style: GlyphStyle::PLAIN,
    };
    assert_eq!(GLYPH, skin);
    assert_eq!(GLYPH.styled(ONE), ONE_BIT);
    assert_eq!(ONE_BIT.styled(GlyphStyle::PLAIN), GLYPH);
}

#[test]
fn side_cells_are_left_clear_and_the_tiles_centred_between_them() {
    let plain = GlyphStyle::PLAIN;
    // 50 cells: 25 tiles, or 24 between a cell at each side.
    assert_eq!(plain.view_tiles(Rect::new(0, 0, 50, 15)), (25, 15));
    assert_eq!(ONE.view_tiles(Rect::new(0, 0, 50, 15)), (24, 15));
    // Two sides' worth and no more: nothing fits.
    assert_eq!(ONE.view_tiles(Rect::new(0, 0, 3, 2)), (0, 2));
    assert_eq!(ONE.view_tiles(Rect::new(0, 0, 4, 2)), (1, 2));
    assert_eq!(ONE.view_tiles(Rect::new(0, 0, -6, -1)), (0, 0));
    let big = MapScene::<Piece>::new(p(0, 0), (40, 20));
    // 9 cells: 3 tiles, a cell and a half to spare each side.
    let layout = ONE.layout(&big, Rect::new(3, 2, 9, 4));
    assert_eq!(layout.cells(), Rect::new(4, 2, 6, 4));
    // 10 cells: 4 tiles, a cell each side.
    let layout = ONE.layout(&big, Rect::new(3, 2, 10, 4));
    assert_eq!(layout.cells(), Rect::new(4, 2, 8, 4));
    // Fewer tiles than fit: from where the fitted ones start.
    let small = MapScene::<Piece>::new(p(0, 0), (1, 1));
    let layout = ONE.layout(&small, Rect::new(3, 2, 10, 4));
    assert_eq!(layout.cells(), Rect::new(4, 2, 2, 1));
    // The plain style's layout is the glyph skin's: from the corner, an
    // odd cell left over at the right.
    for area in [Rect::new(3, 2, 9, 4), Rect::new(-5, 0, 7, 3)] {
        assert_eq!(plain.layout(&big, area), Layout::new(&big, area));
    }
    assert_eq!(
        ONE.layout(&big, Rect::new(0, 0, -4, 3)).cells(),
        Rect::new(0, 0, 0, 3)
    );
    // The skin's tiles and grid are the style's.
    assert_eq!(
        GridSkin::<Piece>::view_tiles(&ONE_BIT, Rect::new(0, 0, 50, 15)),
        (24, 15)
    );
    let grid = GridSkin::<Piece>::grid(&ONE_BIT, &big, Rect::new(3, 2, 10, 4));
    assert_eq!(grid.rect(p(0, 0)), Some(Rect::new(32, 32, 16, 16)));
}

#[test]
fn the_range_laid_on_last_fills_its_tiles_but_a_skipped_one() {
    let kit = Kit::new();
    let looks = kit.looks();
    let pal = &kit.palette;
    let g = looks.terrains.glyphs(GRASS).unwrap();
    let (fg, bg) = (pal.lookup(g.fg).unwrap(), pal.lookup(g.bg).unwrap());
    let text = pal.lookup("text").unwrap();
    let mut scene = grass(5, 1);
    scene.tint([p(0, 0), p(1, 0), p(2, 0)], MOVE);
    scene.tint([p(1, 0)], ATTACK);
    // Skipped: under it, the move range fills; alone, nothing does.
    scene.tint([p(0, 0), p(3, 0)], DANGER);
    // What a tile would become is drawn over its fill.
    scene.tile_mut(p(2, 0)).unwrap().becomes = Some(WATER);
    let mut buf = console();
    let area = Rect::new(0, 0, 10, 1);
    ONE.draw_tiles(&looks, &mut buf, &scene, &ONE.layout(&scene, area));
    let cell = |x| *buf.get(x, 0).unwrap();
    // No side cells: 4 tiles fit, from cell 1.
    assert_eq!(cell(0), blank());
    let filled = |kind| looks.range(kind);
    assert_eq!(cell(1), Cell::new(g.glyphs[0], text, filled(MOVE)));
    assert_eq!(cell(2), Cell::new(g.glyphs[1], text, filled(MOVE)));
    assert_eq!(cell(3), Cell::new(g.glyphs[0], text, filled(ATTACK)));
    assert_eq!(cell(4), Cell::new(g.glyphs[1], text, filled(ATTACK)));
    let w = looks.terrains.glyphs(WATER).unwrap();
    let (wf, wb) = (pal.lookup(w.fg).unwrap(), pal.lookup(w.bg).unwrap());
    assert_eq!(cell(5), Cell::new(w.glyphs[0], wf, wb));
    assert_eq!(cell(7), Cell::new(g.glyphs[0], fg, bg));
    assert_eq!(cell(8), Cell::new(g.glyphs[1], fg, bg));
    assert_eq!(cell(9), blank());
    let found = buf.overlays();
    assert!(found.is_empty(), "{found:?}");
    // A tile off the map is filled too: its cells' glyphs kept.
    let mut scene = MapScene::<Piece>::new(p(0, 0), (1, 1));
    scene.tint([p(0, 0)], MOVE);
    let mut buf = console();
    let area = Rect::new(0, 0, 4, 1);
    ONE.draw_tiles(&looks, &mut buf, &scene, &ONE.layout(&scene, area));
    let off = Cell::new('x', text, filled(MOVE));
    assert_eq!((buf.get(1, 0), buf.get(2, 0)), (Some(&off), Some(&off)));
    assert_eq!(buf.get(3, 0), Some(&blank()));
}

#[test]
fn a_fill_reaches_only_cells_on_the_console() {
    let kit = Kit::new();
    let mut scene = grass(2, 1);
    scene.tint([p(0, 0), p(1, 0)], MOVE);
    let mut buf = console();
    // Tiles at cells -1..1 and 1..3: half of the first is off the console.
    let area = Rect::new(-2, 0, 6, 1);
    ONE.draw_tiles(&kit.looks(), &mut buf, &scene, &ONE.layout(&scene, area));
    let moved = kit.looks().range(MOVE);
    for x in 0..3 {
        assert_eq!(buf.get(x, 0).map(|c| c.bg), Some(moved), "{x}");
    }
    assert_eq!(buf.get(3, 0), Some(&blank()));
}

#[test]
fn a_tile_flashes_inverted_while_its_flash_is_at_least_half() {
    let kit = Kit::new();
    let looks = kit.looks();
    let mut scene = grass(4, 1);
    scene.tile_mut(p(0, 0)).unwrap().flashes = vec![0.49];
    scene.tile_mut(p(1, 0)).unwrap().flashes = vec![0.2, 0.5];
    scene.tile_mut(p(2, 0)).unwrap().flashes = vec![1.0];
    let plain = {
        let mut buf = console();
        let style = GlyphStyle {
            flash: FlashLook::Invert,
            ..GlyphStyle::PLAIN
        };
        let area = Rect::new(0, 0, 8, 1);
        style.draw_tiles(&looks, &mut buf, &scene, &style.layout(&scene, area));
        buf
    };
    let invert = Rgb::from(Ink::Invert.code());
    let rects: Vec<_> = plain
        .overlays()
        .iter()
        .map(|o| (o.rect, o.color, o.layer))
        .collect();
    assert_eq!(
        rects,
        [
            (Rect::new(16, 0, 16, 16), invert, Layer::Over),
            (Rect::new(32, 0, 16, 16), invert, Layer::Over),
        ]
    );
    // The cells are as without a flash: nothing blends.
    let mut still = scene.clone();
    for tile in &mut still.tiles {
        tile.flashes.clear();
    }
    let mut buf = console();
    let area = Rect::new(0, 0, 8, 1);
    draw_tiles_plain(&looks, &mut buf, &still, area);
    for x in 0..16 {
        assert_eq!(plain.get(x, 0), buf.get(x, 0), "{x}");
    }
}

/// `scene`'s tiles in the plain style into `area` of `buf`.
fn draw_tiles_plain(looks: &Looks, buf: &mut GlyphBuffer, scene: &MapScene<Piece>, area: Rect) {
    let plain = GlyphStyle::PLAIN;
    plain.draw_tiles(looks, buf, scene, &plain.layout(scene, area));
}

/// The scene's cursor at `pos` in `style`, half bright, drawn flat in a
/// 12 × 3-cell area from cell (2, 0) with a cell at each side.
fn flat(style: CursorStyle, pos: Pos) -> Vec<Rect> {
    let mut scene = MapScene::<Piece>::new(p(0, 0), (5, 3));
    scene.cursor = Some(CursorView {
        pos,
        brightness: 0.5,
        style,
    });
    let ink = CursorInk {
        color: Rgb::new(250, 240, 90),
        dark: Rgb::new(0, 0, 0),
        foot_px: 2,
    };
    let area = Rect::new(2, 0, 12, 3);
    let mut buf = console();
    ONE.draw_cursor(ink, &mut buf, &scene, &ONE.layout(&scene, area), area);
    for o in buf.overlays() {
        assert_eq!(
            (o.color, o.layer),
            (ink.color, Layer::Over),
            "at full strength"
        );
    }
    buf.overlays().iter().map(|o| o.rect).collect()
}

#[test]
fn a_flat_cursor_has_no_pixel_in_two_arms() {
    let r = Rect::new;
    // Tile (1, 1): cells (5, 1) and (6, 1), pixels 40..56 × 16..32. The
    // upper corners' uprights give up their top pixel, the lower ones'
    // their bottom one.
    let corners = flat(CursorStyle::Corners, p(1, 1));
    assert_eq!(
        corners,
        [
            r(39, 16, 3, 1),
            r(39, 17, 1, 2),
            r(53, 16, 3, 1),
            r(55, 17, 1, 2),
            r(39, 29, 3, 1),
            r(39, 27, 1, 2),
            r(53, 29, 3, 1),
            r(55, 27, 1, 2),
        ]
    );
    for (i, a) in corners.iter().enumerate() {
        for b in &corners[i + 1..] {
            assert_eq!(a.intersect(b), None, "{a:?} and {b:?}");
        }
    }
    let large = flat(CursorStyle::LargeCorners, p(1, 1));
    assert_eq!(large[..2], [r(39, 16, 4, 1), r(39, 17, 1, 3)]);
    assert_eq!(large[5], r(39, 26, 1, 3));
    // The glow covers the whole tile.
    assert_eq!(flat(CursorStyle::TileGlow, p(1, 1)), [r(40, 16, 16, 16)]);
}

#[test]
fn a_flat_cursor_shows_in_the_side_cells_but_not_past_the_area() {
    let r = Rect::new;
    // Tile (0, 0) at cells (3, 0) and (4, 0): its left arms in the side
    // cell (2, 0), whose pixels are the area's.
    let first = flat(CursorStyle::Corners, p(0, 0));
    assert_eq!(first[..2], [r(23, 0, 3, 1), r(23, 1, 1, 2)]);
    // On the last row of the area: the lower arms are inside it too.
    assert_eq!(flat(CursorStyle::Corners, p(0, 2)).len(), 8);
    // Off the view: nothing.
    let found = flat(CursorStyle::Corners, p(5, 0));
    assert!(found.is_empty(), "{found:?}");
    let found = flat(CursorStyle::Corners, p(0, 3));
    assert!(found.is_empty(), "{found:?}");
    // With no side cell, the left arms' column is off the area: the
    // uprights go, the flat arms lose a pixel.
    let mut scene = MapScene::<Piece>::new(p(0, 0), (1, 1));
    scene.cursor = Some(CursorView {
        pos: p(0, 0),
        brightness: 1.0,
        style: CursorStyle::Corners,
    });
    let ink = ONE_BIT.cursor_ink(&Kit::new().looks());
    let edgeless = GlyphStyle {
        side_cells: 0,
        ..ONE
    };
    let area = Rect::new(0, 0, 2, 1);
    let draw = |scene: &MapScene<Piece>| {
        let mut buf = console();
        let layout = edgeless.layout(scene, area);
        edgeless.draw_cursor(ink, &mut buf, scene, &layout, area);
        buf.overlays().iter().map(|o| o.rect).collect::<Vec<_>>()
    };
    let rects = draw(&scene);
    assert_eq!(rects.len(), 6);
    assert_eq!(rects[..2], [r(0, 0, 2, 1), r(13, 0, 3, 1)]);
    assert_eq!(rects[3], r(0, 13, 2, 1));
    // No cursor: nothing.
    scene.cursor = None;
    let found = draw(&scene);
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn plain_threat_marks_are_the_arrows_color_on_the_named_edge() {
    let kit = Kit::new();
    let looks = kit.looks();
    let pal = &kit.palette;
    let colors = ONE.threat_colors(&looks, &COLORS);
    let foe = pal.lookup("foe").unwrap();
    assert_eq!(
        colors,
        threat::Colors {
            arrow: foe,
            mark: foe,
            edge: pal.lookup("text").unwrap(),
        }
    );
    assert_eq!(
        GlyphStyle::PLAIN.threat_colors(&looks, &COLORS),
        looks.threat(&COLORS)
    );
    // Names the palette lacks: the engine's text and background.
    let lacking = GlyphStyle {
        threat: ThreatLook::Plain { edge: "no_such" },
        ..ONE
    };
    let missing = MapColors {
        threat: "no_such",
        ..COLORS
    };
    let colors = lacking.threat_colors(&looks, &missing);
    assert_eq!(colors.arrow, pal.engine(EngineColor::Text));
    assert_eq!(colors.edge, pal.engine(EngineColor::Background));
    // The skin paints its threat in them.
    let mut scene = grass(5, 3);
    scene.threat = Some(ThreatView {
        tile: p(0, 0),
        enemies: vec![p(4, 2)],
    });
    let buf = paint(&kit, &ONE_BIT, &scene);
    let grid = GridSkin::<Piece>::grid(&ONE_BIT, &scene, Rect::new(0, 0, 12, 3));
    let threat = scene.threat.as_ref().unwrap();
    let start = threat::Start::Edge;
    let expect = threat::threat_overlays(threat, &grid, start, ONE.threat_colors(&looks, &COLORS));
    assert_eq!(buf.overlays(), expect);
}

/// A 5 × 3 field under the one-bit example: the move range filling three
/// tiles, the attack range laid over one of them, the danger range (not a
/// fill) over another and alone on a fourth; a tile flashing at full
/// strength (inverted) and one too faint to show; a tile becoming water;
/// your piece, and the flat cursor at half brightness (drawn full).
#[test]
fn one_bit_example_snapshot() {
    let kit = Kit::new();
    let mut scene = grass(5, 3);
    scene.tile_mut(p(4, 0)).unwrap().terrain = Some(WATER);
    scene.tint([p(0, 0), p(1, 0), p(2, 0), p(1, 1)], MOVE);
    scene.tint([p(1, 0)], ATTACK);
    scene.tint([p(0, 0), p(3, 0)], DANGER);
    scene.tile_mut(p(3, 1)).unwrap().flashes = vec![1.0];
    scene.tile_mut(p(4, 1)).unwrap().flashes = vec![0.3];
    scene.tile_mut(p(1, 1)).unwrap().becomes = Some(WATER);
    scene.push_unit(Piece::new(1, 'y', p(0, 2)));
    scene.cursor = Some(CursorView {
        pos: p(2, 2),
        brightness: 0.5,
        style: CursorStyle::Corners,
    });
    let buf = paint(&kit, &ONE_BIT, &scene);
    assert_snapshot!(buf.to_snapshot(&kit.palette));
}

proptest! {
    /// Whatever the scene and the area (even one partly off the console),
    /// the one-bit example changes no cell outside the area and adds no
    /// item outside its pixels.
    #[test]
    fn the_one_bit_example_paints_only_the_area(scene in any_scene(), area in any_area()) {
        let kit = Kit::new();
        let fill = Cell::new('x', Rgb::new(1, 2, 3), Rgb::new(4, 5, 6));
        let mut buf = GlyphBuffer::new(100, 32, fill);
        ONE_BIT.paint(&kit.looks(), &scene, area, &NoUnits, &mut buf);
        for y in 0..32 {
            for x in 0..100 {
                if !area.contains(x, y) {
                    prop_assert_eq!(buf.get(x, y), Some(&fill), "cell ({}, {})", x, y);
                }
            }
        }
        let px = px_rect(area);
        for item in buf.items() {
            let seen = item.visible();
            prop_assert_eq!(seen.intersect(&px), Some(seen), "{:?} in {:?}", item, area);
        }
    }
}
