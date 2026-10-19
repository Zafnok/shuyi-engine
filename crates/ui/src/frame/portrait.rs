//! Drawing a picture in an area of cells (game ADR-0043): one sprite item, the
//! image at the largest whole scale that fits, centred. A game's
//! portraits are drawn this way.

#[cfg(not(feature = "std"))]
use crate::float::Float;
use shuyi_assets::image::{ImageId, ImageInfo};

use crate::frame::glyph_buffer::{GlyphBuffer, Layer, PxRect, Rect, Sprite};

/// Where an image of `size` is drawn in `frame` (console pixels): at the
/// largest whole scale that fits, centred (an odd pixel left over goes to
/// the right or the bottom). `None` if it doesn't fit even at 1×, or has no
/// pixels.
pub fn fit_whole_scale(size: ImageInfo, frame: PxRect) -> Option<PxRect> {
    let (w, h) = (i64::from(size.width), i64::from(size.height));
    if w == 0 || h == 0 {
        return None;
    }
    let scale = (i64::from(frame.w) / w).min(i64::from(frame.h) / h);
    if scale < 1 {
        return None;
    }
    // Both are at most the frame's side, so they fit in i32.
    let (w, h) = (
        i32::try_from(w * scale).ok()?,
        i32::try_from(h * scale).ok()?,
    );
    Some(Rect::new(
        frame.x + (frame.w - w) / 2,
        frame.y + (frame.h - h) / 2,
        w,
        h,
    ))
}

/// The opacity of a picture dimmed by `dim` (0 = full color, 1 = gone):
/// the renderer fades every pixel toward what is behind it (game ADR-0018).
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // clamped to 0..=255
pub fn dim_opacity(dim: f32) -> u8 {
    (255.0 * (1.0 - dim)).round().clamp(0.0, 255.0) as u8
}

/// Draws `image` (of `size` pixels) in the area of `(w, h)` cells whose
/// top-left cell is `(x, y)`, at the largest whole scale that fits it,
/// centred, as one sprite under the glyphs: a transparent pixel shows the
/// cells' background, so clear the area first. `dim` (0 = full color,
/// 1 = gone) fades it toward that background; `mirror` flips it left to
/// right. Clipped to the buffer. Draws nothing if the picture is too big
/// for the area even at 1×.
pub fn draw_fitted(
    buf: &mut GlyphBuffer,
    (x, y): (i32, i32),
    (w, h): (i32, i32),
    (image, size): (ImageId, ImageInfo),
    dim: f32,
    mirror: bool,
) {
    let (cw, ch) = buf.spec().cell_px();
    let frame = Rect::new(
        x.saturating_mul(cw),
        y.saturating_mul(ch),
        w.saturating_mul(cw),
        h.saturating_mul(ch),
    );
    let src = (i32::try_from(size.width), i32::try_from(size.height));
    if let (Some(dest), (Ok(w), Ok(h))) = (fit_whole_scale(size, frame), src) {
        buf.add_sprite(Sprite {
            flip_x: mirror,
            opacity: dim_opacity(dim),
            ..Sprite::new(image, Rect::new(0, 0, w, h), dest, Layer::Under)
        });
    }
}

#[cfg(test)]
mod tests;
