//! The tests of ticket 2203's formats (JPEG, WebP, QOI, JPEG XL, FLAC and
//! MP3) on their fixtures, `tests/fixtures/formats/`: our own files, made
//! as its `README.md` says. Their golden rows are in `golden_tests.rs`,
//! their fuzz-style tests in `builtin_tests.rs`.
use super::*;
use crate::audio::{Flac, FlacTrack, MAX_SOUND_BYTES, Mp3, Mp3Track, MusicTrack, Sound};
use crate::image::{
    Image, ImageInfo, ImageTable, Jpeg, JpegInfo, JpegXl, JpegXlInfo, MAX_PNG_BYTES, Png, Qoi,
    QoiInfo, WebP, WebPInfo,
};

/// A fixture's bytes, by file name.
macro_rules! fixture {
    ($name:literal) => {
        include_bytes!(concat!("../../tests/fixtures/formats/", $name)).as_slice()
    };
}

/// The source picture, 20 × 12 px, its alpha falling to the right and
/// down: what the lossless fixtures hold.
pub(crate) const PICTURE: &[u8] = fixture!("picture.png");
/// The source picture, opaque: what the lossy JPEG XL and JPEG were made
/// from.
pub(crate) const OPAQUE: &[u8] = fixture!("opaque.png");
/// [`OPAQUE`] as a JPEG of quality 90, its colour at half size (4:2:0).
pub(crate) const JPEG: &[u8] = fixture!("picture.jpg");
/// [`PICTURE`] as a lossy WebP, its alpha lossless.
pub(crate) const WEBP: &[u8] = fixture!("picture.webp");
/// [`PICTURE`] as a lossless WebP.
pub(crate) const WEBP_LOSSLESS: &[u8] = fixture!("lossless.webp");
/// [`PICTURE`] as a QOI.
pub(crate) const QOI: &[u8] = fixture!("picture.qoi");
/// [`OPAQUE`] as a lossy JPEG XL (distance 1).
pub(crate) const JPEG_XL: &[u8] = fixture!("picture.jxl");
/// [`PICTURE`] as a lossless JPEG XL.
pub(crate) const JPEG_XL_LOSSLESS: &[u8] = fixture!("lossless.jxl");
/// [`JPEG`] as libjpeg-turbo's `djpeg` decodes it.
const JPEG_BY_LIBJPEG: &[u8] = fixture!("jpeg-by-libjpeg.png");
/// [`WEBP`] as libwebp's `dwebp` decodes it.
const WEBP_BY_LIBWEBP: &[u8] = fixture!("webp-by-libwebp.png");
/// [`JPEG_XL`] as libjxl's `djxl` decodes it.
const JPEG_XL_BY_LIBJXL: &[u8] = fixture!("jxl-by-libjxl.png");
/// 50 ms (2,205 samples) of a 440 Hz tone on the left and 660 Hz on the
/// right, as FLAC at 44.1 kHz.
pub(crate) const FLAC: &[u8] = fixture!("tone.flac");
/// The same tone, mono, as a 64 kbit/s MP3 with a LAME tag.
pub(crate) const MP3: &[u8] = fixture!("tone.mp3");

/// What `format` reads from `bytes` at `path`, with no bundle.
fn alone<F: Format>(format: &F, path: &str, bytes: &[u8]) -> Result<F::Asset, Vec<ContentError>> {
    format.decode(&AssetPath::new(path), bytes, &DecodeCx::new())
}

/// The errors of `result`, each with its file.
fn shown<T>(result: Result<T, Vec<ContentError>>) -> Vec<String> {
    result
        .err()
        .unwrap_or_default()
        .iter()
        .map(ToString::to_string)
        .collect()
}

/// The pixels of a PNG fixture.
fn png(bytes: &[u8]) -> Image {
    alone(&Png, "a.png", bytes).unwrap_or_default()
}

/// The biggest difference between any channel of any pixel of `a` and `b`,
/// which must be the same size.
fn most_apart(a: &Image, b: &Image) -> u8 {
    assert_eq!((a.width, a.height), (b.width, b.height));
    assert_eq!(a.rgba.len(), b.rgba.len());
    a.rgba
        .iter()
        .zip(&b.rgba)
        .map(|(x, y)| x.abs_diff(*y))
        .max()
        .unwrap_or(0)
}

#[test]
fn each_codec_has_its_id_extensions_and_limit() {
    fn named<F: Format>(format: &F, id: &str, extensions: &[&str], max_bytes: usize) {
        assert_eq!(format.id(), id);
        assert_eq!(format.extensions(), extensions);
        assert_eq!(format.max_bytes(), max_bytes);
    }
    named(&Jpeg, "jpeg", &["jpg", "jpeg"], MAX_PNG_BYTES);
    named(&JpegInfo, "jpeg", &["jpg", "jpeg"], MAX_PNG_BYTES);
    named(&WebP, "webp", &["webp"], MAX_PNG_BYTES);
    named(&WebPInfo, "webp", &["webp"], MAX_PNG_BYTES);
    named(&Qoi, "qoi", &["qoi"], MAX_PNG_BYTES);
    named(&QoiInfo, "qoi", &["qoi"], MAX_PNG_BYTES);
    named(&JpegXl, "jpeg-xl", &["jxl"], MAX_PNG_BYTES);
    named(&JpegXlInfo, "jpeg-xl", &["jxl"], MAX_PNG_BYTES);
    named(&Flac, "flac", &["flac"], MAX_SOUND_BYTES);
    named(&FlacTrack, "flac", &["flac"], MAX_SOUND_BYTES);
    named(&Mp3, "mp3", &["mp3"], MAX_SOUND_BYTES);
    named(&Mp3Track, "mp3", &["mp3"], MAX_SOUND_BYTES);
}

#[test]
fn each_codec_knows_its_magic_bytes_and_no_other() {
    let container = b"\0\0\0\x0cJXL \r\n\x87\n";
    // Each fixture, and the shortest bytes each format knows.
    let files: [&[u8]; 12] = [
        JPEG,
        WEBP,
        QOI,
        JPEG_XL,
        FLAC,
        MP3,
        &[0xff, 0xd8, 0xff],
        b"RIFF\0\0\0\0WEBP",
        b"qoif",
        container,
        b"fLaC",
        b"ID3",
    ];
    for (i, bytes) in files.iter().enumerate() {
        let sniffed = [
            Jpeg.sniff(bytes) && JpegInfo.sniff(bytes),
            WebP.sniff(bytes) && WebPInfo.sniff(bytes),
            Qoi.sniff(bytes) && QoiInfo.sniff(bytes),
            JpegXl.sniff(bytes) && JpegXlInfo.sniff(bytes),
            Flac.sniff(bytes) && FlacTrack.sniff(bytes),
            Mp3.sniff(bytes) && Mp3Track.sniff(bytes),
        ];
        let expected: Vec<bool> = (0..6).map(|format| format == i % 6).collect();
        assert_eq!(sniffed.to_vec(), expected, "file {i}");
    }
    // One byte short, or one byte off, is none of them.
    let near: [&[u8]; 8] = [
        &[0xff, 0xd8],
        &[0xff, 0xd8, 0xfe],
        b"RIFF\0\0\0\0WEBQ",
        b"RIFX\0\0\0\0WEBP",
        b"qoi",
        &container[..11],
        b"fLa",
        b"ID",
    ];
    for bytes in near {
        let sniffed = [
            Jpeg.sniff(bytes),
            WebP.sniff(bytes),
            Qoi.sniff(bytes),
            JpegXl.sniff(bytes),
            Flac.sniff(bytes),
            Mp3.sniff(bytes),
        ];
        assert_eq!(sniffed, [false; 6], "{bytes:?}");
    }
    // An MP3 frame's sync, and a bare JPEG XL codestream.
    assert!(Mp3.sniff(&[0xff, 0xfb, 0x50]));
    assert!(Mp3.sniff(&[0xff, 0xe0]));
    assert!(!Mp3.sniff(&[0xff, 0xd8]));
    assert!(!Mp3.sniff(&[0xff]));
    assert!(JpegXl.sniff(&[0xff, 0x0a, 0x00]));
}

#[test]
fn the_lossless_fixtures_decode_to_their_source() {
    let source = png(PICTURE);
    assert_eq!((source.width, source.height), (20, 12));
    // Alpha falls from 255 at the top left.
    assert_eq!(source.pixel(0, 0).map(|p| p[3]), Some(255));
    assert_eq!(source.pixel(19, 11).map(|p| p[3]), Some(255 - 30 * 8));
    assert_eq!(alone(&Qoi, "a.qoi", QOI), Ok(source.clone()));
    assert_eq!(alone(&WebP, "a.webp", WEBP_LOSSLESS), Ok(source.clone()));
    assert_eq!(alone(&JpegXl, "a.jxl", JPEG_XL_LOSSLESS), Ok(source));
}

#[test]
fn the_lossy_fixtures_decode_as_the_reference_decoders_do() {
    let (picture, opaque) = (png(PICTURE), png(OPAQUE));
    let jpeg = alone(&Jpeg, "a.jpg", JPEG).unwrap_or_default();
    let webp = alone(&WebP, "a.webp", WEBP).unwrap_or_default();
    let jpeg_xl = alone(&JpegXl, "a.jxl", JPEG_XL).unwrap_or_default();
    // Each channel of each pixel as libwebp decodes it, within a step of
    // libjxl, and within four of libjpeg-turbo, whose upsampling of the
    // colour differs.
    let apart = [
        most_apart(&jpeg, &png(JPEG_BY_LIBJPEG)),
        most_apart(&webp, &png(WEBP_BY_LIBWEBP)),
        most_apart(&jpeg_xl, &png(JPEG_XL_BY_LIBJXL)),
    ];
    assert!(apart[0] <= 4 && apart[1] == 0 && apart[2] <= 1, "{apart:?}");
    // And no farther from the source than theirs are (15, 16 and 26).
    let apart = [
        most_apart(&jpeg, &opaque),
        most_apart(&webp, &picture),
        most_apart(&jpeg_xl, &opaque),
    ];
    assert!(
        apart[0] <= 15 && apart[1] <= 16 && apart[2] <= 26,
        "{apart:?}"
    );
    assert!(jpeg.rgba.as_chunks::<4>().0.iter().all(|p| p[3] == 255));
    assert!(jpeg_xl.rgba.as_chunks::<4>().0.iter().all(|p| p[3] == 255));
    // The WebP's alpha is lossless.
    let alpha =
        |image: &Image| -> Vec<u8> { image.rgba.as_chunks::<4>().0.iter().map(|p| p[3]).collect() };
    assert_eq!(alpha(&webp), alpha(&picture));
}

#[test]
fn each_size_format_reads_the_size_its_pixels_have() {
    let size = Ok(ImageInfo::new(20, 12));
    assert_eq!(alone(&JpegInfo, "a.jpg", JPEG), size);
    assert_eq!(alone(&WebPInfo, "a.webp", WEBP), size);
    assert_eq!(alone(&WebPInfo, "a.webp", WEBP_LOSSLESS), size);
    assert_eq!(alone(&QoiInfo, "a.qoi", QOI), size);
    assert_eq!(alone(&JpegXlInfo, "a.jxl", JPEG_XL), size);
    assert_eq!(alone(&JpegXlInfo, "a.jxl", JPEG_XL_LOSSLESS), size);
}

/// `bytes` with the big-endian `u16` at `at` set to `value`.
fn with_u16(bytes: &[u8], at: usize, value: u16) -> Vec<u8> {
    let mut bytes = bytes.to_vec();
    bytes[at..at + 2].copy_from_slice(&value.to_be_bytes());
    bytes
}

/// Where the JPEG fixture's frame header (`SOF0` or `SOF2`) starts.
fn frame_header(jpeg: &[u8]) -> usize {
    jpeg.windows(2)
        .position(|marker| marker == [0xff, 0xc0] || marker == [0xff, 0xc2])
        .unwrap_or(0)
}

#[test]
fn a_picture_too_big_is_refused_before_its_pixels_are_decoded() {
    // A QOI header that claims 5000 × 1 px, with no pixels after it.
    let mut wide = b"qoif".to_vec();
    wide.extend(5000_u32.to_be_bytes());
    wide.extend(1_u32.to_be_bytes());
    wide.extend([4, 0]);
    let refused = ["a.qoi: image is 5000×1 px; neither side may be over 4096"];
    assert_eq!(shown(alone(&QoiInfo, "a.qoi", &wide)), refused);
    assert_eq!(shown(alone(&Qoi, "a.qoi", &wide)), refused);
    // The JPEG's frame header made 4097 px tall: its height, then width.
    let sof = frame_header(JPEG);
    assert!(sof > 0);
    let tall = with_u16(JPEG, sof + 5, 4097);
    let refused = ["a.jpg: image is 20×4097 px; neither side may be over 4096"];
    assert_eq!(shown(alone(&JpegInfo, "a.jpg", &tall)), refused);
    assert_eq!(shown(alone(&Jpeg, "a.jpg", &tall)), refused);
    // A side of 0 is refused too, by the size alone.
    let flat = with_u16(JPEG, sof + 7, 0);
    assert_eq!(
        shown(alone(&JpegInfo, "a.jpg", &flat)).len(),
        1,
        "{:?}",
        shown(alone(&JpegInfo, "a.jpg", &flat))
    );
}

#[test]
fn broken_bytes_are_an_error_that_names_the_format() {
    let cut = |bytes: &[u8]| bytes[..bytes.len() / 2].to_vec();
    let starts = |errors: Vec<String>, start: &str| {
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(errors[0].starts_with(start), "{errors:?}");
    };
    starts(
        shown(alone(&JpegInfo, "a.jpg", b"\xff\xd8\xff")),
        "a.jpg: not a valid JPEG file",
    );
    starts(
        shown(alone(&Jpeg, "a.jpg", &cut(JPEG)[..20])),
        "a.jpg: not a valid JPEG file",
    );
    starts(
        shown(alone(&WebPInfo, "a.webp", b"RIFF")),
        "a.webp: not a valid WebP file",
    );
    starts(
        shown(alone(&WebP, "a.webp", &cut(WEBP))),
        "a.webp: not a valid WebP file",
    );
    starts(
        shown(alone(&QoiInfo, "a.qoi", b"qoif")),
        "a.qoi: not a valid QOI file",
    );
    starts(
        shown(alone(&Qoi, "a.qoi", &cut(QOI))),
        "a.qoi: not a valid QOI file",
    );
    starts(
        shown(alone(&JpegXlInfo, "a.jxl", &[0xff, 0x0a])),
        "a.jxl: not a valid JPEG XL file",
    );
    starts(
        shown(alone(&JpegXl, "a.jxl", &cut(JPEG_XL))),
        "a.jxl: not a valid JPEG XL file",
    );
    assert_eq!(
        shown(alone(&Flac, "a.flac", b"fLaC")),
        ["a.flac: not a FLAC file"]
    );
    assert_eq!(
        shown(alone(&Mp3, "a.mp3", b"ID3")),
        ["a.mp3: not an MP3 file"]
    );
}

#[test]
fn flac_reads_a_sounds_header_and_a_tracks_length() {
    assert_eq!(alone(&Flac, "a.flac", FLAC), Ok(Sound::stereo(44_100)));
    let track = MusicTrack {
        channels: 2,
        sample_rate: 44_100,
        length_ms: 50,
    };
    assert_eq!(alone(&FlacTrack, "a.flac", FLAC), Ok(track));
    // STREAMINFO's body starts at byte 8: its sample rate is the 20 bits
    // from byte 18, then three of channels, five of bits a sample, and 36
    // of the sample count.
    let mut fast = FLAC.to_vec();
    let rate = 48_000_u32 << 4 | u32::from(fast[20] & 0x0f);
    fast[18..21].copy_from_slice(&rate.to_be_bytes()[1..]);
    let refused = ["a.flac: 48000 Hz; must be 44100 Hz"];
    assert_eq!(shown(alone(&Flac, "a.flac", &fast)), refused);
    assert_eq!(shown(alone(&FlacTrack, "a.flac", &fast)), refused);
    // A count of 0 is unknown: a sound still, but no track.
    let mut unknown = FLAC.to_vec();
    unknown[21] &= 0xf0;
    unknown[22..26].fill(0);
    assert_eq!(alone(&Flac, "a.flac", &unknown), Ok(Sound::stereo(44_100)));
    assert_eq!(
        shown(alone(&FlacTrack, "a.flac", &unknown)),
        ["a.flac: its length can't be read (its STREAMINFO has no sample count)"]
    );
}

#[test]
fn mp3_reads_a_sounds_header_and_a_tracks_length_without_the_encoders_padding() {
    assert_eq!(alone(&Mp3, "a.mp3", MP3), Ok(Sound::mono(44_100)));
    // 2,205 samples: the LAME tag's delay and padding are taken off.
    let track = MusicTrack {
        channels: 1,
        sample_rate: 44_100,
        length_ms: 50,
    };
    assert_eq!(alone(&Mp3Track, "a.mp3", MP3), Ok(track));
    assert_eq!(
        shown(alone(&Mp3Track, "a.mp3", b"\xff\xfb")),
        ["a.mp3: not an MP3 file"]
    );
}

#[test]
fn the_standard_registry_knows_every_codec_by_its_magic_bytes() {
    // Its ids are in `tests.rs`, beside the other standard formats'.
    let formats = Formats::standard();
    let cx = DecodeCx::new();
    let size = Ok(ImageInfo::new(20, 12));
    for bytes in [JPEG, WEBP, QOI, JPEG_XL] {
        assert_eq!(
            formats.decode::<ImageInfo>(&AssetPath::new("a.png"), bytes, &cx),
            size
        );
    }
    let file = AssetPath::new("a.ogg");
    assert_eq!(
        formats.decode::<Sound>(&file, FLAC, &cx),
        Ok(Sound::stereo(44_100))
    );
    assert_eq!(
        formats.decode::<Sound>(&file, MP3, &cx),
        Ok(Sound::mono(44_100))
    );
}

#[test]
fn the_image_table_sizes_every_codecs_files_and_load_finds_them() {
    let files = super::builtin_tests::bundle(vec![
        ("images/a.jpg", JPEG.to_vec()),
        ("images/b.JPEG", JPEG.to_vec()),
        ("images/c.webp", WEBP.to_vec()),
        ("images/d.qoi", QOI.to_vec()),
        ("images/e.jxl", JPEG_XL.to_vec()),
        ("music/f.flac", FLAC.to_vec()),
    ]);
    let formats = Formats::standard();
    let table = ImageTable::load_with(files, &formats, &[]).unwrap_or_default();
    let sizes: Vec<(&str, ImageInfo)> = table.images.into_iter().collect();
    let size = ImageInfo::new(20, 12);
    assert_eq!(
        sizes,
        [
            ("images/a.jpg", size),
            ("images/b.JPEG", size),
            ("images/c.webp", size),
            ("images/d.qoi", size),
            ("images/e.jxl", size),
        ]
    );
    let pixels = formats.load::<Image>(files, "images/d");
    assert_eq!(pixels.map(|image| image.info()), Ok(size));
    let track = formats.load::<MusicTrack>(files, "music/f");
    assert_eq!(track.map(|track| track.length_ms), Ok(50));
}
