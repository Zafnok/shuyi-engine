//! A whole grid map painted once into a buffer just big enough, to pan
//! over (ticket 1154): a cinematic's backdrop ([`Pan`](crate::cinematic::Pan))
//! or the scene camera's ([`debug::camera`](crate::debug::camera)).

use alloc::rc::Rc;

use super::scene::MapScene;
use super::skin::{GridSkin, Looks, UnitPainter};
use crate::frame::console::ConsoleSpec;
use crate::frame::glyph_buffer::{Cell, GlyphBuffer, Rect};

/// An area surely big enough for one tile of any skin, to ask its size.
pub const ROOMY: Rect = Rect::new(0, 0, 64, 64);

/// A whole map as a skin painted it, to pan over.
#[derive(Debug, Clone, PartialEq)]
pub struct MapPicture {
    /// The picture: just big enough for the map.
    pub scene: Rc<GlyphBuffer>,
    /// One tile's size in the picture, in pixels.
    pub tile_px: (i32, i32),
}

impl MapPicture {
    /// All of `scene` (a view of a whole map, from its first tile) painted
    /// by `skin`, its units by `units`, into a buffer of `console`'s cells
    /// just big enough, cleared to `blank` first. `None` if the skin can't
    /// place the scene's first tile (a scene of no tiles).
    pub fn paint<U>(
        looks: &Looks,
        skin: &dyn GridSkin<U>,
        units: &dyn UnitPainter<U>,
        scene: &MapScene<U>,
        console: ConsoleSpec,
        blank: Cell,
    ) -> Option<Self> {
        let tile = skin.tile_px(scene, ROOMY, scene.origin)?;
        let paint = |buf: &mut GlyphBuffer| skin.paint(looks, scene, buf.bounds(), units, buf);
        Some(Self::painted(
            (tile.w, tile.h),
            scene.size,
            console,
            blank,
            paint,
        ))
    }

    /// A picture of `tiles` tiles (across × down) of `tile_px` pixels each,
    /// painted by `paint` into a buffer of `console`'s cells just big
    /// enough (rounded up to whole cells), cleared to `blank` first: for a
    /// painter that isn't a [`GridSkin`].
    pub fn painted(
        tile_px: (i32, i32),
        tiles: (i32, i32),
        console: ConsoleSpec,
        blank: Cell,
        paint: impl FnOnce(&mut GlyphBuffer),
    ) -> Self {
        let cells = |tiles: i32, tile_px: i32, cell_px: u16| {
            let cell_px = i32::from(cell_px);
            u16::try_from((tiles * tile_px + cell_px - 1) / cell_px).unwrap_or(0)
        };
        let (w, h) = (
            cells(tiles.0, tile_px.0, console.cell_w),
            cells(tiles.1, tile_px.1, console.cell_h),
        );
        let mut buf = GlyphBuffer::with_spec(console.sized(w, h), blank);
        paint(&mut buf);
        Self {
            scene: Rc::new(buf),
            tile_px,
        }
    }

    /// The picture's size in its own pixels.
    pub fn size_px(&self) -> (u32, u32) {
        pixel_size(&self.scene)
    }

    /// The picture's pixel at the map point `tiles` (a whole number is the
    /// middle of that tile).
    #[allow(clippy::cast_precision_loss)] // a tile is a few pixels
    pub fn point_px(&self, tiles: (f32, f32)) -> (f32, f32) {
        (
            (tiles.0 + 0.5) * self.tile_px.0 as f32,
            (tiles.1 + 0.5) * self.tile_px.1 as f32,
        )
    }
}

/// `buf`'s size in its own pixels.
pub fn pixel_size(buf: &GlyphBuffer) -> (u32, u32) {
    let px = buf.pixel_bounds();
    (
        u32::try_from(px.w).unwrap_or(0),
        u32::try_from(px.h).unwrap_or(0),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::color::{EngineColor, Rgb};
    use crate::grid::NoUnits;
    use crate::toy::grid::{Cards, Field, GLYPH, Kit, Letters, Piece, sprite, tiles};
    use shuyi_core::Pos;

    const CONSOLE: ConsoleSpec = ConsoleSpec::text(40, 20);

    fn blank(kit: &Kit) -> Cell {
        let c = |e| kit.palette.engine(e);
        Cell::new(' ', c(EngineColor::Text), c(EngineColor::Background))
    }

    /// The toy's whole field, painted by `skin` and `units`.
    fn picture(
        kit: &Kit,
        skin: &dyn GridSkin<Piece>,
        units: &dyn UnitPainter<Piece>,
        scene: &MapScene<Piece>,
    ) -> Option<MapPicture> {
        MapPicture::paint(&kit.looks(), skin, units, scene, CONSOLE, blank(kit))
    }

    #[test]
    fn a_glyph_picture_is_the_whole_field_with_its_pieces() {
        let kit = Kit::new();
        let scene = Field::glyphs().whole();
        // 10 × 8 tiles of two cells by one, 16 px each way.
        let shown = picture(&kit, &GLYPH, &Letters, &scene).unwrap();
        assert_eq!((shown.scene.width(), shown.scene.height()), (20, 8));
        assert_eq!(shown.tile_px, (16, 16));
        assert_eq!(shown.size_px(), (160, 128));
        // A whole number is a tile's middle.
        assert_eq!(shown.point_px((0.0, 0.0)), (8.0, 8.0));
        assert_eq!(shown.point_px((5.75, 3.5)), (100.0, 64.0));
        assert_eq!(shown.point_px((-0.5, 7.5)), (0.0, 128.0));
        // The pieces are on it: your piece's tile isn't bare grass.
        let bare = picture(&kit, &GLYPH, &NoUnits, &scene).unwrap();
        assert_eq!(shown.scene.get(4, 3).map(|c| c.glyph), Some('y'));
        assert_ne!(shown.scene.get(4, 3), bare.scene.get(4, 3));
        assert_eq!(shown.scene.get(0, 0), bare.scene.get(0, 0));
        assert_ne!(shown, bare);
        assert!(shown.scene.backdrop().is_none());
        assert!(shown.scene.sprites().is_empty());
    }

    #[test]
    fn a_sprite_picture_is_in_whole_cells_of_the_tiles_pixels() {
        let kit = Kit::new();
        let skin = sprite(Some(tiles(kit.tiles_image())));
        let cards = Cards { card: kit.card() };
        let scene = Field::glyphs().whole();
        // The toy's tiles are 24 px: 240 × 192, in whole cells.
        let shown = picture(&kit, &skin, &cards, &scene).unwrap();
        assert_eq!((shown.scene.width(), shown.scene.height()), (30, 12));
        assert_eq!(shown.tile_px, (24, 24));
        assert_eq!(shown.size_px(), (240, 192));
        assert_eq!(shown.point_px((1.0, 2.0)), (36.0, 60.0));
        // The ground's tiles and the two cards.
        assert_eq!(shown.scene.sprites().len(), 10 * 8 + 2);
        // A size that isn't whole cells is rounded up to them.
        let odd = MapScene::<Piece>::new(Pos::new(0, 0), (1, 1));
        let shown = picture(&kit, &skin, &cards, &odd).unwrap();
        assert_eq!((shown.scene.width(), shown.scene.height()), (3, 2));
        assert_eq!(shown.size_px(), (24, 32));
        // Nothing to paint, in either skin.
        let none = MapScene::<Piece>::new(Pos::new(0, 0), (0, 0));
        assert!(picture(&kit, &skin, &cards, &none).is_none());
        assert!(picture(&kit, &GLYPH, &Letters, &none).is_none());
    }

    #[test]
    fn a_picture_painted_by_hand_is_cleared_first_and_sized_in_cells() {
        let (ink, paper) = (Rgb::new(1, 2, 3), Rgb::new(4, 5, 6));
        let blank = Cell::new('.', ink, paper);
        let mut seen = None;
        let shown = MapPicture::painted((24, 24), (3, 2), CONSOLE, blank, |buf| {
            seen = buf.get(8, 2).copied();
            buf.set(0, 0, Cell::new('#', paper, ink));
        });
        assert_eq!(seen, Some(blank));
        // 72 × 48 px: 9 cells by 3.
        assert_eq!((shown.scene.width(), shown.scene.height()), (9, 3));
        assert_eq!(shown.scene.get(0, 0), Some(&Cell::new('#', paper, ink)));
        assert_eq!(pixel_size(&shown.scene), (72, 48));
    }
}
