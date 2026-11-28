//! A part set's look painted (the `parts` feature, off by default): the
//! skin's half of `shuyi_assets::parts`. The rules keep a person's seed
//! and kind; the skin rolls their [`Appearance`] with
//! [`PartSet::appearance`] and [`paint`]s it into a [`Painted`] picture,
//! which every painter draws (ADR-0078):
//!
//! ```ignore
//! let look = set.appearance(&person.kind, person.seed, &person.worn)?;
//! let face = buf.pictures_mut().get_or_insert_with(FACE, || Painted::rgba(32, 32));
//! parts::paint(&set, &look, |id| images.get(&id), face)?;
//! ```
//!
//! A game keeps one [`PaintedId`](crate::frame::PaintedId) for each
//! place a face shows, and paints it again only when the person changes:
//! each paint moves the picture's revision, which uploads it again.

use crate::prelude::*;

use shuyi_assets::image::{Image, ImageId};
use shuyi_assets::parts::{Appearance, DRAWN_ALPHA, PartSet};

use crate::frame::painted::{Painted, PaintedKind};

/// Why a look can't be painted ([`paint`]).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum PaintError {
    /// The picture to paint isn't RGBA.
    #[error("a part set paints an RGBA picture, not a {0} one")]
    NotRgba(&'static str),
    /// The picture isn't the set's size.
    #[error("the picture is {got:?} px; the part set's are {want:?}")]
    WrongSize {
        /// The set's size.
        want: (u32, u32),
        /// The picture's.
        got: (u32, u32),
    },
    /// The look names a part the set doesn't have, or has more slots.
    #[error("the look isn't of this part set: {0}")]
    NotOfTheSet(String),
    /// A part's image has no pixels, or too few.
    #[error("part \"{part}\": the pixels of {image} aren't there")]
    NoPixels {
        /// The part.
        part: String,
        /// Its image's path.
        image: &'static str,
    },
}

/// Paints `look`, a look of `set`, into `out`, an RGBA picture of the
/// set's size, `pixels` giving an image's pixels. It clears `out`, then
/// draws each slot's part in slot order: a pixel with alpha under 128 is
/// left out; a grey that is a key is drawn in the key's colour; any other
/// pixel keeps its own colour (outlines, eye whites). Every pixel drawn
/// is opaque. It borrows `out`'s pixels once, so the revision moves once,
/// and not at all when it fails.
pub fn paint<'a>(
    set: &PartSet,
    look: &Appearance,
    pixels: impl Fn(ImageId) -> Option<&'a Image>,
    out: &mut Painted,
) -> Result<(), PaintError> {
    if out.kind() != PaintedKind::Rgba {
        return Err(PaintError::NotRgba(out.kind().name()));
    }
    let (width, height) = out.size();
    let got = (u32::from(width), u32::from(height));
    if got != set.size {
        return Err(PaintError::WrongSize {
            want: set.size,
            got,
        });
    }
    if look.parts.len() > set.slots.len() {
        return Err(PaintError::NotOfTheSet(format!(
            "it has {} slots, the set {}",
            look.parts.len(),
            set.slots.len()
        )));
    }
    // Every part and its pixels first, so a failure paints nothing.
    let mut layers = Vec::new();
    for name in look.parts.iter().flatten() {
        let part = set
            .parts
            .get(name)
            .ok_or_else(|| PaintError::NotOfTheSet(format!("the set has no part \"{name}\"")))?;
        let image = pixels(part.image)
            .filter(|i| holds(i, part.at, got))
            .ok_or_else(|| PaintError::NoPixels {
                part: name.clone(),
                image: part.image.path(),
            })?;
        layers.push((image, part.at));
    }
    let mut colour: [Option<[u8; 3]>; 256] = [None; 256];
    for (&key, &rgb) in &look.colours {
        colour[usize::from(key)] = Some(rgb);
    }
    let width = usize::from(width);
    let rgba = out.pixels_mut();
    rgba.fill(0);
    for (image, (ax, ay)) in layers {
        let stride = image.width as usize * 4;
        let (ax, ay) = (ax as usize, ay as usize);
        for (y, row) in rgba.chunks_exact_mut(width * 4).enumerate() {
            let from = (ay + y) * stride + ax * 4;
            let source = &image.rgba[from..from + width * 4];
            for (dst, &[r, g, b, a]) in row
                .as_chunks_mut::<4>()
                .0
                .iter_mut()
                .zip(source.as_chunks::<4>().0)
            {
                if a < DRAWN_ALPHA {
                    continue;
                }
                let [r, g, b] = match colour[usize::from(r)] {
                    Some(rgb) if r == g && g == b => rgb,
                    _ => [r, g, b],
                };
                *dst = [r, g, b, u8::MAX];
            }
        }
    }
    Ok(())
}

/// Whether `image` has its bytes, and a `size` picture at `at`.
fn holds(image: &Image, (x, y): (u32, u32), (w, h): (u32, u32)) -> bool {
    let bytes = u64::from(image.width) * u64::from(image.height) * 4;
    u64::try_from(image.rgba.len()).is_ok_and(|n| n == bytes)
        && u64::from(x) + u64::from(w) <= u64::from(image.width)
        && u64::from(y) + u64::from(h) <= u64::from(image.height)
}

#[cfg(test)]
mod tests;
