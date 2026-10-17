//! One-bit pictures (ADR-0085): the file a one-bit screen draws in place of
//! a PNG, made ahead of time by [`convert`] so the device never decodes a
//! PNG. Pixels are packed one bit a pixel in rows, the Playdate's own
//! bitmap layout: each row [`stride`](Bitmap::stride) bytes, a byte's
//! highest bit its left pixel, a set bit white.
//!
//! A file holds one picture in one or more **styles** (the same picture
//! converted different ways, all the same size) and at most one **mask**
//! they share: a set bit where the picture is drawn, a clear one where it
//! is see-through.
//!
//! A picture is usually bigger than the PNG it was made from: it is
//! enlarged to the size it is shown at before it is dithered, so each dot
//! is one screen pixel. The file keeps the PNG's own size too (the
//! **source size**), so a game keeps naming parts of the picture in the
//! PNG's pixels and a renderer scales them to the picture's.
//!
//! The layout, all numbers little-endian:
//!
//! | Bytes | What |
//! | ----- | ---- |
//! | 0–3 | [`MAGIC`], `1BIT` |
//! | 4 | [`VERSION`] |
//! | 5 | flags: bit 0 set if there is a mask; the others clear |
//! | 6 | the number of styles, at least 1 |
//! | 7 | 0 |
//! | 8–9, 10–11 | width and height in pixels, each at least 1 |
//! | 12–13, 14–15 | the source size, each at least 1 |
//! | 16… | each style's rows, in order, then the mask's rows if any |
use crate::prelude::*;

#[cfg(feature = "std")]
pub mod convert;

/// The first four bytes of a one-bit picture file.
pub const MAGIC: [u8; 4] = *b"1BIT";

/// The layout's version: the fifth byte.
pub const VERSION: u8 = 1;

/// Bytes before the first style's rows.
pub const HEADER_BYTES: usize = 16;

/// The extension of a one-bit picture file on disk (`.1bit`).
pub const EXTENSION: &str = "1bit";

/// Flag bit: the file has a mask.
const HAS_MASK: u8 = 1;

/// A one-bit picture file, read without copying ([`Bitmap::parse`]).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Bitmap<'a> {
    width: u16,
    height: u16,
    source_width: u16,
    source_height: u16,
    styles: u8,
    /// Every style's rows, one after the other.
    planes: &'a [u8],
    mask: Option<&'a [u8]>,
}

impl<'a> Bitmap<'a> {
    /// Whether `bytes` starts as a one-bit picture file does.
    pub fn is_bitmap(bytes: &[u8]) -> bool {
        bytes.starts_with(&MAGIC)
    }

    /// Reads the file `bytes`, or says in plain words what is wrong with it.
    pub fn parse(bytes: &'a [u8]) -> Result<Self, &'static str> {
        let header = bytes
            .get(..HEADER_BYTES)
            .ok_or("too short for a one-bit picture's header")?;
        if !Self::is_bitmap(header) {
            return Err("not a one-bit picture file");
        }
        if header[4] != VERSION {
            return Err("a one-bit picture of an unknown version");
        }
        let flags = header[5];
        if flags & !HAS_MASK != 0 || header[7] != 0 {
            return Err("a one-bit picture with unknown flags");
        }
        let styles = header[6];
        let word = |at: usize| u16::from_le_bytes([header[at], header[at + 1]]);
        let (width, height) = (word(8), word(10));
        let (source_width, source_height) = (word(12), word(14));
        if styles == 0 || width == 0 || height == 0 || source_width == 0 || source_height == 0 {
            return Err("a one-bit picture with no styles or no pixels");
        }
        let plane = Self::plane_bytes(width, height);
        let has_mask = flags & HAS_MASK != 0;
        let body = &bytes[HEADER_BYTES..];
        let planes_bytes = plane * usize::from(styles);
        if body.len() != planes_bytes + if has_mask { plane } else { 0 } {
            return Err("a one-bit picture's size doesn't match its header");
        }
        let (planes, mask) = body.split_at(planes_bytes);
        Ok(Self {
            width,
            height,
            source_width,
            source_height,
            styles,
            planes,
            mask: has_mask.then_some(mask),
        })
    }

    /// Bytes per row of a picture `width` pixels wide.
    pub const fn stride(width: u16) -> usize {
        (width as usize).div_ceil(8)
    }

    /// Bytes of one style (or of the mask) of a `width` × `height` picture.
    pub const fn plane_bytes(width: u16, height: u16) -> usize {
        Self::stride(width) * height as usize
    }

    /// Width in pixels.
    pub const fn width(&self) -> u16 {
        self.width
    }

    /// Height in pixels.
    pub const fn height(&self) -> u16 {
        self.height
    }

    /// The size of the PNG it was made from, in that PNG's pixels.
    pub const fn source_size(&self) -> (u16, u16) {
        (self.source_width, self.source_height)
    }

    /// How many styles it holds.
    pub const fn styles(&self) -> usize {
        self.styles as usize
    }

    /// The rows of style `n` (0 is the first), or `None` past the last.
    pub fn style(&self, n: usize) -> Option<&'a [u8]> {
        let plane = Self::plane_bytes(self.width, self.height);
        (n < self.styles()).then(|| &self.planes[n * plane..(n + 1) * plane])
    }

    /// The mask's rows, if it has one.
    pub const fn mask(&self) -> Option<&'a [u8]> {
        self.mask
    }

    /// Pixel `(x, y)` of style `n`: `Some(true)` white, `Some(false)`
    /// black, `None` see-through, off the picture or past the last style.
    pub fn pixel(&self, n: usize, x: u16, y: u16) -> Option<bool> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let at = usize::from(y) * Self::stride(self.width) + usize::from(x / 8);
        let bit = |rows: &[u8]| rows[at] >> (7 - x % 8) & 1 == 1;
        let rows = self.style(n)?;
        match self.mask {
            Some(mask) if !bit(mask) => None,
            _ => Some(bit(rows)),
        }
    }
}

/// The size, in pixels, of one-bit picture file `bytes`, and its source
/// size, or `None` if it doesn't parse ([`Bitmap::parse`]).
pub fn bitmap_sizes(bytes: &[u8]) -> Option<((u16, u16), (u16, u16))> {
    let bitmap = Bitmap::parse(bytes).ok()?;
    Some(((bitmap.width, bitmap.height), bitmap.source_size()))
}

/// The file of a `width` × `height` picture made from a PNG of
/// `source` size: `styles` (each [`Bitmap::plane_bytes`] long) and the
/// `mask` they share, or what is wrong with them.
pub fn encode(
    (width, height): (u16, u16),
    source: (u16, u16),
    styles: &[&[u8]],
    mask: Option<&[u8]>,
) -> Result<Vec<u8>, &'static str> {
    let plane = Bitmap::plane_bytes(width, height);
    let count = u8::try_from(styles.len()).map_err(|_| "more than 255 styles")?;
    if count == 0 || width == 0 || height == 0 || source.0 == 0 || source.1 == 0 {
        return Err("a one-bit picture needs a style and pixels");
    }
    if styles
        .iter()
        .chain(mask.as_ref())
        .any(|rows| rows.len() != plane)
    {
        return Err("a style or the mask isn't the picture's size");
    }
    let mut out = Vec::with_capacity(HEADER_BYTES + plane * (styles.len() + 1));
    out.extend(MAGIC);
    out.extend([VERSION, if mask.is_some() { HAS_MASK } else { 0 }, count, 0]);
    for word in [width, height, source.0, source.1] {
        out.extend(word.to_le_bytes());
    }
    for rows in styles.iter().chain(mask.as_ref()) {
        out.extend_from_slice(rows);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    /// A 9 × 2 picture made from a 3 × 1 PNG: two styles and a mask.
    fn sample() -> Vec<u8> {
        let a: &[u8] = &[0b1000_0000, 0b1000_0000, 0, 0];
        let b: &[u8] = &[0b0111_1111, 0, 0xff, 0x80];
        let mask: &[u8] = &[0xff, 0x80, 0b0111_1111, 0];
        encode((9, 2), (3, 1), &[a, b], Some(mask)).unwrap()
    }

    #[test]
    fn the_header_is_as_documented() {
        let file = sample();
        assert_eq!(&file[..4], b"1BIT");
        assert_eq!(file[4..8], [1, 1, 2, 0]);
        assert_eq!(file[8..16], [9, 0, 2, 0, 3, 0, 1, 0]);
        assert_eq!(file.len(), HEADER_BYTES + 3 * 4);
        let plain = encode((8, 1), (8, 1), &[&[0x0f]], None).unwrap();
        assert_eq!(plain[5], 0);
        assert_eq!(plain[16..], [0x0f]);
    }

    #[test]
    fn sizes_past_255_read_both_bytes() {
        let plane = vec![0u8; Bitmap::plane_bytes(300, 1)];
        let file = encode((300, 1), (258, 513), &[&plane], None).unwrap();
        assert_eq!(file[8..16], [0x2c, 1, 1, 0, 2, 1, 1, 2]);
        let bitmap = Bitmap::parse(&file).unwrap();
        assert_eq!((bitmap.width(), bitmap.height()), (300, 1));
        assert_eq!(bitmap.source_size(), (258, 513));
    }

    #[test]
    fn a_file_reads_back_as_written() {
        let file = sample();
        let bitmap = Bitmap::parse(&file).unwrap();
        assert_eq!((bitmap.width(), bitmap.height()), (9, 2));
        assert_eq!(bitmap.source_size(), (3, 1));
        assert_eq!(bitmap.styles(), 2);
        assert_eq!(bitmap.style(0), Some(&[0x80, 0x80, 0, 0][..]));
        assert_eq!(bitmap.style(1), Some(&[0x7f, 0, 0xff, 0x80][..]));
        assert_eq!(bitmap.style(2), None);
        assert_eq!(bitmap.mask(), Some(&[0xff, 0x80, 0x7f, 0][..]));
        assert_eq!(bitmap_sizes(&file), Some(((9, 2), (3, 1))));
        assert!(Bitmap::is_bitmap(&file));
        assert!(!Bitmap::is_bitmap(b"\x89PNG"));
    }

    #[test]
    fn pixels_are_read_highest_bit_first_and_masked() {
        let file = sample();
        let bitmap = Bitmap::parse(&file).unwrap();
        assert_eq!(bitmap.pixel(0, 0, 0), Some(true));
        assert_eq!(bitmap.pixel(0, 1, 0), Some(false));
        assert_eq!(bitmap.pixel(0, 8, 0), Some(true));
        assert_eq!(bitmap.pixel(1, 0, 0), Some(false));
        assert_eq!(bitmap.pixel(1, 1, 0), Some(true));
        // The mask's first pixel of the second row is clear: see-through.
        assert_eq!(bitmap.pixel(1, 0, 1), None);
        assert_eq!(bitmap.pixel(1, 1, 1), Some(true));
        assert_eq!(bitmap.pixel(1, 8, 1), None);
        assert_eq!(bitmap.pixel(0, 9, 0), None);
        assert_eq!(bitmap.pixel(0, 0, 2), None);
        assert_eq!(bitmap.pixel(2, 0, 0), None);
    }

    #[test]
    fn bad_files_say_what_is_wrong() {
        let file = sample();
        let error = |bytes: &[u8]| Bitmap::parse(bytes).err().unwrap_or_default();
        assert_eq!(
            error(&file[..15]),
            "too short for a one-bit picture's header"
        );
        let mut png = file.clone();
        png[..4].copy_from_slice(b"\x89PNG");
        assert_eq!(error(&png), "not a one-bit picture file");
        for (at, value, message) in [
            (4, 2, "a one-bit picture of an unknown version"),
            (5, 3, "a one-bit picture with unknown flags"),
            (7, 1, "a one-bit picture with unknown flags"),
            (6, 0, "a one-bit picture with no styles or no pixels"),
            (8, 0, "a one-bit picture with no styles or no pixels"),
            (10, 0, "a one-bit picture with no styles or no pixels"),
            (12, 0, "a one-bit picture with no styles or no pixels"),
            (14, 0, "a one-bit picture with no styles or no pixels"),
            (6, 3, "a one-bit picture's size doesn't match its header"),
            (5, 0, "a one-bit picture's size doesn't match its header"),
        ] {
            let mut bad = file.clone();
            bad[at] = value;
            if at == 8 || at == 10 || at == 12 || at == 14 {
                bad[at + 1] = 0;
            }
            assert_eq!(error(&bad), message, "byte {at} = {value}");
        }
        let mut long = file.clone();
        long.push(0);
        assert_eq!(
            error(&long),
            "a one-bit picture's size doesn't match its header"
        );
        assert_eq!(bitmap_sizes(&long), None);
    }

    #[test]
    fn encode_refuses_what_it_cannot_write() {
        let row: &[u8] = &[0];
        assert!(encode((8, 1), (8, 1), &[], None).is_err());
        assert!(encode((0, 1), (8, 1), &[&[]], None).is_err());
        assert!(encode((8, 0), (8, 1), &[&[]], None).is_err());
        assert!(encode((8, 1), (0, 1), &[row], None).is_err());
        assert!(encode((8, 1), (8, 0), &[row], None).is_err());
        assert!(encode((8, 1), (8, 1), &[&[0, 0]], None).is_err());
        assert!(encode((8, 1), (8, 1), &[row], Some(&[])).is_err());
        assert!(encode((8, 1), (8, 1), &[row; 256], None).is_err());
        assert!(encode((8, 1), (8, 1), &[row; 255], None).is_ok());
    }

    proptest! {
        /// Any picture written reads back with the same pixels.
        #[test]
        fn any_picture_round_trips(
            width in 1u16..20,
            height in 1u16..6,
            source in (1u16..30, 1u16..30),
            styles in 1usize..4,
            masked: bool,
            seed: u64,
        ) {
            let plane = Bitmap::plane_bytes(width, height);
            let byte = |n: usize| seed.rotate_left(u32::try_from(n % 64).unwrap() * 7).to_le_bytes()[0] ^ n.to_le_bytes()[0];
            let planes: Vec<Vec<u8>> = (0..=styles)
                .map(|s| (0..plane).map(|i| byte(s * plane + i)).collect())
                .collect();
            let refs: Vec<&[u8]> = planes[..styles].iter().map(Vec::as_slice).collect();
            let mask = masked.then_some(planes[styles].as_slice());
            let file = encode((width, height), source, &refs, mask).unwrap();
            let bitmap = Bitmap::parse(&file).unwrap();
            prop_assert_eq!(bitmap.source_size(), source);
            for (n, rows) in refs.iter().enumerate() {
                prop_assert_eq!(bitmap.style(n), Some(*rows));
            }
            prop_assert_eq!(bitmap.mask(), mask);
        }
    }
}
