//! [`HurryClock`]: the time of something timed that the player can hurry by
//! holding a key and finish with a press (ticket 2312): a text box typing
//! out, a tally counting up, a fight played back.

use crate::input::frame_seconds;

/// How much faster time passes while the key is held, once it has been
/// held for [`HURRY_DELAY`] (the text box's fast forward).
pub const HURRY_FACTOR: f32 = 6.0;
/// Seconds the key must be held before time passes faster, so the tap
/// that started the step doesn't hurry it.
pub const HURRY_DELAY: f32 = 0.3;

/// The clock of one timed step: each frame's seconds as they pass, faster
/// once the player has held the key a moment, and whether a press has
/// skipped the step.
///
/// Its owner calls [`tick`](Self::tick) each frame with whether the key
/// that hurries is held, and moves its step on by the seconds it returns.
/// A press calls [`skip`](Self::skip), and the owner, seeing
/// [`is_skipped`](Self::is_skipped), finishes the step at once. A new step
/// [`reset`](Self::reset)s the clock.
///
/// The hold counts from the first frame the key is held, including that
/// frame: the part of a frame past [`delay`](Self::delay) goes
/// [`factor`](Self::factor) times as fast, the part before it as fast as
/// ever.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HurryClock {
    /// Seconds the key must be held before time passes faster.
    pub delay: f32,
    /// How much faster time passes after that.
    pub factor: f32,
    /// Seconds the key has been held, this frame's included.
    held_s: f32,
    /// Whether a press has skipped the step.
    skipped: bool,
}

impl Default for HurryClock {
    /// The text box's: [`HURRY_DELAY`] and [`HURRY_FACTOR`].
    fn default() -> Self {
        Self::new(HURRY_DELAY, HURRY_FACTOR)
    }
}

impl HurryClock {
    /// A clock that passes time `factor` times as fast once the key has
    /// been held `delay` seconds; nothing held, nothing skipped.
    pub const fn new(delay: f32, factor: f32) -> Self {
        Self {
            delay,
            factor,
            held_s: 0.0,
            skipped: false,
        }
    }

    /// `dt` seconds on, with the key `held` or not: the seconds that pass
    /// this frame ([`hold`](Self::hold), then [`passing`](Self::passing)).
    pub fn tick(&mut self, dt: f32, held: bool) -> f32 {
        self.hold(held, dt);
        self.passing(dt)
    }

    /// Counts `dt` seconds of the key held, or, let go, starts the count
    /// over. A `dt` that is not finite or is negative counts as 0.
    pub fn hold(&mut self, held: bool, dt: f32) {
        if held {
            self.held_s += frame_seconds(dt);
        } else {
            self.held_s = 0.0;
        }
    }

    /// The seconds that pass for a frame of `dt` that the key has been held
    /// through so far: `dt`, with the part of it past the delay
    /// [`factor`](Self::factor) times as fast. A `dt` that is not finite
    /// or is negative counts as 0.
    pub fn passing(&self, dt: f32) -> f32 {
        let dt = frame_seconds(dt);
        let fast = (self.held_s - self.delay).clamp(0.0, dt);
        dt + fast * (self.factor - 1.0)
    }

    /// The key is no longer counted as held (the step waits on something
    /// else), as if let go.
    pub fn release(&mut self) {
        self.held_s = 0.0;
    }

    /// A press finishes the step: [`is_skipped`](Self::is_skipped) until
    /// [`reset`](Self::reset).
    pub fn skip(&mut self) {
        self.skipped = true;
    }

    /// Whether a press has skipped the step.
    pub fn is_skipped(&self) -> bool {
        self.skipped
    }

    /// A new step: nothing held, nothing skipped, the same delay and
    /// factor.
    pub fn reset(&mut self) {
        self.held_s = 0.0;
        self.skipped = false;
    }
}

#[cfg(test)]
mod tests;
