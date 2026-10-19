//! The tile camera: which map tiles the viewport shows. How many tiles
//! that is (the view's size) comes from the grid skin (game ADR-0038).

use shuyi_core::Pos;

/// The map tile shown in the viewport's top-left corner. It can be negative
/// (or run past the map) when the map is smaller than the viewport: the map
/// is then centred and the cells around it are left blank.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Camera {
    /// Top-left tile of the viewport.
    pub origin: Pos,
}

impl Camera {
    /// Default [`follow`](Self::follow) margin, in tiles.
    pub const MARGIN: i32 = 3;

    /// A camera with `target` as near the centre of a viewport of `view`
    /// tiles (across, down) as the map allows, on a `map_w × map_h` map.
    pub fn centred_on(target: Pos, map_w: u16, map_h: u16, view: (i32, i32)) -> Self {
        let origin = Pos::new(
            clamp_axis(target.x - view.0 / 2, map_w, view.0),
            clamp_axis(target.y - view.1 / 2, map_h, view.1),
        );
        Self { origin }
    }

    /// Scrolls as little as possible so `target` is at least `margin` tiles
    /// from each edge of a viewport of `view` tiles (across, down), without
    /// showing past the map's edges. On an axis where the map is smaller
    /// than the viewport, the map is centred. A margin over half the
    /// viewport is treated as half.
    pub fn follow(&mut self, target: Pos, map_w: u16, map_h: u16, view: (i32, i32), margin: i32) {
        self.origin = Pos::new(
            follow_axis(self.origin.x, target.x, map_w, view.0, margin),
            follow_axis(self.origin.y, target.y, map_h, view.1, margin),
        );
    }
}

/// One axis of [`Camera::follow`].
fn follow_axis(origin: i32, target: i32, map_len: u16, view: i32, margin: i32) -> i32 {
    let margin = margin.clamp(0, (view - 1) / 2);
    let lowest = target - (view - 1 - margin);
    let highest = target - margin;
    clamp_axis(origin.clamp(lowest, highest), map_len, view)
}

/// Keeps `origin` within the map (`0..=map_len - view`), or centres a map
/// shorter than the viewport.
fn clamp_axis(origin: i32, map_len: u16, view: i32) -> i32 {
    let map_len = i32::from(map_len);
    if map_len <= view {
        -((view - map_len) / 2)
    } else {
        origin.clamp(0, map_len - view)
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    /// A view of 35 × 30 tiles (the glyph skin's on a 70 × 30-cell map
    /// area).
    const VIEW: (i32, i32) = (35, 30);

    /// Whether `tile` is in a `view`-tile viewport of camera `c`.
    fn visible(tile: Pos, c: Camera, view: (i32, i32)) -> bool {
        (c.origin.x..c.origin.x + view.0).contains(&tile.x)
            && (c.origin.y..c.origin.y + view.1).contains(&tile.y)
    }

    fn cam(x: i32, y: i32) -> Camera {
        Camera {
            origin: Pos::new(x, y),
        }
    }

    fn followed(from: Camera, target: Pos, w: u16, h: u16) -> Pos {
        let mut c = from;
        c.follow(target, w, h, VIEW, Camera::MARGIN);
        c.origin
    }

    #[test]
    fn small_map_is_centred() {
        // 14 × 8: (35 − 14) / 2 = 10 blank tiles left, (30 − 8) / 2 = 11 above.
        for target in [Pos::new(0, 0), Pos::new(13, 7), Pos::new(6, 3)] {
            assert_eq!(followed(cam(0, 0), target, 14, 8), Pos::new(-10, -11));
            assert_eq!(followed(cam(5, 9), target, 14, 8), Pos::new(-10, -11));
        }
        assert_eq!(
            Camera::centred_on(Pos::new(0, 0), 14, 8, VIEW),
            cam(-10, -11)
        );
        // Exactly viewport-sized: origin 0.
        assert_eq!(
            followed(cam(3, 3), Pos::new(34, 29), 35, 30),
            Pos::new(0, 0)
        );
        // One axis small, the other large.
        assert_eq!(
            followed(cam(0, 0), Pos::new(0, 50), 34, 64),
            Pos::new(0, 24)
        );
    }

    #[test]
    fn large_map_clamps_at_every_edge() {
        let (w, h) = (64, 40);
        // Top-left.
        assert_eq!(followed(cam(10, 5), Pos::new(0, 0), w, h), Pos::new(0, 0));
        // Bottom-right: origin = size − view.
        assert_eq!(
            followed(cam(0, 0), Pos::new(63, 39), w, h),
            Pos::new(29, 10)
        );
        // Top-right and bottom-left.
        assert_eq!(followed(cam(0, 5), Pos::new(63, 0), w, h), Pos::new(29, 0));
        assert_eq!(followed(cam(20, 0), Pos::new(0, 39), w, h), Pos::new(0, 10));
        // An origin already out of range is pulled back in.
        assert_eq!(
            followed(cam(-5, 99), Pos::new(10, 20), w, h),
            Pos::new(0, 10)
        );
        let centred = |x, y| Camera::centred_on(Pos::new(x, y), w, h, VIEW);
        assert_eq!(centred(63, 39), cam(29, 10));
        assert_eq!(centred(0, 0), cam(0, 0));
        assert_eq!(centred(32, 20), cam(15, 5));
    }

    #[test]
    fn margin_is_respected() {
        let (w, h) = (64, 64);
        let start = cam(10, 10);
        // Inside the margins: no scrolling.
        assert_eq!(followed(start, Pos::new(13, 13), w, h), Pos::new(10, 10));
        assert_eq!(followed(start, Pos::new(41, 36), w, h), Pos::new(10, 10));
        // One tile into a margin scrolls by one.
        assert_eq!(followed(start, Pos::new(12, 12), w, h), Pos::new(9, 9));
        assert_eq!(followed(start, Pos::new(42, 37), w, h), Pos::new(11, 11));
        // Far away: jumps so the target sits exactly at the margin.
        assert_eq!(followed(start, Pos::new(60, 3), w, h), Pos::new(29, 0));
        // Custom and oversized margins.
        let mut c = start;
        c.follow(Pos::new(10, 10), w, h, VIEW, 0);
        assert_eq!(c.origin, Pos::new(10, 10));
        c.follow(Pos::new(30, 30), w, h, VIEW, 99);
        // Clamped to half the view (17, 14): the least scroll that centres it.
        assert_eq!(c.origin, Pos::new(13, 15));
        c.follow(Pos::new(30, 30), w, h, VIEW, -4);
        assert_eq!(c.origin, Pos::new(13, 15));
    }

    #[test]
    fn the_view_size_is_the_callers() {
        // A 17 × 15 view (a skin with bigger tiles) on a 64 × 40 map.
        let view = (17, 15);
        let centred = |x, y, w, h| Camera::centred_on(Pos::new(x, y), w, h, view);
        assert_eq!(centred(32, 20, 64, 40), cam(24, 13));
        assert_eq!(centred(63, 39, 64, 40), cam(47, 25));
        assert_eq!(centred(0, 0, 64, 40), cam(0, 0));
        // A 14 × 8 map is smaller than it: centred, 1 blank tile left and 3
        // above.
        assert_eq!(centred(3, 5, 14, 8), cam(-1, -3));
        // Across and down are separate.
        assert_eq!(
            Camera::centred_on(Pos::new(32, 20), 64, 40, (10, 4)),
            cam(27, 18)
        );
        let mut c = cam(0, 0);
        c.follow(Pos::new(20, 20), 64, 40, view, 3);
        assert_eq!(c.origin, Pos::new(7, 9));
        c.follow(Pos::new(8, 10), 64, 40, view, 3);
        assert_eq!(c.origin, Pos::new(5, 7));
        assert!(visible(Pos::new(8, 10), c, view));
        assert!(!visible(Pos::new(22, 10), c, view));
        assert!(!visible(Pos::new(8, 22), c, view));
        assert!(!visible(Pos::new(4, 10), c, view));
        assert!(!visible(Pos::new(8, 6), c, view));
    }

    proptest! {
        #[test]
        fn follow_keeps_target_visible_and_map_filling_the_view(
            ox in -100..100i32, oy in -100..100i32,
            tx in 0..64i32, ty in 0..64i32,
            w in 1u16..65, h in 1u16..65, margin in 0..10i32,
            view in (1..40i32, 1..40i32),
        ) {
            let target = Pos::new(tx.min(i32::from(w) - 1), ty.min(i32::from(h) - 1));
            let mut c = cam(ox, oy);
            c.follow(target, w, h, view, margin);
            prop_assert!(visible(target, c, view));
            for (o, len, view) in [(c.origin.x, w, view.0), (c.origin.y, h, view.1)] {
                let len = i32::from(len);
                if len > view {
                    prop_assert!(o >= 0 && o + view <= len);
                } else {
                    prop_assert_eq!(o, -((view - len) / 2));
                }
            }
        }
    }
}
