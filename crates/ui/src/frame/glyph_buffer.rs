//! The [`GlyphBuffer`] virtual console every screen draws into (game ADR-0003).
//!
//! Positions are `i32` so callers can draw partly (or wholly) off-buffer:
//! every primitive clips to the buffer and never panics.
//!
//! Besides cells, a buffer holds [`Item`]s, placed in console pixels:
//! [`Overlay`]s, coloured rectangles for what whole cells can't draw (HP
//! bars, the path line; game ADR-0018), and [`Sprite`]s, pictures from image
//! files (game ADR-0038). Within a layer, items are drawn in the order they were
//! added. They belong to the cells they were drawn with: replacing cells
//! ([`GlyphBuffer::fill_rect`], [`GlyphBuffer::blit`]) removes the parts of
//! items over them.
//!
//! A buffer may also hold one [`Backdrop`] (game ADR-0048): a second buffer, the
//! *scene*, shown through a window of cells at a pixel offset and a whole
//! zoom, behind the cells marked [`Cell::see_through`].

mod backdrop;

use crate::prelude::*;
use alloc::rc::Rc;

pub use backdrop::{Backdrop, MAX_ZOOM};

use shuyi_assets::image::ImageId;
use shuyi_assets::lighting::Light;

use crate::frame::color::Rgb;
use crate::frame::console::ConsoleSpec;

/// One console cell: a glyph with foreground and background colours.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub struct Cell {
    /// The character drawn in the cell.
    pub glyph: char,
    /// Glyph colour.
    pub fg: Rgb,
    /// Background colour.
    pub bg: Rgb,
    /// No background is drawn: the buffer's [`Backdrop`] shows behind the
    /// glyph (the console's clear colour where there is none). `bg` is
    /// kept but not shown.
    pub see_through: bool,
}

impl Cell {
    /// Builds a cell with a solid background.
    pub const fn new(glyph: char, fg: Rgb, bg: Rgb) -> Self {
        Self {
            glyph,
            fg,
            bg,
            see_through: false,
        }
    }

    /// A cell with no background (game ADR-0048): `glyph` in `fg` over the
    /// buffer's [`Backdrop`]. A space is a plain hole.
    /// [`GlyphBuffer::print_fg`] over it keeps it see-through; anything
    /// that sets a whole cell makes it solid again.
    pub const fn see_through(glyph: char, fg: Rgb) -> Self {
        Self {
            glyph,
            fg,
            bg: Rgb::new(0, 0, 0),
            see_through: true,
        }
    }
}

/// A rectangle of cells. A non-positive `w` or `h` means empty.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub struct Rect {
    /// Left column.
    pub x: i32,
    /// Top row.
    pub y: i32,
    /// Width in cells.
    pub w: i32,
    /// Height in cells.
    pub h: i32,
}

impl Rect {
    /// Builds a rectangle.
    pub const fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self { x, y, w, h }
    }

    /// Whether the rectangle has no cells.
    pub const fn is_empty(&self) -> bool {
        self.w <= 0 || self.h <= 0
    }

    /// Whether cell `(x, y)` lies inside.
    pub fn contains(&self, x: i32, y: i32) -> bool {
        let (x, y) = (i64::from(x), i64::from(y));
        let (left, top) = (i64::from(self.x), i64::from(self.y));
        x >= left && y >= top && x < left + i64::from(self.w) && y < top + i64::from(self.h)
    }

    /// The overlap of two rectangles, or `None` if they don't overlap.
    pub fn intersect(&self, other: &Rect) -> Option<Rect> {
        let x0 = self.x.max(other.x);
        let y0 = self.y.max(other.y);
        let x1 =
            (i64::from(self.x) + i64::from(self.w)).min(i64::from(other.x) + i64::from(other.w));
        let y1 =
            (i64::from(self.y) + i64::from(self.h)).min(i64::from(other.y) + i64::from(other.h));
        // The overlap is no wider than either input, so it fits in i32.
        let w = i32::try_from(x1 - i64::from(x0)).ok()?;
        let h = i32::try_from(y1 - i64::from(y0)).ok()?;
        let r = Rect::new(x0, y0, w, h);
        (!r.is_empty()).then_some(r)
    }
}

/// A rectangle in console pixels (the logical space before scaling: a cell
/// is its buffer's [`ConsoleSpec::cell_px`]). Same shape as a cell
/// [`Rect`].
pub type PxRect = Rect;

/// When an [`Item`] is drawn relative to the cells' glyphs.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub enum Layer {
    /// After cell backgrounds, before glyphs (e.g. the path line).
    Under,
    /// After glyphs (e.g. HP bars).
    Over,
}

impl Layer {
    /// Lowercase name, as in snapshots.
    pub const fn name(self) -> &'static str {
        match self {
            Layer::Under => "under",
            Layer::Over => "over",
        }
    }
}

/// A solid rectangle drawn on top of the cell grid (game ADR-0018).
#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub struct Overlay {
    /// Where, in console pixels.
    pub rect: PxRect,
    /// Fill colour.
    pub color: Rgb,
    /// Under or over the glyphs.
    pub layer: Layer,
}

impl Overlay {
    /// Builds an overlay.
    pub const fn new(rect: PxRect, color: Rgb, layer: Layer) -> Self {
        Self { rect, color, layer }
    }
}

/// How a [`Sprite`]'s pixels are coloured (game ADR-0049). Whichever it is, a
/// pixel is as see-through as the image has it.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash, Default)]
pub enum Paint {
    /// The image's own colours.
    #[default]
    Image,
    /// Every pixel in this one colour: the picture's silhouette (a unit's
    /// outline, a hit flash).
    Solid(Rgb),
    /// Each pixel [dimmed](Rgb::dimmed): grey and darker (a unit that has
    /// acted).
    Dimmed,
    /// Each pixel as it is in this light (game ADR-0058): the ground of a map
    /// at dusk.
    Shaded(Light),
}

impl Paint {
    /// The colour a pixel of the image's colour `rgb` is drawn in.
    #[must_use]
    pub fn apply(self, rgb: Rgb) -> Rgb {
        match self {
            Paint::Image => rgb,
            Paint::Solid(color) => color,
            Paint::Dimmed => rgb.dimmed(),
            Paint::Shaded(light) => light.apply([rgb.r, rgb.g, rgb.b]).into(),
        }
    }
}

/// A picture on top of the cell grid (game ADR-0038): part of an image file,
/// scaled into a rectangle of console pixels. `app` draws it from a texture
/// with nearest-pixel sampling, so whole-number scales stay sharp.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub struct Sprite {
    /// Which image (`ImageTable::id`).
    pub image: ImageId,
    /// The part of the image shown, in image pixels. It must lie inside
    /// the image.
    pub src: PxRect,
    /// Where `src` is stretched to, in console pixels.
    pub dest: PxRect,
    /// The part of `dest` that is drawn. Clipping and cutting only ever
    /// shrink this; `src` and `dest` stay as given, so the picture never
    /// shifts or rescales.
    pub clip: PxRect,
    /// Under or over the glyphs.
    pub layer: Layer,
    /// Mirrored left to right within `dest`.
    pub flip_x: bool,
    /// 255 = solid; lower lets what is under it show through.
    pub opacity: u8,
    /// How bright its pixels are drawn: 255 = as the image and the paint
    /// have them, lower = that much darker, 0 = black where it shows.
    /// Unlike a lower opacity, a picture over another (a tile's edge over
    /// its ground) darkens as much as one alone ([`GlyphBuffer::fade`]).
    pub brightness: u8,
    /// How its pixels are coloured.
    pub paint: Paint,
    /// The console pixels it stands on, for a picture that reaches above
    /// them (a unit's head over the tile above its own): where cells
    /// replace its base up to one of the base's edges, what it shows past
    /// that edge goes too, so a menu over a unit doesn't leave its head
    /// showing above the menu.
    pub base: Option<PxRect>,
}

impl Sprite {
    /// A solid, unflipped sprite showing all of `dest`.
    pub const fn new(image: ImageId, src: PxRect, dest: PxRect, layer: Layer) -> Self {
        Self {
            image,
            src,
            dest,
            clip: dest,
            layer,
            flip_x: false,
            opacity: u8::MAX,
            brightness: u8::MAX,
            paint: Paint::Image,
            base: None,
        }
    }

    /// What goes of this sprite, beyond `hole` itself, when the pixels
    /// `hole` are replaced: the part of its [`base`](Self::base) that
    /// `hole` covers, stretched outwards, past each edge of the base it
    /// reaches, to this sprite's own edge (up over the head, when it
    /// covers the base's top). `None` if it has no base or `hole` misses
    /// it. The rectangle may be empty: the sprite then shows nothing there.
    fn overhang(&self, hole: PxRect) -> Option<PxRect> {
        let base = self.base?;
        let cover = hole.intersect(&base)?;
        let seen = self.clip;
        // The low and the high edge of the stretched cover along one axis.
        let stretch = |cover: (i32, i32), base: (i32, i32), seen: (i32, i32)| {
            let low = if cover.0 == base.0 { seen.0 } else { cover.0 };
            let high = if cover.0 + cover.1 == base.0 + base.1 {
                seen.0 + seen.1
            } else {
                cover.0 + cover.1
            };
            (low, high)
        };
        let (left, right) = stretch((cover.x, cover.w), (base.x, base.w), (seen.x, seen.w));
        let (top, bottom) = stretch((cover.y, cover.h), (base.y, base.h), (seen.y, seen.h));
        Some(Rect::new(left, top, right - left, bottom - top))
    }

    /// This sprite coloured as `paint`.
    #[must_use]
    pub const fn painted(self, paint: Paint) -> Self {
        Self { paint, ..self }
    }

    /// The part of `src` that maps to `clip`, as `[x, y, w, h]` in image
    /// pixels (fractions of a pixel when `clip` cuts through a scaled
    /// one). With `flip_x`, the left of `clip` shows the right of `src`.
    #[allow(clippy::cast_precision_loss)] // pixel coordinates are small
    pub fn clipped_src(&self) -> [f32; 4] {
        let Self {
            src, dest, clip, ..
        } = *self;
        let wide = |v: i32| i64::from(v);
        // From `dest`'s edge to `clip`'s, on the side `src` starts from.
        let inset_x = if self.flip_x {
            wide(dest.x) + wide(dest.w) - wide(clip.x) - wide(clip.w)
        } else {
            wide(clip.x) - wide(dest.x)
        };
        let inset_y = wide(clip.y) - wide(dest.y);
        let scale_x = src.w as f32 / dest.w as f32;
        let scale_y = src.h as f32 / dest.h as f32;
        [
            src.x as f32 + inset_x as f32 * scale_x,
            src.y as f32 + inset_y as f32 * scale_y,
            clip.w as f32 * scale_x,
            clip.h as f32 * scale_y,
        ]
    }
}

/// Something drawn on top of the cell grid, in console pixels.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub enum Item {
    /// A solid rectangle.
    Rect(Overlay),
    /// A picture.
    Sprite(Sprite),
}

impl Item {
    /// Under or over the glyphs.
    pub const fn layer(&self) -> Layer {
        match self {
            Item::Rect(o) => o.layer,
            Item::Sprite(s) => s.layer,
        }
    }

    /// The console pixels the item is drawn in: a rectangle's `rect`, a
    /// sprite's `clip`.
    pub const fn visible(&self) -> PxRect {
        match self {
            Item::Rect(o) => o.rect,
            Item::Sprite(s) => s.clip,
        }
    }

    /// What goes of the item, beyond `hole` itself, when the pixels
    /// `hole` are replaced: a sprite's [`overhang`](Sprite::overhang).
    fn overhang(&self, hole: PxRect) -> Option<PxRect> {
        match self {
            Item::Rect(_) => None,
            Item::Sprite(s) => s.overhang(hole),
        }
    }

    /// The item drawn only in `visible`.
    const fn with_visible(self, visible: PxRect) -> Self {
        match self {
            Item::Rect(o) => Item::Rect(Overlay { rect: visible, ..o }),
            Item::Sprite(s) => Item::Sprite(Sprite { clip: visible, ..s }),
        }
    }
}

/// `r` moved by `(dx, dy)`, or `None` if it no longer fits in `i32`.
fn offset(r: PxRect, dx: i64, dy: i64) -> Option<PxRect> {
    let x = i32::try_from(i64::from(r.x) + dx).ok()?;
    let y = i32::try_from(i64::from(r.y) + dy).ok()?;
    Some(Rect::new(x, y, r.w, r.h))
}

/// The part of `r`, moved by `(dx, dy)`, that lies inside `bounds` (which
/// starts at the origin), or `None` if nothing does.
fn offset_clipped(r: PxRect, dx: i64, dy: i64, bounds: PxRect) -> Option<PxRect> {
    // Clip in i64 first: the offset can push `x` past `i32`.
    let (left, top) = (i64::from(r.x) + dx, i64::from(r.y) + dy);
    let (x0, y0) = (left.max(0), top.max(0));
    let x1 = (left + i64::from(r.w)).min(i64::from(bounds.w));
    let y1 = (top + i64::from(r.h)).min(i64::from(bounds.h));
    if x1 <= x0 || y1 <= y0 {
        return None;
    }
    let fit = |v: i64| i32::try_from(v).unwrap_or(0);
    Some(Rect::new(fit(x0), fit(y0), fit(x1 - x0), fit(y1 - y0)))
}

/// The pixel rectangle covered by cells `x..x + w`, `y..y + h` of a
/// `spec` console, or `None` if it doesn't fit in `i32` (only far off any
/// buffer).
fn cells_to_px(spec: ConsoleSpec, x: i64, y: i64, w: i64, h: i64) -> Option<PxRect> {
    let (cw, ch) = (i64::from(spec.cell_w), i64::from(spec.cell_h));
    let fit = |v: i64| i32::try_from(v).ok();
    Some(Rect::new(
        fit(x * cw)?,
        fit(y * ch)?,
        fit(w * cw)?,
        fit(h * ch)?,
    ))
}

/// The parts of `r` outside `hole` (up to four rectangles).
fn subtract(r: Rect, hole: Rect) -> Vec<Rect> {
    let Some(i) = r.intersect(&hole) else {
        return vec![r];
    };
    // Every edge below lies within `r`, so none of this overflows.
    let (r_right, r_bottom) = (r.x + r.w, r.y + r.h);
    let (i_right, i_bottom) = (i.x + i.w, i.y + i.h);
    [
        Rect::new(r.x, r.y, r.w, i.y - r.y),
        Rect::new(r.x, i_bottom, r.w, r_bottom - i_bottom),
        Rect::new(r.x, i.y, i.x - r.x, i.h),
        Rect::new(i_right, i.y, r_right - i_right, i.h),
    ]
    .into_iter()
    .filter(|p| !p.is_empty())
    .collect()
}

/// Border style for [`GlyphBuffer::draw_box`] (game ADR-0012: single for panels,
/// double for focus/modal).
#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub enum BoxStyle {
    /// `┌─┐│└┘`
    Single,
    /// `╔═╗║╚╝`
    Double,
}

impl BoxStyle {
    /// Glyphs: top-left, top-right, bottom-left, bottom-right, horizontal, vertical.
    const fn glyphs(self) -> [char; 6] {
        match self {
            BoxStyle::Single => ['┌', '┐', '└', '┘', '─', '│'],
            BoxStyle::Double => ['╔', '╗', '╚', '╝', '═', '║'],
        }
    }
}

/// The part of `start..start + len` inside `0..limit`.
fn clip_span(start: i32, len: i32, limit: u16) -> core::ops::Range<i32> {
    let limit = i64::from(limit);
    let lo = i64::from(start).clamp(0, limit);
    let hi = (i64::from(start) + i64::from(len)).clamp(lo, limit);
    // Both lie in 0..=u16::MAX, so they fit in i32.
    let fit = |v: i64| i32::try_from(v).unwrap_or(0);
    fit(lo)..fit(hi)
}

/// A grid of [`Cell`]s, row-major.
#[derive(Clone, PartialEq, Debug)]
pub struct GlyphBuffer {
    /// Its size in cells and the size of a cell.
    spec: ConsoleSpec,
    cells: Vec<Cell>,
    /// In drawing order; each one's [`Item::visible`] lies inside
    /// [`pixel_bounds`](Self::pixel_bounds).
    items: Vec<Item>,
    /// The scene behind the see-through cells, if any.
    backdrop: Option<Backdrop>,
}

impl GlyphBuffer {
    /// A `width × height` buffer of 8 × 16-pixel cells
    /// ([`ConsoleSpec::text`]) with every cell set to `fill`.
    pub fn new(width: u16, height: u16, fill: Cell) -> Self {
        Self::with_spec(ConsoleSpec::text(width, height), fill)
    }

    /// A buffer the size of a `spec` console, with every cell set to
    /// `fill`.
    pub fn with_spec(spec: ConsoleSpec, fill: Cell) -> Self {
        Self {
            spec,
            cells: vec![fill; usize::from(spec.cols) * usize::from(spec.rows)],
            items: Vec::new(),
            backdrop: None,
        }
    }

    /// Its size in cells and the size of a cell, in pixels.
    pub fn spec(&self) -> ConsoleSpec {
        self.spec
    }

    /// The scene shown behind the see-through cells, if one is set.
    pub fn backdrop(&self) -> Option<&Backdrop> {
        self.backdrop.as_ref()
    }

    /// Shows `scene` behind the see-through cells of `clip` (console
    /// cells, clipped to the buffer), replacing any backdrop already set:
    /// the scene pixel `origin_px` is at the top-left of `clip`, and each
    /// scene pixel is `zoom` console pixels big (1 to [`MAX_ZOOM`]; other
    /// values are clamped). A `clip` wholly outside the buffer (or empty)
    /// removes the backdrop. A backdrop `scene` has of its own is not drawn.
    pub fn set_backdrop(
        &mut self,
        scene: Rc<GlyphBuffer>,
        clip: Rect,
        origin_px: (f32, f32),
        zoom: u8,
    ) {
        self.backdrop = clip
            .intersect(&self.bounds())
            .map(|clip| Backdrop::new(scene, self.spec, clip, origin_px, zoom));
    }

    /// Removes the backdrop.
    pub fn clear_backdrop(&mut self) {
        self.backdrop = None;
    }

    /// The whole buffer in console pixels.
    pub fn pixel_bounds(&self) -> PxRect {
        let (w, h) = self.spec.px();
        Rect::new(0, 0, w, h)
    }

    /// The rectangles and sprites, in drawing order (within a layer, later
    /// ones on top).
    pub fn items(&self) -> &[Item] {
        &self.items
    }

    /// Only the rectangles, in drawing order.
    pub fn overlays(&self) -> Vec<Overlay> {
        let rect = |item: &Item| match item {
            Item::Rect(o) => Some(*o),
            Item::Sprite(_) => None,
        };
        self.items.iter().filter_map(rect).collect()
    }

    /// Only the sprites, in drawing order.
    pub fn sprites(&self) -> Vec<Sprite> {
        let sprite = |item: &Item| match item {
            Item::Sprite(s) => Some(*s),
            Item::Rect(_) => None,
        };
        self.items.iter().filter_map(sprite).collect()
    }

    /// Adds `overlay`, clipped to the buffer; one wholly outside (or empty)
    /// is dropped.
    pub fn add_overlay(&mut self, overlay: Overlay) {
        if let Some(rect) = overlay.rect.intersect(&self.pixel_bounds()) {
            self.items.push(Item::Rect(Overlay { rect, ..overlay }));
        }
    }

    /// Adds `sprite` with its `clip` clipped to its `dest` and to the
    /// buffer; `src` and `dest` are kept as given. One with nothing to show
    /// (wholly outside, or an empty `src`, `dest` or `clip`) is dropped.
    pub fn add_sprite(&mut self, sprite: Sprite) {
        if sprite.src.is_empty() {
            return;
        }
        let clip = sprite
            .clip
            .intersect(&sprite.dest)
            .and_then(|clip| clip.intersect(&self.pixel_bounds()));
        if let Some(clip) = clip {
            self.items.push(Item::Sprite(Sprite { clip, ..sprite }));
        }
    }

    /// Removes the parts of items over the cells of `rect`: the cells there
    /// were just replaced, and items belong to the cells they were drawn
    /// with. A sprite is split into up to four with smaller `clip`s; one
    /// with a `base` also loses what it shows past the base's edges
    /// beside the cells replaced.
    fn cut_items(&mut self, rect: Rect) {
        let Some(hole) = rect
            .intersect(&self.bounds())
            .and_then(|r| cells_to_px(self.spec, r.x.into(), r.y.into(), r.w.into(), r.h.into()))
        else {
            return;
        };
        self.items = self
            .items
            .iter()
            .flat_map(|item| {
                let overhang = item.overhang(hole);
                subtract(item.visible(), hole)
                    .into_iter()
                    .flat_map(move |part| match overhang {
                        Some(overhang) => subtract(part, overhang),
                        None => vec![part],
                    })
                    .map(move |visible| item.with_visible(visible))
            })
            .collect();
    }

    /// Width in cells.
    pub fn width(&self) -> u16 {
        self.spec.cols
    }

    /// Height in cells.
    pub fn height(&self) -> u16 {
        self.spec.rows
    }

    /// The whole buffer as a rectangle.
    pub fn bounds(&self) -> Rect {
        Rect::new(0, 0, i32::from(self.spec.cols), i32::from(self.spec.rows))
    }

    /// Index of `(x, y)` in `cells`, or `None` if out of bounds.
    fn index(&self, x: i32, y: i32) -> Option<usize> {
        let x = u16::try_from(x).ok().filter(|&x| x < self.spec.cols)?;
        let y = u16::try_from(y).ok().filter(|&y| y < self.spec.rows)?;
        Some(usize::from(y) * usize::from(self.spec.cols) + usize::from(x))
    }

    /// The cell at `(x, y)`, or `None` if out of bounds.
    pub fn get(&self, x: i32, y: i32) -> Option<&Cell> {
        self.index(x, y).map(|i| &self.cells[i])
    }

    /// Mutable access to the cell at `(x, y)`, or `None` if out of bounds.
    fn get_mut(&mut self, x: i32, y: i32) -> Option<&mut Cell> {
        self.index(x, y).map(|i| &mut self.cells[i])
    }

    /// Sets the cell at `(x, y)`; out of bounds is silently ignored.
    pub fn set(&mut self, x: i32, y: i32, cell: Cell) {
        if let Some(c) = self.get_mut(x, y) {
            *c = cell;
        }
    }

    /// Calls `f` on every cell of `rect` that lies inside the buffer.
    fn for_each_in(&mut self, rect: Rect, mut f: impl FnMut(i32, i32, &mut Cell)) {
        for y in clip_span(rect.y, rect.h, self.spec.rows) {
            for x in clip_span(rect.x, rect.w, self.spec.cols) {
                if let Some(c) = self.get_mut(x, y) {
                    f(x, y, c);
                }
            }
        }
    }

    /// Writes `text` left to right from `(x, y)`, one `char` per cell,
    /// clipped to the buffer. Returns how many cells were written.
    pub fn print(&mut self, x: i32, y: i32, text: &str, fg: Rgb, bg: Rgb) -> u16 {
        self.print_with(x, y, text, |c, glyph| *c = Cell::new(glyph, fg, bg))
    }

    /// Like [`print`](Self::print) but keeps each cell's background.
    pub fn print_fg(&mut self, x: i32, y: i32, text: &str, fg: Rgb) -> u16 {
        self.print_with(x, y, text, |c, glyph| {
            c.glyph = glyph;
            c.fg = fg;
        })
    }

    fn print_with(
        &mut self,
        x: i32,
        y: i32,
        text: &str,
        mut f: impl FnMut(&mut Cell, char),
    ) -> u16 {
        let mut written = 0;
        for (i, glyph) in text.chars().enumerate() {
            let Some(cx) = i32::try_from(i).ok().and_then(|i| x.checked_add(i)) else {
                break;
            };
            if cx >= i32::from(self.spec.cols) {
                break;
            }
            if let Some(c) = self.get_mut(cx, y) {
                f(c, glyph);
                written += 1;
            }
        }
        written
    }

    /// Sets every cell of `rect` to `cell`, removing the parts of items
    /// (rectangles and sprites) over it. Filling the whole buffer with a
    /// solid cell (how a frame is cleared) also removes the backdrop.
    pub fn fill_rect(&mut self, rect: Rect, cell: Cell) {
        self.for_each_in(rect, |_, _, c| *c = cell);
        self.cut_items(rect);
        if !cell.see_through && rect.intersect(&self.bounds()) == Some(self.bounds()) {
            self.backdrop = None;
        }
    }

    /// Draws the border of `rect` in `style`; the interior is untouched.
    pub fn draw_box(&mut self, rect: Rect, style: BoxStyle, fg: Rgb, bg: Rgb) {
        if rect.is_empty() {
            return;
        }
        let [tl, tr, bl, br, horiz, vert] = style.glyphs();
        let right = i64::from(rect.x) + i64::from(rect.w) - 1;
        let bottom = i64::from(rect.y) + i64::from(rect.h) - 1;
        self.for_each_in(rect, |x, y, c| {
            let left_edge = x == rect.x;
            let right_edge = i64::from(x) == right;
            let top_edge = y == rect.y;
            let bottom_edge = i64::from(y) == bottom;
            let glyph = match (top_edge, bottom_edge, left_edge, right_edge) {
                (true, _, true, _) => tl,
                (true, _, _, true) => tr,
                (_, true, true, _) => bl,
                (_, true, _, true) => br,
                (true, _, _, _) | (_, true, _, _) => horiz,
                (_, _, true, _) | (_, _, _, true) => vert,
                _ => return,
            };
            *c = Cell::new(glyph, fg, bg);
        });
    }

    /// Tints the background of every cell in `rect` towards `color` by `t`
    /// (`0` = unchanged, `1` = `color`); used for range overlays.
    pub fn blend_bg(&mut self, rect: Rect, color: Rgb, t: f32) {
        self.for_each_in(rect, |_, _, c| c.bg = c.bg.lerp(color, t));
    }

    /// Scales foreground and background of every cell in `rect` by `factor`
    /// (e.g. to dim an inactive portrait). Cells only: rectangles and
    /// sprites over them are not touched (a sprite is dimmed by its
    /// `opacity`).
    pub fn dim(&mut self, rect: Rect, factor: f32) {
        self.for_each_in(rect, |_, _, c| {
            c.fg = c.fg.scale(factor);
            c.bg = c.bg.scale(factor);
        });
    }

    /// Darkens the whole frame towards black by `brightness` (1 = as it
    /// is, 0 = black; clamped, and not a number counts as 1): every cell's
    /// colours, every rectangle's colour, every sprite's `brightness`, and
    /// the backdrop's scene the same way. Unlike [`dim`](Self::dim) it
    /// leaves nothing out (game ADR-0060, game ADR-0075).
    pub fn fade(&mut self, brightness: f32) {
        let brightness = if brightness.is_nan() {
            1.0
        } else {
            brightness.clamp(0.0, 1.0)
        };
        if brightness >= 1.0 {
            return;
        }
        self.dim(self.bounds(), brightness);
        for item in &mut self.items {
            match item {
                Item::Rect(o) => o.color = o.color.scale(brightness),
                Item::Sprite(s) => {
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // 0..=255
                    let darker = (f32::from(s.brightness) * brightness).round() as u8;
                    s.brightness = darker;
                }
            }
        }
        if let Some(backdrop) = &mut self.backdrop {
            backdrop.fade(brightness);
        }
    }

    /// Copies all of `other` so its top-left lands at `(dest_x, dest_y)`,
    /// clipped to this buffer. Items already over the copied area are
    /// removed, and `other`'s items come along, offset and clipped (a
    /// sprite's `dest` moves with it, its `clip` is clipped). Both buffers
    /// have cells of the same size.
    pub fn blit(&mut self, other: &GlyphBuffer, dest_x: i32, dest_y: i32) {
        let target = Rect::new(
            dest_x,
            dest_y,
            i32::from(other.width()),
            i32::from(other.height()),
        );
        self.for_each_in(target, |x, y, c| {
            // In-bounds of `target`, so the source offsets are in `other`.
            if let Some(src) = other.get(x - dest_x, y - dest_y) {
                *c = *src;
            }
        });
        self.cut_items(target);
        let bounds = self.pixel_bounds();
        let (dx, dy) = (
            i64::from(dest_x) * i64::from(self.spec.cell_w),
            i64::from(dest_y) * i64::from(self.spec.cell_h),
        );
        for item in &other.items {
            let Some(visible) = offset_clipped(item.visible(), dx, dy, bounds) else {
                continue;
            };
            match *item {
                Item::Rect(_) => self.items.push(item.with_visible(visible)),
                Item::Sprite(s) => {
                    // A `dest` whose visible part is on the buffer fits.
                    if let Some(dest) = offset(s.dest, dx, dy) {
                        let base = s.base.and_then(|base| offset(base, dx, dy));
                        let moved = Sprite { dest, base, ..s };
                        self.items.push(Item::Sprite(moved).with_visible(visible));
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod backdrop_tests;
#[cfg(test)]
mod sprite_tests;

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    const FG: Rgb = Rgb::new(200, 200, 200);
    const BG: Rgb = Rgb::new(0, 0, 0);
    const RED: Rgb = Rgb::new(255, 0, 0);
    const BLUE: Rgb = Rgb::new(0, 0, 255);
    const BLANK: Cell = Cell::new(' ', FG, BG);

    fn buf(w: u16, h: u16) -> GlyphBuffer {
        GlyphBuffer::new(w, h, BLANK)
    }

    /// The glyphs of row `y` as a string.
    fn row(b: &GlyphBuffer, y: i32) -> String {
        (0..i32::from(b.width()))
            .map(|x| b.get(x, y).unwrap().glyph)
            .collect()
    }

    fn glyph(b: &GlyphBuffer, x: i32, y: i32) -> char {
        b.get(x, y).unwrap().glyph
    }

    #[test]
    fn new_and_size() {
        let b = buf(3, 2);
        assert_eq!((b.width(), b.height()), (3, 2));
        assert_eq!(b.bounds(), Rect::new(0, 0, 3, 2));
        assert_eq!(b.get(2, 1), Some(&BLANK));
    }

    #[test]
    fn get_out_of_bounds() {
        let b = buf(3, 2);
        for (x, y) in [(3, 0), (0, 2), (-1, 0), (0, -1), (i32::MAX, i32::MIN)] {
            assert_eq!(b.get(x, y), None, "({x}, {y})");
        }
    }

    #[test]
    fn set_in_and_out_of_bounds() {
        let mut b = buf(3, 2);
        let c = Cell::new('@', RED, BLUE);
        b.set(1, 1, c);
        assert_eq!(b.get(1, 1), Some(&c));
        assert_eq!(b.get(1, 0), Some(&BLANK));
        let before = b.clone();
        b.set(3, 1, c);
        b.set(-1, 0, c);
        b.set(0, 2, c);
        assert_eq!(b, before);
    }

    #[test]
    fn rect_contains_edges() {
        let r = Rect::new(1, 2, 3, 4);
        assert!(r.contains(1, 2));
        assert!(r.contains(3, 5));
        assert!(!r.contains(0, 2));
        assert!(!r.contains(1, 1));
        assert!(!r.contains(4, 2));
        assert!(!r.contains(1, 6));
        assert!(!Rect::new(0, 0, 0, 5).contains(0, 0));
        assert!(Rect::new(i32::MAX, 0, i32::MAX, 1).contains(i32::MAX, 0));
    }

    #[test]
    fn rect_is_empty() {
        assert!(Rect::new(0, 0, 0, 1).is_empty());
        assert!(Rect::new(0, 0, 1, 0).is_empty());
        assert!(Rect::new(0, 0, -1, 1).is_empty());
        assert!(!Rect::new(0, 0, 1, 1).is_empty());
    }

    #[test]
    fn rect_intersect() {
        let a = Rect::new(0, 0, 4, 4);
        assert_eq!(
            a.intersect(&Rect::new(2, 1, 5, 2)),
            Some(Rect::new(2, 1, 2, 2))
        );
        assert_eq!(
            a.intersect(&Rect::new(-2, -3, 3, 5)),
            Some(Rect::new(0, 0, 1, 2))
        );
        assert_eq!(a.intersect(&Rect::new(4, 0, 1, 1)), None);
        assert_eq!(a.intersect(&Rect::new(0, 4, 1, 1)), None);
        assert_eq!(a.intersect(&Rect::new(1, 1, 0, 1)), None);
        assert_eq!(Rect::new(1, 1, -5, 3).intersect(&a), None);
        assert_eq!(a.intersect(&a), Some(a));
        let huge = Rect::new(i32::MIN, i32::MIN, i32::MAX, i32::MAX);
        assert_eq!(huge.intersect(&a), None);
        let huge = Rect::new(-5, -5, i32::MAX, i32::MAX);
        assert_eq!(huge.intersect(&a), Some(a));
    }

    #[test]
    fn clip_span_cases() {
        assert_eq!(clip_span(2, 3, 10), 2..5);
        assert_eq!(clip_span(-2, 5, 10), 0..3);
        assert_eq!(clip_span(8, 5, 10), 8..10);
        assert_eq!(clip_span(12, 5, 10), 10..10);
        assert_eq!(clip_span(3, -5, 10), 3..3);
        assert_eq!(clip_span(i32::MIN, i32::MAX, 10), 0..0);
        assert_eq!(clip_span(i32::MAX, i32::MAX, u16::MAX), 65535..65535);
    }

    #[test]
    fn print_writes_and_counts() {
        let mut b = buf(5, 2);
        assert_eq!(b.print(1, 1, "abc", RED, BLUE), 3);
        assert_eq!(row(&b, 1), " abc ");
        assert_eq!(b.get(1, 1), Some(&Cell::new('a', RED, BLUE)));
        assert_eq!(row(&b, 0), "     ");
    }

    #[test]
    fn print_clips_right_left_and_rows() {
        let mut b = buf(5, 2);
        assert_eq!(b.print(3, 0, "wxyz", RED, BG), 2);
        assert_eq!(row(&b, 0), "   wx");
        assert_eq!(b.print(-2, 1, "abcd", RED, BG), 2);
        assert_eq!(row(&b, 1), "cd   ");
        let before = b.clone();
        assert_eq!(b.print(0, 2, "abc", RED, BG), 0);
        assert_eq!(b.print(0, -1, "abc", RED, BG), 0);
        assert_eq!(b.print(5, 0, "abc", RED, BG), 0);
        assert_eq!(b.print(i32::MAX, 0, "abc", RED, BG), 0);
        assert_eq!(b.print(i32::MIN, 0, "abc", RED, BG), 0);
        assert_eq!(b, before);
    }

    #[test]
    fn print_one_char_per_cell() {
        let mut b = buf(4, 1);
        assert_eq!(b.print(0, 0, "é♣≈", RED, BG), 3);
        assert_eq!(row(&b, 0), "é♣≈ ");
    }

    #[test]
    fn print_fg_keeps_bg() {
        let mut b = buf(3, 1);
        b.set(1, 0, Cell::new('x', FG, BLUE));
        assert_eq!(b.print_fg(0, 0, "ab", RED), 2);
        assert_eq!(b.get(0, 0), Some(&Cell::new('a', RED, BG)));
        assert_eq!(b.get(1, 0), Some(&Cell::new('b', RED, BLUE)));
        assert_eq!(b.get(2, 0), Some(&BLANK));
        assert_eq!(b.print_fg(2, 0, "yz", RED), 1);
    }

    #[test]
    fn fill_rect_fills_and_clips() {
        let mut b = buf(4, 3);
        let c = Cell::new('#', RED, BLUE);
        b.fill_rect(Rect::new(1, 1, 2, 1), c);
        assert_eq!(row(&b, 0), "    ");
        assert_eq!(row(&b, 1), " ## ");
        assert_eq!(row(&b, 2), "    ");
        b.fill_rect(Rect::new(-1, -1, 2, 2), c);
        assert_eq!(row(&b, 0), "#   ");
        b.fill_rect(Rect::new(3, 2, 10, 10), c);
        assert_eq!(row(&b, 2), "   #");
    }

    #[test]
    fn draw_box_single_and_double() {
        let mut b = buf(5, 4);
        b.draw_box(Rect::new(0, 0, 5, 4), BoxStyle::Single, RED, BLUE);
        assert_eq!(row(&b, 0), "┌───┐");
        assert_eq!(row(&b, 1), "│   │");
        assert_eq!(row(&b, 2), "│   │");
        assert_eq!(row(&b, 3), "└───┘");
        assert_eq!(b.get(0, 0), Some(&Cell::new('┌', RED, BLUE)));
        assert_eq!(b.get(1, 1), Some(&BLANK));
        let mut b = buf(4, 3);
        b.draw_box(Rect::new(1, 0, 3, 3), BoxStyle::Double, RED, BLUE);
        assert_eq!(row(&b, 0), " ╔═╗");
        assert_eq!(row(&b, 1), " ║ ║");
        assert_eq!(row(&b, 2), " ╚═╝");
    }

    #[test]
    fn draw_box_clipped_keeps_corners_of_real_rect() {
        let mut b = buf(3, 3);
        b.draw_box(Rect::new(-1, -1, 3, 3), BoxStyle::Single, RED, BLUE);
        assert_eq!(row(&b, 0), " │ ");
        assert_eq!(row(&b, 1), "─┘ ");
        assert_eq!(row(&b, 2), "   ");
        let mut b = buf(3, 3);
        b.draw_box(Rect::new(1, 1, 5, 5), BoxStyle::Double, RED, BLUE);
        assert_eq!(row(&b, 0), "   ");
        assert_eq!(row(&b, 1), " ╔═");
        assert_eq!(row(&b, 2), " ║ ");
    }

    #[test]
    fn draw_box_corners_each_quadrant() {
        let mut b = buf(2, 2);
        b.draw_box(Rect::new(0, 0, 2, 2), BoxStyle::Single, RED, BLUE);
        assert_eq!(
            [
                glyph(&b, 0, 0),
                glyph(&b, 1, 0),
                glyph(&b, 0, 1),
                glyph(&b, 1, 1)
            ],
            ['┌', '┐', '└', '┘']
        );
    }

    #[test]
    fn draw_box_degenerate() {
        let mut b = buf(3, 3);
        b.draw_box(Rect::new(0, 0, 0, 3), BoxStyle::Single, RED, BLUE);
        assert_eq!(b, buf(3, 3));
        b.draw_box(Rect::new(0, 0, 1, 3), BoxStyle::Single, RED, BLUE);
        assert_eq!(
            [glyph(&b, 0, 0), glyph(&b, 0, 1), glyph(&b, 0, 2)],
            ['┌', '│', '└']
        );
        b.draw_box(Rect::new(0, 0, 3, 1), BoxStyle::Single, RED, BLUE);
        assert_eq!(row(&b, 0), "┌─┐");
    }

    #[test]
    fn blend_bg_tints_only_rect() {
        let mut b = buf(3, 1);
        b.set(0, 0, Cell::new('x', RED, Rgb::new(0, 0, 100)));
        b.blend_bg(Rect::new(0, 0, 2, 1), Rgb::new(100, 0, 0), 0.5);
        assert_eq!(b.get(0, 0), Some(&Cell::new('x', RED, Rgb::new(50, 0, 50))));
        assert_eq!(b.get(1, 0), Some(&Cell::new(' ', FG, Rgb::new(50, 0, 0))));
        assert_eq!(b.get(2, 0), Some(&BLANK));
    }

    #[test]
    fn dim_scales_fg_and_bg() {
        let mut b = GlyphBuffer::new(
            2,
            1,
            Cell::new('a', Rgb::new(200, 100, 50), Rgb::new(10, 20, 40)),
        );
        b.dim(Rect::new(1, 0, 5, 5), 0.5);
        assert_eq!(b.get(0, 0).unwrap().fg, Rgb::new(200, 100, 50));
        assert_eq!(
            b.get(1, 0),
            Some(&Cell::new('a', Rgb::new(100, 50, 25), Rgb::new(5, 10, 20)))
        );
    }

    #[test]
    fn blit_copies_and_clips() {
        let mut src = buf(2, 2);
        src.print(0, 0, "ab", RED, BLUE);
        src.print(0, 1, "cd", RED, BLUE);
        let mut b = buf(3, 3);
        b.blit(&src, 1, 1);
        assert_eq!(row(&b, 0), "   ");
        assert_eq!(row(&b, 1), " ab");
        assert_eq!(row(&b, 2), " cd");
        assert_eq!(b.get(1, 1), Some(&Cell::new('a', RED, BLUE)));
        let mut b = buf(3, 3);
        b.blit(&src, -1, -1);
        assert_eq!(row(&b, 0), "d  ");
        assert_eq!(row(&b, 1), "   ");
        let mut b = buf(3, 3);
        b.blit(&src, 2, 0);
        assert_eq!(row(&b, 0), "  a");
        assert_eq!(row(&b, 1), "  c");
        let before = b.clone();
        b.blit(&src, i32::MAX, i32::MIN);
        b.blit(&src, 3, 0);
        assert_eq!(b, before);
    }

    fn over(x: i32, y: i32, w: i32, h: i32) -> Overlay {
        Overlay::new(Rect::new(x, y, w, h), RED, Layer::Over)
    }

    #[test]
    fn overlays_are_added_and_clipped() {
        let mut b = buf(3, 2);
        assert_eq!(b.pixel_bounds(), Rect::new(0, 0, 24, 32));
        assert!(b.overlays().is_empty());
        b.add_overlay(over(2, 14, 16, 2));
        b.add_overlay(Overlay::new(Rect::new(-4, 30, 10, 5), BLUE, Layer::Under));
        b.add_overlay(over(20, 0, 10, 1));
        b.add_overlay(over(24, 0, 1, 1));
        b.add_overlay(over(0, 0, 0, 1));
        b.add_overlay(over(i32::MAX, i32::MIN, i32::MAX, 1));
        assert_eq!(
            b.overlays(),
            [
                over(2, 14, 16, 2),
                Overlay::new(Rect::new(0, 30, 6, 2), BLUE, Layer::Under),
                over(20, 0, 4, 1),
            ]
        );
        assert_eq!(Layer::Under.name(), "under");
        assert_eq!(Layer::Over.name(), "over");
    }

    #[test]
    fn blit_offsets_and_clips_overlays() {
        let mut src = buf(2, 2);
        src.add_overlay(over(0, 14, 16, 2));
        src.add_overlay(over(8, 16, 8, 16));
        let mut b = buf(3, 3);
        b.blit(&src, 1, 1);
        assert_eq!(b.overlays(), [over(8, 30, 16, 2), over(16, 32, 8, 16)]);
        let mut b = buf(3, 3);
        b.blit(&src, -1, -1);
        // First: x 0..16 → -8..8, y 14..16 → -2..0: gone. Second: 0..8, 0..16.
        assert_eq!(b.overlays(), [over(0, 0, 8, 16)]);
        let mut b = buf(3, 3);
        b.blit(&src, 2, 2);
        assert_eq!(b.overlays(), [over(16, 46, 8, 2)]);
        let mut b = buf(3, 3);
        b.blit(&src, i32::MAX, i32::MIN);
        b.blit(&src, i32::MIN, i32::MAX);
        assert!(b.overlays().is_empty());
    }

    #[test]
    fn replacing_cells_removes_their_overlays() {
        let mut b = buf(4, 2);
        b.add_overlay(over(0, 14, 32, 2));
        b.add_overlay(Overlay::new(Rect::new(0, 0, 32, 32), BLUE, Layer::Under));
        b.fill_rect(Rect::new(1, 0, 2, 1), BLANK);
        assert_eq!(
            b.overlays(),
            [
                over(0, 14, 8, 2),
                over(24, 14, 8, 2),
                Overlay::new(Rect::new(0, 16, 32, 16), BLUE, Layer::Under),
                Overlay::new(Rect::new(0, 0, 8, 16), BLUE, Layer::Under),
                Overlay::new(Rect::new(24, 0, 8, 16), BLUE, Layer::Under),
            ]
        );
        // Off-buffer and empty fills leave overlays alone.
        let before = b.clone();
        b.fill_rect(Rect::new(5, 0, 2, 2), BLANK);
        b.fill_rect(Rect::new(0, 0, 0, 2), BLANK);
        assert_eq!(b, before);
        // Blitting over cells replaces their overlays with the source's.
        let src = buf(1, 1);
        b.blit(&src, 0, 1);
        assert!(b.overlays().iter().all(|o| o.rect.y < 16 || o.rect.x >= 8));
        b.fill_rect(b.bounds(), BLANK);
        assert!(b.overlays().is_empty());
    }

    #[test]
    fn subtract_cases() {
        let r = Rect::new(0, 0, 10, 10);
        assert_eq!(subtract(r, Rect::new(20, 0, 5, 5)), [r]);
        assert!(subtract(r, Rect::new(-1, -1, 20, 20)).is_empty());
        assert_eq!(
            subtract(r, Rect::new(2, 3, 4, 5)),
            [
                Rect::new(0, 0, 10, 3),
                Rect::new(0, 8, 10, 2),
                Rect::new(0, 3, 2, 5),
                Rect::new(6, 3, 4, 5),
            ]
        );
        let pc = ConsoleSpec::text(100, 32);
        assert_eq!(cells_to_px(pc, 1, 2, 3, 4), Some(Rect::new(8, 32, 24, 64)));
        assert_eq!(cells_to_px(pc, i64::from(i32::MAX), 0, 1, 1), None);
        let small = ConsoleSpec {
            cell_w: 4,
            cell_h: 6,
            ..pc
        };
        assert_eq!(
            cells_to_px(small, 1, 2, 3, 4),
            Some(Rect::new(4, 12, 12, 24))
        );
    }

    fn marked(w: u16, h: u16) -> GlyphBuffer {
        let mut b = buf(w, h);
        for y in 0..i32::from(h) {
            for x in 0..i32::from(w) {
                let g = char::from_u32(0x100 + u32::try_from(y * 64 + x).unwrap()).unwrap();
                b.set(x, y, Cell::new(g, FG, BG));
            }
        }
        b
    }

    /// Asserts every cell outside `target` is unchanged.
    fn unchanged_outside(before: &GlyphBuffer, after: &GlyphBuffer, target: Rect) {
        for y in 0..i32::from(before.height()) {
            for x in 0..i32::from(before.width()) {
                if !target.contains(x, y) {
                    assert_eq!(before.get(x, y), after.get(x, y), "cell ({x}, {y}) changed");
                }
            }
        }
    }

    fn coord() -> impl Strategy<Value = i32> {
        prop_oneof![-20..40i32, any::<i32>()]
    }

    fn rect() -> impl Strategy<Value = Rect> {
        (coord(), coord(), coord(), coord()).prop_map(|(x, y, w, h)| Rect::new(x, y, w, h))
    }

    proptest! {
        #[test]
        fn print_only_touches_its_row_span(
            w in 0u16..30, h in 0u16..10, x in coord(), y in coord(), text in "\\PC{0,40}",
        ) {
            let before = marked(w, h);
            let mut after = before.clone();
            let n = after.print(x, y, &text, RED, BLUE);
            let len = i32::try_from(text.chars().count()).unwrap();
            let target = Rect::new(x, y, len, 1);
            unchanged_outside(&before, &after, target);
            let inside = target.intersect(&before.bounds()).map_or(0, |r| r.w);
            prop_assert_eq!(i32::from(n), inside);
        }

        #[test]
        fn fill_rect_only_touches_rect(w in 0u16..30, h in 0u16..10, r in rect()) {
            let before = marked(w, h);
            let mut after = before.clone();
            let c = Cell::new('#', RED, BLUE);
            after.fill_rect(r, c);
            unchanged_outside(&before, &after, r);
            if let Some(i) = r.intersect(&before.bounds()) {
                prop_assert_eq!(after.get(i.x, i.y), Some(&c));
                prop_assert_eq!(after.get(i.x + i.w - 1, i.y + i.h - 1), Some(&c));
            }
        }

        #[test]
        fn blit_only_touches_dest(
            w in 0u16..30, h in 0u16..10, sw in 0u16..20, sh in 0u16..8, x in coord(), y in coord(),
        ) {
            let before = marked(w, h);
            let mut after = before.clone();
            let src = GlyphBuffer::new(sw, sh, Cell::new('s', RED, BLUE));
            after.blit(&src, x, y);
            unchanged_outside(&before, &after, Rect::new(x, y, i32::from(sw), i32::from(sh)));
        }

        #[test]
        fn other_primitives_only_touch_rect(w in 0u16..30, h in 0u16..10, r in rect(), t in any::<f32>()) {
            let before = marked(w, h);
            let mut after = before.clone();
            after.draw_box(r, BoxStyle::Double, RED, BLUE);
            after.blend_bg(r, RED, t);
            after.dim(r, t);
            unchanged_outside(&before, &after, r);
        }

        #[test]
        fn overlays_stay_inside_the_buffer(
            w in 0u16..30, h in 0u16..10, r in rect(), x in coord(), y in coord(), f in rect(),
        ) {
            let mut src = buf(4, 3);
            src.add_overlay(over(r.x, r.y, r.w, r.h));
            let mut b = buf(w, h);
            b.add_overlay(over(r.x, r.y, r.w, r.h));
            b.blit(&src, x, y);
            b.fill_rect(f, BLANK);
            let px = b.pixel_bounds();
            for o in b.overlays() {
                prop_assert_eq!(o.rect.intersect(&px), Some(o.rect));
            }
        }

        #[test]
        fn get_and_set_never_panic(w in 0u16..30, h in 0u16..10, x in coord(), y in coord()) {
            let mut b = marked(w, h);
            let inside = b.bounds().contains(x, y);
            prop_assert_eq!(b.get(x, y).is_some(), inside);
            b.set(x, y, BLANK);
        }
    }
}
