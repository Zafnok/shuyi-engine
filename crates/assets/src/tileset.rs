//! A tileset's terrain tiles (game ADR-0052): the pictures a grid map's
//! sprite skin paints its ground with. For each look a map may ask for
//! (outdoors, indoors), a tile for every terrain and the layers painted
//! over those tiles. A layer's picture is chosen from the tiles around it,
//! so a terrain grid alone says what the map looks like: a map file only
//! ever names terrain.
//!
//! Two kinds of layer:
//!
//! - **Corners**: pictures drawn *between* tiles, each centred on the
//!   point where four tiles meet, one for each mix of those four being in
//!   the layer or not. A shore, a road's edge and a forest's outline are
//!   these: they end where their tiles end.
//! - **Tiles**: one picture on each tile of the layer (a tower, a bridge),
//!   which may depend on which of its four neighbours are certain terrains
//!   (a bridge turns to cross the water).
//!
//! These are the tiles as a skin paints them; how a game writes them in a
//! file, and which terrains it has, are the game's.
use crate::prelude::*;

use alloc::collections::{BTreeMap, BTreeSet};

use shuyi_core::TerrainId;

use crate::format::AssetKind;
use crate::image::{ImageId, ImageRect};

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

/// The tileset kind (ADR-0007), with no format of the engine's: how a
/// game writes its tiles is its own, and its format reads them into this.
impl AssetKind for TerrainTiles {
    const NAME: &'static str = "tileset";
}

/// A tileset's terrain tiles.
#[derive(Debug, Clone, PartialEq, Eq)]
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
    /// The look a map naming `name` is painted as: that look if the
    /// tileset has it, else its own.
    pub fn look(&self, name: &str) -> &Look {
        self.looks.get(name).unwrap_or(&self.look)
    }
}

/// A terrain look: how one kind of place (outdoors, indoors) is drawn.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Look {
    /// Each terrain's own tile, painted first.
    pub tiles: BTreeMap<TerrainId, ImageRect>,
    /// What is painted over the tiles, the first undermost.
    pub layers: Vec<Layer>,
}

/// One layer of a [`Look`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Layer {
    /// Pictures between tiles.
    Corners(CornerLayer),
    /// A picture on each tile.
    Tiles(TileLayer),
}

/// Pictures drawn between tiles: each is one tile big and centred on a
/// point where four tiles meet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CornerLayer {
    /// The terrains that are in the layer.
    pub of: BTreeSet<TerrainId>,
    /// The picture for each [`mix`] of the four tiles round a point being
    /// in the layer. A mix with none draws nothing; mix 0 never has one.
    pub tiles: [Option<ImageRect>; MIXES],
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
        let own = Look {
            tiles: [(TerrainId(0), rect(0))].into(),
            layers: Vec::new(),
        };
        let indoor = Look {
            tiles: [(TerrainId(0), rect(16))].into(),
            layers: Vec::new(),
        };
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
}
