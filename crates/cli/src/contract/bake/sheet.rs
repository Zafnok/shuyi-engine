//! The contact sheet of a one-bit bake (`bake --sheet <file>`): every
//! picture the bake dithered, a row each, its source on the left and its
//! one-bit picture from the pack on the right, so an artist sees at a
//! glance which ones the dither spoiled and are worth drawing by hand in
//! the target's layer. Each pixel is [`SCALE`] × [`SCALE`], so the dots
//! show, on a grey ground that see-through pixels show too.

use shuyi_assets::bitmap::Bitmap;
use shuyi_assets::bitmap::convert::png_pixels;
use shuyi_assets::bundle::Bundle;

use crate::codec::encode_png;

/// How many sheet pixels a picture's pixel is, each way.
pub const SCALE: u32 = 2;

/// The ground between and around the pictures, in sheet pixels.
pub const GAP: u32 = 8;

/// The ground's colour: a middle grey, neither the one-bit black nor its
/// white.
pub const GROUND: [u8; 4] = [128, 128, 128, 255];

/// One row of the sheet: the source's pixels and the pack's one-bit
/// picture.
struct Row<'a> {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
    bitmap: Bitmap<'a>,
}

impl Row<'_> {
    /// The row's height in sheet pixels, the taller picture's.
    fn height(&self) -> u32 {
        self.height.max(u32::from(self.bitmap.height())) * SCALE
    }

    /// Its width in sheet pixels, both pictures and the gap between.
    fn width(&self) -> u32 {
        (self.width + u32::from(self.bitmap.width())) * SCALE + GAP
    }
}

/// The contact sheet, a PNG file, of the pictures at `paths`: each read
/// from `source` (a PNG) and from `pack` (its one-bit picture). An error
/// names a picture that is no PNG in the sources or no one-bit picture
/// in the pack.
pub fn draw(source: &Bundle, pack: &Bundle, paths: &[String]) -> Result<Vec<u8>, String> {
    let mut rows = Vec::with_capacity(paths.len());
    for path in paths {
        let colour = source.bytes(path).ok_or_else(|| format!("no {path}"))?;
        let (width, height, rgba) =
            png_pixels(colour).map_err(|e| format!("{}: {e}", source.display_path(path)))?;
        let one_bit = pack.bytes(path).unwrap_or_default();
        let bitmap = Bitmap::parse(one_bit).map_err(|e| format!("{path} in the pack: {e}"))?;
        rows.push(Row {
            width,
            height,
            rgba,
            bitmap,
        });
    }
    let width = rows.iter().map(Row::width).max().unwrap_or(0) + 2 * GAP;
    let height = rows.iter().map(|row| row.height() + GAP).sum::<u32>() + GAP;
    let mut sheet = Sheet {
        width,
        pixels: GROUND.repeat(usize::try_from(width * height).unwrap_or(0)),
    };
    let mut top = GAP;
    for row in &rows {
        for y in 0..row.height {
            for x in 0..row.width {
                let at = usize::try_from((y * row.width + x) * 4).unwrap_or(0);
                let mut pixel = [0; 4];
                pixel.copy_from_slice(&row.rgba[at..at + 4]);
                sheet.dot(GAP + x * SCALE, top + y * SCALE, over_ground(pixel));
            }
        }
        let left = GAP + row.width * SCALE + GAP;
        for y in 0..row.bitmap.height() {
            for x in 0..row.bitmap.width() {
                let dot = match row.bitmap.pixel(0, x, y) {
                    Some(true) => [255, 255, 255, 255],
                    Some(false) => [0, 0, 0, 255],
                    None => GROUND,
                };
                let (x, y) = (u32::from(x), u32::from(y));
                sheet.dot(left + x * SCALE, top + y * SCALE, dot);
            }
        }
        top += row.height() + GAP;
    }
    encode_png(width, height, &sheet.pixels)
}

/// The sheet being drawn: RGBA pixels, row by row.
struct Sheet {
    width: u32,
    pixels: Vec<u8>,
}

impl Sheet {
    /// Paints the [`SCALE`] × [`SCALE`] dot whose top-left is `(x, y)`.
    fn dot(&mut self, x: u32, y: u32, rgba: [u8; 4]) {
        for dy in 0..SCALE {
            for dx in 0..SCALE {
                let at = usize::try_from(((y + dy) * self.width + x + dx) * 4).unwrap_or(0);
                if let Some(pixel) = self.pixels.get_mut(at..at + 4) {
                    pixel.copy_from_slice(&rgba);
                }
            }
        }
    }
}

/// `pixel` laid over the [`GROUND`] by its alpha, opaque.
fn over_ground(pixel: [u8; 4]) -> [u8; 4] {
    let alpha = u32::from(pixel[3]);
    let mix = |channel: usize| {
        let (colour, ground) = (u32::from(pixel[channel]), u32::from(GROUND[channel]));
        u8::try_from((colour * alpha + ground * (255 - alpha) + 127) / 255).unwrap_or(u8::MAX)
    };
    [mix(0), mix(1), mix(2), 255]
}

#[cfg(test)]
mod tests {
    use shuyi_assets::bitmap::encode;
    use shuyi_assets::bundle::Layer;

    use super::*;

    /// A bundle of the file `bytes` at `a.png`.
    fn bundle(name: &'static str, bytes: Vec<u8>) -> Bundle {
        let file: &'static [u8] = bytes.leak();
        let files: &'static [(&str, &[u8])] = vec![("a.png", file)].leak();
        Bundle::new(vec![Layer::table(name, files)])
    }

    #[test]
    fn each_picture_is_drawn_beside_its_source_twice_its_size() {
        // A 2 × 1 source: red, then half see-through white.
        let png = encode_png(2, 1, &[255, 0, 0, 255, 255, 255, 255, 128]).unwrap();
        // Its one-bit picture: black, then see-through.
        let one_bit = encode((2, 1), (2, 1), &[&[0]], Some(&[0x80])).unwrap();
        let source = bundle("assets", png);
        let pack = bundle("pack", one_bit);
        let sheet = draw(&source, &pack, &["a.png".to_owned()]).unwrap();
        let (width, height, rgba) = png_pixels(&sheet).unwrap();
        // Ground, source (4), ground, picture (4), ground.
        assert_eq!((width, height), (GAP + 4 + GAP + 4 + GAP, GAP + 2 + GAP));
        let at = |x: u32, y: u32| {
            let i = usize::try_from((y * width + x) * 4).unwrap();
            [rgba[i], rgba[i + 1], rgba[i + 2], rgba[i + 3]]
        };
        assert_eq!(at(0, 0), GROUND);
        assert_eq!(at(GAP, GAP), [255, 0, 0, 255]);
        assert_eq!(at(GAP + 1, GAP + 1), [255, 0, 0, 255]);
        assert_eq!(at(GAP + 2, GAP), [192, 192, 192, 255]);
        let left = GAP + 4 + GAP;
        assert_eq!(at(left, GAP), [0, 0, 0, 255]);
        assert_eq!(at(left + 3, GAP + 1), GROUND);
        // A picture missing from the pack is named.
        let png = bundle("assets", b"not a png".to_vec());
        let error = draw(&png, &pack, &["a.png".to_owned()]).unwrap_err();
        assert!(error.starts_with("assets/a.png: "), "{error}");
        let error = draw(&source, &source, &["a.png".to_owned()]).unwrap_err();
        assert!(error.starts_with("a.png in the pack: "), "{error}");
        // No picture: the ground alone.
        let empty = png_pixels(&draw(&source, &pack, &[]).unwrap()).unwrap();
        assert_eq!((empty.0, empty.1), (2 * GAP, GAP));
    }
}
