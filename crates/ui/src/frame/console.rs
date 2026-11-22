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

impl Layout {
    /// The console pixel of a `spec` console this layout places that holds
    /// the window pixel `window` (`x` right, `y` down, in the window pixels
    /// the layout was made in), rounded down; `None` in the letterbox, or
    /// for a place that isn't a number. It inverts the placement: each
    /// console pixel's `scale × scale` window pixels give it back
    /// (ADR-0134 §2).
    pub fn frame_px(self, spec: ConsoleSpec, window: (f32, f32)) -> Option<(i32, i32)> {
        #[allow(clippy::cast_precision_loss)] // scale ≤ 2^24 for any real window
        let scale = self.scale.max(1) as f32;
        let (width, height) = spec.px();
        let axis = |at: f32, offset: f32, side: i32| -> Option<i32> {
            let px = ((at - offset) / scale).floor();
            #[allow(clippy::cast_precision_loss)] // a console's side is far below 2^24
            let inside = px >= 0.0 && px < side as f32;
            // `px` is a whole number in `0..side` here.
            #[allow(clippy::cast_possible_truncation)]
            inside.then_some(px as i32)
        };
        Some((
            axis(window.0, self.offset_x, width)?,
            axis(window.1, self.offset_y, height)?,
        ))
    }
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

    /// The window a `spec` console fills at `scale`, with `margin` pixels of
    /// letterbox on every side, and its layout. A margin under half the
    /// console's width keeps the next scale from fitting.
    fn placed(spec: ConsoleSpec, scale: u32, margin: (i32, i32)) -> ((f32, f32), Layout) {
        let (w, h) = spec.px();
        let s = i32::try_from(scale).unwrap();
        #[allow(clippy::cast_precision_loss)] // small test sizes
        let window = ((w * s + 2 * margin.0) as f32, (h * s + 2 * margin.1) as f32);
        let fit = layout(spec, window.0, window.1);
        assert_eq!(fit.scale, scale, "{spec:?} in {window:?}");
        (window, fit)
    }

    #[test]
    fn every_window_pixel_of_a_console_pixel_gives_it_back_at_scales_1_to_4() {
        let spec = ConsoleSpec {
            cell_w: 3,
            cell_h: 2,
            ..ConsoleSpec::text(4, 3)
        };
        let (w, h) = spec.px();
        for scale in 1..=4 {
            let (_, fit) = placed(spec, scale, (5, 9));
            let s = i32::try_from(scale).unwrap();
            for (x, y) in (0..w).flat_map(|x| (0..h).map(move |y| (x, y))) {
                for (dx, dy) in (0..s).flat_map(|dx| (0..s).map(move |dy| (dx, dy))) {
                    #[allow(clippy::cast_precision_loss)] // small test sizes
                    let at = (
                        fit.offset_x + (x * s + dx) as f32 + 0.5,
                        fit.offset_y + (y * s + dy) as f32 + 0.5,
                    );
                    assert_eq!(fit.frame_px(spec, at), Some((x, y)), "{at:?} at {scale}x");
                }
            }
        }
    }

    #[test]
    fn the_letterbox_and_a_place_that_is_no_number_are_outside_the_frame() {
        // 1080p over 800 × 512 at 2x: bars of 160 left and right, 28 above
        // and below.
        let fit = layout(WIDE, 1920.0, 1080.0);
        let at = |x, y| fit.frame_px(WIDE, (x, y));
        assert_eq!(at(160.0, 28.0), Some((0, 0)));
        assert_eq!(at(1759.9, 1051.9), Some((799, 511)));
        assert_eq!(at(159.9, 500.0), None);
        assert_eq!(at(1760.0, 500.0), None);
        assert_eq!(at(500.0, 27.9), None);
        assert_eq!(at(500.0, 1052.0), None);
        assert_eq!(at(-1.0, -1.0), None);
        assert_eq!(at(f32::NAN, 500.0), None);
        assert_eq!(at(500.0, f32::INFINITY), None);
        // A window too small crops the console: what shows maps.
        let cropped = layout(WIDE, 600.0, 412.0);
        assert_eq!(cropped.frame_px(WIDE, (0.0, 0.0)), Some((100, 50)));
    }

    proptest::proptest! {
        #[test]
        fn frame_px_inverts_the_placement(
            cols in 1u16..60,
            rows in 1u16..40,
            cell in (1u16..12, 1u16..20),
            scale in 1u32..=4,
            margin in (0.0f64..1.0, 0.0f64..1.0),
            pixel in (0.0f64..1.0, 0.0f64..1.0),
            within in (0.0f32..1.0, 0.0f32..1.0),
        ) {
            let spec = ConsoleSpec { cols, rows, cell_w: cell.0, cell_h: cell.1 };
            let (w, h) = spec.px();
            // Under half the width (so the next scale can't fit), any height.
            #[allow(clippy::cast_possible_truncation)] // in 0..side
            let margin = (
                (margin.0 * f64::from(w - 1) / 2.0) as i32,
                (margin.1 * 300.0) as i32,
            );
            let (window, fit) = placed(spec, scale, margin);
            // A console pixel, and a place in its square of window pixels.
            #[allow(clippy::cast_possible_truncation)] // in 0..side
            let (x, y) = (
                (pixel.0 * f64::from(w)) as i32,
                (pixel.1 * f64::from(h)) as i32,
            );
            #[allow(clippy::cast_precision_loss)] // small test sizes
            let s = scale as f32;
            #[allow(clippy::cast_precision_loss)] // small test sizes
            let at = (
                fit.offset_x + (x as f32 + within.0.min(0.999)) * s,
                fit.offset_y + (y as f32 + within.1.min(0.999)) * s,
            );
            proptest::prop_assert_eq!(fit.frame_px(spec, at), Some((x, y)));
            // The letterbox's edges give none.
            proptest::prop_assert_eq!(fit.frame_px(spec, (fit.offset_x - 0.01, at.1)), None);
            proptest::prop_assert_eq!(fit.frame_px(spec, (at.0, fit.offset_y - 0.01)), None);
            #[allow(clippy::cast_precision_loss)] // small test sizes
            let (right, bottom) = (fit.offset_x + w as f32 * s, fit.offset_y + h as f32 * s);
            proptest::prop_assert!(right <= window.0 && bottom <= window.1);
            proptest::prop_assert_eq!(fit.frame_px(spec, (right, at.1)), None);
            proptest::prop_assert_eq!(fit.frame_px(spec, (at.0, bottom)), None);
        }
    }

    #[test]
    fn non_finite_sizes_act_as_zero() {
        assert_eq!(layout(WIDE, f32::NAN, f32::INFINITY), l(1, -400.0, -256.0));
        assert_eq!(layout(WIDE, f32::NEG_INFINITY, 512.0), l(1, -400.0, 0.0));
    }
}
