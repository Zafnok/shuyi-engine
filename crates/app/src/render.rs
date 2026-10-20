//! Blits a [`GlyphBuffer`] to the window with the font atlas (game ADR-0003):
//! integer-scaled, centred, black letterbox. The buffer's items are drawn
//! scaled, in the order they were added: `Under` ones after the cell
//! backgrounds, `Over` ones after the glyphs. A rectangle (game ADR-0018) is a
//! solid fill; a sprite (game ADR-0038) is part of an image's texture, or of a
//! recoloured copy of it when the sprite isn't painted in the image's own
//! colours (game ADR-0049). A buffer's backdrop (game ADR-0048), a second buffer
//! panned and zoomed, is drawn the same way inside its window, behind the
//! console.

use std::collections::{HashMap, HashSet};

use macroquad::prelude::*;
use shuyi_assets::font::{AtlasRect, FALLBACK_GLYPH, FontAtlasDef};
use shuyi_assets::image::{ImageId, ImageTable};
use shuyi_assets::lighting::Light;
use shuyi_ui::frame::Rgb;
use shuyi_ui::frame::console::{Layout, layout};
use shuyi_ui::frame::glyph_buffer::{Backdrop, GlyphBuffer, Item, Layer, Paint, Sprite};

/// The colour of what is missing: the fallback glyph drawn for a glyph the
/// atlas lacks, and the rectangle drawn for a sprite whose image has no
/// texture.
const MISSING_COLOR: Color = MAGENTA;

/// A recoloured copy of an image, for a sprite that isn't painted in the
/// image's own colours.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Recolour {
    /// Every pixel white, as see-through as it was: drawn tinted, it is
    /// the picture's silhouette in one colour ([`Paint::Solid`]).
    Silhouette,
    /// Every pixel grey and darker ([`Paint::Dimmed`]).
    Dimmed,
    /// Every pixel as it is in a light ([`Paint::Shaded`]).
    Shaded(Light),
}

/// Which solid cells get their background drawn.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Fills {
    /// All but those in the clear colour: it is already there.
    NotClear,
    /// Every one: something else may be behind them.
    All,
}

/// Draws glyph buffers with a font atlas texture.
pub struct Renderer {
    texture: Texture2D,
    atlas: FontAtlasDef,
    /// Background drawn behind the console; cells with this bg skip their fill.
    clear: Rgb,
    /// Missing glyphs already logged, so each is reported once.
    warned: HashSet<char>,
    /// A texture per image of the image table.
    images: HashMap<ImageId, Texture2D>,
    /// The recoloured copies made so far: each is made the first time a
    /// sprite needs it. `None` for an image that can't be decoded.
    recoloured: HashMap<(ImageId, Recolour), Option<Texture2D>>,
    /// Images without a texture already logged, so each is reported once.
    warned_images: HashSet<ImageId>,
    /// The game's embedded files by path, for the recoloured copies.
    bytes: fn(&str) -> Option<&'static [u8]>,
}

impl Renderer {
    /// Uploads the atlas image (`png`) and every image in `images` (read
    /// with `bytes`, the game's embedded files by path). `clear` is the
    /// console background (the palette's `black`). An image that can't be
    /// decoded is logged and left without a texture.
    pub fn new(
        atlas: FontAtlasDef,
        png: &[u8],
        clear: Rgb,
        images: &ImageTable,
        bytes: fn(&str) -> Option<&'static [u8]>,
    ) -> Result<Self, String> {
        let texture = upload(png).map_err(|e| format!("font atlas image: {e}"))?;
        let mut textures = HashMap::new();
        for id in images.ids() {
            match upload(bytes(id.path()).unwrap_or_default()) {
                Ok(texture) => {
                    textures.insert(id, texture);
                }
                Err(e) => warn!("image {}: {}", id.path(), e),
            }
        }
        Ok(Self {
            texture,
            atlas,
            clear,
            warned: HashSet::new(),
            images: textures,
            recoloured: HashMap::new(),
            warned_images: HashSet::new(),
            bytes,
        })
    }

    /// Sets the console background (the palette's `black`), which a
    /// colour theme changes.
    pub fn set_clear(&mut self, clear: Rgb) {
        self.clear = clear;
    }

    /// Draws `buf` scaled to the current window size.
    ///
    /// The layout is computed in physical framebuffer pixels (the window is
    /// `high_dpi`), so the integer scale holds on scaled Windows displays;
    /// coordinates are divided by the DPI factor only because macroquad's
    /// default camera works in logical units.
    ///
    /// With a backdrop (game ADR-0048) the order is: the clear colour, the
    /// scene inside its window, then the console over it, whose
    /// see-through cells have no background.
    pub fn draw(&mut self, buf: &GlyphBuffer) {
        let dpi = screen_dpi_scale();
        // Rounded: logical size × DPI can land a hair under the real pixel
        // count (913.714 × 1.75 = 1599.99…), which would drop a whole scale step.
        let window_h = (screen_height() * dpi).round();
        let fit = layout(buf.spec(), (screen_width() * dpi).round(), window_h);
        #[allow(clippy::cast_precision_loss)] // scale is small
        let scale = fit.scale as f32 / dpi;
        let (offset_x, offset_y) = (fit.offset_x / dpi, fit.offset_y / dpi);
        clear_background(BLACK);
        draw_rectangle(
            offset_x,
            offset_y,
            f32::from(buf.width()) * f32::from(buf.spec().cell_w) * scale,
            f32::from(buf.height()) * f32::from(buf.spec().cell_h) * scale,
            color(self.clear),
        );
        if let Some(backdrop) = buf.backdrop() {
            self.draw_backdrop(backdrop, &fit, window_h);
        }
        // Over a scene, a cell in the clear colour must hide it.
        let fills = if buf.backdrop().is_some() {
            Fills::All
        } else {
            Fills::NotClear
        };
        self.draw_layers(buf, (offset_x, offset_y), scale, fills);
    }

    /// Draws `backdrop`'s scene in its window: every scene pixel
    /// `zoom × fit.scale` physical pixels big, placed on whole physical
    /// pixels ([`Backdrop::scene_offset`]).
    ///
    /// The window clips it with a camera whose view is exactly the window,
    /// one unit per physical pixel: what falls outside a camera's view
    /// isn't drawn. (macroquad's scissor is only reachable through an
    /// `unsafe` call, which this workspace forbids.) `window_h` is the
    /// window's height in physical pixels: a viewport counts rows from the
    /// bottom.
    #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)] // whole pixels
    fn draw_backdrop(&mut self, backdrop: &Backdrop, fit: &Layout, window_h: f32) {
        let scale = fit.scale as f32;
        let clip = backdrop.clip_px();
        let (x, y) = (
            fit.offset_x + clip.x as f32 * scale,
            fit.offset_y + clip.y as f32 * scale,
        );
        let (w, h) = (clip.w as f32 * scale, clip.h as f32 * scale);
        set_camera(&Camera2D {
            target: vec2(w / 2.0, h / 2.0),
            zoom: vec2(2.0 / w, 2.0 / h),
            viewport: Some((x as i32, (window_h - y - h) as i32, w as i32, h as i32)),
            ..Default::default()
        });
        let (dx, dy) = backdrop.scene_offset(fit.scale);
        let scene_scale = f32::from(backdrop.zoom()) * scale;
        self.draw_layers(
            backdrop.scene(),
            (dx as f32, dy as f32),
            scene_scale,
            Fills::NotClear,
        );
        set_default_camera();
    }

    /// Draws `buf`'s cells and items with its top-left corner at `origin`
    /// and each of its pixels `scale` big: cell backgrounds (`fills` says
    /// which; never a see-through cell's), `Under` items, glyphs, `Over`
    /// items.
    fn draw_layers(&mut self, buf: &GlyphBuffer, origin: (f32, f32), scale: f32, fills: Fills) {
        let (offset_x, offset_y) = origin;
        let cell_w = f32::from(buf.spec().cell_w) * scale;
        let cell_h = f32::from(buf.spec().cell_h) * scale;
        let cells = || {
            (0..i32::from(buf.height())).flat_map(move |y| {
                (0..i32::from(buf.width())).filter_map(move |x| {
                    #[allow(clippy::cast_precision_loss)] // cell coords < 2^16
                    let at = (offset_x + x as f32 * cell_w, offset_y + y as f32 * cell_h);
                    buf.get(x, y).map(|cell| (at, cell))
                })
            })
        };
        for ((px, py), cell) in cells() {
            if !cell.see_through && (fills == Fills::All || cell.bg != self.clear) {
                draw_rectangle(px, py, cell_w, cell_h, color(cell.bg));
            }
        }
        self.draw_items(buf, Layer::Under, origin, scale);
        for ((px, py), cell) in cells() {
            if cell.glyph == ' ' {
                continue;
            }
            let Some((rect, fg)) = self.atlas_cell(cell.glyph, cell.fg) else {
                continue;
            };
            draw_texture_ex(
                &self.texture,
                px,
                py,
                fg,
                DrawTextureParams {
                    dest_size: Some(vec2(cell_w, cell_h)),
                    source: Some(source_rect(rect)),
                    ..Default::default()
                },
            );
        }
        self.draw_items(buf, Layer::Over, origin, scale);
    }

    /// Draws `buf`'s items of `layer`, in order. `origin` is the buffer's
    /// top-left corner and `scale` the size of one of its pixels.
    fn draw_items(&mut self, buf: &GlyphBuffer, layer: Layer, origin: (f32, f32), scale: f32) {
        for item in buf.items().iter().filter(|item| item.layer() == layer) {
            let visible = item.visible();
            #[allow(clippy::cast_precision_loss)] // console pixels < 2^21
            let (x, y, size) = (
                origin.0 + visible.x as f32 * scale,
                origin.1 + visible.y as f32 * scale,
                vec2(visible.w as f32, visible.h as f32) * scale,
            );
            match item {
                Item::Rect(rect) => draw_rectangle(x, y, size.x, size.y, color(rect.color)),
                Item::Sprite(sprite) => self.draw_sprite(sprite, x, y, size),
            }
        }
    }

    /// Draws the visible part of `sprite` (its `clip`) at `(x, y)`, `size`
    /// big: the matching part of its image, or a [`MISSING_COLOR`]
    /// rectangle if the image has no texture (logged once per image).
    fn draw_sprite(&mut self, sprite: &Sprite, x: f32, y: f32, size: Vec2) {
        let (recolour, tint) = match sprite.paint {
            Paint::Image => (None, Rgb::new(255, 255, 255)),
            Paint::Solid(color) => (Some(Recolour::Silhouette), color),
            Paint::Dimmed => (Some(Recolour::Dimmed), Rgb::new(255, 255, 255)),
            Paint::Shaded(light) => (Some(Recolour::Shaded(light)), Rgb::new(255, 255, 255)),
        };
        let bytes = self.bytes;
        let texture = match recolour {
            None => self.images.get(&sprite.image),
            Some(recolour) => self
                .recoloured
                .entry((sprite.image, recolour))
                .or_insert_with(|| recoloured(bytes(sprite.image.path())?, recolour))
                .as_ref(),
        };
        let Some(texture) = texture else {
            if self.warned_images.insert(sprite.image) {
                warn!("image {} has no texture", sprite.image.path());
            }
            draw_rectangle(x, y, size.x, size.y, MISSING_COLOR);
            return;
        };
        let [sx, sy, sw, sh] = sprite.clipped_src();
        // Darker as the sprite's brightness has it (game ADR-0075).
        let tint = tint.scale(f32::from(sprite.brightness) / 255.0);
        draw_texture_ex(
            texture,
            x,
            y,
            Color::from_rgba(tint.r, tint.g, tint.b, sprite.opacity),
            DrawTextureParams {
                dest_size: Some(size),
                source: Some(Rect::new(sx, sy, sw, sh)),
                flip_x: sprite.flip_x,
                ..Default::default()
            },
        );
    }

    /// Where to find `glyph` in the atlas and what colour to tint it: `fg`,
    /// or the fallback glyph in [`MISSING_COLOR`] (logged once per glyph).
    fn atlas_cell(&mut self, glyph: char, fg: Rgb) -> Option<(AtlasRect, Color)> {
        if let Some(rect) = self.atlas.glyph_rect(glyph) {
            return Some((rect, color(fg)));
        }
        if self.warned.insert(glyph) {
            warn!(
                "glyph {:?} (U+{:04X}) not in font atlas",
                glyph,
                u32::from(glyph)
            );
        }
        Some((self.atlas.glyph_rect(FALLBACK_GLYPH)?, MISSING_COLOR))
    }
}

/// Decodes PNG file `png` into a texture drawn with nearest-pixel sampling.
/// A texture of PNG file `png`, or why it can't be decoded.
pub(crate) fn upload(png: &[u8]) -> Result<Texture2D, String> {
    let image =
        Image::from_file_with_format(png, Some(ImageFormat::Png)).map_err(|e| e.to_string())?;
    let texture = Texture2D::from_image(&image);
    texture.set_filter(FilterMode::Nearest);
    Ok(texture)
}

/// A texture of PNG file `png` recoloured as `recolour`, or `None` if it
/// can't be decoded.
fn recoloured(png: &[u8], recolour: Recolour) -> Option<Texture2D> {
    let mut picture = Image::from_file_with_format(png, Some(ImageFormat::Png)).ok()?;
    for pixel in picture.bytes.as_chunks_mut::<4>().0 {
        let own = Rgb::new(pixel[0], pixel[1], pixel[2]);
        let rgb = match recolour {
            Recolour::Silhouette => Rgb::new(255, 255, 255),
            Recolour::Dimmed => Paint::Dimmed.apply(own),
            Recolour::Shaded(light) => Paint::Shaded(light).apply(own),
        };
        pixel[..3].copy_from_slice(&[rgb.r, rgb.g, rgb.b]);
    }
    let texture = Texture2D::from_image(&picture);
    texture.set_filter(FilterMode::Nearest);
    Some(texture)
}

/// `c` as macroquad's colour.
pub(crate) fn color(c: Rgb) -> Color {
    Color::from_rgba(c.r, c.g, c.b, 255)
}

#[allow(clippy::cast_precision_loss)] // atlas coordinates are small
fn source_rect(r: AtlasRect) -> Rect {
    Rect::new(r.x as f32, r.y as f32, r.w as f32, r.h as f32)
}
