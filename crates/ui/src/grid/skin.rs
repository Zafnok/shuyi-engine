//! A grid skin: how a [`MapScene`] looks (game ADR-0038). The skin paints
//! the ground, the ranges, the path, the cursor and the threat arrows; the
//! game's [`UnitPainter`] paints its units, between the skin's layers.
//! What the skin paints with besides the scene (the palette, the game's
//! terrains, the lights) it is given as [`Looks`].

use shuyi_assets::lighting::Lighting;
use shuyi_core::{Pos, TerrainId};

use super::layout::{CELL_PX, Grid};
use super::scene::{MapScene, RangeName};
use super::threat;
use crate::frame::color::{EngineColor, Palette, Rgb};
use crate::frame::glyph_buffer::{GlyphBuffer, PxRect, Rect};

/// How the glyph skin draws one terrain: its two glyphs, in the palette's
/// colors of these names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerrainGlyphs<'a> {
    /// The tile's two glyphs, left then right.
    pub glyphs: [char; 2],
    /// The palette's name of the glyphs' color.
    pub fg: &'a str,
    /// The palette's name of the tile's background.
    pub bg: &'a str,
}

/// A game's terrains, as far as its map's look goes: what each is called,
/// and how the glyph skin draws it. The game's table, by [`TerrainId`].
pub trait Terrains {
    /// The name of terrain `id` (`"water"`), if the game has it.
    fn name(&self, id: TerrainId) -> Option<&str>;

    /// How the glyph skin draws terrain `id`, if the game has it.
    fn glyphs(&self, id: TerrainId) -> Option<TerrainGlyphs<'_>>;
}

/// The palette's names of the colors a grid skin paints the path, the
/// cursor and the threat arrows in: what the game's palette calls them.
/// The ranges are painted in the colors of their own names; a threat's
/// marks are the engine's highlight color, and every dark edge (and what
/// the cursor fades toward) its background color.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapColors {
    /// The path's line and arrowhead.
    pub path: &'static str,
    /// The cursor.
    pub cursor: &'static str,
    /// The threat arrows: the color of whoever threatens.
    pub threat: &'static str,
}

/// What a skin paints a scene with besides the scene itself: the palette,
/// the game's terrains and the lights a map may be in.
#[derive(Clone, Copy)]
pub struct Looks<'a> {
    /// The colors.
    pub palette: &'a Palette,
    /// The game's terrains.
    pub terrains: &'a dyn Terrains,
    /// The lights, by the name a map's look gives
    /// ([`MapLook::light`](shuyi_assets::grid_map::MapLook::light)).
    pub lights: &'a Lighting,
}

impl core::fmt::Debug for Looks<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Looks")
            .field("palette", self.palette)
            .field("lights", self.lights)
            .finish_non_exhaustive()
    }
}

impl Looks<'_> {
    /// The palette's color called `name`, or the engine's `fallback`
    /// (a palette checked against the game's colors has every name the
    /// game uses, so this only guards against a bug).
    pub fn named(&self, name: &str, fallback: EngineColor) -> Rgb {
        self.palette
            .lookup(name)
            .unwrap_or_else(|| self.palette.engine(fallback))
    }

    /// The color a range of `kind` tints its tiles with: the palette's of
    /// its name.
    pub fn range(&self, kind: RangeName) -> Rgb {
        self.named(kind.0, EngineColor::Text)
    }

    /// The colors of `colors`' names.
    pub fn path(&self, colors: &MapColors) -> Rgb {
        self.named(colors.path, EngineColor::Text)
    }

    /// The cursor's color, at full brightness.
    pub fn cursor(&self, colors: &MapColors) -> Rgb {
        self.named(colors.cursor, EngineColor::Text)
    }

    /// The colors of the threat arrows and marks: the arrows in
    /// `colors`' threat color, the marks in the engine's highlight color,
    /// both edged in its background color.
    pub fn threat(&self, colors: &MapColors) -> threat::Colors {
        threat::Colors {
            arrow: self.named(colors.threat, EngineColor::Text),
            mark: self.palette.engine(EngineColor::Highlight),
            edge: self.palette.engine(EngineColor::Background),
        }
    }

    /// The engine's background color: every dark edge, and what the
    /// cursor fades toward.
    pub fn dark(&self) -> Rgb {
        self.palette.engine(EngineColor::Background)
    }
}

/// How fast a unit's walk is shown, in tiles per second: how long the
/// player watches a move, never what the move does. A skin's setting.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WalkSpeeds {
    /// As a rule.
    pub normal: f32,
    /// With the player's fast animations on.
    pub fast: f32,
    /// While the player holds a key to hurry another side's turn along.
    pub held: f32,
}

impl WalkSpeeds {
    /// `normal`, with fast animations the same, and four times it while
    /// a turn is hurried.
    pub const fn of(normal: f32) -> Self {
        Self {
            normal,
            fast: normal,
            held: 4.0 * normal,
        }
    }
}

/// Paints a game's units on a grid skin's tiles: its letters, bars,
/// marks or pictures (game ADR-0038). A skin calls it between its own
/// layers.
pub trait UnitPainter<U> {
    /// What lies on the ground under the units (a light square under a
    /// unit picked out): painted after the ground and the ranges, before
    /// the path's line. Nothing unless the painter says.
    fn paint_under(&self, _scene: &MapScene<U>, _grid: &Grid, _buf: &mut GlyphBuffer) {}

    /// The units of `scene` on `grid`'s tiles: after the path's line (on
    /// the glyph skin, after the ground), before its arrowhead and the
    /// cursor.
    fn paint(&self, scene: &MapScene<U>, grid: &Grid, buf: &mut GlyphBuffer);
}

/// Paints no units: a map shown with nobody on it, or the ground alone.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NoUnits;

impl<U> UnitPainter<U> for NoUnits {
    fn paint(&self, _: &MapScene<U>, _: &Grid, _: &mut GlyphBuffer) {}
}

/// Paints grid maps of units `U`. The tile size is the skin's: screens ask
/// it how many tiles fit and where a tile is. A skin never changes the
/// game: only the frame, how many tiles are on screen, and how fast a
/// walk is shown.
pub trait GridSkin<U>: core::fmt::Debug {
    /// A stable `snake_case` name for the kind of skin, e.g. `"glyph"`.
    fn name(&self) -> &'static str;

    /// A map tile's size on screen, across × down, in console pixels.
    fn tile_size(&self) -> (i32, i32);

    /// How many tiles, across × down, fit in a map area of `area` cells.
    fn view_tiles(&self, area: Rect) -> (i32, i32);

    /// Where `scene`'s tiles go when it is painted into `area`.
    fn grid(&self, scene: &MapScene<U>, area: Rect) -> Grid;

    /// How fast a unit's walk is shown under this skin.
    fn walk(&self) -> WalkSpeeds;

    /// Paints `scene` into the cells of `area`, its units by `units`.
    /// Touches nothing outside it.
    fn paint(
        &self,
        looks: &Looks,
        scene: &MapScene<U>,
        area: Rect,
        units: &dyn UnitPainter<U>,
        buf: &mut GlyphBuffer,
    );

    /// The console pixels of `tile` when `scene` is painted into `area`, or
    /// `None` if it isn't visible. Menus and popups go beside it.
    fn tile_px(&self, scene: &MapScene<U>, area: Rect, tile: Pos) -> Option<PxRect> {
        self.grid(scene, area).rect(tile)
    }

    /// The cells [`tile_px`](Self::tile_px) touches.
    fn tile_cells(&self, scene: &MapScene<U>, area: Rect, tile: Pos) -> Option<Rect> {
        let px = self.tile_px(scene, area, tile)?;
        Some(cells_of(px))
    }
}

/// The cells the pixels `px` touch.
pub fn cells_of(px: PxRect) -> Rect {
    let (cw, ch) = CELL_PX;
    let (x, y) = (px.x.div_euclid(cw), px.y.div_euclid(ch));
    let right = (px.x + px.w + cw - 1).div_euclid(cw);
    let bottom = (px.y + px.h + ch - 1).div_euclid(ch);
    Rect::new(x, y, right - x, bottom - y)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::toy::grid::{Kit, Piece};

    /// A skin whose every tile is the pixel rectangle it was built with.
    #[derive(Debug)]
    struct Fixed(Option<PxRect>);

    impl GridSkin<Piece> for Fixed {
        fn name(&self) -> &'static str {
            "fixed"
        }
        fn tile_size(&self) -> (i32, i32) {
            (16, 16)
        }
        fn view_tiles(&self, _: Rect) -> (i32, i32) {
            (1, 1)
        }
        fn grid(&self, scene: &MapScene<Piece>, _: Rect) -> Grid {
            Grid::top_down(scene.origin, (0, 0), (16, 16), (1, 1), 1)
        }
        fn walk(&self) -> WalkSpeeds {
            WalkSpeeds::of(3.0)
        }
        fn paint(
            &self,
            _: &Looks,
            _: &MapScene<Piece>,
            _: Rect,
            _: &dyn UnitPainter<Piece>,
            _: &mut GlyphBuffer,
        ) {
        }
        fn tile_px(&self, _: &MapScene<Piece>, _: Rect, _: Pos) -> Option<PxRect> {
            self.0
        }
    }

    fn cells(px: Option<PxRect>) -> Option<Rect> {
        let scene = MapScene::<Piece>::new(Pos::new(0, 0), (1, 1));
        Fixed(px).tile_cells(&scene, Rect::new(0, 0, 2, 1), Pos::new(0, 0))
    }

    #[test]
    fn tile_cells_are_the_cells_the_tile_touches() {
        let r = Rect::new;
        // Cells are 8 × 16 px. A glyph tile: two whole cells.
        assert_eq!(cells(Some(r(160, 176, 16, 16))), Some(r(20, 11, 2, 1)));
        // One pixel into the next cell, each way.
        assert_eq!(cells(Some(r(160, 176, 17, 17))), Some(r(20, 11, 3, 2)));
        assert_eq!(cells(Some(r(159, 175, 16, 16))), Some(r(19, 10, 3, 2)));
        // A 32 px tile: four cells by two.
        assert_eq!(cells(Some(r(32, 32, 32, 32))), Some(r(4, 2, 4, 2)));
        // Left of and above the console.
        assert_eq!(cells(Some(r(-16, -16, 16, 16))), Some(r(-2, -1, 2, 1)));
        assert_eq!(cells(Some(r(-1, -1, 2, 2))), Some(r(-1, -1, 2, 2)));
        assert_eq!(cells(None), None);
    }

    #[test]
    fn a_skins_tiles_are_where_its_grid_puts_them_unless_it_says() {
        #[derive(Debug)]
        struct Plain;
        impl GridSkin<Piece> for Plain {
            fn name(&self) -> &'static str {
                "plain_grid"
            }
            fn tile_size(&self) -> (i32, i32) {
                (16, 16)
            }
            fn view_tiles(&self, _: Rect) -> (i32, i32) {
                (2, 2)
            }
            fn grid(&self, scene: &MapScene<Piece>, _: Rect) -> Grid {
                Grid::top_down(scene.origin, (8, 32), (16, 16), (2, 2), 1)
            }
            fn walk(&self) -> WalkSpeeds {
                WalkSpeeds::of(3.0)
            }
            fn paint(
                &self,
                _: &Looks,
                _: &MapScene<Piece>,
                _: Rect,
                _: &dyn UnitPainter<Piece>,
                _: &mut GlyphBuffer,
            ) {
            }
        }
        let scene = MapScene::<Piece>::new(Pos::new(4, 4), (2, 2));
        let area = Rect::new(0, 0, 4, 4);
        assert_eq!(
            Plain.tile_px(&scene, area, Pos::new(5, 4)),
            Some(Rect::new(24, 32, 16, 16))
        );
        assert_eq!(Plain.tile_px(&scene, area, Pos::new(6, 4)), None);
        assert_eq!(
            Plain.tile_cells(&scene, area, Pos::new(5, 5)),
            Some(Rect::new(3, 3, 2, 1))
        );
        assert_eq!(Plain.walk(), WalkSpeeds::of(3.0));
    }

    #[test]
    fn walk_speeds_of_a_pace_are_the_same_fast_and_four_times_held() {
        let w = WalkSpeeds::of(5.0);
        assert_eq!((w.normal, w.fast, w.held), (5.0, 5.0, 20.0));
    }

    #[test]
    fn looks_name_the_palettes_colors_and_fall_back_to_the_engines() {
        let kit = Kit::new();
        let palette = &kit.palette;
        let looks = kit.looks();
        let lookup = |name| palette.lookup(name).unwrap();
        assert_eq!(
            looks.named("grass_bg", EngineColor::Text),
            lookup("grass_bg")
        );
        let text = palette.engine(EngineColor::Text);
        assert_eq!(looks.named("no_such_color", EngineColor::Text), text);
        let warning = palette.engine(EngineColor::Warning);
        assert_eq!(looks.named("no_such_color", EngineColor::Warning), warning);
        let colors = crate::toy::grid::COLORS;
        assert_eq!(looks.range(RangeName("move")), lookup("move"));
        assert_eq!(looks.range(RangeName("nothing")), text);
        assert_eq!(looks.path(&colors), lookup("path"));
        assert_eq!(looks.cursor(&colors), lookup("cursor"));
        let threat = looks.threat(&colors);
        assert_eq!(threat.arrow, lookup("foe"));
        assert_eq!(threat.mark, palette.engine(EngineColor::Highlight));
        assert_eq!(threat.edge, palette.engine(EngineColor::Background));
        assert_eq!(looks.dark(), palette.engine(EngineColor::Background));
        // A painter that paints nothing under or over the ground.
        let scene = MapScene::<Piece>::new(Pos::new(0, 0), (2, 2));
        let grid = Grid::top_down(scene.origin, (0, 0), (16, 16), (2, 2), 1);
        let mut buf = GlyphBuffer::new(8, 4, crate::frame::Cell::new('x', text, text));
        let before = buf.clone();
        NoUnits.paint_under(&scene, &grid, &mut buf);
        NoUnits.paint(&scene, &grid, &mut buf);
        assert_eq!(buf, before);
        assert!(format!("{looks:?}").starts_with("Looks"));
    }
}
