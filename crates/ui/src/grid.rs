//! The tactics kit's map (game ADR-0038, game ADR-0088): a screen says *what*
//! is on the visible part of a grid map as a [`MapScene`] (plain data),
//! and a [`GridSkin`] says how it looks. The skin paints the ground (each
//! tile's terrain, flashing where it just changed), the ranges by their
//! names, the path, the cursor and the threat arrows; the units are the
//! game's own view of them, painted by the game's [`UnitPainter`] between
//! the skin's layers.
//!
//! Two skins: the [`GlyphSkin`] draws each tile as two glyph cells, in
//! its own look or another [`GlyphStyle`] (a one-bit screen's), and the
//! [`SpriteSkin`] paints the ground from a tileset's terrain tiles, with
//! layers between tiles by the corner rule ([`corners`]) and the map's
//! light, at any of the zooms it offers. Both lay their tiles out top-down
//! ([`Projection`]). How a game with painters of its own and several looks
//! uses them, with no skin trait of its own: [`skin`]'s docs.
//!
//! With them, the rest of a grid stage's look: the tile [`Camera`], the
//! virtual [`Cursor`], a path that follows it ([`steer()`]) and how long a
//! unit's walk is shown ([`walk`]); and a whole map painted once to pan
//! over ([`MapPicture`]). Anything new shown on a map goes in the scene
//! and is painted by every skin; it is never drawn straight into the
//! buffer by a screen.

pub mod camera;
pub mod corners;
pub mod cursor;
pub mod glyph;
pub mod layout;
pub mod path;
pub mod picture;
pub mod scene;
pub mod skin;
pub mod sprite;
pub mod steer;
#[cfg(test)]
pub(crate) mod testing;
pub mod threat;
pub mod walk;

pub use camera::Camera;
pub use cursor::Cursor;
pub use glyph::{GlyphSkin, GlyphStyle};
pub use layout::{CELL_PX, Grid, Projection, px_rect};
pub use picture::MapPicture;
pub use scene::{
    CursorStyle, CursorView, MapScene, MapUnit, RangeLayer, RangeName, ThreatView, TileView, Tints,
};
pub use skin::{
    GridSkin, Looks, MapColors, NoUnits, TerrainGlyphs, Terrains, UnitPainter, WalkSpeeds,
};
pub use sprite::SpriteSkin;
pub use steer::steer;
pub use walk::{Facing, Gait, STANDING_FRAME};
