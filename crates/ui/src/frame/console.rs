//! The console's size, which the game picks ([`ConsoleSpec`]), and how it
//! fits the window (game ADR-0012).

#[cfg(not(feature = "std"))]
use crate::float::Float;

/// A console's size: `cols × rows` cells of `cell_w × cell_h` logical
/// pixels. The frame ([`GlyphBuffer`](super::GlyphBuffer)) keeps the spec
/// it was made with; the game picks one when it starts (game ADR-0070 §6).
#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub struct ConsoleSpec {
    /// Width in cells.
    pub cols: u16,
    /// Height in cells.
    pub rows: u16,
    /// Cell width in logical pixels.
    pub cell_w: u16,
    /// Cell height in logical pixels.
    pub cell_h: u16,
}

impl ConsoleSpec {
    /// `cols × rows` cells of 8 × 16 pixels, the cell of a classic text
    /// console (and of [`GlyphBuffer::new`](super::GlyphBuffer::new)).
    pub const fn text(cols: u16, rows: u16) -> Self {
        Self {
            cols,
            rows,
            cell_w: 8,
            cell_h: 16,
        }
    }

    /// The same cells, `cols × rows` of them.
    #[must_use]
    pub const fn sized(self, cols: u16, rows: u16) -> Self {
        Self { cols, rows, ..self }
    }

    /// A cell's width and height in logical pixels.
    pub fn cell_px(self) -> (i32, i32) {
        (i32::from(self.cell_w), i32::from(self.cell_h))
    }

    /// The whole console's width and height in logical pixels.
    pub fn px(self) -> (i32, i32) {
        (
            i32::from(self.cols) * i32::from(self.cell_w),
            i32::from(self.rows) * i32::from(self.cell_h),
        )
    }
}

/// Where the console goes in a window: integer `scale` and the top-left
/// pixel offset that centres it. Everything around it is letterbox.
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Layout {
    /// Pixels per logical pixel; never 0.
    pub scale: u32,
    /// Left edge of the console in window pixels (negative if it overflows).
    pub offset_x: f32,
    /// Top edge of the console in window pixels (negative if it overflows).
    pub offset_y: f32,
}

/// Fits a `spec` console into a `screen_w × screen_h` window: the largest
/// integer scale that fits (at least 1, so a too-small window crops the
/// edges evenly), centred on whole pixels so glyphs stay crisp.
pub fn layout(spec: ConsoleSpec, screen_w: f32, screen_h: f32) -> Layout {
    let console_w = f32::from(spec.cols) * f32::from(spec.cell_w);
    let console_h = f32::from(spec.rows) * f32::from(spec.cell_h);
    let finite = |v: f32| if v.is_finite() { v } else { 0.0 };
    let (screen_w, screen_h) = (finite(screen_w), finite(screen_h));
    let fit = (screen_w / console_w).min(screen_h / console_h).floor();
    // `fit` is in 0..=f32::MAX here; `as` saturates anyway.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let scale = (fit.max(1.0) as u32).max(1);
    #[allow(clippy::cast_precision_loss)] // scale ≤ 2^24 for any real window
    let s = scale as f32;
    Layout {
        scale,
        offset_x: ((screen_w - console_w * s) / 2.0).floor(),
        offset_y: ((screen_h - console_h * s) / 2.0).floor(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A console of 100 × 32 text cells: 800 × 512 pixels.
    const WIDE: ConsoleSpec = ConsoleSpec::text(100, 32);

    fn l(scale: u32, offset_x: f32, offset_y: f32) -> Layout {
        Layout {
            scale,
            offset_x,
            offset_y,
        }
    }

    #[test]
    fn exact_fit() {
        assert_eq!(layout(WIDE, 800.0, 512.0), l(1, 0.0, 0.0));
        assert_eq!(layout(WIDE, 1600.0, 1024.0), l(2, 0.0, 0.0));
        assert_eq!(layout(WIDE, 2400.0, 1536.0), l(3, 0.0, 0.0));
    }

    #[test]
    fn oversize_window_letterboxes() {
        // 1080p: 2x (1600x1024), bars of 160 px left/right and 28 top/bottom.
        assert_eq!(layout(WIDE, 1920.0, 1080.0), l(2, 160.0, 28.0));
        // Height limits: 3000 wide would allow 3x, 1100 tall only 2x.
        assert_eq!(layout(WIDE, 3000.0, 1100.0), l(2, 700.0, 38.0));
        // Width limits.
        assert_eq!(layout(WIDE, 1700.0, 5000.0), l(2, 50.0, 1988.0));
        // Odd leftover pixels round down to a whole pixel.
        assert_eq!(layout(WIDE, 801.0, 513.0), l(1, 0.0, 0.0));
        assert_eq!(layout(WIDE, 803.0, 515.0), l(1, 1.0, 1.0));
    }

    #[test]
    fn undersize_window_keeps_scale_1_and_centres() {
        assert_eq!(layout(WIDE, 600.0, 412.0), l(1, -100.0, -50.0));
        assert_eq!(layout(WIDE, 799.0, 1024.0), l(1, -1.0, 256.0));
        assert_eq!(layout(WIDE, 0.0, 0.0), l(1, -400.0, -256.0));
        assert_eq!(layout(WIDE, -5.0, -5.0).scale, 1);
    }

    #[test]
    fn a_console_of_any_size_fits_its_own_size() {
        let small = ConsoleSpec::text(50, 15);
        assert_eq!(small.px(), (400, 240));
        assert_eq!(WIDE.px(), (800, 512));
        assert_eq!(small.cell_px(), (8, 16));
        assert_eq!(
            small.sized(3, 2),
            ConsoleSpec {
                cols: 3,
                rows: 2,
                ..small
            }
        );
        assert_eq!(layout(small, 400.0, 240.0), l(1, 0.0, 0.0));
        // A 1080p window takes it at 4x: 1600 × 960, bars of 160 and 60.
        assert_eq!(layout(small, 1920.0, 1080.0), l(4, 160.0, 60.0));
        // Cells of another size: 40 × 20 cells of 8 × 8 are 320 × 160.
        let square = ConsoleSpec {
            cell_h: 8,
            ..ConsoleSpec::text(40, 20)
        };
        assert_eq!(square.px(), (320, 160));
        assert_eq!(square.cell_px(), (8, 8));
        assert_eq!(layout(square, 1920.0, 1080.0), l(6, 0.0, 60.0));
    }

    #[test]
    fn non_finite_sizes_act_as_zero() {
        assert_eq!(layout(WIDE, f32::NAN, f32::INFINITY), l(1, -400.0, -256.0));
        assert_eq!(layout(WIDE, f32::NEG_INFINITY, 512.0), l(1, -400.0, 0.0));
    }
}
