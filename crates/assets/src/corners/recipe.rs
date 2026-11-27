//! A corner sheet's recipe: a RON file, `<name>.corners.ron`, saying what
//! the sheet is made from and where it goes. A tool's input, not an asset
//! kind: the game never loads it, only the PNG it makes.
//!
//! ```ron
//! (
//!     make: Generated(
//!         tile: (image: "tiles/terrain.png", at: (24, 0), size: (24, 24)),
//!         wobble: Some((strength: 30, cells: 4, seed: 7)),
//!         rim: Some((colour: "#E8F4FF", width: 2, side: Outside)),
//!     ),
//!     out: "tiles/water-corners.png",
//! )
//! ```
//!
//! or `make: Rotated(image: "tiles/shore.png", size: (16, 16), pieces:
//! [(0, 0), (16, 0), (32, 0), (48, 0), (64, 0)])`, the pieces of the mixes
//! [`PIECES`](super::PIECES) in that order. Paths are relative to the
//! recipe's folder. `wobble` and `rim` may be left out.

use crate::prelude::*;

use serde::Deserialize;

use crate::error::ContentError;
use crate::image::{Image, ImageRect};

use super::{Rim, Wobble};

/// The end of a recipe's file name.
pub const EXTENSION: &str = ".corners.ron";

/// A recipe, as [`parse`] reads it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct Recipe {
    /// What the sheet is made from.
    pub make: Make,
    /// The PNG it is written to, from the recipe's folder.
    pub out: String,
    /// The file it was read from, and the lines of its pictures.
    #[serde(skip)]
    from: Source,
}

/// Where a recipe was read from, for its errors.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct Source {
    file: String,
    /// The line of the tile's `at`, or the `pieces`.
    pictures: usize,
}

/// How a sheet is made.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub enum Make {
    /// From one tile ([`generate`](super::generate)).
    Generated {
        /// The tile.
        tile: Tile,
        /// Its edges' wobble, if any.
        wobble: Option<Wobble>,
        /// Its edges' rim, if any.
        rim: Option<Rim>,
    },
    /// From 5 drawn pieces and their quarter turns
    /// ([`rotated`](super::rotated)).
    Rotated {
        /// The picture the pieces are in, from the recipe's folder.
        image: String,
        /// A piece's size, across and down.
        size: (u32, u32),
        /// Each piece's top-left in it, for the mixes
        /// [`PIECES`](super::PIECES) in order: a list of 5.
        #[serde(deserialize_with = "five")]
        pieces: [(u32, u32); 5],
    },
}

/// A tile in a picture.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct Tile {
    /// The picture, from the recipe's folder.
    pub image: String,
    /// The tile's top-left in it.
    pub at: (u32, u32),
    /// Its size, across and down.
    pub size: (u32, u32),
}

/// A list of exactly 5 places.
fn five<'de, D: serde::Deserializer<'de>>(d: D) -> Result<[(u32, u32); 5], D::Error> {
    let places = Vec::<(u32, u32)>::deserialize(d)?;
    let count = places.len();
    places
        .try_into()
        .map_err(|_| serde::de::Error::custom(format!("5 pieces, not {count}")))
}

/// The recipe `text`, read from `file` (a path shown in errors, and the
/// one a tool remakes it from).
///
/// # Errors
/// What RON and the recipe's shape find, with the file, line and column:
/// an unknown field, a colour not `#RRGGBB`, a missing field.
pub fn parse(file: &str, text: &str) -> Result<Recipe, ContentError> {
    let mut recipe: Recipe = crate::ron::parse_ron(file, text)?;
    let key = match recipe.make {
        Make::Generated { .. } => "at",
        Make::Rotated { .. } => "pieces",
    };
    recipe.from = Source {
        file: file.to_owned(),
        pictures: line_of(text, key).unwrap_or(1),
    };
    Ok(recipe)
}

/// The line, from 1, of the first field `key` in `text`.
fn line_of(text: &str, key: &str) -> Option<usize> {
    let is_word = |c: char| c.is_alphanumeric() || c == '_';
    text.lines()
        .position(|line| {
            line.match_indices(key).any(|(at, _)| {
                let before = line[..at].chars().next_back();
                let after = line[at + key.len()..].trim_start();
                !before.is_some_and(is_word) && after.starts_with(':')
            })
        })
        .map(|line| line + 1)
}

impl Recipe {
    /// The file it was read from, as [`parse`] was given it.
    pub fn file(&self) -> &str {
        &self.from.file
    }

    /// The sheet it makes, with `picture` decoding the picture at a path
    /// written in the recipe (from its folder).
    ///
    /// # Errors
    /// `picture`'s, a tile or piece outside its picture (with the
    /// recipe's file and line), or the sheet's ([`CornersError`](super::CornersError)).
    pub fn sheet(&self, picture: impl Fn(&str) -> Result<Image, String>) -> Result<Image, String> {
        let decoded = |path: &str| picture(path).map_err(|e| format!("{path}: {e}"));
        let sheet = match &self.make {
            Make::Generated { tile, wobble, rim } => {
                let image = decoded(&tile.image)?;
                let tile = self.cut(&image, &tile.image, tile.at, tile.size, "the tile")?;
                super::generate(&tile, *wobble, *rim)
            }
            Make::Rotated {
                image,
                size,
                pieces,
            } => {
                let picture = decoded(image)?;
                let mut cut = Vec::with_capacity(pieces.len());
                for (i, at) in pieces.iter().enumerate() {
                    let what = format!("piece {}", i + 1);
                    cut.push(self.cut(&picture, image, *at, *size, &what)?);
                }
                let Ok(cut) = <&[Image; 5]>::try_from(&cut[..]) else {
                    return Err("a recipe has 5 pieces".to_owned());
                };
                super::rotated(cut.each_ref())
            }
        };
        sheet.map_err(|e| format!("{}: {e}", self.from.file))
    }

    /// The `size` picture at `at` of `image`, the picture at `path`.
    fn cut(
        &self,
        image: &Image,
        path: &str,
        at: (u32, u32),
        size: (u32, u32),
        what: &str,
    ) -> Result<Image, String> {
        let rect = ImageRect::cell((0, 0), size, at);
        if !image.info().holds(rect) {
            return Err(format!(
                "{}:{}: {what}, {}×{} pixels at ({}, {}), lies outside {path}, {}×{} pixels",
                self.from.file,
                self.from.pictures,
                size.0,
                size.1,
                at.0,
                at.1,
                image.width,
                image.height,
            ));
        }
        let mut cut = Image::filled(size.0, size.1, [0; 4]);
        for y in 0..size.1 {
            for x in 0..size.0 {
                let pixel = image.pixel(rect.x + x, rect.y + y).unwrap_or([0; 4]);
                super::put(&mut cut, (x, y), pixel);
            }
        }
        Ok(cut)
    }
}
