//! More picture formats (ADR-0007 §3): JPEG, WebP, QOI and
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
    /// never does. A file whose frame says its sections end past 4 GiB
    /// from the codestream's start is refused on every target.
    JpegXl,
    /// JPEG XL read for its size (`jpeg-xl`, `.jxl`), with the `jpeg-xl`
    /// feature: its image header, after its frames' tables of contents
    /// are checked as [`JpegXl`] checks them.
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

    /// The furthest a frame's sections may end, in bytes from the
    /// codestream's start: the most a 32-bit `usize` holds.
    pub(super) const MOST_BYTES: u64 = u32::MAX as u64;

    /// Refuses `bytes` if a frame's sections end past [`MOST_BYTES`],
    /// before `jxl-oxide` reads them: `jxl-frame` 0.13.3 adds a frame's
    /// section sizes in a `usize` (`Toc::parse`), which overflows on a
    /// 32-bit target, wasm32 among them. Refused on every target, so a
    /// file reads alike on each. It goes, with [`sections`], once a
    /// release of `jxl-frame` refuses such a file itself.
    fn sections_fit(bytes: &[u8]) -> Decoded<()> {
        match sections::first_end_past(&sections::codestream(bytes), MOST_BYTES) {
            Some(_) => Err(invalid(
                "its frame's sections end past 4 GiB from its start",
            )),
            None => Ok(()),
        }
    }

    pub(super) fn size(bytes: &[u8]) -> Decoded<(u32, u32)> {
        sections_fit(bytes)?;
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
        sections_fit(bytes)?;
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

    /// A walk over a codestream's frames that reads where each one's
    /// sections end, with `jxl-oxide`'s own parsers of its headers and
    /// tables of contents, and none of their sections: [`sections_fit`].
    pub(super) mod sections {
        use crate::prelude::*;
        use jxl_bitstream::{Bitstream, ContainerParser, ParseEvent, U};
        use jxl_oxide::{FrameHeader, ImageHeader};
        use jxl_oxide_common::Bundle;

        /// The codestream of `bytes`, a bare one or a container's boxes
        /// of it, as far as it can be read.
        pub(in super::super) fn codestream(bytes: &[u8]) -> Vec<u8> {
            let mut stream = Vec::new();
            let mut container = ContainerParser::new();
            for event in container.feed_bytes(bytes) {
                match event {
                    Ok(ParseEvent::Codestream(part)) => stream.extend_from_slice(part),
                    Ok(_) => {}
                    Err(_) => break,
                }
            }
            stream
        }

        /// Where the first frame of `stream` whose sections end past
        /// `most` (bytes from its start) says they end. `None` if none
        /// does, as far as the walk reads: it stops at the last frame, at
        /// a frame cut short and at bytes it can't read, which the decoder
        /// then refuses itself, for it reads them alike.
        pub(in super::super) fn first_end_past(stream: &[u8], most: u64) -> Option<u64> {
            let mut bits = Bitstream::new(stream);
            let image = ImageHeader::parse(&mut bits, ()).ok()?;
            if image.metadata.colour_encoding.want_icc() {
                jxl_color::icc::read_icc(&mut bits).ok()?;
            }
            // A preview frame comes first, if there is one, and then the
            // image's frames whatever it says.
            let mut preview = image.metadata.preview.is_some();
            let mut start = 0;
            loop {
                bits.zero_pad_to_byte().ok()?;
                let header = FrameHeader::parse(&mut bits, &image).ok()?;
                if !sized_to_read(&header) {
                    return None;
                }
                let sizes = sections_bytes(&mut bits, &header)?;
                let end = (start + bits.num_read_bits() / 8) as u64 + sizes;
                if end > most {
                    return Some(end);
                }
                if (header.is_last && !preview) || end > stream.len() as u64 {
                    return None;
                }
                preview = false;
                start = usize::try_from(end).ok()?;
                bits = Bitstream::new(&stream[start..]);
            }
        }

        /// Whether `jxl-frame`'s `Frame::parse` reads on from `header` to
        /// its table of contents: 1 to 2³⁰ px a side, 2⁴⁰ px at most. It
        /// refuses the others first, and `FrameHeader::num_groups`
        /// overflows on some of them.
        fn sized_to_read(header: &FrameHeader) -> bool {
            let side = 1..=1_u64 << 30;
            let (width, height) = (u64::from(header.width), u64::from(header.height));
            side.contains(&width) && side.contains(&height) && width * height <= 1 << 40
        }

        /// The bytes of the sections a frame's table of contents lists,
        /// read as `jxl-frame`'s `Toc::parse` reads it, from `bits` just
        /// after the frame's `header`.
        fn sections_bytes(bits: &mut Bitstream, header: &FrameHeader) -> Option<u64> {
            let (groups, passes) = (header.num_groups(), header.passes.num_passes);
            let entries = if groups == 1 && passes == 1 {
                1
            } else {
                let lf_groups = u64::from(header.num_lf_groups());
                u32::try_from(2 + lf_groups + u64::from(groups) * u64::from(passes)).ok()?
            };
            if entries > 1 << 16 {
                return None;
            }
            if bits.read_bool().ok()? {
                // Its sections' order, which the walk steps over.
                let mut decoder = jxl_coding::Decoder::parse(bits, 8).ok()?;
                decoder.begin(bits).ok()?;
                jxl_coding::read_permutation(bits, &mut decoder, entries, 0).ok()?;
                decoder.finalize().ok()?;
            }
            bits.zero_pad_to_byte().ok()?;
            let mut total = 0;
            for _ in 0..entries {
                let size = bits.read_u32(U(10), 1024 + U(14), 17408 + U(22), 4_211_712 + U(30));
                total += u64::from(size.ok()?);
            }
            bits.zero_pad_to_byte().ok()?;
            Some(total)
        }
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

    /// `jpeg_xl_past_4_gib`'s image header, `frame` in place of its
    /// frame's header and the start of its table of contents (byte 4),
    /// then its five sections of `sizes`, each written as the fourth kind
    /// of size.
    #[cfg(feature = "jpeg-xl")]
    fn jpeg_xl_with(frame: &[u8], sizes: [u32; 5]) -> Vec<u8> {
        let mut bytes = crate::format::codec_tests::jpeg_xl_past_4_gib()[..4].to_vec();
        bytes.extend_from_slice(frame);
        for size in sizes {
            bytes.extend((3 | (size - 4_211_712) << 2).to_le_bytes());
        }
        bytes
    }

    #[cfg(feature = "jpeg-xl")]
    #[test]
    fn a_jpeg_xl_walk_finds_where_its_frames_sections_end() {
        use crate::format::codec_tests::{JPEG_XL, JPEG_XL_LOSSLESS};
        use jpeg_xl::sections::{codestream, first_end_past};
        // A one-frame file's sections end where its bytes do.
        for fixture in [JPEG_XL, JPEG_XL_LOSSLESS] {
            let (stream, end) = (codestream(fixture), fixture.len() as u64);
            assert_eq!(stream, fixture);
            assert_eq!(first_end_past(&stream, end - 1), Some(end));
            assert_eq!(first_end_past(&stream, end), None);
        }
        // A container's codestream is its `jxlc` box's.
        let mut boxed = b"\0\0\0\x0cJXL \r\n\x87\n\0\0\0\x14ftypjxl \0\0\0\0jxl ".to_vec();
        boxed.extend(u32::try_from(8 + JPEG_XL.len()).unwrap_or(0).to_be_bytes());
        boxed.extend(b"jxlc");
        boxed.extend(JPEG_XL);
        assert_eq!(codestream(&boxed), JPEG_XL);
        // 25 bytes of headers and 4 GiB less 25 of sections end at the
        // last byte a 32-bit `usize` counts; a byte more is past it.
        let most = jpeg_xl::MOST_BYTES;
        let big = 1_072_688_889;
        let at_most = jpeg_xl_with(&[1], [big, big, big, big, 4_211_714]);
        assert_eq!(first_end_past(&at_most, most), None);
        let past = jpeg_xl_with(&[1], [big, big, big, big, 4_211_715]);
        assert_eq!(first_end_past(&past, most), Some(most + 1));
        // A table of contents with an order (1, after the frame's header
        // all default): no LZ77 (0); its 8 contexts in one cluster, simply
        // (1, entries of 0 bits: 00); a prefix code (1) splitting at 2¹⁵
        // (15: 1111) and of one symbol (0), so every number read is 0 and
        // the order is empty. Its sizes are read after it.
        let ordered = jpeg_xl_with(&[0b1100_1011, 0b0000_0111], [4_211_712; 5]);
        let end = 26 + 5 * 4_211_712;
        assert_eq!(first_end_past(&ordered, end - 1), Some(end));
        assert_eq!(first_end_past(&ordered, end), None);
    }

    #[cfg(feature = "jpeg-xl")]
    #[test]
    fn a_jpeg_xl_walk_leaves_a_frame_too_big_to_the_decoder() {
        use jpeg_xl::sections::first_end_past;
        // 2³⁰ px tall (not small: 0, the fourth kind: 11, 30 bits of 1),
        // then a ratio (3 bits), metadata all default (11), zeros to the
        // byte, and a frame's header all default: 2⁶⁰ px at 1:1 (ratio
        // 1), 2³¹ px wide at 2:1 (ratio 7). Counting their groups would
        // overflow; `jxl-frame` refuses them before, and so the walk
        // leaves them.
        for last in [0b0011_0011, 0b0011_1111] {
            let big = [0xff, 0x0a, 0xfe, 0xff, 0xff, 0xff, last, 0b0000_0001];
            assert_eq!(first_end_past(&big, 0), None);
            let refused = jpeg_xl::size(&big);
            assert!(refused.is_err_and(|e| e.starts_with("not a valid JPEG XL file")));
        }
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
