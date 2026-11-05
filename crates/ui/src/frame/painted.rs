//! Pictures a skin paints at run time (ADR-0078): a [`Painted`] picture
//! whose pixels the game makes while it runs (a decoded video frame, a
//! software renderer's screen, a plasma), kept between frames in the
//! frame's [`PaintedTable`] and placed with a [`PaintedSprite`].
//!
//! The table lives in the [`GlyphBuffer`] a screen draws into: the
//! game's buffer outlives every frame, so its pictures do too. A skin
//! takes its picture from the table, writes its pixels when they change,
//! and places it:
//!
//! ```
//! use shuyi_ui::frame::painted::{Painted, PaintedId, PaintedSprite};
//! use shuyi_ui::frame::{Cell, GlyphBuffer, Layer, PxRect, Rgb};
//!
//! const PLASMA: PaintedId = PaintedId::new("plasma");
//! let mut buf = GlyphBuffer::new(20, 6, Cell::new(' ', Rgb::new(255, 255, 255), Rgb::new(0, 0, 0)));
//! let picture = buf
//!     .pictures_mut()
//!     .get_or_insert_with(PLASMA, || Painted::rgba(64, 48));
//! for pixel in picture.pixels_mut().chunks_exact_mut(4) {
//!     pixel.copy_from_slice(&[255, 128, 0, 255]);
//! }
//! let whole = PxRect::new(0, 0, 64, 48);
//! buf.add_painted(PaintedSprite::new(PLASMA, whole, PxRect::new(8, 8, 128, 96), Layer::Over));
//! assert_eq!(buf.pictures().get(PLASMA).map(Painted::revision), Some(1));
//! ```
//!
//! Every pixel is written with [`Painted::pixels_mut`] (or the palette
//! with [`Painted::palette_mut`]), which moves the picture's
//! [revision](Painted::revision): `shuyi-app` uploads a picture again only
//! in a frame whose revision moved. Borrow them only in a frame that
//! changes them.
//!
//! [`GlyphBuffer`]: crate::frame::glyph_buffer::GlyphBuffer

use crate::prelude::*;
use alloc::collections::BTreeMap;
use alloc::collections::btree_map::Entry;
use core::fmt;

use shuyi_assets::fnv::Hasher;

use crate::frame::color::Rgb;
use crate::frame::glyph_buffer::{Layer, PxRect};

/// How many colours an indexed picture's palette holds.
pub const PALETTE_LEN: usize = 256;

/// The game's name for a painted picture, as an image's path names an
/// image: cheap to copy and compare.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PaintedId(&'static str);

impl PaintedId {
    /// The picture named `name`.
    pub const fn new(name: &'static str) -> Self {
        Self(name)
    }

    /// Its name.
    pub const fn name(self) -> &'static str {
        self.0
    }
}

impl fmt::Display for PaintedId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

/// What a [`Painted`] picture's bytes are.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PaintedKind {
    /// Four bytes a pixel, red, green, blue and alpha, row after row.
    Rgba,
    /// A byte a pixel, an index into the picture's palette of
    /// [`PALETTE_LEN`] colours, row after row. Every pixel is opaque.
    Indexed,
    /// A bit a pixel: rows of `ceil(width / 8)` bytes, a byte's highest
    /// bit its leftmost pixel, a set bit white, as
    /// [`onebit::Image`](crate::frame::onebit::Image) has them; with an
    /// optional mask of the same shape, a set bit drawn.
    OneBit,
}

impl PaintedKind {
    /// Its name in a snapshot.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Rgba => "rgba",
            Self::Indexed => "indexed",
            Self::OneBit => "one-bit",
        }
    }

    /// Bytes per row of a picture of this kind `width` pixels wide.
    pub const fn stride(self, width: u16) -> usize {
        match self {
            Self::Rgba => width as usize * 4,
            Self::Indexed => width as usize,
            Self::OneBit => (width as usize).div_ceil(8),
        }
    }
}

/// A picture a skin paints at run time (ADR-0078). Its fields keep their
/// sizes, so they are read and written through methods; the ones that
/// write move its [revision](Self::revision).
#[derive(Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Painted {
    /// First, so comparing two pictures stops at it when it differs.
    revision: u32,
    width: u16,
    height: u16,
    kind: PaintedKind,
    pixels: Vec<u8>,
    /// [`PALETTE_LEN`] colours for an indexed picture, else none.
    palette: Vec<Rgb>,
    /// A one-bit picture's mask, if it has one.
    mask: Option<Vec<u8>>,
}

impl Painted {
    /// A picture of `kind`, `width` × `height`, every byte 0.
    fn blank(kind: PaintedKind, width: u16, height: u16) -> Self {
        Self {
            revision: 0,
            width,
            height,
            kind,
            pixels: vec![0; kind.stride(width) * usize::from(height)],
            palette: Vec::new(),
            mask: None,
        }
    }

    /// An RGBA picture `width` × `height`, every pixel clear.
    pub fn rgba(width: u16, height: u16) -> Self {
        Self::blank(PaintedKind::Rgba, width, height)
    }

    /// An indexed picture `width` × `height`, every pixel index 0, its
    /// palette the first [`PALETTE_LEN`] colours of `palette` and black
    /// for the rest.
    pub fn indexed(width: u16, height: u16, palette: &[Rgb]) -> Self {
        let mut colours = palette
            .iter()
            .copied()
            .take(PALETTE_LEN)
            .collect::<Vec<_>>();
        colours.resize(PALETTE_LEN, Rgb::new(0, 0, 0));
        Self {
            palette: colours,
            ..Self::blank(PaintedKind::Indexed, width, height)
        }
    }

    /// A one-bit picture `width` × `height`, every pixel black, without a
    /// mask (every pixel drawn).
    pub fn one_bit(width: u16, height: u16) -> Self {
        Self::blank(PaintedKind::OneBit, width, height)
    }

    /// The same one-bit picture with a mask, every pixel drawn; any other
    /// picture as it is.
    #[must_use]
    pub fn with_mask(mut self) -> Self {
        if self.kind == PaintedKind::OneBit {
            self.mask = Some(vec![u8::MAX; self.pixels.len()]);
        }
        self
    }

    /// Its width and height in pixels.
    pub const fn size(&self) -> (u16, u16) {
        (self.width, self.height)
    }

    /// What its bytes are.
    pub const fn kind(&self) -> PaintedKind {
        self.kind
    }

    /// Bytes per row of its [pixels](Self::pixels) (and mask).
    pub const fn stride(&self) -> usize {
        self.kind.stride(self.width)
    }

    /// How many times its pixels, palette or mask were borrowed for
    /// writing, wrapping: a picture whose revision didn't move since a
    /// frame is that frame's.
    pub const fn revision(&self) -> u32 {
        self.revision
    }

    /// Its bytes, as its [kind](Self::kind) says.
    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    /// Its bytes to write; moves the revision.
    pub fn pixels_mut(&mut self) -> &mut [u8] {
        self.touch();
        &mut self.pixels
    }

    /// An indexed picture's [`PALETTE_LEN`] colours; none for another.
    pub fn palette(&self) -> &[Rgb] {
        &self.palette
    }

    /// An indexed picture's colours to write (none for another); moves the
    /// revision.
    pub fn palette_mut(&mut self) -> &mut [Rgb] {
        self.touch();
        &mut self.palette
    }

    /// A one-bit picture's mask, a set bit drawn, if it has one.
    pub fn mask(&self) -> Option<&[u8]> {
        self.mask.as_deref()
    }

    /// The mask to write, if it has one; then moves the revision.
    pub fn mask_mut(&mut self) -> Option<&mut [u8]> {
        if self.mask.is_some() {
            self.touch();
        }
        self.mask.as_deref_mut()
    }

    /// Moves the revision.
    fn touch(&mut self) {
        self.revision = self.revision.wrapping_add(1);
    }

    /// Pixel `(x, y)`, which must lie in the picture, as RGBA8: an indexed
    /// one's colour, opaque; a one-bit one white or black, clear where the
    /// mask leaves it out.
    pub fn rgba_at(&self, x: u16, y: u16) -> [u8; 4] {
        let row = usize::from(y) * self.stride();
        match self.kind {
            PaintedKind::Rgba => {
                let at = row + usize::from(x) * 4;
                [
                    self.pixels[at],
                    self.pixels[at + 1],
                    self.pixels[at + 2],
                    self.pixels[at + 3],
                ]
            }
            PaintedKind::Indexed => {
                let c = self.palette[usize::from(self.pixels[row + usize::from(x)])];
                [c.r, c.g, c.b, u8::MAX]
            }
            PaintedKind::OneBit => {
                let (at, shift) = (row + usize::from(x >> 3), 7 - (x & 7));
                let bit = |bytes: &[u8]| bytes[at] >> shift & 1 == 1;
                let white = if bit(&self.pixels) { u8::MAX } else { 0 };
                let drawn = self.mask.as_deref().is_none_or(bit);
                [white, white, white, if drawn { u8::MAX } else { 0 }]
            }
        }
    }

    /// Writes the whole picture as RGBA8, row after row, into `out` (its
    /// old bytes gone): each pixel as [`rgba_at`](Self::rgba_at) has it.
    /// What a painter that draws RGBA uploads.
    pub fn rgba_into(&self, out: &mut Vec<u8>) {
        out.clear();
        if self.kind == PaintedKind::Rgba {
            out.extend_from_slice(&self.pixels);
            return;
        }
        for y in 0..self.height {
            for x in 0..self.width {
                out.extend_from_slice(&self.rgba_at(x, y));
            }
        }
    }

    /// The FNV hash (`shuyi_assets::fnv`) of its size, kind, pixels,
    /// palette and mask, not its revision: what a snapshot shows of it.
    pub fn fingerprint(&self) -> u64 {
        let mut hash = Hasher::new();
        hash.word(u64::from(self.width));
        hash.word(u64::from(self.height));
        hash.sized(self.kind.name().as_bytes());
        hash.sized(&self.pixels);
        let palette: Vec<u8> = self.palette.iter().flat_map(|c| [c.r, c.g, c.b]).collect();
        hash.sized(&palette);
        match &self.mask {
            Some(mask) => hash.sized(mask),
            None => hash.word(u64::MAX),
        }
        hash.finish()
    }

    /// Whether it shows the same as `other`: everything but the revision.
    fn same_look(&self, other: &Self) -> bool {
        self.width == other.width
            && self.height == other.height
            && self.kind == other.kind
            && self.pixels == other.pixels
            && self.palette == other.palette
            && self.mask == other.mask
    }
}

/// Its size, kind, revision and fingerprint: its pixels would bury a
/// failed test's message.
impl fmt::Debug for Painted {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Painted({}x{} {} r{} {:016x})",
            self.width,
            self.height,
            self.kind.name(),
            self.revision,
            self.fingerprint()
        )
    }
}

/// Where a painted picture is drawn (ADR-0078): part of it, `src`,
/// stretched to `dest` with nearest-pixel sampling, never smoothed.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
#[non_exhaustive]
pub struct PaintedSprite {
    /// Which picture of the frame's table.
    pub picture: PaintedId,
    /// The part of the picture shown, in its pixels. It must lie inside
    /// the picture.
    pub src: PxRect,
    /// Where `src` is stretched to, in console pixels.
    pub dest: PxRect,
    /// The part of `dest` that is drawn; clipping only ever shrinks it,
    /// as a sprite's.
    pub clip: PxRect,
    /// Under or over the glyphs.
    pub layer: Layer,
    /// How bright its pixels are drawn: 255 = as painted, lower = that
    /// much darker ([`GlyphBuffer::fade`] lowers it).
    ///
    /// [`GlyphBuffer::fade`]: crate::frame::glyph_buffer::GlyphBuffer::fade
    pub brightness: u8,
}

impl PaintedSprite {
    /// `src` of `picture` shown in all of `dest`, as painted.
    pub const fn new(picture: PaintedId, src: PxRect, dest: PxRect, layer: Layer) -> Self {
        Self {
            picture,
            src,
            dest,
            clip: dest,
            layer,
            brightness: u8::MAX,
        }
    }

    /// The part of `src` that maps to `clip`, as `[x, y, w, h]` in the
    /// picture's pixels, as a sprite's
    /// [`clipped_src`](crate::frame::glyph_buffer::Sprite::clipped_src).
    pub fn clipped_src(&self) -> [f32; 4] {
        crate::frame::glyph_buffer::clipped_src(self.src, self.dest, self.clip, false)
    }
}

/// The painted pictures of a frame, by id (ADR-0078). A [`GlyphBuffer`]
/// holds one ([`GlyphBuffer::pictures_mut`]).
///
/// [`GlyphBuffer`]: crate::frame::glyph_buffer::GlyphBuffer
/// [`GlyphBuffer::pictures_mut`]: crate::frame::glyph_buffer::GlyphBuffer::pictures_mut
#[derive(Clone, Default, PartialEq, Eq, Debug)]
pub struct PaintedTable {
    pictures: BTreeMap<PaintedId, Painted>,
}

impl PaintedTable {
    /// The picture `id`, if the table has it.
    pub fn get(&self, id: PaintedId) -> Option<&Painted> {
        self.pictures.get(&id)
    }

    /// The picture `id` to change, if the table has it. Taking it moves
    /// nothing; writing its pixels does.
    pub fn get_mut(&mut self, id: PaintedId) -> Option<&mut Painted> {
        self.pictures.get_mut(&id)
    }

    /// The picture `id`, made by `make` if the table hasn't one, whatever
    /// its size: put another in with [`insert`](Self::insert).
    pub fn get_or_insert_with(
        &mut self,
        id: PaintedId,
        make: impl FnOnce() -> Painted,
    ) -> &mut Painted {
        self.pictures.entry(id).or_insert_with(make)
    }

    /// Puts `picture` in as `id`, in place of any it had: its revision
    /// moves past the old one's, so it never names the old pixels.
    pub fn insert(&mut self, id: PaintedId, mut picture: Painted) -> &mut Painted {
        match self.pictures.entry(id) {
            Entry::Occupied(mut slot) => {
                picture.revision = slot.get().revision.wrapping_add(1);
                slot.insert(picture);
                slot.into_mut()
            }
            Entry::Vacant(slot) => slot.insert(picture),
        }
    }

    /// Takes the picture `id` out.
    pub fn remove(&mut self, id: PaintedId) -> Option<Painted> {
        self.pictures.remove(&id)
    }

    /// Every picture, by id.
    pub fn iter(&self) -> impl Iterator<Item = (PaintedId, &Painted)> + '_ {
        self.pictures.iter().map(|(&id, picture)| (id, picture))
    }

    /// How many pictures it holds.
    pub fn len(&self) -> usize {
        self.pictures.len()
    }

    /// Whether it holds none.
    pub fn is_empty(&self) -> bool {
        self.pictures.is_empty()
    }

    /// Puts `picture` in as `id` unless the table's already looks the same
    /// (its size, kind, pixels, palette and mask), which it then keeps: so
    /// the revision moves only when the picture changed, and a skin may
    /// make its picture afresh each frame and have it uploaded only when it
    /// differs. One that differs replaces the table's, as
    /// [`insert`](Self::insert) does.
    pub fn update(&mut self, id: PaintedId, mut picture: Painted) -> &mut Painted {
        match self.pictures.entry(id) {
            Entry::Occupied(mut slot) => {
                if !slot.get().same_look(&picture) {
                    picture.revision = slot.get().revision.wrapping_add(1);
                    slot.insert(picture);
                }
                slot.into_mut()
            }
            Entry::Vacant(slot) => slot.insert(picture),
        }
    }

    /// Copies `picture` in as `id` as [`update`](Self::update) does: a
    /// sub-buffer's or a scene's picture coming into the frame (ADR-0078
    /// §3), copied only when it differs.
    pub(crate) fn absorb(&mut self, id: PaintedId, picture: &Painted) {
        if !self.get(id).is_some_and(|old| old.same_look(picture)) {
            self.update(id, picture.clone());
        }
    }
}

#[cfg(test)]
mod tests;
