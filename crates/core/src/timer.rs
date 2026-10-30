//! Timers a game's rules set and forget, counted in steps or turns:
//! [`Countdown`], which fires once and is the cooldown too, and [`Every`],
//! which repeats. With them, the step rate ([`STEPS_PER_SECOND`]) and the
//! two conversions a length in milliseconds goes through
//! ([`steps_from_ms`], [`ms_from_steps`]).
//!
//! # Two kinds of time
//!
//! **Simulation time** is steps or turns. A tick game steps its rules once
//! for each of `input.ticks()` and advances its timers by 1 in each step,
//! so a timer fires on the same step at 30 and at 60 frames a second. A
//! turn game advances by 1 where its turn or round ends, in its own
//! [`TurnRules::apply`](crate::TurnRules::apply). Timers live in the state
//! the rules own, so they save, hash ([`state_hash`](crate::state_hash))
//! and replay with it. A timer moves only when it is advanced, so a paused
//! game's timers wait.
//!
//! **Presentation time** is `FrameInput::dt()`, `Ctx::clock_s` and
//! `HurryClock`, in `shuyi-ui`: glides, blinks, text typing out, a clock on
//! screen. It may stay in floats, and no rule ever reads it.
//!
//! ```
//! use shuyi_core::timer::Every;
//!
//! // Something that moves every 8 steps. `ticks` stands in for
//! // `input.ticks()`, the steps due this frame.
//! let mut mover = Every::new(8);
//! let mut x = 0;
//! for ticks in [1, 1, 2, 4, 1, 0, 2] {
//!     for _ in 0..ticks {
//!         // One step of the rules.
//!         x += mover.advance(1);
//!     }
//! }
//! assert_eq!(x, 1);
//! assert_eq!(mover.until_next(), 5);
//! ```
//!
//! # Many timers, and things that happen later
//!
//! Many timers are a `BTreeMap<K, Countdown>` in the state, never a
//! `HashMap`, whose order changes the hash ([`hash`](crate::hash)). An event
//! that happens later is a [`Countdown`] kept beside it, which sends it when
//! it fires.
//!
//! # Values read from a file
//!
//! Nothing checks a timer read from a hand-edited save. A value out of range
//! (`left` above `length`, `since` at or above `period`) still behaves by the
//! formulas on each method, and no method panics on it.

use serde::{Deserialize, Serialize};

/// Fixed simulation steps a second.
pub const STEPS_PER_SECOND: u32 = 60;

/// Milliseconds in a second.
const MS_PER_SECOND: u64 = 1000;

/// Whole steps in `ms` milliseconds at [`STEPS_PER_SECOND`], rounded up, so
/// any time above 0 is at least one step. It turns a length a designer wrote
/// in milliseconds into steps, once, at load. A timer never counts
/// milliseconds.
#[expect(
    clippy::cast_possible_truncation,
    reason = "the most it gives is 257 698 038, for `u32::MAX` ms, which fits"
)]
pub const fn steps_from_ms(ms: u32) -> u32 {
    (ms as u64 * STEPS_PER_SECOND as u64).div_ceil(MS_PER_SECOND) as u32
}

/// The milliseconds `steps` steps last at [`STEPS_PER_SECOND`], rounded down:
/// 0, 16, 33, 50, … and exactly 1000 at step 60, so it never drifts as a whole
/// number of milliseconds per step would. [`steps_from_ms`] of it gives
/// `steps` back while that fits a `u32`.
///
/// It is the engine's one rule from steps to milliseconds: data timed in
/// milliseconds (a clip, a particle's life, a bone clip) is read at
/// `ms_from_steps(step - start_step)`, by `frame_at`, `particles_at` and
/// `pose_at` alike.
pub const fn ms_from_steps(steps: u32) -> u64 {
    steps as u64 * MS_PER_SECOND / STEPS_PER_SECOND as u64
}

/// A timer that fires once, after `length` steps or turns, and waits to be
/// [restarted](Self::restart).
///
/// It is the cooldown too. Advance it by 1 each step or turn, and:
///
/// ```
/// use shuyi_core::timer::Countdown;
///
/// // Ready for its first use, then two turns' wait.
/// let mut cooldown = Countdown::new_ready(2);
/// let mut uses = 0;
/// for used in [true, true, false, true] {
///     if used && cooldown.ready() {
///         uses += 1;
///         cooldown.restart();
///     }
///     cooldown.advance(1);
/// }
/// assert_eq!(uses, 2);
/// ```
///
/// The default has a length of 0 and is ready.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Countdown {
    /// What [`restart`](Self::restart) starts it from.
    length: u32,
    /// What is left of it.
    left: u32,
}

impl Countdown {
    /// One started: it fires after `length`.
    pub const fn new(length: u32) -> Self {
        Self {
            length,
            left: length,
        }
    }

    /// One done already, for a cooldown whose first use goes through.
    pub const fn new_ready(length: u32) -> Self {
        Self { length, left: 0 }
    }

    /// Takes `n` off what is left, stopping at 0. `true` only on the call
    /// that takes it from above 0 to 0, so it fires once however far `n`
    /// overshoots, and `advance(0)` never fires. A length of 0 is ready at
    /// once and never fires.
    pub fn advance(&mut self, n: u32) -> bool {
        let running = self.left > 0;
        self.left = self.left.saturating_sub(n);
        running && self.left == 0
    }

    /// Whether nothing is left.
    pub const fn ready(&self) -> bool {
        self.left == 0
    }

    /// What is left of it.
    pub const fn remaining(&self) -> u32 {
        self.left
    }

    /// How much of it has passed, for a bar that fills: its length less what
    /// is left, stopping at 0.
    pub const fn elapsed(&self) -> u32 {
        self.length.saturating_sub(self.left)
    }

    /// The whole of it.
    pub const fn length(&self) -> u32 {
        self.length
    }

    /// Starts it again from its length.
    pub const fn restart(&mut self) {
        self.left = self.length;
    }
}

/// A timer that fires every `period` steps or turns.
///
/// A period of 0 acts as 1, whether given to [`new`](Self::new) or read from
/// a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Every {
    /// How often it fires. Read through [`period`](Self::period).
    period: u32,
    /// How far it is since it last fired.
    since: u32,
}

impl Every {
    /// One that first fires after a whole `period`.
    pub const fn new(period: u32) -> Self {
        Self {
            period: if period == 0 { 1 } else { period },
            since: 0,
        }
    }

    /// Moves it on by `n`, and says how many times it fired: `(since + n) /
    /// period` times, keeping the remainder. The count saturates at
    /// `u32::MAX`.
    pub fn advance(&mut self, n: u32) -> u32 {
        let period = u64::from(self.period());
        let total = u64::from(self.since) + u64::from(n);
        // Less than `period`, so it fits.
        self.since = u32::try_from(total % period).unwrap_or(0);
        u32::try_from(total / period).unwrap_or(u32::MAX)
    }

    /// How often it fires, at least 1.
    pub fn period(&self) -> u32 {
        self.period.max(1)
    }

    /// How far it is to its next firing: the period less how far it has
    /// come, and 1 if that's out of range, because the next `advance(1)`
    /// fires then.
    pub fn until_next(&self) -> u32 {
        self.period().saturating_sub(self.since).max(1)
    }

    /// Back to a whole period before its next firing.
    pub const fn reset(&mut self) {
        self.since = 0;
    }
}

#[cfg(test)]
mod tests;
