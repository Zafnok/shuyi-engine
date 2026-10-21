//! The glyph skin: a grid map drawn with colored glyphs (game ADR-0018,
//! game ADR-0024, game ADR-0029). A tile is two 8 × 16 cells: a 16 × 16 pixel
//! square. That size is known only here. A [`GlyphStyle`] changes how
//! ranges, flashes, the cursor and the threat marks show on those tiles.

pub mod cursor;
pub mod style;

use shuyi_core::{Pos, TerrainId};

use super::layout::{CELL_PX, Grid};
use super::path;
use super::scene::MapScene;
use super::skin::{GridSkin, Looks, MapColors, UnitPainter, WalkSpeeds};
use super::threat;
use crate::frame::color::EngineColor;
use crate::frame::glyph_buffer::{Cell, GlyphBuffer, Rect};

pub use style::{CursorLook, FlashLook, GlyphStyle, RangeLook, ThreatLook};

/// Cells per map tile, across (a tile is two glyphs wide, game ADR-0018).
pub const TILE_W_CELLS: i32 = 2;

/// A map tile in console pixels, across × down: for a skin that paints
/// its units over this skin's ground (game ADR-0049).
pub const fn tile_px() -> (i32, i32) {
    (TILE_W_CELLS * CELL_PX.0, CELL_PX.1)
}

/// How far ranges tint a tile's background toward their color (about
/// 75%). *Tunable.*
pub const OVERLAY_BLEND: f32 = 0.75;

/// A grid map as colored glyphs: each tile its terrain's two glyphs, the
/// ranges tinting their background (in its [plain style](GlyphStyle::PLAIN)).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlyphSkin {
    /// The palette's names of the path's, the cursor's and the threat
    /// arrows' colors.
    pub colors: MapColors,
    /// How fast a walk is shown.
    pub walk: WalkSpeeds,
    /// How many pixels at the foot of a tile the units' own marks use (a
    /// game's HP bar): the cursor's lower arms stay above them.
    pub foot_px: i32,
    /// How ranges, flashes, the cursor and the threat marks show, and the
    /// cells left at the sides. (The sprite skin's glyph ground is always
    /// in the plain style.)
    pub style: GlyphStyle,
}

impl<U> GridSkin<U> for GlyphSkin {
    fn name(&self) -> &'static str {
        "glyph"
    }

    fn tile_size(&self) -> (i32, i32) {
        tile_px()
    }

    fn view_tiles(&self, area: Rect) -> (i32, i32) {
        self.style.view_tiles(area)
    }

    fn grid(&self, scene: &MapScene<U>, area: Rect) -> Grid {
        self.style.layout(scene, area).grid()
    }

    fn walk(&self) -> WalkSpeeds {
        self.walk
    }

    /// Terrain, then the ranges on it, then what lies under the units
    /// ([`UnitPainter::paint_under`]), the units, the path, what lies over
    /// them ([`UnitPainter::paint_over`]) and the cursor; over them all,
    /// the threat arrows and marks.
    fn paint(
        &self,
        looks: &Looks,
        scene: &MapScene<U>,
        area: Rect,
        units: &dyn UnitPainter<U>,
        buf: &mut GlyphBuffer,
    ) {
        let style = &self.style;
        let layout = style.layout(scene, area);
        let grid = layout.grid();
        style.draw_tiles(looks, buf, scene, &layout);
        units.paint_under(scene, &grid, buf);
        units.paint(scene, &grid, buf);
        let color = looks.path(&self.colors);
        for overlay in path::path_overlays(&scene.path, &grid, color) {
            buf.add_overlay(overlay);
        }
        units.paint_over(scene, &grid, buf);
        style.draw_cursor(self.cursor_ink(looks), buf, scene, &layout, area);
        // From the edge of each enemy's tile: never over its letters.
        let colors = style.threat_colors(looks, &self.colors);
        threat::paint(scene, &grid, threat::Start::Edge, colors, buf);
    }
}

impl GlyphSkin {
    /// The skin of `colors` in the plain style, showing a walk at `walk`,
    /// its cursor's lower arms above the `foot_px` pixels at a tile's foot.
    pub const fn new(colors: MapColors, walk: WalkSpeeds, foot_px: i32) -> Self {
        Self {
            colors,
            walk,
            foot_px,
            style: GlyphStyle::PLAIN,
        }
    }

    /// The same skin in `style`.
    #[must_use]
    pub const fn styled(mut self, style: GlyphStyle) -> Self {
        self.style = style;
        self
    }

    /// How its cursor looks in `looks`' colors.
    pub fn cursor_ink(&self, looks: &Looks) -> cursor::CursorInk {
        cursor::CursorInk {
            color: looks.cursor(&self.colors),
            dark: looks.dark(),
            foot_px: self.foot_px,
        }
    }

    /// Draws the scene's cursor, if it is on a tile of `layout`, as the
    /// plain style does.
    pub fn draw_cursor<U>(
        &self,
        looks: &Looks,
        buf: &mut GlyphBuffer,
        scene: &MapScene<U>,
        layout: &Layout,
    ) {
        if let Some(c) = &scene.cursor
            && let Some(at) = tile_to_cell(c.pos, layout)
        {
            cursor::draw_cursor(buf, self.cursor_ink(looks), c, layout, at);
        }
    }
}

/// How many tiles, across × down, fit in an area of `area` cells: two
/// cells to a tile across, one down. An odd cell is left over.
pub fn view_tiles(area: Rect) -> (i32, i32) {
    (area.w.max(0) / TILE_W_CELLS, area.h.max(0))
}

/// Where a scene's tiles go in an area of cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Layout {
    /// The map tile in the area's top-left corner.
    origin: Pos,
    /// The top-left cell.
    cell: (i32, i32),
    /// Tiles drawn, across × down: the scene's, or as many as fit.
    tiles: (i32, i32),
}

impl Layout {
    /// The layout of `scene` in `area`.
    pub fn new<U>(scene: &MapScene<U>, area: Rect) -> Self {
        Self::fitted(scene, (area.x, area.y), view_tiles(area))
    }

    /// The layout of `scene` from the cell `corner`, drawing no more than
    /// `fit` tiles: for a skin of two-cell tiles that places them its own
    /// way (centred in its area).
    pub fn fitted<U>(scene: &MapScene<U>, corner: (i32, i32), fit: (i32, i32)) -> Self {
        Self {
            origin: scene.origin,
            cell: corner,
            tiles: (scene.size.0.clamp(0, fit.0), scene.size.1.clamp(0, fit.1)),
        }
    }

    /// The cells the tiles cover.
    pub fn cells(&self) -> Rect {
        let (x, y) = self.cell;
        Rect::new(x, y, TILE_W_CELLS * self.tiles.0, self.tiles.1)
    }

    /// The tiles in console pixels: 16 × 16 each.
    pub fn grid(&self) -> Grid {
        let (cw, ch) = CELL_PX;
        Grid::top_down(
            self.origin,
            (self.cell.0 * cw, self.cell.1 * ch),
            (TILE_W_CELLS * cw, ch),
            self.tiles,
            1,
        )
    }
}

/// The console cell of the left glyph of `tile`, or `None` if the tile is
/// outside the layout. The only place the "two cells per tile" rule lives
/// (game ADR-0018).
pub fn tile_to_cell(tile: Pos, layout: &Layout) -> Option<(i32, i32)> {
    let dx = tile.x.checked_sub(layout.origin.x)?;
    let dy = tile.y.checked_sub(layout.origin.y)?;
    let inside = (0..layout.tiles.0).contains(&dx) && (0..layout.tiles.1).contains(&dy);
    inside.then(|| (layout.cell.0 + TILE_W_CELLS * dx, layout.cell.1 + dy))
}

/// The console cell of the left glyph of `tile` on `grid`, a grid of this
/// skin's tiles ([`Layout::grid`]), or `None` if the tile isn't drawn: where
/// a game's unit painter writes a unit's glyphs.
pub fn cell_of(grid: &Grid, tile: Pos) -> Option<(i32, i32)> {
    let px = grid.rect(tile)?;
    let (cw, ch) = CELL_PX;
    Some((px.x.div_euclid(cw), px.y.div_euclid(ch)))
}

/// Draws terrain `id` on the tile whose left cell is `(x, y)`: its two
/// glyphs in its colors. Nothing for a terrain the game lacks.
pub fn draw_terrain(looks: &Looks, buf: &mut GlyphBuffer, id: TerrainId, (x, y): (i32, i32)) {
    let Some(t) = looks.terrains.glyphs(id) else {
        return;
    };
    let fg = looks.named(t.fg, EngineColor::Text);
    let bg = looks.named(t.bg, EngineColor::Background);
    for (i, glyph) in (0..).zip(t.glyphs) {
        buf.set(x + i, y, Cell::new(glyph, fg, bg));
    }
}

/// Draws every tile in the plain style ([`GlyphStyle::draw_tiles`]): its
/// terrain (tiles off the map are left as they are, blank); a tile whose
/// terrain just changed flashes, its background tinted towards its glyph
/// color, fading out; then its ranges tint it, the first laid on
/// undermost; and a tile that would change is drawn as the terrain it would
/// become instead.
pub fn draw_tiles<U>(looks: &Looks, buf: &mut GlyphBuffer, scene: &MapScene<U>, layout: &Layout) {
    GlyphStyle::PLAIN.draw_tiles(looks, buf, scene, layout);
}

#[cfg(test)]
pub(crate) mod tests {
    use proptest::prelude::*;

    use super::*;
    use crate::frame::color::Rgb;
    use crate::frame::glyph_buffer::{Layer, Overlay};
    use crate::grid::layout::px_rect;
    use crate::grid::scene::{CursorStyle, CursorView, RangeName, ThreatView};
    use crate::grid::skin::NoUnits;
    use crate::grid::testing::{any_area, any_scene};
    use crate::prelude::*;
    use crate::toy::grid::{GLYPH, GRASS, Kit, Letters, Piece, SAND, WATER};

    /// A 70 × 30-cell map area: 35 × 30 of this skin's tiles.
    pub(crate) const AREA: Rect = Rect::new(0, 0, 70, 30);

    /// [`AREA`] with the view's top-left on `origin`.
    pub(crate) fn layout_at(origin: Pos) -> Layout {
        let size = view_tiles(AREA);
        Layout::new(&MapScene::<Piece>::new(origin, size), AREA)
    }

    fn p(x: i32, y: i32) -> Pos {
        Pos::new(x, y)
    }

    const MOVE: RangeName = RangeName("move");
    const DANGER: RangeName = RangeName("danger");
    const HEAL: RangeName = RangeName("heal");

    /// A buffer the size of a 100 × 32 console, every cell `fill()`.
    fn blank() -> GlyphBuffer {
        GlyphBuffer::new(100, 32, fill())
    }

    fn fill() -> Cell {
        Cell::new('x', Rgb::new(1, 2, 3), Rgb::new(4, 5, 6))
    }

    fn paint(kit: &Kit, scene: &MapScene<Piece>, area: Rect) -> GlyphBuffer {
        let mut buf = blank();
        GLYPH.paint(&kit.looks(), scene, area, &Letters, &mut buf);
        buf
    }

    #[test]
    fn a_70_by_30_cell_area_shows_35_by_30_tiles() {
        let skin: &dyn GridSkin<Piece> = &GLYPH;
        assert_eq!(skin.name(), "glyph");
        assert_eq!(skin.view_tiles(AREA), (35, 30));
        assert_eq!(skin.tile_size(), (16, 16));
        assert_eq!(skin.walk(), GLYPH.walk);
        // An odd cell is left over; an empty area shows nothing.
        assert_eq!(skin.view_tiles(Rect::new(3, 4, 7, 5)), (3, 5));
        assert_eq!(skin.view_tiles(Rect::new(0, 0, -4, -1)), (0, 0));
    }

    #[test]
    fn tile_to_cell_maps_two_cells_per_tile() {
        let c = layout_at(p(5, 2));
        assert_eq!(tile_to_cell(Pos::new(5, 2), &c), Some((0, 0)));
        assert_eq!(tile_to_cell(Pos::new(6, 3), &c), Some((2, 1)));
        assert_eq!(tile_to_cell(Pos::new(39, 31), &c), Some((68, 29)));
        assert_eq!(tile_to_cell(Pos::new(40, 2), &c), None);
        assert_eq!(tile_to_cell(Pos::new(5, 32), &c), None);
        assert_eq!(tile_to_cell(Pos::new(4, 2), &c), None);
        assert_eq!(tile_to_cell(Pos::new(5, 1), &c), None);
        let small = layout_at(p(-10, -11));
        assert_eq!(tile_to_cell(Pos::new(0, 0), &small), Some((20, 11)));
        assert_eq!(tile_to_cell(Pos::new(i32::MIN, 0), &c), None);
        assert_eq!(tile_to_cell(Pos::new(0, i32::MIN), &c), None);
        // The same cells from the layout's grid.
        let grid = c.grid();
        for tile in [p(5, 2), p(6, 3), p(39, 31), p(40, 2), p(4, 2), p(5, 1)] {
            assert_eq!(cell_of(&grid, tile), tile_to_cell(tile, &c), "{tile:?}");
        }
        // Left of and above the console too.
        let off = Layout::new(
            &MapScene::<Piece>::new(p(0, 0), (3, 3)),
            Rect::new(-6, -2, 6, 3),
        );
        assert_eq!(cell_of(&off.grid(), p(0, 0)), Some((-6, -2)));
        assert_eq!(tile_to_cell(p(1, 1), &off), Some((-4, -1)));
        assert_eq!(cell_of(&off.grid(), p(1, 1)), Some((-4, -1)));
    }

    #[test]
    fn a_layout_starts_at_its_area_and_draws_no_more_tiles_than_fit() {
        let area = Rect::new(4, 3, 7, 2);
        // The scene is bigger than the area: 3 × 2 tiles fit.
        let big = Layout::new(&MapScene::<Piece>::new(p(1, 1), (9, 9)), area);
        assert_eq!(big.cells(), Rect::new(4, 3, 6, 2));
        assert_eq!(tile_to_cell(p(1, 1), &big), Some((4, 3)));
        assert_eq!(tile_to_cell(p(3, 2), &big), Some((8, 4)));
        assert_eq!(tile_to_cell(p(4, 2), &big), None);
        assert_eq!(tile_to_cell(p(3, 3), &big), None);
        assert_eq!(big.grid().tile_px(p(1, 1)), (32, 48));
        assert_eq!(big.grid().tile_px(p(0, 3)), (16, 80));
        assert_eq!(big.grid().bounds(), px_rect(big.cells()));
        assert_eq!(big.grid().tile, (16, 16));
        assert_eq!(tile_px(), (16, 16));
        // The scene is smaller: only its tiles.
        let small = Layout::new(&MapScene::<Piece>::new(p(1, 1), (2, 1)), area);
        assert_eq!(small.cells(), Rect::new(4, 3, 4, 1));
        assert_eq!(tile_to_cell(p(2, 1), &small), Some((6, 3)));
        assert_eq!(tile_to_cell(p(3, 1), &small), None);
        assert_eq!(tile_to_cell(p(1, 2), &small), None);
        // Fitted from a corner of its own, with room for fewer.
        let fitted = Layout::fitted(&MapScene::<Piece>::new(p(1, 1), (9, 9)), (5, 0), (2, 3));
        assert_eq!(fitted.cells(), Rect::new(5, 0, 4, 3));
        assert_eq!(layout_at(p(0, 0)).cells(), AREA);
    }

    #[test]
    fn tile_px_is_the_16_px_square_of_a_visible_tile() {
        let scene = MapScene::<Piece>::new(p(-10, -11), view_tiles(AREA));
        let px = |tile| GLYPH.tile_px(&scene, AREA, tile);
        assert_eq!(px(p(0, 0)), Some(Rect::new(160, 176, 16, 16)));
        assert_eq!(px(p(24, 18)), Some(Rect::new(544, 464, 16, 16)));
        assert_eq!(px(p(25, 18)), None);
        assert_eq!(px(p(-11, 0)), None);
        assert_eq!(
            GLYPH.tile_cells(&scene, AREA, p(3, 5)),
            Some(Rect::new(26, 16, 2, 1))
        );
        // In another area: from its corner.
        let area = Rect::new(4, 3, 7, 2);
        assert_eq!(
            GLYPH.tile_px(&scene, area, p(-9, -10)),
            Some(Rect::new(48, 64, 16, 16))
        );
    }

    /// The threat arrows and marks are the last things painted, over the
    /// units, the path and the cursor, on the skin's 16 px tiles.
    #[test]
    fn threat_arrows_and_marks_are_painted_last_over_everything() {
        let kit = Kit::new();
        let looks = kit.looks();
        let area = Rect::new(10, 5, 8, 3);
        let mut scene = MapScene::new(p(2, 2), (4, 3));
        scene.push_unit(Piece::new(4, 'f', p(5, 3)));
        scene.path = vec![p(2, 2), p(3, 2)];
        scene.cursor = Some(CursorView {
            pos: p(3, 2),
            brightness: 1.0,
            style: CursorStyle::Corners,
        });
        let buf = paint(&kit, &scene, area);
        let before = buf.overlays();
        let threat = ThreatView {
            tile: p(3, 2),
            enemies: vec![p(5, 3)],
        };
        scene.threat = Some(threat.clone());
        let with = paint(&kit, &scene, area);
        // No cell changes: the arrows are drawn over the glyphs.
        for y in 0..32 {
            for x in 0..100 {
                assert_eq!(with.get(x, y), buf.get(x, y), "({x}, {y})");
            }
        }
        let colors = looks.threat(&GLYPH.colors);
        let grid = Layout::new(&scene, area).grid();
        let expect = threat::threat_overlays(&threat, &grid, threat::Start::Edge, colors);
        let after = with.overlays();
        assert_eq!(&after[..before.len()], before);
        assert_eq!(&after[before.len()..], expect);
        // The arrow's line starts just off the foe's tile (cells (16, 6)
        // and (17, 6): pixels 128..144 × 96..112), on its way up and left
        // from the tile's middle, and no arrow is on the tile; the mark is
        // above that tile, 3 px clear of it.
        let arrows: Vec<_> = expect.iter().filter(|o| o.color == colors.arrow).collect();
        assert_eq!(arrows[0].rect, Rect::new(127, 100, 1, 1));
        let tile = Rect::new(128, 96, 16, 16);
        assert!(arrows.iter().all(|o| o.rect.intersect(&tile).is_none()));
        assert!(arrows.iter().all(|o| o.layer == Layer::Over));
        let marks: Vec<_> = expect.iter().filter(|o| o.color == colors.mark).collect();
        assert_eq!(marks[0].rect, Rect::new(134, 84, 3, 6));
        assert_eq!(marks[1].rect, Rect::new(134, 91, 3, 2));
        assert_eq!(marks.len(), 2);
    }

    #[test]
    fn paints_terrain_then_tints_then_units_path_and_cursor() {
        let kit = Kit::new();
        let looks = kit.looks();
        let pal = &kit.palette;
        let grass = looks.terrains.glyphs(GRASS).unwrap();
        let (fg, bg) = (pal.lookup(grass.fg).unwrap(), pal.lookup(grass.bg).unwrap());
        let area = Rect::new(10, 5, 8, 3);
        let mut scene = MapScene::new(p(2, 2), (4, 3));
        for pos in [p(2, 2), p(3, 2), p(4, 2), p(5, 2), p(2, 3)] {
            scene.tile_mut(pos).unwrap().terrain = Some(GRASS);
        }
        // A terrain the game lacks is left blank, as off the map.
        scene.tile_mut(p(3, 3)).unwrap().terrain = Some(TerrainId(999));
        scene.tint([p(3, 2)], DANGER);
        scene.tint([p(3, 2), p(4, 2)], MOVE);
        // A range off the map still tints.
        scene.tint([p(4, 4)], HEAL);
        scene.push_unit(Piece::new(4, 'f', p(4, 2)));
        scene.path = vec![p(2, 2), p(3, 2)];
        scene.cursor = Some(CursorView {
            pos: p(5, 2),
            brightness: 1.0,
            style: CursorStyle::Corners,
        });
        let buf = paint(&kit, &scene, area);
        let cell = |x, y| *buf.get(x, y).unwrap();
        // Tile (2, 2) at cells (10, 5) and (11, 5): grass.
        assert_eq!(cell(10, 5), Cell::new(grass.glyphs[0], fg, bg));
        assert_eq!(cell(11, 5), Cell::new(grass.glyphs[1], fg, bg));
        // (3, 2): danger under move.
        let blend = |bg: Rgb, kind| bg.lerp(looks.range(kind), OVERLAY_BLEND);
        let both = blend(blend(bg, DANGER), MOVE);
        assert_eq!(cell(12, 5), Cell::new(grass.glyphs[0], fg, both));
        assert_eq!(cell(13, 5).bg, both);
        // (4, 2): the piece's letter on the tinted tile.
        let moved = blend(bg, MOVE);
        let piece = pal.lookup("piece").unwrap();
        assert_eq!(cell(14, 5), Cell::new('f', piece, moved));
        assert_eq!(cell(15, 5), Cell::new(grass.glyphs[1], fg, moved));
        // (3, 3) unknown and (4, 3) off the map: untouched; (4, 4) off the
        // map but tinted.
        assert_eq!(cell(12, 6), fill());
        assert_eq!(cell(14, 6), fill());
        let healed = Cell {
            bg: blend(fill().bg, HEAL),
            ..fill()
        };
        assert_eq!((cell(14, 7), cell(15, 7)), (healed, healed));
        assert_eq!(cell(16, 7), fill());
        // Overlays in order: the piece's own mark, the path, the cursor.
        let colors: Vec<Rgb> = buf.overlays().iter().map(|o| o.color).collect();
        let path = looks.path(&GLYPH.colors);
        let mut expect = vec![piece, path];
        expect.extend([path; 6]);
        expect.extend([looks.cursor(&GLYPH.colors); 8]);
        assert_eq!(colors, expect);
        // The piece's mark along the foot of its tile (pixels 112..128 ×
        // 80..96), a pixel in from each side.
        assert_eq!(buf.overlays()[0].rect, Rect::new(113, 94, 14, 2));
    }

    #[test]
    fn a_changed_tile_flashes_and_a_tile_that_would_change_shows_what_it_becomes() {
        let kit = Kit::new();
        let looks = kit.looks();
        let pal = &kit.palette;
        let look = |id| {
            let t = looks.terrains.glyphs(id).unwrap();
            (
                t.glyphs,
                pal.lookup(t.fg).unwrap(),
                pal.lookup(t.bg).unwrap(),
            )
        };
        let (grass, water) = (look(GRASS), look(WATER));
        let mut scene = MapScene::new(p(0, 0), (5, 1));
        for tile in &mut scene.tiles {
            tile.terrain = Some(GRASS);
        }
        // Half a flash; a full flash under a range; two flashes at once.
        scene.tile_mut(p(0, 0)).unwrap().flashes = vec![0.5];
        scene.tile_mut(p(1, 0)).unwrap().flashes = vec![1.0];
        scene.tile_mut(p(4, 0)).unwrap().flashes = vec![1.0, 0.5];
        scene.tint([p(1, 0), p(2, 0), p(3, 0)], RangeName("attack"));
        // Water to be, and a terrain the game lacks.
        scene.tile_mut(p(2, 0)).unwrap().becomes = Some(WATER);
        scene.tile_mut(p(3, 0)).unwrap().becomes = Some(TerrainId(999));
        let buf = paint(&kit, &scene, Rect::new(0, 0, 10, 1));
        let cell = |x| *buf.get(x, 0).unwrap();
        let (glyphs, fg, bg) = grass;
        let attack = looks.range(RangeName("attack"));
        // Towards the tile's own glyph color, by the flash's strength.
        let half = bg.lerp(fg, OVERLAY_BLEND * 0.5);
        assert_eq!(cell(0), Cell::new(glyphs[0], fg, half));
        assert_eq!(cell(1), Cell::new(glyphs[1], fg, half));
        // The range goes over the flash.
        let full = bg.lerp(fg, OVERLAY_BLEND);
        assert_eq!(cell(2).bg, full.lerp(attack, OVERLAY_BLEND));
        // What it would become replaces the tile, range and all…
        assert_eq!(cell(4), Cell::new(water.0[0], water.1, water.2));
        assert_eq!(cell(5), Cell::new(water.0[1], water.1, water.2));
        // …unless the game lacks it.
        assert_eq!(
            cell(6),
            Cell::new(glyphs[0], fg, bg.lerp(attack, OVERLAY_BLEND))
        );
        // Each flash tints in turn.
        assert_eq!(cell(8).bg, full.lerp(fg, OVERLAY_BLEND * 0.5));
        assert_ne!(water, grass);
    }

    #[test]
    fn a_terrain_whose_colors_the_palette_lacks_is_drawn_in_the_engines() {
        let kit = Kit::new();
        let looks = kit.looks();
        let mut scene = MapScene::<Piece>::new(p(0, 0), (1, 1));
        scene.tiles[0].terrain = Some(SAND);
        let mut buf = blank();
        GLYPH.paint(&looks, &scene, Rect::new(0, 0, 2, 1), &NoUnits, &mut buf);
        let sand = looks.terrains.glyphs(SAND).unwrap();
        let text = kit.palette.engine(EngineColor::Text);
        let background = kit.palette.engine(EngineColor::Background);
        assert!(kit.palette.lookup(sand.fg).is_none());
        assert!(kit.palette.lookup(sand.bg).is_none());
        assert_eq!(
            buf.get(0, 0),
            Some(&Cell::new(sand.glyphs[0], text, background))
        );
        assert_eq!(
            buf.get(1, 0),
            Some(&Cell::new(sand.glyphs[1], text, background))
        );
    }

    /// The pieces' letters, and over them a mark: a `^` on each piece's
    /// left cell and a rectangle of [`MARK`] on its tile.
    struct Marked;

    /// The color of [`Marked`]'s rectangles.
    const MARK: Rgb = Rgb::new(9, 8, 7);

    impl UnitPainter<Piece> for Marked {
        fn paint(&self, scene: &MapScene<Piece>, grid: &Grid, buf: &mut GlyphBuffer) {
            Letters.paint(scene, grid, buf);
        }

        fn paint_over(&self, scene: &MapScene<Piece>, grid: &Grid, buf: &mut GlyphBuffer) {
            for piece in &scene.units {
                if let (Some((x, y)), Some(tile)) = (cell_of(grid, piece.pos), grid.rect(piece.pos))
                    && let Some(&cell) = buf.get(x, y)
                {
                    buf.set(x, y, Cell { glyph: '^', ..cell });
                    buf.add_overlay(Overlay::new(tile, MARK, Layer::Over));
                }
            }
        }
    }

    /// What the painter lays over the units goes over them and the path,
    /// and under the cursor.
    #[test]
    fn what_lies_over_the_units_is_painted_under_the_cursor() {
        let kit = Kit::new();
        let looks = kit.looks();
        let area = Rect::new(10, 5, 8, 3);
        let mut scene = MapScene::new(p(2, 2), (4, 3));
        scene.push_unit(Piece::new(4, 'f', p(4, 2)));
        scene.path = vec![p(2, 2), p(3, 2)];
        let paint = |scene: &MapScene<Piece>, units: &dyn UnitPainter<Piece>| {
            let mut buf = blank();
            GLYPH.paint(&looks, scene, area, units, &mut buf);
            buf
        };
        let bare = paint(&scene, &Marked);
        // Over the piece's letter.
        assert_eq!(bare.get(14, 5).map(|c| c.glyph), Some('^'));
        // Its rectangle last: after the piece's bar and the path.
        let colors: Vec<Rgb> = bare.overlays().iter().map(|o| o.color).collect();
        let path = looks.path(&GLYPH.colors);
        let piece = kit.palette.lookup("piece").unwrap();
        let mut expect = vec![piece];
        expect.extend([path; 7]);
        expect.push(MARK);
        assert_eq!(colors, expect);
        // The cursor comes after it.
        scene.cursor = Some(CursorView {
            pos: p(4, 2),
            brightness: 1.0,
            style: CursorStyle::Corners,
        });
        let with = paint(&scene, &Marked);
        let n = bare.overlays().len();
        assert_eq!(&with.overlays()[..n], bare.overlays());
        let cursor = looks.cursor(&GLYPH.colors);
        assert_eq!(with.overlays()[n..].len(), 8);
        assert!(with.overlays()[n..].iter().all(|o| o.color == cursor));
        // A painter that lays nothing over the units: the letter, and no
        // mark.
        let letters = paint(&scene, &Letters);
        assert_eq!(letters.overlays().len(), n + 8 - 1);
        assert_eq!(letters.get(14, 5).map(|c| c.glyph), Some('f'));
    }

    #[test]
    fn units_and_the_cursor_off_the_view_are_not_drawn() {
        let kit = Kit::new();
        let area = Rect::new(0, 0, 4, 1);
        let mut scene = MapScene::new(p(0, 0), (2, 1));
        // Not through `push_unit`: a scene built by hand.
        scene.units.push(Piece::new(4, 'f', p(2, 0)));
        scene.cursor = Some(CursorView {
            pos: p(0, 1),
            brightness: 1.0,
            style: CursorStyle::TileGlow,
        });
        assert_eq!(paint(&kit, &scene, area), blank());
    }

    proptest! {
        /// Whatever the scene and the area (even one partly off the
        /// console), the skin changes no cell outside the area and adds no
        /// item (rectangle or sprite) outside its pixels.
        #[test]
        fn glyph_skin_paints_only_the_area(scene in any_scene(), area in any_area()) {
            let kit = Kit::new();
            let buf = paint(&kit, &scene, area);
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
    }
}
