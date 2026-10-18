//! How the stage looks: the terrains' glyphs and colors, the two pieces as
//! the map scene holds them, and the engine's two skins, each with a
//! painter of the pieces: the glyph skin (two cells a tile, a piece's
//! letter on its tile) and the sprite skin (the tiles of
//! `files/tiles.png`, which `cargo shuyi glyph-tiles` draws from the same
//! glyphs and colors, `shuyi.ron` says which).

use shuyi_assets::image::{ImageId, ImageRect};
use shuyi_assets::tileset::{Look as TileLook, TerrainTiles};
use shuyi_core::{Pos, TerrainId};
use shuyi_ui::frame::glyph_buffer::{Cell, GlyphBuffer, Layer, Rect, Sprite};
use shuyi_ui::frame::{EngineColor, Palette};
use shuyi_ui::grid::glyph;
use shuyi_ui::grid::{
    GlyphSkin, Grid, MapColors, MapScene, MapUnit, RangeName, SpriteSkin, TerrainGlyphs, Terrains,
    UnitPainter, WalkSpeeds,
};

use crate::rules::{Side, TERRAIN_NAMES};

/// Each terrain's two glyphs and colors, by [`TerrainId`]: floor, wall,
/// goal.
pub const TERRAIN_LOOKS: [([char; 2], &str, &str); 3] = [
    (['.', ' '], "floor_fg", "floor_bg"),
    (['▓', '▓'], "wall_fg", "wall_bg"),
    (['[', ']'], "goal_fg", "goal_bg"),
];

/// The terrains as the skins see them.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Terrain;

impl Terrains for Terrain {
    fn name(&self, id: TerrainId) -> Option<&str> {
        TERRAIN_NAMES.get(usize::from(id.0)).copied()
    }

    fn glyphs(&self, id: TerrainId) -> Option<TerrainGlyphs<'_>> {
        let &(glyphs, fg, bg) = TERRAIN_LOOKS.get(usize::from(id.0))?;
        Some(TerrainGlyphs { glyphs, fg, bg })
    }
}

/// The palette's names of the path's, the cursor's and the threat's
/// colors.
pub const COLORS: MapColors = MapColors {
    path: "path",
    cursor: "cursor",
    threat: "threat",
};

/// The range of tiles your picked-up piece can walk to: the palette's
/// `move` tints it.
pub const MOVE: RangeName = RangeName("move");

/// A piece's letter, its palette color and its tile in `tiles.png`.
pub const fn piece_look(side: Side) -> (char, &'static str, usize) {
    match side {
        Side::Hero => ('@', "hero", HERO_TILE),
        Side::Foe => ('&', "foe", FOE_TILE),
    }
}

/// A piece on the map, as the scene holds it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Piece {
    /// Whose it is.
    pub side: Side,
    /// The tile it is shown on.
    pub pos: Pos,
}

impl MapUnit for Piece {
    type Id = Side;

    fn id(&self) -> Side {
        self.side
    }

    fn pos(&self) -> Pos {
        self.pos
    }

    fn text(&self) -> String {
        let (letter, ..) = piece_look(self.side);
        format!("{letter} ({},{})", self.pos.x, self.pos.y)
    }
}

/// How fast a walk is shown, in tiles a second.
pub const WALK: WalkSpeeds = WalkSpeeds::of(8.0);

/// The glyph skin.
pub const GLYPHS: GlyphSkin = GlyphSkin {
    colors: COLORS,
    walk: WALK,
    foot_px: 0,
};

/// The tileset's picture, in the game's images.
pub const TILES_PNG: &str = "tiles.png";
/// A tile's side in `tiles.png`, in pixels.
pub const TILE_PX: u32 = 24;
/// Tiles per row of `tiles.png`.
pub const TILE_COLUMNS: u32 = 8;
/// The hero's tile in `tiles.png`, after the three terrains'.
pub const HERO_TILE: usize = 3;
/// The foe's tile.
pub const FOE_TILE: usize = 4;

/// Tile `index` of `tiles.png`.
pub fn tile_rect(index: usize) -> ImageRect {
    let i = u32::try_from(index).unwrap_or(u32::MAX);
    ImageRect {
        x: i % TILE_COLUMNS * TILE_PX,
        y: i / TILE_COLUMNS * TILE_PX,
        w: TILE_PX,
        h: TILE_PX,
    }
}

/// The terrain tiles in `image`: each terrain's tile, in id order.
pub fn terrain_tiles(image: ImageId) -> TerrainTiles {
    let tiles = (0..TERRAIN_LOOKS.len())
        .map(|i| (TerrainId(u16::try_from(i).unwrap_or(0)), tile_rect(i)))
        .collect();
    TerrainTiles {
        image,
        tile_px: (TILE_PX, TILE_PX),
        look: TileLook {
            tiles,
            layers: Vec::new(),
        },
        looks: std::collections::BTreeMap::new(),
    }
}

/// How many times their size the tiles are drawn.
pub const ZOOM: i32 = 2;

/// The sprite skin of the tiles in `image`, zoomed [`ZOOM`] times.
pub fn sprites(image: ImageId) -> SpriteSkin {
    SpriteSkin::new(Some(terrain_tiles(image)), GLYPHS, WALK).zoomed(ZOOM)
}

/// The pieces under the glyph skin: each its letter, in its color, over
/// its tile's left cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Letters<'a> {
    /// The palette the colors are named in.
    pub palette: &'a Palette,
}

impl UnitPainter<Piece> for Letters<'_> {
    fn paint(&self, scene: &MapScene<Piece>, grid: &Grid, buf: &mut GlyphBuffer) {
        for piece in &scene.units {
            let (letter, color, _) = piece_look(piece.side);
            let fg = self
                .palette
                .lookup(color)
                .unwrap_or_else(|| self.palette.engine(EngineColor::Text));
            if let Some((x, y)) = glyph::cell_of(grid, piece.pos)
                && let Some(&cell) = buf.get(x, y)
            {
                buf.set(
                    x,
                    y,
                    Cell {
                        glyph: letter,
                        fg,
                        ..cell
                    },
                );
            }
        }
    }
}

/// The pieces under the sprite skin: each its tile of `tiles.png`, over
/// the ground.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tiles {
    /// `tiles.png`.
    pub image: ImageId,
}

impl UnitPainter<Piece> for Tiles {
    fn paint(&self, scene: &MapScene<Piece>, grid: &Grid, buf: &mut GlyphBuffer) {
        for piece in &scene.units {
            let Some(dest) = grid.rect(piece.pos) else {
                continue;
            };
            let (.., index) = piece_look(piece.side);
            let r = tile_rect(index);
            let px = |v: u32| i32::try_from(v).unwrap_or(0);
            let src = Rect::new(px(r.x), px(r.y), px(r.w), px(r.h));
            buf.add_sprite(Sprite::new(self.image, src, dest, Layer::Over));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_terrain_has_a_name_glyphs_and_a_tile() {
        for (i, &name) in TERRAIN_NAMES.iter().enumerate() {
            let id = TerrainId(u16::try_from(i).unwrap());
            assert_eq!(Terrain.name(id), Some(name));
            let glyphs = Terrain.glyphs(id).unwrap();
            assert_eq!(
                (glyphs.glyphs, glyphs.fg, glyphs.bg),
                TERRAIN_LOOKS[i],
                "{name}"
            );
        }
        let none = TerrainId(3);
        assert_eq!(Terrain.name(none), None);
        assert!(Terrain.glyphs(none).is_none());
    }

    #[test]
    fn tiles_go_eight_to_a_row() {
        assert_eq!(
            tile_rect(1),
            ImageRect {
                x: 24,
                y: 0,
                w: 24,
                h: 24
            }
        );
        assert_eq!(
            tile_rect(9),
            ImageRect {
                x: 24,
                y: 24,
                w: 24,
                h: 24
            }
        );
        let table = shuyi_assets::image::ImageTable {
            images: [(
                TILES_PNG,
                shuyi_assets::image::ImageInfo {
                    width: 192,
                    height: 24,
                },
            )]
            .into(),
        };
        let tiles = terrain_tiles(table.id(TILES_PNG).unwrap());
        assert_eq!(tiles.look.tiles.len(), 3);
        assert_eq!(tiles.look.tiles[&TerrainId(2)], tile_rect(2));
    }

    #[test]
    fn a_piece_says_who_it_is_and_where() {
        let hero = Piece {
            side: Side::Hero,
            pos: Pos::new(2, 3),
        };
        assert_eq!((hero.id(), hero.pos()), (Side::Hero, Pos::new(2, 3)));
        assert_eq!(hero.text(), "@ (2,3)");
        let foe = Piece {
            side: Side::Foe,
            pos: Pos::new(0, 1),
        };
        assert_eq!(foe.text(), "& (0,1)");
        assert_ne!(piece_look(Side::Hero), piece_look(Side::Foe));
    }
}
