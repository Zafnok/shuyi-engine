//! The sheet `frame-png --every --count` writes: the frames of a
//! [`Capture`](super::Capture) in one picture, left to right then top to
//! bottom, each under a strip with its time since the script ended
//! (`+50ms`). The strips are drawn in the tool's own 3 × 5 pixel font at
//! twice its size: the labels are the tool's, not a game's text.

use super::{Image, Shot};

/// Pixels between two cells, and around them.
const GUTTER: u32 = 2;

/// The strip above each cell that holds its time.
const LABEL_HEIGHT: u32 = 14;

/// The widest and tallest sheet, in pixels.
const MAX_SIDE: u64 = 16_384;

/// The sheet's background, under the gutters and the strips.
const BACKGROUND: [u8; 4] = [0x20, 0x20, 0x20, 0xff];

/// The labels' colour.
const INK: [u8; 4] = [0xe0, 0xe0, 0xe0, 0xff];

/// Pixels per font pixel.
const FONT_SCALE: u32 = 2;

/// A glyph's width, in font pixels; one more parts two glyphs.
const GLYPH_WIDTH: u32 = 3;

/// The label's distance from the top of its strip.
const LABEL_TOP: u32 = 2;

/// The frames of `shots`, `every` frames apart, as one picture, `columns`
/// cells in a row (as square as it can be, if `None`).
///
/// # Errors
/// No shot, `columns` of 0 or more than the shots, or a sheet wider or
/// taller than 16 384 pixels: that error names its size and suggests a
/// smaller `--scale` or other `--columns`.
pub fn build(shots: &[Shot], every: u32, columns: Option<u32>) -> Result<Image, String> {
    let count = u32::try_from(shots.len()).map_err(|_| "too many frames for a sheet")?;
    let cell_width = shots.iter().map(|s| s.image.width).max().unwrap_or(0);
    let cell_height = shots.iter().map(|s| s.image.height).max().unwrap_or(0);
    let layout = Layout::new(count, columns, cell_width, cell_height)?;
    let (width, height) = (layout.width(), layout.height());
    if width > MAX_SIDE || height > MAX_SIDE {
        return Err(format!(
            "the sheet would be {width}×{height} pixels, more than {MAX_SIDE} on a side: \
             take a smaller --scale or other --columns"
        ));
    }
    // Both at most `MAX_SIDE`, checked above.
    let (width, height) = (
        u32::try_from(width).unwrap_or(0),
        u32::try_from(height).unwrap_or(0),
    );
    let mut sheet = Image::filled(width, height, BACKGROUND);
    for (i, shot) in (0..).zip(shots) {
        let (x, y) = layout.cell(i);
        let millis = u64::from(i) * u64::from(every) * 1000 / 60;
        draw_text(
            &mut sheet,
            &format!("+{millis}ms"),
            (x, y + LABEL_TOP),
            x + cell_width,
        );
        copy(&mut sheet, &shot.image, (x, y + LABEL_HEIGHT));
    }
    Ok(sheet)
}

/// Where a sheet's cells go: `columns` by `rows` cells of a frame
/// `cell_width` × `cell_height` under its strip.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Layout {
    columns: u32,
    rows: u32,
    cell_width: u32,
    cell_height: u32,
}

impl Layout {
    /// The layout of `count` cells, `columns` in a row (the smallest `n`
    /// with `n × n ≥ count`, if `None`).
    fn new(
        count: u32,
        columns: Option<u32>,
        cell_width: u32,
        cell_height: u32,
    ) -> Result<Self, String> {
        if count == 0 {
            return Err("a sheet needs a frame".to_owned());
        }
        let columns = columns.unwrap_or_else(|| default_columns(count));
        if !(1..=count).contains(&columns) {
            return Err(format!("columns: 1 to {count}, not {columns}"));
        }
        Ok(Self {
            columns,
            rows: count.div_ceil(columns),
            cell_width,
            cell_height,
        })
    }

    /// The sheet's width in pixels.
    fn width(&self) -> u64 {
        u64::from(GUTTER)
            + u64::from(self.columns) * (u64::from(self.cell_width) + u64::from(GUTTER))
    }

    /// The sheet's height in pixels.
    fn height(&self) -> u64 {
        u64::from(GUTTER) + u64::from(self.rows) * u64::from(self.row_height())
    }

    /// A row's height: the strip, the frame and the gutter under it.
    fn row_height(&self) -> u32 {
        LABEL_HEIGHT + self.cell_height + GUTTER
    }

    /// The top-left corner of cell `i`'s strip; its frame is
    /// [`LABEL_HEIGHT`] below. Only for a sheet within [`MAX_SIDE`].
    fn cell(&self, i: u32) -> (u32, u32) {
        let (column, row) = (i % self.columns, i / self.columns);
        (
            GUTTER + column * (self.cell_width + GUTTER),
            GUTTER + row * self.row_height(),
        )
    }
}

/// The smallest `n` with `n × n ≥ count`.
fn default_columns(count: u32) -> u32 {
    let root = count.isqrt();
    if root * root < count { root + 1 } else { root }
}

/// Copies `image` into `sheet` with its top-left corner at `(x, y)`.
fn copy(sheet: &mut Image, image: &Image, (x, y): (u32, u32)) {
    let row = image.width as usize * 4;
    for (dy, line) in (0..).zip(image.rgba.chunks_exact(row)) {
        let at = index(sheet, x, y + dy);
        sheet.rgba[at..at + row].copy_from_slice(line);
    }
}

/// Where pixel `(x, y)` of `image` starts in its bytes.
fn index(image: &Image, x: u32, y: u32) -> usize {
    (y as usize * image.width as usize + x as usize) * 4
}

/// Draws `text` in the label font with its top-left corner at `(x, y)`,
/// cut at column `right`. A character the font lacks is left out.
fn draw_text(image: &mut Image, text: &str, (x, y): (u32, u32), right: u32) {
    let advance = (GLYPH_WIDTH + 1) * FONT_SCALE;
    for (n, rows) in (0..).zip(text.chars().filter_map(glyph)) {
        let left = x + n * advance;
        for (dy, bits) in (0..).zip(rows) {
            for dx in (0..GLYPH_WIDTH).filter(|dx| bits & (0b100 >> dx) != 0) {
                for (px, py) in square(left + dx * FONT_SCALE, y + dy * FONT_SCALE) {
                    if px < right && px < image.width && py < image.height {
                        let at = index(image, px, py);
                        image.rgba[at..at + 4].copy_from_slice(&INK);
                    }
                }
            }
        }
    }
}

/// The pixels of one font pixel whose top-left is `(x, y)`.
fn square(x: u32, y: u32) -> impl Iterator<Item = (u32, u32)> {
    (0..FONT_SCALE).flat_map(move |dy| (0..FONT_SCALE).map(move |dx| (x + dx, y + dy)))
}

/// The label font's glyph of `c`: five rows of three pixels, the left one
/// the highest bit. Only the digits, `+`, `m` and `s`.
fn glyph(c: char) -> Option<[u8; 5]> {
    Some(match c {
        '0' => [0b111, 0b101, 0b101, 0b101, 0b111],
        '1' => [0b010, 0b110, 0b010, 0b010, 0b111],
        '2' => [0b111, 0b001, 0b111, 0b100, 0b111],
        '3' => [0b111, 0b001, 0b111, 0b001, 0b111],
        '4' => [0b101, 0b101, 0b111, 0b001, 0b001],
        '5' => [0b111, 0b100, 0b111, 0b001, 0b111],
        '6' => [0b111, 0b100, 0b111, 0b101, 0b111],
        '7' => [0b111, 0b001, 0b001, 0b001, 0b001],
        '8' => [0b111, 0b101, 0b111, 0b101, 0b111],
        '9' => [0b111, 0b101, 0b111, 0b001, 0b111],
        '+' => [0b000, 0b010, 0b111, 0b010, 0b000],
        'm' => [0b000, 0b110, 0b111, 0b101, 0b101],
        's' => [0b000, 0b011, 0b010, 0b001, 0b110],
        _ => return None,
    })
}

#[cfg(test)]
mod tests;
