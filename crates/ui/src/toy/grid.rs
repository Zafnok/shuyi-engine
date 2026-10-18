//! The toy game's grid map (ticket 1115): four terrains, pieces that are a
//! letter each, the map's colors in the toy's palette, terrain tiles for
//! the sprite skin, both skins, a painter of the pieces for each, and a
//! stage: a 10 × 8 [`Field`] with two pieces, yours to move and one in the
//! way.

use crate::prelude::*;
use alloc::collections::{BTreeMap, BTreeSet};
use alloc::rc::Rc;
use core::any::Any;

use shuyi_assets::image::{ImageId, ImageInfo, ImageRect};
use shuyi_assets::lighting::{Light, Lighting};
use shuyi_assets::tileset::{
    CornerLayer, Layer as TileLayerKind, Look, MIXES, TerrainTiles, TileLayer,
};
use shuyi_core::{Dir, Grid as Map, Pos, Reach, TerrainId, reachable};

use super::{Action, Toy, ToyCtx};
use crate::frame::color::{EngineColor, Palette, Rgb};
use crate::frame::glyph_buffer::{Cell, GlyphBuffer, Layer, Overlay, PxRect, Rect, Sprite};
use crate::grid::glyph::{self, GlyphSkin};
use crate::grid::layout::Grid;
use crate::grid::scene::{CursorStyle, CursorView, MapScene, MapUnit, RangeName};
use crate::grid::skin::{
    GridSkin, Looks, MapColors, TerrainGlyphs, Terrains, UnitPainter, WalkSpeeds,
};
use crate::grid::sprite::SpriteSkin;
use crate::grid::{Camera, Cursor, steer, walk};
use crate::screen::{FrameInput, Screen, Transition};
use shuyi_assets::keymap::{ActionSet, UiAction};

/// Grass: every piece walks it.
pub const GRASS: TerrainId = TerrainId(0);
/// Water: no piece walks it.
pub const WATER: TerrainId = TerrainId(1);
/// A bridge over the water.
pub const BRIDGE: TerrainId = TerrainId(2);
/// Sand, whose colors the toy's palette lacks: drawn in the engine's.
pub const SAND: TerrainId = TerrainId(3);

/// Each terrain's name, glyphs and colors, by id.
const TERRAINS: [(&str, [char; 2], &str, &str); 4] = [
    ("grass", ['.', ','], "grass_fg", "grass_bg"),
    ("water", ['~', '~'], "water_fg", "water_bg"),
    ("bridge", ['=', '='], "bridge_fg", "bridge_bg"),
    ("sand", [':', '.'], "sand_fg", "sand_bg"),
];

/// The toy's terrains.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Terrain;

impl Terrain {
    /// Its name, glyphs and colors.
    fn of(id: TerrainId) -> Option<&'static (&'static str, [char; 2], &'static str, &'static str)> {
        TERRAINS.get(usize::from(id.0))
    }
}

impl Terrains for Terrain {
    fn name(&self, id: TerrainId) -> Option<&str> {
        Self::of(id).map(|t| t.0)
    }

    fn glyphs(&self, id: TerrainId) -> Option<TerrainGlyphs<'_>> {
        Self::of(id).map(|&(_, glyphs, fg, bg)| TerrainGlyphs { glyphs, fg, bg })
    }
}

/// The toy palette's names of the map's colors.
pub const COLORS: MapColors = MapColors {
    path: "path",
    cursor: "cursor",
    threat: "foe",
};

/// The map's colors in the toy's palette: no value is one of the engine's
/// colors'.
pub const MAP_PALETTE: [(&str, [u8; 3]); 14] = [
    ("grass_fg", [40, 160, 60]),
    ("grass_bg", [20, 90, 30]),
    ("water_fg", [120, 170, 250]),
    ("water_bg", [30, 60, 160]),
    ("bridge_fg", [200, 160, 110]),
    ("bridge_bg", [110, 80, 40]),
    ("path", [250, 250, 120]),
    ("cursor", [255, 230, 0]),
    ("foe", [230, 60, 50]),
    ("piece", PIECE_RGB),
    ("move", [60, 110, 230]),
    ("attack", [210, 60, 60]),
    ("danger", [150, 40, 150]),
    ("heal", [60, 200, 120]),
];

/// The pieces' color, the palette's `piece`.
const PIECE_RGB: [u8; 3] = [240, 240, 255];

/// [`PIECE_RGB`]: what the toy's painters paint its pieces in.
pub const PIECE: Rgb = Rgb::new(PIECE_RGB[0], PIECE_RGB[1], PIECE_RGB[2]);

/// The range of tiles a selected piece can move to.
pub const MOVE: RangeName = RangeName("move");

/// The glyph skin as the toy paints it: its pieces' bars are 2 pixels.
pub const GLYPH: GlyphSkin = GlyphSkin {
    colors: COLORS,
    walk: WalkSpeeds::of(10.0),
    foot_px: 2,
};

/// How fast a piece walks under the sprite skin: slower, to be seen.
pub const SPRITE_WALK: WalkSpeeds = WalkSpeeds {
    normal: 5.0,
    fast: 6.0,
    held: 10.0,
};

/// The image the toy's terrain tiles are in: 24 px tiles, 8 across and 6
/// down.
pub const TILES: &str = "grid_tiles.png";

/// The size of [`TILES`].
pub const TILES_SIZE: ImageInfo = ImageInfo {
    width: 192,
    height: 144,
};

/// The 24 px tile at `(column, row)` of [`TILES`].
pub const fn tile_at(column: u32, row: u32) -> ImageRect {
    ImageRect {
        x: 24 * column,
        y: 24 * row,
        w: 24,
        h: 24,
    }
}

/// The picture of mix `m` in [`layered`]'s shore: rows 4 and 5 of
/// [`TILES`].
pub const fn mix_rect(m: u8) -> ImageRect {
    // `m` is a mix: below 16.
    tile_at(m as u32 % 8, 4 + m as u32 / 8)
}

/// A look of each terrain's tile on `row` of [`TILES`], in id order, with
/// no layers.
fn row_look(row: u32) -> Look {
    Look {
        tiles: (0..4u16)
            .map(|i| (TerrainId(i), tile_at(u32::from(i), row)))
            .collect(),
        layers: Vec::new(),
    }
}

/// The toy's terrain tiles in `image`: each terrain's tile on row 0, in id
/// order.
pub fn tiles(image: ImageId) -> TerrainTiles {
    TerrainTiles {
        image,
        tile_px: (24, 24),
        look: row_look(0),
        looks: BTreeMap::new(),
    }
}

/// `tiles` with an indoor look: each terrain's tile on row 3.
pub fn with_indoor(mut tiles: TerrainTiles) -> TerrainTiles {
    tiles.looks.insert("indoor".to_owned(), row_look(3));
    tiles
}

/// [`tiles`] with layers: a shore between the water (and the bridge) and
/// the rest, a picture for every mix; then the bridge, which has another
/// picture with water to its left and right.
pub fn layered(image: ImageId) -> TerrainTiles {
    let mut tiles = tiles(image);
    let mut shore = [None; MIXES];
    for m in 1..16u8 {
        shore[usize::from(m)] = Some(mix_rect(m));
    }
    let shore = CornerLayer {
        of: BTreeSet::from([WATER, BRIDGE]),
        tiles: shore,
    };
    let mut sides = [None; MIXES];
    sides[0b0101] = Some(mix_rect(0));
    let bridge = TileLayer {
        of: BTreeSet::from([BRIDGE]),
        tile: mix_rect(15),
        beside: BTreeSet::from([WATER]),
        sides,
    };
    tiles.look.layers = vec![TileLayerKind::Corners(shore), TileLayerKind::Tiles(bridge)];
    tiles
}

/// The sprite skin of `terrain` (the glyph skin's ground without it), as
/// the toy paints it.
pub fn sprite(terrain: Option<TerrainTiles>) -> SpriteSkin {
    SpriteSkin::new(terrain, GLYPH, SPRITE_WALK)
}

/// What the toy paints its maps with besides a scene, for tests: its
/// palette, its images and its lights (noon, which changes nothing, and
/// dusk).
#[derive(Debug, Clone)]
pub struct Kit {
    /// The toy's day palette.
    pub palette: Palette,
    /// Its images: [`TILES`] and the card.
    pub images: shuyi_assets::image::ImageTable,
    /// Its lights.
    pub lights: Lighting,
}

impl Kit {
    /// The toy's.
    ///
    /// # Panics
    ///
    /// If the toy's palette lacks a color.
    pub fn new() -> Self {
        let toy = Toy::new();
        let palette = Palette::new::<EngineColor>(&toy.day).unwrap_or_else(|e| panic!("{e:?}"));
        Self {
            palette,
            images: toy.images,
            lights: toy.lights,
        }
    }

    /// The looks to paint with.
    pub fn looks(&self) -> Looks<'_> {
        Looks {
            palette: &self.palette,
            terrains: &Terrain,
            lights: &self.lights,
        }
    }

    /// The id of [`TILES`].
    ///
    /// # Panics
    ///
    /// If the toy's images lack it.
    pub fn tiles_image(&self) -> ImageId {
        self.images
            .id(TILES)
            .unwrap_or_else(|| panic!("no {TILES}"))
    }

    /// The id of the toy's card.
    ///
    /// # Panics
    ///
    /// If the toy's images lack it.
    pub fn card(&self) -> ImageId {
        let card = super::CARD;
        self.images.id(card).unwrap_or_else(|| panic!("no {card}"))
    }
}

impl Default for Kit {
    fn default() -> Self {
        Self::new()
    }
}

/// The toy's lights: noon, which changes nothing, and dusk.
pub fn lights() -> Lighting {
    Lighting {
        lights: BTreeMap::from([
            ("noon".to_owned(), Light::PLAIN),
            ("dusk".to_owned(), Light::new(0.3, [1.0, 0.8, 0.6])),
        ]),
    }
}

/// A piece on the toy's map: a letter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Piece {
    /// Which piece.
    pub id: u32,
    /// Its letter.
    pub letter: char,
    /// The tile it is shown on.
    pub pos: Pos,
}

impl Piece {
    /// Piece `id`, `letter`, on `pos`.
    pub const fn new(id: u32, letter: char, pos: Pos) -> Self {
        Self { id, letter, pos }
    }
}

impl MapUnit for Piece {
    type Id = u32;

    fn id(&self) -> u32 {
        self.id
    }

    fn pos(&self) -> Pos {
        self.pos
    }

    fn text(&self) -> String {
        format!(
            "#{} {} ({},{})",
            self.id, self.letter, self.pos.x, self.pos.y
        )
    }
}

/// The pieces as the glyph skin shows them: each its letter in the piece
/// color over its tile's left glyph, and a bar along its tile's foot, a
/// pixel in from each side.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Letters;

impl UnitPainter<Piece> for Letters {
    fn paint(&self, scene: &MapScene<Piece>, grid: &Grid, buf: &mut GlyphBuffer) {
        for piece in &scene.units {
            let (Some((x, y)), Some(tile)) =
                (glyph::cell_of(grid, piece.pos), grid.rect(piece.pos))
            else {
                continue;
            };
            if let Some(&cell) = buf.get(x, y) {
                let cell = Cell {
                    glyph: piece.letter,
                    fg: PIECE,
                    ..cell
                };
                buf.set(x, y, cell);
            }
            let bar = Rect::new(tile.x + 1, tile.y + tile.h - 2, tile.w - 2, 2);
            buf.add_overlay(Overlay::new(bar, PIECE, Layer::Over));
        }
    }
}

/// The pieces as the sprite skin shows them: under each a light square,
/// then the card, centred on its tile at the skin's zoom.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cards {
    /// The card's image.
    pub card: ImageId,
}

impl Cards {
    /// The card's place on the pixels `tile` of `grid`, at the art's size
    /// then zoomed: 16 × 16, centred.
    fn dest(grid: &Grid, tile: PxRect) -> PxRect {
        let art = grid.art_tile();
        let mark = Rect::new((art.w - 16) / 2, (art.h - 16) / 2, 16, 16);
        grid.zoomed((tile.x, tile.y), mark)
    }
}

impl UnitPainter<Piece> for Cards {
    fn paint_under(&self, scene: &MapScene<Piece>, grid: &Grid, buf: &mut GlyphBuffer) {
        for piece in &scene.units {
            if let Some(tile) = grid.rect(piece.pos) {
                buf.add_overlay(Overlay::new(tile, PIECE, Layer::Under));
            }
        }
    }

    fn paint(&self, scene: &MapScene<Piece>, grid: &Grid, buf: &mut GlyphBuffer) {
        let view = grid.bounds();
        for piece in &scene.units {
            let Some(tile) = grid.rect(piece.pos) else {
                continue;
            };
            let dest = Self::dest(grid, tile);
            let src = Rect::new(0, 0, 16, 16);
            let mut sprite = Sprite::new(self.card, src, dest, Layer::Over);
            if let Some(clip) = dest.intersect(&view) {
                sprite.clip = clip;
                buf.add_sprite(sprite);
            }
        }
    }
}

/// The field's width and height in tiles.
pub const FIELD: (u16, u16) = (10, 8);

/// The field: grass, a river down column 5 with a bridge on row 2, and a
/// patch of sand.
pub fn field_map() -> Map<TerrainId> {
    let mut map = Map::filled(FIELD.0, FIELD.1, GRASS);
    for y in 0..i32::from(FIELD.1) {
        let tile = if y == 2 { BRIDGE } else { WATER };
        if let Some(t) = map.get_mut(Pos::new(5, y)) {
            *t = tile;
        }
    }
    for pos in [Pos::new(1, 6), Pos::new(2, 6), Pos::new(2, 7)] {
        if let Some(t) = map.get_mut(pos) {
            *t = SAND;
        }
    }
    map
}

/// How many tiles a piece may walk.
pub const MOVES: u32 = 4;

/// The cells the field is shown in: the whole of the toy's 40 × 20
/// console.
pub const AREA: Rect = Rect::new(0, 0, 40, 20);

/// A piece on its way along its path.
#[derive(Debug, Clone, PartialEq)]
pub struct Walk {
    /// Which piece, in [`Field::pieces`].
    pub piece: usize,
    /// Its path, its own tile first.
    pub path: Vec<Pos>,
    /// Seconds since it set off.
    pub t: f32,
}

/// A stage of the toy game: the [`field_map`] with your piece `y` and a
/// foe `f` in the way. Confirm on your piece shows where it can go; moving
/// the cursor steers its path; Confirm on a tile it can reach walks it
/// there at the skin's pace; Cancel puts it down.
pub struct Field {
    /// The skin it is painted with.
    pub skin: Rc<dyn GridSkin<Piece>>,
    /// What paints its pieces under that skin.
    pub painter: Rc<dyn UnitPainter<Piece>>,
    /// The terrain.
    pub map: Map<TerrainId>,
    /// The pieces: yours first.
    pub pieces: Vec<Piece>,
    /// The cursor.
    pub cursor: Cursor,
    /// The camera.
    pub camera: Camera,
    /// Your piece's reach and its path, while it is picked up.
    pub selected: Option<(Reach, Vec<Pos>)>,
    /// A piece walking, if one is.
    pub walking: Option<Walk>,
    /// The animation clock, in milliseconds.
    pub clock_ms: u64,
}

impl core::fmt::Debug for Field {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Field")
            .field("skin", &self.skin.name())
            .field("pieces", &self.pieces)
            .field("cursor", &self.cursor.pos)
            .finish_non_exhaustive()
    }
}

impl Field {
    /// The field painted with `skin`, its pieces by `painter`.
    pub fn new(skin: Rc<dyn GridSkin<Piece>>, painter: Rc<dyn UnitPainter<Piece>>) -> Self {
        let pieces = vec![
            Piece::new(1, 'y', Pos::new(2, 3)),
            Piece::new(2, 'f', Pos::new(3, 4)),
        ];
        let cursor = Cursor::new(pieces[0].pos);
        let view = skin.view_tiles(AREA);
        let camera = Camera::centred_on(cursor.pos, FIELD.0, FIELD.1, view);
        Self {
            skin,
            painter,
            map: field_map(),
            pieces,
            cursor,
            camera,
            selected: None,
            walking: None,
            clock_ms: 0,
        }
    }

    /// The field under the glyph skin.
    pub fn glyphs() -> Self {
        Self::new(Rc::new(GLYPH), Rc::new(Letters))
    }

    /// The field under the sprite skin of the toy's terrain tiles, in
    /// `ctx`'s images.
    ///
    /// # Panics
    ///
    /// If the toy's images lack its tiles or its card.
    pub fn pictures(ctx: &ToyCtx) -> Self {
        let id = |path| {
            ctx.game
                .images
                .id(path)
                .unwrap_or_else(|| panic!("no {path}"))
        };
        let skin = sprite(Some(tiles(id(TILES))));
        Self::new(
            Rc::new(skin),
            Rc::new(Cards {
                card: id(super::CARD),
            }),
        )
    }

    /// Where your piece can stop: within [`MOVES`], off the water and
    /// the foe's tile.
    fn reach(&self) -> Reach {
        let foe = self.pieces.get(1).map(|p| p.pos);
        let cost = |pos: Pos| {
            let walkable = self.map.get(pos).is_some_and(|&t| t != WATER);
            (walkable && Some(pos) != foe).then_some(1)
        };
        reachable(FIELD.0, FIELD.1, self.pieces[0].pos, MOVES, cost)
    }

    /// What the field shows, in a view the skin fits in [`AREA`].
    pub fn scene(&self) -> MapScene<Piece> {
        let view = self.skin.view_tiles(AREA);
        let mut scene = MapScene::new(self.camera.origin, view);
        scene.set_terrain(|pos| self.map.get(pos).copied());
        if let Some((reach, path)) = &self.selected {
            scene.tint(reach.tiles().iter(), MOVE);
            scene.path.clone_from(path);
        }
        for (i, piece) in self.pieces.iter().enumerate() {
            let mut shown = *piece;
            if let Some(walk) = self.walking.as_ref().filter(|w| w.piece == i) {
                let tiles = walk.t * self.skin.walk().normal;
                let at = walk::steps(walk.path.len(), tiles);
                shown.pos = walk.path.get(at).copied().unwrap_or(piece.pos);
            }
            scene.push_unit(shown);
        }
        if self.walking.is_none() {
            scene.cursor = Some(CursorView {
                pos: self.cursor.pos,
                brightness: self.cursor.brightness(),
                style: CursorStyle::Corners,
            });
        }
        scene.clock_ms = self.clock_ms;
        scene
    }

    /// Moves the cursor `dir`, steering the path while a piece is picked
    /// up.
    fn step(&mut self, dir: Dir) {
        if !self.cursor.step(dir, FIELD.0, FIELD.1) {
            return;
        }
        let view = self.skin.view_tiles(AREA);
        self.camera
            .follow(self.cursor.pos, FIELD.0, FIELD.1, view, Camera::MARGIN);
        let to = self.cursor.pos;
        if let Some((reach, path)) = &mut self.selected {
            let legal = |path: &[Pos]| {
                path.windows(2).all(|w| Pos::manhattan(w[0], w[1]) == 1)
                    && path.iter().all(|&p| reach.contains(p))
                    && path.len() <= usize::try_from(MOVES).unwrap_or(usize::MAX) + 1
            };
            *path = steer(path, to, |to| reach.path_to(to), legal);
        }
    }

    /// Confirm: picks your piece up, or sets it walking to the cursor.
    fn confirm(&mut self) {
        let at = self.cursor.pos;
        match self.selected.take() {
            None if at == self.pieces[0].pos => {
                let reach = self.reach();
                self.selected = Some((reach, vec![at]));
            }
            None => {}
            Some((reach, path)) if reach.contains(at) && path.last() == Some(&at) => {
                self.walking = Some(Walk {
                    piece: 0,
                    path,
                    t: 0.0,
                });
            }
            Some(kept) => self.selected = Some(kept),
        }
    }
}

impl Screen<Toy> for Field {
    fn name(&self) -> &'static str {
        "field"
    }

    fn update(&mut self, _: &mut ToyCtx, input: &FrameInput<Action>) -> Transition<Toy> {
        let dt = if input.dt.is_finite() {
            input.dt.max(0.0)
        } else {
            0.0
        };
        self.cursor.tick(dt);
        // Whole milliseconds of a frame: far fewer than u64 holds.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let ms = (dt * 1000.0).round() as u64;
        self.clock_ms = self.clock_ms.saturating_add(ms);
        if let Some(walk) = &mut self.walking {
            walk.t += dt;
            let tiles = walk.t * self.skin.walk().normal;
            if walk::steps(walk.path.len(), tiles) + 1 >= walk.path.len() {
                let end = walk.path.last().copied();
                let piece = walk.piece;
                self.walking = None;
                if let (Some(end), Some(p)) = (end, self.pieces.get_mut(piece)) {
                    p.pos = end;
                }
            }
            return Transition::None;
        }
        for &action in &input.actions {
            match action.ui() {
                Some(UiAction::Up) => self.step(Dir::Up),
                Some(UiAction::Down) => self.step(Dir::Down),
                Some(UiAction::Left) => self.step(Dir::Left),
                Some(UiAction::Right) => self.step(Dir::Right),
                Some(UiAction::Confirm) => self.confirm(),
                Some(UiAction::Cancel) => self.selected = None,
                Some(UiAction::Debug) | None => {}
            }
        }
        Transition::None
    }

    fn draw(&self, ctx: &ToyCtx, buf: &mut GlyphBuffer) {
        let looks = Looks {
            palette: &ctx.palette,
            terrains: &Terrain,
            lights: &ctx.game.lights,
        };
        self.skin
            .paint(&looks, &self.scene(), AREA, self.painter.as_ref(), buf);
    }

    fn as_any(&self) -> Option<&dyn Any> {
        Some(self)
    }
}
