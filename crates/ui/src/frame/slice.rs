//! A [`NineSlice`] as sprites: [`nine_slice_sprites`] lays
//! its corners, edges and middle over a rectangle of console pixels, and
//! [`GlyphBuffer::nine_slice`] adds them to a buffer. They are plain
//! [`Sprite`]s, so every backend draws them as it draws any other: a cut
//! corner or a cut last tile is a whole-size `dest` with a smaller
//! `clip`, never a squeezed picture.

use crate::prelude::*;

use shuyi_assets::image::ImageRect;

pub use shuyi_assets::nine_slice::{Borders, CentreFill, EdgeFill, NineSlice};

use crate::frame::glyph_buffer::{GlyphBuffer, Layer, PxRect, Rect, Sprite};

/// `rect` of an image as a sprite's `src`, or `None` past `i32`.
fn src_of(rect: ImageRect) -> Option<PxRect> {
    let side = |v: u32| i32::try_from(v).ok();
    Some(Rect::new(
        side(rect.x)?,
        side(rect.y)?,
        side(rect.w)?,
        side(rect.h)?,
    ))
}

/// A rectangle from `i64` sides, or `None` if one doesn't fit in `i32`.
fn rect64(x: i64, y: i64, w: i64, h: i64) -> Option<PxRect> {
    let fit = |v: i64| i32::try_from(v).ok();
    Some(Rect::new(fit(x)?, fit(y)?, fit(w)?, fit(h)?))
}

/// How many of a `len`-long side two corners of `low` and `high` keep:
/// both whole when they fit; otherwise the low one keeps
/// `min(low, max(len - high, (len + 1) / 2))` and the high one the rest,
/// each cut on its inner side.
fn kept(len: i64, low: i64, high: i64) -> (i64, i64) {
    if len >= low + high {
        (low, high)
    } else {
        let l = low.min((len - high).max((len + 1) / 2)).max(0);
        (l, len - l)
    }
}

/// The sprites that fill `area` with `src`: one stretched over it, or,
/// with `tile` (a tile's size in console pixels), copies from its
/// top-left, the last row and column cut. Every clip is also cut to
/// `within`; a sprite that shows nothing is left out.
fn fill(
    out: &mut Vec<Sprite>,
    base: Sprite,
    area: (i64, i64, i64, i64),
    tile: Option<(i64, i64)>,
    within: PxRect,
) {
    let (x, y, w, h) = area;
    let Some(area_px) = rect64(x, y, w, h) else {
        return;
    };
    let Some(seen) = area_px.intersect(&within) else {
        return;
    };
    // A tile is a part of the picture that isn't empty, times the zoom: never
    // 0 wide or tall.
    let Some((tw, th)) = tile else {
        out.push(Sprite {
            dest: area_px,
            clip: seen,
            ..base
        });
        return;
    };
    // Only the tiles that touch what `within` shows of the area.
    let first = |from: i64, to: i64, step: i64| (to - from).max(0) / step;
    let (c0, r0) = (
        first(x, i64::from(seen.x), tw),
        first(y, i64::from(seen.y), th),
    );
    let (sx1, sy1) = (
        i64::from(seen.x) + i64::from(seen.w),
        i64::from(seen.y) + i64::from(seen.h),
    );
    let (Ok(down), Ok(across)) = (usize::try_from(th), usize::try_from(tw)) else {
        return;
    };
    let skipped = |n: i64| usize::try_from(n).unwrap_or(usize::MAX);
    // From the area's corner, skipping the tiles before what `within` shows.
    for ty in (y..sy1).step_by(down).skip(skipped(r0)) {
        for tx in (x..sx1).step_by(across).skip(skipped(c0)) {
            let clip = rect64(tx, ty, tw, th).and_then(|d| d.intersect(&seen).map(|c| (d, c)));
            if let Some((dest, clip)) = clip {
                out.push(Sprite { dest, clip, ..base });
            }
        }
    }
}

/// The sprites of `slice` laid over the console pixels `dest`, in
/// `layer`, each clip cut to `within`; a part wholly outside it is not
/// made, so a tiled middle over a huge `dest` makes only the tiles
/// `within` shows. None for an empty `dest` or `within`.
///
/// The rule, in console pixels with `z` the slice's
/// [`scale`](NineSlice::scale) and its borders [`fitted`](Borders::fitted)
/// to its picture:
///
/// - each corner is its borders times `z` (the top-left is `left × z` by
///   `top × z`), anchored at its corner of `dest`;
/// - when `dest` is narrower than the left and right corners together,
///   they are cut on their inner side: the left keeps
///   `min(left × z, max(dest.w − right × z, (dest.w + 1) / 2))` columns
///   and the right the rest; the top and bottom likewise, the top taking
///   the odd row. A corner keeps its full `dest`: only its `clip` shrinks;
/// - the edges fill what lies between the corners, as thick as their
///   border times `z`: one stretched sprite ([`EdgeFill::Stretch`]) or
///   copies from the corner's inner side ([`EdgeFill::Tile`]);
/// - the middle fills what is left: stretched, tiled from its top-left,
///   or not at all ([`CentreFill::Empty`]);
/// - a part whose picture is 0 wide or tall is not drawn.
///
/// ```
/// use shuyi_assets::image::{ImageInfo, ImageRect, ImageTable, Picture};
/// use shuyi_ui::frame::slice::{nine_slice_sprites, Borders, NineSlice};
/// use shuyi_ui::frame::{Layer, Rect};
/// let table = ImageTable::new([("frame.png", ImageInfo::new(9, 9))].into());
/// let rect = ImageRect { x: 0, y: 0, w: 9, h: 9 };
/// let picture = Picture { image: table.id("frame.png").unwrap(), rect };
/// let slice = NineSlice::new(picture, Borders::uniform(3));
/// let area = Rect::new(0, 0, 40, 20);
/// let parts = nine_slice_sprites(&slice, area, area, Layer::Under);
/// assert_eq!(parts.len(), 9); // four corners, four edges, the middle
/// assert_eq!(parts[0].dest, Rect::new(0, 0, 3, 3)); // the top-left corner
/// ```
pub fn nine_slice_sprites(
    slice: &NineSlice,
    dest: PxRect,
    within: PxRect,
    layer: Layer,
) -> Vec<Sprite> {
    let mut out = Vec::new();
    let Some(pic) = src_of(slice.picture.rect) else {
        return out;
    };
    let fit = slice
        .borders
        .fitted(slice.picture.rect.w, slice.picture.rect.h);
    let zoom = i64::from(slice.scale());
    let across = Axis {
        src: (pic.x.into(), pic.w.into()),
        borders: (fit.left.into(), fit.right.into()),
        dest: (dest.x.into(), dest.w.into()),
    };
    let down = Axis {
        src: (pic.y.into(), pic.h.into()),
        borders: (fit.top.into(), fit.bottom.into()),
        dest: (dest.y.into(), dest.h.into()),
    };
    let (columns, rows) = (across.bands(zoom), down.bands(zoom));
    for (j, row) in rows.iter().enumerate() {
        for (i, column) in columns.iter().enumerate() {
            let tile = (column.src.1 * zoom, row.src.1 * zoom);
            let tile = match usize::from(i == 1) + usize::from(j == 1) {
                0 => None,
                1 => matches!(slice.edges, EdgeFill::Tile).then_some(tile),
                _ => match slice.centre {
                    CentreFill::Stretch => None,
                    CentreFill::Tile => Some(tile),
                    _ => continue,
                },
            };
            let src = rect64(column.src.0, row.src.0, column.src.1, row.src.1);
            let keep = rect64(column.keep.0, row.keep.0, column.keep.1, row.keep.1)
                .and_then(|keep| keep.intersect(&within));
            let (Some(src), Some(keep)) = (src.filter(|s| !s.is_empty()), keep) else {
                continue;
            };
            let base = Sprite::new(slice.picture.image, src, Rect::new(0, 0, 0, 0), layer);
            let area = (column.dest.0, row.dest.0, column.dest.1, row.dest.1);
            fill(&mut out, base, area, tile, keep);
        }
    }
    out
}

/// One side of a nine-slice: its picture's start and length, its two
/// borders (low and high, in picture pixels), and its `dest`'s start and
/// length, in console pixels.
struct Axis {
    src: (i64, i64),
    borders: (i64, i64),
    dest: (i64, i64),
}

/// A third of an [`Axis`]: the part of the picture (start, length), where
/// it is laid (start, length) and the part of that shown.
struct Band {
    src: (i64, i64),
    dest: (i64, i64),
    keep: (i64, i64),
}

impl Axis {
    /// The low corner, the middle and the high corner, at `zoom`.
    fn bands(&self, zoom: i64) -> [Band; 3] {
        let (from, len) = self.src;
        let (low, high) = self.borders;
        let (at, room) = self.dest;
        let (low_px, high_px) = (low * zoom, high * zoom);
        let (low_kept, high_kept) = kept(room, low_px, high_px);
        let middle = (at + low_kept, room - low_kept - high_kept);
        [
            Band {
                src: (from, low),
                dest: (at, low_px),
                keep: (at, low_kept),
            },
            Band {
                src: (from + low, len - low - high),
                dest: middle,
                keep: middle,
            },
            Band {
                src: (from + len - high, high),
                dest: (at + room - high_px, high_px),
                keep: (at + room - high_kept, high_kept),
            },
        ]
    }
}

impl GlyphBuffer {
    /// Adds the sprites of `slice` laid over the console pixels `dest`, in
    /// `layer` ([`nine_slice_sprites`], cut to the buffer).
    pub fn nine_slice(&mut self, slice: &NineSlice, dest: PxRect, layer: Layer) {
        for sprite in nine_slice_sprites(slice, dest, self.pixel_bounds(), layer) {
            self.add_sprite(sprite);
        }
    }
}

#[cfg(test)]
mod tests;
