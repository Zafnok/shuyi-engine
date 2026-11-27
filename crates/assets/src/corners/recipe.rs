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
//! Paths are relative to the recipe's folder. `wobble` and `rim` may be
//! left out.

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
    /// The line of the tile's `at`.
    tile: usize,
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

/// The recipe `text`, read from `file` (a path shown in errors, and the
/// one a tool remakes it from).
///
/// # Errors
/// What RON and the recipe's shape find, with the file, line and column:
/// an unknown field, a colour not `#RRGGBB`, a missing field.
pub fn parse(file: &str, text: &str) -> Result<Recipe, ContentError> {
    let mut recipe: Recipe = crate::ron::parse_ron(file, text)?;
    recipe.from = Source {
        file: file.to_owned(),
        tile: line_of(text, "at").unwrap_or(1),
    };
    Ok(recipe)
}

/// The line, from 1, of the first field `key` in `text`.
pub(super) fn line_of(text: &str, key: &str) -> Option<usize> {
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
    /// `picture`'s, a tile outside its picture (with the
    /// recipe's file and line), or the sheet's ([`CornersError`](super::CornersError)).
    pub fn sheet(&self, picture: impl Fn(&str) -> Result<Image, String>) -> Result<Image, String> {
        let Make::Generated { tile, wobble, rim } = &self.make;
        let image = picture(&tile.image).map_err(|e| format!("{}: {e}", tile.image))?;
        let cut = self.cut(&image, tile)?;
        super::generate(&cut, *wobble, *rim).map_err(|e| format!("{}: {e}", self.from.file))
    }

    /// The tile `tile` cut out of its picture, `image`.
    fn cut(&self, image: &Image, tile: &Tile) -> Result<Image, String> {
        let (at, size) = (tile.at, tile.size);
        let rect = ImageRect::cell((0, 0), size, at);
        if !image.info().holds(rect) {
            return Err(format!(
                "{}:{}: the tile, {}×{} pixels at ({}, {}), lies outside {}, {}×{} pixels",
                self.from.file,
                self.from.tile,
                size.0,
                size.1,
                at.0,
                at.1,
                tile.image,
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
