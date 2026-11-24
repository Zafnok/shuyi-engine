//! Seeded random numbers for a deterministic simulation.
//!
//! [`SimRng`] is a PCG32 (XSH RR, 64-bit state) written in this crate, so its
//! output is fixed by this file alone: same seed ⇒ same numbers on every
//! platform and every dependency version. It is serialisable so a game's
//! state (and its RNG position) can be saved, rewound and replayed.
//!
//! Game rules take a [`RandomSource`], so tests can inject a [`ScriptedRng`]
//! that returns a given list of rolls.
//!
//! Which generator to make:
//!
//! - [`SimRng::new`] for a game with one stream of luck;
//! - [`SimRng::stream`] for one stream per purpose (`"combat"`, `"loot"`,
//!   `"weather"`): a draw added on one never moves another, so a new
//!   cosmetic pick leaves the battle's replays as they were;
//! - [`SimRng::fork`] for a sub-task (a level built from a run's luck, a
//!   bot's search): the child draws as much as it likes, the parent moves
//!   by two draws.
//!
//! [`RandomSource`]'s helpers give a range, a chance, an index, a pick, a
//! shuffle and a weighted pick, each from a fixed number of
//! [`roll_below`](RandomSource::roll_below) draws, never the biased
//! `next_u32() % n`. A [`Bag`] deals each of its items once before any
//! repeats. [`mix64`] and [`mix_at`] are `SplitMix64`, for a value that is a
//! function of a seed and an index with no state.
//!
//! Every output here is frozen: saves, replays and golden hashes depend on
//! them (ADR-0143, `docs/adr/0143-the-rngs-helpers-streams-and-forks-are-frozen-too.md`).
//!
//! Dealing a shuffled deck:
//!
//! ```
//! use shuyi_core::{RandomSource, SimRng};
//!
//! let mut rng = SimRng::stream(2026, "deal");
//! let mut deck: Vec<u8> = (1..=52).collect();
//! rng.shuffle(&mut deck);
//! let hand: Vec<u8> = deck.drain(..5).collect();
//! assert_eq!(hand.len(), 5);
//! assert_eq!(deck.len(), 47);
//! ```
//!
//! Putting a pickup on a free cell:
//!
//! ```
//! use shuyi_core::{Pos, RandomSource, SimRng};
//!
//! let mut rng = SimRng::stream(2026, "pickups");
//! let free = [Pos::new(1, 1), Pos::new(4, 2), Pos::new(3, 5)];
//! let at = rng.pick(&free).copied();
//! assert!(at.is_some_and(|pos| free.contains(&pos)));
//! ```
//!
//! Drawing pieces from a bag of seven:
//!
//! ```
//! use shuyi_core::{Bag, SimRng};
//!
//! let mut rng = SimRng::stream(2026, "pieces");
//! let mut bag = Bag::new(vec!['I', 'O', 'T', 'S', 'Z', 'J', 'L']);
//! let mut first: Vec<char> = (0..7).filter_map(|_| bag.draw(&mut rng).copied()).collect();
//! first.sort_unstable();
//! assert_eq!(first, ['I', 'J', 'L', 'O', 'S', 'T', 'Z']);
//! assert_eq!(bag.left(), 0);
//! ```

use alloc::vec::Vec;
use core::ops::Range;

use serde::{Deserialize, Serialize};

mod bag;

pub use bag::Bag;

/// The golden ratio's 64-bit increment, `SplitMix64`'s step.
const GOLDEN_GAMMA: u64 = 0x9E37_79B9_7F4A_7C15;

/// `SplitMix64`'s output for the state `x` (Steele, Lea and Flood, "Fast
/// Splittable Pseudorandom Number Generators", 2014): add the golden
/// gamma, then the finalizer. It spreads nearby values (a counter, a seed
/// plus one) over the whole range.
#[must_use]
pub const fn mix64(x: u64) -> u64 {
    let mut z = x.wrapping_add(GOLDEN_GAMMA);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// The `index`-th output of `SplitMix64` started at `seed`, with no state:
/// a value that is a function of the seed and the index alone (a
/// particle's, a noise lattice point's).
#[must_use]
pub const fn mix_at(seed: u64, index: u64) -> u64 {
    mix64(seed.wrapping_add(index.wrapping_mul(GOLDEN_GAMMA)))
}

/// Where game rules get their random numbers from.
///
/// Its helpers are built on [`roll_below`](Self::roll_below) alone, so
/// every source has them, and each makes a fixed number of draws (frozen
/// by ADR-0143). None calls `roll_below(0)`.
pub trait RandomSource {
    /// A uniform, unbiased integer in `0..=99`.
    fn roll_percent(&mut self) -> u8;

    /// A uniform, unbiased integer in `0..n`. `n` must be at least 1; `0`
    /// gives `0`.
    fn roll_below(&mut self, n: u32) -> u32;

    /// A value in `r`, each as likely; `r.start` when `r` is empty. One
    /// draw, none when empty. Any `i32` range works, nothing wraps.
    fn range(&mut self, r: Range<i32>) -> i32 {
        if r.is_empty() {
            return r.start;
        }
        let span = u32::try_from(i64::from(r.end) - i64::from(r.start)).unwrap_or(u32::MAX);
        let value = i64::from(r.start) + i64::from(self.roll_below(span));
        i32::try_from(value).unwrap_or(r.start)
    }

    /// `true` with odds `num / den`; `false` when `den` is 0. One draw when
    /// `den` is above 0, even when `num >= den`, so changing the odds never
    /// changes the draw count.
    fn chance(&mut self, num: u32, den: u32) -> bool {
        den > 0 && self.roll_below(den) < num
    }

    /// A value in `0..len`, each as likely; `None` when `len` is 0. One
    /// draw, none when 0. A `len` above `u32::MAX` picks among the first
    /// `u32::MAX`.
    fn index(&mut self, len: usize) -> Option<usize> {
        if len == 0 {
            return None;
        }
        let n = u32::try_from(len).unwrap_or(u32::MAX);
        usize::try_from(self.roll_below(n)).ok()
    }

    /// One of `items`, each as likely; `None` when there are none. Draws as
    /// [`index`](Self::index).
    fn pick<'a, T>(&mut self, items: &'a [T]) -> Option<&'a T>
    where
        Self: Sized,
    {
        items.get(self.index(items.len())?)
    }

    /// `items` in a random order, each order as likely (Fisher-Yates): for
    /// `i` from `len - 1` down to 1, one `roll_below(i + 1)`, then `i` is
    /// swapped with the draw. `len - 1` draws, none for 0 or 1 items.
    fn shuffle<T>(&mut self, items: &mut [T])
    where
        Self: Sized,
    {
        for i in (1..items.len()).rev() {
            let n = u32::try_from(i + 1).unwrap_or(u32::MAX);
            let j = usize::try_from(self.roll_below(n)).unwrap_or(0);
            items.swap(i, j);
        }
    }

    /// `i` with odds `weights[i] / total`; `None` when the total is 0 or
    /// above `u32::MAX`. One `roll_below(total)`, none on `None`.
    fn weighted_index(&mut self, weights: &[u32]) -> Option<usize> {
        weighted(self, || weights.iter().copied())
    }

    /// One of `items`, by the weight beside it. Draws as
    /// [`weighted_index`](Self::weighted_index).
    fn pick_weighted<'a, T>(&mut self, items: &'a [(u32, T)]) -> Option<&'a T>
    where
        Self: Sized,
    {
        let i = weighted(self, || items.iter().map(|(weight, _)| *weight))?;
        items.get(i).map(|(_, item)| item)
    }
}

/// The weighted pick of [`RandomSource::weighted_index`], over the weights
/// `weights` gives (twice: once for the total, once to walk).
fn weighted<R, I>(rng: &mut R, weights: impl Fn() -> I) -> Option<usize>
where
    R: RandomSource + ?Sized,
    I: Iterator<Item = u32>,
{
    let total = weights().fold(0u64, |sum, w| sum.saturating_add(u64::from(w)));
    let total = u32::try_from(total).ok().filter(|&t| t > 0)?;
    let draw = rng.roll_below(total);
    let mut below = 0u32;
    weights().position(|w| {
        below += w;
        draw < below
    })
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
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SimRng {
    state: u64,
    inc: u64,
}

impl SimRng {
    /// A generator seeded with `seed`.
    pub fn new(seed: u64) -> Self {
        Self::with_stream(seed, DEFAULT_STREAM)
    }

    /// The stream named `name` of `seed`: one stream per purpose
    /// (`"combat"`, `"loot"`), so draws on one never move another. With
    /// `id` the 64-bit FNV-1a hash of the name, it is PCG32 stream `id`
    /// seeded with [`mix_at`]`(seed, id)`. Names are told apart by that
    /// hash alone.
    pub fn stream(seed: u64, name: &str) -> Self {
        let id = crate::hash::fnv1a(name.as_bytes());
        Self::with_stream(mix_at(seed, id), id)
    }

    /// A child generator for a sub-task, named `name`: exactly two
    /// [`next_u32`](Self::next_u32) draws from `self` make a 64-bit seed
    /// (the first its high half), and the child is
    /// [`stream`](Self::stream)`(seed, name)`. However much the child
    /// draws, `self` moved by two.
    #[must_use]
    pub fn fork(&mut self, name: &str) -> Self {
        let high = u64::from(self.next_u32());
        let low = u64::from(self.next_u32());
        Self::stream((high << 32).wrapping_add(low), name)
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
mod tests;

#[cfg(test)]
mod pcg_tests {
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
