//! The raster: a frame painted in software, the reference picture of a
//! [`GlyphBuffer`] that every backend must match. `shuyi-app`'s GPU
//! renderer is held to it (ticket 2018), and [`compare`] is the one
//! comparison every backend's pixels go through. `frame-png`, the tool
//! contract and a game's PNG snapshots paint with it; a game may too, at
//! run time (a save's thumbnail, a photo mode). It takes decoded pictures:
//! decoding a file is the caller's (`shuyi-cli` decodes PNG).
//!
//! [`Painter`] is a software copy of `shuyi-app`'s renderer, pixel for
//! pixel at whole scales: the clear colour, cell backgrounds, `Under`
//! items in order, glyphs tinted with their fg, `Over` items in order;
//! and before all that, a backdrop's scene in its window. Textures are
//! sampled at the nearest pixel and blended over what is beneath with
//! their alpha, as the GPU does. [`one_bit_picture`] paints a one-bit
//! frame black and white, and [`test_card`] is a generated test image for
//! sprite items. [`card`] is the render card: one frame with every kind of
//! thing a frame draws, which a backend draws to hold its pixels to the
//! raster's.
//!
//! Behind the `raster` feature (the tests have it anyway): the default
//! engine and a console's build don't need it.

use alloc::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use shuyi_assets::font::{AtlasRect, FALLBACK_GLYPH, FontAtlasDef};
use shuyi_assets::image::ImageId;

use crate::prelude::*;

use crate::frame::color::Rgb;
use crate::frame::glyph_buffer::{Backdrop, GlyphBuffer, Item, Layer, Paint, PxRect, Sprite};
use crate::frame::onebit::{self, Frame};

pub mod card;
pub mod test_card;

/// The colour of what is missing, as in `shuyi-app` (macroquad's
/// `MAGENTA`): an image the painter lacks, a glyph the atlas lacks, and a
/// pixel over the tolerance in a [`Comparison`]'s `diff`.
pub const MISSING_COLOR: Rgb = Rgb::new(255, 0, 255);

/// `frame` as a black and white picture, each pixel `scale` × `scale`.
pub fn one_bit_picture(frame: &Frame, scale: u32) -> Image {
    let (width, height) = (onebit::WIDTH.unsigned_abs(), onebit::HEIGHT.unsigned_abs());
    let mut image = Image::filled(width * scale, height * scale, Rgb::new(0, 0, 0));
    let s = i64::from(scale);
    let at = |v: i64| i32::try_from(v / s).ok();
    for y in 0..i64::from(image.height) {
        for x in 0..i64::from(image.width) {
            if at(x)
                .zip(at(y))
                .is_some_and(|(x, y)| onebit::is_white(frame, x, y))
            {
                image.blend(x, y, [1.0; 3], 1.0);
            }
        }
    }
    image
}

/// An RGBA8 picture, row-major.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Four bytes per pixel.
    pub rgba: Vec<u8>,
}

impl Image {
    /// A `width` × `height` picture of `color`, solid.
    pub fn filled(width: u32, height: u32, color: Rgb) -> Self {
        let mut image = Self {
            width,
            height,
            rgba: vec![0; width as usize * height as usize * 4],
        };
        image.fill((0, 0, i64::from(width), i64::from(height)), color);
        image
    }

    /// The pixel at `(x, y)`, clamped to the picture (an empty picture is
    /// transparent).
    fn texel(&self, x: i64, y: i64) -> [u8; 4] {
        if self.width == 0 || self.height == 0 {
            return [0; 4];
        }
        let clamp =
            |v: i64, size: u32| usize::try_from(v.clamp(0, i64::from(size) - 1)).unwrap_or(0);
        let i = (clamp(y, self.height) * self.width as usize + clamp(x, self.width)) * 4;
        [
            self.rgba[i],
            self.rgba[i + 1],
            self.rgba[i + 2],
            self.rgba[i + 3],
        ]
    }

    /// The pixel at `(x, y)`, or `None` outside the picture.
    pub fn pixel(&self, x: u32, y: u32) -> Option<[u8; 4]> {
        (x < self.width && y < self.height).then(|| self.texel(i64::from(x), i64::from(y)))
    }

    /// Blends `color` with alpha `alpha` (0 to 1) over the pixel at
    /// `(x, y)`, as the GPU does: in floats, rounded once. Off the picture
    /// it does nothing.
    fn blend(&mut self, x: i64, y: i64, color: [f32; 3], alpha: f32) {
        let (Ok(x), Ok(y)) = (u32::try_from(x), u32::try_from(y)) else {
            return;
        };
        if x >= self.width || y >= self.height {
            return;
        }
        let i = (y as usize * self.width as usize + x as usize) * 4;
        for (c, &src) in color.iter().enumerate() {
            let dst = f32::from(self.rgba[i + c]) / 255.0;
            let out = src * alpha + dst * (1.0 - alpha);
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // 0..=255
            let byte = (out * 255.0).round().clamp(0.0, 255.0) as u8;
            self.rgba[i + c] = byte;
        }
        self.rgba[i + 3] = 255;
    }

    /// Fills `(x, y, w, h)` with `color`, solid.
    fn fill(&mut self, (x, y, w, h): (i64, i64, i64, i64), color: Rgb) {
        for py in y..y + h {
            for px in x..x + w {
                self.blend(px, py, unit(color), 1.0);
            }
        }
    }
}

/// `c` as floats from 0 to 1.
fn unit(c: Rgb) -> [f32; 3] {
    [c.r, c.g, c.b].map(|v| f32::from(v) / 255.0)
}

/// The font atlas and images, decoded, ready to render any number of
/// frames.
pub struct Painter {
    atlas: FontAtlasDef,
    atlas_image: Image,
    /// Background drawn behind the console; cells with this bg skip their
    /// fill, as in `app`.
    clear: Rgb,
    /// The images there are; the others draw as `MISSING_COLOR`.
    images: BTreeMap<ImageId, Image>,
}

impl Painter {
    /// A painter of font atlas `atlas`, whose picture is `atlas_image`,
    /// with `images` (an image not among them draws as a `MISSING_COLOR`
    /// rectangle). `clear` is the console background (the palette's
    /// `black`).
    pub fn new(
        atlas: FontAtlasDef,
        atlas_image: Image,
        images: BTreeMap<ImageId, Image>,
        clear: Rgb,
    ) -> Self {
        Self {
            atlas,
            atlas_image,
            clear,
            images,
        }
    }

    /// Draws `buf` with each console pixel `scale` × `scale` pixels big.
    /// With a backdrop (game ADR-0048): the clear colour, the scene inside its
    /// window, then the console over it, as in `app`.
    pub fn render(&self, buf: &GlyphBuffer, scale: u32) -> Image {
        let s = i64::from(scale);
        let spec = buf.spec();
        let (cell_w, cell_h) = (i64::from(spec.cell_w) * s, i64::from(spec.cell_h) * s);
        let width = u32::try_from(i64::from(buf.width()) * cell_w).unwrap_or(0);
        let height = u32::try_from(i64::from(buf.height()) * cell_h).unwrap_or(0);
        let mut out = Image::filled(width, height, self.clear);
        if let Some(backdrop) = buf.backdrop() {
            self.draw_backdrop(&mut out, backdrop, scale);
        }
        // Over a scene, a cell in the clear colour must hide it.
        let all_fills = buf.backdrop().is_some();
        self.draw_layers(&mut out, buf, (0, 0), s, all_fills);
        out
    }

    /// Draws `backdrop`'s scene in its window of `out`: every scene pixel
    /// `zoom × scale` pixels big, placed by [`Backdrop::scene_offset`].
    /// The scene is drawn into a picture the size of the window, which
    /// clips it, and that is copied in.
    fn draw_backdrop(&self, out: &mut Image, backdrop: &Backdrop, scale: u32) {
        let s = i64::from(scale);
        let clip = backdrop.clip_px();
        let size = |v: i32| u32::try_from(i64::from(v) * s).unwrap_or(0);
        let mut window = Image::filled(size(clip.w), size(clip.h), self.clear);
        let scene_px = i64::from(backdrop.zoom()) * s;
        self.draw_layers(
            &mut window,
            backdrop.scene(),
            backdrop.scene_offset(scale),
            scene_px,
            false,
        );
        let (left, top) = (i64::from(clip.x) * s, i64::from(clip.y) * s);
        for y in 0..i64::from(window.height) {
            for x in 0..i64::from(window.width) {
                let [r, g, b, _] = window.texel(x, y);
                out.blend(left + x, top + y, unit(Rgb::new(r, g, b)), 1.0);
            }
        }
    }

    /// Draws `buf`'s cells and items into `out` with its top-left corner
    /// at `corner` and each of its pixels `px` × `px` big: cell
    /// backgrounds (all of them if `all_fills`, else all but those in the
    /// clear colour; never a see-through cell's), `Under` items, glyphs,
    /// `Over` items.
    fn draw_layers(
        &self,
        out: &mut Image,
        buf: &GlyphBuffer,
        corner: (i64, i64),
        px: i64,
        all_fills: bool,
    ) {
        let spec = buf.spec();
        let (cell_w, cell_h) = (i64::from(spec.cell_w) * px, i64::from(spec.cell_h) * px);
        let cells = || {
            (0..i32::from(buf.height())).flat_map(move |y| {
                (0..i32::from(buf.width())).filter_map(move |x| {
                    let at = (
                        corner.0 + i64::from(x) * cell_w,
                        corner.1 + i64::from(y) * cell_h,
                    );
                    buf.get(x, y).map(|cell| (at, cell))
                })
            })
        };
        for ((x, y), cell) in cells() {
            if !cell.see_through && (all_fills || cell.bg != self.clear) {
                out.fill((x, y, cell_w, cell_h), cell.bg);
            }
        }
        self.draw_items(out, buf, Layer::Under, corner, px);
        for ((x, y), cell) in cells() {
            if cell.glyph == ' ' {
                continue;
            }
            let Some((rect, fg)) = self.atlas_cell(cell.glyph, cell.fg) else {
                continue;
            };
            let fg = unit(fg);
            #[allow(clippy::cast_precision_loss)] // atlas coordinates are small
            let src = [rect.x, rect.y, rect.w, rect.h].map(|v| v as f32);
            blit(
                out,
                &self.atlas_image,
                src,
                (x, y, cell_w, cell_h),
                false,
                Paint::Image,
                [fg[0], fg[1], fg[2], 1.0],
            );
        }
        self.draw_items(out, buf, Layer::Over, corner, px);
    }

    /// Draws `buf`'s items of `layer`, in order, from `corner`, each of
    /// the buffer's pixels `scale` big.
    fn draw_items(
        &self,
        out: &mut Image,
        buf: &GlyphBuffer,
        layer: Layer,
        corner: (i64, i64),
        scale: i64,
    ) {
        for item in buf.items().iter().filter(|item| item.layer() == layer) {
            let to_px = |r: PxRect| {
                (
                    corner.0 + i64::from(r.x) * scale,
                    corner.1 + i64::from(r.y) * scale,
                    i64::from(r.w) * scale,
                    i64::from(r.h) * scale,
                )
            };
            match item {
                Item::Rect(rect) => out.fill(to_px(rect.rect), rect.color),
                Item::Sprite(sprite) => self.draw_sprite(out, sprite, to_px(sprite.clip)),
            }
        }
    }

    /// Draws the visible part of `sprite` into `dest`: the matching part
    /// of its image, or a `MISSING_COLOR` rectangle.
    fn draw_sprite(&self, out: &mut Image, sprite: &Sprite, dest: (i64, i64, i64, i64)) {
        let Some(image) = self.images.get(&sprite.image) else {
            out.fill(dest, MISSING_COLOR);
            return;
        };
        let opacity = f32::from(sprite.opacity) / 255.0;
        // Darker as the sprite's brightness has it (game ADR-0075).
        let bright = f32::from(sprite.brightness) / 255.0;
        let src = sprite.clipped_src();
        blit(
            out,
            image,
            src,
            dest,
            sprite.flip_x,
            sprite.paint,
            [bright, bright, bright, opacity],
        );
    }

    /// Where to find `glyph` in the atlas and what colour to tint it:
    /// `fg`, or the fallback glyph in `MISSING_COLOR`.
    fn atlas_cell(&self, glyph: char, fg: Rgb) -> Option<(AtlasRect, Rgb)> {
        if let Some(rect) = self.atlas.glyph_rect(glyph) {
            return Some((rect, fg));
        }
        Some((self.atlas.glyph_rect(FALLBACK_GLYPH)?, MISSING_COLOR))
    }
}

/// Stretches the `src` part of `image` (`[x, y, w, h]` in image pixels)
/// over `dest` in `out`, mirrored if `flip_x`, each texel coloured as
/// `paint`, multiplied by `tint` (RGBA, 0 to 1) and blended with the
/// result's alpha. Each pixel takes the texel under its centre, as nearest
/// sampling does.
#[allow(clippy::cast_precision_loss)] // pixel counts are small
fn blit(
    out: &mut Image,
    image: &Image,
    src: [f32; 4],
    (left, top, width, height): (i64, i64, i64, i64),
    flip_x: bool,
    paint: Paint,
    tint: [f32; 4],
) {
    let [src_x, src_y, src_w, src_h] = src;
    for dy in 0..height {
        let tex_y = src_y + (dy as f32 + 0.5) / height as f32 * src_h;
        for dx in 0..width {
            let across = (dx as f32 + 0.5) / width as f32;
            let tex_x = if flip_x {
                src_x + src_w * (1.0 - across)
            } else {
                src_x + src_w * across
            };
            #[allow(clippy::cast_possible_truncation)] // floored image coordinates
            let texel = image.texel(tex_x.floor() as i64, tex_y.floor() as i64);
            let [red, green, blue] = unit(paint.apply(Rgb::new(texel[0], texel[1], texel[2])));
            let color = [red * tint[0], green * tint[1], blue * tint[2]];
            let alpha = f32::from(texel[3]) / 255.0;
            out.blend(left + dx, top + dy, color, alpha * tint[3]);
        }
    }
}

/// How far a backend's picture may be from the reference and still pass
/// [`compare`]. Read from a plan file as `(channel: 4)`; a key left out
/// takes its default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
// Built with `Tolerance::new` or `default()`: it may grow (ADR-0039).
#[non_exhaustive]
pub struct Tolerance {
    /// The most a colour channel of a pixel may differ (0 to 255) before
    /// the pixel counts as different.
    pub channel: u8,
    /// How many pixels may differ.
    pub pixels: u32,
}

impl Tolerance {
    /// Up to `channel` in each colour channel, and up to `pixels` pixels
    /// past that.
    pub const fn new(channel: u8, pixels: u32) -> Self {
        Self { channel, pixels }
    }
}

impl Default for Tolerance {
    /// 2 in each channel (a GPU's rounding), and no pixel past that.
    fn default() -> Self {
        Self::new(2, 0)
    }
}

/// What [`compare`] found. Over pictures of different sizes, every field
/// but `same_size` covers the top-left area both share.
#[derive(Debug, Clone, PartialEq, Eq)]
// Made by `compare` alone: it may grow (ADR-0039).
#[non_exhaustive]
pub struct Comparison {
    /// Whether the two pictures are the same size.
    pub same_size: bool,
    /// How many pixels differ by more than `tolerance.channel` in some
    /// colour channel (alpha is ignored: a window has none).
    pub over: u32,
    /// The largest difference in any colour channel of any pixel.
    pub worst: u8,
    /// The first pixel counted in `over`, in row order.
    pub first: Option<(u32, u32)>,
    /// The reference with every colour channel halved, and each pixel
    /// counted in `over` in [`MISSING_COLOR`].
    pub diff: Image,
    /// The tolerance compared with.
    pub tolerance: Tolerance,
}

impl Comparison {
    /// Whether the pictures match: the same size, and no more pixels over
    /// the tolerance than it allows.
    pub fn passes(&self) -> bool {
        self.same_size && self.over <= self.tolerance.pixels
    }
}

/// Compares a backend's picture, `window`, with the `reference` the
/// raster painted, pixel by pixel, within `tolerance`.
pub fn compare(window: &Image, reference: &Image, tolerance: Tolerance) -> Comparison {
    let width = window.width.min(reference.width);
    let height = window.height.min(reference.height);
    let mut diff = Image {
        width,
        height,
        rgba: Vec::with_capacity(width as usize * height as usize * 4),
    };
    let (mut over, mut worst, mut first) = (0, 0, None);
    for y in 0..height {
        for x in 0..width {
            let (Some(seen), Some(want)) = (window.pixel(x, y), reference.pixel(x, y)) else {
                continue;
            };
            let most = (0..3).map(|c| seen[c].abs_diff(want[c])).max().unwrap_or(0);
            worst = worst.max(most);
            if most > tolerance.channel {
                over += 1;
                first = first.or(Some((x, y)));
                let Rgb { r, g, b } = MISSING_COLOR;
                diff.rgba.extend([r, g, b, 255]);
            } else {
                diff.rgba
                    .extend([want[0] / 2, want[1] / 2, want[2] / 2, 255]);
            }
        }
    }
    Comparison {
        same_size: (window.width, window.height) == (reference.width, reference.height),
        over,
        worst,
        first,
        diff,
        tolerance,
    }
}

#[cfg(test)]
mod tests;
