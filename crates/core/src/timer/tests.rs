//! The conversions at their edges, each timer at its edges, values out of
//! range from a file, and what any split of an advance leaves the same.

use alloc::vec::Vec;

use proptest::prelude::*;

use super::*;
use crate::hash::{StateHash, state_hash};

#[test]
fn steps_from_ms_rounds_up_to_whole_steps() {
    for (ms, steps) in [
        (0, 0),
        (1, 1),
        (16, 1),
        (17, 2),
        (50, 3),
        (250, 15),
        (1000, 60),
        (u32::MAX, 257_698_038),
    ] {
        assert_eq!(steps_from_ms(ms), steps, "{ms} ms");
    }
}

#[test]
fn ms_from_steps_rounds_down_and_never_drifts() {
    for (steps, ms) in [
        (0, 0),
        (1, 16),
        (2, 33),
        (3, 50),
        (60, 1000),
        (u32::MAX, 71_582_788_250),
    ] {
        assert_eq!(ms_from_steps(steps), ms, "{steps} steps");
    }
}

#[test]
fn a_countdown_fires_once_on_the_advance_that_reaches_zero() {
    let mut exactly = Countdown::new(3);
    assert!(!exactly.advance(0));
    assert!(!exactly.advance(2));
    assert_eq!(exactly.remaining(), 1);
    assert!(!exactly.ready());
    assert!(exactly.advance(1));
    assert!(exactly.ready());
    // Fired: it never fires again by itself.
    assert!(!exactly.advance(1));
    assert!(!exactly.advance(0));

    let mut past = Countdown::new(3);
    assert!(past.advance(100));
    assert_eq!(past.remaining(), 0);
}

#[test]
fn a_restarted_countdown_fires_again_after_its_length() {
    let mut timer = Countdown::new(2);
    assert!(timer.advance(2));
    timer.restart();
    assert_eq!(timer.remaining(), 2);
    assert!(!timer.advance(1));
    assert!(timer.advance(1));
}

#[test]
fn a_ready_countdown_needs_no_advance_and_a_length_of_zero_never_fires() {
    let mut ready = Countdown::new_ready(5);
    assert!(ready.ready());
    assert_eq!(ready.length(), 5);
    assert!(!ready.advance(1));
    ready.restart();
    assert_eq!(ready.remaining(), 5);

    for mut zero in [Countdown::new(0), Countdown::default()] {
        assert!(zero.ready());
        assert_eq!(zero.length(), 0);
        assert!(!zero.advance(1));
    }
}

#[test]
fn elapsed_fills_as_what_is_left_empties() {
    let mut timer = Countdown::new(5);
    assert_eq!(timer.elapsed(), 0);
    timer.advance(2);
    assert_eq!(timer.elapsed(), 2);
    assert_eq!(timer.length(), 5);
    timer.advance(9);
    assert_eq!(timer.elapsed(), 5);
    assert_eq!(Countdown::new_ready(5).elapsed(), 5);
}

#[test]
fn every_fires_by_the_periods_an_advance_crosses() {
    let mut timer = Every::new(4);
    assert_eq!(timer.period(), 4);
    assert_eq!(timer.until_next(), 4);
    assert_eq!(timer.advance(3), 0);
    assert_eq!(timer.until_next(), 1);
    assert_eq!(timer.advance(1), 1);
    assert_eq!(timer.until_next(), 4);
    assert_eq!(timer.advance(9), 2);
    assert_eq!(timer.until_next(), 3);
    timer.reset();
    assert_eq!(timer.until_next(), 4);
    assert_eq!(timer.advance(0), 0);
}

#[test]
fn every_saturates_its_count() {
    let mut timer: Every = ron::from_str(&alloc::format!("(period: 1, since: {})", u32::MAX))
        .expect("a timer's fields");
    assert_eq!(timer.advance(u32::MAX), u32::MAX);
    assert_eq!(timer.until_next(), 1);
    // Fires as often as the period says, with nothing lost below the top.
    let mut wide = Every::new(u32::MAX);
    assert_eq!(wide.advance(u32::MAX), 1);
    assert_eq!(wide.advance(u32::MAX - 1), 0);
    assert_eq!(wide.until_next(), 1);
}

#[test]
fn a_period_of_zero_acts_as_one_from_new_and_from_a_file() {
    let read: Every = ron::from_str("(period: 0, since: 0)").expect("a timer's fields");
    for mut timer in [Every::new(0), read] {
        assert_eq!(timer.period(), 1);
        assert_eq!(timer.until_next(), 1);
        assert_eq!(timer.advance(1), 1);
        assert_eq!(timer.advance(5), 5);
        assert_eq!(timer.until_next(), 1);
    }
    assert_eq!(Every::new(0), Every::new(1));
}

#[test]
fn values_out_of_range_from_a_file_behave_by_the_formulas() {
    let mut late: Countdown = ron::from_str("(length: 2, left: 9)").expect("a timer's fields");
    assert_eq!(late.elapsed(), 0);
    assert_eq!(late.remaining(), 9);
    assert!(!late.ready());
    assert!(!late.advance(8));
    assert!(late.advance(1));
    assert_eq!(late.elapsed(), 2);

    let mut past: Every = ron::from_str("(period: 3, since: 10)").expect("a timer's fields");
    assert_eq!(past.until_next(), 1);
    assert_eq!(past.period(), 3);
    assert_eq!(past.advance(0), 3);
    assert_eq!(past.until_next(), 2);

    let at: Every = ron::from_str("(period: 3, since: 3)").expect("a timer's fields");
    assert_eq!(at.until_next(), 1);
}

#[test]
fn a_timers_hash_never_changes_unnoticed() {
    let mut countdown = Countdown::new(5);
    countdown.advance(2);
    let mut every = Every::new(8);
    every.advance(3);
    assert_eq!(state_hash(&countdown), Ok(StateHash(0x74b4_6df9_a98b_d500)));
    assert_eq!(state_hash(&every), Ok(StateHash(0xac98_aed4_9f88_1ad9)));
}

/// `n` cut into the parts `cuts` picks; empty parts are kept, since an
/// advance by 0 is one too.
fn parts(n: u32, cuts: &[u32]) -> Vec<u32> {
    let mut parts = Vec::new();
    let mut left = n;
    for &cut in cuts {
        let part = cut.min(left);
        parts.push(part);
        left -= part;
    }
    parts.push(left);
    parts
}

proptest! {
    #[test]
    fn steps_from_ms_undoes_ms_from_steps(steps in 0..=257_698_037_u32) {
        prop_assert_eq!(
            u64::from(steps_from_ms(u32::try_from(ms_from_steps(steps)).unwrap())),
            u64::from(steps),
        );
    }

    #[test]
    fn a_split_countdown_ends_as_one_advance_and_fires_once(
        length in 0..1000_u32,
        before in 0..1000_u32,
        n in 0..2000_u32,
        cuts in proptest::collection::vec(0..2000_u32, 0..8),
    ) {
        let mut start = Countdown::new(length);
        start.advance(before);

        let mut whole = start;
        let fired = whole.advance(n);
        let mut split = start;
        let fires = parts(n, &cuts)
            .into_iter()
            .filter(|&part| split.advance(part))
            .count();

        prop_assert_eq!(split, whole);
        prop_assert_eq!(fires, usize::from(fired));
        prop_assert_eq!(fired, !start.ready() && n >= start.remaining());
    }

    #[test]
    fn a_split_every_ends_as_one_advance_and_fires_as_often(
        period in 1..100_u32,
        before in 0..1000_u32,
        n in 0..2000_u32,
        cuts in proptest::collection::vec(0..2000_u32, 0..8),
    ) {
        let mut start = Every::new(period);
        start.advance(before);
        let since = period - start.until_next();

        let mut whole = start;
        let fired = whole.advance(n);
        let mut split = start;
        let fires: u32 = parts(n, &cuts)
            .into_iter()
            .map(|part| split.advance(part))
            .sum();

        prop_assert_eq!(split, whole);
        prop_assert_eq!(fires, fired);
        prop_assert_eq!(fired, (since + n) / period);
    }

    #[test]
    fn timers_read_back_equal_from_ron(
        length in any::<u32>(),
        gone in any::<u32>(),
        period in any::<u32>(),
        moved in any::<u32>(),
    ) {
        let mut countdown = Countdown::new(length);
        countdown.advance(gone);
        let text = ron::to_string(&countdown).unwrap();
        prop_assert_eq!(ron::from_str::<Countdown>(&text).unwrap(), countdown);

        let mut every = Every::new(period);
        every.advance(moved);
        let text = ron::to_string(&every).unwrap();
        prop_assert_eq!(ron::from_str::<Every>(&text).unwrap(), every);
    }
}
