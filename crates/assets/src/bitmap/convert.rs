//! Making a one-bit picture from a colour one, ahead of time (game ADR-0085).
//! A tool runs it (a build script, a command); never the device.
//!
//! The picture is first **enlarged** to the size it is shown at: by
//! doubling pixels when the scale is a whole number, by a smooth
//! (bilinear) resample when it isn't, so rows and columns aren't doubled
//! unevenly. Then each [`Method`] makes one style of it, at the enlarged
//! size, so each dot is one screen pixel:
//!
//! 1. each pixel's brightness, optionally through a **curve**;
//! 2. the **stretch**: the darkest share of the pixels becomes black, the
//!    lightest share white, the rest spread between;
//! 3. optionally each grey **rounded** to one of a few levels, so an area
//!    is one even pattern;
//! 4. a 4 × 4 **Bayer** ordered dither;
//! 5. optionally the art's own **dark lines** drawn solid black on top.
//!
//! A pixel less than half opaque is see-through: clear in the mask, white
//! in every style.
use crate::prelude::*;

use super::{Bitmap, encode};

/// The 4 × 4 Bayer matrix: a pixel of brightness `b` (0 to 1) is white
/// where `16 b` is over its rank plus a half.
const BAYER: [[u8; 4]; 4] = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];

/// A colour picture: rows of RGBA pixels, 4 bytes each, no padding.
#[derive(Copy, Clone, Debug)]
pub struct Rgba<'a> {
    width: u32,
    height: u32,
    pixels: &'a [u8],
}

impl<'a> Rgba<'a> {
    /// The picture, or `None` if `pixels` isn't exactly `width × height`
    /// pixels or a side is 0.
    pub fn new(width: u32, height: u32, pixels: &'a [u8]) -> Option<Self> {
        let size = u64::from(width) * u64::from(height) * 4;
        (width > 0 && height > 0 && pixels.len() as u64 == size).then_some(Self {
            width,
            height,
            pixels,
        })
    }
}

/// How one style of a picture is made (the steps at the top of the
/// module).
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Method {
    /// Each brightness `b` (0 to 1) becomes `b ^ curve` first: 1 leaves it,
    /// below 1 lightens the middle greys, above 1 darkens them.
    pub curve: f32,
    /// The share of pixels stretched to black at the dark end, and to
    /// white at the light end (0.03 for 3%); 0 leaves the greys as they
    /// are.
    pub stretch: f32,
    /// Each grey rounded to one of this many evenly spaced levels before
    /// the dither (black and white among them); 0 or 1 rounds nothing.
    pub levels: u8,
    /// The art's own dark lines drawn black on top, if any.
    pub lines: Option<Lines>,
}

impl Method {
    /// The plain method: the brightness stretched by 3% at each end, then
    /// the Bayer dither (the Playdate's style A).
    pub const BAYER: Self = Self {
        curve: 1.0,
        stretch: 0.03,
        levels: 0,
        lines: None,
    };
}

/// Which pixels of the art are its dark lines: an opaque art pixel (before
/// enlarging) that is `contrast` darker than the brightest of its eight
/// neighbours, or at most `dark` bright. A see-through neighbour counts
/// as white, so a figure's dark outline is a line; past the picture's
/// edge there are no neighbours.
/// Brightnesses are 0 to 1, read before the curve.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Lines {
    /// How much darker than its brightest neighbour a line pixel is.
    pub contrast: f32,
    /// The brightness at or below which a pixel is a line anyway.
    pub dark: f32,
}

/// The one-bit picture file of `image` enlarged `num / den` times (each
/// side rounded to the nearest pixel), one style per method in order, or
/// what is wrong.
pub fn convert(
    image: &Rgba<'_>,
    (num, den): (u32, u32),
    methods: &[Method],
) -> Result<Vec<u8>, String> {
    if num == 0 || den == 0 {
        return Err(format!("the scale {num} / {den} isn't a size"));
    }
    if methods.is_empty() {
        return Err("no method to convert with".to_string());
    }
    let enlarge = |v: u32| {
        let scaled = (u64::from(v) * u64::from(num) + u64::from(den / 2)) / u64::from(den);
        u16::try_from(scaled.max(1)).ok()
    };
    let too_big = || {
        format!(
            "{}×{} px is too big for a one-bit picture",
            image.width, image.height
        )
    };
    let source = (
        u16::try_from(image.width).map_err(|_| too_big())?,
        u16::try_from(image.height).map_err(|_| too_big())?,
    );
    let size = (
        enlarge(image.width).ok_or_else(too_big)?,
        enlarge(image.height).ok_or_else(too_big)?,
    );
    let art = Art::of(image);
    let grey = art.enlarged(size, num % den == 0);
    let styles: Vec<Vec<u8>> = methods
        .iter()
        .map(|method| style(&art, &grey, method))
        .collect();
    let mask = grey
        .opaque
        .iter()
        .any(|&o| !o)
        .then(|| grey.bits(|i| grey.opaque[i]));
    let refs: Vec<&[u8]> = styles.iter().map(Vec::as_slice).collect();
    encode(size, source, &refs, mask.as_deref()).map_err(ToString::to_string)
}

/// The art at its own size: each pixel's brightness and opacity, 0 to 1.
struct Art {
    width: usize,
    height: usize,
    light: Vec<f32>,
    alpha: Vec<f32>,
}

impl Art {
    fn of(image: &Rgba<'_>) -> Self {
        let (light, alpha) = image
            .pixels
            .as_chunks::<4>()
            .0
            .iter()
            .map(|p| {
                let [r, g, b, a] = p.map(f32::from);
                ((0.299 * r + 0.587 * g + 0.114 * b) / 255.0, a / 255.0)
            })
            .unzip();
        Self {
            width: image.width as usize,
            height: image.height as usize,
            light,
            alpha,
        }
    }

    fn opaque(&self, i: usize) -> bool {
        self.alpha[i] >= 0.5
    }

    /// The art pixel nearest the centre of enlarged pixel `t` of a side
    /// `to` long, the art's side being `from`.
    fn nearest(t: usize, from: usize, to: usize) -> usize {
        (2 * t + 1) * from / (2 * to)
    }

    /// The art enlarged to `size`: by doubling pixels if `whole`, else by
    /// a bilinear resample weighted by opacity, so a see-through
    /// neighbour doesn't darken an edge.
    fn enlarged(&self, (width, height): (u16, u16), whole: bool) -> Grey {
        let (width, height) = (usize::from(width), usize::from(height));
        let mut light = Vec::with_capacity(width * height);
        let mut opaque = Vec::with_capacity(width * height);
        for y in 0..height {
            for x in 0..width {
                let (l, a) = if whole {
                    let i = Self::nearest(y, self.height, height) * self.width
                        + Self::nearest(x, self.width, width);
                    (self.light[i], self.alpha[i])
                } else {
                    self.bilinear(x, y, width, height)
                };
                light.push(l);
                opaque.push(a >= 0.5);
            }
        }
        Grey {
            width,
            height,
            light,
            opaque,
        }
    }

    /// Enlarged pixel `(x, y)` of a `width` × `height` picture, sampled
    /// between the four art pixels around its centre: its brightness
    /// and opacity. Where all four are see-through its brightness is 0
    /// (it is never drawn).
    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )] // small pictures
    fn bilinear(&self, x: usize, y: usize, width: usize, height: usize) -> (f32, f32) {
        let axis = |t: usize, to: usize, from: usize| {
            // The centre of the last pixel is half a pixel inside the art,
            // so `low` is always one of its pixels.
            let f = ((t as f32 + 0.5) * from as f32 / to as f32 - 0.5).max(0.0);
            let low = f.floor() as usize;
            (low, (low + 1).min(from - 1), f - f.floor())
        };
        let (x0, x1, wx) = axis(x, width, self.width);
        let (y0, y1, wy) = axis(y, height, self.height);
        let (mut lit, mut alpha) = (0.0, 0.0);
        for (i, w) in [
            (y0 * self.width + x0, (1.0 - wx) * (1.0 - wy)),
            (y0 * self.width + x1, wx * (1.0 - wy)),
            (y1 * self.width + x0, (1.0 - wx) * wy),
            (y1 * self.width + x1, wx * wy),
        ] {
            lit += self.light[i] * self.alpha[i] * w;
            alpha += self.alpha[i] * w;
        }
        (lit / alpha.max(f32::MIN_POSITIVE), alpha)
    }

    /// Which art pixels are dark lines (see [`Lines`]).
    fn lines(&self, lines: Lines) -> Vec<bool> {
        let at = |x: usize, y: usize| y * self.width + x;
        let mut out = vec![false; self.light.len()];
        for y in 0..self.height {
            for x in 0..self.width {
                let i = at(x, y);
                if !self.opaque(i) {
                    continue;
                }
                let mut brightest: f32 = 0.0;
                for (dx, dy) in [
                    (-1, -1),
                    (0, -1),
                    (1, -1),
                    (-1, 0),
                    (1, 0),
                    (-1, 1),
                    (0, 1),
                    (1, 1),
                ] {
                    let Some(n) = x
                        .checked_add_signed(dx)
                        .zip(y.checked_add_signed(dy))
                        .filter(|&(nx, ny)| nx < self.width && ny < self.height)
                        .map(|(nx, ny)| at(nx, ny))
                    else {
                        continue;
                    };
                    let light = if self.opaque(n) { self.light[n] } else { 1.0 };
                    brightest = brightest.max(light);
                }
                let light = self.light[i];
                out[i] = light <= lines.dark || brightest - light >= lines.contrast;
            }
        }
        out
    }
}

/// The art enlarged: brightness 0 to 1 and whether it is drawn, by pixel.
struct Grey {
    width: usize,
    height: usize,
    light: Vec<f32>,
    opaque: Vec<bool>,
}

impl Grey {
    /// The rows of a plane whose pixel `i` (row by row) is set if `set(i)`.
    fn bits(&self, set: impl Fn(usize) -> bool) -> Vec<u8> {
        #[allow(clippy::cast_possible_truncation)] // a picture's width fits a u16
        let stride = Bitmap::stride(self.width as u16);
        let mut rows = vec![0u8; stride * self.height];
        for y in 0..self.height {
            for x in 0..self.width {
                if set(y * self.width + x) {
                    rows[y * stride + x / 8] |= 0x80 >> (x % 8);
                }
            }
        }
        rows
    }
}

/// One style of `grey` (the enlarged `art`) made by `method`.
fn style(art: &Art, grey: &Grey, method: &Method) -> Vec<u8> {
    let mut light: Vec<f32> = grey.light.clone();
    for l in &mut light {
        *l = l.powf(method.curve);
    }
    stretch(&mut light, &grey.opaque, method.stretch);
    if method.levels >= 2 {
        let steps = f32::from(method.levels - 1);
        for l in &mut light {
            *l = (*l * steps).round() / steps;
        }
    }
    let lines = method.lines.map(|lines| art.lines(lines));
    grey.bits(|i| {
        let (x, y) = (i % grey.width, i / grey.width);
        if !grey.opaque[i] {
            return true;
        }
        if let Some(lines) = &lines {
            let at = Art::nearest(y, art.height, grey.height) * art.width
                + Art::nearest(x, art.width, grey.width);
            if lines[at] {
                return false;
            }
        }
        light[i] * 16.0 > f32::from(BAYER[y % 4][x % 4]) + 0.5
    })
}

/// Stretches the brightness of the opaque pixels so the darkest `share`
/// of them is black and the lightest `share` white, clamping the rest; a
/// picture with no spread is left as it is.
#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)] // small pictures
fn stretch(light: &mut [f32], opaque: &[bool], share: f32) {
    let mut sorted: Vec<f32> = light
        .iter()
        .zip(opaque)
        .filter_map(|(&l, &o)| o.then_some(l))
        .collect();
    if sorted.is_empty() || share <= 0.0 {
        return;
    }
    sorted.sort_by(f32::total_cmp);
    let last = (sorted.len() - 1) as f32;
    let rank = |p: f32| sorted[(last * p.clamp(0.0, 1.0)).round() as usize];
    let (low, high) = (rank(share), rank(1.0 - share));
    if high - low <= f32::EPSILON {
        return;
    }
    for l in light {
        *l = ((*l - low) / (high - low)).clamp(0.0, 1.0);
    }
}

/// PNG file `bytes` as a colour picture: its width, height and RGBA
/// pixels ([`Rgba::new`]), whatever kind of PNG it is (palette, grey,
/// with or without alpha, 16 bits a channel).
#[cfg(feature = "png")]
pub fn png_pixels(bytes: &[u8]) -> Result<(u32, u32, Vec<u8>), String> {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    let mut wanted = png::Transformations::EXPAND;
    wanted.insert(png::Transformations::STRIP_16);
    decoder.set_transformations(wanted);
    let mut reader = decoder.read_info().map_err(|e| e.to_string())?;
    let size = reader
        .output_buffer_size()
        .ok_or("the picture is too large to decode")?;
    let mut buf = vec![0; size];
    let info = reader.next_frame(&mut buf).map_err(|e| e.to_string())?;
    buf.truncate(info.buffer_size());
    let rgba = match info.color_type {
        png::ColorType::Rgba => buf,
        png::ColorType::Rgb => buf
            .as_chunks::<3>()
            .0
            .iter()
            .flat_map(|&[r, g, b]| [r, g, b, 255])
            .collect(),
        png::ColorType::GrayscaleAlpha => buf
            .as_chunks::<2>()
            .0
            .iter()
            .flat_map(|&[v, a]| [v, v, v, a])
            .collect(),
        png::ColorType::Grayscale => buf.iter().flat_map(|&v| [v, v, v, 255]).collect(),
        png::ColorType::Indexed => return Err("a palette the decoder didn't expand".to_string()),
    };
    Ok((info.width, info.height, rgba))
}

#[cfg(test)]
#[cfg(feature = "png")]
mod png_tests {
    use super::*;

    /// A 2 × 1 PNG of `kind` from the bytes of its two pixels.
    fn png_file(
        kind: png::ColorType,
        depth: png::BitDepth,
        pixels: &[u8],
        palette: Option<&[u8]>,
    ) -> Vec<u8> {
        let mut out = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut out, 2, 1);
            encoder.set_color(kind);
            encoder.set_depth(depth);
            if let Some(palette) = palette {
                encoder.set_palette(palette.to_vec());
            }
            let mut writer = encoder.write_header().unwrap();
            writer.write_image_data(pixels).unwrap();
        }
        out
    }

    #[test]
    fn every_kind_of_png_reads_as_rgba() {
        use png::{BitDepth::Eight, BitDepth::Sixteen, ColorType};
        let rgba = [10, 20, 30, 40, 50, 60, 70, 80];
        let cases = [
            (png_file(ColorType::Rgba, Eight, &rgba, None), rgba),
            (
                png_file(ColorType::Rgb, Eight, &[10, 20, 30, 50, 60, 70], None),
                [10, 20, 30, 255, 50, 60, 70, 255],
            ),
            (
                png_file(ColorType::GrayscaleAlpha, Eight, &[9, 99, 200, 7], None),
                [9, 9, 9, 99, 200, 200, 200, 7],
            ),
            (
                png_file(ColorType::Grayscale, Eight, &[9, 200], None),
                [9, 9, 9, 255, 200, 200, 200, 255],
            ),
            (
                png_file(ColorType::Grayscale, Sixteen, &[9, 1, 200, 2], None),
                [9, 9, 9, 255, 200, 200, 200, 255],
            ),
            (
                png_file(
                    ColorType::Indexed,
                    Eight,
                    &[1, 0],
                    Some(&[1, 2, 3, 4, 5, 6]),
                ),
                [4, 5, 6, 255, 1, 2, 3, 255],
            ),
        ];
        for (file, expected) in cases {
            assert_eq!(png_pixels(&file), Ok((2, 1, expected.to_vec())));
        }
        assert!(png_pixels(b"not a png").is_err());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// No stretch, no curve, no lines: the dither alone.
    const PLAIN: Method = Method {
        curve: 1.0,
        stretch: 0.0,
        levels: 0,
        lines: None,
    };

    /// An opaque grey pixel of brightness `v` (0 to 255).
    fn grey(v: u8) -> [u8; 4] {
        [v, v, v, 255]
    }

    const CLEAR: [u8; 4] = [0, 0, 0, 0];

    /// The file of the `width`-wide picture of `pixels` enlarged `scale`.
    fn file(width: u32, pixels: &[[u8; 4]], scale: (u32, u32), methods: &[Method]) -> Vec<u8> {
        let bytes: Vec<u8> = pixels.iter().flatten().copied().collect();
        let height = u32::try_from(pixels.len()).unwrap() / width;
        let image = Rgba::new(width, height, &bytes).unwrap();
        convert(&image, scale, methods).unwrap()
    }

    /// Style `n`'s rows as strings, `#` black, `.` white, ` ` see-through.
    fn rows(file: &[u8], n: usize) -> Vec<String> {
        let bitmap = Bitmap::parse(file).unwrap();
        (0..bitmap.height())
            .map(|y| {
                (0..bitmap.width())
                    .map(|x| match bitmap.pixel(n, x, y) {
                        Some(true) => '.',
                        Some(false) => '#',
                        None => ' ',
                    })
                    .collect()
            })
            .collect()
    }

    #[test]
    fn half_grey_is_the_bayer_checker() {
        // 128 / 255 is just over a half: 8 of every 16 pixels white.
        let f = file(2, &[grey(128); 4], (2, 1), &[PLAIN]);
        assert_eq!(rows(&f, 0), [".#.#", "#.#.", ".#.#", "#.#."]);
        let bitmap = Bitmap::parse(&f).unwrap();
        assert_eq!((bitmap.width(), bitmap.height()), (4, 4));
        assert_eq!(bitmap.source_size(), (2, 2));
        assert_eq!(bitmap.mask(), None);
    }

    #[test]
    fn each_grey_has_its_share_of_white() {
        // Brightness k / 16 lights the pixels of rank below k.
        let f = file(1, &[grey(64)], (4, 1), &[PLAIN]);
        assert_eq!(rows(&f, 0), [".#.#", "####", ".#.#", "####"]);
        let f = file(1, &[grey(0)], (4, 1), &[PLAIN]);
        assert_eq!(rows(&f, 0), ["####"; 4]);
        let f = file(1, &[grey(255)], (4, 1), &[PLAIN]);
        assert_eq!(rows(&f, 0), ["...."; 4]);
    }

    #[test]
    fn a_whole_scale_doubles_pixels() {
        let f = file(2, &[grey(0), grey(255)], (2, 1), &[PLAIN]);
        assert_eq!(rows(&f, 0), ["##..", "##.."]);
        let f = file(2, &[grey(0), grey(255)], (6, 2), &[PLAIN]);
        assert_eq!(rows(&f, 0), ["###...", "###...", "###..."]);
    }

    #[test]
    fn a_scale_between_whole_numbers_resamples_smoothly() {
        // 1.25 times: the third of five columns falls half on black, half
        // on white, so it is a half grey, not a doubled column.
        let f = file(
            4,
            &[grey(0), grey(0), grey(255), grey(255)],
            (5, 4),
            &[PLAIN],
        );
        assert_eq!(rows(&f, 0), ["##..."]);
        let f = file(
            4,
            &[grey(0), grey(0), grey(255), grey(255)].repeat(4),
            (5, 4),
            &[PLAIN],
        );
        assert_eq!(rows(&f, 0), ["##...", "###..", "##...", "###..", "##..."]);
        assert_eq!(Bitmap::parse(&f).unwrap().source_size(), (4, 4));
    }

    #[test]
    fn see_through_pixels_are_masked_and_dont_darken_their_neighbours() {
        let f = file(2, &[grey(255), CLEAR], (2, 1), &[PLAIN]);
        assert_eq!(rows(&f, 0), ["..  ", "..  "]);
        let bitmap = Bitmap::parse(&f).unwrap();
        assert_eq!(bitmap.mask(), Some(&[0b1100_0000, 0b1100_0000][..]));
        assert_eq!(bitmap.style(0), Some(&[0xf0, 0xf0][..]));
        // Resampled, the white pixel next to the see-through one stays
        // white rather than greying towards its black.
        let f = file(4, &[grey(255), grey(255), CLEAR, CLEAR], (5, 4), &[PLAIN]);
        assert_eq!(rows(&f, 0), ["...  "]);
    }

    #[test]
    fn the_stretch_makes_the_darkest_black_and_the_lightest_white() {
        let pixels = [grey(100), grey(120), grey(140), grey(160)];
        let stretched = Method {
            stretch: 0.03,
            ..PLAIN
        };
        let f = file(4, &pixels, (1, 1), &[PLAIN, stretched]);
        // Unstretched, all four are middle greys: about half on.
        assert_eq!(rows(&f, 0), [".#.#"]);
        // Stretched: 100 → black, 120 → 1/3, 140 → 2/3, 160 → white.
        assert_eq!(rows(&f, 1), ["##.."]);
        assert_eq!(Method::BAYER, stretched);
        // A flat picture has nothing to stretch.
        let f = file(1, &[grey(128)], (4, 1), &[stretched]);
        assert_eq!(rows(&f, 0), [".#.#", "#.#.", ".#.#", "#.#."]);
    }

    #[test]
    fn the_curve_lightens_and_the_levels_round() {
        let curved = Method {
            curve: 0.5,
            ..PLAIN
        };
        // 64 / 255 is about a quarter; its square root about a half.
        let f = file(1, &[grey(64)], (4, 1), &[curved]);
        assert_eq!(rows(&f, 0), [".#.#", "#.#.", ".#.#", "#.#."]);
        let rounded = Method { levels: 3, ..PLAIN };
        // 0.6 rounds to the middle of three levels: a half.
        let f = file(1, &[grey(153)], (4, 1), &[rounded]);
        assert_eq!(rows(&f, 0), [".#.#", "#.#.", ".#.#", "#.#."]);
        let f = file(1, &[grey(200)], (4, 1), &[rounded]);
        assert_eq!(rows(&f, 0), ["...."; 4]);
    }

    #[test]
    fn dark_lines_are_drawn_on_top() {
        let lined = Method {
            lines: Some(Lines {
                contrast: 0.3,
                dark: 0.15,
            }),
            ..PLAIN
        };
        // A grey dot on white is a line; the white around it isn't.
        let mut pixels = [grey(255); 9];
        pixels[4] = grey(150);
        let f = file(3, &pixels, (2, 1), &[PLAIN, lined]);
        // Dithered alone, the dot is half dots.
        assert_eq!(rows(&f, 0)[2], "...#..");
        let dot = ["......", "......", "..##..", "..##..", "......", "......"];
        assert_eq!(rows(&f, 1), dot);
        // A middle grey among middle greys isn't, nor one at the
        // picture's edge; a dark one always is.
        let mut pixels = [grey(128); 9];
        pixels[0] = grey(30);
        let f = file(3, &pixels, (1, 1), &[lined]);
        assert_eq!(rows(&f, 0), ["##.", "#.#", ".#."]);
        // Next to a see-through pixel (white ground), a middle grey is the
        // outline.
        let f = file(2, &[grey(128), CLEAR], (1, 1), &[lined]);
        assert_eq!(rows(&f, 0), ["# "]);
    }

    #[test]
    fn every_method_is_a_style_sharing_one_mask() {
        let methods = [PLAIN, Method::BAYER, Method { levels: 5, ..PLAIN }];
        let f = file(2, &[grey(10), CLEAR, grey(240), grey(90)], (1, 1), &methods);
        let bitmap = Bitmap::parse(&f).unwrap();
        assert_eq!(bitmap.styles(), 3);
        assert_eq!(bitmap.mask(), Some(&[0b1000_0000, 0b1100_0000][..]));
    }

    #[test]
    fn bad_input_is_refused() {
        let px = grey(0);
        assert!(Rgba::new(1, 1, &px).is_some());
        assert!(Rgba::new(2, 1, &px).is_none());
        assert!(Rgba::new(0, 0, &[]).is_none());
        let image = Rgba::new(1, 1, &px).unwrap();
        assert!(convert(&image, (0, 1), &[PLAIN]).is_err());
        assert!(convert(&image, (1, 0), &[PLAIN]).is_err());
        assert!(convert(&image, (1, 1), &[]).is_err());
        let wide = vec![0u8; 70_000 * 4];
        let image = Rgba::new(70_000, 1, &wide).unwrap();
        assert!(convert(&image, (1, 1), &[PLAIN]).is_err());
        let image = Rgba::new(1, 1, &px).unwrap();
        assert!(convert(&image, (70_000, 1), &[PLAIN]).is_err());
        // A side never rounds to nothing.
        let f = convert(&image, (1, 4), &[PLAIN]).unwrap();
        assert_eq!(Bitmap::parse(&f).unwrap().width(), 1);
    }

    /// Art of opaque greys, each a brightness 0 to 1 already.
    fn art(width: usize, light: &[f32]) -> Art {
        Art {
            width,
            height: light.len() / width,
            light: light.to_vec(),
            alpha: vec![1.0; light.len()],
        }
    }

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-5
    }

    #[test]
    fn the_resample_weighs_the_four_pixels_around_each_centre() {
        // 2 × 2 art enlarged to 5 × 5: column 1's centre falls a tenth of
        // the way from art column 0 to 1, row 3's nine tenths.
        let a = art(2, &[0.0, 1.0, 0.5, 0.25]);
        let (light, alpha) = a.bilinear(1, 3, 5, 5);
        let expected = 0.9 * 0.1 * 0.0 + 0.1 * 0.1 * 1.0 + 0.9 * 0.9 * 0.5 + 0.1 * 0.9 * 0.25;
        assert!(close(light, expected), "{light}");
        assert!(close(alpha, 1.0));
        let (light, _) = a.bilinear(3, 1, 5, 5);
        let expected = 0.1 * 0.9 * 0.0 + 0.9 * 0.9 * 1.0 + 0.1 * 0.1 * 0.5 + 0.9 * 0.1 * 0.25;
        assert!(close(light, expected), "{light}");
        // The edges hold the art's edge pixels.
        assert!(close(a.bilinear(0, 0, 5, 5).0, 0.0));
        assert!(close(a.bilinear(4, 4, 5, 5).0, 0.25));
        // A half see-through neighbour: half the opacity, the opaque
        // pixel's brightness.
        let mut half = art(2, &[0.6, 0.0]);
        half.alpha = vec![1.0, 0.0];
        let (light, alpha) = half.bilinear(1, 0, 4, 1);
        assert!(close(alpha, 0.75), "{alpha}");
        assert!(close(light, 0.6), "{light}");
        let (light, alpha) = half.bilinear(2, 0, 4, 1);
        assert!(close(alpha, 0.25), "{alpha}");
        assert!(close(light, 0.6), "{light}");
    }

    #[test]
    fn a_pixel_less_than_half_opaque_is_see_through() {
        let f = file(
            2,
            &[[255, 255, 255, 100], [255, 255, 255, 160]],
            (1, 1),
            &[PLAIN],
        );
        assert_eq!(rows(&f, 0), [" ."]);
    }

    #[test]
    fn each_of_the_eight_neighbours_can_make_a_line() {
        let lined = Method {
            lines: Some(Lines {
                contrast: 0.3,
                dark: 0.15,
            }),
            ..PLAIN
        };
        // A middle grey centre (white undithered at its place) is a line
        // when any one neighbour is white.
        for n in [0, 1, 2, 3, 5, 6, 7, 8] {
            let mut pixels = [grey(128); 9];
            pixels[n] = grey(255);
            let f = file(3, &pixels, (1, 1), &[PLAIN, lined]);
            assert_eq!(rows(&f, 0)[1].chars().nth(1), Some('.'), "neighbour {n}");
            assert_eq!(rows(&f, 1)[1].chars().nth(1), Some('#'), "neighbour {n}");
        }
    }

    #[test]
    fn lines_land_on_the_nearest_art_pixel_when_resampled() {
        let lined = Method {
            lines: Some(Lines {
                contrast: 0.3,
                dark: 0.15,
            }),
            ..PLAIN
        };
        // Four white columns but the third, a grey line, enlarged to five:
        // the line covers the screen columns whose centres are in art
        // column 2, the third and fourth of five.
        let pixels = [grey(255), grey(255), grey(150), grey(255)];
        let f = file(4, &pixels, (5, 4), &[lined]);
        assert_eq!(rows(&f, 0), ["..##."]);
        assert_eq!(Art::nearest(2, 4, 5), 2);
        assert_eq!(Art::nearest(4, 4, 5), 3);
        assert_eq!(Art::nearest(5, 3, 6), 2);
    }

    #[test]
    fn rows_wider_than_a_byte_are_packed_in_order() {
        // 5 × 2 art doubled: rows of 10 pixels, 2 bytes each.
        let (b, w) = (grey(0), grey(255));
        let f = file(5, &[w, b, w, b, b, b, b, b, b, w], (2, 1), &[PLAIN]);
        assert_eq!(
            rows(&f, 0),
            ["..##..####", "..##..####", "########..", "########.."]
        );
        let bitmap = Bitmap::parse(&f).unwrap();
        assert_eq!(
            bitmap.style(0),
            Some(
                &[
                    0b1100_1100,
                    0b0000_0000,
                    0b1100_1100,
                    0,
                    0,
                    0b1100_0000,
                    0,
                    0b1100_0000
                ][..]
            )
        );
    }

    #[test]
    fn the_stretch_takes_the_share_from_each_end() {
        let mut light: Vec<f32> = (0..100u8).map(|i| f32::from(i) / 100.0).collect();
        let opaque = vec![true; 100];
        stretch(&mut light, &opaque, 0.03);
        // The 4th darkest (0.03) is black, the 4th lightest (0.96) white.
        assert!(close(light[3], 0.0) && close(light[2], 0.0));
        assert!(close(light[96], 1.0) && close(light[99], 1.0));
        assert!(close(light[95], 0.92 / 0.93), "{}", light[95]);
        assert!(close(light[50], 0.47 / 0.93), "{}", light[50]);
        // See-through pixels don't count.
        let mut light = vec![0.0, 0.4, 0.6, 1.0];
        stretch(&mut light, &[false, true, true, false], 0.03);
        assert!(close(light[1], 0.0) && close(light[2], 1.0));
    }

    #[test]
    fn a_grey_exactly_on_a_rank_is_black_there() {
        // 33 levels: 135 / 255 rounds to 17 / 32, and 16 × 17 / 32 is 8.5,
        // the second pixel's rank (8) plus a half.
        let rounded = Method {
            levels: 33,
            ..PLAIN
        };
        let f = file(1, &[grey(135)], (4, 1), &[rounded]);
        assert_eq!(rows(&f, 0), [".#.#", "#.#.", ".#.#", "#.#."]);
    }
}
