//! A skin's one-bit look by role (ADR-0008 §4): [`PatternMap`] draws each
//! colour of the palette in its role's patterns, [`to_one_bit`] draws a
//! frame with it into a [`OneBitFrame`], and [`changed_rows`] says which
//! rows of the screen two frames differ in, as the Playdate sends only
//! those to its display. Integers only, so a frame is the same bits on
//! every platform.

use core::fmt;

use alloc::collections::BTreeMap;

use shuyi_assets::onebit::{PatternMapDef, dither};

use super::{FRAME_BYTES, Font, Frame, HEIGHT, Images, ROW_BYTES, Shade, Shades, WIDTH};
use crate::frame::color::Rgb;
use crate::frame::glyph_buffer::GlyphBuffer;
use crate::prelude::*;

/// A colour's luminance, 0 (black) to 255 (white), in integer Rec. 601
/// weights: `(77 r + 150 g + 29 b) >> 8`.
#[allow(clippy::cast_possible_truncation)] // at most 255 × 256 >> 8
pub const fn luminance(rgb: Rgb) -> u8 {
    ((77 * rgb.r as u32 + 150 * rgb.g as u32 + 29 * rgb.b as u32) >> 8) as u8
}

/// A role's patterns, as the renderer draws them.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Role {
    name: String,
    fg: Shade,
    bg: Shade,
}

/// What each colour of a skin's palette is drawn in on a one-bit target:
/// the patterns of its role, by the skin's pattern map. A colour that is
/// no role's is dithered by its [`luminance`] (Bayer 8 × 8, as
/// [`dither`] does); a glyph in it is white or black, whichever is
/// nearer, as text is never dithered.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PatternMap {
    /// Each colour's role, with its patterns.
    roles: BTreeMap<Rgb, Role>,
}

impl PatternMap {
    /// The map of the roles of `palette` (name and colour, as
    /// [`Palette::iter`](crate::frame::Palette::iter) gives them) by
    /// pattern map `map`. A role `map` lacks has no patterns, so its
    /// colour is drawn as a colour of no role. When roles share a colour,
    /// the first that `palette` gives with patterns is its role (the
    /// frame holds colours, so it can't tell them apart:
    /// `shuyi_assets::onebit::check` says so).
    pub fn new<'a>(palette: impl IntoIterator<Item = (&'a str, Rgb)>, map: &PatternMapDef) -> Self {
        let mut roles = BTreeMap::new();
        for (name, rgb) in palette {
            if let Some(patterns) = map.get(name) {
                roles.entry(rgb).or_insert_with(|| Role {
                    name: name.to_owned(),
                    fg: patterns.fg.into(),
                    bg: patterns.background().into(),
                });
            }
        }
        Self { roles }
    }

    /// The role `rgb` is drawn as, if it is one's.
    pub fn role_of(&self, rgb: Rgb) -> Option<&str> {
        self.roles.get(&rgb).map(|role| role.name.as_str())
    }

    /// `rgb` dithered by its luminance.
    fn dithered(rgb: Rgb) -> Shade {
        Shade::rows(dither(255 - luminance(rgb)))
    }
}

impl Shades for PatternMap {
    /// The role's background, or the colour dithered.
    fn ground(&self, rgb: Rgb) -> Shade {
        self.roles
            .get(&rgb)
            .map_or_else(|| Self::dithered(rgb), |role| role.bg)
    }

    /// The role's foreground, or solid white or black by the colour's
    /// luminance.
    fn glyph(&self, rgb: Rgb) -> Shade {
        self.roles.get(&rgb).map_or_else(
            || {
                let solid = if luminance(rgb) >= 128 { u8::MAX } else { 0 };
                Shade::rows([solid; 8])
            },
            |role| role.fg,
        )
    }

    /// The role's foreground, or the colour dithered.
    fn shape(&self, rgb: Rgb) -> Shade {
        self.roles
            .get(&rgb)
            .map_or_else(|| Self::dithered(rgb), |role| role.fg)
    }
}

/// A one-bit picture of the display, 400 × 240, in the Playdate's layout:
/// each row [`ROW_BYTES`] bytes (padded to a whole number of 32-bit words,
/// as its display takes them), a byte's highest bit its left pixel, a set
/// bit white.
#[derive(Clone, PartialEq, Eq)]
pub struct OneBitFrame {
    bytes: Box<Frame>,
}

impl OneBitFrame {
    /// The display, all white.
    pub fn white() -> Self {
        Self {
            bytes: Box::new([u8::MAX; FRAME_BYTES]),
        }
    }

    /// Its width in pixels.
    pub const fn width(&self) -> i32 {
        WIDTH
    }

    /// Its height in pixels.
    pub const fn height(&self) -> i32 {
        HEIGHT
    }

    /// Its bytes, row after row.
    pub fn bytes(&self) -> &Frame {
        &self.bytes
    }

    /// Its bytes, to draw in.
    pub fn bytes_mut(&mut self) -> &mut Frame {
        &mut self.bytes
    }

    /// Whether pixel `(x, y)` is white; off the display, `false`.
    pub fn is_white(&self, x: i32, y: i32) -> bool {
        super::is_white(&self.bytes, x, y)
    }

    /// The rows it differs from `previous` in.
    pub fn changed_rows(&self, previous: &Self) -> RowMask {
        changed_rows(&previous.bytes, &self.bytes)
    }
}

impl fmt::Debug for OneBitFrame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "OneBitFrame({WIDTH} × {HEIGHT})")
    }
}

/// Draws `frame` as a one-bit target shows it: its colours in their roles'
/// patterns by `map`, glyphs from `font`, sprites' one-bit pictures from
/// `images` ([`render_with`](super::render_with)). The frame is drawn
/// from the display's top-left corner, and what lies past 400 × 240
/// pixels is cut off.
pub fn to_one_bit(
    frame: &GlyphBuffer,
    map: &PatternMap,
    font: &Font<'_>,
    images: &impl Images,
) -> OneBitFrame {
    let mut out = OneBitFrame::white();
    super::render_with(frame, font, images, map, out.bytes_mut());
    out
}

/// A set of the display's rows, a bit each.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct RowMask([u32; RowMask::WORDS]);

impl RowMask {
    /// Words of 32 rows, enough for the display's.
    const WORDS: usize = (HEIGHT as usize).div_ceil(32);

    /// Adds row `y` (a row off the display adds nothing).
    #[allow(clippy::cast_sign_loss)] // checked first
    pub fn insert(&mut self, y: i32) {
        if (0..HEIGHT).contains(&y) {
            self.0[y as usize / 32] |= 1 << (y % 32);
        }
    }

    /// Whether it holds row `y`.
    #[allow(clippy::cast_sign_loss)] // checked first
    pub fn contains(&self, y: i32) -> bool {
        (0..HEIGHT).contains(&y) && self.0[y as usize / 32] >> (y % 32) & 1 == 1
    }

    /// How many rows it holds.
    pub fn count(&self) -> u32 {
        self.0.iter().map(|word| word.count_ones()).sum()
    }

    /// Whether it holds none.
    pub fn is_empty(&self) -> bool {
        self.0.iter().all(|&word| word == 0)
    }

    /// The rows it holds, top to bottom.
    pub fn rows(&self) -> impl Iterator<Item = i32> + '_ {
        (0..HEIGHT).filter(|&y| self.contains(y))
    }
}

/// The rows `new` differs from `old` in.
pub fn changed_rows(old: &Frame, new: &Frame) -> RowMask {
    let mut mask = RowMask::default();
    let rows = old.as_chunks::<ROW_BYTES>().0.iter().zip(new.as_chunks().0);
    for (y, (a, b)) in (0..).zip(rows) {
        if a != b {
            mask.insert(y);
        }
    }
    mask
}

#[cfg(test)]
mod tests;
