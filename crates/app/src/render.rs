//! Blits a [`GlyphBuffer`] to the window with the font atlas:
//! integer-scaled, centred, black letterbox; or, for a game that fills the
//! window ([`ScaleMode::Fit`], ADR-0140), drawn at a whole scale into a
//! texture that is then stretched to fill with linear filtering ("sharp
//! bilinear": a pixel's edge blends over one window pixel at most). The buffer's items are drawn
//! scaled, in the order they were added: `Under` ones after the cell
//! backgrounds, `Over` ones after the glyphs. A rectangle is a
//! solid fill; a sprite is part of an image's texture, or of a
//! recoloured copy of it when the sprite isn't painted in the image's own
//! colours. A buffer's backdrop, a second buffer
//! panned and zoomed, is drawn the same way inside its window, behind the
//! console. A wide glyph (its continuation cell after it) is one quad over
//! both its cells, as the raster paints it. A painted picture (ADR-0078) is
//! a texture of its own, uploaded only in a frame whose revision moved
//! (`PaintedTextures`, in `painted.rs`), drawn with nearest-pixel sampling.
//! An image's texture is made the first frame a sprite draws it
//! (`ImageTextures`, in `image_textures.rs`; ADR-0128 §5). A sprite
//! atlas's page is a painted picture (`shuyi_ui::frame::atlas`, ticket
//! 2222): uploaded the first frame a sprite draws it, and again in a frame
//! whose page changed.
//!
//! A picture's pixels come from `pixels`: a PNG decoded by macroquad, any
//! other file by the formats of `Formats::standard()`, those whose feature
//! of `shuyi-assets` the game turns on (JPEG, WebP, QOI, JPEG XL; ticket
//! 2234). The font atlas stays a PNG.

use std::borrow::Cow;
use std::collections::{HashMap, HashSet};

use macroquad::prelude::*;
use shuyi_assets::font::{AtlasRect, FALLBACK_GLYPH, FontAtlasDef};
use shuyi_assets::format::{AssetPath, DecodeCx, Formats};
use shuyi_assets::image::{self as assets_image, ImageId, PNG_SIGNATURE};
use shuyi_assets::lighting::Light;
use shuyi_ui::frame::Rgb;
use shuyi_ui::frame::console::{ConsoleSpec, Layout, Placement, ScaleMode, frame_of, place};
use shuyi_ui::frame::glyph_buffer::{Backdrop, GlyphBuffer, Item, Layer, Paint, Sprite};
use shuyi_ui::frame::painted::{PaintedSprite, PaintedTable};

use crate::image_textures::{ImageTextures, Missing, Read};
use crate::painted::PaintedTextures;
use crate::pictures::{Fresh, Remake, src_fits};

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
    /// The images' textures, each made the first frame a sprite draws it.
    images: ImageTextures<Texture2D>,
    /// The recoloured copies made so far: each is made the first time a
    /// sprite needs it, from its image's file. `None` if that can't be
    /// decoded.
    recoloured: HashMap<(ImageId, Recolour), Option<Texture2D>>,
    /// Images without a texture already logged, so each is reported once.
    warned_images: HashSet<ImageId>,
    /// The size of each image made again since the start, for what its
    /// sprites may show ([`src_fits`]).
    remade: HashMap<ImageId, (u16, u16)>,
    /// The textures of the frame's painted pictures.
    painted: PaintedTextures<Texture2D>,
    /// How the frame fills the window ([`set_scale`](Self::set_scale)).
    scale: ScaleMode,
    /// What a frame shown at a fraction is drawn into first, kept between
    /// frames ([`Stretch`]).
    stretch: Stretch,
    /// What decodes a picture that isn't a PNG ([`pixels`]).
    formats: Formats,
}

impl Renderer {
    /// Uploads the atlas image (`png`). `clear` is the console background
    /// (the palette's `black`). An image's texture is made the first frame
    /// a sprite draws it, from what `bytes` (the game's files by path)
    /// gives then (ADR-0128 §5): an image it gives nothing for is drawn
    /// in magenta and read again the next frame that draws it; one
    /// that can't be decoded is logged once and drawn so for the run.
    pub fn new(
        atlas: FontAtlasDef,
        png: &[u8],
        clear: Rgb,
        bytes: fn(&str) -> Option<Cow<'static, [u8]>>,
    ) -> Result<Self, String> {
        Self::with_files(atlas, png, clear, bytes)
    }

    /// [`new`](Self::new), with `bytes` any way of reading the files.
    pub(crate) fn with_files(
        atlas: FontAtlasDef,
        png: &[u8],
        clear: Rgb,
        bytes: impl Fn(&str) -> Option<Cow<'static, [u8]>> + 'static,
    ) -> Result<Self, String> {
        let formats = Formats::standard();
        let texture =
            upload(&formats, "font atlas", png).map_err(|e| format!("font atlas image: {e}"))?;
        let read: Read = Box::new(bytes);
        Ok(Self {
            texture,
            atlas,
            clear,
            warned: HashSet::new(),
            images: ImageTextures::new(read),
            recoloured: HashMap::new(),
            warned_images: HashSet::new(),
            remade: HashMap::new(),
            painted: PaintedTextures::new(),
            scale: ScaleMode::Integer,
            stretch: Stretch::default(),
            formats,
        })
    }

    /// What decodes a picture that isn't a PNG ([`pixels`]).
    pub(crate) const fn formats(&self) -> &Formats {
        &self.formats
    }

    /// Makes the textures of `fresh` again, from pictures that decoded. A
    /// font's picture takes `font`, the game's atlas now. An image's
    /// recoloured copies are made again as sprites next need them.
    pub(crate) fn remake(&mut self, fresh: Vec<Fresh<Image>>, font: &FontAtlasDef) {
        for Fresh {
            remake,
            decoded,
            bytes,
        } in fresh
        {
            let texture = texture_of(&decoded);
            match remake {
                Remake::Font => {
                    self.texture = texture;
                    self.atlas = font.clone();
                    self.warned.clear();
                }
                Remake::Image(id) => {
                    self.images.replace(id, texture, bytes);
                    self.remade.insert(id, (decoded.width, decoded.height));
                    self.recoloured.retain(|(image, _), _| *image != id);
                    self.warned_images.remove(&id);
                }
            }
        }
    }

    /// Sets the console background (the palette's `black`), which a
    /// colour theme changes.
    pub fn set_clear(&mut self, clear: Rgb) {
        self.clear = clear;
    }

    /// Sets how the frame fills the window (the frame's
    /// [`scale`](shuyi_platform::Frame::scale)).
    pub fn set_scale(&mut self, scale: ScaleMode) {
        self.scale = scale;
    }

    /// Draws `buf` scaled to the current window size.
    ///
    /// The placement is computed in physical framebuffer pixels (the window
    /// is `high_dpi`), so the scale holds on scaled Windows displays;
    /// coordinates are divided by the DPI factor only because macroquad's
    /// default camera works in logical units. A placement at a whole scale
    /// is drawn straight into the window; one at a fraction is drawn at
    /// its [`draw_scale`](Placement::draw_scale) into a texture first
    /// (a render target kept between frames), then stretched into its rectangle.
    ///
    /// With a backdrop the order is: the clear colour, the
    /// scene inside its window, then the console over it, whose
    /// see-through cells have no background.
    pub fn draw(&mut self, buf: &GlyphBuffer) {
        let dpi = screen_dpi_scale();
        let placed = placement(
            buf.spec(),
            self.scale,
            (screen_width(), screen_height()),
            dpi,
        );
        clear_background(BLACK);
        let frame = frame_of(buf.spec());
        if placed.is_whole(frame) {
            #[allow(clippy::cast_precision_loss)] // whole pixels of a window
            let fit = Layout {
                scale: placed.draw_scale,
                offset_x: placed.x as f32,
                offset_y: placed.y as f32,
            };
            let window_h = (screen_height() * dpi).round();
            self.draw_frame(buf, &fit, dpi, Surface::Window { height: window_h });
        } else {
            let mut stretch = core::mem::take(&mut self.stretch);
            stretch.draw(frame, placed, dpi, |target| {
                let fit = Layout {
                    scale: placed.draw_scale,
                    offset_x: 0.0,
                    offset_y: 0.0,
                };
                self.draw_frame(buf, &fit, 1.0, Surface::Target(target));
            });
            self.stretch = stretch;
        }
        self.painted.end_frame();
    }

    /// Draws `buf` with its top-left corner and scale `fit` (in physical
    /// pixels), on `surface`: the clear colour, the backdrop, the layers.
    /// `dpi` turns physical pixels into the camera's units.
    fn draw_frame(&mut self, buf: &GlyphBuffer, fit: &Layout, dpi: f32, surface: Surface<'_>) {
        #[allow(clippy::cast_precision_loss)] // scale is small
        let scale = fit.scale as f32 / dpi;
        let (offset_x, offset_y) = (fit.offset_x / dpi, fit.offset_y / dpi);
        draw_rectangle(
            offset_x,
            offset_y,
            f32::from(buf.width()) * f32::from(buf.spec().cell_w) * scale,
            f32::from(buf.height()) * f32::from(buf.spec().cell_h) * scale,
            color(self.clear),
        );
        let pictures = buf.pictures();
        if let Some(backdrop) = buf.backdrop() {
            self.draw_backdrop(backdrop, pictures, fit, surface);
        }
        // Over a scene, a cell in the clear colour must hide it.
        let fills = if buf.backdrop().is_some() {
            Fills::All
        } else {
            Fills::NotClear
        };
        self.draw_layers((buf, pictures), (offset_x, offset_y), scale, fills);
    }

    /// Draws `backdrop`'s scene in its window: every scene pixel
    /// `zoom × fit.scale` physical pixels big, placed on whole physical
    /// pixels ([`Backdrop::scene_offset`]).
    ///
    /// The window clips it with a camera whose view is exactly the window,
    /// one unit per physical pixel: what falls outside a camera's view
    /// isn't drawn. (macroquad's scissor is only reachable through an
    /// `unsafe` call, which this workspace forbids.) On the window, a
    /// viewport counts rows from the bottom; on a target, from its first
    /// row, which is the frame's top ([`Stretch`]). Its painted items show
    /// the frame's `pictures`. It gives the surface back its own camera.
    #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)] // whole pixels
    fn draw_backdrop(
        &mut self,
        backdrop: &Backdrop,
        pictures: &PaintedTable,
        fit: &Layout,
        surface: Surface<'_>,
    ) {
        let scale = fit.scale as f32;
        let clip = backdrop.clip_px();
        let (x, y) = (
            fit.offset_x + clip.x as f32 * scale,
            fit.offset_y + clip.y as f32 * scale,
        );
        let (w, h) = (clip.w as f32 * scale, clip.h as f32 * scale);
        let (row, render_target) = match surface {
            Surface::Window { height } => (height - y - h, None),
            Surface::Target(target) => (y, Some(target.clone())),
        };
        set_camera(&Camera2D {
            target: vec2(w / 2.0, h / 2.0),
            zoom: vec2(2.0 / w, 2.0 / h),
            viewport: Some((x as i32, row as i32, w as i32, h as i32)),
            render_target,
            ..Default::default()
        });
        let (dx, dy) = backdrop.scene_offset(fit.scale);
        let scene_scale = f32::from(backdrop.zoom()) * scale;
        self.draw_layers(
            (backdrop.scene(), pictures),
            (dx as f32, dy as f32),
            scene_scale,
            Fills::NotClear,
        );
        match surface {
            Surface::Window { .. } => set_default_camera(),
            Surface::Target(target) => set_camera(&target_camera(target)),
        }
    }

    /// Draws `buf`'s cells and items with its top-left corner at `origin`
    /// and each of its pixels `scale` big: cell backgrounds (`fills` says
    /// which; never a see-through cell's), `Under` items, glyphs, `Over`
    /// items. Its painted items show `pictures`, the frame's.
    fn draw_layers(
        &mut self,
        (buf, pictures): (&GlyphBuffer, &PaintedTable),
        origin: (f32, f32),
        scale: f32,
        fills: Fills,
    ) {
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
        self.draw_items((buf, pictures), Layer::Under, origin, scale);
        for (x, y, cell, wide) in buf.glyphs() {
            if cell.glyph == ' ' {
                continue;
            }
            let set = buf.widths().wide_font();
            let Some((rect, fg, cells)) = self.atlas_cell(cell.glyph, cell.fg, wide, set) else {
                continue;
            };
            // A key cap's glyph: moved off the grid, and bold (ticket 3215).
            let style = buf.glyph_style(x, y);
            let (rect, strikes) = self
                .atlas
                .bold_strikes(cell.glyph, rect, style.bold && !wide);
            #[allow(clippy::cast_precision_loss)] // cell coords < 2^16
            let (px, py) = (
                offset_x + x as f32 * cell_w + f32::from(style.dx) * scale,
                offset_y + y as f32 * cell_h + f32::from(style.dy) * scale,
            );
            for strike in 0..strikes {
                draw_texture_ex(
                    &self.texture,
                    px + f32::from(strike) * scale,
                    py,
                    fg,
                    DrawTextureParams {
                        dest_size: Some(vec2(cell_w * cells, cell_h)),
                        source: Some(source_rect(rect)),
                        ..Default::default()
                    },
                );
            }
        }
        self.draw_items((buf, pictures), Layer::Over, origin, scale);
    }

    /// Draws `buf`'s items of `layer`, in order, its painted items from
    /// `pictures`. `origin` is the buffer's top-left corner and `scale` the
    /// size of one of its pixels.
    fn draw_items(
        &mut self,
        (buf, pictures): (&GlyphBuffer, &PaintedTable),
        layer: Layer,
        origin: (f32, f32),
        scale: f32,
    ) {
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
                Item::Painted(painted) => self.draw_painted(painted, pictures, x, y, size),
                // One rectangle a run of whole console pixels, by
                // `frame::shape`'s rules, so the window equals `frame-png`.
                #[allow(clippy::cast_precision_loss)] // console pixels < 2^21
                Item::Gizmo(gizmo) => gizmo.for_each_run(|row, x0, x1| {
                    draw_rectangle(
                        origin.0 + x0 as f32 * scale,
                        origin.1 + row as f32 * scale,
                        (x1 - x0) as f32 * scale,
                        scale,
                        color(gizmo.color),
                    );
                }),
                // A kind of item this backend doesn't know yet draws nothing.
                _ => {}
            }
        }
    }

    /// Draws the visible part of `sprite` (its `clip`) at `(x, y)`, `size`
    /// big: the matching part of its image, its texture made now if it has
    /// none yet, or a [`MISSING_COLOR`] rectangle if the image has no
    /// texture, or was saved too small for the sprite since the start
    /// (logged once per image).
    fn draw_sprite(&mut self, sprite: &Sprite, x: f32, y: f32, size: Vec2) {
        if let Some(&picture) = self.remade.get(&sprite.image)
            && !src_fits(sprite.src, picture)
        {
            if self.warned_images.insert(sprite.image) {
                log::warn!(target: "assets", "image {} is {} by {} now: a sprite's source is past its edge",
                    sprite.image.path(),
                    picture.0,
                    picture.1
                );
            }
            draw_rectangle(x, y, size.x, size.y, MISSING_COLOR);
            return;
        }
        let (recolour, tint) = match sprite.paint {
            Paint::Solid(color) => (Some(Recolour::Silhouette), color),
            Paint::Dimmed => (Some(Recolour::Dimmed), Rgb::new(255, 255, 255)),
            Paint::Shaded(light) => (Some(Recolour::Shaded(light)), Rgb::new(255, 255, 255)),
            // `Paint::Image`, and a paint newer than this backend: the
            // image's own colours.
            _ => (None, Rgb::new(255, 255, 255)),
        };
        let path = sprite.image.path();
        let formats = &self.formats;
        let upload = |bytes: &[u8]| decode(formats, path, bytes).map(|image| texture_of(&image));
        if let Err(missing) = self.images.texture(sprite.image, upload) {
            match missing {
                Missing::Undecodable(Some(e)) => {
                    log::warn!(target: "assets", "image {path}: {e}");
                }
                Missing::NotRead if self.warned_images.insert(sprite.image) => {
                    log::warn!(target: "assets", "image {path}: nothing read yet");
                }
                _ => {}
            }
            draw_rectangle(x, y, size.x, size.y, MISSING_COLOR);
            return;
        }
        let texture = match recolour {
            None => self.images.get(sprite.image),
            Some(recolour) => {
                let source = self.images.source(sprite.image);
                self.recoloured
                    .entry((sprite.image, recolour))
                    .or_insert_with(|| recoloured(formats, path, source?, recolour))
                    .as_ref()
            }
        };
        let Some(texture) = texture else {
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

    /// Draws the visible part of `painted` (its `clip`) at `(x, y)`, `size`
    /// big, from its picture's texture, uploaded only when the picture's
    /// revision moved; a [`MISSING_COLOR`] rectangle if `pictures` lacks
    /// it (ADR-0078).
    fn draw_painted(
        &mut self,
        painted: &PaintedSprite,
        pictures: &PaintedTable,
        x: f32,
        y: f32,
        size: Vec2,
    ) {
        let Some(picture) = pictures.get(painted.picture) else {
            draw_rectangle(x, y, size.x, size.y, MISSING_COLOR);
            return;
        };
        let (width, height) = picture.size();
        if width == 0 || height == 0 {
            return;
        }
        // A sprite atlas's page is one of these pictures (ticket 2222):
        // made the first frame a sprite draws it, again when it changed.
        let make = |rgba: &[u8]| upload_rgba(width, height, rgba);
        let update = |texture: &Texture2D, rgba: &[u8]| {
            texture.update_from_bytes(width.into(), height.into(), rgba);
        };
        let texture = self.painted.texture(painted.picture, picture, make, update);
        let [sx, sy, sw, sh] = painted.clipped_src();
        let bright = painted.brightness;
        draw_texture_ex(
            texture,
            x,
            y,
            Color::from_rgba(bright, bright, bright, 255),
            DrawTextureParams {
                dest_size: Some(size),
                source: Some(Rect::new(sx, sy, sw, sh)),
                ..Default::default()
            },
        );
    }

    /// Where to find `glyph` in the atlas, what colour to tint it and how
    /// many cells it covers: `fg` over both cells for a wide glyph whose
    /// continuation follows it (`wide`), from the language's wide set `set`
    /// first ([`FontAtlasDef::wide_cell`]), over one for a narrow glyph, or
    /// the fallback glyph in [`MISSING_COLOR`] over one (logged once per
    /// glyph). As the raster's painter does.
    fn atlas_cell(
        &mut self,
        glyph: char,
        fg: Rgb,
        wide: bool,
        set: Option<&str>,
    ) -> Option<(AtlasRect, Color, f32)> {
        // A narrow glyph is never looked up in the wide sets: most cells
        // are narrow, and a lookup costs a search of every set (3244).
        if wide && let Some(rect) = self.atlas.wide_rect_in(set, glyph) {
            return Some((rect, color(fg), 2.0));
        }
        if let Some(rect) = self.atlas.glyph_rect(glyph) {
            return Some((rect, color(fg), 1.0));
        }
        if self.warned.insert(glyph) {
            log::warn!(target: "assets", "glyph {:?} (U+{:04X}) not in font atlas",
                glyph,
                u32::from(glyph)
            );
        }
        Some((self.atlas.glyph_rect(FALLBACK_GLYPH)?, MISSING_COLOR, 1.0))
    }
}

/// Where a `spec` console goes, scaled as `mode`, in a window `logical`
/// pixels big at a DPI factor of `dpi`, in the physical pixels
/// [`Renderer::draw`] places it in (the window is `high_dpi`). The pointer
/// maps the mouse back through it ([`pointer`](crate::pointer)).
pub(crate) fn placement(
    spec: ConsoleSpec,
    mode: ScaleMode,
    logical: (f32, f32),
    dpi: f32,
) -> Placement {
    place(mode, frame_of(spec), physical(logical, dpi))
}

/// A window `logical` pixels big at a DPI factor of `dpi`, in whole
/// physical pixels. Rounded: logical size × DPI can land a hair under the
/// real pixel count (913.714 × 1.75 = 1599.99…), which would drop a whole
/// scale step.
pub(crate) fn physical(logical: (f32, f32), dpi: f32) -> (u32, u32) {
    let whole = |side: f32| {
        let px = (side * dpi).round();
        // In 0..=f32::MAX after the check; `as` saturates.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let px = if px.is_finite() {
            px.max(0.0) as u32
        } else {
            0
        };
        px
    };
    (whole(logical.0), whole(logical.1))
}

/// What a frame is drawn on: the window, `height` physical pixels tall, or
/// a [`Stretch`]'s target.
#[derive(Clone, Copy)]
enum Surface<'a> {
    Window { height: f32 },
    Target(&'a RenderTarget),
}

/// The texture a picture shown at a fraction of a pixel is drawn into at
/// a whole scale first, then stretched from with linear filtering, so a
/// pixel's edge blends over one window pixel at most (ADR-0140). It is
/// kept between frames and made again when its size changes.
///
/// Its camera has one unit per texture pixel and `y` down. macroquad turns
/// `y` the other way for a render target (its `Camera2D::matrix`), so the
/// frame's top row is the texture's first row, which a draw of the texture
/// puts at the top: neither is flipped.
#[derive(Default)]
pub(crate) struct Stretch {
    target: Option<RenderTarget>,
}

impl Stretch {
    /// Draws a `frame`-pixel picture shown at `placed`: `paint` draws it at
    /// [`draw_scale`](Placement::draw_scale) on the target (whose camera
    /// is set), then the target is stretched into `placed`'s rectangle,
    /// divided by `dpi`. A rectangle of 0 pixels draws nothing.
    pub(crate) fn draw(
        &mut self,
        frame: (u32, u32),
        placed: Placement,
        dpi: f32,
        paint: impl FnOnce(&RenderTarget),
    ) {
        if placed.w == 0 || placed.h == 0 {
            return;
        }
        let size = (
            frame.0.saturating_mul(placed.draw_scale).max(1),
            frame.1.saturating_mul(placed.draw_scale).max(1),
        );
        let target = self.target_of(size);
        set_camera(&target_camera(&target));
        clear_background(BLACK);
        paint(&target);
        set_default_camera();
        #[allow(clippy::cast_precision_loss)] // whole pixels of a window
        let params = DrawTextureParams {
            dest_size: Some(vec2(placed.w as f32 / dpi, placed.h as f32 / dpi)),
            ..Default::default()
        };
        #[allow(clippy::cast_precision_loss)] // whole pixels of a window
        let (x, y) = (placed.x as f32 / dpi, placed.y as f32 / dpi);
        draw_texture_ex(&target.texture, x, y, WHITE, params);
    }

    /// The target, `size` pixels big: the one kept, or a new one.
    fn target_of(&mut self, size: (u32, u32)) -> RenderTarget {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // a texture's side
        let kept = self.target.as_ref().filter(|target| {
            let texture = &target.texture;
            (texture.width() as u32, texture.height() as u32) == size
        });
        if let Some(target) = kept {
            return target.clone();
        }
        let target = render_target(size.0, size.1);
        target.texture.set_filter(FilterMode::Linear);
        self.target = Some(target.clone());
        target
    }
}

/// The camera that draws on `target`: one unit per pixel, `y` down from
/// its top-left corner ([`Stretch`]).
fn target_camera(target: &RenderTarget) -> Camera2D {
    let (w, h) = (target.texture.width(), target.texture.height());
    Camera2D {
        target: vec2(w / 2.0, h / 2.0),
        zoom: vec2(2.0 / w, 2.0 / h),
        render_target: Some(target.clone()),
        ..Default::default()
    }
}

/// The pixels of the picture file at `path`, whose bytes are `bytes`: its
/// width, height and RGBA bytes, or why it can't be decoded. A PNG is
/// decoded by macroquad, as it always was; any other file by `formats`
/// (`Formats::standard()`), whose error names the formats it has.
pub(crate) fn pixels(
    formats: &Formats,
    path: &str,
    bytes: &[u8],
) -> Result<(u16, u16, Vec<u8>), String> {
    if bytes.starts_with(PNG_SIGNATURE) {
        let image = Image::from_file_with_format(bytes, Some(ImageFormat::Png))
            .map_err(|e| e.to_string())?;
        return Ok((image.width, image.height, image.bytes));
    }
    let decoded: assets_image::Image = formats
        .decode(&AssetPath::new(path), bytes, &DecodeCx::new())
        .map_err(|errors| {
            let messages: Vec<String> = errors.into_iter().map(|e| e.message).collect();
            messages.join("; ")
        })?;
    // A format's picture is at most `image::MAX_SIDE` (4096) on a side.
    let side = |px: u32| u16::try_from(px).map_err(|_| format!("{px} pixels is too wide"));
    Ok((side(decoded.width)?, side(decoded.height)?, decoded.rgba))
}

/// The picture file at `path` ([`pixels`]), as macroquad's image.
pub(crate) fn decode(formats: &Formats, path: &str, bytes: &[u8]) -> Result<Image, String> {
    let (width, height, bytes) = pixels(formats, path, bytes)?;
    Ok(Image {
        bytes,
        width,
        height,
    })
}

/// A texture of `width × height` RGBA pixels, drawn with nearest-pixel
/// sampling.
pub(crate) fn upload_rgba(width: u16, height: u16, rgba: &[u8]) -> Texture2D {
    let texture = Texture2D::from_rgba8(width, height, rgba);
    texture.set_filter(FilterMode::Nearest);
    texture
}

/// `image` as a texture drawn with nearest-pixel sampling.
fn texture_of(image: &Image) -> Texture2D {
    upload_rgba(image.width, image.height, &image.bytes)
}

/// A texture of the picture file at `path` ([`pixels`]), or why it can't
/// be decoded.
pub(crate) fn upload(formats: &Formats, path: &str, bytes: &[u8]) -> Result<Texture2D, String> {
    decode(formats, path, bytes).map(|image| texture_of(&image))
}

/// A texture of the picture file at `path` ([`pixels`]) recoloured as
/// `recolour`, or `None` if it can't be decoded.
fn recoloured(
    formats: &Formats,
    path: &str,
    bytes: &[u8],
    recolour: Recolour,
) -> Option<Texture2D> {
    let mut picture = decode(formats, path, bytes).ok()?;
    for pixel in picture.bytes.as_chunks_mut::<4>().0 {
        let own = Rgb::new(pixel[0], pixel[1], pixel[2]);
        let rgb = match recolour {
            Recolour::Silhouette => Rgb::new(255, 255, 255),
            Recolour::Dimmed => Paint::Dimmed.apply(own),
            Recolour::Shaded(light) => Paint::Shaded(light).apply(own),
        };
        pixel[..3].copy_from_slice(&[rgb.r, rgb.g, rgb.b]);
    }
    Some(texture_of(&picture))
}

/// `c` as macroquad's colour.
pub(crate) fn color(c: Rgb) -> Color {
    Color::from_rgba(c.r, c.g, c.b, 255)
}

#[allow(clippy::cast_precision_loss)] // atlas coordinates are small
fn source_rect(r: AtlasRect) -> Rect {
    Rect::new(r.x as f32, r.y as f32, r.w as f32, r.h as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fixture of the picture formats' tests, by file name.
    macro_rules! fixture {
        ($name:literal) => {
            include_bytes!(concat!("../../assets/tests/fixtures/formats/", $name)).as_slice()
        };
    }

    #[test]
    fn a_picture_that_isnt_a_png_has_the_pixels_of_the_registry() {
        let formats = Formats::standard();
        let fixtures = [
            ("a.jpg", fixture!("picture.jpg")),
            ("a.webp", fixture!("picture.webp")),
            ("a.webp", fixture!("lossless.webp")),
            ("a.qoi", fixture!("picture.qoi")),
            ("a.jxl", fixture!("picture.jxl")),
            ("a.jxl", fixture!("lossless.jxl")),
        ];
        for (path, bytes) in fixtures {
            let decoded: assets_image::Image = formats
                .decode(&AssetPath::new(path), bytes, &DecodeCx::new())
                .unwrap();
            assert_eq!((decoded.width, decoded.height), (20, 12), "{path}");
            assert_eq!(
                pixels(&formats, path, bytes),
                Ok((20, 12, decoded.rgba)),
                "{path}"
            );
        }
    }

    #[test]
    fn a_png_is_decoded_by_macroquad_as_before() {
        let png = fixture!("picture.png");
        let before = Image::from_file_with_format(png, Some(ImageFormat::Png)).unwrap();
        // With no format for it in the registry, it decodes the same.
        let none = Formats::new();
        for formats in [&none, &Formats::standard()] {
            let (width, height, rgba) = pixels(formats, "a.png", png).unwrap();
            assert_eq!((width, height), (before.width, before.height));
            assert_eq!(rgba, before.bytes);
        }
        // The lossless fixtures hold the PNG's pixels.
        let qoi = pixels(&Formats::standard(), "a.qoi", fixture!("picture.qoi"));
        assert_eq!(qoi, Ok((20, 12, before.bytes)));
    }

    #[test]
    fn bytes_no_format_reads_give_the_message_of_the_registry() {
        let formats = Formats::standard();
        let why = pixels(&formats, "a.bmp", b"BM junk").unwrap_err();
        assert!(
            why.starts_with("no image pixels format knows its bytes"),
            "{why}"
        );
        assert!(why.contains("jpeg"), "it names the formats: {why}");
        let none = pixels(&Formats::new(), "a.jpg", fixture!("picture.jpg")).unwrap_err();
        assert!(
            none.ends_with("no image pixels format is registered"),
            "{none}"
        );
        // A broken PNG is macroquad's to refuse.
        let png = fixture!("picture.png");
        assert!(pixels(&formats, "a.png", &png[..20]).is_err());
        // Each side fits macroquad's `u16`: a picture is at most 4096 wide.
        assert!(u16::try_from(assets_image::MAX_SIDE).is_ok());
    }
}
