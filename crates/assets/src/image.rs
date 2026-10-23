//! The image table: every PNG in a [`Bundle`] the game names, by bundle
//! path, with its size. Screens name a picture by an [`ImageId`] from this
//! table; the platform layer decodes and draws it. Only each file's header
//! is read here, never its pixels.
//!
//! A one-bit screen's bundle may hold a one-bit picture file
//! ([`crate::bitmap`]) at a PNG's path in place of the PNG: the table
//! then gives the size of the PNG it was made from (its source size), so
//! the image's id and size are the same as with the PNG.
//!
//! As formats (ADR-0007): an image's size is the [`ImageInfo`] kind, read
//! by [`PngInfo`] and [`OneBitInfo`](crate::bitmap::OneBitInfo), and
//! [`ImageTable::load_with`] builds the table with every image format a
//! game registers. Its pixels are the [`Image`] kind, which `Png` reads
//! with the `png` feature. A game that draws through `shuyi-app` needs
//! none: the platform decodes a PNG's pixels when it uploads the picture.
use crate::prelude::*;
use serde::{Deserialize, Serialize};

use alloc::collections::BTreeMap;

use crate::bitmap::Bitmap;
use crate::bundle::Bundle;
use crate::error::ContentError;
use crate::format::{AssetKind, AssetPath, DecodeCx, Format, Formats};

/// The longest side an image may have, in pixels: what every graphics card
/// the engine runs on (WebGL included) takes as one texture.
pub const MAX_SIDE: u32 = 4096;

/// An image's size in pixels. Build it with [`ImageInfo::new`] or
/// [`ImageInfo::checked`] (ADR-0039).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ImageInfo {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
}

impl ImageInfo {
    /// The size `width × height`, unchecked: [`checked`](Self::checked)
    /// says whether an image may have it.
    #[must_use]
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    /// The size `width × height`, if an image may have it: neither side
    /// 0, nor over [`MAX_SIDE`]. Otherwise what is wrong, in plain words.
    pub fn checked(width: u32, height: u32) -> Result<Self, String> {
        if width == 0 || height == 0 {
            return Err(format!(
                "image is {width}×{height} px; both sides must be greater than 0"
            ));
        }
        if width > MAX_SIDE || height > MAX_SIDE {
            return Err(format!(
                "image is {width}×{height} px; neither side may be over {MAX_SIDE}"
            ));
        }
        Ok(Self { width, height })
    }
}

impl AssetKind for ImageInfo {
    const NAME: &'static str = "image";
}

/// An image's pixels: four bytes each (red, green, blue, alpha), row by
/// row from the top.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Image {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// `width × height × 4` bytes.
    pub rgba: Vec<u8>,
}

impl Image {
    /// Its size.
    pub fn info(&self) -> ImageInfo {
        ImageInfo {
            width: self.width,
            height: self.height,
        }
    }
}

impl AssetKind for Image {
    const NAME: &'static str = "image pixels";
}

/// The first eight bytes of every PNG file.
pub const PNG_SIGNATURE: &[u8; 8] = b"\x89PNG\r\n\x1a\n";

/// The most bytes a PNG may have: 64 MiB, what a [`MAX_SIDE`] square of
/// 8-bit RGBA pixels takes without compression.
pub const MAX_PNG_BYTES: usize = 64 << 20;

/// PNG read for its size (`png`, `.png`): only its header, never its
/// pixels. The size must be one [`ImageInfo::checked`] allows.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PngInfo;

impl Format for PngInfo {
    type Asset = ImageInfo;

    fn id(&self) -> &'static str {
        "png"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["png"]
    }

    fn sniff(&self, bytes: &[u8]) -> bool {
        bytes.starts_with(PNG_SIGNATURE)
    }

    fn max_bytes(&self) -> usize {
        MAX_PNG_BYTES
    }

    fn decode(
        &self,
        file: &AssetPath,
        bytes: &[u8],
        _cx: &DecodeCx<'_>,
    ) -> Result<ImageInfo, Vec<ContentError>> {
        png_info(bytes).map_err(|message| vec![ContentError::new(file.shown(), message)])
    }
}

/// The checked size in PNG file `bytes`, or what is wrong with it.
fn png_info(bytes: &[u8]) -> Result<ImageInfo, String> {
    let (width, height) = png_size(bytes).ok_or("not a valid PNG file")?;
    ImageInfo::checked(width, height)
}

/// PNG read for its pixels (`png`, `.png`), with the `png` feature. Its
/// size is checked first, as [`PngInfo`] does, so a picture too big is
/// refused before its pixels are decoded.
#[cfg(feature = "png")]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Png;

#[cfg(feature = "png")]
impl Format for Png {
    type Asset = Image;

    fn id(&self) -> &'static str {
        "png"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["png"]
    }

    fn sniff(&self, bytes: &[u8]) -> bool {
        bytes.starts_with(PNG_SIGNATURE)
    }

    fn max_bytes(&self) -> usize {
        MAX_PNG_BYTES
    }

    fn decode(
        &self,
        file: &AssetPath,
        bytes: &[u8],
        _cx: &DecodeCx<'_>,
    ) -> Result<Image, Vec<ContentError>> {
        let error = |message: String| vec![ContentError::new(file.shown(), message)];
        png_info(bytes).map_err(error)?;
        let (width, height, rgba) = crate::bitmap::convert::png_pixels(bytes).map_err(error)?;
        Ok(Image {
            width,
            height,
            rgba,
        })
    }
}

/// Names one image of an [`ImageTable`]: its interned bundle path. Cheap to
/// copy and compare. Only [`ImageTable::id`] makes one, so an id always
/// names an image some table holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ImageId(&'static str);

impl ImageId {
    /// The image's path in the bundle, e.g. `images/test_card.png`.
    pub const fn path(self) -> &'static str {
        self.0
    }
}

/// A rectangle of an image, in image pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ImageRect {
    /// Left edge.
    pub x: u32,
    /// Top edge.
    pub y: u32,
    /// Width.
    pub w: u32,
    /// Height.
    pub h: u32,
}

/// A picture: a rectangle of an image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Picture {
    /// The image.
    pub image: ImageId,
    /// Where in it.
    pub rect: ImageRect,
}

/// Every image the game can draw, by bundle path (`/`-separated). Build it
/// with [`ImageTable::new`], [`Default`] or [`ImageTable::load`]
/// (ADR-0039).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct ImageTable {
    /// Size by path.
    pub images: BTreeMap<&'static str, ImageInfo>,
}

impl ImageTable {
    /// The table of `images`, their sizes by path.
    #[must_use]
    pub const fn new(images: BTreeMap<&'static str, ImageInfo>) -> Self {
        Self { images }
    }

    /// Reads the size of every PNG in `bundle` but those at the paths in
    /// `skip` (e.g. a font atlas, checked on its own), reporting every bad
    /// file.
    pub fn load(bundle: &Bundle, skip: &[&str]) -> Result<Self, Vec<ContentError>> {
        let files = bundle
            .files_under("")
            .into_iter()
            .filter(|path| is_png(path) && !skip.contains(path))
            .map(|path| (path, bundle.bytes(path).unwrap_or_default()));
        Self::from_files(bundle, files)
    }

    /// Like [`ImageTable::load`], with the image formats of `formats`:
    /// every file in `bundle` whose extension one of them claims
    /// ([`Formats::claims`]), but those at the paths in `skip`, read by
    /// them ([`ImageTable::from_files_with`]). With
    /// [`Formats::standard`] it reads the files `load` reads.
    pub fn load_with(
        bundle: &Bundle,
        formats: &Formats,
        skip: &[&str],
    ) -> Result<Self, Vec<ContentError>> {
        let files = bundle
            .files_under("")
            .into_iter()
            .filter(|path| formats.claims::<ImageInfo>(path) && !skip.contains(path))
            .map(|path| (path, bundle.bytes(path).unwrap_or_default()));
        Self::from_files_with(bundle, formats, files)
    }

    /// Like [`ImageTable::from_files`], each file read by the image
    /// formats of `formats` ([`Formats::decode`]). With
    /// [`Formats::standard`] it gives what `from_files` gives for files
    /// named `.png`.
    pub fn from_files_with<'a>(
        bundle: &Bundle,
        formats: &Formats,
        files: impl IntoIterator<Item = (&'static str, &'a [u8])>,
    ) -> Result<Self, Vec<ContentError>> {
        let cx = DecodeCx::in_bundle(bundle).with_limits(formats.limits);
        let mut images = BTreeMap::new();
        let mut errors = Vec::new();
        for (path, bytes) in files {
            let file = AssetPath::in_bundle(bundle, path);
            match formats.decode::<ImageInfo>(&file, bytes, &cx) {
                Ok(info) => {
                    images.insert(path, info);
                }
                Err(e) => errors.extend(e),
            }
        }
        if errors.is_empty() {
            Ok(Self { images })
        } else {
            Err(errors)
        }
    }

    /// The table of `files` (bundle path and contents), or every problem
    /// found: a file that isn't a PNG (or a one-bit picture in a PNG's
    /// place), an empty image, or one with a side
    /// over [`MAX_SIDE`]. Errors name each file as `bundle` shows it.
    pub fn from_files<'a>(
        bundle: &Bundle,
        files: impl IntoIterator<Item = (&'static str, &'a [u8])>,
    ) -> Result<Self, Vec<ContentError>> {
        let mut images = BTreeMap::new();
        let mut errors = Vec::new();
        for (path, bytes) in files {
            match read_info(bytes) {
                Ok(info) => {
                    images.insert(path, info);
                }
                Err(message) => {
                    errors.push(ContentError::new(bundle.display_path(path), message));
                }
            }
        }
        if errors.is_empty() {
            Ok(Self { images })
        } else {
            Err(errors)
        }
    }

    /// The id of the image at `path`, or `None` if the table lacks it.
    pub fn id(&self, path: &str) -> Option<ImageId> {
        self.images
            .get_key_value(path)
            .map(|(&path, _)| ImageId(path))
    }

    /// The size of image `id`, or `None` if it is another table's.
    pub fn info(&self, id: ImageId) -> Option<ImageInfo> {
        self.images.get(id.path()).copied()
    }

    /// Every image's id, in path order.
    pub fn ids(&self) -> impl Iterator<Item = ImageId> + '_ {
        self.images.keys().map(|&path| ImageId(path))
    }
}

/// Whether `path` is named as a PNG (`.png`, any case).
fn is_png(path: &str) -> bool {
    path.rsplit_once('.').is_some_and(|(stem, ext)| {
        !stem.is_empty() && !stem.ends_with('/') && ext.eq_ignore_ascii_case("png")
    })
}

/// The size in PNG file `bytes`, or in one-bit picture file `bytes` the
/// size of the PNG it was made from, or what is wrong with it in plain
/// words.
fn read_info(bytes: &[u8]) -> Result<ImageInfo, String> {
    if Bitmap::is_bitmap(bytes) {
        crate::bitmap::source_info(bytes)
    } else {
        png_info(bytes)
    }
}

/// Width and height from a PNG's header, or `None` if `bytes` isn't a PNG.
pub fn png_size(bytes: &[u8]) -> Option<(u32, u32)> {
    // Signature, then the IHDR chunk: length (4), type (4), width (4), height (4).
    let header = bytes.get(..24)?;
    if &header[..8] != PNG_SIGNATURE || &header[12..16] != b"IHDR" {
        return None;
    }
    let be = |at: usize| {
        u32::from_be_bytes([header[at], header[at + 1], header[at + 2], header[at + 3]])
    };
    Some((be(16), be(20)))
}

/// The start of a `w × h` PNG file: enough for [`png_size`].
#[cfg(test)]
pub(crate) fn png_header(w: u32, h: u32) -> Vec<u8> {
    let mut v = b"\x89PNG\r\n\x1a\n\0\0\0\x0dIHDR".to_vec();
    v.extend(w.to_be_bytes());
    v.extend(h.to_be_bytes());
    v.extend([8, 6, 0, 0, 0]);
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundle::Layer;

    /// A bundle of `files`, shown as `base/...`.
    fn bundle(files: &'static [(&'static str, &'static [u8])]) -> Bundle {
        Bundle::new(vec![Layer::table("base", files)])
    }

    fn leaked(bytes: Vec<u8>) -> &'static [u8] {
        Box::leak(bytes.into_boxed_slice())
    }

    fn messages(result: Result<ImageTable, Vec<ContentError>>) -> Vec<String> {
        result
            .err()
            .unwrap_or_default()
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    #[test]
    fn png_size_reads_header() {
        assert_eq!(png_size(&png_header(256, 288)), Some((256, 288)));
        // Distinct bytes, so any byte-order slip shows.
        assert_eq!(
            png_size(&png_header(0x0102_0304, 0x0506_0708)),
            Some((0x0102_0304, 0x0506_0708))
        );
        assert_eq!(png_size(&png_header(256, 288)[..23]), None);
        let mut bad = png_header(1, 1);
        bad[0] = 0;
        assert_eq!(png_size(&bad), None);
        let mut bad = png_header(1, 1);
        bad[12] = b'X';
        assert_eq!(png_size(&bad), None);
    }

    #[test]
    fn is_png_goes_by_the_extension() {
        assert!(is_png("images/a.png"));
        assert!(is_png("a.PNG"));
        assert!(!is_png("images/a.ron"));
        assert!(!is_png("images/png"));
        assert!(!is_png("images/.png"));
        assert!(!is_png("a.png.txt"));
    }

    #[test]
    fn from_files_keeps_each_size_by_path() {
        // The biggest image allowed is `MAX_SIDE` each way.
        let (a, b) = (png_header(16, 24), png_header(MAX_SIDE, MAX_SIDE));
        let files = [("x/b.png", &b[..]), ("a.png", &a[..])];
        let table = ImageTable::from_files(&bundle(&[]), files).unwrap_or_default();
        let sizes: Vec<_> = table.images.iter().collect();
        assert_eq!(
            sizes,
            [
                (
                    &"a.png",
                    &ImageInfo {
                        width: 16,
                        height: 24
                    }
                ),
                (
                    &"x/b.png",
                    &ImageInfo {
                        width: MAX_SIDE,
                        height: MAX_SIDE
                    }
                ),
            ]
        );
        assert_eq!(
            ImageTable::from_files(&bundle(&[]), []),
            Ok(ImageTable::default())
        );
    }

    #[test]
    fn ids_name_images_by_path() {
        let png = png_header(16, 24);
        let files = [("a.png", &png[..]), ("b.png", &png[..])];
        let table = ImageTable::from_files(&bundle(&[]), files).unwrap_or_default();
        let a = table.id("a.png");
        assert_eq!(a.map(ImageId::path), Some("a.png"));
        assert_ne!(a, table.id("b.png"));
        assert_eq!(table.id("c.png"), None);
        let size = ImageInfo {
            width: 16,
            height: 24,
        };
        assert_eq!(a.and_then(|id| table.info(id)), Some(size));
        let paths: Vec<_> = table.ids().map(ImageId::path).collect();
        assert_eq!(paths, ["a.png", "b.png"]);
        // An id of another table's image has no size here.
        assert_eq!(a.and_then(|id| ImageTable::default().info(id)), None);
    }

    #[test]
    fn a_one_bit_picture_in_a_pngs_place_has_the_pngs_size() {
        // 4 × 2 pixels, made from a 2 × 1 PNG.
        let bits = crate::bitmap::encode((4, 2), (2, 1), &[&[0, 0]], None).unwrap();
        let mut broken = bits.clone();
        broken.pop();
        let files = [("a.png", &bits[..]), ("b.png", &broken[..])];
        let table = ImageTable::from_files(&bundle(&[]), files[..1].iter().copied());
        let a = table.as_ref().ok().and_then(|t| t.id("a.png"));
        let size = a.and_then(|id| table.as_ref().ok()?.info(id));
        assert_eq!(
            size,
            Some(ImageInfo {
                width: 2,
                height: 1
            })
        );
        assert_eq!(
            messages(ImageTable::from_files(&bundle(&[]), files)),
            ["base/b.png: a one-bit picture's size doesn't match its header"]
        );
    }

    #[test]
    fn every_bad_file_is_reported_by_name() {
        let ok = png_header(1, 1);
        let (wide, tall) = (png_header(MAX_SIDE + 1, 8), png_header(8, MAX_SIDE + 1));
        let (flat, thin) = (png_header(8, 0), png_header(0, 8));
        let files = [
            ("images/ok.png", &ok[..]),
            ("images/text.png", &b"not an image"[..]),
            ("images/wide.png", &wide[..]),
            ("images/tall.png", &tall[..]),
            ("images/flat.png", &flat[..]),
            ("images/thin.png", &thin[..]),
        ];
        assert_eq!(
            messages(ImageTable::from_files(&bundle(&[]), files)),
            [
                "base/images/text.png: not a valid PNG file",
                "base/images/wide.png: image is 4097×8 px; neither side may be over 4096",
                "base/images/tall.png: image is 8×4097 px; neither side may be over 4096",
                "base/images/flat.png: image is 8×0 px; both sides must be greater than 0",
                "base/images/thin.png: image is 0×8 px; both sides must be greater than 0",
            ]
        );
    }

    #[test]
    fn load_reads_every_png_of_the_bundle_but_the_skipped() {
        let files: &'static [(&'static str, &'static [u8])] = Box::leak(Box::new([
            ("data/a.ron", &b"()"[..]),
            ("fonts/atlas.png", leaked(png_header(128, 64))),
            ("images/card.png", leaked(png_header(16, 16))),
            ("units/Big.PNG", leaked(png_header(32, 48))),
        ]));
        let bundle = bundle(files);
        let table = ImageTable::load(&bundle, &["fonts/atlas.png"]).unwrap_or_default();
        let paths: Vec<_> = table.ids().map(ImageId::path).collect();
        assert_eq!(paths, ["images/card.png", "units/Big.PNG"]);
        let all = ImageTable::load(&bundle, &[]).unwrap_or_default();
        assert_eq!(all.images.len(), 3);
        let broken: &'static [(&'static str, &'static [u8])] =
            &[("images/bad.png", b"not an image")];
        assert_eq!(
            messages(ImageTable::load(&self::bundle(broken), &[])),
            ["base/images/bad.png: not a valid PNG file"]
        );
    }

    /// A size and a table as games build theirs, in `const` items.
    const SIZE: ImageInfo = ImageInfo::new(16, 8);
    const NONE: ImageTable = ImageTable::new(BTreeMap::new());

    #[test]
    fn the_constructors_give_the_literals() {
        assert_eq!(
            SIZE,
            ImageInfo {
                width: 16,
                height: 8,
            }
        );
        assert_eq!(ImageInfo::checked(16, 8), Ok(SIZE));
        assert_eq!(NONE, ImageTable::default());
        assert_eq!(
            ImageTable::new([("a.png", SIZE)].into()),
            ImageTable {
                images: [("a.png", SIZE)].into(),
            }
        );
    }
}
