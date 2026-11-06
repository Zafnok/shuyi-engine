//! Shapes in whole console pixels, by one integer rule every renderer
//! follows (ADR-0085 §8): each gives its pixels as runs, and a renderer
//! fills each run as a rectangle one console pixel high. So `shuyi-app`,
//! the raster and the one-bit renderers draw the same pixels by
//! construction.
//!
//! The rules, in console pixels, integers only. A point names a pixel:
//! `(x, y)` is that pixel's centre.
//!
//! - **A line one pixel wide** ([`line_runs`]) is Bresenham's, from
//!   `from` to `to`, both ends drawn: the all-octant form whose error term
//!   starts at `dx + dy` (`dx = |x1 − x0|`, `dy = −|y1 − y0|`). Along its
//!   major axis (`x` when `|dx| ≥ |dy|`) it takes one pixel a step; pixel
//!   `k` from `from` is `round(k · minor / major)` along the other, a half
//!   rounded towards `to`. That closed form gives the pixels of the walk,
//!   so a line far longer than the clip costs no more than the clip: only
//!   the steps inside it are taken.
//! - **A circle's outline of radius `r`** ([`circle_outline_runs`]) is
//!   `2r + 1` pixels across: the midpoint circle whose decision value
//!   starts at `1 − r`, one octant walked (at most `r` steps), each step
//!   giving its eight mirrored pixels.
//!
//! A run is `(y, x0, x1)`: row `y`, pixels `x0..x1`, half-open, always
//! inside the clip. Runs may overlap (a circle's mirrored pixels where the
//! octants meet); every renderer paints them opaque, so that doesn't
//! matter.

use crate::frame::glyph_buffer::PxRect;

/// `clip`'s rows as `y0..y1` and columns as `x0..x1`, in `i64`.
fn spans(clip: PxRect) -> ((i64, i64), (i64, i64)) {
    let (x, y) = (i64::from(clip.x), i64::from(clip.y));
    (
        (x, x + i64::from(clip.w.max(0))),
        (y, y + i64::from(clip.h.max(0))),
    )
}

/// Calls `f(y, x0, x1)` with the part of run `x0..x1` on row `y` inside
/// `clip`, if any. Everything inside `clip` fits in `i32`.
pub(crate) fn clip_run(clip: PxRect, y: i64, x0: i64, x1: i64, f: &mut impl FnMut(i32, i32, i32)) {
    let ((left, right), (top, bottom)) = spans(clip);
    let (x0, x1) = (x0.max(left), x1.min(right));
    if x0 >= x1 || y < top || y >= bottom {
        return;
    }
    if let (Ok(y), Ok(x0), Ok(x1)) = (i32::try_from(y), i32::try_from(x0), i32::try_from(x1)) {
        f(y, x0, x1);
    }
}

/// Calls `f(y, x0, x1)` for the runs of the line one pixel wide from
/// `from` to `to`, both ends drawn, inside `clip` (the module's rule).
/// Each row of the line is one run.
pub fn line_runs(from: (i32, i32), to: (i32, i32), clip: PxRect, mut f: impl FnMut(i32, i32, i32)) {
    let (x0, y0) = (i64::from(from.0), i64::from(from.1));
    let (x1, y1) = (i64::from(to.0), i64::from(to.1));
    let (dx, dy) = ((x1 - x0).abs(), (y1 - y0).abs());
    let (sx, sy) = (if x1 < x0 { -1 } else { 1 }, if y1 < y0 { -1 } else { 1 });
    let x_major = dx >= dy;
    let ((major0, major_step, major_len), (minor0, minor_step, minor_len)) = if x_major {
        ((x0, sx, dx), (y0, sy, dy))
    } else {
        ((y0, sy, dy), (x0, sx, dx))
    };
    let (columns, rows) = spans(clip);
    let (major_span, minor_span) = if x_major {
        (columns, rows)
    } else {
        (rows, columns)
    };
    // The steps whose major coordinate is inside the clip.
    let (lo, hi) = if major_step > 0 {
        (major_span.0 - major0, major_span.1 - major0)
    } else {
        (major0 - major_span.1 + 1, major0 - major_span.0 + 1)
    };
    let (first, end) = (lo.max(0), hi.min(major_len + 1));
    // The run being gathered on one row (x-major lines only): its row and
    // its first and last pixel.
    let mut run: Option<(i64, i64, i64)> = None;
    for k in first..end {
        let major = major0 + major_step * k;
        let minor = minor0 + minor_step * minor_offset(k, major_len, minor_len);
        if minor < minor_span.0 || minor >= minor_span.1 {
            continue;
        }
        if !x_major {
            clip_run(clip, major, minor, minor + 1, &mut f);
            continue;
        }
        let (column, line) = (major, minor);
        run = match run {
            Some((row, first, last))
                if row == line && (column == first - 1 || column == last + 1) =>
            {
                Some((row, first.min(column), last.max(column)))
            }
            Some((row, first, last)) => {
                clip_run(clip, row, first, last + 1, &mut f);
                Some((line, column, column))
            }
            None => Some((line, column, column)),
        };
    }
    if let Some((row, first, last)) = run {
        clip_run(clip, row, first, last + 1, &mut f);
    }
}

/// How far pixel `k` of a line is along its minor axis: `k · minor /
/// major` rounded, a half up (`⌊(2k · minor + major) / (2 · major)⌋`),
/// which is where Bresenham's walk puts it. `major` is at least `minor`.
fn minor_offset(k: i64, major: i64, minor: i64) -> i64 {
    if major == 0 {
        return 0;
    }
    let (k, major, minor) = (i128::from(k), i128::from(major), i128::from(minor));
    let offset = (2 * k * minor + major).div_euclid(2 * major);
    // At most `minor`, which came from an `i64`.
    i64::try_from(offset).unwrap_or(0)
}

/// Calls `f(y, x0, x1)` for the runs of the outline of the circle of
/// `radius` round `center` inside `clip` (the module's rule): one pixel a
/// run.
pub fn circle_outline_runs(
    center: (i32, i32),
    radius: u16,
    clip: PxRect,
    mut f: impl FnMut(i32, i32, i32),
) {
    let (cx, cy) = (i64::from(center.0), i64::from(center.1));
    let (mut x, mut y) = (i64::from(radius), 0_i64);
    let mut d = 1 - x;
    let mut dot = |px: i64, py: i64| clip_run(clip, py, px, px + 1, &mut f);
    while x >= y {
        for (px, py) in [
            (x, y),
            (y, x),
            (-y, x),
            (-x, y),
            (-x, -y),
            (-y, -x),
            (y, -x),
            (x, -y),
        ] {
            dot(cx + px, cy + py);
        }
        y += 1;
        if d < 0 {
            d += 2 * y + 1;
        } else {
            x -= 1;
            d += 2 * (y - x) + 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use alloc::collections::BTreeSet;

    use proptest::prelude::*;

    use super::*;
    use crate::frame::glyph_buffer::Rect;
    use crate::prelude::*;

    /// Every pixel the runs cover.
    fn pixels(runs: impl FnOnce(&mut dyn FnMut(i32, i32, i32))) -> BTreeSet<(i32, i32)> {
        let mut out = BTreeSet::new();
        runs(&mut |y, x0, x1| {
            assert!(x0 < x1, "empty run {y} {x0}..{x1}");
            out.extend((x0..x1).map(|x| (x, y)));
        });
        out
    }

    fn line(from: (i32, i32), to: (i32, i32), clip: PxRect) -> BTreeSet<(i32, i32)> {
        pixels(|f| line_runs(from, to, clip, f))
    }

    /// Bresenham's walk itself, the error term from `dx + dy`.
    fn walked(from: (i32, i32), to: (i32, i32)) -> BTreeSet<(i32, i32)> {
        let (mut x, mut y) = (i64::from(from.0), i64::from(from.1));
        let (x1, y1) = (i64::from(to.0), i64::from(to.1));
        let (dx, dy) = ((x1 - x).abs(), -(y1 - y).abs());
        let (sx, sy) = (if x < x1 { 1 } else { -1 }, if y < y1 { 1 } else { -1 });
        let mut err = dx + dy;
        let mut out = BTreeSet::new();
        loop {
            out.insert((i32::try_from(x).unwrap(), i32::try_from(y).unwrap()));
            if x == x1 && y == y1 {
                return out;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                err += dx;
                y += sy;
            }
        }
    }

    const WIDE: PxRect = Rect::new(-100, -100, 200, 200);

    #[test]
    fn a_line_in_each_octant_is_bresenhams() {
        // From the centre to a point in each of the eight octants, and
        // along the axes and diagonals between them.
        let ends = [
            (7, 2),
            (2, 7),
            (-2, 7),
            (-7, 2),
            (-7, -2),
            (-2, -7),
            (2, -7),
            (7, -2),
            (5, 0),
            (0, 5),
            (-5, 0),
            (0, -5),
            (4, 4),
            (-4, 4),
            (-4, -4),
            (4, -4),
            (0, 0),
        ];
        for to in ends {
            assert_eq!(line((0, 0), to, WIDE), walked((0, 0), to), "to {to:?}");
            assert_eq!(line(to, (0, 0), WIDE), walked(to, (0, 0)), "from {to:?}");
        }
        // The shallow line rises where a half rounds up.
        let shallow: Vec<_> = line((0, 0), (4, 1), WIDE).into_iter().collect();
        assert_eq!(shallow, [(0, 0), (1, 0), (2, 1), (3, 1), (4, 1)]);
    }

    #[test]
    fn a_line_is_one_run_a_row_inside_the_clip() {
        let mut runs = Vec::new();
        line_runs((0, 0), (9, 2), WIDE, |y, x0, x1| runs.push((y, x0, x1)));
        assert_eq!(runs, [(0, 0, 3), (1, 3, 7), (2, 7, 10)]);
        // Backwards, the same rows (each one run).
        let mut back = Vec::new();
        line_runs((9, 2), (0, 0), WIDE, |y, x0, x1| back.push((y, x0, x1)));
        assert_eq!(back.len(), 3);
        // A clip cuts the runs.
        let mut cut = Vec::new();
        line_runs((0, 0), (9, 2), Rect::new(2, 1, 3, 5), |y, x0, x1| {
            cut.push((y, x0, x1));
        });
        assert_eq!(cut, [(1, 3, 5)]);
        // An empty clip draws nothing.
        assert!(line((0, 0), (9, 2), Rect::new(0, 0, 0, 5)).is_empty());
    }

    #[test]
    fn a_line_far_longer_than_its_clip_takes_only_the_clips_steps() {
        let clip = Rect::new(0, 0, 16, 16);
        let far = line((i32::MIN, i32::MIN), (i32::MAX, i32::MAX), clip);
        let near: BTreeSet<_> = (0..16).map(|i| (i, i)).collect();
        assert_eq!(far, near);
        let flat = line((i32::MIN, 3), (i32::MAX, 3), clip);
        assert_eq!(flat, (0..16).map(|x| (x, 3)).collect());
    }

    #[test]
    fn the_midpoint_circle_starts_from_one_less_the_radius() {
        let circle = |r| pixels(|f| circle_outline_runs((0, 0), r, WIDE, f));
        assert_eq!(circle(0), BTreeSet::from([(0, 0)]));
        // Radius 1: the four neighbours and no corners (d = 0 steps in).
        assert_eq!(
            circle(1),
            BTreeSet::from([(1, 0), (0, 1), (-1, 0), (0, -1)])
        );
        // Radius 3, the textbook octant: (3,0) (3,1) (2,2).
        let three = circle(3);
        for p in [(3, 0), (3, 1), (2, 2), (1, 3), (0, 3), (-3, -1), (-2, -2)] {
            assert!(three.contains(&p), "{p:?}");
        }
        assert!(!three.contains(&(3, 2)));
        assert_eq!(three.len(), 16);
        // It is 2r + 1 pixels across, and symmetric.
        let five = circle(5);
        let xs: BTreeSet<_> = five.iter().map(|p| p.0).collect();
        assert_eq!(xs.len(), 11);
        assert!(five.iter().all(|&(x, y)| five.contains(&(y, x))));
        // Clipped to a quarter.
        let quarter = pixels(|f| circle_outline_runs((0, 0), 5, Rect::new(0, 0, 10, 10), f));
        assert!(quarter.iter().all(|&(x, y)| x >= 0 && y >= 0));
        assert!(quarter.is_subset(&five));
    }

    proptest! {
        #[test]
        fn the_closed_form_is_the_walk(
            x0 in -40..40, y0 in -40..40, x1 in -40..40, y1 in -40..40,
            cx in -50..50, cy in -50..50, cw in 0..100, ch in 0..100,
        ) {
            let clip = Rect::new(cx, cy, cw, ch);
            let want: BTreeSet<_> = walked((x0, y0), (x1, y1))
                .into_iter()
                .filter(|&(x, y)| clip.contains(x, y))
                .collect();
            prop_assert_eq!(line((x0, y0), (x1, y1), clip), want);
        }

        #[test]
        fn a_circle_never_leaves_its_clip(
            r in 0_u16..200, cx in -300..300, cy in -300..300,
            x in -50..50, y in -50..50, w in 0..100, h in 0..100,
        ) {
            let clip = Rect::new(x, y, w, h);
            let got = pixels(|f| circle_outline_runs((cx, cy), r, clip, f));
            prop_assert!(got.iter().all(|&(px, py)| clip.contains(px, py)));
        }
    }
}
