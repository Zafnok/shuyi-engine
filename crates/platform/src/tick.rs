//! [`TickClock`]: real time turned into fixed simulation steps (ADR-0003
//! §2): 60 a second, at most 4 in one frame, after which the game slows
//! down rather than spiral. Whole nanoseconds, so it counts the same on
//! every machine.

use core::time::Duration;

/// Fixed simulation steps a second.
pub const STEPS_PER_SECOND: u32 = 60;

/// The most steps one frame runs: a frame later than this drops the rest
/// of its time.
pub const MAX_STEPS: u32 = 4;

/// Nanoseconds in a second, in the units [`TickClock`] counts in.
const SECOND: u64 = 1_000_000_000;

/// The time a frame brings, turned into fixed steps; what is left of a step
/// waits for the next frame.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TickClock {
    /// Time not yet stepped, in sixtieths of a nanosecond, so a step is a
    /// whole number of them ([`SECOND`]).
    owed: u64,
}

impl TickClock {
    /// A clock owing nothing.
    pub const fn new() -> Self {
        Self { owed: 0 }
    }

    /// How many steps `elapsed` brings, at most [`MAX_STEPS`]. When a frame
    /// is that late its extra time is dropped, keeping only the part of a
    /// step it was into.
    pub fn advance(&mut self, elapsed: Duration) -> u32 {
        let nanos = u64::try_from(elapsed.as_nanos()).unwrap_or(u64::MAX);
        self.owed = self
            .owed
            .saturating_add(nanos.saturating_mul(u64::from(STEPS_PER_SECOND)));
        let due = self.owed / SECOND;
        self.owed %= SECOND;
        // At most MAX_STEPS, which fits.
        u32::try_from(due.min(u64::from(MAX_STEPS))).unwrap_or(MAX_STEPS)
    }

    /// Forgets the time owed: after a sleep, or a load that took seconds.
    pub fn reset(&mut self) {
        self.owed = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;

    const STEP_NS: u64 = 16_666_667;

    #[test]
    fn a_frame_of_a_sixtieth_of_a_second_is_one_step() {
        let mut clock = TickClock::new();
        let frame = Duration::from_nanos(STEP_NS);
        let steps: u32 = (0..600).map(|_| clock.advance(frame)).sum();
        assert_eq!(steps, 600);
    }

    #[test]
    fn whole_milliseconds_add_up_to_sixty_steps_a_second() {
        let mut clock = TickClock::new();
        // 16, 17, 17 ms: 50 ms every three frames, as a millisecond clock at
        // 60 frames a second gives.
        let frames = [16, 17, 17].map(Duration::from_millis);
        let steps: Vec<u32> = frames
            .iter()
            .cycle()
            .take(60)
            .map(|&frame| clock.advance(frame))
            .collect();
        assert_eq!(steps.iter().sum::<u32>(), 60);
        assert!(steps.iter().all(|&s| s <= 2), "{steps:?}");
        // A 120 Hz display: two frames a step.
        let mut fast = TickClock::new();
        let steps: Vec<u32> = (0..120)
            .map(|_| fast.advance(Duration::from_nanos(STEP_NS / 2 + 1)))
            .collect();
        assert_eq!(steps.iter().sum::<u32>(), 60);
        assert!(steps.chunks(2).all(|pair| pair.iter().sum::<u32>() == 1));
    }

    #[test]
    fn a_late_frame_runs_at_most_four_steps_and_drops_the_rest() {
        let mut clock = TickClock::new();
        assert_eq!(clock.advance(Duration::from_secs(600)), MAX_STEPS);
        // Nothing of the ten minutes is left over.
        assert_eq!(clock.advance(Duration::from_millis(1)), 0);
        assert_eq!(clock.advance(Duration::MAX), MAX_STEPS);
        assert_eq!(clock.advance(Duration::ZERO), 0);
        // Three steps' worth is three steps.
        assert_eq!(clock.advance(Duration::from_nanos(3 * STEP_NS)), 3);
    }

    #[test]
    fn reset_forgets_a_part_step() {
        let mut clock = TickClock::new();
        assert_eq!(clock.advance(Duration::from_millis(10)), 0);
        clock.reset();
        assert_eq!(clock.advance(Duration::from_millis(10)), 0);
        assert_eq!(clock.advance(Duration::from_millis(10)), 1);
    }
}
