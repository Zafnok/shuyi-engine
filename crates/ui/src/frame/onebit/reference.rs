//! 1032's renderer, as it was before ticket 1055 made [`render`](super::render)
//! fast: plain and slow (64-bit coordinates, a pattern byte worked out for
//! every byte written, every cell visited whatever the clip). Kept as the
//! reference the fast one must equal pixel for pixel: the tests compare
//! the two, and the Playdate's bench build (`cargo xtask playdate --bench`)
//! times both on the device. Never drawn by the game.

use shuyi_assets::font::FALLBACK_GLYPH;
use shuyi_assets::ink::Ink;

use super::{Font, Frame, HEIGHT, Image, Images, Phase, ROW_BYTES, WIDTH, ink_is_white, ink_of};
use crate::frame::glyph_buffer::{GlyphBuffer, Item, Layer, Paint, PxRect, Sprite};

/// Draws `buf` into `out` exactly as [`render`](super::render) does.
pub fn render(buf: &GlyphBuffer, font: &Font<'_>, images: &impl Images, out: &mut Frame) {
    render_marked(buf, font, images, out, &mut |_| {});
}

/// [`render`], calling `mark` as each [`Phase`] ends.
pub fn render_marked(
    buf: &GlyphBuffer,
    font: &Font<'_>,
    images: &impl Images,
    out: &mut Frame,
    mark: &mut impl FnMut(Phase),
) {
    out.fill(u8::MAX);
    mark(Phase::Ground);
    let mut target = Target {
        frame: out,
        clip: Clip::SCREEN,
    };
    let place = Place {
        x: 0,
        y: 0,
        scale: 1,
    };
    if let Some(backdrop) = buf.backdrop() {
        let window = place.screen(backdrop.clip_px());
        let (dx, dy) = backdrop.scene_offset(1);
        let mut scene = Target {
            frame: &mut *target.frame,
            clip: Clip::SCREEN.within(window),
        };
        let at = Place {
            x: window.x0 + dx,
            y: window.y0 + dy,
            scale: i64::from(backdrop.zoom()),
        };
        draw_layers(&mut scene, backdrop.scene(), at, font, images, &mut |_| {});
    }
    mark(Phase::Backdrop);
    draw_layers(&mut target, buf, place, font, images, mark);
}

/// The eight pixels of screen row `y` that `ink` paints in a byte.
fn ink_byte(ink: Ink, y: i32) -> u8 {
    (0..8).fold(0, |byte, x| (byte << 1) + u8::from(ink_is_white(ink, x, y)))
}

/// The index in a [`Frame`] of the byte holding screen pixel `(x, y)`.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // on the screen
const fn byte_index(x: i64, y: i64) -> usize {
    y as usize * ROW_BYTES + (x >> 3) as usize
}

/// The pixel rows of `glyph`, or of the fallback glyph, looked up in the
/// glyph map.
fn rows<'a>(font: &Font<'a>, glyph: char) -> Option<&'a [u8]> {
    let index = font
        .atlas
        .glyphs
        .get(&glyph)
        .or_else(|| font.atlas.glyphs.get(&FALLBACK_GLYPH))?;
    let rows = font.atlas.cell_h as usize;
    let start = *index as usize * rows;
    font.bits.get(start..start + rows)
}

/// A rectangle of screen pixels, `x0..x1` × `y0..y1`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct Clip {
    x0: i64,
    y0: i64,
    x1: i64,
    y1: i64,
}

impl Clip {
    const SCREEN: Self = Self {
        x0: 0,
        y0: 0,
        x1: WIDTH as i64,
        y1: HEIGHT as i64,
    };

    fn within(self, other: Self) -> Self {
        Self {
            x0: self.x0.max(other.x0),
            y0: self.y0.max(other.y0),
            x1: self.x1.min(other.x1),
            y1: self.y1.min(other.y1),
        }
    }
}

/// Where a buffer is drawn: its top-left pixel at screen `(x, y)`, each of
/// its pixels `scale × scale` screen pixels.
#[derive(Copy, Clone, Debug)]
struct Place {
    x: i64,
    y: i64,
    scale: i64,
}

impl Place {
    fn screen(self, r: PxRect) -> Clip {
        let x0 = self.x + i64::from(r.x) * self.scale;
        let y0 = self.y + i64::from(r.y) * self.scale;
        Clip {
            x0,
            y0,
            x1: x0 + i64::from(r.w) * self.scale,
            y1: y0 + i64::from(r.h) * self.scale,
        }
    }
}

struct Target<'f> {
    frame: &'f mut Frame,
    clip: Clip,
}

#[derive(Copy, Clone)]
enum Pen {
    Ink(Ink),
    Flip,
}

impl Target<'_> {
    #[allow(clippy::cast_possible_truncation)] // a byte's bits, and screen rows
    fn run(&mut self, y: i64, x: i64, bits: u64, pen: Pen) {
        let clip = self.clip;
        if bits == 0 || y < clip.y0 || y >= clip.y1 {
            return;
        }
        let (lo, hi) = (x.max(clip.x0), (x + 64).min(clip.x1));
        for start in ((lo & !7)..hi).step_by(8) {
            let wide = u128::from(bits) << 56 << (start - x + 8);
            let mut mask = (wide >> 120) as u8;
            mask &= u8::MAX >> (lo - start).max(0);
            mask &= u8::MAX << (start + 8 - hi).max(0);
            let byte = &mut self.frame[byte_index(start, y)];
            *byte = match pen {
                Pen::Ink(ink) => *byte ^ ((*byte ^ ink_byte(ink, y as i32)) & mask),
                Pen::Flip => *byte ^ mask,
            };
        }
    }

    fn fill(&mut self, rect: Clip, pen: Pen) {
        let rect = rect.within(self.clip);
        for y in rect.y0..rect.y1 {
            for x in (rect.x0..rect.x1).step_by(64) {
                let len = (rect.x1 - x).min(64);
                self.run(y, x, u64::MAX << (64 - len), pen);
            }
        }
    }

    fn set(&mut self, x: i64, y: i64, white: bool) {
        let byte = &mut self.frame[byte_index(x, y)];
        let bit = 0x80 >> (x & 7);
        if white {
            *byte |= bit;
        } else {
            *byte &= !bit;
        }
    }
}

fn draw_layers(
    target: &mut Target<'_>,
    buf: &GlyphBuffer,
    at: Place,
    font: &Font<'_>,
    images: &impl Images,
    mark: &mut impl FnMut(Phase),
) {
    let spec = buf.spec();
    let (cell_w, cell_h) = (i32::from(spec.cell_w), i32::from(spec.cell_h));
    let cells = || {
        (0..i32::from(buf.height())).flat_map(move |y| {
            (0..i32::from(buf.width())).filter_map(move |x| {
                let rect = PxRect::new(x * cell_w, y * cell_h, cell_w, cell_h);
                buf.get(x, y).map(|cell| (at.screen(rect), cell))
            })
        })
    };
    for (rect, cell) in cells() {
        if !cell.see_through {
            target.fill(rect, Pen::Ink(ink_of(cell.bg)));
        }
    }
    mark(Phase::Backgrounds);
    draw_items(target, buf, Layer::Under, at, images);
    mark(Phase::Under);
    for (rect, cell) in cells() {
        if ink_of(cell.fg) == Ink::BlackEdged {
            draw_glyph(target, font, cell.glyph, rect, at.scale, Glyph::Edge);
        }
    }
    mark(Phase::Edges);
    for (rect, cell) in cells() {
        draw_glyph(
            target,
            font,
            cell.glyph,
            rect,
            at.scale,
            Glyph::Ink(ink_of(cell.fg)),
        );
    }
    mark(Phase::Glyphs);
    draw_items(target, buf, Layer::Over, at, images);
    mark(Phase::Over);
    for item in buf.items() {
        if let Item::Rect(o) = item
            && ink_of(o.color) == Ink::Invert
        {
            target.fill(at.screen(o.rect), Pen::Flip);
        }
    }
    mark(Phase::Invert);
}

fn draw_items(
    target: &mut Target<'_>,
    buf: &GlyphBuffer,
    layer: Layer,
    at: Place,
    images: &impl Images,
) {
    for item in buf.items().iter().filter(|item| item.layer() == layer) {
        match item {
            Item::Rect(o) => {
                let ink = ink_of(o.color);
                if ink != Ink::Invert {
                    target.fill(at.screen(o.rect), Pen::Ink(ink));
                }
            }
            Item::Sprite(s) => draw_sprite(target, s, at, images),
        }
    }
}

#[derive(Copy, Clone)]
enum Glyph {
    Ink(Ink),
    Edge,
}

fn draw_glyph(
    target: &mut Target<'_>,
    font: &Font<'_>,
    glyph: char,
    cell: Clip,
    scale: i64,
    part: Glyph,
) {
    if glyph == ' ' {
        return;
    }
    let Some(rows) = rows(font, glyph) else {
        return;
    };
    let height = i64::try_from(rows.len()).unwrap_or(0) * scale;
    let row = |dy: i64| {
        let at = (0..height).contains(&dy).then(|| dy / scale);
        let byte = at.and_then(|at| rows.get(usize::try_from(at).ok()?));
        byte.map_or(0, |&byte| widen(byte, scale))
    };
    match part {
        Glyph::Ink(ink) => {
            for dy in 0..height {
                target.run(cell.y0 + dy, cell.x0, row(dy), Pen::Ink(ink));
            }
        }
        Glyph::Edge => {
            for dy in -1..=height {
                let near = (row(dy - 1) | row(dy) | row(dy + 1)) >> 1;
                let edge = near | near << 1 | near >> 1;
                target.run(cell.y0 + dy, cell.x0 - 1, edge, Pen::Ink(Ink::White));
            }
        }
    }
}

fn widen(byte: u8, scale: i64) -> u64 {
    let mut out = 0u64;
    for i in (0..8).rev() {
        for _ in 0..scale {
            out = (out << 1) + u64::from(byte >> i & 1);
        }
    }
    out << (64 - 8 * scale)
}

const BAYER: [[u32; 4]; 4] = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];

#[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)] // 0..4
fn shows(opacity: u8, x: i64, y: i64) -> bool {
    let rank = BAYER[(y & 3) as usize][(x & 3) as usize];
    u32::from(opacity) * 16 > rank * 255 + 127
}

fn draw_sprite(target: &mut Target<'_>, sprite: &Sprite, at: Place, images: &impl Images) {
    let shown = at.screen(sprite.clip).within(target.clip);
    let Some(image) = images.image(sprite.image) else {
        target.fill(shown, Pen::Ink(Ink::Grey));
        return;
    };
    let (src, dest) = (sprite.src, sprite.dest);
    let (src_w, src_h) = (i64::from(src.w), i64::from(src.h));
    let (dest_w, dest_h) = (i64::from(dest.w), i64::from(dest.h));
    let solid = match sprite.paint {
        Paint::Solid(color) => Some(ink_of(color)),
        Paint::Image | Paint::Dimmed | Paint::Shaded(_) => None,
    };
    for y in shown.y0..shown.y1 {
        let dy = (y - at.y).div_euclid(at.scale) - i64::from(dest.y);
        let v = Image::sample(
            dy,
            src.y.into(),
            src_h,
            dest_h,
            image.height,
            image.source_height,
        );
        for x in shown.x0..shown.x1 {
            let mut dx = (x - at.x).div_euclid(at.scale) - i64::from(dest.x);
            if sprite.flip_x {
                dx = dest_w - 1 - dx;
            }
            let u = Image::sample(
                dx,
                src.x.into(),
                src_w,
                dest_w,
                image.width,
                image.source_width,
            );
            let Some(white) = image.pixel(u, v) else {
                continue;
            };
            if !shows(sprite.opacity, x, y) {
                continue;
            }
            let white = white && shows(sprite.brightness, x, y);
            #[allow(clippy::cast_possible_truncation)] // screen pixels
            let white = match (solid, sprite.paint) {
                (Some(ink), _) => ink_is_white(ink, x as i32, y as i32),
                (None, Paint::Dimmed) => white || ink_is_white(Ink::Grey, x as i32, y as i32),
                (None, _) => white,
            };
            target.set(x, y, white);
        }
    }
}
