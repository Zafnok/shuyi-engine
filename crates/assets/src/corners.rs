//! Corner sheets made, not drawn (the `corners` feature, off by default):
//! the 15 pictures of a [`CornerLayer`] made from a terrain's one tile
//! ([`generate`]), or turned from 5 drawn pieces ([`rotated`]), at tool
//! time and in integers. Hand-drawn corner layers stay the grid kit's
//! way; this is a tool a game opts into, for one terrain's edges or all of
//! them.
//!
//! A sheet is 8 × 2 pictures of the tile's size, mix `m` (the
//! [`mix`](crate::tileset::mix) of the four tiles round a point) at column
//! `m % 8`, row `m / 8` ([`sheet_rect`]), mix 0's place clear. It is an
//! ordinary picture, so every painter draws it as it draws a hand-drawn
//! one, and [`layer`] names its 15 pictures: pasted into the tileset's
//! picture, or as an image of its own ([`sheet_layer`]). A tool writes it to a PNG from
//! a [`recipe`], and keeps it equal to what the recipe makes
//! (`shuyi-cli`'s `corners` feature).
//!
//! # How a picture is made
//!
//! A picture's four corners are the centres of the four tiles round its
//! point, so its field is one bilinear patch of those tiles' in-values
//! (0 or 1, from the mix's bits 8, 4, 2 and 1: top-left, top-right,
//! bottom-left, bottom-right). A pixel is in where the field, plus an
//! optional [`Wobble`], is over one half:
//!
//! - one tile in gives a rounded quarter, three in the hole the other way;
//! - two pictures that share a border have the same field along it, and
//!   the wobble repeats with the picture's size, so they meet without a
//!   seam;
//! - two across (`#..#`, `.##.`) stays two pieces: the centre's field is
//!   exactly one half, which isn't over it.
//!
//! An in pixel takes the tile's pixel half a tile over (wrapping), so mix
//! 15 lines up with the plain tiles round it; an out pixel is clear. A
//! [`Rim`] colours the pixels near the edge, read from the field too.
//!
//! # Turned from five pieces
//!
//! Drawn by hand, a sheet needs only 5 pieces ([`PIECES`]): one corner in
//! (mix 8, `#...`), two side by side (12, `##..`), two across (9, `#..#`),
//! three in (14, `###.`) and all four (15). The other 10 are their quarter
//! turns. A quarter turn clockwise moves each corner to the next, top-left
//! to top-right to bottom-right to bottom-left, both for the pixels
//! (`(x, y)` to `(n − 1 − y, x)`, so a piece is square) and for the mix's
//! bits (8 to 4 to 1 to 2 to 8): 8 turns to 4, 1 and 2; 12 to 5, 3 and
//! 10; 9 to 6; 14 to 13, 7 and 11. Mix 15 is its piece as drawn.

use crate::prelude::*;

use alloc::collections::BTreeSet;

use serde::Deserialize;

use crate::image::{Image, ImageId, ImageRect};
use shuyi_core::TerrainId;

use crate::tileset::{CornerLayer, MIXES};

mod field;
pub mod recipe;
#[cfg(test)]
mod tests;

use field::Lattice;

/// A sheet's pictures across.
const COLUMNS: u32 = 8;

/// A sheet's pictures down.
const ROWS: u32 = 2;

/// The longest side a tile may have, in pixels.
pub const MAX_SIDE: u32 = 256;

/// The most lattice points a [`Wobble`] has across (and down).
pub const MAX_CELLS: u8 = 32;

/// The strongest a [`Wobble`] is.
pub const MAX_STRENGTH: u8 = 100;

/// A wavy edge: value noise added to the field, on a lattice that repeats
/// every picture. Build it with [`Wobble::new`] (ADR-0039).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct Wobble {
    /// How far the edge moves, 0 (not at all) to [`MAX_STRENGTH`] (up to
    /// half the field). 30 in [`new`](Self::new).
    pub strength: u8,
    /// Lattice points across and down each picture, 1 to [`MAX_CELLS`]:
    /// more is a busier edge. 4 in [`new`](Self::new).
    pub cells: u8,
    /// The seed the lattice's values are rolled from.
    pub seed: u64,
}

impl Wobble {
    /// The wobble of `seed`, of strength 30 on 4 × 4 lattice points.
    #[must_use]
    pub const fn new(seed: u64) -> Self {
        Self {
            strength: 30,
            cells: 4,
            seed,
        }
    }

    /// This wobble of strength `strength`.
    #[must_use]
    pub const fn with_strength(self, strength: u8) -> Self {
        Self { strength, ..self }
    }

    /// This wobble on `cells` × `cells` lattice points.
    #[must_use]
    pub const fn with_cells(self, cells: u8) -> Self {
        Self { cells, ..self }
    }
}

/// Which side of the edge a [`Rim`] is drawn on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[non_exhaustive]
pub enum RimSide {
    /// On the out pixels by the edge, opaque: foam on the water.
    Outside,
    /// On the in pixels by the edge, in place of the tile: a dark edge on
    /// the grass.
    Inside,
}

/// A line of one colour along the edge. Build it with [`Rim::new`]
/// (ADR-0039).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct Rim {
    /// Its colour, written `#RRGGBB` in a recipe.
    #[serde(deserialize_with = "hex")]
    pub colour: [u8; 3],
    /// How many pixels wide it is, measured in the field: one pixel is
    /// `2 / (w + h)` of it, a pixel across a straight edge on a square
    /// tile. 1 in [`new`](Self::new); 0 draws none.
    pub width: u8,
    /// Which side of the edge it is on.
    pub side: RimSide,
}

impl Rim {
    /// A rim of `colour` on `side`, one pixel wide.
    #[must_use]
    pub const fn new(colour: [u8; 3], side: RimSide) -> Self {
        Self {
            colour,
            width: 1,
            side,
        }
    }

    /// This rim, `width` pixels wide.
    #[must_use]
    pub const fn with_width(self, width: u8) -> Self {
        Self { width, ..self }
    }
}

/// A colour written `#RRGGBB`, as a palette's ([`crate::palette::parse_hex`]).
fn hex<'de, D: serde::Deserializer<'de>>(d: D) -> Result<[u8; 3], D::Error> {
    let text = String::deserialize(d)?;
    crate::palette::parse_hex(&text).map_err(serde::de::Error::custom)
}

/// Why a sheet can't be made.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum CornersError {
    /// The tile has no pixels.
    #[error("the tile is empty: {0:?} pixels across and down")]
    Empty((u32, u32)),
    /// A side of the tile is over [`MAX_SIDE`].
    #[error("the tile is {0:?} pixels across and down: a side may be at most 256")]
    TooLarge((u32, u32)),
    /// A wobble's `cells` is 0 or over [`MAX_CELLS`].
    #[error("a wobble's cells are 1 to 32, not {0}")]
    Cells(u8),
    /// A wobble's `strength` is over [`MAX_STRENGTH`].
    #[error("a wobble's strength is 0 to 100, not {0}")]
    Strength(u8),
    /// A piece of [`rotated`] isn't square.
    #[error("piece {piece} is {size:?} pixels across and down: a piece is square")]
    NotSquare {
        /// Which piece, from 0, in the order of [`PIECES`].
        piece: usize,
        /// Its size.
        size: (u32, u32),
    },
    /// A piece of [`rotated`] isn't the size of the first.
    #[error("piece {piece} is {size:?} pixels across and down, and the first is {first:?}")]
    PieceSize {
        /// Which piece, from 0, in the order of [`PIECES`].
        piece: usize,
        /// Its size.
        size: (u32, u32),
        /// The first piece's.
        first: (u32, u32),
    },
}

/// The mixes whose pieces [`rotated`] is given, in its order: one corner in
/// (`#...`), two side by side (`##..`), two across (`#..#`), three in
/// (`###.`) and all four (`####`).
pub const PIECES: [u8; 5] = [8, 12, 9, 14, 15];

/// The place of mix `m`'s picture in a sheet of `tile_px` pictures whose
/// top-left is at `origin`: column `m % 8`, row `m / 8`.
#[must_use]
pub const fn sheet_rect(origin: (u32, u32), tile_px: (u32, u32), m: u8) -> ImageRect {
    // A mix is below 16; `u32::from` isn't `const`.
    let m = m as u32;
    ImageRect::cell((m % COLUMNS, m / COLUMNS), tile_px, origin)
}

/// The corner layer of the terrains `of` drawn from the sheet of
/// `tile_px` pictures at `origin`: mixes 1 to 15 from their places
/// ([`sheet_rect`]), no tile skipped and no offset.
#[must_use]
pub fn layer(of: BTreeSet<TerrainId>, origin: (u32, u32), tile_px: (u32, u32)) -> CornerLayer {
    let mut tiles = [None; MIXES];
    for (m, tile) in (0u8..).zip(&mut tiles).skip(1) {
        *tile = Some(sheet_rect(origin, tile_px, m));
    }
    CornerLayer::new(of, tiles)
}

/// The corner layer of the terrains `of` drawn from the sheet `image` as
/// it was made, an image of its own: [`layer`] at `(0, 0)`, its pictures
/// in `image` rather than the tileset's ([`CornerLayer::image`]).
#[must_use]
pub fn sheet_layer(of: BTreeSet<TerrainId>, image: ImageId, tile_px: (u32, u32)) -> CornerLayer {
    layer(of, (0, 0), tile_px).with_image(image)
}

/// The size of `tile`, if it can be a tile.
fn tile_size(tile: &Image) -> Result<(u32, u32), CornersError> {
    let size = (tile.width, tile.height);
    if size.0 == 0 || size.1 == 0 {
        return Err(CornersError::Empty(size));
    }
    if size.0 > MAX_SIDE || size.1 > MAX_SIDE {
        return Err(CornersError::TooLarge(size));
    }
    Ok(size)
}

/// The sheet made from the one tile `tile`, its edges moved by `wobble`
/// and lined by `rim` ([module docs](self)).
///
/// # Errors
/// An empty tile, a side over [`MAX_SIDE`], or a wobble's `cells` or
/// `strength` out of range.
pub fn generate(
    tile: &Image,
    wobble: Option<Wobble>,
    rim: Option<Rim>,
) -> Result<Image, CornersError> {
    let size = tile_size(tile)?;
    let lattice = wobble.map(Lattice::new).transpose()?;
    let mut sheet = Image::filled(size.0 * COLUMNS, size.1 * ROWS, [0; 4]);
    let (w, h) = (i64::from(size.0), i64::from(size.1));
    for m in 1..16u8 {
        let at = sheet_rect((0, 0), size, m);
        for y in 0..size.1 {
            for x in 0..size.0 {
                let place = (2 * i64::from(x) + 1, 2 * i64::from(y) + 1);
                let wobbled = lattice.as_ref().map_or(0, |l| l.wobble((w, h), place));
                // Over the threshold, in the units of `2F`: `2F + W - 4wh`.
                let over = 2 * field::field(m, (w, h), place) + wobbled - 4 * w * h;
                let pixel = pixel(tile, (x, y), over, rim);
                put(&mut sheet, (at.x + x, at.y + y), pixel);
            }
        }
    }
    Ok(sheet)
}

/// The sheet made from the five drawn `pieces` of the mixes in [`PIECES`],
/// each other mix a quarter turn of one of them, clockwise once, twice or
/// three times ([module docs](self)). Mix 15's is its piece as drawn.
///
/// # Errors
/// An empty first piece or a side over [`MAX_SIDE`] (as a tile's), a
/// piece that isn't square, or one not the size of the first.
pub fn rotated(pieces: [&Image; 5]) -> Result<Image, CornersError> {
    let first = (pieces[0].width, pieces[0].height);
    for (piece, image) in pieces.iter().enumerate() {
        let size = (image.width, image.height);
        if piece == 0 {
            tile_size(image)?;
        } else if size != first {
            return Err(CornersError::PieceSize { piece, size, first });
        }
        if size.0 != size.1 {
            return Err(CornersError::NotSquare { piece, size });
        }
    }
    let mut sheet = Image::filled(first.0 * COLUMNS, first.1 * ROWS, [0; 4]);
    let mut filled = [false; MIXES];
    for (m, piece) in PIECES.into_iter().zip(pieces) {
        let (mut m, mut picture) = (m, piece.clone());
        for _ in 0..4 {
            if let Some(done) = filled.get_mut(usize::from(m)).filter(|done| !**done) {
                *done = true;
                let at = sheet_rect((0, 0), first, m);
                for y in 0..first.1 {
                    for x in 0..first.0 {
                        let pixel = picture.pixel(x, y).unwrap_or([0; 4]);
                        put(&mut sheet, (at.x + x, at.y + y), pixel);
                    }
                }
            }
            (m, picture) = (turn_mix(m), turn(&picture));
        }
    }
    Ok(sheet)
}

/// The square `picture` turned a quarter clockwise: `(x, y)` to
/// `(n − 1 − y, x)`.
fn turn(picture: &Image) -> Image {
    let n = picture.width;
    let mut turned = Image::filled(n, n, [0; 4]);
    for y in 0..n {
        for x in 0..n {
            let pixel = picture.pixel(x, y).unwrap_or([0; 4]);
            put(&mut turned, (n - 1 - y, x), pixel);
        }
    }
    turned
}

/// Mix `m` turned a quarter clockwise: top-left (8) to top-right (4),
/// top-right to bottom-right (1), bottom-right to bottom-left (2) and
/// bottom-left to top-left. The four bits moved are apart, so they add up
/// to the turned mix.
const fn turn_mix(m: u8) -> u8 {
    ((m & 8) >> 1) + ((m & 4) >> 2) + ((m & 1) << 1) + ((m & 2) << 2)
}

/// The colour of the pixel at `(x, y)` of a picture whose field is `over`
/// its threshold ([`generate`]).
fn pixel(tile: &Image, at: (u32, u32), over: i64, rim: Option<Rim>) -> [u8; 4] {
    let (w, h) = (i64::from(tile.width), i64::from(tile.height));
    // Within `width` pixels' worth of the threshold: `|over| < width × 8wh
    // × 2 / (w + h)`.
    let near = |rim: &Rim| over.abs() * (w + h) < i64::from(rim.width) * 16 * w * h;
    let edged = rim.filter(|rim| near(rim) && (rim.side == RimSide::Inside) == (over > 0));
    if let Some(rim) = edged {
        let [red, green, blue] = rim.colour;
        return [red, green, blue, 255];
    }
    if over <= 0 {
        return [0; 4];
    }
    // Half a tile over, as the painter centres a picture on its point: its
    // top-left half a tile, rounded down, up and left of it.
    let (across, down) = (tile.width, tile.height);
    let from = (
        (at.0 + across - across / 2) % across,
        (at.1 + down - down / 2) % down,
    );
    tile.pixel(from.0, from.1).unwrap_or([0; 4])
}

/// Sets the pixel at `(x, y)` of `image`, which holds it.
fn put(image: &mut Image, (x, y): (u32, u32), pixel: [u8; 4]) {
    let i = (y as usize * image.width as usize + x as usize) * 4;
    if let Some(to) = image.rgba.get_mut(i..i + 4) {
        to.copy_from_slice(&pixel);
    }
}
