//! Seeded random numbers for a deterministic simulation.
//!
//! [`SimRng`] is a PCG32 (XSH RR, 64-bit state) written in this crate, so its
//! output is fixed by this file alone: same seed ⇒ same numbers on every
//! platform and every dependency version. It is serialisable so a game's
//! state (and its RNG position) can be saved, rewound and replayed.
//!
//! Game rules take a [`RandomSource`], so tests can inject a [`ScriptedRng`]
//! that returns a given list of rolls.

use alloc::vec::Vec;

use serde::{Deserialize, Serialize};

/// Where game rules get their random numbers from.
pub trait RandomSource {
    /// A uniform, unbiased integer in `0..=99`.
    fn roll_percent(&mut self) -> u8;

    /// A uniform, unbiased integer in `0..n`. `n` must be at least 1; `0`
    /// gives `0`.
    fn roll_below(&mut self, n: u32) -> u32;
}

/// The PCG32 multiplier (O'Neill, "PCG: A Family of Simple Fast
/// Space-Efficient Statistically Good Algorithms for Random Number
/// Generation", 2014).
const PCG_MULTIPLIER: u64 = 6_364_136_223_846_793_005;

/// The stream every [`SimRng::new`] uses. Any odd increment works; one fixed
/// stream keeps a seed a complete description of the sequence.
const DEFAULT_STREAM: u64 = 54;

/// The largest multiple of 100 that fits in `2^32`: a `u32` below this,
/// taken mod 100, is uniform in `0..=99`.
const PERCENT_LIMIT: u32 = (u32::MAX / 100) * 100;

/// The most redraws one [`SimRng::roll_percent`] makes.
const MAX_REDRAWS: u32 = 16;

/// The seeded simulation RNG, kept in the game's state.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SimRng {
    state: u64,
    inc: u64,
}

impl SimRng {
    /// A generator seeded with `seed`.
    pub fn new(seed: u64) -> Self {
        Self::with_stream(seed, DEFAULT_STREAM)
    }

    /// PCG32's reference seeding (`pcg32_srandom_r`) with stream `stream`.
    fn with_stream(seed: u64, stream: u64) -> Self {
        let mut rng = Self {
            state: 0,
            inc: (stream << 1) + 1,
        };
        rng.next_u32();
        rng.state = rng.state.wrapping_add(seed);
        rng.next_u32();
        rng
    }

    /// The next 32 random bits.
    pub fn next_u32(&mut self) -> u32 {
        let old = self.state;
        self.state = old.wrapping_mul(PCG_MULTIPLIER).wrapping_add(self.inc);
        // XSH RR output: the top 32 of the xorshifted 64 bits, rotated by the
        // top 5 bits of the old state. Both truncations are the algorithm.
        #[allow(clippy::cast_possible_truncation)]
        let xorshifted = (((old >> 18) ^ old) >> 27) as u32;
        #[allow(clippy::cast_possible_truncation)]
        let rot = (old >> 59) as u32;
        xorshifted.rotate_right(rot)
    }

    /// A uniform integer in `0..=99`. Rejection sampling: draws that would
    /// make low values more likely (the top `2^32 mod 100` values) are
    /// thrown away and redrawn, at most 16 times. Each draw is
    /// rejected with chance `96 / 2^32`, so the cap is never reached in
    /// practice; it only guarantees the loop ends.
    pub fn roll_percent(&mut self) -> u8 {
        let mut x = self.next_u32();
        for _ in 0..MAX_REDRAWS {
            if accepted(x) {
                break;
            }
            x = self.next_u32();
        }
        u8::try_from(x % 100).unwrap_or(u8::MAX)
    }

    /// A uniform integer in `0..n` (`0` when `n` is 0). Rejection sampling
    /// as in [`Self::roll_percent`]: draws at or above the largest multiple
    /// of `n` that fits in `2^32` are redrawn, at most 16 times.
    pub fn roll_below(&mut self, n: u32) -> u32 {
        if n == 0 {
            return 0;
        }
        let mut x = self.next_u32();
        for _ in 0..MAX_REDRAWS {
            if accepted_below(x, n) {
                break;
            }
            x = self.next_u32();
        }
        x % n
    }
}

/// The largest multiple of `n` (≥ 1) that is at most `u32::MAX`: a `u32`
/// below it, taken mod `n`, is uniform.
fn below_limit(n: u32) -> u32 {
    (u32::MAX / n) * n
}

/// Whether `x mod n` is unbiased: `x` is below [`below_limit`]`(n)`.
fn accepted_below(x: u32, n: u32) -> bool {
    x < below_limit(n)
}

/// Whether `x mod 100` is unbiased: `x` is below [`PERCENT_LIMIT`].
fn accepted(x: u32) -> bool {
    x < PERCENT_LIMIT
}

impl RandomSource for SimRng {
    fn roll_percent(&mut self) -> u8 {
        SimRng::roll_percent(self)
    }

    fn roll_below(&mut self, n: u32) -> u32 {
        SimRng::roll_below(self, n)
    }
}

/// A [`RandomSource`] that returns a fixed list of rolls, in order. For
/// tests that need an exact trace of rolls. Percent rolls and
/// [`roll_below`](RandomSource::roll_below) draws come from two separate
/// lists.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ScriptedRng {
    rolls: Vec<u8>,
    next: usize,
    below: Vec<u32>,
    next_below: usize,
}

impl ScriptedRng {
    /// Returns `rolls` one by one. Each must be in `0..=99`.
    pub fn new(rolls: impl Into<Vec<u8>>) -> Self {
        Self {
            rolls: rolls.into(),
            ..Self::default()
        }
    }

    /// Also returns `draws`, one by one, from
    /// [`roll_below`](RandomSource::roll_below).
    #[must_use]
    pub fn with_below(mut self, draws: impl Into<Vec<u32>>) -> Self {
        self.below = draws.into();
        self
    }

    /// How many percent rolls have been taken so far.
    pub fn consumed(&self) -> usize {
        self.next
    }

    /// How many `roll_below` draws have been taken so far.
    pub fn consumed_below(&self) -> usize {
        self.next_below
    }
}

impl RandomSource for ScriptedRng {
    /// # Panics
    ///
    /// When the script is used up: the test expected fewer rolls.
    fn roll_percent(&mut self) -> u8 {
        let Some(&roll) = self.rolls.get(self.next) else {
            panic!("ScriptedRng ran out after {} rolls", self.next);
        };
        self.next += 1;
        roll
    }

    /// # Panics
    ///
    /// When the draws are used up, or the next one isn't below `n`: the
    /// test expected other draws.
    fn roll_below(&mut self, n: u32) -> u32 {
        let Some(&draw) = self.below.get(self.next_below) else {
            panic!("ScriptedRng ran out after {} draws", self.next_below);
        };
        assert!(draw < n.max(1), "scripted draw {draw} is not below {n}");
        self.next_below += 1;
        draw
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percent_limit_is_the_largest_multiple_of_100_in_u32() {
        assert_eq!(PERCENT_LIMIT % 100, 0);
        assert_eq!(u64::from(PERCENT_LIMIT) + 100, (1u64 << 32) + 4);
        assert_eq!(PERCENT_LIMIT, 4_294_967_200);
    }

    #[test]
    fn biased_draws_are_rejected() {
        assert!(accepted(0));
        assert!(accepted(PERCENT_LIMIT - 1));
        assert!(!accepted(PERCENT_LIMIT));
        assert!(!accepted(u32::MAX));
    }

    #[test]
    fn matches_pcg32_reference_output() {
        // First outputs of the PCG reference `pcg32-demo` (seed 42, stream 54).
        let mut rng = SimRng::with_stream(42, 54);
        let got: Vec<u32> = (0..6).map(|_| rng.next_u32()).collect();
        assert_eq!(
            got,
            [
                0xa15c_02b7,
                0x7b47_f409,
                0xba1d_3330,
                0x83d2_f293,
                0xbfa4_784b,
                0xcbed_606e
            ]
        );
    }

    #[test]
    fn fixed_seed_gives_fixed_rolls() {
        let mut rng = SimRng::new(2026);
        let got: Vec<u8> = (0..10).map(|_| rng.roll_percent()).collect();
        assert_eq!(got, [74, 36, 25, 49, 16, 67, 87, 89, 28, 12]);
    }

    #[test]
    fn different_seeds_differ() {
        let a: Vec<u32> = {
            let mut r = SimRng::new(1);
            (0..4).map(|_| r.next_u32()).collect()
        };
        let b: Vec<u32> = {
            let mut r = SimRng::new(2);
            (0..4).map(|_| r.next_u32()).collect()
        };
        assert_ne!(a, b);
    }

    #[test]
    fn serialised_rng_resumes_the_same_sequence() {
        let mut rng = SimRng::new(7);
        for _ in 0..5 {
            rng.next_u32();
        }
        let text = ron::to_string(&rng).expect("serialise");
        let mut copy: SimRng = ron::from_str(&text).expect("deserialise");
        assert_eq!(copy, rng);
        let a: Vec<u8> = (0..20).map(|_| rng.roll_percent()).collect();
        let b: Vec<u8> = (0..20).map(|_| copy.roll_percent()).collect();
        assert_eq!(a, b);
    }

    #[test]
    fn roll_percent_is_roughly_uniform() {
        // Chi-square over 100 buckets (99 degrees of freedom: mean 99,
        // sd ≈ 14). Seeded, so the value is fixed; the bound is ~4 sd.
        let mut rng = SimRng::new(12_345);
        let n = 100_000;
        let mut counts = [0u32; 100];
        for _ in 0..n {
            counts[usize::from(rng.roll_percent())] += 1;
        }
        let expected = f64::from(n) / 100.0;
        let chi2: f64 = counts
            .iter()
            .map(|&c| (f64::from(c) - expected).powi(2) / expected)
            .sum();
        assert!(chi2 < 155.0, "chi-square {chi2}");
    }

    #[test]
    fn roll_percent_trait_matches_inherent() {
        let mut a = SimRng::new(9);
        let mut b = SimRng::new(9);
        for _ in 0..50 {
            assert_eq!(RandomSource::roll_percent(&mut a), b.roll_percent());
        }
    }

    #[test]
    fn scripted_rng_replays_its_script() {
        let mut rng = ScriptedRng::new([3, 99, 0]);
        assert_eq!(rng.consumed(), 0);
        assert_eq!(rng.roll_percent(), 3);
        assert_eq!(rng.roll_percent(), 99);
        assert_eq!(rng.consumed(), 2);
        assert_eq!(rng.roll_percent(), 0);
        assert_eq!(rng.consumed(), 3);
    }

    #[test]
    fn below_limit_is_a_multiple_of_n() {
        assert_eq!(below_limit(1), u32::MAX);
        assert_eq!(below_limit(100), PERCENT_LIMIT);
        assert_eq!(below_limit(3), u32::MAX);
        assert_eq!(below_limit(280), 4_294_967_040);
        assert_eq!(below_limit(u32::MAX), u32::MAX);
    }

    #[test]
    fn biased_below_draws_are_rejected() {
        // 280 × 15_339_168 = 4_294_967_040: draws from there on are redrawn.
        assert!(accepted_below(0, 280));
        assert!(accepted_below(4_294_967_039, 280));
        assert!(!accepted_below(4_294_967_040, 280));
        assert!(!accepted_below(u32::MAX, 280));
    }

    #[test]
    fn roll_below_stays_in_range_and_is_fixed_by_the_seed() {
        let mut rng = SimRng::new(2026);
        let got: Vec<u32> = (0..10).map(|_| rng.roll_below(280)).collect();
        assert!(got.iter().all(|&r| r < 280));
        let mut again = SimRng::new(2026);
        let same: Vec<u32> = (0..10)
            .map(|_| RandomSource::roll_below(&mut again, 280))
            .collect();
        assert_eq!(got, same);
        assert_eq!(rng.roll_below(0), 0);
        assert_eq!(rng.roll_below(1), 0);
    }

    #[test]
    fn roll_below_is_roughly_uniform() {
        // Chi-square over 7 buckets (6 degrees of freedom: mean 6, sd ≈ 3.5).
        let mut rng = SimRng::new(99);
        let n = 70_000;
        let mut counts = [0u32; 7];
        for _ in 0..n {
            counts[rng.roll_below(7) as usize] += 1;
        }
        let expected = f64::from(n) / 7.0;
        let chi2: f64 = counts
            .iter()
            .map(|&c| (f64::from(c) - expected).powi(2) / expected)
            .sum();
        assert!(chi2 < 25.0, "chi-square {chi2}");
    }

    #[test]
    fn scripted_rng_replays_its_draws() {
        let mut rng = ScriptedRng::new([4]).with_below([7, 0]);
        assert_eq!(rng.roll_below(10), 7);
        assert_eq!(rng.roll_percent(), 4);
        assert_eq!(rng.roll_below(1), 0);
        assert_eq!((rng.consumed(), rng.consumed_below()), (1, 2));
    }

    #[test]
    #[should_panic(expected = "not below 5")]
    fn scripted_rng_checks_draws_are_in_range() {
        ScriptedRng::new([]).with_below([5]).roll_below(5);
    }

    #[test]
    #[should_panic(expected = "ran out after 0 draws")]
    fn scripted_rng_panics_when_draws_run_out() {
        ScriptedRng::new([]).roll_below(5);
    }

    #[test]
    #[should_panic(expected = "ran out after 1 rolls")]
    fn scripted_rng_panics_when_exhausted() {
        let mut rng = ScriptedRng::new([5]);
        rng.roll_percent();
        rng.roll_percent();
    }
}
