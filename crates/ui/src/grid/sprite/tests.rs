use proptest::prelude::*;
use shuyi_core::{Pos, TerrainId};

use super::*;
use crate::frame::color::Rgb;
use crate::frame::glyph_buffer::{Cell, Item};
use crate::grid::glyph::tests::AREA as MAP_AREA;
use crate::grid::scene::{CursorView, RangeName, ThreatView};
use crate::grid::skin::NoUnits;
use crate::grid::testing::{any_area, any_scene};
use crate::toy::grid::{
    Cards, GLYPH, GRASS, Kit, Letters, Piece, SAND, SPRITE_WALK, WATER, layered, sprite, tiles,
    with_indoor,
};

pub(crate) fn p(x: i32, y: i32) -> Pos {
    Pos::new(x, y)
}

/// The skin of the toy's tiles (24 × 24).
pub(crate) fn skin(kit: &Kit) -> SpriteSkin {
    sprite(Some(tiles(kit.tiles_image())))
}

/// The skin with no terrain tiles: the glyph skin's ground.
pub(crate) fn sheets() -> SpriteSkin {
    sprite(None)
}

/// [`skin`] with `w × h` tiles.
fn sized(kit: &Kit, w: u32, h: u32) -> SpriteSkin {
    let mut terrain = tiles(kit.tiles_image());
    terrain.tile_px = (w, h);
    sprite(Some(terrain))
}

/// A buffer the size of a 100 × 32 console, every cell `fill()`.
pub(crate) fn blank() -> GlyphBuffer {
    GlyphBuffer::new(100, 32, fill())
}

pub(crate) fn fill() -> Cell {
    Cell::new('x', Rgb::new(1, 2, 3), Rgb::new(4, 5, 6))
}

/// A 3 × 2 view from (0, 0): grass, but water at (1, 0).
pub(crate) fn plains() -> MapScene<Piece> {
    let mut scene = MapScene::new(p(0, 0), (3, 2));
    for tile in &mut scene.tiles {
        tile.terrain = Some(GRASS);
    }
    scene.tile_mut(p(1, 0)).unwrap().terrain = Some(WATER);
    scene
}

/// `scene` painted by `skin` into a blank console, in the 6 × 4 cells
/// from (2, 1): 48 × 64 px from (16, 16), so 2 × 2 tiles of 24 px with
/// 8 px to spare above and below; its pieces as cards.
fn painted(kit: &Kit, skin: &SpriteSkin, scene: &MapScene<Piece>) -> GlyphBuffer {
    let mut buf = blank();
    let cards = Cards { card: kit.card() };
    skin.paint(&kit.looks(), scene, Rect::new(2, 1, 6, 4), &cards, &mut buf);
    buf
}

/// The item at `i` as a rectangle.
pub(crate) fn rect_at(buf: &GlyphBuffer, i: usize) -> Overlay {
    match buf.items()[i] {
        Item::Rect(o) => o,
        Item::Sprite(s) => panic!("item {i} is a sprite: {s:?}"),
    }
}

/// The item at `i` as a sprite.
pub(crate) fn sprite_at(buf: &GlyphBuffer, i: usize) -> Sprite {
    match buf.items()[i] {
        Item::Sprite(s) => s,
        Item::Rect(o) => panic!("item {i} is a rectangle: {o:?}"),
    }
}

fn threat(tile: Pos, enemies: &[Pos]) -> ThreatView {
    ThreatView {
        tile,
        enemies: enemies.to_vec(),
    }
}

#[test]
fn a_skin_is_named_by_whether_it_has_terrain_tiles_and_walks_at_its_pace() {
    let kit = Kit::new();
    let own: &dyn GridSkin<Piece> = &skin(&kit);
    let mixed: &dyn GridSkin<Piece> = &sheets();
    assert_eq!((own.name(), mixed.name()), ("sprite", "sprite_units"));
    assert_eq!((own.walk(), mixed.walk()), (SPRITE_WALK, SPRITE_WALK));
    assert_eq!(own.tile_size(), (24, 24));
    assert_eq!(mixed.tile_size(), (16, 16));
    assert_eq!(own.view_tiles(MAP_AREA), (23, 20));
    let scene = MapScene::<Piece>::new(p(0, 0), (23, 20));
    assert_eq!(
        own.grid(&scene, MAP_AREA),
        skin(&kit).grid(&scene, MAP_AREA)
    );
    assert_eq!(skin(&kit).terrain(), Some(&tiles(kit.tiles_image())));
    assert_eq!(sheets().terrain(), None);
}

#[test]
fn view_tiles_and_the_margin_follow_the_tile_size() {
    let kit = Kit::new();
    let s = skin(&kit);
    // The map area is 560 × 480 px.
    let fit = |w, h, area| {
        let s = sized(&kit, w, h);
        let scene = MapScene::<Piece>::new(p(0, 0), s.view_tiles(area));
        let g = s.grid(&scene, area);
        (g.tiles, g.corner)
    };
    // 552 × 480 px of 24 px tiles: 4 px to spare each side.
    assert_eq!(fit(24, 24, MAP_AREA), ((23, 20), (4, 0)));
    assert_eq!(fit(16, 16, MAP_AREA), ((35, 30), (0, 0)));
    assert_eq!(fit(64, 64, MAP_AREA), ((8, 7), (24, 16)));
    // 9 × 13: 2 px to spare across, 12 down.
    assert_eq!(fit(9, 13, MAP_AREA), ((62, 36), (1, 6)));
    // One cell: no tile across; the odd pixel goes right.
    assert_eq!(fit(9, 13, Rect::new(0, 0, 1, 1)), ((0, 1), (4, 1)));
    // An area away from the corner: 56 × 32 px from (32, 48).
    assert_eq!(fit(24, 24, Rect::new(4, 3, 7, 2)), ((2, 1), (36, 52)));
    // An empty area shows nothing.
    let empty = Rect::new(3, 3, -4, -1);
    assert_eq!(s.view_tiles(empty), (0, 0));
    let five = MapScene::<Piece>::new(p(0, 0), (5, 5));
    assert_eq!(s.grid(&five, empty).tiles, (0, 0));
    // Where the skin has a tile, for menus beside it.
    let scene = MapScene::<Piece>::new(p(-3, 2), s.view_tiles(MAP_AREA));
    let px = |tile| s.tile_px(&scene, MAP_AREA, tile);
    assert_eq!(px(p(-3, 2)), Some(Rect::new(4, 0, 24, 24)));
    assert_eq!(px(p(0, 3)), Some(Rect::new(76, 24, 24, 24)));
    assert_eq!(px(p(20, 2)), None);
    let cells = s.tile_cells(&scene, MAP_AREA, p(0, 3));
    assert_eq!(cells, Some(Rect::new(9, 1, 4, 2)));
    // A scene smaller than the area: only its tiles.
    let small = s.grid(&MapScene::<Piece>::new(p(0, 0), (2, 1)), MAP_AREA);
    assert_eq!(small.tiles, (2, 1));
}

#[test]
fn the_zoom_multiplies_the_tile_and_divides_the_view() {
    let kit = Kit::new();
    let s = skin(&kit);
    assert_eq!(s.zoom(), 1);
    let twice = s.clone().zoomed(2);
    assert_eq!((twice.zoom(), twice.tile_size()), (2, (48, 48)));
    // 560 × 480 px of 48 px tiles: 11 × 10, 16 px to spare each side.
    assert_eq!(twice.view_tiles(MAP_AREA), (11, 10));
    let scene = MapScene::<Piece>::new(p(0, 0), (11, 10));
    let grid = twice.grid(&scene, MAP_AREA);
    assert_eq!((grid.corner, grid.tile, grid.scale), ((16, 0), (48, 48), 2));
    assert_eq!(s.grid(&scene, MAP_AREA).scale, 1);
    assert_eq!(
        twice.tile_px(&scene, MAP_AREA, p(2, 1)),
        Some(Rect::new(112, 48, 48, 48))
    );
    // A menu beside a tile: the cells it touches (8 × 16 px each).
    assert_eq!(
        twice.tile_cells(&scene, MAP_AREA, p(2, 1)),
        Some(Rect::new(14, 3, 6, 3))
    );
    // 16 px tiles: 35 × 30 and 17 × 15.
    let small = sized(&kit, 16, 16);
    assert_eq!(small.view_tiles(MAP_AREA), (35, 30));
    assert_eq!(small.zoomed(2).view_tiles(MAP_AREA), (17, 15));
    // A zoom below once is once.
    assert_eq!(s.clone().zoomed(0), s);
    assert_eq!(s.clone().zoomed(-3).zoom(), 1);
    // Pictures on glyph ground stay on font cells: no zoom.
    let mixed = sheets();
    let forced = mixed.clone().zoomed(2);
    assert_eq!(forced.tile_size(), (16, 16));
    assert_eq!(forced.view_tiles(MAP_AREA), mixed.view_tiles(MAP_AREA));
    assert_eq!(forced.grid(&scene, MAP_AREA).scale, 1);
}

/// A skin kept as a `dyn GridSkin` is asked for its zoom levels, and gives
/// the same skin at another of them: the same tiles, a bigger tile size.
#[test]
fn a_zoomable_skin_gives_itself_at_each_of_its_levels() {
    let kit = Kit::new();
    // Sorted, each once, nothing below 1.
    let zoomable = skin(&kit).with_zooms(&[3, 1, 0, 2, 1, -2]);
    let s: Rc<dyn GridSkin<Piece>> = Rc::new(zoomable.clone());
    assert_eq!(s.zoom_levels(), [1, 2, 3]);
    assert_eq!(s.zoom(), Some(1));
    let twice = s.at_zoom(2).unwrap();
    assert_eq!(twice.zoom(), Some(2));
    assert_eq!(twice.zoom_levels(), [1, 2, 3]);
    assert_eq!((s.tile_size(), twice.tile_size()), ((24, 24), (48, 48)));
    assert_eq!(twice.name(), s.name());
    assert_eq!(twice.walk(), s.walk());
    // Back again, and to a level it lacks: none.
    assert_eq!(twice.at_zoom(1).unwrap().tile_size(), (24, 24));
    assert!(s.at_zoom(4).is_none());
    assert!(s.at_zoom(0).is_none());
    // The concrete skin keeps its levels and its own zoom.
    assert_eq!(zoomable.clone().zoomed(3).zoom(), 3);
    assert_eq!(zoomable, skin(&kit).with_zooms(&[1, 2, 3]));
    // Without levels it doesn't zoom, whatever it draws at.
    let fixed: &dyn GridSkin<Piece> = &skin(&kit).zoomed(2);
    assert!(fixed.zoom_levels().is_empty());
    assert_eq!(fixed.zoom(), None);
    assert!(fixed.at_zoom(2).is_none());
    // Nor on glyph ground, levels or not.
    let mixed: &dyn GridSkin<Piece> = &sheets().with_zooms(&[1, 2]);
    assert!(mixed.zoom_levels().is_empty());
    assert_eq!(mixed.zoom(), None);
    assert!(mixed.at_zoom(1).is_none());
    // Nor the glyph skin: its tile is two font cells.
    let glyph: &dyn GridSkin<Piece> = &GLYPH;
    assert!(glyph.zoom_levels().is_empty());
    assert_eq!(glyph.zoom(), None);
    assert!(glyph.at_zoom(1).is_none());
}

/// A scene with one of everything, painted at 1× into 2 × 2 tiles from
/// the console's corner and at 2× into an area twice the size: the same
/// items in the same order, every one twice as far from the corner and
/// twice as big (tiles, pieces and what lies under them, the path and the
/// ranges), each showing the same part of the same image. (Not the
/// cursor: its marks have a size for each zoom.)
#[test]
fn at_twice_the_zoom_everything_is_twice_the_size() {
    let kit = Kit::new();
    let s = sprite(Some(layered(kit.tiles_image())));
    let twice = s.clone().zoomed(2);
    let double = |r: PxRect| Rect::new(2 * r.x, 2 * r.y, 2 * r.w, 2 * r.h);
    let mut scene = MapScene::new(p(0, 0), (2, 2));
    scene.set_terrain(|pos| Some(if pos == p(1, 1) { WATER } else { GRASS }));
    scene.tint([p(0, 0)], RangeName("move"));
    scene.tile_mut(p(1, 0)).unwrap().flashes.push(0.5);
    scene.push_unit(Piece::new(1, 'y', p(0, 1)));
    scene.push_unit(Piece::new(2, 'f', p(1, 0)));
    scene.path = vec![p(0, 0), p(0, 1), p(1, 1)];
    scene.threat = Some(threat(p(0, 0), &[p(1, 0)]));
    let paint = |skin: &SpriteSkin, area| {
        let mut buf = blank();
        let cards = Cards { card: kit.card() };
        skin.paint(&kit.looks(), &scene, area, &cards, &mut buf);
        buf
    };
    // 48 × 48 px, and 96 × 96 px.
    let small = paint(&s, Rect::new(0, 0, 6, 3));
    let big = paint(&twice, Rect::new(0, 0, 12, 6));
    assert!(small.items().len() > 20, "{}", small.items().len());
    assert_eq!(small.items().len(), big.items().len());
    for (one, two) in small.items().iter().zip(big.items()) {
        let expected = match *one {
            Item::Rect(o) => Item::Rect(Overlay {
                rect: double(o.rect),
                ..o
            }),
            Item::Sprite(sprite) => Item::Sprite(Sprite {
                dest: double(sprite.dest),
                clip: double(sprite.clip),
                base: sprite.base.map(double),
                ..sprite
            }),
        };
        assert_eq!(*two, expected);
    }
}

#[test]
fn a_tile_is_its_terrain_sprite() {
    let kit = Kit::new();
    let s = skin(&kit);
    let buf = painted(&kit, &s, &plains());
    let own = |id| src_rect(tiles(kit.tiles_image()).look.tiles[&id]);
    let sprites = buf.sprites();
    // 2 × 2 tiles fit, from (16, 24).
    assert_eq!(sprites.len(), 4);
    let at = |x, y| Rect::new(x, y, 24, 24);
    let expect = [
        (own(GRASS), at(16, 24)),
        (own(WATER), at(40, 24)),
        (own(GRASS), at(16, 48)),
        (own(GRASS), at(40, 48)),
    ];
    for (sprite, (src, dest)) in sprites.iter().zip(expect) {
        assert_eq!((sprite.src, sprite.dest, sprite.clip), (src, dest, dest));
        assert_eq!(sprite.image, kit.tiles_image());
        let look = (sprite.layer, sprite.opacity, sprite.flip_x);
        assert_eq!(look, (Layer::Under, 255, false));
    }
    assert_ne!(own(GRASS), own(WATER));
    // No cell changes.
    let cells = |b: &GlyphBuffer| {
        let all = (0..32).flat_map(|y| (0..100).map(move |x| (x, y)));
        all.map(|(x, y)| *b.get(x, y).unwrap()).collect::<Vec<_>>()
    };
    assert_eq!(cells(&buf), cells(&blank()));
    // Tiles off the map, or of a terrain the tiles lack: nothing.
    let mut scene = plains();
    scene.tiles[0].terrain = None;
    scene.tiles[1].terrain = Some(TerrainId(999));
    assert_eq!(painted(&kit, &s, &scene).sprites().len(), 2);
}

/// Game ticket 0444: thick marks with a dark edge at the art's own size,
/// thin ones with a dark edge zoomed in; a pixel longer as large corners;
/// the glow style is the corners.
#[test]
fn the_cursor_is_dark_edged_corner_marks_inside_its_tile() {
    let kit = Kit::new();
    let looks = kit.looks();
    let s = skin(&kit);
    let cursor = looks.cursor(&GLYPH.colors);
    let black = looks.dark();
    let painted_with = |skin: &SpriteSkin, pos, style, brightness| {
        let mut scene = plains();
        scene.cursor = Some(CursorView {
            pos,
            brightness,
            style,
        });
        painted(&kit, skin, &scene)
    };
    let buf = painted_with(&s, p(1, 1), CursorStyle::Corners, 1.0);
    let r = Rect::new;
    let rects = |buf: &GlyphBuffer| buf.overlays().iter().map(|o| o.rect).collect::<Vec<_>>();
    // Tile (1, 1): pixels 40..64 × 48..72. The dark edges, then the marks
    // a pixel inside them.
    assert_eq!(
        rects(&buf),
        [
            r(40, 48, 6, 4),
            r(40, 48, 4, 6),
            r(58, 48, 6, 4),
            r(60, 48, 4, 6),
            r(40, 68, 6, 4),
            r(40, 66, 4, 6),
            r(58, 68, 6, 4),
            r(60, 66, 4, 6),
            r(41, 49, 4, 2),
            r(41, 49, 2, 4),
            r(59, 49, 4, 2),
            r(61, 49, 2, 4),
            r(41, 69, 4, 2),
            r(41, 67, 2, 4),
            r(59, 69, 4, 2),
            r(61, 67, 2, 4),
        ]
    );
    let looks_of = |buf: &GlyphBuffer, range: core::ops::Range<usize>, color| {
        let look = |o: &Overlay| (o.color, o.layer);
        buf.overlays()[range]
            .iter()
            .all(|o| look(o) == (color, Layer::Over))
    };
    assert!(looks_of(&buf, 0..8, black));
    assert!(looks_of(&buf, 8..16, cursor));
    // Large corners: a pixel longer. The marks pulse; the edge doesn't.
    let large = painted_with(&s, p(1, 1), CursorStyle::LargeCorners, 0.5);
    assert_eq!(large.overlays()[2].rect, r(57, 48, 7, 4));
    assert_eq!(large.overlays()[8].rect, r(41, 49, 5, 2));
    assert!(looks_of(&large, 0..8, black));
    assert!(looks_of(&large, 8..16, cursor.scale(0.5)));
    // The glow style: the corners, and nothing tints the tile.
    let glow = painted_with(&s, p(1, 1), CursorStyle::TileGlow, 1.0);
    assert_eq!(glow, buf);
    // At 2×, one 48 px tile from (16, 24): thin marks, 1 px of the art
    // thick and 3 long, their edge 1 px of the art.
    let twice = s.clone().zoomed(2);
    let zoomed = painted_with(&twice, p(0, 0), CursorStyle::Corners, 1.0);
    let all = rects(&zoomed);
    assert_eq!(all.len(), 16);
    assert_eq!((all[0], all[1]), (r(16, 24, 10, 6), r(16, 24, 6, 10)));
    assert_eq!((all[8], all[9]), (r(18, 26, 6, 2), r(18, 26, 2, 6)));
    assert_eq!((all[7], all[15]), (r(58, 62, 6, 10), r(60, 64, 2, 6)));
    let zoomed = painted_with(&twice, p(0, 0), CursorStyle::LargeCorners, 1.0);
    let all = rects(&zoomed);
    assert_eq!((all[0], all[8]), (r(16, 24, 12, 6), r(18, 26, 8, 2)));
    // A cursor off the view: nothing.
    let off = painted_with(&s, p(2, 0), CursorStyle::Corners, 1.0);
    assert_eq!(off, painted(&kit, &s, &plains()));
}

#[test]
fn the_path_runs_under_the_pieces_and_its_arrowhead_over_them() {
    let kit = Kit::new();
    let looks = kit.looks();
    let s = skin(&kit);
    let mut scene = plains();
    scene.push_unit(Piece::new(4, 'f', p(1, 1)));
    scene.path = vec![p(0, 1), p(1, 1)];
    let buf = painted(&kit, &s, &scene);
    let path = looks.path(&GLYPH.colors);
    let kinds: Vec<&str> = buf
        .items()
        .iter()
        .map(|i| match i {
            Item::Sprite(s) if s.image == kit.card() => "card",
            Item::Sprite(_) => "tile",
            Item::Rect(o) if o.color == path => "path",
            Item::Rect(_) => "under",
        })
        .collect();
    // Four tiles; what lies under the piece; the line; the piece; the
    // arrowhead.
    let mut expect = vec!["tile"; 4];
    expect.extend(["under", "path", "card"]);
    expect.extend(["path"; 6]);
    assert_eq!(kinds, expect);
    // Through the centres of 24 px tiles: pixels 11..=13 of each.
    let line = rect_at(&buf, 5);
    assert_eq!(
        (line.rect, line.layer),
        (Rect::new(40, 59, 14, 3), Layer::Under)
    );
}

/// The threat arrows and marks are the last things painted, over the
/// pieces and the cursor, on the skin's own tiles at its zoom: with
/// terrain tiles and without, from the middle of the foe's tile.
#[test]
fn threat_arrows_and_marks_are_painted_last_on_the_skins_tiles() {
    let kit = Kit::new();
    let colors = kit.looks().threat(&GLYPH.colors);
    // 192 × 128 px: the whole 3 × 2 view even at twice the size.
    let area = Rect::new(2, 1, 24, 8);
    let skins = [
        skin(&kit),
        skin(&kit).zoomed(2),
        sheets(),
        sheets().zoomed(2),
    ];
    for skin in skins {
        let mut scene = plains();
        scene.push_unit(Piece::new(4, 'f', p(1, 1)));
        scene.cursor = Some(CursorView {
            pos: p(1, 1),
            brightness: 1.0,
            style: CursorStyle::Corners,
        });
        let paint = |scene: &MapScene<Piece>| {
            let mut buf = blank();
            let cards = Cards { card: kit.card() };
            skin.paint(&kit.looks(), scene, area, &cards, &mut buf);
            buf
        };
        let before = paint(&scene).items().len();
        scene.threat = Some(threat(p(0, 0), &[p(1, 1)]));
        let buf = paint(&scene);
        let grid = skin.grid(&scene, area);
        let arrows = threat(p(0, 0), &[p(1, 1)]);
        let expect: Vec<Item> =
            threat::threat_overlays(&arrows, &grid, threat::Start::Middle, colors)
                .into_iter()
                .map(Item::Rect)
                .collect();
        let name = GridSkin::<Piece>::name(&skin);
        assert!(expect.len() > 4, "{name}");
        assert_eq!(&buf.items()[before..], expect, "{name}");
    }
}

/// One change to a scene the skin must show.
struct Feature {
    name: &'static str,
    without: MapScene<Piece>,
    with: MapScene<Piece>,
}

/// Every feature of a scene the skin paints itself, each as a scene
/// without it and the same scene with it. (Its units are the painter's.)
fn features() -> Vec<Feature> {
    let mut base = plains();
    base.push_unit(Piece::new(4, 'f', p(1, 1)));
    let feature = |name, change: &dyn Fn(&mut MapScene<Piece>)| {
        let mut with = base.clone();
        change(&mut with);
        Feature {
            name,
            without: base.clone(),
            with,
        }
    };
    let mut out = vec![
        feature("terrain", &|s| s.tiles[0].terrain = Some(WATER)),
        feature("flash", &|s| s.tiles[0].flashes.push(0.5)),
        feature("becomes", &|s| s.tiles[0].becomes = Some(WATER)),
        feature("unit pos", &|s| s.units[0].pos = p(0, 1)),
        feature("path", &|s| s.path = vec![p(0, 0), p(1, 0)]),
        feature("threat", &|s| s.threat = Some(threat(p(0, 0), &[p(1, 1)]))),
    ];
    for kind in ["move", "attack", "danger", "heal"] {
        out.push(feature("range", &move |s| {
            s.tint([p(0, 0)], RangeName(kind));
        }));
    }
    for style in [
        CursorStyle::Corners,
        CursorStyle::LargeCorners,
        CursorStyle::TileGlow,
    ] {
        let at = move |brightness| CursorView {
            pos: p(0, 1),
            brightness,
            style,
        };
        out.push(feature("cursor", &move |s| s.cursor = Some(at(1.0))));
        let mut dim = feature("cursor brightness", &move |s| s.cursor = Some(at(0.5)));
        dim.without.cursor = Some(at(1.0));
        out.push(dim);
    }
    out
}

/// For each feature of a scene, the skin's frame with it differs from the
/// frame without it: nothing in a scene goes unpainted (game ADR-0038).
/// With terrain tiles and on the glyph skin's ground.
#[test]
fn every_scene_feature_is_painted() {
    let kit = Kit::new();
    for s in [skin(&kit), sheets()] {
        let features = features();
        assert_eq!(features.len(), 6 + 4 + 6);
        for f in features {
            assert_ne!(f.without, f.with, "{}: no change", f.name);
            let (without, with) = (painted(&kit, &s, &f.without), painted(&kit, &s, &f.with));
            let name = GridSkin::<Piece>::name(&s);
            assert_ne!(without, with, "{} under {name}: not painted", f.name);
        }
    }
}

/// The ring of terrain round the view and the map's look change the frame
/// of tiles that have layers and another look (and of no other: tiles
/// without them have nothing to paint them with).
#[test]
fn the_ring_and_the_look_are_painted_by_tiles_that_have_layers_and_looks() {
    let kit = Kit::new();
    let s = sprite(Some(with_indoor(layered(kit.tiles_image()))));
    let base = plains();
    let mut ring = base.clone();
    // Left of the view's first tile.
    ring.rim[5] = Some(WATER);
    let mut indoor = base.clone();
    indoor.look.tiles = "indoor".to_owned();
    for (name, with) in [("rim", &ring), ("look", &indoor)] {
        assert_ne!(base, *with, "{name}: no change");
        let (without, with) = (painted(&kit, &s, &base), painted(&kit, &s, with));
        assert_ne!(without, with, "{name}: not painted");
    }
    for plain in [skin(&kit), sheets()] {
        assert_eq!(painted(&kit, &plain, &base), painted(&kit, &plain, &ring));
        assert_eq!(painted(&kit, &plain, &base), painted(&kit, &plain, &indoor));
    }
}

/// The map's light changes the frame of every skin with terrain tiles
/// (game ADR-0058), and of none that paints the glyph skin's ground.
#[test]
fn the_light_is_painted_by_tiles() {
    let kit = Kit::new();
    let base = plains();
    let mut dusk = base.clone();
    dusk.look.light = "dusk".to_owned();
    assert_ne!(base, dusk);
    for s in [skin(&kit), sprite(Some(layered(kit.tiles_image())))] {
        assert_ne!(painted(&kit, &s, &base), painted(&kit, &s, &dusk));
    }
    let s = sheets();
    assert_eq!(painted(&kit, &s, &base), painted(&kit, &s, &dusk));
}

#[test]
fn without_terrain_tiles_the_ground_is_the_glyph_skins() {
    let kit = Kit::new();
    let looks = kit.looks();
    let s = sheets();
    // The glyph skin's tiles: 16 × 16 px, 35 × 30 in the map area.
    assert_eq!(s.tile_size(), (16, 16));
    assert_eq!(s.view_tiles(MAP_AREA), (35, 30));
    let area = Rect::new(2, 1, 6, 2);
    let mut scene = plains();
    scene.tiles[2].terrain = Some(SAND);
    scene.tint([p(1, 0)], RangeName("move"));
    scene.path = vec![p(0, 0), p(1, 0)];
    scene.cursor = Some(CursorView {
        pos: p(2, 1),
        brightness: 1.0,
        style: CursorStyle::Corners,
    });
    assert_eq!(
        s.tile_px(&scene, area, p(1, 1)),
        GLYPH.tile_px(&scene, area, p(1, 1))
    );
    assert_eq!(
        s.tile_px(&scene, area, p(1, 1)),
        Some(Rect::new(32, 32, 16, 16))
    );
    // With no piece, the frame is the glyph skin's, its cursor and all.
    let paint = |skin: &dyn GridSkin<Piece>, scene: &MapScene<Piece>| {
        let mut buf = blank();
        skin.paint(&looks, scene, area, &NoUnits, &mut buf);
        buf
    };
    assert_eq!(paint(&s, &scene), paint(&GLYPH, &scene));
    // With one, its picture goes between the path's line and its
    // arrowhead; the cursor's marks stay last.
    scene.push_unit(Piece::new(4, 'f', p(1, 0)));
    let mut buf = blank();
    let cards = Cards { card: kit.card() };
    s.paint(&looks, &scene, area, &cards, &mut buf);
    let path = looks.path(&GLYPH.colors);
    let cursor = looks.cursor(&GLYPH.colors);
    let kinds: Vec<&str> = buf
        .items()
        .iter()
        .map(|i| match i {
            Item::Sprite(_) => "card",
            Item::Rect(o) if o.color == path => "path",
            Item::Rect(o) if o.color == cursor => "cursor",
            Item::Rect(_) => "under",
        })
        .collect();
    let mut expect = vec!["under", "path", "card"];
    expect.extend(["path"; 6]);
    expect.extend(["cursor"; 8]);
    assert_eq!(kinds, expect);
    // And the glyph skin's letters are the glyph skin's alone.
    let mut letters = blank();
    s.paint(&looks, &scene, area, &Letters, &mut letters);
    let mut glyphs = blank();
    GLYPH.paint(&looks, &scene, area, &Letters, &mut glyphs);
    assert_eq!(letters.get(4, 1), glyphs.get(4, 1));
}

proptest! {
    /// Whatever the scene, the area (even one partly off the console),
    /// the tile size and the zoom, the skin changes no cell and adds no
    /// item (rectangle or sprite) outside the area's pixels.
    #[test]
    fn sprite_skin_paints_only_the_area(
        scene in any_scene(),
        area in any_area(),
        tile in (8..=64u32, 8..=64u32),
        zoom in 1..=2i32,
    ) {
        let kit = Kit::new();
        let mut buf = blank();
        let skin = sized(&kit, tile.0, tile.1).zoomed(zoom);
        let cards = Cards { card: kit.card() };
        skin.paint(&kit.looks(), &scene, area, &cards, &mut buf);
        for y in 0..32 {
            for x in 0..100 {
                prop_assert_eq!(buf.get(x, y), Some(&fill()), "cell ({}, {})", x, y);
            }
        }
        let px = px_rect(area);
        for item in buf.items() {
            let seen = item.visible();
            prop_assert_eq!(seen.intersect(&px), Some(seen), "{:?} in {:?}", item, area);
        }
    }

    /// Whatever the scene and the area, the skin on the glyph skin's
    /// ground changes no cell outside the area and adds no item outside
    /// its pixels.
    #[test]
    fn sprite_units_skin_paints_only_the_area(scene in any_scene(), area in any_area()) {
        let kit = Kit::new();
        let mut buf = blank();
        let cards = Cards { card: kit.card() };
        sheets().paint(&kit.looks(), &scene, area, &cards, &mut buf);
        for y in 0..32 {
            for x in 0..100 {
                if !area.contains(x, y) {
                    prop_assert_eq!(buf.get(x, y), Some(&fill()), "cell ({}, {})", x, y);
                }
            }
        }
        let px = px_rect(area);
        for item in buf.items() {
            let seen = item.visible();
            prop_assert_eq!(seen.intersect(&px), Some(seen), "{:?} in {:?}", item, area);
        }
    }

    /// For any tile size at either zoom, every visible tile lies inside
    /// the area, and the tiles sit edge to edge without overlapping.
    #[test]
    fn tiles_lie_inside_the_area_edge_to_edge(
        area in any_area(),
        tile in (8..=64u32, 8..=64u32),
        origin in (-30..30i32, -30..30i32),
        zoom in 1..=2i32,
    ) {
        let kit = Kit::new();
        let s = sized(&kit, tile.0, tile.1).zoomed(zoom);
        let (w, h) = (side(tile.0) * zoom, side(tile.1) * zoom);
        let all = px_rect(area);
        prop_assert_eq!(s.view_tiles(area), (all.w.max(0) / w, all.h.max(0) / h));
        let scene = MapScene::<Piece>::new(Pos::new(origin.0, origin.1), s.view_tiles(area));
        let px = px_rect(area);
        let rect = |dx, dy| s.tile_px(&scene, area, Pos::new(origin.0 + dx, origin.1 + dy));
        for dy in 0..scene.size.1 {
            for dx in 0..scene.size.0 {
                let r = rect(dx, dy);
                prop_assert!(r.is_some());
                let r = r.unwrap_or(Rect::new(0, 0, 0, 0));
                prop_assert_eq!((r.w, r.h), (w, h));
                prop_assert_eq!(r.intersect(&px), Some(r), "{:?} in {:?}", r, area);
                if let Some(right) = rect(dx + 1, dy) {
                    prop_assert_eq!((right.x, right.y), (r.x + w, r.y));
                }
                if let Some(below) = rect(dx, dy + 1) {
                    prop_assert_eq!((below.x, below.y), (r.x, r.y + h));
                }
            }
        }
        let (across, down) = scene.size;
        prop_assert!(rect(across, 0).is_none() && rect(0, down).is_none());
        prop_assert!(rect(-1, 0).is_none() && rect(0, -1).is_none());
    }
}

#[test]
fn a_size_of_the_art_too_big_for_the_console_is_the_largest_there_is() {
    assert_eq!(side(24), 24);
    assert_eq!(side(u32::MAX), i32::MAX);
    let rect = ImageRect {
        x: 1,
        y: 2,
        w: 3,
        h: 4,
    };
    assert_eq!(src_rect(rect), Rect::new(1, 2, 3, 4));
}
