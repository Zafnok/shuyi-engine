//! Where a voice is in its source, in fixed point: a sound's rate against the
//! output's, and a playback rate, as an exact step, with linear interpolation
//! between frames.
//!
//! A position counts 1/65536 of a source frame. A step in those units is
//! `source rate * rate / output rate`, whose remainder is carried from frame to
//! frame as a whole fraction of the output rate, so nothing drifts however long
//! a sound plays, and no frame divides anything.

/// A step and how far past the last whole frame a voice is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Position {
    /// 1/65536 of a frame a step moves, rounded down.
    step: u64,
    /// What rounding left of a step, in 1/`denominator` of 1/65536 of a frame.
    remainder: u64,
    /// The output's rate.
    denominator: u64,
    /// How far past the current frame it is, in 1/65536 of a frame.
    fraction: u32,
    /// What the steps have left over so far, under `denominator`.
    carried: u64,
}

impl Position {
    /// The start of a source of `source_rate` played at `rate` (16.16) on an
    /// output of `output_rate`.
    pub(super) fn new(source_rate: u32, rate: u32, output_rate: u32) -> Self {
        let per_second = u64::from(source_rate) * u64::from(rate);
        let denominator = u64::from(output_rate.max(1));
        Self {
            step: per_second / denominator,
            remainder: per_second % denominator,
            denominator,
            fraction: 0,
            carried: 0,
        }
    }

    /// One output frame on: how many whole source frames that passes.
    pub(super) fn advance(&mut self) -> u64 {
        self.carried += self.remainder;
        let mut moved = u64::from(self.fraction) + self.step;
        if self.carried >= self.denominator {
            self.carried -= self.denominator;
            moved += 1;
        }
        self.fraction = u32::try_from(moved & 0xFFFF).unwrap_or(0);
        moved >> 16
    }

    /// How far past the current frame it is, in 1/65536 of a frame.
    pub(super) const fn fraction(&self) -> u32 {
        self.fraction
    }
}

/// `fraction`/65536 of the way from `from` to `to`, rounded to the nearest. A
/// result is never outside the two.
pub(super) fn between(from: i16, to: i16, fraction: u32) -> i32 {
    let distance = i64::from(to) - i64::from(from);
    let part = (distance * i64::from(fraction) + 0x8000) >> 16;
    i32::from(from) + i32::try_from(part).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    /// The output frames a source of `frames` frames plays for: one for each
    /// position before its end.
    fn played(frames: u64, source_rate: u32, rate: u32, output_rate: u32) -> u64 {
        let mut position = Position::new(source_rate, rate, output_rate);
        let (mut at, mut count) = (0, 0);
        while at < frames {
            count += 1;
            at += position.advance();
        }
        count
    }

    #[test]
    fn the_same_rate_moves_a_whole_frame_a_frame() {
        let mut position = Position::new(48_000, 65_536, 48_000);
        for _ in 0..1000 {
            assert_eq!(position.advance(), 1);
            assert_eq!(position.fraction(), 0);
        }
    }

    #[test]
    fn a_length_comes_out_exactly_whatever_the_rates() {
        assert_eq!(played(44_100, 44_100, 65_536, 48_000), 48_000);
        assert_eq!(played(48_000, 48_000, 65_536, 44_100), 44_100);
        assert_eq!(played(22_050, 22_050, 65_536, 48_000), 48_000);
        assert_eq!(played(100, 48_000, 131_072, 48_000), 50);
        assert_eq!(played(101, 48_000, 131_072, 48_000), 51);
        assert_eq!(played(100, 48_000, 32_768, 48_000), 200);
        // An hour of it: a rounded step would be off by frames by now.
        assert_eq!(played(44_100 * 3600, 44_100, 65_536, 48_000), 48_000 * 3600);
    }

    #[test]
    fn between_is_exact_at_its_ends_and_rounds_to_the_nearest() {
        assert_eq!(between(-7, 9, 0), -7);
        assert_eq!(between(0, 100, 0x8000), 50);
        assert_eq!(between(0, 1, 0x8000), 1);
        assert_eq!(between(0, 1, 0x7FFF), 0);
        assert_eq!(between(100, 0, 0xC000), 25);
        assert_eq!(
            between(i16::MIN, i16::MAX, 0x8000),
            0,
            "half of the whole range"
        );
        assert_eq!(between(i16::MIN, i16::MAX, 0xFFFF), 32_766);
        assert_eq!(between(i16::MAX, i16::MIN, 0xFFFF), -32_767);
    }

    proptest! {
        /// However the rates go, a position's whole frames over many steps are
        /// `steps * source rate * rate / output rate`, rounded down.
        #[test]
        fn positions_are_exact(
            source_rate in 1u32..=192_000,
            rate in 256u32..=1 << 24,
            output_rate in 1u32..=192_000,
            steps in 1u64..2000,
        ) {
            let mut position = Position::new(source_rate, rate, output_rate);
            let moved: u64 = (0..steps).map(|_| position.advance()).sum();
            let exact = u128::from(steps) * u128::from(source_rate) * u128::from(rate)
                / u128::from(output_rate) / 65_536;
            prop_assert_eq!(u128::from(moved), exact);
        }

        /// A result stays between its two ends.
        #[test]
        fn between_stays_between(from: i16, to: i16, fraction in 0u32..65_536) {
            let result = between(from, to, fraction);
            prop_assert!(result >= i32::from(from.min(to)));
            prop_assert!(result <= i32::from(from.max(to)));
        }
    }
}
