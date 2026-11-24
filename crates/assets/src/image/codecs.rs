//! More picture formats (ticket 2203, ADR-0007 §3): JPEG, WebP, QOI and
//! JPEG XL, each behind its own feature. Each is two formats, as PNG is:
//! one reads the size ([`ImageInfo`]) from the file's header, the other
//! the pixels ([`Image`]), after checking that size, so a picture too big
//! is refused before its pixels are decoded.
//!
//! The decoders are the outside crates in `Cargo.toml`, with no SIMD and no
//! threads, so the same bytes give the same pixels on every target
//! (ADR-0007 §5), except as JPEG XL's entry says.
use crate::prelude::*;

use super::{Image, ImageInfo, MAX_PNG_BYTES};
use crate::error::ContentError;
use crate::format::{AssetPath, DecodeCx, Format};

/// What a picture decoder gives for a file's bytes.
type Decoded<T> = Result<T, String>;

/// A codec's reading of a file's size, and of its pixels once that size
/// is checked.
type SizeFn = fn(&[u8]) -> Decoded<(u32, u32)>;
type PixelsFn = fn(&[u8], ImageInfo) -> Decoded<Vec<u8>>;

/// The size `size` reads from `bytes` (the file `file`), checked.
fn read_size(
    file: &AssetPath,
    bytes: &[u8],
    size: SizeFn,
    _pixels: PixelsFn,
) -> Result<ImageInfo, Vec<ContentError>> {
    let error = |message: String| vec![ContentError::new(file.shown(), message)];
    let (width, height) = size(bytes).map_err(error)?;
    ImageInfo::checked(width, height).map_err(error)
}

/// The picture a codec reads from `bytes` (the file `file`): its size
/// from `size`, checked, then its RGBA pixels from `pixels`, which must be
/// as many as the size says.
fn read_image(
    file: &AssetPath,
    bytes: &[u8],
    size: SizeFn,
    pixels: PixelsFn,
) -> Result<Image, Vec<ContentError>> {
    let checked = read_size(file, bytes, size, pixels)?;
    let error = |message: String| vec![ContentError::new(file.shown(), message)];
    let rgba = pixels(bytes, checked).map_err(error)?;
    let wanted = checked.width as usize * checked.height as usize * 4;
    if rgba.len() != wanted {
        return Err(error(format!(
            "its pixels are {} bytes; a {}×{} px picture has {wanted}",
            rgba.len(),
            checked.width,
            checked.height
        )));
    }
    Ok(Image {
        width: checked.width,
        height: checked.height,
        rgba,
    })
}

/// `samples`, `channels` bytes a pixel (gray, gray and alpha, RGB or
/// RGBA), as RGBA; `None` for another count.
#[cfg(any(feature = "webp", feature = "jpeg-xl"))]
fn to_rgba(samples: &[u8], channels: usize) -> Option<Vec<u8>> {
    let pixel = |p: &[u8]| match *p {
        [v] => [v, v, v, 255],
        [v, a] => [v, v, v, a],
        [r, g, b] => [r, g, b, 255],
        [r, g, b, a] => [r, g, b, a],
        _ => [0; 4],
    };
    (1..=4)
        .contains(&channels)
        .then(|| samples.chunks_exact(channels).flat_map(pixel).collect())
}

/// The two formats of one codec: `$pixels` reads [`Image`], `$info`
/// [`ImageInfo`], both with the codec module `$codec`'s `ID`,
/// `EXTENSIONS`, `sniff`, `size` and `pixels`.
macro_rules! codec_formats {
    ($codec:ident, $feature:literal, $(#[$pixels_doc:meta])* $pixels:ident, $(#[$info_doc:meta])* $info:ident) => {
        codec_formats!(@one $codec, $feature, $(#[$pixels_doc])* $pixels, Image, read_image);
        codec_formats!(@one $codec, $feature, $(#[$info_doc])* $info, ImageInfo, read_size);
    };
    (@one $codec:ident, $feature:literal, $(#[$doc:meta])* $name:ident, $asset:ty, $read:ident) => {
        $(#[$doc])*
        #[cfg(feature = $feature)]
        #[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
        pub struct $name;

        #[cfg(feature = $feature)]
        impl Format for $name {
            type Asset = $asset;

            fn id(&self) -> &'static str {
                $codec::ID
            }

            fn extensions(&self) -> &'static [&'static str] {
                $codec::EXTENSIONS
            }

            fn sniff(&self, bytes: &[u8]) -> bool {
                $codec::sniff(bytes)
            }

            fn max_bytes(&self) -> usize {
                MAX_PNG_BYTES
            }

            fn decode(
                &self,
                file: &AssetPath,
                bytes: &[u8],
                _cx: &DecodeCx<'_>,
            ) -> Result<$asset, Vec<ContentError>> {
                $read(file, bytes, $codec::size, $codec::pixels)
            }
        }
    };
}

codec_formats!(
    jpeg,
    "jpeg",
    /// JPEG read for its pixels (`jpeg`, `.jpg` and `.jpeg`), with the
    /// `jpeg` feature: baseline and progressive, gray, RGB and CMYK, made
    /// opaque RGBA. Decoded by `zune-jpeg` in integers.
    Jpeg,
    /// JPEG read for its size (`jpeg`, `.jpg` and `.jpeg`), with the `jpeg`
    /// feature: only its headers.
    JpegInfo
);

codec_formats!(
    webp,
    "webp",
    /// WebP read for its pixels (`webp`, `.webp`), with the `webp` feature:
    /// lossy or lossless, with or without alpha; an animation gives its
    /// first frame. Decoded by `image-webp` in integers.
    WebP,
    /// WebP read for its size (`webp`, `.webp`), with the `webp` feature:
    /// only its headers.
    WebPInfo
);

codec_formats!(
    qoi,
    "qoi",
    /// QOI read for its pixels (`qoi`, `.qoi`), with the `qoi` feature: RGB
    /// or RGBA, its colour space tag ignored.
    Qoi,
    /// QOI read for its size (`qoi`, `.qoi`), with the `qoi` feature: only
    /// its 14-byte header.
    QoiInfo
);

codec_formats!(
    jpeg_xl,
    "jpeg-xl",
    /// JPEG XL read for its pixels (`jpeg-xl`, `.jxl`), with the `jpeg-xl`
    /// feature: a bare codestream or a container, lossless or lossy, its
    /// first frame in sRGB. Decoded by `jxl-oxide`, which renders a lossy
    /// picture in floats with SIMD it picks at run time: the fixture's
    /// pixels are pinned on x64, ARM64 and wasm32 (ADR-0007 §5), but a
    /// lossy picture could round a pixel apart on another; a lossless one
    /// never does.
    JpegXl,
    /// JPEG XL read for its size (`jpeg-xl`, `.jxl`), with the `jpeg-xl`
    /// feature: only its image header.
    JpegXlInfo
);

/// JPEG, by `zune-jpeg`.
#[cfg(feature = "jpeg")]
mod jpeg {
    use super::{Decoded, ImageInfo};
    use crate::prelude::*;
    use zune_jpeg::JpegDecoder;
    use zune_jpeg::zune_core::bytestream::ZCursor;
    use zune_jpeg::zune_core::colorspace::ColorSpace;
    use zune_jpeg::zune_core::options::DecoderOptions;

    pub(super) const ID: &str = "jpeg";
    pub(super) const EXTENSIONS: &[&str] = &["jpg", "jpeg"];

    /// A start-of-image marker, then another marker.
    pub(super) fn sniff(bytes: &[u8]) -> bool {
        bytes.starts_with(&[0xff, 0xd8, 0xff])
    }

    fn decoder(bytes: &[u8]) -> JpegDecoder<ZCursor<&[u8]>> {
        let options = DecoderOptions::new_safe().jpeg_set_out_colorspace(ColorSpace::RGBA);
        JpegDecoder::new_with_options(ZCursor::new(bytes), options)
    }

    fn invalid(error: impl core::fmt::Display) -> String {
        format!("not a valid JPEG file: {error}")
    }

    pub(super) fn size(bytes: &[u8]) -> Decoded<(u32, u32)> {
        let mut decoder = decoder(bytes);
        decoder.decode_headers().map_err(invalid)?;
        let (width, height) = decoder
            .dimensions()
            .ok_or_else(|| invalid("no frame header"))?;
        let side = |side: usize| u32::try_from(side).unwrap_or(u32::MAX);
        Ok((side(width), side(height)))
    }

    pub(super) fn pixels(bytes: &[u8], _info: ImageInfo) -> Decoded<Vec<u8>> {
        decoder(bytes).decode().map_err(invalid)
    }
}

/// WebP, by `image-webp`.
#[cfg(feature = "webp")]
mod webp {
    use super::{Decoded, ImageInfo, to_rgba};
    use crate::prelude::*;
    use image_webp::WebPDecoder;
    use std::io::Cursor;

    pub(super) const ID: &str = "webp";
    pub(super) const EXTENSIONS: &[&str] = &["webp"];

    /// A RIFF file of WebP.
    pub(super) fn sniff(bytes: &[u8]) -> bool {
        bytes.get(..4) == Some(b"RIFF") && bytes.get(8..12) == Some(b"WEBP")
    }

    fn decoder(bytes: &[u8]) -> Decoded<WebPDecoder<Cursor<&[u8]>>> {
        WebPDecoder::new(Cursor::new(bytes)).map_err(invalid)
    }

    fn invalid(error: impl core::fmt::Display) -> String {
        format!("not a valid WebP file: {error}")
    }

    pub(super) fn size(bytes: &[u8]) -> Decoded<(u32, u32)> {
        Ok(decoder(bytes)?.dimensions())
    }

    pub(super) fn pixels(bytes: &[u8], _info: ImageInfo) -> Decoded<Vec<u8>> {
        let mut decoder = decoder(bytes)?;
        let channels = if decoder.has_alpha() { 4 } else { 3 };
        let size = decoder
            .output_buffer_size()
            .ok_or_else(|| invalid("its size overflows"))?;
        let mut samples = vec![0; size];
        decoder.read_image(&mut samples).map_err(invalid)?;
        to_rgba(&samples, channels).ok_or_else(|| invalid("no channels"))
    }
}

/// QOI, by `qoi`.
#[cfg(feature = "qoi")]
mod qoi {
    use super::{Decoded, ImageInfo};
    use crate::prelude::*;
    use qoi::{Channels, Decoder};

    pub(super) const ID: &str = "qoi";
    pub(super) const EXTENSIONS: &[&str] = &["qoi"];

    /// QOI's magic bytes.
    pub(super) fn sniff(bytes: &[u8]) -> bool {
        bytes.starts_with(b"qoif")
    }

    fn invalid(error: impl core::fmt::Display) -> String {
        format!("not a valid QOI file: {error}")
    }

    pub(super) fn size(bytes: &[u8]) -> Decoded<(u32, u32)> {
        let header = qoi::decode_header(bytes).map_err(invalid)?;
        Ok((header.width, header.height))
    }

    pub(super) fn pixels(bytes: &[u8], _info: ImageInfo) -> Decoded<Vec<u8>> {
        Decoder::new(bytes)
            .map_err(invalid)?
            .with_channels(Channels::Rgba)
            .decode_to_vec()
            .map_err(invalid)
    }
}

/// JPEG XL, by `jxl-oxide`.
#[cfg(feature = "jpeg-xl")]
mod jpeg_xl {
    use super::{Decoded, ImageInfo, to_rgba};
    use crate::prelude::*;
    use jxl_oxide::{
        AllocTracker, EnumColourEncoding, InitializeResult, JxlImage, RenderingIntent,
    };

    pub(super) const ID: &str = "jpeg-xl";
    pub(super) const EXTENSIONS: &[&str] = &["jxl"];

    /// The most bytes the decoder may hold at once: 1 GiB, room for a
    /// [`MAX_SIDE`](super::super::MAX_SIDE) square's frame in floats and
    /// what renders it.
    const MAX_HELD: usize = 1 << 30;

    /// A container's first box: its size (12), `JXL ` and four bytes.
    const CONTAINER: [u8; 12] = [0, 0, 0, 12, b'J', b'X', b'L', b' ', 13, 10, 0x87, 10];

    /// A bare codestream's signature, or a container's first box.
    pub(super) fn sniff(bytes: &[u8]) -> bool {
        bytes.starts_with(&[0xff, 0x0a]) || bytes.starts_with(&CONTAINER)
    }

    fn invalid(error: impl core::fmt::Display) -> String {
        format!("not a valid JPEG XL file: {error}")
    }

    /// A decoder that holds at most `max_held` bytes at once.
    fn builder(max_held: usize) -> jxl_oxide::JxlImageBuilder {
        JxlImage::builder().alloc_tracker(AllocTracker::with_limit(max_held))
    }

    pub(super) fn size(bytes: &[u8]) -> Decoded<(u32, u32)> {
        let mut image = builder(MAX_HELD).build_uninit();
        image.feed_bytes(bytes).map_err(invalid)?;
        match image.try_init().map_err(invalid)? {
            InitializeResult::Initialized(image) => Ok((image.width(), image.height())),
            InitializeResult::NeedMoreData(_) => Err(invalid("its header is cut short")),
        }
    }

    pub(super) fn pixels(bytes: &[u8], info: ImageInfo) -> Decoded<Vec<u8>> {
        pixels_within(bytes, info, MAX_HELD)
    }

    /// [`pixels`], holding at most `max_held` bytes at once.
    pub(super) fn pixels_within(
        bytes: &[u8],
        info: ImageInfo,
        max_held: usize,
    ) -> Decoded<Vec<u8>> {
        let mut image = builder(max_held).read(bytes).map_err(invalid)?;
        image.request_color_encoding(EnumColourEncoding::srgb(RenderingIntent::Relative));
        let render = image.render_frame(0).map_err(invalid)?;
        let mut stream = render.stream();
        if (stream.width(), stream.height()) != (info.width, info.height) {
            return Err(invalid("its frame isn't the size of its header"));
        }
        let channels = stream.channels() as usize;
        let mut samples = vec![0_u8; info.width as usize * info.height as usize * channels];
        stream.write_to_buffer(&mut samples);
        to_rgba(&samples, channels)
            .ok_or_else(|| invalid(format!("{channels} channels; must be 1 to 4")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_codec_must_give_as_many_pixels_as_its_size_says() {
        let file = AssetPath::new("a.x");
        let two_by_one = |_: &[u8]| Ok((2, 1));
        let right = |_: &[u8], _: ImageInfo| Ok(vec![7; 8]);
        let short = |_: &[u8], _: ImageInfo| Ok(vec![7; 7]);
        let broken = |_: &[u8], _: ImageInfo| Err("broken".to_owned());
        let decoded = read_image(&file, b"", two_by_one, right).unwrap_or_default();
        assert_eq!(
            (decoded.width, decoded.height, decoded.rgba),
            (2, 1, vec![7; 8])
        );
        let message = |result: Result<Image, Vec<ContentError>>| {
            result
                .err()
                .unwrap_or_default()
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        };
        assert_eq!(
            message(read_image(&file, b"", two_by_one, short)),
            ["a.x: its pixels are 7 bytes; a 2×1 px picture has 8"]
        );
        assert_eq!(
            message(read_image(&file, b"", two_by_one, broken)),
            ["a.x: broken"]
        );
        // The size is checked before the pixels are asked for.
        let empty = |_: &[u8]| Ok((0, 1));
        assert_eq!(
            message(read_image(&file, b"", empty, right)),
            ["a.x: image is 0×1 px; both sides must be greater than 0"]
        );
    }

    #[cfg(feature = "jpeg-xl")]
    #[test]
    fn a_jpeg_xl_decoder_holds_no_more_than_it_may() {
        let (bytes, info) = (crate::format::codec_tests::JPEG_XL, ImageInfo::new(20, 12));
        assert!(jpeg_xl::pixels_within(bytes, info, 1 << 30).is_ok());
        let held = jpeg_xl::pixels_within(bytes, info, 1024);
        assert!(held.is_err_and(|e| e.starts_with("not a valid JPEG XL file")));
    }

    #[cfg(any(feature = "webp", feature = "jpeg-xl"))]
    #[test]
    fn samples_of_one_to_four_channels_become_rgba() {
        assert_eq!(to_rgba(&[9, 8], 1), Some(vec![9, 9, 9, 255, 8, 8, 8, 255]));
        assert_eq!(to_rgba(&[9, 5], 2), Some(vec![9, 9, 9, 5]));
        assert_eq!(to_rgba(&[1, 2, 3], 3), Some(vec![1, 2, 3, 255]));
        assert_eq!(to_rgba(&[1, 2, 3, 4], 4), Some(vec![1, 2, 3, 4]));
        assert_eq!(to_rgba(&[1, 2, 3, 4, 5], 5), None);
        assert_eq!(to_rgba(&[1], 0), None);
    }
}
