//! A tileset's terrain tiles: the pictures a grid map's
//! sprite skin paints its ground with. For each look a map may ask for
//! (outdoors, indoors), a tile for every terrain and the layers painted
//! over those tiles. A layer's picture is chosen from the tiles around it,
//! so a terrain grid alone says what the map's ground looks like: a map
//! file names terrain, and only the few pictures placed by hand besides.
//!
//! Two kinds of layer:
//!
//! - **Corners**: pictures drawn *between* tiles, each placed by the
//!   point where four tiles meet, one for each mix of those four being in
//!   the layer or not. A shore, a road's edge and a forest's outline are
//!   these: they end where their tiles end. A picture is a tile big and
//!   centred on its point unless the layer says otherwise: it may be
//!   another size and sit off its point (raised ground's face hanging on
//!   the tile below it, a drop shadow shifted a few pixels), and the
//!   tiles of some terrains may get none of it (a wall left bare).
//! - **Tiles**: one picture on each tile of the layer (a tower, a bridge),
//!   which may depend on which of its four neighbours are certain terrains
//!   (a bridge turns to cross the water).
//!
//! Over the layers, a look may also have pictures a map places by hand
//! ([`Look::props`]): a lamp at a gate, where no terrain says one goes.
//! A file names them in a look's `props`, each its own.
//!
//! These are the tiles as a skin paints them. A file writes them as a
//! [`TerrainTilesDef`], read by the format [`TerrainTilesRon`] and checked
//! alone, then resolved against the image table and the game's terrains,
//! by name ([`TerrainTilesDef::resolve`]). Every picture is a `(column,
//! row)` cell of `tile_px` cells:
//!
//! ```ron
//! (
//!     image: "tiles/farm.png",
//!     tile_px: (16, 16),
//!     terrain: { "soil": (0, 0), "tilled": (1, 0), "water": (2, 0) },
//!     layers: [
//!         // The pond's shore: a picture by each point where four tiles
//!         // meet, for which of the four (up-left, up-right, down-left,
//!         // down-right) are water.
//!         Corners(of: ["water"], tiles: [
//!             (corners: "...#", at: (0, 1)),
//!             (corners: "..##", at: (1, 1)),
//!         ]),
//!         // Optional: `size: (16, 24)` (left out, `tile_px`),
//!         // `offset: (1, 3)` and `skip: ["soil"]` (none).
//!         Tiles(of: ["tilled"], at: (3, 0)),
//!     ],
//!     // Pictures a map places on its tiles by name, over the layers.
//!     // Optional: `size` (left out, `tile_px`).
//!     props: { "lamp": (at: (3, 1), size: (16, 32)), "sign": (at: (0, 2)) },
//!     // Other looks, by the name a map's look gives.
//!     looks: { "winter": (terrain: { "soil": (0, 2), "tilled": (1, 2), "water": (2, 2) }) },
//! )
//! ```
//!
//! Which terrains there are, and which looks a map may name, are the
//! game's: it gives them to the format ([`TileRules`]) and to `resolve`.
use crate::prelude::*;

use alloc::collections::{BTreeMap, BTreeSet};

use shuyi_core::TerrainId;

use crate::format::AssetKind;
use crate::image::{ImageId, ImageRect};

mod def;
pub use def::{
    CornersAt, LayerDef, LookDef, PropAt, SidesAt, TerrainTilesDef, TerrainTilesRon, TileRules,
};

/// How many mixes four yes-or-no places have: a layer has a picture for
/// each (or none).
pub const MIXES: usize = 16;

/// The character of a place that is in, in a mix as written (`"##.."`).
pub const MIX_IN: char = '#';

/// The character of a place that is out.
pub const MIX_OUT: char = '.';

/// The number of a mix of four places, the first the highest bit: for a
/// corners layer the tiles at the top-left, top-right, bottom-left and
/// bottom-right of a corner point; for a tile's sides its neighbours up,
/// right, down and left.
pub fn mix(places: [bool; 4]) -> u8 {
    places.iter().fold(0, |m, &p| 2 * m + u8::from(p))
}

/// The mix written as four of [`MIX_IN`] and [`MIX_OUT`], or `None` if
/// `text` isn't that.
pub fn parse_mix(text: &str) -> Option<u8> {
    let mut places = [false; 4];
    let mut chars = text.chars();
    for place in &mut places {
        *place = match chars.next()? {
            MIX_IN => true,
            MIX_OUT => false,
            _ => return None,
        };
    }
    chars.next().is_none().then(|| mix(places))
}

/// Mix `m` as written: four of [`MIX_IN`] and [`MIX_OUT`].
pub fn mix_name(m: u8) -> String {
    [8, 4, 2, 1]
        .map(|bit| if m & bit == 0 { MIX_OUT } else { MIX_IN })
        .iter()
        .collect()
}

/// The tileset kind (ADR-0007). The engine's file of it is
/// [`TerrainTilesDef`] (the format [`TerrainTilesRon`]), which
/// [`TerrainTilesDef::resolve`] makes into this; a game that writes its
/// tiles otherwise reads them into this with a format of its own.
impl AssetKind for TerrainTiles {
    const NAME: &'static str = "tileset";
}

/// A tileset's terrain tiles. Build it with [`TerrainTiles::new`]
/// (ADR-0039).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct TerrainTiles {
    /// The image the tiles are in.
    pub image: ImageId,
    /// A map tile's size, across × down, in pixels: in the image, and on
    /// screen.
    pub tile_px: (u32, u32),
    /// The tileset's own look: what a map is painted as unless
    /// [`looks`](Self::looks) has the look it names.
    pub look: Look,
    /// Its other looks, by the name a map's look gives
    /// ([`MapLook::tiles`](crate::grid_map::MapLook::tiles)).
    pub looks: BTreeMap<String, Look>,
}

impl TerrainTiles {
    /// The tiles of `image`, `tile_px` each, in the look `look` and no
    /// other (`looks` starts empty).
    #[must_use]
    pub const fn new(image: ImageId, tile_px: (u32, u32), look: Look) -> Self {
        Self {
            image,
            tile_px,
            look,
            looks: BTreeMap::new(),
        }
    }

    /// The look a map naming `name` is painted as: that look if the
    /// tileset has it, else its own.
    pub fn look(&self, name: &str) -> &Look {
        self.looks.get(name).unwrap_or(&self.look)
    }
}

/// A terrain look: how one kind of place (outdoors, indoors) is drawn.
/// Build it with [`Look::new`] or [`Default`] (ADR-0039).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct Look {
    /// Each terrain's own tile, painted first.
    pub tiles: BTreeMap<TerrainId, ImageRect>,
    /// What is painted over the tiles, the first undermost.
    pub layers: Vec<Layer>,
    /// What each name a map places on a tile
    /// ([`MapLook::props`](crate::grid_map::MapLook::props)) is drawn as,
    /// over the layers: a picture at its own size, standing on its tile
    /// (its bottom on the tile's bottom, centred across), so it may reach
    /// up over the tiles above. A name it lacks draws nothing. Empty in
    /// [`new`](Self::new).
    pub props: BTreeMap<String, ImageRect>,
}

impl Look {
    /// The look of the tiles `tiles` with the layers `layers` over them,
    /// and no picture for a map to place.
    #[must_use]
    pub const fn new(tiles: BTreeMap<TerrainId, ImageRect>, layers: Vec<Layer>) -> Self {
        Self {
            tiles,
            layers,
            props: BTreeMap::new(),
        }
    }
}

/// One layer of a [`Look`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Layer {
    /// Pictures between tiles.
    Corners(CornerLayer),
    /// A picture on each tile.
    Tiles(TileLayer),
}

/// Pictures drawn between tiles, one by each point where four tiles
/// meet. Build it with [`CornerLayer::new`] (ADR-0039).
///
/// A picture is drawn at its own size, its top-left half a tile (rounded
/// down) up and left of its point, moved by [`offset`](Self::offset): a
/// tile-sized picture with no offset is centred on its point. It is
/// drawn only inside the tiles it covers that are drawn, on the map and
/// not of a terrain in [`skip`](Self::skip).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct CornerLayer {
    /// The terrains that are in the layer.
    pub of: BTreeSet<TerrainId>,
    /// The picture for each [`mix`] of the four tiles round a point being
    /// in the layer. A mix with none draws nothing; mix 0 never has one.
    pub tiles: [Option<ImageRect>; MIXES],
    /// The terrains whose tiles get no part of the layer's pictures (a
    /// wall a cliff's face doesn't hang on). Empty in
    /// [`new`](Self::new).
    pub skip: BTreeSet<TerrainId>,
    /// Pixels across and down, at the art's size, added to where each
    /// picture's top-left goes. `(0, 0)` in [`new`](Self::new).
    pub offset: (i32, i32),
}

impl CornerLayer {
    /// The pictures `tiles` between the tiles of the terrains `of`, each
    /// where its own size puts it, no tile skipped and no offset.
    #[must_use]
    pub const fn new(of: BTreeSet<TerrainId>, tiles: [Option<ImageRect>; MIXES]) -> Self {
        Self {
            of,
            tiles,
            skip: BTreeSet::new(),
            offset: (0, 0),
        }
    }
}

/// A picture on each tile of some terrains.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TileLayer {
    /// The terrains that get the picture.
    pub of: BTreeSet<TerrainId>,
    /// The picture, unless [`sides`](Self::sides) has one.
    pub tile: ImageRect,
    /// The terrains a neighbour is looked at for.
    pub beside: BTreeSet<TerrainId>,
    /// The picture for a [`mix`] of the tile's four neighbours being one
    /// of [`beside`](Self::beside), where it differs from
    /// [`tile`](Self::tile).
    pub sides: [Option<ImageRect>; MIXES],
}

impl TileLayer {
    /// The picture of a tile whose neighbours mix as `sides`.
    pub fn picture(&self, sides: u8) -> ImageRect {
        let at = usize::from(sides);
        self.sides.get(at).copied().flatten().unwrap_or(self.tile)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image::ImageTable;

    #[test]
    fn a_mix_is_four_places_the_first_the_highest_bit() {
        assert_eq!(mix([false; 4]), 0);
        assert_eq!(mix([true, false, false, false]), 8);
        assert_eq!(mix([false, true, false, false]), 4);
        assert_eq!(mix([false, false, true, false]), 2);
        assert_eq!(mix([false, false, false, true]), 1);
        assert_eq!(mix([true; 4]), 15);
        assert_eq!(mix([true, false, true, true]), 11);
    }

    #[test]
    fn a_mix_is_written_as_four_of_hash_and_dot() {
        for m in 0..16 {
            assert_eq!(parse_mix(&mix_name(m)), Some(m), "{m}");
        }
        assert_eq!(mix_name(0), "....");
        assert_eq!(mix_name(8), "#...");
        assert_eq!(mix_name(5), ".#.#");
        assert_eq!(mix_name(15), "####");
        assert_eq!(parse_mix("#..#"), Some(9));
        for bad in ["", "#", "###", "#####", "##.x", "x#..", "##. "] {
            assert_eq!(parse_mix(bad), None, "{bad:?}");
        }
    }

    fn rect(x: u32) -> ImageRect {
        ImageRect {
            x,
            y: 0,
            w: 16,
            h: 16,
        }
    }

    #[test]
    fn a_tile_layer_shows_its_sides_picture_where_it_has_one() {
        let mut sides = [None; MIXES];
        sides[5] = Some(rect(32));
        let layer = TileLayer {
            of: BTreeSet::new(),
            tile: rect(16),
            beside: BTreeSet::new(),
            sides,
        };
        assert_eq!(layer.picture(5), rect(32));
        assert_eq!(layer.picture(0), rect(16));
        assert_eq!(layer.picture(15), rect(16));
        // No such mix: the plain picture.
        assert_eq!(layer.picture(16), rect(16));
        assert_eq!(layer.picture(u8::MAX), rect(16));
    }

    #[test]
    fn a_new_corner_layer_skips_nothing_and_sits_on_its_points() {
        let mut tiles = [None; MIXES];
        tiles[15] = Some(rect(16));
        let of = BTreeSet::from([TerrainId(1)]);
        assert_eq!(
            CornerLayer::new(of.clone(), tiles),
            CornerLayer {
                of,
                tiles,
                skip: BTreeSet::new(),
                offset: (0, 0),
            }
        );
    }

    #[test]
    fn a_map_is_painted_in_the_look_it_names_if_the_tileset_has_it() {
        let table = ImageTable {
            images: [(
                "tiles.png",
                crate::image::ImageInfo {
                    width: 64,
                    height: 64,
                },
            )]
            .into(),
        };
        let own = Look::new([(TerrainId(0), rect(0))].into(), Vec::new());
        let indoor = Look::new([(TerrainId(0), rect(16))].into(), Vec::new());
        let tiles = TerrainTiles {
            image: table.id("tiles.png").unwrap(),
            tile_px: (16, 16),
            look: own.clone(),
            looks: [("indoor".to_owned(), indoor.clone())].into(),
        };
        assert_eq!(tiles.look("indoor"), &indoor);
        assert_eq!(tiles.look("outdoor"), &own);
        assert_eq!(tiles.look("cave"), &own);
    }

    #[test]
    fn a_new_look_places_no_picture() {
        let tiles = BTreeMap::from([(TerrainId(0), rect(0))]);
        let layers = vec![Layer::Corners(CornerLayer::new(
            BTreeSet::new(),
            [None; MIXES],
        ))];
        assert_eq!(
            Look::new(tiles.clone(), layers.clone()),
            Look {
                tiles,
                layers,
                props: BTreeMap::new(),
            }
        );
        assert_eq!(Look::default(), Look::new(BTreeMap::new(), Vec::new()));
    }

    #[test]
    fn new_tiles_are_the_literal_with_no_other_look() {
        let look = Look::new(BTreeMap::new(), alloc::vec::Vec::new());
        let images = ImageTable::new([("tiles.png", crate::image::ImageInfo::new(48, 24))].into());
        let image = images.id("tiles.png").unwrap();
        assert_eq!(
            TerrainTiles::new(image, (24, 24), look.clone()),
            TerrainTiles {
                image,
                tile_px: (24, 24),
                look,
                looks: BTreeMap::new(),
            }
        );
    }
}
