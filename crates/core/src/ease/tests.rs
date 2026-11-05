//! Every curve at every `p` against its formula in `f64` (easings.net's,
//! written out), the tables against theirs, the golden table, `Lerp` at and
//! past its ends, `Tween` over its time, and RON.

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::f64::consts::PI;
use core::fmt::Write as _;

use proptest::prelude::*;

use super::tables::{EXP2, QUARTER_SINE};
use super::*;

const FAMILIES: [Family; 10] = [
    Family::Sine,
    Family::Quad,
    Family::Cubic,
    Family::Quart,
    Family::Quint,
    Family::Expo,
    Family::Circ,
    Family::Back,
    Family::Elastic,
    Family::Bounce,
];

/// Every curve the criteria name: `Linear`, `Smooth`, three `Steps` and
/// each family in all four ways.
fn curves() -> Vec<Ease> {
    let mut curves = alloc::vec![
        Ease::Linear,
        Ease::Smooth,
        Ease::Steps(1),
        Ease::Steps(3),
        Ease::Steps(16),
    ];
    for family in FAMILIES {
        curves.extend([
            Ease::In(family),
            Ease::Out(family),
            Ease::InOut(family),
            Ease::OutIn(family),
        ]);
    }
    curves
}

// easings.net's formulas, as written there; `x <= 0.0` and `x >= 1.0` are
// its `x === 0` and `x === 1`.

fn bounce_out(x: f64) -> f64 {
    let (n1, d1) = (7.5625, 2.75);
    if x < 1.0 / d1 {
        n1 * x * x
    } else if x < 2.0 / d1 {
        let x = x - 1.5 / d1;
        n1 * x * x + 0.75
    } else if x < 2.5 / d1 {
        let x = x - 2.25 / d1;
        n1 * x * x + 0.9375
    } else {
        let x = x - 2.625 / d1;
        n1 * x * x + 0.984_375
    }
}

fn ease_in(family: Family, x: f64) -> f64 {
    let (c1, c4) = (1.701_58, 2.0 * PI / 3.0);
    match family {
        Family::Expo | Family::Elastic if x <= 0.0 => 0.0,
        Family::Sine => 1.0 - (x * PI / 2.0).cos(),
        Family::Quad => x.powi(2),
        Family::Cubic => x.powi(3),
        Family::Quart => x.powi(4),
        Family::Quint => x.powi(5),
        Family::Expo => 2_f64.powf(10.0 * x - 10.0),
        Family::Circ => 1.0 - (1.0 - x.powi(2)).sqrt(),
        Family::Back => (c1 + 1.0) * x.powi(3) - c1 * x.powi(2),
        Family::Elastic if x >= 1.0 => 1.0,
        Family::Elastic => -(2_f64.powf(10.0 * x - 10.0)) * ((x * 10.0 - 10.75) * c4).sin(),
        Family::Bounce => 1.0 - bounce_out(1.0 - x),
    }
}

fn ease_out(family: Family, x: f64) -> f64 {
    let (c1, c4) = (1.701_58, 2.0 * PI / 3.0);
    let c3 = c1 + 1.0;
    match family {
        Family::Elastic if x <= 0.0 => 0.0,
        Family::Expo | Family::Elastic if x >= 1.0 => 1.0,
        Family::Sine => (x * PI / 2.0).sin(),
        Family::Quad => 1.0 - (1.0 - x).powi(2),
        Family::Cubic => 1.0 - (1.0 - x).powi(3),
        Family::Quart => 1.0 - (1.0 - x).powi(4),
        Family::Quint => 1.0 - (1.0 - x).powi(5),
        Family::Expo => 1.0 - 2_f64.powf(-10.0 * x),
        Family::Circ => (1.0 - (x - 1.0).powi(2)).sqrt(),
        Family::Back => 1.0 + c3 * (x - 1.0).powi(3) + c1 * (x - 1.0).powi(2),
        Family::Elastic => 2_f64.powf(-10.0 * x) * ((x * 10.0 - 0.75) * c4).sin() + 1.0,
        Family::Bounce => bounce_out(x),
    }
}

fn ease_in_out(family: Family, x: f64) -> f64 {
    let (c1, c5) = (1.701_58, 2.0 * PI / 4.5);
    let c2 = c1 * 1.525;
    let low = x < 0.5;
    match family {
        Family::Expo | Family::Elastic if x <= 0.0 => 0.0,
        Family::Expo | Family::Elastic if x >= 1.0 => 1.0,
        Family::Sine => -((PI * x).cos() - 1.0) / 2.0,
        Family::Quad if low => 2.0 * x * x,
        Family::Quad => 1.0 - (-2.0 * x + 2.0).powi(2) / 2.0,
        Family::Cubic if low => 4.0 * x.powi(3),
        Family::Cubic => 1.0 - (-2.0 * x + 2.0).powi(3) / 2.0,
        Family::Quart if low => 8.0 * x.powi(4),
        Family::Quart => 1.0 - (-2.0 * x + 2.0).powi(4) / 2.0,
        Family::Quint if low => 16.0 * x.powi(5),
        Family::Quint => 1.0 - (-2.0 * x + 2.0).powi(5) / 2.0,
        Family::Expo if low => 2_f64.powf(20.0 * x - 10.0) / 2.0,
        Family::Expo => (2.0 - 2_f64.powf(-20.0 * x + 10.0)) / 2.0,
        Family::Circ if low => (1.0 - (1.0 - (2.0 * x).powi(2)).sqrt()) / 2.0,
        Family::Circ => f64::midpoint((1.0 - (-2.0 * x + 2.0).powi(2)).sqrt(), 1.0),
        Family::Back if low => (2.0 * x).powi(2) * ((c2 + 1.0) * 2.0 * x - c2) / 2.0,
        Family::Back => f64::midpoint(
            (2.0 * x - 2.0).powi(2) * ((c2 + 1.0) * (x * 2.0 - 2.0) + c2),
            2.0,
        ),
        Family::Elastic if low => {
            -(2_f64.powf(20.0 * x - 10.0) * ((20.0 * x - 11.125) * c5).sin()) / 2.0
        }
        Family::Elastic => {
            2_f64.powf(-20.0 * x + 10.0) * ((20.0 * x - 11.125) * c5).sin() / 2.0 + 1.0
        }
        Family::Bounce if low => (1.0 - bounce_out(1.0 - 2.0 * x)) / 2.0,
        Family::Bounce => f64::midpoint(1.0, bounce_out(2.0 * x - 1.0)),
    }
}

/// Godot's `EASE_OUT_IN`: `Out` over the first half, `In` over the second.
fn ease_out_in(family: Family, x: f64) -> f64 {
    if x < 0.5 {
        ease_out(family, 2.0 * x) / 2.0
    } else {
        f64::midpoint(1.0, ease_in(family, 2.0 * x - 1.0))
    }
}

/// The curve's formula at `x` (0 to 1).
fn formula(ease: Ease, x: f64) -> f64 {
    match ease {
        Ease::Linear => x,
        Ease::Smooth => 3.0 * x * x - 2.0 * x.powi(3),
        Ease::Steps(n) => {
            let n = f64::from(n.max(1));
            if x >= 1.0 { 1.0 } else { (x * n).floor() / n }
        }
        Ease::In(family) => ease_in(family, x),
        Ease::Out(family) => ease_out(family, x),
        Ease::InOut(family) => ease_in_out(family, x),
        Ease::OutIn(family) => ease_out_in(family, x),
    }
}

/// How far each curve may be from its formula, in 1/65536.
fn bound(ease: Ease) -> f64 {
    match ease {
        Ease::In(f) | Ease::Out(f) | Ease::InOut(f) | Ease::OutIn(f)
            if !matches!(
                f,
                Family::Quad | Family::Cubic | Family::Quart | Family::Quint
            ) =>
        {
            2.0
        }
        _ => 1.0,
    }
}

#[test]
fn every_curve_is_its_formula_at_every_p() {
    for ease in curves() {
        let (mut max, mut at) = (0.0_f64, 0);
        for p in 0..=ONE {
            let want = formula(ease, f64::from(p) / f64::from(ONE)) * f64::from(ONE);
            let off = (f64::from(ease.ease(p)) - want).abs();
            if off > max {
                (max, at) = (off, p);
            }
        }
        assert!(
            max <= bound(ease),
            "{ease:?} is {max} from its formula at {at}"
        );
    }
}

#[test]
fn every_curve_is_exactly_0_and_one_at_its_ends_and_holds_them_past() {
    for ease in curves().into_iter().chain([Ease::Steps(0)]) {
        assert_eq!(ease.ease(0), 0, "{ease:?}");
        assert_eq!(ease.ease(ONE), ONE, "{ease:?}");
        for below in [-1, -ONE, i32::MIN] {
            assert_eq!(ease.ease(below), 0, "{ease:?} at {below}");
        }
        for above in [ONE + 1, 2 * ONE, i32::MAX] {
            assert_eq!(ease.ease(above), ONE, "{ease:?} at {above}");
        }
    }
}

#[test]
fn each_curve_at_a_half() {
    let half = ONE / 2;
    for (ease, want) in [
        (Ease::Linear, half),
        (Ease::Smooth, half),
        (Ease::Steps(1), 0),
        (Ease::Steps(2), half),
        (Ease::In(Family::Quad), ONE / 4),
        (Ease::Out(Family::Quad), 3 * ONE / 4),
        (Ease::In(Family::Cubic), ONE / 8),
        (Ease::Out(Family::Cubic), 7 * ONE / 8),
        (Ease::In(Family::Expo), 2048),
        (Ease::Out(Family::Expo), ONE - 2048),
    ] {
        assert_eq!(ease.ease(half), want, "{ease:?}");
    }
    // In and out, and out and in, pass the middle.
    for family in FAMILIES {
        assert_eq!(Ease::InOut(family).ease(half), half, "{family:?}");
        assert_eq!(Ease::OutIn(family).ease(half), half, "{family:?}");
    }
}

#[test]
fn out_is_in_backwards_for_every_family() {
    for family in FAMILIES {
        for p in (0..=ONE).step_by(97) {
            assert_eq!(
                Ease::Out(family).ease(p),
                ONE - Ease::In(family).ease(ONE - p),
                "{family:?} at {p}"
            );
        }
    }
}

#[test]
fn steps_jump_at_the_end_of_each_step() {
    // The first p at or past k thirds.
    let third = |k: i32| (k * ONE + 2) / 3;
    let steps = Ease::Steps(3);
    assert_eq!(steps.ease(third(1) - 1), 0);
    assert_eq!(steps.ease(third(1)), 21_845);
    assert_eq!(steps.ease(third(2) - 1), 21_845);
    assert_eq!(steps.ease(third(2)), 43_691);
    assert_eq!(steps.ease(ONE - 1), 43_691);
    assert_eq!(steps.ease(ONE), ONE);
    // Steps(0) holds the start until the end, as Steps(1).
    for p in [0, 1, ONE / 2, ONE - 1, ONE] {
        assert_eq!(Ease::Steps(0).ease(p), Ease::Steps(1).ease(p), "{p}");
    }
    assert_eq!(Ease::Steps(0).ease(ONE - 1), 0);
}

#[test]
fn back_and_elastic_overshoot() {
    assert!(Ease::In(Family::Back).ease(ONE / 4) < 0);
    assert!(Ease::Out(Family::Back).ease(3 * ONE / 4) > ONE);
    let elastic: Vec<i32> = (0..=ONE)
        .step_by(64)
        .map(|p| Ease::Out(Family::Elastic).ease(p))
        .collect();
    assert!(elastic.iter().any(|&v| v > ONE));
    assert!(elastic.iter().any(|&v| v < ONE && v > ONE / 2));
}

/// A table as `tables.rs` writes it.
fn table_text(name: &str, doc: &str, values: &[i64]) -> String {
    let mut text = format!("/// {doc}\npub(super) const {name}: [i64; 257] = [\n");
    for row in values.chunks(8) {
        let row: Vec<String> = row.iter().map(|v| format!("{},", grouped(*v))).collect();
        let _ = writeln!(text, "    {}", row.join(" "));
    }
    text.push_str("];\n");
    text
}

/// `v` with `_` between groups of three digits.
fn grouped(v: i64) -> String {
    let digits = v.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push('_');
        }
        out.push(c);
    }
    out
}

#[test]
fn the_tables_are_their_formulas() {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "every entry is at most 131 072"
    )]
    let rebuild = |f: fn(f64) -> f64| -> Vec<i64> {
        (0..=256)
            .map(|i| (f(f64::from(i) / 256.0) * f64::from(ONE)).round() as i64)
            .collect()
    };
    let sine = rebuild(|x| (x * PI / 2.0).sin());
    let exp2 = rebuild(|x| 2_f64.powf(x));
    assert!(
        sine == QUARTER_SINE && exp2 == EXP2,
        "a table differs; paste these into tables.rs:\n\n{}\n{}",
        table_text(
            "QUARTER_SINE",
            "`sin(i / 256 * PI / 2)` for `i` from 0 to 256: a quarter of a turn.",
            &sine
        ),
        table_text("EXP2", "`2^(i / 256)` for `i` from 0 to 256.", &exp2),
    );
}

#[test]
fn every_curve_at_65_points() {
    let mut text = String::new();
    for ease in curves() {
        let values: Vec<String> = (0..=64).map(|k| ease.ease(k * 1024).to_string()).collect();
        let _ = writeln!(text, "{ease:?}: {}", values.join(" "));
    }
    insta::assert_snapshot!(text);
}

#[test]
fn every_ease_reads_back_from_ron() {
    for ease in curves().into_iter().chain([Ease::Steps(0)]) {
        let text = ron::to_string(&ease).expect("an ease writes");
        assert_eq!(ron::from_str::<Ease>(&text).expect("an ease reads"), ease);
    }
    for (text, ease) in [
        ("Linear", Ease::Linear),
        ("InOut(Cubic)", Ease::InOut(Family::Cubic)),
        ("Steps(4)", Ease::Steps(4)),
    ] {
        assert_eq!(ron::to_string(&ease).expect("an ease writes"), text);
    }
}

#[test]
fn a_tween_reads_back_from_ron_and_takes_defaults() {
    let full = Tween::new((1, -2), (30, 40), 250)
        .with_delay(10)
        .with_ease(Ease::OutIn(Family::Bounce))
        .with_repeat(Repeat::Times(3))
        .with_yoyo(true);
    let text = ron::to_string(&full).expect("a tween writes");
    assert_eq!(
        ron::from_str::<Tween<(i32, i32)>>(&text).expect("a tween reads"),
        full
    );

    let short: Tween<u8> = ron::from_str("(from: 0, to: 255, duration: 300)").expect("it reads");
    assert_eq!(short, Tween::new(0, 255, 300));
    assert_eq!(
        (short.delay, short.ease, short.repeat, short.yoyo),
        (0, Ease::Linear, Repeat::Once, false)
    );
    let looped: Tween<i32> =
        ron::from_str("(from: 0, to: 1, duration: 2, repeat: Loop)").expect("it reads");
    assert_eq!(looped.repeat, Repeat::Loop);
}

#[test]
fn mix_hits_its_ends_and_goes_on_past_them() {
    let (before, past) = (-ONE / 10, 11 * ONE / 10);
    assert_eq!((i32::mix(10, 20, before), i32::mix(10, 20, past)), (9, 21));
    assert_eq!(
        (i64::mix(-10, 10, before), i64::mix(-10, 10, past)),
        (-12, 12)
    );
    assert_eq!(<(i32, i32)>::mix((0, 100), (100, 0), before), (-10, 110));
    assert_eq!(<(i32, i32)>::mix((0, 100), (100, 0), past), (110, -10));
    // Unsigned values and channels stop at their bounds.
    assert_eq!((u8::mix(0, 255, before), u8::mix(0, 255, past)), (0, 255));
    assert_eq!((u8::mix(5, 250, before), u8::mix(5, 250, past)), (0, 255));
    assert_eq!(u16::mix(0, 1000, before), 0);
    assert_eq!(u32::mix(10, 0, past), 0);
    assert_eq!(i32::mix(i32::MAX - 1, i32::MAX, past), i32::MAX);
    assert_eq!(i64::mix(i64::MIN, i64::MAX, ONE / 2), 0);
    // Halves round up.
    assert_eq!(i32::mix(0, 1, ONE / 2), 1);
    assert_eq!(i32::mix(0, -1, ONE / 2), 0);
}

#[test]
fn a_tween_holds_from_through_its_delay() {
    let tween = Tween::new(10, 20, 100).with_delay(50);
    assert_eq!(tween.value_at(0), 10);
    assert_eq!(tween.value_at(49), 10);
    assert_eq!(tween.value_at(50), 10);
    assert_eq!(tween.value_at(100), 15);
    assert_eq!(tween.value_at(150), 20);
    assert_eq!(tween.done_at(), Some(150));
}

#[test]
fn a_tween_plays_times_with_yoyo_and_ends_where_its_last_play_did() {
    let twice = Tween::new(0, 100, 10)
        .with_repeat(Repeat::Times(2))
        .with_yoyo(true);
    assert_eq!(twice.done_at(), Some(20));
    let values: Vec<i32> = (0..=22).map(|t| twice.value_at(t)).collect();
    assert_eq!(
        values,
        [
            0, 10, 20, 30, 40, 50, 60, 70, 80, 90, 100, 90, 80, 70, 60, 50, 40, 30, 20, 10, 0, 0, 0
        ]
    );
    // Without yoyo each play starts again from `from`.
    let again = twice.with_yoyo(false);
    assert_eq!(
        (again.value_at(9), again.value_at(10), again.value_at(20)),
        (90, 0, 100)
    );
    // Three plays with yoyo end on `to`; Times(0) is once.
    assert_eq!(twice.with_repeat(Repeat::Times(3)).value_at(1000), 100);
    assert_eq!(twice.with_repeat(Repeat::Times(0)).done_at(), Some(10));
}

#[test]
fn a_tween_of_no_duration_jumps_at_the_end_of_its_delay() {
    let jump = Tween::new(1, 9, 0).with_delay(5);
    assert_eq!((jump.value_at(4), jump.value_at(5)), (1, 9));
    assert_eq!(jump.done_at(), Some(5));
    let back = jump.with_repeat(Repeat::Times(2)).with_yoyo(true);
    assert_eq!(back.value_at(5), 1);
    let looped = jump.with_repeat(Repeat::Loop).with_yoyo(true);
    assert_eq!((looped.value_at(5), looped.value_at(u32::MAX)), (9, 9));
    assert_eq!(looped.done_at(), None);
}

#[test]
fn a_tween_at_the_end_of_time() {
    let long = Tween::new(0, 1000, u32::MAX).with_delay(10);
    assert_eq!(long.done_at(), Some(u32::MAX));
    assert_eq!(long.value_at(u32::MAX), 1000);
    assert_eq!(long.value_at(u32::MAX / 2), 500);
    let looped = Tween::new(0, 1000, 7).with_repeat(Repeat::Loop);
    assert_eq!(looped.value_at(u32::MAX), looped.value_at(u32::MAX % 7));
    assert!(!looped.finished_between(0, u32::MAX));
}

#[test]
fn a_tween_done_at_0_finishes_in_the_first_window() {
    let now = Tween::new(0, 1, 0);
    assert_eq!(now.done_at(), Some(0));
    assert!(!now.finished_between(0, 0));
    assert!(now.finished_between(0, 1));
    assert!(!now.finished_between(1, 2));
}

proptest! {
    #[test]
    fn value_at_is_pure(
        from in any::<i32>(),
        to in any::<i32>(),
        delay in 0_u32..1000,
        duration in 0_u32..1000,
        family in 0_usize..10,
        yoyo in any::<bool>(),
        t in any::<u32>(),
    ) {
        let tween = Tween::new(from, to, duration)
            .with_delay(delay)
            .with_ease(Ease::InOut(FAMILIES[family]))
            .with_repeat(Repeat::Times(3))
            .with_yoyo(yoyo);
        prop_assert_eq!(tween.value_at(t), tween.value_at(t));
        if let Some(done) = tween.done_at() {
            prop_assert_eq!(tween.value_at(done.saturating_add(t % 1000)), tween.value_at(done));
        }
    }

    #[test]
    fn a_loop_repeats(
        delay in 0_u32..1000,
        duration in 1_u32..1000,
        family in 0_usize..10,
        yoyo in any::<bool>(),
        t in 0_u32..1_000_000,
    ) {
        let tween = Tween::new(-500, 500, duration)
            .with_delay(delay)
            .with_ease(Ease::Out(FAMILIES[family]))
            .with_repeat(Repeat::Loop)
            .with_yoyo(yoyo);
        let t = delay + t;
        let period = if yoyo { 2 * duration } else { duration };
        prop_assert_eq!(tween.value_at(t + period), tween.value_at(t));
        if !yoyo {
            prop_assert_eq!(tween.value_at(t + duration), tween.value_at(t));
        }
    }

    #[test]
    fn finished_between_fires_once_over_any_split_of_time(
        delay in 0_u32..100,
        duration in 0_u32..100,
        times in 0_u16..4,
        steps in proptest::collection::vec(0_u32..40, 1..40),
    ) {
        let tween = Tween::new(0, 1, duration)
            .with_delay(delay)
            .with_repeat(Repeat::Times(times));
        let looped = tween.with_repeat(Repeat::Loop);
        let mut prev = 0_u32;
        let (mut fired, mut looped_fired) = (0, 0);
        for step in steps.iter().chain([&1000]) {
            let now = prev + step;
            fired += u32::from(tween.finished_between(prev, now));
            looped_fired += u32::from(looped.finished_between(prev, now));
            prev = now;
        }
        prop_assert_eq!(fired, 1);
        prop_assert_eq!(looped_fired, 0);
    }

    #[test]
    fn mix_hits_its_ends(a in any::<i32>(), b in any::<i32>(), c in any::<u8>(), d in any::<u8>()) {
        prop_assert_eq!((i32::mix(a, b, 0), i32::mix(a, b, ONE)), (a, b));
        prop_assert_eq!((i64::mix(a.into(), b.into(), 0), i64::mix(a.into(), b.into(), ONE)), (a.into(), b.into()));
        prop_assert_eq!((u8::mix(c, d, 0), u8::mix(c, d, ONE)), (c, d));
        prop_assert_eq!((u16::mix(c.into(), d.into(), 0), u16::mix(c.into(), d.into(), ONE)), (c.into(), d.into()));
        prop_assert_eq!((u32::mix(a.unsigned_abs(), b.unsigned_abs(), 0), u32::mix(a.unsigned_abs(), b.unsigned_abs(), ONE)), (a.unsigned_abs(), b.unsigned_abs()));
        prop_assert_eq!((<(i32, i32)>::mix((a, b), (b, a), 0), <(i32, i32)>::mix((a, b), (b, a), ONE)), ((a, b), (b, a)));
    }
}
