//! The field of a corner picture and its wobble, over doubled coordinates
//! `X = 2x + 1`, `Y = 2y + 1` (a pixel's centre), so a border is `X = 0` or
//! `X = 2w`. In integers: the field `F` is in units of `1 / 4wh`, and the
//! wobble `W` in the units of `2F`, so a pixel is in where `2F + W > 4wh`.

use crate::prelude::*;

use shuyi_core::rng::SimRng;

use super::{CornersError, MAX_CELLS, MAX_STRENGTH, Wobble};

/// The field of mix `m` at `(X, Y)` on a `w × h` picture, times `4wh`: `tl
/// (2w − X)(2h − Y) + tr X (2h − Y) + bl (2w − X) Y + br X Y`.
pub(super) fn field(m: u8, (w, h): (i64, i64), (across, down): (i64, i64)) -> i64 {
    let bit = |b: u8| i64::from(m & b != 0);
    let (left, top) = (2 * w - across, 2 * h - down);
    bit(8) * left * top + bit(4) * across * top + bit(2) * left * down + bit(1) * across * down
}

/// A wobble's lattice: `cells × cells` values in `−128..=128`, rolled in
/// row order, repeating every picture.
pub(super) struct Lattice {
    cells: i64,
    strength: i64,
    values: Vec<i64>,
}

impl Lattice {
    /// The lattice of `wobble`, if its numbers are in range.
    pub(super) fn new(wobble: Wobble) -> Result<Self, CornersError> {
        if wobble.cells == 0 || wobble.cells > MAX_CELLS {
            return Err(CornersError::Cells(wobble.cells));
        }
        if wobble.strength > MAX_STRENGTH {
            return Err(CornersError::Strength(wobble.strength));
        }
        let mut rng = SimRng::new(wobble.seed);
        let points = usize::from(wobble.cells) * usize::from(wobble.cells);
        let values = (0..points)
            .map(|_| i64::from(rng.roll_below(257)) - 128)
            .collect();
        Ok(Self {
            cells: i64::from(wobble.cells),
            strength: i64::from(wobble.strength),
            values,
        })
    }

    /// The value at lattice point `(column, row)`, wrapping.
    fn value(&self, column: i64, row: i64) -> i64 {
        let at = (row % self.cells) * self.cells + column % self.cells;
        usize::try_from(at)
            .ok()
            .and_then(|at| self.values.get(at))
            .copied()
            .unwrap_or(0)
    }

    /// `W` at `(X, Y)` on a `w × h` picture: `N × strength × 4wh / (128 ×
    /// 100)`, with `N` the bilinear blend of the four lattice points round
    /// it, rounded toward zero once, at the end.
    pub(super) fn wobble(&self, (w, h): (i64, i64), (across, down): (i64, i64)) -> i64 {
        let (w2, h2) = (2 * w, 2 * h);
        // Lattice point `i` is at `X = 2w × i / cells`: the point left of
        // `X`, and how far past it `X` is, in units of `1 / 2w` of a cell.
        let (column, right) = (across * self.cells / w2, across * self.cells % w2);
        let (row, below) = (down * self.cells / h2, down * self.cells % h2);
        let (left, above) = (w2 - right, h2 - below);
        // `N × 4wh`.
        let blend = self.value(column, row) * left * above
            + self.value(column + 1, row) * right * above
            + self.value(column, row + 1) * left * below
            + self.value(column + 1, row + 1) * right * below;
        blend * self.strength / (128 * 100)
    }
}
