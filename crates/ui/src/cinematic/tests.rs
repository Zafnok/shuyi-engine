//! Tests of the timeline, easing and the pan (moved from the game's
//! cinematic tests, ticket 1113). The player plays a toy cinematic in the
//! integration tests (`tests/it/cinematic.rs`).

use proptest::prelude::*;

use super::*;

fn test_timeline() -> Timeline {
    Timeline::new(vec![0, 6000], 10_000)
}

fn at(index: usize, progress: f32) -> ShotAt {
    ShotAt { index, progress }
}

fn near(a: (f32, f32), b: (f32, f32)) -> bool {
    (a.0 - b.0).abs() < 1e-3 && (a.1 - b.1).abs() < 1e-3
}

#[test]
fn a_time_falls_in_the_shot_that_started_last() {
    let t = test_timeline();
    assert_eq!(t.length_ms(), 10_000);
    // A shot's start, middle and end.
    assert_eq!(t.at_ms(0), at(0, 0.0));
    assert_eq!(t.at_ms(3000), at(0, 0.5));
    assert_eq!(t.at_ms(1500), at(0, 0.25));
    let end = t.at_ms(5999);
    assert_eq!(end.index, 0);
    assert!(end.progress > 0.999 && end.progress < 1.0, "{end:?}");
    // The next shot starts on the millisecond; the last ends at the length.
    assert_eq!(t.at_ms(6000), at(1, 0.0));
    assert_eq!(t.at_ms(8000), at(1, 0.5));
    assert_eq!(t.at_ms(9000), at(1, 0.75));
    assert_eq!(t.at_ms(9999).index, 1);
    // The wrap: the length is the start again, and so on round.
    assert_eq!(t.at_ms(10_000), at(0, 0.0));
    assert_eq!(t.at_ms(13_000), at(0, 0.5));
    assert_eq!(t.at_ms(28_000), at(1, 0.5));
    assert_eq!(t.wrapped_ms(28_000), 8000);
    assert_eq!(t.wrapped_ms(9999), 9999);
    assert_eq!(t.wrapped_ms(u64::MAX), 1615);
    // Three shots: the middle one ends where the third starts.
    let three = Timeline::new(vec![0, 1000, 3000], 4000);
    assert_eq!(three.at_ms(999).index, 0);
    assert_eq!(three.at_ms(1000), at(1, 0.0));
    assert_eq!(three.at_ms(2500), at(1, 0.75));
    assert_eq!(three.at_ms(3500), at(2, 0.5));
    // The same, as milliseconds into the shot.
    assert_eq!(three.into_shot(0), (0, 0));
    assert_eq!(three.into_shot(999), (0, 999));
    assert_eq!(three.into_shot(1000), (1, 0));
    assert_eq!(three.into_shot(2500), (1, 1500));
    assert_eq!(three.into_shot(3999), (2, 999));
    assert_eq!(three.into_shot(4000), (0, 0));
    assert_eq!(three.into_shot(4000 * 7 + 3001), (2, 1));
    assert_eq!(t.into_shot(28_000), (1, 2000));
    // Before a first shot that doesn't start at 0: at its very start.
    let late = Timeline::new(vec![500, 1000], 2000);
    assert_eq!(late.into_shot(200), (0, 0));
    assert_eq!(late.into_shot(700), (0, 200));
    assert_eq!(late.at_ms(200), at(0, 0.0));
}

#[test]
fn the_last_shot_runs_from_its_start_to_the_end() {
    let t = test_timeline();
    assert_eq!(t.last_start_ms(), 6000);
    assert!(!t.in_last_shot(0));
    assert!(!t.in_last_shot(5999));
    assert!(t.in_last_shot(6000));
    assert!(t.in_last_shot(9999));
    // Wrapped: the length is the first shot again.
    assert!(!t.in_last_shot(10_000));
    assert!(t.in_last_shot(16_000));
    assert_eq!(
        Timeline::new(vec![0, 1000, 3000], 4000).last_start_ms(),
        3000
    );
    // One shot is the last one; no shots start at 0.
    assert!(Timeline::new(vec![0], 500).in_last_shot(0));
    assert_eq!(Timeline::new(vec![], 500).last_start_ms(), 0);
}

#[test]
fn seconds_are_whole_milliseconds_from_zero() {
    let t = test_timeline();
    assert_eq!(t.at(0.0), at(0, 0.0));
    assert_eq!(t.at(3.0), at(0, 0.5));
    assert_eq!(t.at(6.0), at(1, 0.0));
    assert_eq!(t.at(10.0), at(0, 0.0));
    assert_eq!(t.at(18.0), at(1, 0.5));
    assert_eq!(millis(1.2344), 1234);
    assert_eq!(millis(1.2346), 1235);
    assert_eq!(millis(0.0004), 0);
    assert_eq!(millis(0.0006), 1);
    for bad in [-0.5, -1e9, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        assert_eq!(millis(bad), 0, "{bad}");
        assert_eq!(t.at(bad), at(0, 0.0), "{bad}");
    }
    assert_eq!(millis(f32::MAX), u64::MAX);
}

#[test]
fn a_timeline_never_divides_by_nothing() {
    // No length: one millisecond, always its start.
    let none = Timeline::new(vec![0], 0);
    assert_eq!(none.length_ms(), 1);
    assert_eq!(none.at_ms(7), at(0, 0.0));
    // No shots: the whole length is "shot 0".
    let empty = Timeline::new(vec![], 1000);
    assert_eq!(empty.at_ms(250), at(0, 0.25));
    // A first shot that starts late: the time before it is its start.
    let late = Timeline::new(vec![400, 800], 1000);
    assert_eq!(late.at_ms(100), at(0, 0.0));
    assert_eq!(late.at_ms(600), at(0, 0.5));
    assert_eq!(late.at_ms(900), at(1, 0.5));
    // Two shots at once: the later one shows.
    let twice = Timeline::new(vec![0, 500, 500], 1000);
    assert_eq!(twice.at_ms(500), at(2, 0.0));
    assert_eq!(twice.at_ms(750), at(2, 0.5));
    // A shot past the end never shows; the one before it runs towards it.
    let past = Timeline::new(vec![0, 2000], 1000);
    assert_eq!(past.at_ms(500), at(0, 0.25));
    assert_eq!(past.at_ms(1500), at(0, 0.25));
}

#[test]
fn easing_is_slow_at_both_ends() {
    let eased = |p: f32, expected: f32| {
        let got = ease_in_out(p);
        assert!((got - expected).abs() < 1e-6, "{p}: {got}");
    };
    eased(0.0, 0.0);
    eased(0.5, 0.5);
    eased(1.0, 1.0);
    eased(0.25, 0.15625);
    eased(0.75, 0.84375);
    eased(-3.0, 0.0);
    eased(7.0, 1.0);
    eased(f32::NAN, 0.0);
    eased(f32::INFINITY, 0.0);
}

proptest! {
    #[test]
    fn any_time_is_in_a_shot_and_the_same_one_a_length_later(
        gaps in prop::collection::vec(1u32..5000, 0..6),
        tail in 1u32..5000,
        t_ms in prop_oneof![0u64..100_000, any::<u64>()],
    ) {
        let mut starts = vec![0];
        for gap in gaps {
            starts.push(starts.last().copied().unwrap_or(0) + gap);
        }
        let length = starts.last().copied().unwrap_or(0) + tail;
        let timeline = Timeline::new(starts.clone(), length);
        let shot = timeline.at_ms(t_ms);
        prop_assert!(shot.index < starts.len());
        prop_assert!((0.0..=1.0).contains(&shot.progress), "{shot:?}");
        // The shot is the one whose span holds the wrapped time.
        let t = timeline.wrapped_ms(t_ms);
        prop_assert!(t < length);
        prop_assert!(starts[shot.index] <= t);
        prop_assert!(starts.get(shot.index + 1).is_none_or(|&next| t < next));
        if let Some(later) = t_ms.checked_add(u64::from(length)) {
            prop_assert_eq!(timeline.at_ms(later), shot);
        }
    }

    #[test]
    fn any_seconds_are_in_a_shot(t in any::<f32>()) {
        let shot = test_timeline().at(t);
        prop_assert!(shot.index < 2);
        prop_assert!((0.0..=1.0).contains(&shot.progress), "{shot:?}");
    }

    #[test]
    fn easing_stays_between_its_ends_and_never_goes_back(a in 0f32..=1.0, b in 0f32..=1.0) {
        let (low, high) = (a.min(b), a.max(b));
        prop_assert!((0.0..=1.0).contains(&ease_in_out(a)));
        prop_assert!(ease_in_out(low) <= ease_in_out(high));
        // The same shape from both ends.
        prop_assert!((ease_in_out(a) + ease_in_out(1.0 - a) - 1.0).abs() < 1e-5);
    }
}

#[test]
fn a_shot_lasts_until_the_next_starts_the_last_until_the_end() {
    let timeline = Timeline::new(vec![0, 6000, 10_000, 16_000], 20_000);
    let lengths: Vec<u32> = (0..6).map(|i| timeline.shot_ms(i)).collect();
    assert_eq!(lengths, [6000, 4000, 6000, 4000, 0, 0]);
    assert_eq!(Timeline::new(vec![0, 500], 400).shot_ms(1), 0);
}

#[test]
fn a_moment_is_its_shot_how_far_into_it_and_the_wrapped_time() {
    let three = Timeline::new(vec![0, 1000, 3000], 4000);
    assert_eq!(
        three.moment(4000 * 2 + 2500),
        Moment {
            index: 1,
            progress: 0.75,
            into_ms: 1500,
            shot_ms: 2000,
            t_ms: 2500,
        }
    );
    assert_eq!(
        three.moment(3999),
        Moment {
            index: 2,
            progress: 0.999,
            into_ms: 999,
            shot_ms: 1000,
            t_ms: 3999,
        }
    );
    // Before a first shot that starts late: its very start.
    let late = Timeline::new(vec![500, 1000], 2000).moment(200);
    assert_eq!((late.index, late.into_ms, late.t_ms), (0, 0, 200));
    assert_eq!((late.progress, late.shot_ms), (0.0, 500));
}

#[test]
fn the_timeline_of_a_cinematic_is_its_shots_starts_and_its_length() {
    use shuyi_assets::cinematic::ShotDef;
    let shot = |at_ms| ShotDef { at_ms, shot: () };
    let def = CinematicDef {
        id: "t".to_owned(),
        music: None,
        length_ms: 20_000,
        shots: vec![shot(0), shot(6000), shot(10_000)],
    };
    assert_eq!(
        Timeline::of(&def),
        Timeline::new(vec![0, 6000, 10_000], 20_000)
    );
}

#[test]
fn a_pan_eases_from_its_start_to_its_end() {
    let pan = Pan {
        from: (100.0, 40.0),
        to: (200.0, 20.0),
        zoom: 2,
    };
    assert!(near(pan.centre(0.0), (100.0, 40.0)));
    assert!(near(pan.centre(0.5), (150.0, 30.0)));
    assert!(near(pan.centre(0.25), (115.625, 36.875)));
    assert!(near(pan.centre(1.0), (200.0, 20.0)));
    assert!(near(pan.centre(f32::NAN), (100.0, 40.0)));
    // A screen of 10 × 4 cells of 8 × 16 pixels at 2× shows 40 × 32 of
    // the picture's pixels, around the centre.
    let console = ConsoleSpec::text(10, 4);
    let screen = Rect::new(0, 0, 10, 4);
    let origin = |progress| pan.origin_px(console, (400, 100), screen, progress);
    assert!(near(origin(0.0), (80.0, 24.0)));
    assert!(near(origin(0.5), (130.0, 14.0)));
    assert!(near(origin(1.0), (180.0, 4.0)));
    // It never leaves the picture: near an edge it stops there.
    let edge = Pan {
        to: (0.0, 0.0),
        ..pan
    };
    assert!(near(
        edge.origin_px(console, (400, 100), screen, 1.0),
        (0.0, 0.0)
    ));
    // No screen: no window to fill.
    let none = pan.origin_px(console, (400, 100), Rect::new(0, 0, -3, 0), 0.0);
    assert!(near(none, (100.0, 40.0)));
}
