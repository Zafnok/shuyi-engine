//! The one-bit tests' colours and their pattern map: eight swatches, one
//! for each kind of thing a role can be drawn as (white, black, black with
//! a white edge, four patterns, and black that flips), each a role of the
//! map [`TEST_ROLES`] (ticket 2389; ticket 2388 proved this map draws as
//! the game's inks did).

use super::{PatternMap, PatternMapDef};
use crate::frame::color::Rgb;

/// A colour of the tests' palette, a role of [`TEST_ROLES`].
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum Swatch {
    /// White: the ground.
    White,
    /// Black.
    Black,
    /// Black, and on a glyph a white edge one pixel wide around it.
    BlackEdged,
    /// A 50% checker.
    Grey,
    /// Light dots, one pixel in eight.
    Dots,
    /// Diagonal lines, one pixel in four.
    Diagonals,
    /// A checker of 2 × 2 squares: a 1-pixel line of it is dashed.
    Dashes,
    /// Black, and a rectangle in it flips the pixels under it.
    Invert,
}

impl Swatch {
    /// Every swatch.
    pub(crate) const ALL: [Self; 8] = [
        Self::White,
        Self::Black,
        Self::BlackEdged,
        Self::Grey,
        Self::Dots,
        Self::Diagonals,
        Self::Dashes,
        Self::Invert,
    ];

    /// Its role's name.
    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::White => "white",
            Self::Black => "black",
            Self::BlackEdged => "black_edged",
            Self::Grey => "grey",
            Self::Dots => "dots",
            Self::Diagonals => "diagonals",
            Self::Dashes => "dashes",
            Self::Invert => "invert",
        }
    }

    /// Its colour: each about as dark as its pattern, so a frame drawn in
    /// colour still reads.
    pub(crate) const fn rgb(self) -> Rgb {
        match self {
            Self::White => Rgb::new(0xff, 0xff, 0xff),
            Self::Black => Rgb::new(0x00, 0x00, 0x00),
            Self::BlackEdged => Rgb::new(0x01, 0x01, 0x01),
            Self::Grey => Rgb::new(0x80, 0x80, 0x80),
            Self::Dots => Rgb::new(0xe0, 0xe0, 0xe0),
            Self::Diagonals => Rgb::new(0xc0, 0xc0, 0xc0),
            Self::Dashes => Rgb::new(0x40, 0x40, 0x40),
            Self::Invert => Rgb::new(0xff, 0x00, 0xff),
        }
    }
}

/// The colour of `swatch`.
pub(crate) const fn c(swatch: Swatch) -> Rgb {
    swatch.rgb()
}

/// The pattern map of the swatches' roles.
pub(crate) const TEST_ROLES: &str = r#"(
    roles: {
        "white": (fg: White),
        "black": (fg: Black),
        "black_edged": (fg: Black, edge: true),
        "grey": (fg: Dither50),
        "dots": (fg: Dots),
        "diagonals": (fg: DiagonalDown),
        "dashes": (fg: Checker2),
        "invert": (fg: Black, invert: true),
    },
)"#;

/// [`TEST_ROLES`] over the swatches' colours, named by role.
pub(crate) fn test_map() -> PatternMap {
    let def = PatternMapDef::from_source("test@onebit.ron", TEST_ROLES).unwrap();
    PatternMap::new(Swatch::ALL.map(|s| (s.name(), s.rgb())), &def)
}
