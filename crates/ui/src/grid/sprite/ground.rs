//! The ground a sprite skin paints from its terrain tiles (game
//! game ADR-0052): each tile's own picture, then the look's layers over them
//! (pictures between tiles by the [corner rule](crate::grid::corners),
//! and pictures on tiles), then what tints a tile. The pictures are
//! shaded by the light the scene's map is in (game ADR-0058); the tints
//! are not.
//!
//! A tile off the map gets no picture, and no part of one: a picture
//! between tiles is drawn only inside the tiles of the map it covers.

use crate::prelude::*;
use shuyi_assets::image::{ImageId, ImageRect, Picture};
use shuyi_assets::tileset::{CornerLayer, Layer, Look, TerrainTiles, TileLayer};

use super::tile_sprite;
use crate::frame::color::{EngineColor, Rgb, to_channel};
use crate::frame::glyph_buffer::{
    GlyphBuffer, Layer as Depth, Overlay, Paint, PxRect, Rect, Sprite,
};
use crate::grid::corners::{Shown, mix_of};
use crate::grid::glyph::OVERLAY_BLEND;
use crate::grid::layout::Grid;
use crate::grid::scene::{MapScene, RangeName, TileView, Tints};
use crate::grid::skin::Looks;

/// Paints `scene`'s ground in `grid` from `terrain`'s tiles, in the look
/// the scene's map asks for (the tiles' own if they lack that one) and in
/// its light (as the pictures are, if `looks` has no such light).
pub(super) fn paint<U>(
    looks: &Looks,
    terrain: &TerrainTiles,
    scene: &MapScene<U>,
    grid: &Grid,
    buf: &mut GlyphBuffer,
) {
    let light = looks.lights.get(&scene.look.light);
    let ground = Ground {
        paint: match light {
            Some(light) if !light.is_plain() => Paint::Shaded(light),
            _ => Paint::Image,
        },
        scene,
        grid,
        image: terrain.image,
        look: terrain.look(&scene.look.tiles),
        shown: Shown::of(scene),
        tints: scene.tints(),
    };
    ground.paint_tiles(buf);
    for layer in &ground.look.layers {
        match layer {
            Layer::Corners(layer) => ground.paint_corners(layer, buf),
            Layer::Tiles(layer) => ground.paint_singles(layer, buf),
        }
    }
    ground.paint_tints(looks, buf);
}

/// One scene's ground being painted.
struct Ground<'a, U> {
    scene: &'a MapScene<U>,
    grid: &'a Grid,
    /// The image every picture is in.
    image: ImageId,
    look: &'a Look,
    /// The terrain each tile is drawn as.
    shown: Shown,
    /// How the pictures are coloured: by the map's light.
    paint: Paint,
    /// The ranges on each tile.
    tints: Tints,
}

impl<U> Ground<'_, U> {
    /// The pixels of the tile `dx` right of and `dy` below the view's
    /// first, if it is drawn and on the map.
    fn tile(&self, dx: i32, dy: i32) -> Option<PxRect> {
        let (across, down) = self.grid.tiles;
        let drawn = (0..across).contains(&dx) && (0..down).contains(&dy);
        let on_map = self.scene.tile_at(dx, dy)?.terrain.is_some();
        (drawn && on_map).then(|| self.grid.rect_at(dx, dy))
    }

    /// Every drawn tile that is on the map: where it is in the view, and
    /// its pixels.
    fn tiles(&self) -> impl Iterator<Item = (i32, i32, PxRect)> + '_ {
        let (across, down) = self.grid.tiles;
        let places = (0..down).flat_map(move |dy| (0..across).map(move |dx| (dx, dy)));
        places.filter_map(|(dx, dy)| Some((dx, dy, self.tile(dx, dy)?)))
    }

    /// `picture` on the pixels `dest`, in the map's light.
    fn sprite(&self, picture: Picture, dest: PxRect) -> Sprite {
        tile_sprite(picture, dest).painted(self.paint)
    }

    /// `rect` of the tileset's image.
    const fn picture(&self, rect: ImageRect) -> Picture {
        Picture {
            image: self.image,
            rect,
        }
    }

    /// The own picture of the terrain shown on the tile `dx` right of and
    /// `dy` below the view's first, if the look has one.
    fn own(&self, dx: i32, dy: i32) -> Option<Picture> {
        let id = self.shown.at(dx, dy)?;
        self.look.tiles.get(&id).map(|&rect| self.picture(rect))
    }

    /// Each tile's own picture.
    fn paint_tiles(&self, buf: &mut GlyphBuffer) {
        for (dx, dy, rect) in self.tiles() {
            if let Some(picture) = self.own(dx, dy) {
                buf.add_sprite(self.sprite(picture, rect));
            }
        }
    }

    /// The pictures of `layer`: one centred on each point where four
    /// tiles meet, for the mix of those four being in the layer, drawn
    /// inside the ones that are drawn and on the map.
    fn paint_corners(&self, layer: &CornerLayer, buf: &mut GlyphBuffer) {
        let (w, h) = self.grid.tile;
        let (left, top) = self.grid.corner;
        let (across, down) = self.grid.tiles;
        for (i, j) in (0..=down).flat_map(|j| (0..=across).map(move |i| (i, j))) {
            let mix = mix_of(self.shown.corners(i, j), &layer.of);
            let Some(rect) = layer.tiles.get(usize::from(mix)).copied().flatten() else {
                continue;
            };
            let dest = Rect::new(left + i * w - w / 2, top + j * h - h / 2, w, h);
            let sprite = self.sprite(self.picture(rect), dest);
            let round = [(i - 1, j - 1), (i, j - 1), (i - 1, j), (i, j)];
            let parts: Vec<PxRect> = round
                .iter()
                .filter_map(|&(dx, dy)| self.tile(dx, dy)?.intersect(&dest))
                .collect();
            if parts.len() == round.len() {
                buf.add_sprite(sprite);
            } else {
                for clip in parts {
                    buf.add_sprite(Sprite { clip, ..sprite });
                }
            }
        }
    }

    /// The picture of `layer` on each of its tiles: the one for the mix
    /// of the tile's four neighbours being terrain the layer looks for.
    fn paint_singles(&self, layer: &TileLayer, buf: &mut GlyphBuffer) {
        for (dx, dy, rect) in self.tiles() {
            if self
                .shown
                .at(dx, dy)
                .is_some_and(|id| layer.of.contains(&id))
            {
                let sides = mix_of(self.shown.sides(dx, dy), &layer.beside);
                let picture = self.picture(layer.picture(sides));
                buf.add_sprite(self.sprite(picture, rect));
            }
        }
    }

    /// What tints each tile, over its pictures.
    fn paint_tints(&self, looks: &Looks, buf: &mut GlyphBuffer) {
        let black = looks.dark();
        let (across, down) = self.grid.tiles;
        for (dx, dy) in (0..down).flat_map(|dy| (0..across).map(move |dx| (dx, dy))) {
            let Some(tile) = self.scene.tile_at(dx, dy) else {
                continue;
            };
            let own = self.tile(dx, dy).and_then(|_| self.own(dx, dy));
            let rect = self.grid.rect_at(dx, dy);
            let tints = tints(looks, tile, self.tints.at(dx, dy));
            paint_tinted(buf, rect, own, &tints, black);
        }
    }
}

/// What tints `tile`, in `ranges`, in order, as colors and strengths: its
/// flashes (towards its terrain's glyph color, as on the glyph skin), then
/// its ranges. A tile that would change is drawn as what it would become,
/// so nothing tints it. (The cursor never tints a picture: game ticket
/// 0444.)
fn tints(looks: &Looks, tile: &TileView, ranges: &[RangeName]) -> Vec<(Rgb, f32)> {
    let mut out = Vec::new();
    if tile.becomes.is_none() {
        let fg = tile
            .terrain
            .and_then(|id| looks.terrains.glyphs(id))
            .map(|t| looks.named(t.fg, EngineColor::Text));
        if let Some(fg) = fg {
            out.extend(tile.flashes.iter().map(|&s| (fg, OVERLAY_BLEND * s)));
        }
        for &kind in ranges {
            out.push((looks.range(kind), OVERLAY_BLEND));
        }
    }
    out
}

/// Tinting a picture by each of `tints` in turn (as `Rgb::lerp` towards
/// the colour by the strength, the glyph skin's `blend_bg`) leaves
/// `kept × picture + added`: returns `kept` and `added`.
fn blend(tints: &[(Rgb, f32)]) -> (f32, [f32; 3]) {
    let mut kept = 1.0;
    let mut added = [0.0; 3];
    for &(color, t) in tints {
        let t = if t.is_nan() { 0.0 } else { t.clamp(0.0, 1.0) };
        kept *= 1.0 - t;
        for (a, c) in added.iter_mut().zip([color.r, color.g, color.b]) {
            *a = *a * (1.0 - t) + f32::from(c) * t;
        }
    }
    (kept, added)
}

/// Tints the tile on the pixels `rect` by `tints`. A tile with a picture
/// of its `own` gets that picture's shape over it, in the one colour the
/// tints add up to and as strong as they are together, so whatever is
/// painted there reads as tinted; a tile with none (off the map) is the
/// tints over `base`.
fn paint_tinted(
    buf: &mut GlyphBuffer,
    rect: PxRect,
    own: Option<Picture>,
    tints: &[(Rgb, f32)],
    base: Rgb,
) {
    let (kept, added) = blend(tints);
    let rgb = |[r, g, b]: [f32; 3]| Rgb::new(to_channel(r), to_channel(g), to_channel(b));
    match own {
        Some(picture) if kept < 1.0 => {
            let color = rgb(added.map(|a| a / (1.0 - kept)));
            let mut wash = tile_sprite(picture, rect).painted(Paint::Solid(color));
            wash.opacity = to_channel(255.0 * (1.0 - kept));
            buf.add_sprite(wash);
        }
        None if !tints.is_empty() => {
            let base = [base.r, base.g, base.b];
            let mut color = added;
            for (c, b) in color.iter_mut().zip(base) {
                *c += kept * f32::from(b);
            }
            buf.add_overlay(Overlay::new(rect, rgb(color), Depth::Under));
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use shuyi_core::{Pos, TerrainId};

    use super::*;
    use crate::frame::glyph_buffer::Item;
    use crate::grid::layout::px_rect;
    use crate::grid::skin::GridSkin;
    use crate::grid::sprite::SpriteSkin;
    use crate::grid::sprite::src_rect;
    use crate::grid::sprite::tests::{blank, p, plains, rect_at, sprite_at};
    use crate::grid::testing::{any_area, any_scene};
    use crate::toy::grid::{
        BRIDGE, Cards, GRASS, Kit, Piece, WATER, layered, mix_rect, sprite, tiles, with_indoor,
    };

    /// 9 × 5 cells: 72 × 80 px, so 3 × 3 tiles of 24 px with 4 px to
    /// spare above and below. The first tile's corner is (0, 4).
    const AREA: Rect = Rect::new(0, 0, 9, 5);

    const MOVE: RangeName = RangeName("move");
    const DANGER: RangeName = RangeName("danger");
    const ATTACK: RangeName = RangeName("attack");
    const HEAL: RangeName = RangeName("heal");

    /// The skin of the toy's tiles with layers: a shore between the water
    /// (and the bridge) and the rest, a picture for every mix; then a
    /// bridge that has another picture with water to its left and right.
    fn shore(kit: &Kit) -> SpriteSkin {
        sprite(Some(layered(kit.tiles_image())))
    }

    /// A 3 × 3 view of `rows` (`.` grass, `~` water, `=` bridge, a space
    /// off the map), with the same terrain running on round it.
    fn view(rows: [&str; 3]) -> MapScene<Piece> {
        let mut scene = MapScene::new(p(0, 0), (3, 3));
        scene.set_terrain(|pos| {
            let row = rows.get(usize::try_from(pos.y.clamp(0, 2)).ok()?)?;
            match row.chars().nth(usize::try_from(pos.x.clamp(0, 2)).ok()?)? {
                '.' => Some(GRASS),
                '~' => Some(WATER),
                '=' => Some(BRIDGE),
                _ => None,
            }
        });
        scene
    }

    fn painted(kit: &Kit, skin: &SpriteSkin, scene: &MapScene<Piece>) -> GlyphBuffer {
        let mut buf = blank();
        let cards = Cards { card: kit.card() };
        skin.paint(&kit.looks(), scene, AREA, &cards, &mut buf);
        buf
    }

    /// The sprites of `buf` from the `from`th on: each one's source
    /// rectangle and the pixels it is drawn in.
    fn drawn(buf: &GlyphBuffer, from: usize) -> Vec<(PxRect, PxRect)> {
        let sprites = buf.sprites();
        sprites[from..].iter().map(|s| (s.src, s.clip)).collect()
    }

    /// The own tile of `id` in the toy's tiles.
    fn own(kit: &Kit, id: TerrainId) -> PxRect {
        src_rect(tiles(kit.tiles_image()).look.tiles[&id])
    }

    #[test]
    fn a_layer_draws_a_picture_round_each_point_where_its_tiles_meet_others() {
        let kit = Kit::new();
        let s = shore(&kit);
        let lake = view(["...", ".~.", "..."]);
        let buf = painted(&kit, &s, &lake);
        // Each tile's own picture first: nine, the water in the middle.
        let sprites = buf.sprites();
        for (i, sprite) in sprites[..9].iter().enumerate() {
            let id = if i == 4 { WATER } else { GRASS };
            let (dx, dy) = (i32::try_from(i % 3).unwrap(), i32::try_from(i / 3).unwrap());
            let tile = Rect::new(24 * dx, 4 + 24 * dy, 24, 24);
            assert_eq!(
                (sprite.src, sprite.dest, sprite.clip),
                (own(&kit, id), tile, tile)
            );
            assert_eq!(sprite.layer, Depth::Under);
        }
        // Then the shore: a picture centred on each corner of the water
        // tile, for the corner of it that is water.
        let cell = |x, y| Rect::new(x, y, 24, 24);
        assert_eq!(
            drawn(&buf, 9),
            [
                (src_rect(mix_rect(0b0001)), cell(12, 16)),
                (src_rect(mix_rect(0b0010)), cell(36, 16)),
                (src_rect(mix_rect(0b0100)), cell(12, 40)),
                (src_rect(mix_rect(0b1000)), cell(36, 40)),
            ]
        );
        for sprite in &sprites[9..] {
            assert_eq!((sprite.dest, sprite.layer), (sprite.clip, Depth::Under));
            assert_eq!((sprite.opacity, sprite.paint), (255, Paint::Image));
            assert_eq!(sprite.image, kit.tiles_image());
        }
        assert_eq!(buf.items().len(), 13);
        // No layer, no pictures between tiles.
        let plain = sprite(Some(tiles(kit.tiles_image())));
        assert_eq!(painted(&kit, &plain, &lake).items().len(), 9);
    }

    #[test]
    fn a_picture_between_tiles_is_drawn_only_inside_the_tiles_of_the_map() {
        let kit = Kit::new();
        let s = shore(&kit);
        // Water down the left column, running on past the view.
        let river = view(["~..", "~..", "~.."]);
        let buf = painted(&kit, &s, &river);
        let full = src_rect(mix_rect(0b1111));
        let bank = src_rect(mix_rect(0b1010));
        let r = Rect::new;
        assert_eq!(
            drawn(&buf, 9),
            [
                // Above the first row: only the halves inside the view.
                (full, r(0, 4, 12, 12)),
                (bank, r(12, 4, 12, 12)),
                (bank, r(24, 4, 12, 12)),
                // Between rows: the left edge cuts the water's in half;
                // the bank's is whole.
                (full, r(0, 16, 12, 12)),
                (full, r(0, 28, 12, 12)),
                (bank, r(12, 16, 24, 24)),
                (full, r(0, 40, 12, 12)),
                (full, r(0, 52, 12, 12)),
                (bank, r(12, 40, 24, 24)),
                // Below the last row.
                (full, r(0, 64, 12, 12)),
                (bank, r(12, 64, 12, 12)),
                (bank, r(24, 64, 12, 12)),
            ]
        );
        // A part keeps its picture's place: the whole picture's pixels.
        let top = buf.sprites()[10];
        assert_eq!(top.dest, r(12, -8, 24, 24));
        // A tile off the map has no picture and no part of one: of a map
        // of one tile, only the quarters inside it.
        let mut gap = MapScene::<Piece>::new(p(0, 0), (3, 3));
        gap.tiles[0].terrain = Some(WATER);
        let buf = painted(&kit, &s, &gap);
        assert_eq!(buf.sprites().len(), 1 + 4);
        assert_eq!(drawn(&buf, 0)[0].1, r(0, 4, 24, 24));
        for (src, clip) in drawn(&buf, 1) {
            assert_eq!(src, full);
            assert_eq!(clip.intersect(&r(0, 4, 24, 24)), Some(clip));
            assert_eq!((clip.w, clip.h), (12, 12));
        }
        // A view wider than the area: the tiles that don't fit get none.
        let mut wide = MapScene::<Piece>::new(p(0, 0), (5, 1));
        wide.set_terrain(|_| Some(WATER));
        let buf = painted(&kit, &s, &wide);
        // Three tiles; above and below each, a half of the picture on
        // each of its upper and lower corners.
        assert_eq!(buf.sprites().len(), 3 + 2 * 6);
        let px = px_rect(AREA);
        for sprite in buf.sprites() {
            assert_eq!(sprite.clip.intersect(&px), Some(sprite.clip));
        }
    }

    #[test]
    fn an_odd_tile_is_cut_where_its_pictures_meet() {
        let kit = Kit::new();
        let mut terrain = layered(kit.tiles_image());
        terrain.tile_px = (9, 13);
        let s = sprite(Some(terrain));
        let mut scene = MapScene::<Piece>::new(p(0, 0), (2, 2));
        scene.set_terrain(|pos| Some(if pos.x <= 0 { WATER } else { GRASS }));
        let mut buf = blank();
        // 3 × 2 cells: 24 × 32 px, so 2 × 2 tiles of 9 × 13 px from (3, 3).
        s.paint(
            &kit.looks(),
            &scene,
            Rect::new(0, 0, 3, 2),
            &crate::grid::NoUnits,
            &mut buf,
        );
        let r = Rect::new;
        let clips: Vec<PxRect> = drawn(&buf, 4).into_iter().map(|(_, clip)| clip).collect();
        // Each picture starts 4 px left of and 6 px above its point.
        assert_eq!(
            clips,
            [
                r(3, 3, 5, 7),
                r(8, 3, 4, 7),
                r(12, 3, 5, 7),
                r(3, 10, 5, 6),
                r(3, 16, 5, 7),
                r(8, 10, 9, 13),
                r(3, 23, 5, 6),
                r(8, 23, 4, 6),
                r(12, 23, 5, 6),
            ]
        );
        assert_eq!(buf.sprites()[9].dest, r(8, 10, 9, 13));
    }

    #[test]
    fn a_tile_layers_picture_goes_by_the_neighbours_it_looks_for() {
        let kit = Kit::new();
        let s = shore(&kit);
        let plain = src_rect(mix_rect(15));
        let across = src_rect(mix_rect(0));
        let tile = |dx, dy| Rect::new(24 * dx, 4 + 24 * dy, 24, 24);
        // Water left and right of the bridge: its other picture.
        let buf = painted(&kit, &s, &view(["...", "~=~", "..."]));
        let last = *buf.sprites().last().unwrap();
        assert_eq!(
            (last.src, last.dest, last.clip),
            (across, tile(1, 1), tile(1, 1))
        );
        // Water above and below: its plain one.
        let buf = painted(&kit, &s, &view([".~.", ".=.", ".~."]));
        assert_eq!(buf.sprites().last().unwrap().src, plain);
        // At the view's edge the neighbour is the tile outside it: here
        // the bridge's own terrain, which isn't water.
        let buf = painted(&kit, &s, &view(["...", "=~.", "..."]));
        let last = *buf.sprites().last().unwrap();
        assert_eq!((last.src, last.dest), (plain, tile(0, 1)));
        // One picture for each bridge, after the shore's.
        let buf = painted(&kit, &s, &view(["=.=", "...", ".=."]));
        let bridges: Vec<PxRect> = buf.sprites().iter().rev().take(3).map(|s| s.dest).collect();
        assert_eq!(bridges, [tile(1, 2), tile(2, 0), tile(0, 0)]);
        // A tile off the map gets nothing, whatever it would become.
        let mut scene = view(["...", "~.~", "..."]);
        scene.tiles[4].terrain = None;
        scene.tiles[4].becomes = Some(BRIDGE);
        for (src, clip) in drawn(&painted(&kit, &s, &scene), 0) {
            assert_eq!(clip.intersect(&tile(1, 1)), None, "{src:?} at {clip:?}");
            assert!(src != across && src != plain);
        }
    }

    #[test]
    fn a_map_is_painted_in_the_look_it_names_if_the_tiles_have_it() {
        let kit = Kit::new();
        let s = sprite(Some(with_indoor(layered(kit.tiles_image()))));
        let mut lake = view(["...", ".~.", "..."]);
        let outdoor = painted(&kit, &s, &lake);
        assert_eq!(outdoor, painted(&kit, &shore(&kit), &lake));
        lake.look.tiles = "indoor".to_owned();
        let buf = painted(&kit, &s, &lake);
        assert_eq!(buf.sprites().len(), 9);
        assert!(buf.sprites().iter().all(|s| s.src.y == 72));
        // A look the tiles lack: their own.
        lake.look.tiles = "cave".to_owned();
        assert_eq!(painted(&kit, &s, &lake), outdoor);
    }

    #[test]
    fn a_maps_light_shades_its_ground_and_nothing_painted_over_it() {
        let kit = Kit::new();
        let s = shore(&kit);
        let dusk = kit.lights.get("dusk").unwrap();
        // A lake and a bridge (a picture on a tile), a range on the
        // first tile and another on the lake, and a piece.
        let mut scene = view(["...", ".~=", "..."]);
        scene.tint([p(0, 0)], MOVE);
        scene.tint([p(1, 1)], ATTACK);
        scene.units.push(Piece::new(4, 'f', p(2, 0)));
        let noon = painted(&kit, &s, &scene);
        assert!(
            noon.sprites()
                .iter()
                .all(|s| s.paint != Paint::Shaded(dusk))
        );
        scene.look.light = "dusk".to_owned();
        let lit = painted(&kit, &s, &scene);
        // The same items in the same places: only the paint differs, and
        // only on the ground's pictures (the tiles' image, in its own
        // colors).
        assert_eq!(noon.items().len(), lit.items().len());
        let (mut ground, mut washes, mut others) = (0, 0, 0);
        for (before, after) in noon.sprites().iter().zip(lit.sprites()) {
            let terrain = before.image == kit.tiles_image();
            match before.paint {
                Paint::Image if terrain && before.layer == Depth::Under => {
                    assert_eq!(after, before.painted(Paint::Shaded(dusk)));
                    ground += 1;
                }
                Paint::Solid(_) if terrain => {
                    assert_eq!(after, *before);
                    washes += 1;
                }
                _ => {
                    assert_eq!(after, *before);
                    others += 1;
                }
            }
        }
        // Nine tiles, the pictures between them, the bridge's own; the
        // ranges' washes; the piece's card.
        assert!(ground > 9 + 1, "{ground}");
        assert_eq!(washes, 2);
        assert_eq!(others, 1);
        let rects = |buf: &GlyphBuffer| -> Vec<Overlay> {
            let items = buf.items().iter();
            items
                .filter_map(|item| match item {
                    Item::Rect(r) => Some(*r),
                    Item::Sprite(_) => None,
                })
                .collect()
        };
        assert_eq!(rects(&noon), rects(&lit));
        // Noon by name, and a light the game doesn't have: as before.
        scene.look.light = "noon".to_owned();
        assert_eq!(painted(&kit, &s, &scene), noon);
        scene.look.light = "midnight".to_owned();
        assert_eq!(painted(&kit, &s, &scene), noon);
        // Dusk's shade is on the very first tile.
        scene.look.light = "dusk".to_owned();
        let first = sprite_at(&painted(&kit, &s, &scene), 0);
        assert_eq!(first.paint, Paint::Shaded(dusk));
    }

    #[test]
    fn a_tile_that_would_change_is_painted_as_what_it_would_become() {
        let kit = Kit::new();
        let s = shore(&kit);
        let mut scene = view(["...", "...", "..."]);
        scene.tiles[4].becomes = Some(WATER);
        // As the lake it would be: its neighbours' pictures join it.
        let lake = view(["...", ".~.", "..."]);
        assert_eq!(painted(&kit, &s, &scene), painted(&kit, &s, &lake));
        // Its ranges and flashes don't tint it.
        scene.tint([p(1, 1)], DANGER);
        scene.tiles[4].flashes.push(1.0);
        assert_eq!(painted(&kit, &s, &scene), painted(&kit, &s, &lake));
        // What the tiles lack becomes nothing: the tile has no picture.
        let mut scene = view(["...", "...", "..."]);
        scene.tiles[4].becomes = Some(TerrainId(999));
        assert_eq!(painted(&kit, &s, &scene).sprites().len(), 8);
    }

    #[test]
    fn tints_blend_as_the_glyph_skin_blends() {
        let red = Rgb::new(200, 0, 0);
        let blue = Rgb::new(0, 0, 255);
        assert_eq!(blend(&[]), (1.0, [0.0; 3]));
        let (kept, added) = blend(&[(red, 0.75)]);
        assert!((kept - 0.25).abs() < 1e-6);
        assert!((added[0] - 150.0).abs() < 1e-4 && added[1].abs() < 1e-6);
        // Bad strengths: none (NaN) or clamped.
        assert_eq!(blend(&[(red, f32::NAN)]), (1.0, [0.0; 3]));
        assert_eq!(blend(&[(red, -1.0)]), (1.0, [0.0; 3]));
        assert_eq!(blend(&[(red, 2.0)]), blend(&[(red, 1.0)]));
        // Whatever the tile's color, `kept × tile + added` is the tile
        // lerped towards each tint in turn.
        let tints = [(red, 0.75), (blue, 0.75), (Rgb::new(9, 200, 9), 0.3)];
        let (kept, added) = blend(&tints);
        for tile in [
            Rgb::new(0, 0, 0),
            Rgb::new(255, 255, 255),
            Rgb::new(30, 140, 60),
        ] {
            let glyph = tints.iter().fold(tile, |bg, &(c, t)| bg.lerp(c, t));
            let channels = [tile.r, tile.g, tile.b].into_iter().zip(added);
            let ours: Vec<f32> = channels.map(|(v, a)| kept * f32::from(v) + a).collect();
            for (o, g) in ours.iter().zip([glyph.r, glyph.g, glyph.b]) {
                assert!(
                    (o - f32::from(g)).abs() <= 1.5,
                    "{tile:?}: {ours:?} vs {glyph:?}"
                );
            }
        }
    }

    #[test]
    fn a_tinted_tile_has_its_own_shape_over_it_in_the_tints_color() {
        let kit = Kit::new();
        let looks = kit.looks();
        let s = shore(&kit);
        let mut scene = view(["...", ".~.", "..."]);
        scene.tint([p(0, 0)], MOVE);
        scene.tint([p(1, 1)], DANGER);
        scene.tint([p(1, 1)], ATTACK);
        let buf = painted(&kit, &s, &scene);
        // After the tiles and the shore: the grass tile's own shape in the
        // range's color, three quarters strong.
        let wash = sprite_at(&buf, 13);
        assert_eq!(
            (wash.src, wash.dest),
            (own(&kit, GRASS), Rect::new(0, 4, 24, 24))
        );
        assert_eq!(
            (wash.paint, wash.opacity),
            (Paint::Solid(looks.range(MOVE)), 191)
        );
        assert_eq!((wash.clip, wash.layer), (wash.dest, Depth::Under));
        // Two ranges: their mix, fifteen sixteenths strong.
        let both = sprite_at(&buf, 14);
        let (danger, attack) = (looks.range(DANGER), looks.range(ATTACK));
        assert_eq!(both.paint, Paint::Solid(danger.lerp(attack, 0.8)));
        assert_eq!((both.src, both.opacity), (own(&kit, WATER), 239));
        assert_eq!(both.dest, Rect::new(24, 28, 24, 24));
        assert_eq!(buf.items().len(), 15);
        // Off the map, a range tints the dark under it.
        let mut scene = plains();
        scene.tiles[0].terrain = None;
        scene.tint([p(0, 0)], HEAL);
        let buf = painted(&kit, &s, &scene);
        // After the five tiles on the map, and the shore round the water.
        let off = buf
            .items()
            .iter()
            .find_map(|item| match item {
                Item::Rect(o) => Some(*o),
                Item::Sprite(_) => None,
            })
            .unwrap();
        assert_eq!(
            off.color,
            looks.dark().lerp(looks.range(HEAL), OVERLAY_BLEND)
        );
        assert_eq!(
            (off.rect, off.layer),
            (Rect::new(0, 4, 24, 24), Depth::Under)
        );
    }

    #[test]
    fn a_tile_is_tinted_only_as_strongly_as_its_tints_are() {
        let rect = Rect::new(8, 16, 24, 24);
        let kit = Kit::new();
        let own = Some(Picture {
            image: kit.tiles_image(),
            rect: tiles(kit.tiles_image()).look.tiles[&GRASS],
        });
        let base = Rgb::new(200, 100, 40);
        let red = Rgb::new(250, 0, 0);
        // Whatever is under a tile with no picture: here not black.
        let mut buf = blank();
        paint_tinted(&mut buf, rect, None, &[(red, 0.5)], base);
        let over = rect_at(&buf, 0);
        assert_eq!((over.rect, over.color), (rect, Rgb::new(225, 50, 20)));
        assert_eq!(over.color, base.lerp(red, 0.5));
        // No tints: nothing, with a picture or without.
        paint_tinted(&mut buf, rect, None, &[], base);
        paint_tinted(&mut buf, rect, own, &[], base);
        // Tints of no strength: nothing over a picture.
        paint_tinted(&mut buf, rect, own, &[(red, 0.0)], base);
        assert_eq!(buf.items().len(), 1);
        // The faintest tint is there.
        paint_tinted(&mut buf, rect, own, &[(red, 0.01)], base);
        let faint = sprite_at(&buf, 1);
        assert_eq!((faint.paint, faint.opacity), (Paint::Solid(red), 3));
        // A full one covers the tile.
        paint_tinted(&mut buf, rect, own, &[(red, 1.0)], base);
        assert_eq!(sprite_at(&buf, 2).opacity, 255);
    }

    #[test]
    fn flashes_tint_towards_the_terrain_color_then_the_ranges() {
        let kit = Kit::new();
        let looks = kit.looks();
        let fg = kit.palette.lookup("grass_fg").unwrap();
        let attack = looks.range(ATTACK);
        let mut tile = TileView {
            terrain: Some(GRASS),
            flashes: vec![1.0, 0.5],
            becomes: None,
        };
        assert_eq!(
            tints(&looks, &tile, &[ATTACK]),
            [
                (fg, OVERLAY_BLEND),
                (fg, OVERLAY_BLEND * 0.5),
                (attack, OVERLAY_BLEND),
            ]
        );
        // A tile that would change: nothing.
        tile.becomes = Some(WATER);
        let found = tints(&looks, &tile, &[ATTACK]);
        assert!(found.is_empty(), "{found:?}");
        // Off the map, a flash has no color to go towards.
        let off = TileView {
            terrain: None,
            flashes: vec![1.0],
            becomes: None,
        };
        assert_eq!(tints(&looks, &off, &[ATTACK]), [(attack, OVERLAY_BLEND)]);
        // A terrain the game lacks: no color either.
        let unknown = TileView {
            terrain: Some(TerrainId(999)),
            ..off
        };
        assert_eq!(tints(&looks, &unknown, &[]), []);
    }

    proptest! {
        /// Whatever the scene, the area and the tile size: every tile of
        /// the map that is drawn has exactly one picture of its own, on
        /// its own pixels (none if the tiles have none for what it
        /// shows); and every picture, or part of one, lies inside drawn
        /// tiles of the map.
        #[test]
        fn every_tile_of_the_map_has_one_picture_of_its_own_and_none_spills(
            scene in any_scene(),
            area in any_area(),
            tile in (8..=64u32, 8..=64u32),
        ) {
            let kit = Kit::new();
            let mut terrain = with_indoor(layered(kit.tiles_image()));
            terrain.tile_px = tile;
            let s = sprite(Some(terrain));
            // The ground alone: nothing on it, nothing tinting it.
            let mut scene = scene;
            scene.units.clear();
            scene.cursor = None;
            scene.path.clear();
            scene.threat = None;
            scene.ranges.clear();
            for tile in &mut scene.tiles {
                tile.flashes.clear();
            }
            let mut buf = blank();
            s.paint(&kit.looks(), &scene, area, &crate::grid::NoUnits, &mut buf);
            prop_assert!(buf.overlays().is_empty());
            let grid = s.grid(&scene, area);
            let shown = Shown::of(&scene);
            let (across, down) = grid.tiles;
            let on_map: Vec<(i32, i32, PxRect)> = (0..down)
                .flat_map(|dy| (0..across).map(move |dx| (dx, dy)))
                .filter(|&(dx, dy)| scene.tile_at(dx, dy).is_some_and(|t| t.terrain.is_some()))
                .map(|(dx, dy)| (dx, dy, grid.rect_at(dx, dy)))
                .collect();
            // A terrain's own tile is on the image's first row (the
            // outdoor look) or its fourth (the indoor one); the layers'
            // pictures are below them.
            let sprites = buf.sprites();
            let own = |sprite: &&Sprite| sprite.src.y < 96;
            // Every terrain has a tile; a made-up id past them has none.
            for &(dx, dy, rect) in &on_map {
                let pictures = sprites.iter().filter(own).filter(|s| s.dest == rect);
                // A tile wholly off the console has nothing to show.
                let seen = rect.intersect(&buf.pixel_bounds()).is_some();
                let has = seen && shown.at(dx, dy).is_some_and(|id| id.0 < 4);
                prop_assert_eq!(pictures.count(), usize::from(has), "tile ({}, {})", dx, dy);
            }
            let area_of = |r: PxRect| i64::from(r.w) * i64::from(r.h);
            for sprite in &sprites {
                let parts = on_map.iter().filter_map(|(_, _, tile)| tile.intersect(&sprite.clip));
                let covered: i64 = parts.map(area_of).sum();
                prop_assert_eq!(covered, area_of(sprite.clip), "{:?}", sprite);
            }
        }
    }

    #[test]
    fn an_empty_place_is_drawn_nowhere() {
        // Not a map at all: nothing.
        let kit = Kit::new();
        let s = shore(&kit);
        let empty = MapScene::<Piece>::new(Pos::new(0, 0), (3, 3));
        let found = painted(&kit, &s, &empty);
        assert!(found.items().is_empty(), "{:?}", found.items());
    }
}
