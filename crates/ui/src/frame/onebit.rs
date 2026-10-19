//! The one-bit renderer (game ADR-0074): draws a [`GlyphBuffer`] the size of
//! the Playdate's screen (50 × 15 cells of 8 × 16 pixels,
//! [`ConsoleSpec::text`](crate::frame::ConsoleSpec::text)) into a 400 × 240
//! one-bit [`Frame`] laid out as the Playdate's own frame buffer, so its
//! front end copies it to the screen as it is.
//!
//! Colours become [`Ink`]s: a colour that is an ink's reserved
//! [code](Ink::code) (the Playdate's palette is made of them) is drawn in
//! that ink, any other one white or black by its brightness. Patterns are
//! fixed to the screen's pixels, so neighbouring tiles line up.
//!
//! The order: the white ground; a backdrop's scene in its window
//! (game ADR-0048), drawn the same way; then the cell backgrounds, `Under`
//! items, the white edges of [`Ink::BlackEdged`] glyphs, the glyphs,
//! `Over` items, and last the [`Ink::Invert`] rectangles, which flip what
//! is under them. Pictures are one-bit [`Image`]s (`docs/design/playdate.md`,
//! *One-bit pictures*). Nothing here allocates.
//!
//! Made for the device's speed (ticket 1055, game ADR-0077): 32-bit
//! coordinates, a pattern's byte looked up, whole bytes written whole,
//! only the cells near the part being drawn visited, and
//! [`render_changed`] redraws only where a frame differs from the last.
//! [`reference`] is the plain renderer this one must equal.

use core::ops::Range;

use shuyi_assets::bitmap::Bitmap;
use shuyi_assets::font::FALLBACK_GLYPH;
use shuyi_assets::font::FontAtlasDef;
use shuyi_assets::image::ImageId;
use shuyi_assets::ink::Ink;

use crate::frame::color::Rgb;
use crate::frame::glyph_buffer::{GlyphBuffer, Item, Layer, Paint, PxRect, Sprite};

#[cfg(any(test, feature = "onebit-reference"))]
pub mod reference;

/// The screen's width in pixels.
pub const WIDTH: i32 = 400;
/// The screen's height in pixels.
pub const HEIGHT: i32 = 240;
/// Bytes per row of a [`Frame`]: the Playdate's stride (50 bytes of
/// pixels, 2 unused).
pub const ROW_BYTES: usize = 52;
/// Bytes in a [`Frame`].
pub const FRAME_BYTES: usize = ROW_BYTES * HEIGHT as usize;

/// A one-bit picture of the screen in the Playdate's layout: row after
/// row of [`ROW_BYTES`] bytes, a byte's highest bit its left pixel, a set
/// bit white.
pub type Frame = [u8; FRAME_BYTES];

/// Whether pixel `(x, y)` of `frame` is white; off the screen, `false`.
pub fn is_white(frame: &Frame, x: i32, y: i32) -> bool {
    if !(0..WIDTH).contains(&x) || !(0..HEIGHT).contains(&y) {
        return false;
    }
    let byte = frame[byte_index(x, y)];
    byte >> (7 - (x & 7)) & 1 == 1
}

/// The ink a colour is drawn in: the ink it is the [code](Ink::code) of,
/// or else white if it is at least half bright (in the eye's weighting of
/// red, green and blue), black if not.
pub fn ink_of(rgb: Rgb) -> Ink {
    Ink::from_code([rgb.r, rgb.g, rgb.b]).unwrap_or_else(|| {
        let luma = 299 * u32::from(rgb.r) + 587 * u32::from(rgb.g) + 114 * u32::from(rgb.b);
        if luma >= 128 * 1000 {
            Ink::White
        } else {
            Ink::Black
        }
    })
}

/// Whether `ink` makes screen pixel `(x, y)` white. [`Ink::BlackEdged`]
/// and [`Ink::Invert`] are black here: the edge is drawn around glyphs,
/// and only rectangles flip.
pub const fn ink_is_white(ink: Ink, x: i32, y: i32) -> bool {
    match ink {
        Ink::White => true,
        Ink::Black | Ink::BlackEdged | Ink::Invert => false,
        // Black where x + y is even.
        Ink::Grey => x.rem_euclid(2) != y.rem_euclid(2),
        // Black every fourth pixel of every other row, the rows staggered.
        Ink::Dots => !(y.rem_euclid(2) == 0 && x.rem_euclid(4) == y.rem_euclid(4)),
        // Black where x − y is a multiple of 4: lines down to the right.
        Ink::Diagonals => (x - y).rem_euclid(4) != 0,
        // A checker of 2 × 2 squares: a 1-pixel line of it is dashed.
        Ink::Dashes => x.div_euclid(2).rem_euclid(2) != y.div_euclid(2).rem_euclid(2),
    }
}

/// The eight pixels of screen row `y` that `ink` paints in a byte of a
/// [`Frame`]: every pattern repeats within 8 pixels across, so the byte is
/// the same all along the row.
const fn ink_byte(ink: Ink, y: i32) -> u8 {
    let mut byte = 0;
    let mut x = 0;
    while x < 8 {
        byte = (byte << 1) + if ink_is_white(ink, x, y) { 1 } else { 0 };
        x += 1;
    }
    byte
}

/// Each ink's [`ink_byte`] for the four rows its pattern repeats within
/// (game ADR-0074), by `ink as usize`.
const PATTERNS: [[u8; 4]; Ink::ALL.len()] = {
    let mut table = [[0; 4]; Ink::ALL.len()];
    let mut i = 0;
    while i < Ink::ALL.len() {
        let ink = Ink::ALL[i];
        let (mut row, mut y) = (0, 0);
        while row < 4 {
            table[ink as usize][row] = ink_byte(ink, y);
            (row, y) = (row + 1, y + 1);
        }
        i += 1;
    }
    table
};

/// [`ink_byte`], looked up.
#[allow(clippy::cast_sign_loss)] // 0..4
const fn pattern(ink: Ink, y: i32) -> u8 {
    PATTERNS[ink as usize][(y & 3) as usize]
}

/// The index in a [`Frame`] of the byte holding screen pixel `(x, y)`,
/// which must be on the screen.
#[allow(clippy::cast_sign_loss)] // on the screen
const fn byte_index(x: i32, y: i32) -> usize {
    y as usize * ROW_BYTES + (x >> 3) as usize
}

/// Where [`Font`] keeps that a glyph has no rows.
const NO_ROWS: u32 = u32::MAX;

/// The font as the one-bit renderer reads it: the glyph map and the
/// one-bit atlas the game gives.
#[derive(Copy, Clone, Debug)]
pub struct Font<'a> {
    atlas: &'a FontAtlasDef,
    bits: &'a [u8],
    /// Where in `bits` each ASCII character's rows start (the fallback
    /// glyph's if the font lacks it), or [`NO_ROWS`]: the glyph map is a
    /// tree, slow to search on the device, and nearly all text is ASCII.
    ascii: [u32; 128],
}

impl<'a> Font<'a> {
    /// The font of glyph map `atlas` and one-bit atlas `bits`. Fails if a
    /// cell is wider than 8 pixels or `bits` lacks a glyph of the map.
    pub fn new(atlas: &'a FontAtlasDef, bits: &'a [u8]) -> Result<Self, &'static str> {
        if atlas.cell_w > 8 {
            // check-text: not player text (a build problem for the log)
            return Err("the one-bit font has cells at most 8 pixels wide");
        }
        let rows = atlas.cell_h as usize;
        let needed = atlas.glyphs.values().map(|&i| (i as usize + 1) * rows);
        if needed.max().unwrap_or(0) > bits.len() {
            // check-text: not player text (a build problem for the log)
            return Err("the one-bit atlas is shorter than its glyph map");
        }
        let mut ascii = [NO_ROWS; 128];
        for (code, start) in (0u8..).zip(&mut ascii) {
            let found = rows_start(atlas, char::from(code));
            *start = found
                .and_then(|at| u32::try_from(at).ok())
                .unwrap_or(NO_ROWS);
        }
        Ok(Self { atlas, bits, ascii })
    }

    /// The pixel rows of `glyph` (a set bit is a pixel of the glyph), or
    /// of the fallback glyph if the font lacks it.
    pub fn rows(&self, glyph: char) -> Option<&'a [u8]> {
        let start = match self.ascii.get(glyph as usize) {
            Some(&NO_ROWS) => return None,
            Some(&start) => start as usize,
            None => rows_start(self.atlas, glyph)?,
        };
        self.bits.get(start..start + self.atlas.cell_h as usize)
    }

    /// How many screen rows a glyph drawn `scale` times as big covers.
    fn height(&self, scale: i32) -> i32 {
        i32::try_from(self.atlas.cell_h).map_or(i32::MAX, |h| h.saturating_mul(scale))
    }
}

/// Where in a one-bit atlas the rows of `glyph` (or of the fallback
/// glyph, if `atlas` lacks it) start.
fn rows_start(atlas: &FontAtlasDef, glyph: char) -> Option<usize> {
    let index = atlas
        .glyphs
        .get(&glyph)
        .or_else(|| atlas.glyphs.get(&FALLBACK_GLYPH))?;
    Some(*index as usize * atlas.cell_h as usize)
}

/// A one-bit picture: `height` rows of [`stride`](Image::stride) bytes, a
/// byte's highest bit its left pixel, a set bit white; and optionally a
/// mask of the same shape, a set bit where the picture is drawn (the
/// Playdate's own bitmap layout). Without a mask every pixel is drawn.
///
/// Its **source size** is the size of the image a sprite names
/// ([`ImageTable`](shuyi_assets::image::ImageTable)), whose pixels the
/// sprite's `src` counts in. It is the picture's own size unless the
/// picture was enlarged before it was dithered ([`shuyi_assets::bitmap`]):
/// then each source pixel covers several of the picture's.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Image<'a> {
    width: u16,
    height: u16,
    source_width: u16,
    source_height: u16,
    data: &'a [u8],
    mask: Option<&'a [u8]>,
}

impl<'a> Image<'a> {
    /// The picture, its source size its own, or `None` if `data` (or
    /// `mask`) is too short for it.
    pub fn new(width: u16, height: u16, data: &'a [u8], mask: Option<&'a [u8]>) -> Option<Self> {
        let size = Self::stride(width) * usize::from(height);
        let fits = |bytes: &[u8]| bytes.len() >= size;
        (fits(data) && mask.is_none_or(fits)).then_some(Self {
            width,
            height,
            source_width: width,
            source_height: height,
            data,
            mask,
        })
    }

    /// Style `style` of one-bit picture file `bitmap`, with its mask and
    /// source size, or `None` if it has no such style.
    pub fn from_bitmap(bitmap: &Bitmap<'a>, style: usize) -> Option<Self> {
        let data = bitmap.style(style)?;
        let image = Self::new(bitmap.width(), bitmap.height(), data, bitmap.mask())?;
        let (width, height) = bitmap.source_size();
        Some(image.with_source(width, height))
    }

    /// The same picture standing for an image `width` × `height` pixels
    /// (its source size); a side of 0 is left as it was.
    #[must_use]
    pub fn with_source(self, width: u16, height: u16) -> Self {
        Self {
            source_width: if width == 0 { self.source_width } else { width },
            source_height: if height == 0 {
                self.source_height
            } else {
                height
            },
            ..self
        }
    }

    /// Bytes per row of a picture `width` pixels wide.
    pub const fn stride(width: u16) -> usize {
        (width as usize).div_ceil(8)
    }

    /// Pixel `(x, y)`: `Some(true)` white, `Some(false)` black, `None`
    /// see-through or off the picture.
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)] // checked first
    fn pixel(&self, x: i64, y: i64) -> Option<bool> {
        if !(0..i64::from(self.width)).contains(&x) || !(0..i64::from(self.height)).contains(&y) {
            return None;
        }
        let at = y as usize * Self::stride(self.width) + (x >> 3) as usize;
        let bit = |bytes: &[u8]| bytes[at] >> (7 - (x & 7)) & 1 == 1;
        match self.mask {
            Some(mask) if !bit(mask) => None,
            _ => Some(bit(self.data)),
        }
    }

    /// The picture's pixel along one side (`size` long, its source side
    /// `source`) under the centre of pixel `d` of a `dest`-long stretch of
    /// the source's `start..start + len`: nearest sampling, as `app` does.
    /// With the source size the picture's own, it is `start + (2d + 1) ×
    /// len / 2dest`.
    fn sample(d: i64, start: i64, len: i64, dest: i64, size: u16, source: u16) -> i64 {
        let (size, source) = (i64::from(size), i64::from(source.max(1)));
        (start * 2 * dest + (2 * d + 1) * len) * size / (2 * dest * source)
    }
}

/// Where the renderer finds a sprite's one-bit picture.
pub trait Images {
    /// The picture of `id`, or `None` if there is none (the sprite is
    /// drawn as a grey rectangle).
    fn image(&self, id: ImageId) -> Option<Image<'_>>;
}

/// No pictures at all.
impl Images for () {
    fn image(&self, _: ImageId) -> Option<Image<'_>> {
        None
    }
}

/// The steps of a render, in order, for timing them on the device
/// ([`render_marked`]).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Phase {
    /// The white ground.
    Ground,
    /// A backdrop's scene, all its steps.
    Backdrop,
    /// The cell backgrounds.
    Backgrounds,
    /// The `Under` items.
    Under,
    /// The white edges of edged glyphs.
    Edges,
    /// The glyphs.
    Glyphs,
    /// The `Over` items.
    Over,
    /// The invert rectangles.
    Invert,
}

impl Phase {
    /// Every phase, in drawing order.
    pub const ALL: [Self; 8] = [
        Self::Ground,
        Self::Backdrop,
        Self::Backgrounds,
        Self::Under,
        Self::Edges,
        Self::Glyphs,
        Self::Over,
        Self::Invert,
    ];

    /// Its name in a log.
    // check-text: not player text (the bench build's log)
    pub const fn name(self) -> &'static str {
        match self {
            Self::Ground => "ground",
            Self::Backdrop => "backdrop",
            Self::Backgrounds => "backgrounds",
            Self::Under => "under items",
            Self::Edges => "edges",
            Self::Glyphs => "glyphs",
            Self::Over => "over items",
            Self::Invert => "invert",
        }
    }
}

/// Draws `buf` into `out`, from its top-left corner; whatever lies past
/// 400 × 240 pixels is cut off. See the [module](self) for the order.
pub fn render(buf: &GlyphBuffer, font: &Font<'_>, images: &impl Images, out: &mut Frame) {
    render_marked(buf, font, images, out, &mut |_| {});
}

/// [`render`], calling `mark` as each [`Phase`] ends (the Playdate's
/// bench build times them).
pub fn render_marked(
    buf: &GlyphBuffer,
    font: &Font<'_>,
    images: &impl Images,
    out: &mut Frame,
    mark: &mut impl FnMut(Phase),
) {
    // The two unused bytes of each row too.
    out.fill(u8::MAX);
    mark(Phase::Ground);
    draw(buf, font, images, out, Clip::SCREEN, mark);
}

/// Makes `out`, which holds `old` as [`render`] drew it, hold `new` as
/// `render` would draw it, redrawing only the 8 × 16 pixel tiles of the
/// screen where they may differ: around a changed cell (one pixel more
/// for a glyph's edge), under a changed item (the old and the new one),
/// and the window of a changed backdrop.
pub fn render_changed(
    old: &GlyphBuffer,
    new: &GlyphBuffer,
    font: &Font<'_>,
    images: &impl Images,
    out: &mut Frame,
) {
    if old.spec() != new.spec() {
        render(new, font, images, out);
        return;
    }
    let mut dirty = Tiles::default();
    let place = Place::ORIGIN;
    let reach = Reach::of(new, font, 1);
    for y in 0..i32::from(new.height()) {
        for x in 0..i32::from(new.width()) {
            if old.get(x, y) != new.get(x, y) {
                dirty.add(reach.around(place, x, y));
            }
        }
    }
    let (before, after) = (old.items(), new.items());
    for i in 0..before.len().max(after.len()) {
        let (was, is) = (before.get(i), after.get(i));
        if was != is {
            for item in was.into_iter().chain(is) {
                dirty.add(place.screen(item.visible()));
            }
        }
    }
    if old.backdrop() != new.backdrop() {
        for backdrop in old.backdrop().into_iter().chain(new.backdrop()) {
            dirty.add(place.screen(backdrop.clip_px()));
        }
    }
    for region in dirty.regions() {
        Target {
            frame: &mut *out,
            clip: region,
        }
        .fill(region, Pen::Ink(Ink::White));
        draw(new, font, images, out, region, &mut |_| {});
    }
}

/// The 8 × 16 pixel tiles of the screen (50 × 15) that must be redrawn,
/// a bit each.
#[derive(Default)]
struct Tiles([u64; Tiles::ROWS]);

impl Tiles {
    const W: i32 = 8;
    const H: i32 = 16;
    const ROWS: usize = (HEIGHT / Self::H) as usize;

    /// Marks every tile `rect` touches on the screen.
    #[allow(clippy::cast_sign_loss)] // on the screen
    fn add(&mut self, rect: Clip) {
        let r = rect.within(Clip::SCREEN);
        if r.is_empty() {
            return;
        }
        let (first, last) = (r.x0 / Self::W, (r.x1 - 1) / Self::W);
        let columns = (u64::MAX >> (63 - last)) & (u64::MAX << first);
        for row in &mut self.0[(r.y0 / Self::H) as usize..=((r.y1 - 1) / Self::H) as usize] {
            *row |= columns;
        }
    }

    /// Each run of marked tiles along a row of them, as screen pixels.
    /// Walks a fixed 64 columns a row, so it always ends.
    #[allow(clippy::cast_possible_wrap, clippy::cast_possible_truncation)] // tiles
    fn regions(&self) -> impl Iterator<Item = Clip> + '_ {
        (0i32..).zip(self.0).flat_map(|(row, bits)| {
            // A run starts at a marked tile whose left neighbour isn't.
            let starts = bits & !(bits << 1);
            (0..u64::BITS)
                .filter(move |&column| starts >> column & 1 == 1)
                .map(move |first| {
                    let len = (bits >> first).trailing_ones();
                    Clip {
                        x0: first as i32 * Self::W,
                        y0: row * Self::H,
                        x1: (first + len) as i32 * Self::W,
                        y1: (row + 1) * Self::H,
                    }
                })
        })
    }
}

/// Draws `buf` over the white ground of `region` of `out`: its backdrop's
/// scene, then its own layers.
fn draw(
    buf: &GlyphBuffer,
    font: &Font<'_>,
    images: &impl Images,
    out: &mut Frame,
    region: Clip,
    mark: &mut impl FnMut(Phase),
) {
    let place = Place::ORIGIN;
    if let Some(backdrop) = buf.backdrop() {
        let window = place.screen(backdrop.clip_px());
        let clip = region.within(window);
        if !clip.is_empty() {
            let (dx, dy) = backdrop.scene_offset(1);
            let mut scene = Target {
                frame: &mut *out,
                clip,
            };
            let at = Place {
                x: window.x0 + offset(dx),
                y: window.y0 + offset(dy),
                scale: i32::from(backdrop.zoom()),
            };
            draw_layers(&mut scene, backdrop.scene(), at, font, images, &mut |_| {});
        }
    }
    mark(Phase::Backdrop);
    let mut target = Target {
        frame: out,
        clip: region,
    };
    draw_layers(&mut target, buf, place, font, images, mark);
}

/// A scene's pixel offset, held within ±2³⁰: a scene that far out is off
/// the screen anyway, however big (a buffer's pixels number less).
#[allow(clippy::cast_possible_truncation)] // held within i32 first
fn offset(d: i64) -> i32 {
    d.clamp(-(1 << 30), 1 << 30) as i32
}

/// A rectangle of screen pixels, `x0..x1` × `y0..y1`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct Clip {
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
}

impl Clip {
    /// The whole screen.
    const SCREEN: Self = Self {
        x0: 0,
        y0: 0,
        x1: WIDTH,
        y1: HEIGHT,
    };

    /// The part of this inside `other` (empty if none is).
    fn within(self, other: Self) -> Self {
        Self {
            x0: self.x0.max(other.x0),
            y0: self.y0.max(other.y0),
            x1: self.x1.min(other.x1),
            y1: self.y1.min(other.y1),
        }
    }

    /// Whether it has no pixels.
    const fn is_empty(self) -> bool {
        self.x0 >= self.x1 || self.y0 >= self.y1
    }
}

/// Where a buffer is drawn: its top-left pixel at screen `(x, y)`, each of
/// its pixels `scale × scale` screen pixels.
#[derive(Copy, Clone, Debug)]
struct Place {
    x: i32,
    y: i32,
    scale: i32,
}

impl Place {
    /// The screen's own buffer: from the top-left, unscaled.
    const ORIGIN: Self = Self {
        x: 0,
        y: 0,
        scale: 1,
    };

    /// The screen pixels of buffer pixels `r`.
    fn screen(self, r: PxRect) -> Clip {
        let x0 = self.x + r.x * self.scale;
        let y0 = self.y + r.y * self.scale;
        Clip {
            x0,
            y0,
            x1: x0 + r.w * self.scale,
            y1: y0 + r.h * self.scale,
        }
    }
}

/// How far a buffer's cell, drawn at some scale, may reach on the screen:
/// its size, or its glyph's if that is bigger, and a pixel all round for
/// the glyph's white edge.
#[derive(Copy, Clone)]
struct Reach {
    /// A cell's size on the screen.
    cell_w: i32,
    cell_h: i32,
    /// How far right of and below its top-left a cell draws, edge and all.
    right: i32,
    below: i32,
}

impl Reach {
    fn of(buf: &GlyphBuffer, font: &Font<'_>, scale: i32) -> Self {
        let spec = buf.spec();
        let (cell_w, cell_h) = (
            i32::from(spec.cell_w) * scale,
            i32::from(spec.cell_h) * scale,
        );
        Self {
            cell_w,
            cell_h,
            right: cell_w.max(8 * scale) + 1,
            below: cell_h.max(font.height(scale)).saturating_add(1),
        }
    }

    /// The screen pixels cell `(x, y)` may draw on, at `at`.
    fn around(self, at: Place, x: i32, y: i32) -> Clip {
        let (x0, y0) = (at.x + x * self.cell_w, at.y + y * self.cell_h);
        Clip {
            x0: x0 - 1,
            y0: y0 - 1,
            x1: x0.saturating_add(self.right),
            y1: y0.saturating_add(self.below),
        }
    }

    /// The cells, of `count` along a line from `origin` in steps of
    /// `size`, that may draw on pixels `lo..hi` of that line, reaching
    /// `reach` past their start.
    fn span(lo: i32, hi: i32, origin: i32, size: i32, reach: i32, count: u16) -> Range<i32> {
        if size <= 0 {
            return 0..0;
        }
        let first = (lo - origin).saturating_sub(reach).div_euclid(size);
        let end = (hi - origin).saturating_add(size).div_euclid(size);
        first.max(0)..end.min(i32::from(count))
    }
}

/// A frame and the part of it that may be drawn on.
struct Target<'f> {
    frame: &'f mut Frame,
    clip: Clip,
}

/// What a run of pixels does to the frame.
#[derive(Copy, Clone)]
enum Pen {
    /// Paints them in the ink.
    Ink(Ink),
    /// Flips them.
    Flip,
}

impl Pen {
    /// Byte `byte` with this pen applied to the pixels `mask` selects, on
    /// screen row `y`.
    const fn apply(self, byte: u8, mask: u8, y: i32) -> u8 {
        match self {
            // The ink's bits where `mask` is set, the frame's elsewhere.
            Pen::Ink(ink) => byte ^ ((byte ^ pattern(ink, y)) & mask),
            Pen::Flip => byte ^ mask,
        }
    }
}

impl Target<'_> {
    /// The pixels of a run from `x` (as [`run`](Self::run) takes it) if they
    /// are one whole byte of the frame inside the clip: a glyph's row at a
    /// whole cell, written straight.
    #[allow(clippy::cast_possible_truncation)] // a byte's bits
    fn whole_byte(&self, x: i32, bits: u64) -> Option<u8> {
        let one_byte = x.trailing_zeros() >= 3 && bits << 8 == 0;
        let inside = x >= self.clip.x0 && x + 8 <= self.clip.x1;
        (one_byte && inside).then_some((bits >> 56) as u8)
    }

    /// Applies `pen` to the pixels of screen row `y` from `x` on that
    /// `bits` selects (its highest bit is pixel `x`), inside the clip.
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_possible_wrap
    )] // a byte's bits; offsets 0..=64; screen bytes
    fn run(&mut self, y: i32, x: i32, bits: u64, pen: Pen) {
        let clip = self.clip;
        if bits == 0 || y < clip.y0 || y >= clip.y1 {
            return;
        }
        if let Some(mask) = self.whole_byte(x, bits) {
            let byte = &mut self.frame[byte_index(x, y)];
            *byte = pen.apply(*byte, mask, y);
            return;
        }
        let (lo, hi) = (x.max(clip.x0), x.saturating_add(64).min(clip.x1));
        if lo >= hi {
            return;
        }
        // Only the bits of pixels `lo..hi`.
        let left = u64::MAX >> (lo - x);
        let right = u64::MAX.checked_shr((hi - x) as u32).unwrap_or(0);
        let bits = bits & left & !right;
        let row = y as usize * ROW_BYTES;
        for column in (lo >> 3)..=((hi - 1) >> 3) {
            // The bits of the byte's 8 pixels, whose first lies `shift`
            // pixels right of `x` (7 left of it at most).
            let shift = column * 8 - x;
            let mask = if shift >= 0 {
                (bits << shift >> 56) as u8
            } else {
                (bits >> 56 >> -shift) as u8
            };
            let byte = &mut self.frame[row + column as usize];
            *byte = pen.apply(*byte, mask, y);
        }
    }

    /// Applies `pen` to every pixel of `rect` inside the clip: the bytes
    /// at its ends in part, those between whole.
    #[allow(clippy::cast_sign_loss)] // screen bytes
    fn fill(&mut self, rect: Clip, pen: Pen) {
        let r = rect.within(self.clip);
        if r.is_empty() {
            return;
        }
        let (first, last) = ((r.x0 >> 3) as usize, ((r.x1 - 1) >> 3) as usize);
        let left = u8::MAX >> (r.x0 & 7);
        let right = u8::MAX << (7 - ((r.x1 - 1) & 7));
        let rows = &mut self.frame[r.y0 as usize * ROW_BYTES..r.y1 as usize * ROW_BYTES];
        for (y, row) in (r.y0..).zip(rows.as_chunks_mut::<ROW_BYTES>().0) {
            if first == last {
                row[first] = pen.apply(row[first], left & right, y);
                continue;
            }
            row[first] = pen.apply(row[first], left, y);
            let whole = &mut row[first + 1..last];
            match pen {
                Pen::Ink(ink) => whole.fill(pattern(ink, y)),
                Pen::Flip => whole.iter_mut().for_each(|byte| *byte = !*byte),
            }
            row[last] = pen.apply(row[last], right, y);
        }
    }

    /// Sets pixel `(x, y)`, which must be inside the clip.
    fn set(&mut self, x: i32, y: i32, white: bool) {
        let byte = &mut self.frame[byte_index(x, y)];
        let bit = 0x80 >> (x & 7);
        if white {
            *byte |= bit;
        } else {
            *byte &= !bit;
        }
    }
}

/// Draws `buf`'s cells and items at `at` into `target`: cell backgrounds
/// (never a see-through cell's), `Under` items, the white edges of
/// edged glyphs, the glyphs, `Over` items, the invert rectangles. Only
/// the cells that may reach into the clip are visited.
fn draw_layers(
    target: &mut Target<'_>,
    buf: &GlyphBuffer,
    at: Place,
    font: &Font<'_>,
    images: &impl Images,
    mark: &mut impl FnMut(Phase),
) {
    let reach = Reach::of(buf, font, at.scale);
    let clip = target.clip;
    let columns = Reach::span(
        clip.x0,
        clip.x1,
        at.x,
        reach.cell_w,
        reach.right,
        buf.width(),
    );
    let rows = Reach::span(
        clip.y0,
        clip.y1,
        at.y,
        reach.cell_h,
        reach.below,
        buf.height(),
    );
    // Cells `x0..x1` of row `y`, on the screen.
    let span = |x0: i32, x1: i32, y: i32| Clip {
        x0: at.x + x0 * reach.cell_w,
        y0: at.y + y * reach.cell_h,
        x1: at.x + x1 * reach.cell_w,
        y1: at.y + (y + 1) * reach.cell_h,
    };
    let (columns, rows) = (&columns, &rows);
    // Each cell with its top-left on the screen.
    let cells = || {
        rows.clone().flat_map(move |y| {
            columns.clone().filter_map(move |x| {
                let top_left = (at.x + x * reach.cell_w, at.y + y * reach.cell_h);
                buf.get(x, y).map(|cell| (top_left, cell))
            })
        })
    };
    let mut inks = Inks::default();
    // Neighbouring backgrounds of one ink are filled as one rectangle.
    for y in rows.clone() {
        let mut from = (columns.start, None);
        for x in columns.start..=columns.end {
            let cell = (x < columns.end).then(|| buf.get(x, y)).flatten();
            let ink = cell.filter(|c| !c.see_through).map(|c| inks.of(c.bg));
            if ink != from.1 {
                if let Some(ink) = from.1 {
                    target.fill(span(from.0, x, y), Pen::Ink(ink));
                }
                from = (x, ink);
            }
        }
    }
    mark(Phase::Backgrounds);
    draw_items(target, buf, Layer::Under, at, images);
    mark(Phase::Under);
    for (top_left, cell) in cells() {
        if cell.glyph != ' ' && inks.of(cell.fg) == Ink::BlackEdged {
            draw_glyph(target, font, cell.glyph, top_left, at.scale, Glyph::Edge);
        }
    }
    mark(Phase::Edges);
    for (top_left, cell) in cells() {
        if cell.glyph != ' ' {
            let ink = Glyph::Ink(inks.of(cell.fg));
            draw_glyph(target, font, cell.glyph, top_left, at.scale, ink);
        }
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

/// [`ink_of`], remembering the last colour asked: neighbouring cells
/// mostly share their colours.
#[derive(Copy, Clone)]
struct Inks {
    rgb: Rgb,
    ink: Ink,
}

impl Default for Inks {
    fn default() -> Self {
        let rgb = Rgb::new(0, 0, 0);
        Self {
            rgb,
            ink: ink_of(rgb),
        }
    }
}

impl Inks {
    /// The ink `rgb` is drawn in.
    fn of(&mut self, rgb: Rgb) -> Ink {
        if rgb != self.rgb {
            *self = Self {
                rgb,
                ink: ink_of(rgb),
            };
        }
        self.ink
    }
}

/// Draws `buf`'s items of `layer` in order, but for the invert
/// rectangles, which are drawn last.
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

/// What part of a glyph is drawn.
#[derive(Copy, Clone)]
enum Glyph {
    /// The glyph's pixels in this ink.
    Ink(Ink),
    /// White around it, one screen pixel wide all round (corners too).
    Edge,
}

/// Draws `glyph` (a space is nothing) of `font` in `part` with its cell's
/// top-left at screen `(left, top)`, each of its pixels `scale` screen
/// pixels big.
#[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)] // a glyph's rows
#[allow(clippy::cast_sign_loss)] // checked to be a row first
fn draw_glyph(
    target: &mut Target<'_>,
    font: &Font<'_>,
    glyph: char,
    (left, top): (i32, i32),
    scale: i32,
    part: Glyph,
) {
    if glyph == ' ' {
        return;
    }
    let Some(rows) = font.rows(glyph) else {
        return;
    };
    match part {
        Glyph::Ink(ink) => {
            let mut y = top;
            for &byte in rows {
                let bits = widen(byte, scale);
                for _ in 0..scale {
                    target.run(y, left, bits, Pen::Ink(ink));
                    y += 1;
                }
            }
        }
        Glyph::Edge => {
            let height = rows.len() as i32 * scale;
            // The glyph's pixels on screen row `top + dy`, from `left`.
            let row = |dy: i32| {
                if (0..height).contains(&dy) {
                    widen(rows[(dy / scale) as usize], scale)
                } else {
                    0
                }
            };
            for dy in -1..=height {
                // From one pixel left of the cell.
                let near = (row(dy - 1) | row(dy) | row(dy + 1)) >> 1;
                let edge = near | near << 1 | near >> 1;
                target.run(top + dy, left - 1, edge, Pen::Ink(Ink::White));
            }
        }
    }
}

/// `byte`'s eight pixels each `scale` (1 to 8) pixels wide, from the
/// highest bit down.
#[allow(clippy::cast_sign_loss)] // 1 to 8
fn widen(byte: u8, scale: i32) -> u64 {
    if scale == 1 {
        return u64::from(byte) << 56;
    }
    let scale = scale as u32;
    let block = u64::MAX >> (64 - scale);
    let mut out = 0u64;
    for i in 0..8 {
        if byte >> (7 - i) & 1 == 1 {
            out |= block << (64 - scale * (i + 1));
        }
    }
    out
}

/// The Bayer 4 × 4 order a sprite's pixels fade out in, as its opacity
/// falls (`docs/design/playdate.md` dithers in the same pattern).
const BAYER: [[u32; 4]; 4] = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];

/// Whether a pixel of a sprite of `opacity` is drawn at screen `(x, y)`:
/// all of them at 255, none at 0, an even dither between.
#[allow(clippy::cast_sign_loss)] // 0..4
fn shows(opacity: u8, x: i32, y: i32) -> bool {
    let rank = BAYER[(y & 3) as usize][(x & 3) as usize];
    u32::from(opacity) * 16 > rank * 255 + 127
}

/// Draws the visible part of `sprite` at `at`: each screen pixel takes
/// the picture's pixel under its centre, as nearest sampling does
/// ([`Image::sample`]), coloured as its paint says. Without an image it is
/// a grey rectangle.
fn draw_sprite(target: &mut Target<'_>, sprite: &Sprite, at: Place, images: &impl Images) {
    let shown = at.screen(sprite.clip).within(target.clip);
    if shown.is_empty() {
        return;
    }
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
        let dy = i64::from((y - at.y).div_euclid(at.scale)) - i64::from(dest.y);
        let v = Image::sample(
            dy,
            src.y.into(),
            src_h,
            dest_h,
            image.height,
            image.source_height,
        );
        for x in shown.x0..shown.x1 {
            let mut dx = i64::from((x - at.x).div_euclid(at.scale)) - i64::from(dest.x);
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
            // A darkened picture (game ADR-0075): fewer of its white pixels.
            let white = white && shows(sprite.brightness, x, y);
            let white = match (solid, sprite.paint) {
                (Some(ink), _) => ink_is_white(ink, x, y),
                // One-bit has no dimming: every other pixel goes white
                // (`docs/design/playdate.md`, the faded listener).
                (None, Paint::Dimmed) => white || ink_is_white(Ink::Grey, x, y),
                (None, _) => white,
            };
            target.set(x, y, white);
        }
    }
}

#[cfg(test)]
mod speed_tests;
#[cfg(test)]
mod tests;
