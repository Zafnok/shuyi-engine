//! The test card: a public test image for sprite items, generated, not
//! art, so the same code always makes the same pixels. `frame-png`'s tests
//! paint it, and a game may write it into its test files.
//!
//! 16×16: a one-pixel white border around four coloured quadrants (red and
//! green above blue and yellow), so a flip, a crop and a scale each show.

/// Width and height of the card in pixels.
pub const SIZE: u32 = 16;
/// The border colour (RGBA).
pub const BORDER: [u8; 4] = [255, 255, 255, 255];
/// The quadrants' colours (RGBA): top-left, top-right, bottom-left,
/// bottom-right.
pub const QUADRANTS: [[u8; 4]; 4] = [
    [204, 51, 51, 255],
    [51, 170, 68, 255],
    [51, 102, 204, 255],
    [230, 190, 50, 255],
];

/// The colour of pixel `(x, y)`.
fn pixel(x: u32, y: u32) -> [u8; 4] {
    let edge = |v: u32| v == 0 || v == SIZE - 1;
    if edge(x) || edge(y) {
        return BORDER;
    }
    let half = SIZE / 2;
    QUADRANTS[usize::from(y >= half) * 2 + usize::from(x >= half)]
}

/// The card's RGBA8 pixels, row-major.
pub fn pixels() -> Vec<u8> {
    (0..SIZE)
        .flat_map(|y| (0..SIZE).flat_map(move |x| pixel(x, y)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn border_and_quadrants() {
        let [red, green, blue, yellow] = QUADRANTS;
        for i in 0..SIZE {
            for edge in [(i, 0), (i, 15), (0, i), (15, i)] {
                assert_eq!(pixel(edge.0, edge.1), BORDER, "{edge:?}");
            }
        }
        // Each quadrant's corners: just inside the border and at the middle.
        for (colour, xs, ys) in [
            (red, [1, 7], [1, 7]),
            (green, [8, 14], [1, 7]),
            (blue, [1, 7], [8, 14]),
            (yellow, [8, 14], [8, 14]),
        ] {
            for x in xs {
                for y in ys {
                    assert_eq!(pixel(x, y), colour, "({x}, {y})");
                }
            }
        }
        let mut distinct = QUADRANTS.to_vec();
        distinct.push(BORDER);
        distinct.sort_unstable();
        distinct.dedup();
        assert_eq!(distinct.len(), 5);
    }

    #[test]
    fn pixels_are_row_major() {
        let rgba = pixels();
        assert_eq!(rgba.len(), 16 * 16 * 4);
        let at = |x: usize, y: usize| &rgba[(y * 16 + x) * 4..][..4];
        assert_eq!(at(0, 0), BORDER);
        assert_eq!(at(14, 1), QUADRANTS[1]);
        assert_eq!(at(1, 14), QUADRANTS[2]);
    }
}
