//! The sprite skin (game ADR-0038, game ADR-0049, game ADR-0052): a grid map's
//! ground painted from a tileset's terrain tiles ([`TerrainTiles`]).
//!
//! - **With terrain tiles** it paints the whole ground: a tile is the
//!   tiles' `tile_px` times the zoom, a size known only here, behind
//!   [`SpriteSkin::tile_size`]. The ground is each tile's own picture and
//!   the layers over them, in the look the map asks for and in its light
//!   (see `ground`). The cursor is corner marks with a dark edge, inside
//!   its tile (game ticket 0444).
//! - **Without** them the ground, the ranges and the cursor are the glyph
//!   skin's, on its tiles: for a game whose units are pictures on glyph
//!   ground (its unit painter takes the terrain's glyphs off a unit's
//!   tile).
//!
//! Either way the path's line goes under the units and its arrowhead over
//! them, and the threat arrows start in the middle of a unit's tile, over
//! its picture.

mod ground;

use crate::prelude::*;

use shuyi_assets::image::{ImageRect, Picture};
use shuyi_assets::tileset::TerrainTiles;

use super::glyph::{self, GlyphSkin};
use super::layout::{Grid, px_rect};
use super::path;
use super::scene::{CursorStyle, MapScene};
use super::skin::{GridSkin, Looks, UnitPainter, WalkSpeeds};
use super::threat;
use crate::frame::glyph_buffer::{GlyphBuffer, Layer, Overlay, PxRect, Rect, Sprite};

/// A grid map's ground painted from terrain tiles.
#[derive(Debug, Clone, PartialEq)]
pub struct SpriteSkin {
    /// The tiles; `None` = the glyph skin's ground.
    terrain: Option<TerrainTiles>,
    /// How big the art is drawn, when the tiles paint the ground: at
    /// least 1.
    zoom: i32,
    /// The glyph skin: its colors, and its ground without terrain tiles.
    glyph: GlyphSkin,
    /// How fast a walk is shown.
    walk: WalkSpeeds,
}

impl SpriteSkin {
    /// The skin of `terrain` at the art's own size, with `glyph`'s colors
    /// (and its ground, without terrain tiles), showing a walk at `walk`.
    pub const fn new(terrain: Option<TerrainTiles>, glyph: GlyphSkin, walk: WalkSpeeds) -> Self {
        Self {
            terrain,
            zoom: 1,
            glyph,
            walk,
        }
    }

    /// The same skin drawn `zoom` times the art's size (at least once).
    #[must_use]
    pub fn zoomed(mut self, zoom: i32) -> Self {
        self.zoom = zoom.max(1);
        self
    }

    /// Its terrain tiles, if it has them.
    pub const fn terrain(&self) -> Option<&TerrainTiles> {
        self.terrain.as_ref()
    }

    /// The zoom it was given: console pixels per pixel of the art, when it
    /// has terrain tiles.
    pub const fn zoom(&self) -> i32 {
        self.zoom
    }

    /// Console pixels per pixel of the art: the zoom with terrain tiles;
    /// always 1 on the glyph skin's tiles, which are font cells.
    fn scale(&self) -> i32 {
        match self.terrain {
            Some(_) => self.zoom,
            None => 1,
        }
    }

    /// A map tile's size on screen, across × down, in console pixels: the
    /// tiles' times the zoom, or the glyph skin's without terrain tiles.
    /// Everything that depends on the tile size goes through this.
    pub fn tile_size(&self) -> (i32, i32) {
        let zoomed = |px: u32| side(px).saturating_mul(self.scale());
        match &self.terrain {
            Some(terrain) => (zoomed(terrain.tile_px.0), zoomed(terrain.tile_px.1)),
            None => glyph::tile_px(),
        }
    }

    /// The area's pixels divided by the tile size, rounded down; without
    /// terrain tiles, what the glyph skin fits.
    pub fn view_tiles(&self, area: Rect) -> (i32, i32) {
        if self.terrain.is_none() {
            return glyph::view_tiles(area);
        }
        let px = px_rect(area);
        let (w, h) = self.tile_size();
        (px.w.max(0) / w, px.h.max(0) / h)
    }

    /// Where `scene`'s tiles go in `area`: as many as fit, the pixels left
    /// over split evenly around them; without terrain tiles, where the
    /// glyph skin puts them.
    pub fn grid<U>(&self, scene: &MapScene<U>, area: Rect) -> Grid {
        if self.terrain.is_none() {
            return glyph::Layout::new(scene, area).grid();
        }
        let px = px_rect(area);
        let tile = self.tile_size();
        let fit = self.view_tiles(area);
        let margin = |len: i32, n: i32, side: i32| (len.max(0) - n * side) / 2;
        Grid::top_down(
            scene.origin,
            (
                px.x + margin(px.w, fit.0, tile.0),
                px.y + margin(px.h, fit.1, tile.1),
            ),
            tile,
            (scene.size.0.clamp(0, fit.0), scene.size.1.clamp(0, fit.1)),
            self.scale(),
        )
    }

    /// The cursor: corner marks in the cursor's color with a dark edge
    /// round them, inside its tile, in every style (pictures have no tile
    /// glow: game ticket 0444). The color pulses; the edge doesn't.
    fn paint_own_cursor<U>(
        &self,
        looks: &Looks,
        scene: &MapScene<U>,
        grid: &Grid,
        buf: &mut GlyphBuffer,
    ) {
        if let Some(c) = &scene.cursor
            && let Some(tile) = grid.rect(c.pos)
        {
            let edge = looks.dark();
            let color = looks.cursor(&self.glyph.colors).scale(c.brightness);
            let marks = CursorMarks::of(c.style, grid.zoom());
            let art = grid.art_tile();
            let edges = corner_marks(art, 0, marks.arm + 2, marks.thick + 2);
            let arms = corner_marks(art, 1, marks.arm, marks.thick);
            for (rects, color) in [(edges, edge), (arms, color)] {
                for r in rects {
                    let r = grid.zoomed((tile.x, tile.y), r);
                    buf.add_overlay(Overlay::new(r, color, Layer::Over));
                }
            }
        }
    }
}

/// The size of the cursor's corner marks on picture tiles, in pixels of
/// the art (Nick, 2026-10-13, game ticket 0444).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CursorMarks {
    /// How long each arm of a corner is.
    arm: i32,
    /// How thick.
    thick: i32,
}

impl CursorMarks {
    /// At the art's own size, thick marks (2 px, 4 px long); zoomed in,
    /// thin ones (1 px of the art, 3 long), which the zoom thickens.
    /// Large corners are a pixel longer; the glow style has no look of
    /// its own here and is the corners.
    const fn of(style: CursorStyle, zoom: i32) -> Self {
        let (arm, thick) = if zoom == 1 { (4, 2) } else { (3, 1) };
        let longer = match style {
            CursorStyle::Corners | CursorStyle::TileGlow => 0,
            CursorStyle::LargeCorners => 1,
        };
        Self {
            arm: arm + longer,
            thick,
        }
    }
}

impl<U> GridSkin<U> for SpriteSkin {
    /// `"sprite"` when the tiles paint the ground, `"sprite_units"` on the
    /// glyph skin's ground.
    fn name(&self) -> &'static str {
        match self.terrain {
            Some(_) => "sprite",
            None => "sprite_units",
        }
    }

    fn tile_size(&self) -> (i32, i32) {
        SpriteSkin::tile_size(self)
    }

    fn view_tiles(&self, area: Rect) -> (i32, i32) {
        SpriteSkin::view_tiles(self, area)
    }

    fn grid(&self, scene: &MapScene<U>, area: Rect) -> Grid {
        SpriteSkin::grid(self, scene, area)
    }

    fn walk(&self) -> WalkSpeeds {
        self.walk
    }

    /// The ground (terrain tinted by flashes and ranges), what lies under
    /// the units, the path's line, the units, the path's arrowhead, then
    /// the cursor; over them all, the threat arrows and marks.
    fn paint(
        &self,
        looks: &Looks,
        scene: &MapScene<U>,
        area: Rect,
        units: &dyn UnitPainter<U>,
        buf: &mut GlyphBuffer,
    ) {
        let grid = SpriteSkin::grid(self, scene, area);
        let layout = glyph::Layout::new(scene, area);
        match &self.terrain {
            Some(terrain) => ground::paint(looks, terrain, scene, &grid, buf),
            None => glyph::draw_tiles(looks, buf, scene, &layout),
        }
        units.paint_under(scene, &grid, buf);
        let color = looks.path(&self.glyph.colors);
        let (line, arrowhead): (Vec<Overlay>, Vec<Overlay>) =
            path::path_overlays(&scene.path, &grid, color)
                .into_iter()
                .partition(|o| o.layer == Layer::Under);
        for overlay in line {
            buf.add_overlay(overlay);
        }
        units.paint(scene, &grid, buf);
        for overlay in arrowhead {
            buf.add_overlay(overlay);
        }
        if self.terrain.is_some() {
            self.paint_own_cursor(looks, scene, &grid, buf);
        } else {
            self.glyph.draw_cursor(looks, buf, scene, &layout);
        }
        let colors = looks.threat(&self.glyph.colors);
        threat::paint(scene, &grid, threat::Start::Middle, colors, buf);
    }
}

/// A size of the art as console pixels.
pub fn side(px: u32) -> i32 {
    i32::try_from(px).unwrap_or(i32::MAX)
}

/// `rect` of an image as a sprite's `src`.
pub fn src_rect(rect: ImageRect) -> PxRect {
    Rect::new(side(rect.x), side(rect.y), side(rect.w), side(rect.h))
}

/// A solid sprite of the tile `picture` on the pixels `dest`.
pub fn tile_sprite(picture: Picture, dest: PxRect) -> Sprite {
    Sprite::new(picture.image, src_rect(picture.rect), dest, Layer::Under)
}

/// The eight arms, each `arm` px long and `thick` px thick, of corner
/// marks `inset` px inside the corners of the pixels `tile`.
fn corner_marks(tile: PxRect, inset: i32, arm: i32, thick: i32) -> [Rect; 8] {
    let (left, top) = (tile.x + inset, tile.y + inset);
    let (right, bottom) = (tile.x + tile.w - inset, tile.y + tile.h - inset);
    let across = |x0, y0| Rect::new(x0, y0, arm, thick);
    let down = |x0, y0| Rect::new(x0, y0, thick, arm);
    [
        across(left, top),
        down(left, top),
        across(right - arm, top),
        down(right - thick, top),
        across(left, bottom - thick),
        down(left, bottom - arm),
        across(right - arm, bottom - thick),
        down(right - thick, bottom - arm),
    ]
}

#[cfg(test)]
pub(crate) mod tests;
