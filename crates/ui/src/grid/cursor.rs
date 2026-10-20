//! The virtual cursor on a grid map: which tile it is on, how it moves
//! (one tile per step, never off the map) and how it pulses: between full
//! and about half brightness, never off. A grid skin draws it
//! ([`CursorView`](super::CursorView)).

use core::f32::consts::TAU;

use shuyi_core::{Dir, Pos};

#[cfg(not(feature = "std"))]
use crate::float::Float;
use crate::input::frame_seconds;

/// One full bright → dim → bright pulse, in seconds. *Tunable.*
pub const BLINK_PERIOD: f32 = 1.0;

/// The dimmest the cursor gets, as a fraction of full brightness. *Tunable.*
pub const BLINK_MIN: f32 = 0.5;

/// The cursor: a map tile and the time into its pulse.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cursor {
    /// The tile under the cursor; always on the map.
    pub pos: Pos,
    /// Seconds into the current pulse, in `0..BLINK_PERIOD`.
    pub blink_t: f32,
}

impl Cursor {
    /// A cursor on `pos`, at full brightness.
    pub const fn new(pos: Pos) -> Self {
        Self { pos, blink_t: 0.0 }
    }

    /// Advances the pulse by `dt` seconds (negative or non-finite `dt`
    /// counts as zero).
    pub fn tick(&mut self, dt: f32) {
        self.blink_t = (self.blink_t + frame_seconds(dt)) % BLINK_PERIOD;
    }

    /// Current brightness: `1` at the start of a pulse, down to
    /// [`BLINK_MIN`] halfway through, then back up (a cosine).
    pub fn brightness(&self) -> f32 {
        pulse(self.blink_t, BLINK_PERIOD)
    }

    /// Moves one tile `dir`, staying on a `map_w × map_h` map. Returns
    /// whether the cursor moved; a move restarts the pulse at full
    /// brightness, so the cursor is easy to follow.
    pub fn step(&mut self, dir: Dir, map_w: u16, map_h: u16) -> bool {
        let (dx, dy) = dir.delta();
        let clamp = |v: i32, len: u16| v.clamp(0, (i32::from(len) - 1).max(0));
        let to = Pos::new(clamp(self.pos.x + dx, map_w), clamp(self.pos.y + dy, map_h));
        if to == self.pos {
            return false;
        }
        self.pos = to;
        self.blink_t = 0.0;
        true
    }

    /// Jumps to `pos` (e.g. the next unit), restarting the pulse.
    pub fn jump(&mut self, pos: Pos) {
        *self = Self::new(pos);
    }
}

/// Brightness `t` seconds into a `period`-long pulse: `1` at the start,
/// [`BLINK_MIN`] halfway, back to `1` at the end (a cosine).
fn pulse(t: f32, period: f32) -> f32 {
    let wave = (TAU * t / period).cos();
    BLINK_MIN + (1.0 - BLINK_MIN) * (1.0 + wave) / 2.0
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    const MOVES: [Dir; 4] = [Dir::Left, Dir::Right, Dir::Up, Dir::Down];

    #[test]
    fn steps_one_tile_and_clamps_at_the_edges() {
        let mut c = Cursor::new(Pos::new(1, 1));
        c.blink_t = 0.3;
        assert!(c.step(Dir::Left, 3, 3));
        assert_eq!((c.pos, c.blink_t), (Pos::new(0, 1), 0.0));
        c.blink_t = 0.3;
        assert!(!c.step(Dir::Left, 3, 3));
        assert_eq!((c.pos, c.blink_t), (Pos::new(0, 1), 0.3));
        assert!(c.step(Dir::Up, 3, 3));
        assert!(!c.step(Dir::Up, 3, 3));
        assert_eq!(c.pos, Pos::new(0, 0));
        assert!(c.step(Dir::Right, 3, 3));
        assert!(c.step(Dir::Right, 3, 3));
        assert!(!c.step(Dir::Right, 3, 3));
        assert!(c.step(Dir::Down, 3, 3));
        assert!(c.step(Dir::Down, 3, 3));
        assert!(!c.step(Dir::Down, 3, 3));
        assert_eq!(c.pos, Pos::new(2, 2));
        // A map of no tiles keeps it on its corner.
        let mut corner = Cursor::new(Pos::new(0, 0));
        assert!(!corner.step(Dir::Right, 0, 0));
        assert_eq!(corner.pos, Pos::new(0, 0));
    }

    #[test]
    fn jump_moves_and_restarts_the_pulse() {
        let mut c = Cursor::new(Pos::new(1, 1));
        c.tick(0.4);
        c.jump(Pos::new(5, 6));
        assert_eq!(c, Cursor::new(Pos::new(5, 6)));
    }

    #[test]
    fn pulses_between_full_and_half_brightness() {
        let mut c = Cursor::new(Pos::new(0, 0));
        assert!((c.brightness() - 1.0).abs() < 1e-6);
        c.tick(0.25);
        assert!((c.blink_t - 0.25).abs() < 1e-6);
        assert!((c.brightness() - 0.75).abs() < 1e-6);
        c.tick(0.25);
        assert!((c.brightness() - 0.5).abs() < 1e-6);
        c.tick(0.5);
        assert!(c.blink_t < 1e-6, "wraps after a period: {}", c.blink_t);
        c.tick(0.25);
        c.tick(-1.0);
        c.tick(f32::NAN);
        c.tick(f32::INFINITY);
        assert!((c.blink_t - 0.25).abs() < 1e-6, "{}", c.blink_t);
    }

    #[test]
    fn pulse_scales_with_its_period() {
        assert!((pulse(0.0, 2.0) - 1.0).abs() < 1e-6);
        assert!((pulse(0.5, 2.0) - 0.75).abs() < 1e-6);
        assert!((pulse(1.0, 2.0) - BLINK_MIN).abs() < 1e-6);
        assert!((pulse(2.0, 2.0) - 1.0).abs() < 1e-6);
    }

    proptest! {
        #[test]
        fn never_leaves_the_map(
            w in 1u16..20,
            h in 1u16..20,
            steps in prop::collection::vec(0usize..4, 0..60),
        ) {
            let mut c = Cursor::new(Pos::new(0, 0));
            for s in steps {
                c.step(MOVES[s], w, h);
                prop_assert!((0..i32::from(w)).contains(&c.pos.x));
                prop_assert!((0..i32::from(h)).contains(&c.pos.y));
            }
        }

        #[test]
        fn brightness_stays_between_min_and_full(t in -10.0f32..10.0) {
            let mut c = Cursor::new(Pos::new(0, 0));
            c.tick(t);
            let b = c.brightness();
            prop_assert!((BLINK_MIN - 1e-6..=1.0 + 1e-6).contains(&b), "{b}");
        }
    }
}
