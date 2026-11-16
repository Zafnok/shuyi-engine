//! The virtual cursor on a grid map: which tile it is on, how it moves
//! (one tile per step, never off the map) and how it pulses: between full
//! and about half brightness, never off. A grid skin draws it
//! ([`CursorView`](super::CursorView)).

use shuyi_core::ease::{ONE, Repeat};
use shuyi_core::{Dir, Ease, Family, Pos, Tween};

#[cfg(not(feature = "std"))]
use crate::float::Float;
use crate::input::frame_seconds;

/// One full bright → dim → bright pulse, in seconds. *Tunable.*
pub const BLINK_PERIOD: f32 = 1.0;

/// The dimmest the cursor gets, as a fraction of full brightness. *Tunable.*
pub const BLINK_MIN: f32 = 0.5;

/// [`ONE`], the whole way, as an `f32`.
const ONE_F32: f32 = 65_536.0;

/// [`BLINK_MIN`] in 1/65536.
#[expect(
    clippy::cast_possible_truncation,
    reason = "a fraction of 1 in 1/65536 fits an i32"
)]
const DIM: i32 = (BLINK_MIN * ONE_F32) as i32;

/// The pulse, its time in 1/65536 of a period: from full brightness
/// ([`ONE`]) down to [`BLINK_MIN`] over half a period, eased in and out
/// along a sine, then back up, for ever.
const PULSE: Tween<i32> = Tween::new(ONE, DIM, ONE.unsigned_abs() / 2)
    .with_ease(Ease::InOut(Family::Sine))
    .with_repeat(Repeat::Loop)
    .with_yoyo(true);

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
    /// [`BLINK_MIN`] halfway through, then back up: a looping, yoyo
    /// [`Tween`] along [`Ease::InOut`]`(`[`Family::Sine`]`)`, the same on
    /// every machine.
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
/// [`BLINK_MIN`] halfway, back to `1` at the end: [`PULSE`], a looping,
/// yoyo [`Tween`] along [`Ease::InOut`]`(`[`Family::Sine`]`)`. `t` is
/// brought into one period first (infinity and not a number count as 0),
/// then read in 1/65536 of a period.
fn pulse(t: f32, period: f32) -> f32 {
    let into = if t.is_finite() { t % period } else { 0.0 };
    let into = if into < 0.0 { into + period } else { into };
    // 0 to 65 536, a whole period, which a `u32` holds; not a number (a
    // period of 0) counts as 0.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "0 to 65 536"
    )]
    let at = (into / period * ONE_F32).round() as u32;
    #[expect(clippy::cast_precision_loss, reason = "BLINK_MIN to 65 536")]
    let value = PULSE.value_at(at) as f32;
    value / ONE_F32
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

    /// The pulse as it was written by hand before ticket 2136: a cosine.
    fn pulse_before(t: f32, period: f32) -> f32 {
        let wave = (core::f32::consts::TAU * t / period).cos();
        BLINK_MIN + (1.0 - BLINK_MIN) * (1.0 + wave) / 2.0
    }

    /// How far the tween may be from the cosine: 4/65536.
    const BOUND: f32 = 4.0 / 65_536.0;

    #[test]
    fn pulse_is_exact_at_its_ends_and_middle() {
        assert_eq!(pulse(0.0, 1.0), 1.0);
        assert_eq!(pulse(0.25, 1.0), 0.75);
        assert_eq!(pulse(0.5, 1.0), 0.5);
        assert_eq!(pulse(0.75, 1.0), 0.75);
        assert_eq!(pulse(1.0, 1.0), 1.0);
        // Below 0 and past the period it goes round.
        assert_eq!(pulse(-0.5, 1.0), 0.5);
        assert_eq!(pulse(-0.25, 1.0), 0.75);
        assert_eq!(pulse(3.5, 1.0), 0.5);
        // Infinity and not a number count as 0.
        for t in [f32::INFINITY, f32::NEG_INFINITY, f32::NAN] {
            assert_eq!(pulse(t, 1.0), 1.0, "{t}");
        }
    }

    #[test]
    fn pulse_stays_near_the_cosine_it_replaced() {
        for k in 0..=4096_u16 {
            let t = f32::from(k) / 4096.0;
            let off = (pulse(t, BLINK_PERIOD) - pulse_before(t, BLINK_PERIOD)).abs();
            assert!(off <= BOUND, "{t}: {off}");
        }
        for t in [-0.3, -1.7, -12.125, 1.3, 2.9, 40.6] {
            let off = (pulse(t, BLINK_PERIOD) - pulse_before(t, BLINK_PERIOD)).abs();
            assert!(off <= BOUND, "{t}: {off}");
        }
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

        #[test]
        fn brightness_stays_between_min_and_full_for_any_time(t in any::<f32>()) {
            let c = Cursor { pos: Pos::new(0, 0), blink_t: t };
            let b = c.brightness();
            prop_assert!((BLINK_MIN..=1.0).contains(&b), "{t}: {b}");
        }
    }
}
