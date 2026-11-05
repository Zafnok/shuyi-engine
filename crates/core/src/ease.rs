//! Ease curves and tweens, in integers, drawn from the time alone.
//!
//! An ease curve ([`Ease`], or any [`Easing`]) bends progress: it takes how
//! far through something is, in 1/65536 ([`ONE`] is the end), and gives how
//! far along the value is. A [`Tween`] is a value going from one end to the
//! other over a time, along a curve. [`Lerp`] is how a type goes between
//! two of its values.
//!
//! ```
//! use shuyi_core::{Ease, Family, Tween};
//!
//! // A score that pops: it rises 6 pixels in 300 milliseconds, overshoots
//! // a little and settles.
//! let pop = Tween::new(0, -6, 300).with_ease(Ease::Out(Family::Back));
//! assert_eq!(pop.value_at(0), 0);
//! assert_eq!(pop.value_at(200), -7); // past its end: the overshoot
//! assert_eq!(pop.value_at(300), -6);
//! assert_eq!(pop.value_at(5000), -6);
//! ```
//!
//! # Drawn from the time alone
//!
//! A tween stores no progress. Its value is a function of its fields and
//! the time since it started ([`Tween::value_at`]), so a tween never needs
//! updating, and replays, saves and a test of any moment are exact. A tick
//! simulation stores the tween and the step it started on; a view stores it
//! and the millisecond.
//!
//! Its time is a plain `u32` whose unit the caller picks: steps in a tick
//! simulation, turns in a turn game, whole milliseconds in a view. A screen
//! gets each frame's time in seconds, so a view counts its own
//! milliseconds. A caller with a `u64` clock passes the time since the
//! start as `u32::try_from(now.saturating_sub(start)).unwrap_or(u32::MAX)`.
//!
//! "It landed" is [`Tween::finished_between`] the previous frame's or
//! step's time and this one's, true once, with no flag kept.
//!
//! # The curves
//!
//! [`Ease`] names Robert Penner's curves as easings.net lists them: ten
//! [`Family`]s, each eased in, out, in and out, or out and in, with
//! [`Linear`](Ease::Linear), smoothstep ([`Smooth`](Ease::Smooth)) and
//! [`Steps`](Ease::Steps). In RON a curve is written `Linear`,
//! `InOut(Cubic)` or `Steps(4)`.
//!
//! - `Out(f)` is `ONE - In(f)(ONE - p)`, for every family.
//! - `InOut(f)` is `In` over the first half and `Out` over the second,
//!   except for `Back` and `Elastic`, whose `InOut` on easings.net has
//!   constants of its own.
//! - `OutIn(f)` is `Out` over the first half and `In` over the second.
//! - `Back` and `Elastic` overshoot: they go below 0 or above [`ONE`] on
//!   the way.
//!
//! Every curve gives exactly 0 at 0 and exactly [`ONE`] at [`ONE`], is
//! within 1/65536 of its formula for `Linear`, `Smooth`, `Steps` and `Quad`
//! to `Quint`, and within 2/65536 for the rest. The curves are computed in
//! integers, so they are the same on every machine; a skin that works in
//! floats eases through `shuyi_ui::ease::ease_f32`, which goes through
//! these.
//!
//! The same curves in other engines, checked against their docs on
//! 2026-11-04 (Godot 4.7's `Tween`, Bevy 0.20.1's `EaseFunction` in
//! `bevy_curve`, Defold 1.14.0's `go.animate`, MDN's `<easing-function>`).
//! A family and a way of easing make the name: `InOut(Cubic)` is
//! easings.net's `easeInOutCubic`, Godot's `TRANS_CUBIC` with
//! `EASE_IN_OUT`, Bevy's `CubicInOut` and Defold's `go.EASING_INOUTCUBIC`.
//!
//! | [`Family`] | easings.net | Godot | Bevy | Defold |
//! | ---------- | ----------- | ----- | ---- | ------ |
//! | `Sine` | `Sine` | `TRANS_SINE` | `Sine` | `SINE` |
//! | `Quad` | `Quad` | `TRANS_QUAD` | `Quadratic` | `QUAD` |
//! | `Cubic` | `Cubic` | `TRANS_CUBIC` | `Cubic` | `CUBIC` |
//! | `Quart` | `Quart` | `TRANS_QUART` | `Quartic` | `QUART` |
//! | `Quint` | `Quint` | `TRANS_QUINT` | `Quintic` | `QUINT` |
//! | `Expo` | `Expo` | `TRANS_EXPO` (1) | `Exponential` (2) | `EXPO` |
//! | `Circ` | `Circ` | `TRANS_CIRC` | `Circular` | `CIRC` |
//! | `Back` | `Back` | `TRANS_BACK` | `Back` (3) | `BACK` |
//! | `Elastic` | `Elastic` | `TRANS_ELASTIC` | `Elastic` | `ELASTIC` |
//! | `Bounce` | `Bounce` | `TRANS_BOUNCE` | `Bounce` | `BOUNCE` |
//!
//! | [`Ease`] | easings.net | Godot | Bevy | Defold | CSS |
//! | -------- | ----------- | ----- | ---- | ------ | --- |
//! | `In(f)` | `easeIn…` | `EASE_IN` | `…In` | `IN…` | none |
//! | `Out(f)` | `easeOut…` | `EASE_OUT` | `…Out` | `OUT…` | none |
//! | `InOut(f)` | `easeInOut…` | `EASE_IN_OUT` | `…InOut` | `INOUT…` | none |
//! | `OutIn(f)` | none | `EASE_OUT_IN` | none | `OUTIN…` | none |
//! | `Linear` | none | `TRANS_LINEAR` | `Linear` | `LINEAR` | `linear` |
//! | `Smooth` | none | none | `SmoothStep` | none | none |
//! | `Steps(n)` | none | none | `Steps(n, JumpAt::End)` | none | `steps(n, jump-end)` |
//!
//! 1. Godot's `TRANS_EXPO` is Penner's first version, shifted by a
//!    thousandth so it has no jump: within 1/1000 of `Expo`.
//! 2. Bevy's `Exponential` curves are scaled to meet 0 and 1 without a
//!    jump: within 1/1000 of `Expo`.
//! 3. Bevy's `BackInOut` takes its constant as `1.70158 + 1.525`, not
//!    `1.70158 * 1.525`, so it overshoots further than `InOut(Back)`.
//!
//! None here matches Godot's `TRANS_SPRING`, Bevy's `Elastic(omega)`,
//! `SmoothStepIn`, `SmoothStepOut` and `SmootherStep` curves, or CSS's
//! named curves (`ease`, `ease-in`, `ease-out`, `ease-in-out`), which are
//! cubic Béziers. CSS's `step-end` is `Steps(1)`.
//!
//! # Curves and tweens are data
//!
//! [`Ease`], [`Tween`] and [`Repeat`] read and write with serde, inside a
//! game's own RON files. A tween written with only `from`, `to` and
//! `duration` takes the defaults for the rest:
//!
//! ```
//! use shuyi_core::{Ease, Family, Tween};
//!
//! let tween: Tween<i32> = ron::from_str("(from: 0, to: 10, duration: 300)")?;
//! assert_eq!(tween, Tween::new(0, 10, 300));
//! let eased: Tween<i32> =
//!     ron::from_str("(from: 0, to: 10, duration: 300, ease: InOut(Cubic))")?;
//! assert_eq!(eased.ease, Ease::InOut(Family::Cubic));
//! # Ok::<(), ron::error::SpannedError>(())
//! ```

use serde::{Deserialize, Serialize};

mod tables;

use tables::{EXP2, QUARTER_SINE};

/// The whole way, in 1/65536: progress runs from 0 to `ONE`.
pub const ONE: i32 = 65_536;

/// [`ONE`], wide.
const WIDE: i64 = 65_536;

/// Half the way.
const HALF: i64 = WIDE / 2;

/// A turn, in the 1/2^24 an angle of `Elastic` is taken in.
const TURN: i64 = 1 << 24;

/// A quarter of a [`TURN`].
const QUARTER: i64 = TURN / 4;

/// `Back`'s constant on easings.net, `c1 = 1.70158`, as a fraction.
const BACK: (i128, i128) = (170_158, 100_000);

/// `Back`'s constant in and out, `c2 = c1 * 1.525`, as a fraction.
const BACK_IN_OUT: (i128, i128) = (259_490_950, 100_000_000);

/// A curve: how far along a value is, `p` of the way through.
pub trait Easing {
    /// The curve at `p`, in 1/65536. `p` below 0 counts as 0 and `p` above
    /// [`ONE`] as `ONE`. The value may go past 0 and `ONE` on the way (an
    /// overshoot), and should be 0 at 0 and `ONE` at `ONE`.
    fn ease(&self, p: i32) -> i32;
}

/// The ten families of Penner's curves, as easings.net names them, from the
/// gentlest to the ones that overshoot and bounce.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Family {
    /// A quarter of a sine wave.
    Sine,
    /// `p²`.
    Quad,
    /// `p³`.
    Cubic,
    /// `p⁴`.
    Quart,
    /// `p⁵`.
    Quint,
    /// `2^(10p - 10)`: still, then fast.
    Expo,
    /// A quarter of a circle.
    Circ,
    /// Pulls back a little before it goes.
    Back,
    /// Springs back and forth.
    Elastic,
    /// Bounces off its end.
    Bounce,
}

/// A named ease curve. The [module docs](self) name each beside other
/// engines' names.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Ease {
    /// No easing: the value moves as the time does.
    #[default]
    Linear,
    /// Smoothstep, `3p² - 2p³`: slow at both ends, fastest in the middle.
    Smooth,
    /// The family's curve: slow at the start.
    In(Family),
    /// The family's curve backwards: slow at the end.
    Out(Family),
    /// [`In`](Self::In) over the first half, [`Out`](Self::Out) over the
    /// second: slow at both ends.
    InOut(Family),
    /// [`Out`](Self::Out) over the first half, [`In`](Self::In) over the
    /// second: fast at both ends.
    OutIn(Family),
    /// `n` equal jumps, each at the end of its step, as CSS's
    /// `steps(n, jump-end)`. `Steps(0)` acts as `Steps(1)`, which holds the
    /// start until the end.
    Steps(u16),
}

impl Easing for Ease {
    fn ease(&self, p: i32) -> i32 {
        let p = i64::from(p.clamp(0, ONE));
        let wide = match *self {
            Ease::Linear => i128::from(p),
            Ease::Smooth => smooth(p).units(),
            Ease::Steps(n) => steps(n, p).units(),
            Ease::In(family) => rise(family, p, false).units(),
            Ease::Out(family) => i128::from(WIDE) - rise(family, WIDE - p, false).units(),
            // Both halves are one curve about the middle, `side` saying
            // which half `p` is in (0 at the middle, where both give HALF).
            Ease::InOut(family) => {
                let (side, off) = (i128::from((p - HALF).signum()), (p - HALF).abs());
                let near = rise(family, 2 * (HALF - off), true).half().units();
                i128::from(HALF) + side * (i128::from(HALF) - near)
            }
            Ease::OutIn(family) => {
                let (side, off) = (i128::from((p - HALF).signum()), (p - HALF).abs());
                i128::from(HALF) + side * rise(family, 2 * off, false).half().units()
            }
        };
        narrow(wide)
    }
}

/// A curve's value, `wide`, as an `i32`. Every curve stays between -1 and 2
/// of [`ONE`].
#[expect(
    clippy::cast_possible_truncation,
    reason = "every curve stays between -65 536 and 131 072"
)]
const fn narrow(wide: i128) -> i32 {
    wide as i32
}

/// A value as an exact fraction, `num / den` with `den` above 0, so that a
/// curve rounds once, at the end.
#[derive(Clone, Copy)]
struct Frac {
    /// The numerator.
    num: i128,
    /// The denominator, above 0.
    den: i128,
}

impl Frac {
    /// `num / den`.
    const fn new(num: i128, den: i128) -> Self {
        Self { num, den }
    }

    /// Half of it.
    const fn half(self) -> Self {
        Self::new(self.num, self.den * 2)
    }

    /// It in 1/65536, rounded to the nearest, halves up.
    fn units(self) -> i128 {
        round_div(self.num * i128::from(WIDE), self.den)
    }
}

/// `num / den` rounded to the nearest, halves up, for `den` above 0.
fn round_div(num: i128, den: i128) -> i128 {
    (2 * num + den).div_euclid(2 * den)
}

/// Smoothstep at `p` (0 to [`ONE`]): `3p² - 2p³`.
fn smooth(p: i64) -> Frac {
    let (p, one) = (i128::from(p), i128::from(WIDE));
    Frac::new(3 * p * p * one - 2 * p * p * p, one * one * one)
}

/// `n` steps at `p` (0 to [`ONE`]): the steps begun, of `n`.
fn steps(n: u16, p: i64) -> Frac {
    let n = i128::from(n.max(1));
    Frac::new(i128::from(p) * n / i128::from(WIDE), n)
}

/// A family's `In` curve at `q` (0 to [`ONE`]). `in_out` takes the
/// constants easings.net's `InOut` has for `Back` and `Elastic`.
fn rise(family: Family, q: i64, in_out: bool) -> Frac {
    let (x, one) = (i128::from(q), i128::from(WIDE));
    match family {
        // Easings.net sets both to 0 at 0, where their formulas aren't.
        Family::Expo | Family::Elastic if q == 0 => Frac::new(0, 1),
        // 1 - cos(x PI/2) = 1 - sin((1 - x) PI/2).
        Family::Sine => {
            let s = quarter_sine((WIDE - q) * (QUARTER / WIDE));
            Frac::new(s.den - s.num, s.den)
        }
        Family::Quad => Frac::new(x.pow(2), one.pow(2)),
        Family::Cubic => Frac::new(x.pow(3), one.pow(3)),
        Family::Quart => Frac::new(x.pow(4), one.pow(4)),
        Family::Quint => Frac::new(x.pow(5), one.pow(5)),
        // 2^(10x - 10).
        Family::Expo => exp2_down(10 * (WIDE - q)),
        // 1 - sqrt(1 - x²), the root taken in 1/2^32.
        Family::Circ => {
            let under = (u128::from(1_u64 << 32) - u128::from(q.unsigned_abs().pow(2))) << 32;
            let root = i128::try_from(under.isqrt()).unwrap_or(0);
            Frac::new((1 << 32) - root, 1 << 32)
        }
        // (c + 1)x³ - c x².
        Family::Back => {
            let (c, d) = if in_out { BACK_IN_OUT } else { BACK };
            Frac::new((d + c) * x.pow(3) - c * x.pow(2) * one, d * one.pow(3))
        }
        // -2^(10x - 10) sin((10x - 10.75) 2PI/3); in and out,
        // sin((20x - 22.25) 2PI/9) in 1/2^24 of a turn. At 1 the angle is
        // exactly a quarter turn back, so it is exactly 1 there.
        Family::Elastic => {
            let (k, start, per) = if in_out {
                (20, 1_458_176, 9)
            } else {
                (10, 704_512, 3)
            };
            let angle = round_div(i128::from(k * q - start) * i128::from(TURN / WIDE), per);
            let grow = exp2_down(10 * (WIDE - q));
            let sine = sine_turn(i64::try_from(angle).unwrap_or(0));
            Frac::new(-grow.num * sine.num, grow.den * sine.den)
        }
        // 1 - easeOutBounce(1 - x).
        Family::Bounce => bounce_down(WIDE - q),
    }
}

/// `1 - easeOutBounce(y)`, `y` being `u` (0 to [`ONE`]). With
/// `7.5625 = 121/16` and `2.75 = 11/4`, each arc is
/// `(22u - a)² / 64 + k` in 1/ONE², for `a` in `0, 12, 18, 21` ONE and `k`
/// in `0, 3/4, 15/16, 63/64`.
fn bounce_down(u: i64) -> Frac {
    let (u, one) = (i128::from(u), i128::from(WIDE));
    // The arcs start at 4/11, 8/11 and 10/11, which no `u` falls on.
    let (a, k) = match 11 * u / one {
        0..=3 => (0, 0),
        4..=7 => (12, 48),
        8 | 9 => (18, 60),
        _ => (21, 63),
    };
    let arc = (22 * u - a * one).pow(2);
    Frac::new((64 - k) * one * one - arc, 64 * one * one)
}

/// `2^(-e)`, `e` being `down` in 1/65536 (0 or more), from the table of
/// `2^x`, in 1/2^24 before the halvings.
fn exp2_down(down: i64) -> Frac {
    let (whole, part) = (down >> 16, down & (WIDE - 1));
    // 2^(-whole - part) = 2^(1 - part) / 2^(whole + 1).
    let at = WIDE - part;
    let index = usize::try_from(at >> 8).unwrap_or(0);
    let between = at & 0xff;
    let next = EXP2[(index + 1).min(256)];
    let value = EXP2[index] * 256 + (next - EXP2[index]) * between;
    Frac::new(i128::from(value), 1_i128 << (24 + whole + 1))
}

/// The sine of `angle`, in 1/2^24 of a turn, any angle.
fn sine_turn(angle: i64) -> Frac {
    let angle = angle.rem_euclid(TURN);
    let into = angle % QUARTER;
    // Up the quarter in the first and third quarters, down it in the others.
    let s = if (angle / QUARTER) % 2 == 0 {
        quarter_sine(into)
    } else {
        quarter_sine(QUARTER - into)
    };
    // Below 0 in the second half of the turn.
    if angle >= 2 * QUARTER {
        Frac::new(-s.num, s.den)
    } else {
        s
    }
}

/// The sine of `angle`, 0 to a quarter of a turn in 1/2^24 of a turn
/// (0 to 2^22), from the table, in 1/2^30.
fn quarter_sine(angle: i64) -> Frac {
    let index = usize::try_from(angle >> 14).unwrap_or(0);
    let between = angle & 0x3fff;
    let next = QUARTER_SINE[(index + 1).min(256)];
    let value = QUARTER_SINE[index] * (1 << 14) + (next - QUARTER_SINE[index]) * between;
    Frac::new(i128::from(value), 1 << 30)
}

/// A type that goes between two of its values.
///
/// `mix(from, to, p)` is `from + (to - from) * p / ONE`, rounded to the
/// nearest with halves up: exactly `from` at 0 and exactly `to` at [`ONE`].
/// Past them (an overshoot) it goes on in a line, and stops at the type's
/// bounds: a `u8`, a colour's channel, stops at 0 and 255.
///
/// It is not called `lerp`, because `shuyi_ui`'s `Rgb` has an inherent
/// `lerp` in `f32`.
pub trait Lerp: Copy {
    /// The value `p` (in 1/65536) of the way from `from` to `to`.
    #[must_use]
    fn mix(from: Self, to: Self, p: i32) -> Self;
}

/// `from + (to - from) * p / ONE`, rounded to the nearest with halves up.
fn mix_wide(from: i128, to: i128, p: i32) -> i128 {
    from + round_div((to - from) * i128::from(p), i128::from(WIDE))
}

/// `Lerp` for integers, stopping at their bounds.
macro_rules! lerp_ints {
    ($($int:ty),*) => {$(
        impl Lerp for $int {
            fn mix(from: Self, to: Self, p: i32) -> Self {
                let wide = mix_wide(from.into(), to.into(), p);
                <$int>::try_from(wide).unwrap_or(if wide < 0 { <$int>::MIN } else { <$int>::MAX })
            }
        }
    )*};
}

lerp_ints!(i32, i64, u8, u16, u32);

impl Lerp for (i32, i32) {
    fn mix(from: Self, to: Self, p: i32) -> Self {
        (i32::mix(from.0, to.0, p), i32::mix(from.1, to.1, p))
    }
}

/// How many times a [`Tween`] plays, in all.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Repeat {
    /// Once.
    #[default]
    Once,
    /// This many times in all; `Times(0)` acts as [`Once`](Self::Once).
    Times(u16),
    /// For ever.
    Loop,
}

impl Repeat {
    /// The plays in all, at least 1; `None` for [`Loop`](Self::Loop).
    const fn plays(self) -> Option<u32> {
        match self {
            Repeat::Once | Repeat::Times(0) => Some(1),
            Repeat::Times(n) => Some(n as u32),
            Repeat::Loop => None,
        }
    }
}

/// A value going from `from` to `to` over `duration`, along a curve, after
/// `delay`: drawn from the time since it started alone
/// ([`value_at`](Self::value_at)). The [module docs](self) say how a
/// simulation and a view keep one.
///
/// ```
/// use shuyi_core::ease::Repeat;
/// use shuyi_core::{Ease, Tween};
///
/// // A title that bobs 2 cells up and down for ever, 500 ms each way.
/// const BOB: Tween<i32> = Tween::new(0, -2, 500)
///     .with_ease(Ease::Smooth)
///     .with_repeat(Repeat::Loop)
///     .with_yoyo(true);
/// assert_eq!(BOB.value_at(500), -2);
/// assert_eq!(BOB.value_at(1000), 0);
/// assert_eq!(BOB.value_at(1250), BOB.value_at(250));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Tween<T, E = Ease> {
    /// The value at the start.
    pub from: T,
    /// The value at the end.
    pub to: T,
    /// How long it holds `from` before its first play. 0 when left out.
    #[serde(default)]
    pub delay: u32,
    /// How long one play lasts.
    pub duration: u32,
    /// Its curve. [`Ease::Linear`] when left out.
    #[serde(default)]
    pub ease: E,
    /// How many times it plays. [`Repeat::Once`] when left out.
    #[serde(default)]
    pub repeat: Repeat,
    /// Whether every second play runs from `to` back to `from`, its time
    /// backwards, as CSS's `alternate`. `false` when left out.
    #[serde(default)]
    pub yoyo: bool,
}

impl<T: Copy> Tween<T> {
    /// One play from `from` to `to` over `duration`, with no delay, along
    /// [`Ease::Linear`].
    pub const fn new(from: T, to: T, duration: u32) -> Self {
        Self {
            from,
            to,
            delay: 0,
            duration,
            ease: Ease::Linear,
            repeat: Repeat::Once,
            yoyo: false,
        }
    }

    /// It along `ease`: an [`Ease`], or a curve of the game's own.
    #[must_use]
    pub const fn with_ease<F>(self, ease: F) -> Tween<T, F> {
        Tween {
            from: self.from,
            to: self.to,
            delay: self.delay,
            duration: self.duration,
            ease,
            repeat: self.repeat,
            yoyo: self.yoyo,
        }
    }
}

/// The builders are `const fn`s, so a game can keep a tween in a `const`.
/// A `const fn` can't drop a part of a type that may need dropping, so they
/// ask for `Copy`: every [`Lerp`] type and every [`Ease`] is. A tween along
/// a curve that isn't `Copy` calls them before
/// [`with_ease`](Tween::with_ease), or assigns its fields.
impl<T: Copy, E: Copy> Tween<T, E> {
    /// It after `delay`.
    #[must_use]
    pub const fn with_delay(self, delay: u32) -> Self {
        Self { delay, ..self }
    }

    /// It played as `repeat` says.
    #[must_use]
    pub const fn with_repeat(self, repeat: Repeat) -> Self {
        Self { repeat, ..self }
    }

    /// It back and forth.
    #[must_use]
    pub const fn with_yoyo(self, yoyo: bool) -> Self {
        Self { yoyo, ..self }
    }
}

impl<T, E> Tween<T, E> {
    /// When its last play ends: `delay + duration * plays`, stopping at
    /// `u32::MAX`. `None` for [`Repeat::Loop`].
    pub const fn done_at(&self) -> Option<u32> {
        match self.repeat.plays() {
            Some(plays) => Some(
                self.delay
                    .saturating_add(self.duration.saturating_mul(plays)),
            ),
            None => None,
        }
    }

    /// Whether it ended in the window after `prev` up to `now`: true when
    /// `prev < d <= now`, `d` being [`done_at`](Self::done_at) and at least
    /// 1, so a tween done at 0 ends in the first window that holds any time.
    ///
    /// A screen or a simulation passes the previous frame's or step's time
    /// and this one's, starting from 0, and hears "it landed" once, with no
    /// flag kept. Never true for [`Repeat::Loop`].
    pub fn finished_between(&self, prev: u32, now: u32) -> bool {
        self.done_at().is_some_and(|done| {
            let done = done.max(1);
            prev < done && done <= now
        })
    }
}

impl<T: Lerp, E: Easing> Tween<T, E> {
    /// Its value `t` after it started: `from` before the delay, the eased
    /// value during a play, and from [`done_at`](Self::done_at) on the
    /// value its last play ended on: for every [`Ease`], `to`, or `from`
    /// after an even number of plays with `yoyo`. With a `duration` of 0
    /// it jumps to that value at the end of the delay; with
    /// [`Repeat::Loop`] it then holds `to`.
    pub fn value_at(&self, t: u32) -> T {
        if t < self.delay {
            return self.from;
        }
        if self.duration == 0 || self.done_at().is_some_and(|done| t >= done) {
            return self.end();
        }
        let since = t - self.delay;
        let play = since / self.duration;
        let into = u64::from(since % self.duration);
        let p = i32::try_from(into * u64::from(ONE.unsigned_abs()) / u64::from(self.duration))
            .unwrap_or(ONE);
        let p = if self.yoyo && play % 2 == 1 {
            ONE - p
        } else {
            p
        };
        T::mix(self.from, self.to, self.ease.ease(p))
    }

    /// The value its last play ends on, through the curve: `to`, or `from`
    /// after a play backwards, for every [`Ease`].
    fn end(&self) -> T {
        let p = match self.repeat.plays() {
            Some(plays) if self.yoyo && plays % 2 == 0 => 0,
            _ => ONE,
        };
        T::mix(self.from, self.to, self.ease.ease(p))
    }
}

#[cfg(test)]
mod tests;
