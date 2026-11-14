//! A recipient's mark in a picture's lowest bits (ADR-0118): a closed
//! build's overlay writes a recipient's [`Code`] into one picture the game
//! approves, and [`read`] finds it in a picture that leaked. Circumstantial
//! evidence, never proof: a lossy re-encode, a picture's colors reduced, or
//! a crop that drops one of its colors, loses the mark.
//!
//! The mark is by color, not by pixel, so the picture keeps its palette: a
//! pixel-art game's strict colors stay as many as they were. Each color
//! that is visible (its alpha above zero) and alone in its *group* (the
//! colors equal but for the two lowest bits of red, green and blue) carries
//! six bits of the code, in those bits: every pixel of that color changes
//! the same way, by at most 3 in each channel. The carriers, in the order
//! of their groups, hold the code's 40 bits one after another, and again
//! from the start as far as they go, so a picture with more than seven
//! carriers holds part of the code twice, and every copy must agree.
//!
//! A picture with fewer than seven carriers can't hold a code
//! ([`MarkError::TooFewColors`]): choose another.
use crate::prelude::*;

use alloc::collections::BTreeMap;

use super::Image;
use crate::recipient::{CODE_BYTES, Code};

/// The bits one color carries: the two lowest of red, green and blue.
pub const BITS_PER_COLOR: usize = 6;

/// The bits a code takes.
const CODE_BITS: usize = CODE_BYTES * 8;

/// The fewest colors that carry a code.
pub const MIN_CARRIERS: usize = CODE_BITS.div_ceil(BITS_PER_COLOR);

/// The bits a color's group ignores.
const LOW: u8 = 0b11;

/// Why a picture can't carry a mark.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum MarkError {
    /// It has too few colors that carry bits.
    #[error(
        "the picture has {found} color(s) that can carry a mark and a code needs {MIN_CARRIERS}: \
         choose a picture with more colors (each visible, and differing from the others above \
         its two lowest bits)"
    )]
    TooFewColors {
        /// Its carriers.
        found: usize,
    },
    /// Its bytes aren't four per pixel.
    #[error("the picture's pixels aren't 4 bytes each")]
    Shape,
}

/// What [`read`] found in a picture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PictureRead {
    /// A code, every copy of each bit agreeing and its check right.
    Code(Code),
    /// No code: the bits don't hold one, or two copies of a bit differ.
    NoMark,
    /// Too few colors to hold a code: a mark there was lost (a crop that
    /// dropped colors, a reduced palette), or there never was one.
    TooFewColors {
        /// Its carriers.
        found: usize,
    },
}

/// A color's group: its red, green and blue without their two lowest bits,
/// and its alpha.
fn group(color: [u8; 4]) -> [u8; 4] {
    let [r, g, b, a] = color;
    [r & !LOW, g & !LOW, b & !LOW, a]
}

/// The picture's distinct colors.
fn colors(image: &Image) -> Result<Vec<[u8; 4]>, MarkError> {
    let (pixels, rest) = image.rgba.as_chunks::<4>();
    if !rest.is_empty() {
        return Err(MarkError::Shape);
    }
    let mut colors: Vec<[u8; 4]> = pixels.to_vec();
    colors.sort_unstable();
    colors.dedup();
    Ok(colors)
}

/// How many distinct colors the picture has: its palette, which a mark
/// keeps.
pub fn palette_count(image: &Image) -> usize {
    colors(image).map_or(0, |colors| colors.len())
}

/// The colors that carry bits, in the order of their groups: each visible
/// and alone in its group.
fn carriers(image: &Image) -> Result<Vec<[u8; 4]>, MarkError> {
    let mut groups: BTreeMap<[u8; 4], Vec<[u8; 4]>> = BTreeMap::new();
    for color in colors(image)? {
        if color[3] > 0 {
            groups.entry(group(color)).or_default().push(color);
        }
    }
    Ok(groups
        .into_values()
        .filter(|colors| colors.len() == 1)
        .map(|colors| colors[0])
        .collect())
}

/// How many bits the picture can carry: six for each carrier.
pub fn capacity(image: &Image) -> usize {
    carriers(image).map_or(0, |carriers| carriers.len() * BITS_PER_COLOR)
}

/// The color `color` becomes carrying `bits` (six, highest first: red's,
/// green's, blue's).
fn carrying(color: [u8; 4], bits: [bool; BITS_PER_COLOR]) -> [u8; 4] {
    let [r, g, b, a] = group(color);
    let low = |high: bool, low: bool| u8::from(high) << 1 | u8::from(low);
    [
        r | low(bits[0], bits[1]),
        g | low(bits[2], bits[3]),
        b | low(bits[4], bits[5]),
        a,
    ]
}

/// The six bits `color` carries, highest first.
fn carried(color: [u8; 4]) -> [bool; BITS_PER_COLOR] {
    let [r, g, b, _] = color;
    let bit = |value: u8, at: u8| value >> at & 1 == 1;
    [
        bit(r, 1),
        bit(r, 0),
        bit(g, 1),
        bit(g, 0),
        bit(b, 1),
        bit(b, 0),
    ]
}

/// Writes `code` into `image`'s lowest bits, keeping its palette: every
/// pixel of a carrier changes as its color does, and no other pixel
/// changes.
pub fn mark(image: &mut Image, code: Code) -> Result<(), MarkError> {
    let carriers = carriers(image)?;
    if carriers.len() < MIN_CARRIERS {
        return Err(MarkError::TooFewColors {
            found: carriers.len(),
        });
    }
    let mut to: BTreeMap<[u8; 4], [u8; 4]> = BTreeMap::new();
    for (index, &color) in carriers.iter().enumerate() {
        let mut bits = [false; BITS_PER_COLOR];
        for (at, bit) in bits.iter_mut().enumerate() {
            *bit = code.bit((index * BITS_PER_COLOR + at) % CODE_BITS);
        }
        to.insert(color, carrying(color, bits));
    }
    let (pixels, _) = image.rgba.as_chunks_mut::<4>();
    for pixel in pixels {
        if let Some(&new) = to.get(pixel) {
            *pixel = new;
        }
    }
    Ok(())
}

/// The code `image` carries, if it carries one.
pub fn read(image: &Image) -> PictureRead {
    let Ok(carriers) = carriers(image) else {
        return PictureRead::NoMark;
    };
    if carriers.len() < MIN_CARRIERS {
        return PictureRead::TooFewColors {
            found: carriers.len(),
        };
    }
    let mut bits: [Option<bool>; CODE_BITS] = [None; CODE_BITS];
    for (index, &color) in carriers.iter().enumerate() {
        for (at, bit) in carried(color).into_iter().enumerate() {
            let place = &mut bits[(index * BITS_PER_COLOR + at) % CODE_BITS];
            match *place {
                Some(seen) if seen != bit => return PictureRead::NoMark,
                _ => *place = Some(bit),
            }
        }
    }
    let mut all = [false; CODE_BITS];
    for (bit, place) in all.iter_mut().zip(bits) {
        *bit = place.unwrap_or(false);
    }
    Code::from_bits(&all).map_or(PictureRead::NoMark, PictureRead::Code)
}

#[cfg(test)]
mod tests;
