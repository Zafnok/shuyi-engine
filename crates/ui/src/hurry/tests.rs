//! The hurry clock's hold, speed and skip.

use super::*;

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-5
}

#[test]
fn unheld_a_frame_passes_as_it_is() {
    let mut clock = HurryClock::default();
    for _ in 0..100 {
        assert!(close(clock.tick(0.016, false), 0.016));
    }
    assert!(!clock.is_skipped());
}

#[test]
fn held_past_the_delay_a_frame_passes_factor_times_as_fast() {
    for (delay, factor) in [(HURRY_DELAY, HURRY_FACTOR), (0.5, 2.0), (0.0, 3.0)] {
        let mut clock = HurryClock::new(delay, factor);
        // Long past it: the whole frame fast.
        clock.hold(true, delay + 1.0);
        assert!(
            close(clock.tick(0.1, true), 0.1 * factor),
            "{delay} {factor}"
        );
    }
}

#[test]
fn the_part_of_a_frame_that_crosses_the_delay_goes_fast() {
    for factor in [HURRY_FACTOR, 2.0, 10.0] {
        let mut clock = HurryClock::new(0.3, factor);
        // 0.25 s held: under the delay.
        assert!(close(clock.tick(0.25, true), 0.25));
        // This frame crosses it 0.05 s in: 0.05 as it is, 0.05 fast.
        assert!(
            close(clock.tick(0.1, true), 0.05 + 0.05 * factor),
            "{factor}"
        );
        // Then every frame fast.
        assert!(close(clock.tick(0.1, true), 0.1 * factor));
    }
}

#[test]
fn letting_go_or_releasing_starts_the_hold_over() {
    let mut clock = HurryClock::default();
    clock.tick(1.0, true);
    assert!(close(clock.tick(0.1, false), 0.1));
    assert!(close(clock.tick(0.1, true), 0.1), "the hold counts again");
    clock.tick(1.0, true);
    clock.release();
    assert!(close(clock.passing(0.1), 0.1));
}

#[test]
fn a_skip_lasts_until_the_clock_is_reset() {
    let mut clock = HurryClock::default();
    clock.tick(1.0, true);
    clock.skip();
    assert!(clock.is_skipped());
    clock.tick(0.1, false);
    assert!(clock.is_skipped());
    clock.tick(1.0, true);
    clock.reset();
    assert!(!clock.is_skipped());
    assert!(close(clock.passing(0.1), 0.1), "nothing held");
    assert_eq!((clock.delay, clock.factor), (HURRY_DELAY, HURRY_FACTOR));
}

#[test]
fn nonsense_frame_times_pass_no_time_and_hold_nothing() {
    let mut clock = HurryClock::default();
    for dt in [f32::NAN, -1.0, f32::INFINITY, f32::NEG_INFINITY] {
        assert!(close(clock.tick(dt, true), 0.0), "{dt}");
    }
    assert!(close(clock.tick(0.1, true), 0.1), "still under the delay");
}
