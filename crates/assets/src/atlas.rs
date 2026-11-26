//! Sprite atlases (ADR-0153): loose images packed into square pages, so a
//! backend binds one texture for many sprites.
//!
//! An [`AtlasBuilder`] takes images by name and packs them into an
//! [`Atlas`]: its pages (each an [`Image`]) and a table of each name's page
//! and rectangle ([`Placed`]). The packer is a skyline: the images go in
//! by height, tallest first, then by name, so the pages don't depend on the
//! order the images were added, and the same images always give the same
//! pages (ADR-0007: a baked atlas is tested equal to the one packed at
//! load). Each rectangle keeps [`padding`](AtlasBuilder::with_padding)
//! pixels clear from every other and from its page's edges. No sprite is
//! rotated or trimmed.
//!
//! While the game runs, [`Atlas::insert`] adds a sprite (a player's
//! avatar, text rendered once, a generated icon): it goes in free space on
//! a page, or on a new page, and no sprite placed before moves. Its
//! [`Placed::page`] is the page that changed, which a backend uploads
//! again. `shuyi_ui::frame::atlas` shows an atlas's pages in a frame.
//!
//! A page is never bigger than [`image::MAX_SIDE`](crate::image::MAX_SIDE),
//! nor than the side a target's profile allows
//! ([`Display::max_texture_side`](crate::target::Display::max_texture_side),
//! 2048 on the web): give it with [`AtlasBuilder::with_max_side`] or
//! [`AtlasRon::with_max_side`].
//!
//! As a format (ADR-0007), an atlas is the [`Atlas`] kind, and
//! [`AtlasRon`] (`atlas`) reads an `atlas.ron` file, an [`AtlasSpec`]:
//! every image directly inside its folder `from`, read through the image
//! formats, packed. An atlas's names, and its pages' names, are
//! `&'static str`, as every path stays (ADR-0128 §1).
//!
//! ```
//! use shuyi_assets::atlas::AtlasBuilder;
//! use shuyi_assets::image::Image;
//!
//! let mut builder = AtlasBuilder::new("hud", 64).with_padding(1);
//! builder.add("coin", &Image::filled(8, 8, [255, 200, 0, 255]))?;
//! builder.add("heart", &Image::filled(10, 9, [255, 0, 0, 255]))?;
//! let mut atlas = builder.build()?;
//! assert_eq!(atlas.pages().len(), 1);
//! let heart = atlas.get("heart").unwrap();
//! assert_eq!((heart.rect.w, heart.rect.h), (10, 9));
//! // At run time: in free space on page 0, which changed.
//! let gem = atlas.insert("gem", &Image::filled(6, 6, [0, 128, 255, 255]))?;
//! assert_eq!(gem.page, 0);
//! assert_eq!(atlas.get("heart"), Some(heart));
//! # Ok::<(), shuyi_assets::atlas::AtlasError>(())
//! ```
use crate::prelude::*;

use alloc::collections::BTreeMap;
use core::fmt;

use serde::{Deserialize, Serialize};

use crate::error::ContentError;
use crate::format::{AssetKind, AssetPath, DecodeCx, Format, Formats, TEXT_MAX_BYTES, text};
use crate::image::{Image, ImageRect, MAX_SIDE};
use crate::ron::parse_ron;

/// Where a sprite is in its [`Atlas`]: which page, and its rectangle there,
/// in the page's pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Placed {
    /// The page, an index into [`Atlas::pages`].
    pub page: usize,
    /// Its rectangle on that page.
    pub rect: ImageRect,
}

/// Why a sprite isn't in an atlas.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum AtlasError {
    /// It is bigger, with its padding on both sides, than a page.
    TooBig {
        /// Its name.
        name: &'static str,
        /// Its width in pixels.
        width: u32,
        /// Its height in pixels.
        height: u32,
        /// A page's side in pixels.
        side: u32,
    },
    /// The atlas has a sprite of that name already.
    Taken {
        /// The name.
        name: &'static str,
    },
    /// It has no pixels: a side of 0, or fewer bytes than its size needs.
    Empty {
        /// Its name.
        name: &'static str,
    },
}

impl fmt::Display for AtlasError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooBig {
                name,
                width,
                height,
                side,
            } => write!(
                f,
                "sprite {name} is {width}×{height} px, too big for a page of {side}×{side} px with its padding"
            ),
            Self::Taken { name } => write!(f, "the atlas has a sprite {name} already"),
            Self::Empty { name } => write!(f, "sprite {name} has no pixels"),
        }
    }
}

impl core::error::Error for AtlasError {}

/// One run of a page's skyline: the free space above `y`, from `x`, `w`
/// pixels wide.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
struct Segment {
    x: u32,
    y: u32,
    w: u32,
}

/// One page of an [`Atlas`]: a square picture, and the free space left on it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page {
    /// `<atlas id>#<index>`.
    name: &'static str,
    image: Image,
    /// The skyline, left to right over the whole usable width.
    skyline: Vec<Segment>,
}

impl Page {
    /// A clear page of `side` pixels, with `padding` kept at its edges.
    fn new(name: &'static str, side: u32, padding: u32) -> Self {
        Self {
            name,
            image: Image::filled(side, side, [0, 0, 0, 0]),
            skyline: vec![Segment {
                x: padding,
                y: padding,
                w: side - padding,
            }],
        }
    }

    /// Its name, `<atlas id>#<index>`: what a frame names its picture by.
    pub const fn name(&self) -> &'static str {
        self.name
    }

    /// Its pixels.
    pub const fn image(&self) -> &Image {
        &self.image
    }

    /// Where a box `w` × `h` goes on the page, its side `side`: the lowest
    /// place, then the leftmost; `None` if it fits nowhere.
    fn fit(&self, (w, h): (u32, u32), side: u32) -> Option<(u32, u32)> {
        let mut best: Option<(u32, u32)> = None;
        for (i, start) in self.skyline.iter().enumerate() {
            let x = start.x;
            if x + w > side {
                break;
            }
            let end = x + w;
            let y = self.skyline[i..]
                .iter()
                .take_while(|s| s.x < end)
                .map(|s| s.y)
                .max()
                .unwrap_or(start.y);
            // Left to right: a place as low as the best is further right.
            if y + h <= side && best.is_none_or(|(by, _)| y < by) {
                best = Some((y, x));
            }
        }
        best.map(|(y, x)| (x, y))
    }

    /// Raises the skyline over `[x, x + w)` to `top`.
    fn raise(&mut self, x: u32, w: u32, top: u32) {
        let end = x + w;
        let raised = Segment { x, y: top, w };
        let mut out = Vec::with_capacity(self.skyline.len() + 2);
        let mut placed = false;
        for &s in &self.skyline {
            let s_end = s.x + s.w;
            if s_end <= x || s.x >= end {
                if s.x >= end && !placed {
                    out.push(raised);
                    placed = true;
                }
                out.push(s);
                continue;
            }
            if s.x < x {
                out.push(Segment { w: x - s.x, ..s });
            }
            if !placed {
                out.push(raised);
                placed = true;
            }
            if s_end > end {
                out.push(Segment {
                    x: end,
                    y: s.y,
                    w: s_end - end,
                });
            }
        }
        if !placed {
            out.push(raised);
        }
        let mut merged: Vec<Segment> = Vec::with_capacity(out.len());
        for s in out {
            match merged.last_mut() {
                Some(last) if last.y == s.y => last.w += s.w,
                _ => merged.push(s),
            }
        }
        self.skyline = merged;
    }

    /// Copies `image` onto the page with its top-left corner at `(x, y)`.
    fn blit(&mut self, image: &Image, x: u32, y: u32) {
        let row = image.width as usize * 4;
        let page_row = self.image.width as usize * 4;
        for r in 0..image.height as usize {
            let from = r * row;
            let to = (y as usize + r) * page_row + x as usize * 4;
            self.image.rgba[to..to + row].copy_from_slice(&image.rgba[from..from + row]);
        }
    }
}

/// Images packed into pages ([module docs](self)): build one with
/// [`AtlasBuilder`], or read one with [`AtlasRon`]. Plain data: two atlases
/// of the same images are equal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Atlas {
    /// What its pages are named after.
    id: &'static str,
    /// A page's side in pixels.
    side: u32,
    /// The pixels kept clear around each sprite.
    padding: u32,
    pages: Vec<Page>,
    sprites: BTreeMap<&'static str, Placed>,
}

impl AssetKind for Atlas {
    const NAME: &'static str = "atlas";
}

impl Atlas {
    /// What its pages are named after: an `atlas.ron`'s path, or the id
    /// its [`AtlasBuilder`] was given.
    pub const fn id(&self) -> &'static str {
        self.id
    }

    /// A page's side in pixels.
    pub const fn page_size(&self) -> u32 {
        self.side
    }

    /// The pixels kept clear around each sprite.
    pub const fn padding(&self) -> u32 {
        self.padding
    }

    /// Its pages, in order.
    pub fn pages(&self) -> &[Page] {
        &self.pages
    }

    /// Where the sprite `name` is, if the atlas has it.
    pub fn get(&self, name: &str) -> Option<Placed> {
        self.sprites.get(name).copied()
    }

    /// Every sprite's name and place, by name.
    pub fn sprites(&self) -> impl Iterator<Item = (&'static str, Placed)> + '_ {
        self.sprites.iter().map(|(&name, &placed)| (name, placed))
    }

    /// Adds sprite `name` while the game runs: in free space on the first
    /// page that has room for it, or on a new page. No sprite placed before
    /// moves; the [`Placed::page`] it gives is the one page that changed.
    pub fn insert(&mut self, name: &'static str, image: &Image) -> Result<Placed, AtlasError> {
        if self.sprites.contains_key(name) {
            return Err(AtlasError::Taken { name });
        }
        self.check(name, image)?;
        Ok(self.place(name, image))
    }

    /// Whether `image` can go on a page, as sprite `name`.
    fn check(&self, name: &'static str, image: &Image) -> Result<(), AtlasError> {
        let pixels = image.width as usize * image.height as usize * 4;
        if image.width == 0 || image.height == 0 || image.rgba.len() < pixels {
            return Err(AtlasError::Empty { name });
        }
        let room = self.side.saturating_sub(2 * self.padding);
        if image.width > room || image.height > room {
            return Err(AtlasError::TooBig {
                name,
                width: image.width,
                height: image.height,
                side: self.side,
            });
        }
        Ok(())
    }

    /// Places `image`, checked, as sprite `name`.
    fn place(&mut self, name: &'static str, image: &Image) -> Placed {
        let size = (image.width + self.padding, image.height + self.padding);
        let found = self
            .pages
            .iter()
            .enumerate()
            .find_map(|(i, page)| page.fit(size, self.side).map(|at| (i, at)));
        let (page, (x, y)) = found.unwrap_or_else(|| {
            let index = self.pages.len();
            let page_name = leak(format!("{}#{index}", self.id));
            let page = Page::new(page_name, self.side, self.padding);
            // A checked image fits a clear page, at its top-left corner.
            let at = page
                .fit(size, self.side)
                .unwrap_or((self.padding, self.padding));
            self.pages.push(page);
            (index, at)
        });
        let on = &mut self.pages[page];
        on.raise(x, size.0, y + size.1);
        on.blit(image, x, y);
        let placed = Placed {
            page,
            rect: ImageRect {
                x,
                y,
                w: image.width,
                h: image.height,
            },
        };
        self.sprites.insert(name, placed);
        placed
    }
}

/// Packs images into an [`Atlas`] ([module docs](self)).
#[derive(Debug, Clone)]
pub struct AtlasBuilder {
    id: &'static str,
    side: u32,
    padding: u32,
    images: BTreeMap<&'static str, Image>,
}

impl AtlasBuilder {
    /// An atlas named `id` (its pages are `id#0`, `id#1`…) of square pages
    /// `page_size` pixels on a side, at most [`MAX_SIDE`], with no padding.
    #[must_use]
    pub fn new(id: &'static str, page_size: u32) -> Self {
        Self {
            id,
            side: page_size.clamp(1, MAX_SIDE),
            padding: 0,
            images: BTreeMap::new(),
        }
    }

    /// The same, keeping `padding` pixels clear between sprites and from
    /// the pages' edges.
    #[must_use]
    pub const fn with_padding(mut self, padding: u32) -> Self {
        self.padding = padding;
        self
    }

    /// The same, with pages no bigger than `side` on a side: a target's
    /// texture limit ([`Display::max_texture_side`](crate::target::Display::max_texture_side)).
    #[must_use]
    pub fn with_max_side(mut self, side: u32) -> Self {
        self.side = self.side.min(side.max(1));
        self
    }

    /// Adds `image` as sprite `name`. The order images are added in doesn't
    /// change the atlas.
    pub fn add(&mut self, name: &'static str, image: &Image) -> Result<(), AtlasError> {
        if self.images.contains_key(name) {
            return Err(AtlasError::Taken { name });
        }
        self.images.insert(name, image.clone());
        Ok(())
    }

    /// The atlas of every image added: tallest first, then by name. The
    /// first sprite that can't go on a page is the error.
    pub fn build(self) -> Result<Atlas, AtlasError> {
        let mut atlas = Atlas {
            id: self.id,
            side: self.side,
            padding: self.padding,
            pages: Vec::new(),
            sprites: BTreeMap::new(),
        };
        let mut order: Vec<(&'static str, Image)> = self.images.into_iter().collect();
        // Stable: equal heights stay in name order.
        order.sort_by_key(|a| core::cmp::Reverse(a.1.height));
        for (name, image) in &order {
            atlas.check(name, image)?;
        }
        for (name, image) in &order {
            atlas.place(name, image);
        }
        Ok(atlas)
    }
}

/// An `atlas.ron` file ([`AtlasRon`]): which folder's images to pack, and
/// how.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct AtlasSpec {
    /// The folder whose images are packed, from the bundle's root
    /// (`"sprites/"`): every image directly inside it, not in folders
    /// below.
    pub from: String,
    /// A page's side in pixels.
    pub page_size: u32,
    /// The pixels kept clear around each sprite: 0 unless said.
    #[serde(default)]
    pub padding: u32,
}

/// An atlas read from an `atlas.ron` file (`atlas`, `.ron`), an
/// [`AtlasSpec`]: every image of its folder read through the image
/// formats ([`Formats::load_each`]), packed into pages no bigger than
/// [`with_max_side`](Self::with_max_side) says. The atlas's id is the
/// file's path.
#[derive(Debug)]
pub struct AtlasRon {
    images: Formats,
    max_side: u32,
}

impl Default for AtlasRon {
    fn default() -> Self {
        Self::new()
    }
}

impl AtlasRon {
    /// The format, reading the images' pixels with the engine's picture
    /// formats whose features are on (PNG with `png`; JPEG, WebP, QOI and
    /// JPEG XL with theirs), pages at most [`MAX_SIDE`] on a side.
    pub fn new() -> Self {
        Self::with_images(pixel_formats())
    }

    /// The format, reading the images' pixels with `images`: a game's own
    /// picture formats.
    pub fn with_images(images: Formats) -> Self {
        Self {
            images,
            max_side: MAX_SIDE,
        }
    }

    /// The same, with pages no bigger than `side` on a side: a target's
    /// texture limit.
    #[must_use]
    pub fn with_max_side(mut self, side: u32) -> Self {
        self.max_side = side;
        self
    }
}

/// A registry of the engine's picture formats whose features are on.
fn pixel_formats() -> Formats {
    #[allow(unused_mut)] // with no picture feature on, nothing is added
    let mut formats = Formats::new();
    // Each is the first of its id and extension in an empty registry.
    #[cfg(feature = "png")]
    let _ = formats.register(crate::image::Png);
    #[cfg(feature = "jpeg")]
    let _ = formats.register(crate::image::Jpeg);
    #[cfg(feature = "webp")]
    let _ = formats.register(crate::image::WebP);
    #[cfg(feature = "qoi")]
    let _ = formats.register(crate::image::Qoi);
    #[cfg(feature = "jpeg-xl")]
    let _ = formats.register(crate::image::JpegXl);
    formats
}

impl Format for AtlasRon {
    type Asset = Atlas;

    fn id(&self) -> &'static str {
        "atlas"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["ron"]
    }

    fn max_bytes(&self) -> usize {
        TEXT_MAX_BYTES
    }

    fn decode(
        &self,
        file: &AssetPath,
        bytes: &[u8],
        cx: &DecodeCx<'_>,
    ) -> Result<Atlas, Vec<ContentError>> {
        let error = |message: String| vec![ContentError::new(file.shown(), message)];
        let spec: AtlasSpec = parse_ron(file.shown(), text(file, bytes)?).map_err(|e| vec![e])?;
        let Some(bundle) = cx.bundle else {
            return Err(error(format!(
                "its images in {} can't be read without the bundle",
                spec.from
            )));
        };
        let dir = spec.from.trim_end_matches('/');
        let images = self.images.load_each::<Image>(bundle, dir);
        if images.is_empty() {
            let known = self.images.extensions::<Image>();
            return Err(error(format!(
                "no image in folder {}: the picture formats read .{}",
                spec.from,
                if known.is_empty() {
                    "nothing (turn on the png feature)".to_owned()
                } else {
                    known.join(", .")
                }
            )));
        }
        let mut builder = AtlasBuilder::new(leak(file.path().to_owned()), spec.page_size)
            .with_padding(spec.padding)
            .with_max_side(self.max_side);
        let mut errors = Vec::new();
        for (path, image) in images {
            match image {
                Ok(image) => builder
                    .add(path, &image)
                    .map_err(|e| error(e.to_string()))?,
                Err(e) => errors.extend(e),
            }
        }
        if !errors.is_empty() {
            return Err(errors);
        }
        builder.build().map_err(|e| error(e.to_string()))
    }
}

/// `s`, kept for the run: a name an atlas hands out as `&'static str`.
fn leak(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}

/// An [`Atlas`]'s binary form (a baked atlas): its names owned, and leaked
/// when it is read back.
#[derive(Serialize, Deserialize)]
struct Form {
    id: String,
    side: u32,
    padding: u32,
    pages: Vec<(String, Image, Vec<Segment>)>,
    sprites: BTreeMap<String, Placed>,
}

impl From<Atlas> for Form {
    fn from(atlas: Atlas) -> Self {
        Self {
            id: atlas.id.to_owned(),
            side: atlas.side,
            padding: atlas.padding,
            pages: atlas
                .pages
                .into_iter()
                .map(|page| (page.name.to_owned(), page.image, page.skyline))
                .collect(),
            sprites: atlas
                .sprites
                .into_iter()
                .map(|(name, placed)| (name.to_owned(), placed))
                .collect(),
        }
    }
}

impl Serialize for Atlas {
    fn serialize<S: serde::Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        Form::from(self.clone()).serialize(ser)
    }
}

impl<'de> Deserialize<'de> for Atlas {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Form::deserialize(d).map(Self::from)
    }
}

impl From<Form> for Atlas {
    fn from(form: Form) -> Self {
        Self {
            id: leak(form.id),
            side: form.side,
            padding: form.padding,
            pages: form
                .pages
                .into_iter()
                .map(|(name, image, skyline)| Page {
                    name: leak(name),
                    image,
                    skyline,
                })
                .collect(),
            sprites: form
                .sprites
                .into_iter()
                .map(|(name, placed)| (leak(name), placed))
                .collect(),
        }
    }
}

#[cfg(test)]
pub(crate) mod tests;
