//! Making a colour picture one-bit at bake time, in integers (ADR-0008
//! §4): `shuyi bake --target playdate` makes every picture a one-bit
//! picture file once, so the device never dithers. The same picture and
//! settings give the same file on every platform, so the baked file is
//! tested equal to dithering its source again (ADR-0007 §4).
//!
//! How a picture is made one-bit is its import settings' `onebit`
//! section ([`OneBitSettings`], ADR-0057): `images/hero.png.meta` holding
//! `(onebit: (dither: bayer))` sets one picture, an `_folder.meta` a
//! folder. The picture keeps its size: one dot a pixel.
//!
//! Each pixel is first a **brightness** from 0 (black) to 255 (white),
//! `(77 r + 150 g + 29 b + 128) / 256`, turned over (`255 − b`) when the
//! settings say [`invert`](OneBitSettings::invert); then the
//! [`Dither`] makes it black or white. A pixel whose alpha is below
//! [`alpha_cut`](OneBitSettings::alpha_cut) is see-through: clear in the
//! mask (the file has one only if some pixel is), white in the picture,
//! and no error spreads to or from it.
//!
//! [`Dithered`] (with the `png` feature) is the format that reads a PNG
//! as a [`OneBitPicture`] this way.
use crate::prelude::*;

use serde::Deserialize;

use super::encode;
use crate::image::Image;
use crate::import_settings::ImportSettings;
use crate::onebit::BAYER;
#[cfg(feature = "png")]
use {
    super::OneBitPicture,
    crate::error::ContentError,
    crate::format::{AssetPath, DecodeCx, Format},
    crate::image::{MAX_PNG_BYTES, PNG_SIGNATURE, Png},
};

/// How a picture's greys become black and white.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Dither {
    /// Atkinson's error diffusion, for pictures: each pixel is white from
    /// a brightness of 128, and an eighth of how far that is from its
    /// brightness goes to each of six pixels after it (the next two on
    /// its row, three on the row below, one two rows down), so a grey
    /// keeps its share of white. Three quarters of the error spread, so
    /// light and dark areas keep their contrast.
    #[default]
    Atkinson,
    /// The 8 × 8 Bayer ordered dither, for flat art and anything that
    /// moves: a pixel is black where `4 × order + 2` is below its
    /// darkness (`255 − brightness`), [`BAYER`]'s order at its place in
    /// the picture, so a flat grey is the pattern
    /// [`onebit::dither`](crate::onebit::dither) gives it.
    Bayer,
    /// White from this brightness up, black below it: for line art.
    Threshold(u8),
    /// No dither: every drawn pixel must already be black or white
    /// (`#000000` or `#ffffff`), and any other is an error.
    None,
}

/// A picture's `onebit` import settings section (ADR-0057): how it is
/// made one-bit. Every field may be left out: `(onebit: ())` is the
/// default, Atkinson with a cut at 128.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(default, deny_unknown_fields)]
#[non_exhaustive]
pub struct OneBitSettings {
    /// How its greys become black and white.
    pub dither: Dither,
    /// The alpha below which a pixel is see-through (0 for none).
    pub alpha_cut: u8,
    /// Whether its brightness is turned over first, so black becomes
    /// white.
    pub invert: bool,
}

impl OneBitSettings {
    /// The default settings, made one-bit by `dither`.
    #[must_use]
    pub const fn new(dither: Dither) -> Self {
        Self {
            dither,
            alpha_cut: 128,
            invert: false,
        }
    }
}

impl Default for OneBitSettings {
    fn default() -> Self {
        Self::new(Dither::Atkinson)
    }
}

impl ImportSettings for OneBitSettings {
    const SECTION: &'static str = "onebit";
}

/// A picture's brightness from 0 to 255 of the pixel `[r, g, b, _]`.
pub fn brightness([r, g, b, _]: [u8; 4]) -> u8 {
    let sum = 77 * u32::from(r) + 150 * u32::from(g) + 29 * u32::from(b) + 128;
    // At most (256 × 255 + 128) / 256: 255.
    u8::try_from(sum >> 8).unwrap_or(u8::MAX)
}

/// The one-bit picture file of `image` made by `settings`: one style the
/// image's size, and a mask if a pixel is see-through. Its source size is
/// the image's own. An error, in plain words, for a picture too big for
/// the file (a side over 65,535 pixels), or for one with a grey under
/// [`Dither::None`].
pub fn dither(image: &Image, settings: OneBitSettings) -> Result<Vec<u8>, String> {
    let too_big = || {
        format!(
            "{}×{} px is too big for a one-bit picture",
            image.width, image.height
        )
    };
    let width = u16::try_from(image.width).map_err(|_| too_big())?;
    let height = u16::try_from(image.height).map_err(|_| too_big())?;
    let size = usize::from(width) * usize::from(height);
    if width == 0 || height == 0 || image.rgba.len() != size * 4 {
        return Err(format!(
            "a {}×{} px picture needs {} bytes of pixels, and has {}",
            image.width,
            image.height,
            size * 4,
            image.rgba.len()
        ));
    }
    let pixels = image.rgba.as_chunks::<4>().0;
    let opaque: Vec<bool> = pixels.iter().map(|p| p[3] >= settings.alpha_cut).collect();
    let light: Vec<u8> = pixels
        .iter()
        .map(|&p| {
            let b = brightness(p);
            if settings.invert { 255 - b } else { b }
        })
        .collect();
    let picture = Picture {
        width: usize::from(width),
        opaque: &opaque,
    };
    let white = match settings.dither {
        Dither::Atkinson => picture.atkinson(&light),
        Dither::Bayer => picture.map(&light, |l, x, y| {
            u16::from(BAYER[y % 8][x % 8]) * 4 + 2 >= u16::from(255 - l)
        }),
        Dither::Threshold(cut) => picture.map(&light, |l, _, _| l >= cut),
        Dither::None => {
            if let Some(at) = (0..size).find(|&i| opaque[i] && !is_black_or_white(pixels[i])) {
                let [r, g, b, _] = pixels[at];
                return Err(format!(
                    "the picture isn't black and white, as `dither: none` needs: the pixel at \
                     column {}, row {} is #{r:02x}{g:02x}{b:02x}",
                    at % picture.width,
                    at / picture.width
                ));
            }
            picture.map(&light, |l, _, _| l >= 128)
        }
    };
    let style = rows(picture.width, &white);
    let mask = opaque
        .contains(&false)
        .then(|| rows(picture.width, &opaque));
    encode((width, height), (width, height), &[&style], mask.as_deref())
        .map_err(ToString::to_string)
}

/// A PNG made a [`OneBitPicture`] by its `onebit` import settings
/// (`dithered`, `.png`, with the `png` feature): its pixels read as
/// [`Png`] reads them, then [`dither`]ed. A tool registers it; a one-bit
/// device reads the pictures it made with
/// [`OneBitFile`](super::OneBitFile).
#[cfg(feature = "png")]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Dithered;

#[cfg(feature = "png")]
impl Format for Dithered {
    type Asset = OneBitPicture;

    fn id(&self) -> &'static str {
        "dithered"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["png"]
    }

    fn sniff(&self, bytes: &[u8]) -> bool {
        bytes.starts_with(PNG_SIGNATURE)
    }

    fn max_bytes(&self) -> usize {
        MAX_PNG_BYTES
    }

    fn decode(
        &self,
        file: &AssetPath,
        bytes: &[u8],
        cx: &DecodeCx<'_>,
    ) -> Result<OneBitPicture, Vec<ContentError>> {
        let image = Png.decode(file, bytes, cx)?;
        let settings = cx.import_settings::<OneBitSettings>(file)?;
        let error = |message: String| vec![ContentError::new(file.shown(), message)];
        let made = dither(&image, settings).map_err(error)?;
        OneBitPicture::new(made).map_err(|e| error(e.to_owned()))
    }
}

/// Whether pixel `[r, g, b, _]` is pure black or pure white.
fn is_black_or_white([r, g, b, _]: [u8; 4]) -> bool {
    r == g && g == b && (r == 0 || r == 255)
}

/// What the dithers need of a picture: its width, and which pixels are
/// drawn, row by row.
struct Picture<'a> {
    width: usize,
    opaque: &'a [bool],
}

impl Picture<'_> {
    /// Each pixel white where `white(brightness, x, y)` says; a
    /// see-through one white.
    fn map(&self, light: &[u8], white: impl Fn(u8, usize, usize) -> bool) -> Vec<bool> {
        light
            .iter()
            .enumerate()
            .map(|(i, &l)| !self.opaque[i] || white(l, i % self.width, i / self.width))
            .collect()
    }

    /// Atkinson's error diffusion ([`Dither::Atkinson`]): each pixel in
    /// order, row by row, white from 128, its error over 8 (rounded
    /// toward 0) added to each drawn pixel of the six after it.
    fn atkinson(&self, light: &[u8]) -> Vec<bool> {
        /// Where the error goes, as `(dx, dy)`.
        const SPREAD: [(isize, usize); 6] = [(1, 0), (2, 0), (-1, 1), (0, 1), (1, 1), (0, 2)];
        let height = light.len() / self.width;
        // Six eighths of each error spread: a level stays between about
        // -400 and 700, well inside an `i16`.
        let mut level: Vec<i16> = light.iter().map(|&l| i16::from(l)).collect();
        let mut white = vec![true; light.len()];
        for y in 0..height {
            for x in 0..self.width {
                let i = y * self.width + x;
                if !self.opaque[i] {
                    continue;
                }
                let old = level[i];
                white[i] = old >= 128;
                let share = (old - if white[i] { 255 } else { 0 }) / 8;
                for (dx, dy) in SPREAD {
                    let Some(nx) = x.checked_add_signed(dx).filter(|&nx| nx < self.width) else {
                        continue;
                    };
                    let n = (y + dy) * self.width + nx;
                    if y + dy < height && self.opaque[n] {
                        level[n] += share;
                    }
                }
            }
        }
        white
    }
}

/// The rows of a plane `width` pixels wide, a pixel's bit set where
/// `set` is.
fn rows(width: usize, set: &[bool]) -> Vec<u8> {
    let stride = width.div_ceil(8);
    let mut rows = vec![0u8; set.len() / width * stride];
    for (i, _) in set.iter().enumerate().filter(|&(_, &on)| on) {
        let (x, y) = (i % width, i / width);
        rows[y * stride + x / 8] |= 0x80 >> (x % 8);
    }
    rows
}

#[cfg(test)]
pub(crate) mod tests;
