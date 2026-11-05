# ADR-0079: Easing and tweens: integer curves in `shuyi-core`, drawn from the time alone

- **Status:** Accepted
- **Date:** 2026-11-04
- **Related tickets:** 2109 (this), 2135 and 2136 (its follow-ups),
  1123 and 2104 (tweens on sprites), 2339, 2362, 2363, 2364, game#1228
- **Supersedes:** one sentence of ADR-0004 §3, "Easing curves come from
  the cinematic module". The rest of §3 stands.

## Context

The comparison of 2026-10-20 with Bevy 0.20, Godot 4.7, Unity 6, Unreal
5.8, GameMaker, Defold and Construct 3 found named ease curves in every
one of them. Godot's `Tween` has 12 transitions by four eases, Defold's
`go.animate` 41 easings, Bevy an `EaseFunction`; most name their curves
after Robert Penner's equations as easings.net lists them
(`easeInOutCubic`), and those are the names agents reach for. Bevy and
Unity leave the tweens themselves to outside crates (`bevy_tweening`,
DOTween).

The engine had one curve: `shuyi_ui::cinematic::ease_in_out`, a
smoothstep in `f32` that `Pan::centre` uses. A simulation can't use it
(no floats in a simulation, ADR-0002 §2), and `shuyi-core` can't reach
it. Yet ADR-0004 §3 says "Easing curves come from the cinematic
module", and two tickets (1123, 2104) once left tweens on sprites out
because "the cinematic easing covers them". The engine's own motions
were written by hand, some in `f32` seconds and some in whole
milliseconds (the cursor's pulse, music fades, the text box typing out,
walk and idle frames).

The game writes its own curves too (its fight stage's quad ease-in and
ease-out, its AI camera's straight pan; game#1228), and so would most
2D games: a score that pops, a menu that slides in, a title that bobs.

Two constraints shape the answer:

- **Floats.** A curve in a tick simulation must be the same on every
  machine (ADR-0003, ADR-0032's float ban), and the parked GBA and DS
  have no floating-point unit (ADR-0012).
- **Time.** ADR-0004 §3 draws clips and cinematics from the time since
  they started (game ADR-0060), so a replay, a save and a test of any
  moment are exact. Screens get each frame's time in seconds (ADR-0024
  §3); a simulation counts steps (ADR-0003 §2) or turns.

## Decision

1. **Easing lives in `shuyi_core::ease`, in integers, `no_std`.**
   Progress is in 1/65536 (`ease::ONE = 65_536`). `Easing` is the
   trait of a curve (`fn ease(&self, p: i32) -> i32`), and `Ease` names
   the curves. `Lerp` is how a type goes between two of its values, and
   `Tween<T, E = Ease>` is a value going from `from` to `to` over a
   time. The polynomials are exact fractions in `i128`, rounded once;
   `Sine`, `Expo` and `Elastic` read two tables of 257 entries (a
   quarter of a sine wave, `2^x`) with straight lines between entries;
   `Circ` takes an integer square root. No float is in the module
   outside its tests. This supersedes ADR-0004 §3's "Easing curves come
   from the cinematic module".
2. **A tween stores no progress.** Its value is a function of its
   fields and the time since it started (`Tween::value_at`), as ADR-0004
   §3 says of clips. A simulation stores a tween and the step it started
   on; a view stores it and the millisecond. "It landed" is
   `Tween::finished_between(prev, now)`, true in exactly one window of
   any split of time from 0, so no flag is kept.
3. **A tween's time is a plain `u32` whose unit the caller picks:**
   whole milliseconds in a view, steps in a tick simulation (ADR-0003
   §2), turns in a turn game. There is no time type. Screens get each
   frame's time in seconds (ADR-0024 §3), so a view counts its own
   milliseconds, as `GridScene::clock_ms` does
   (`crates/ui/src/grid/scene.rs`).
4. **Curves are named as easings.net and Godot name them:** ten
   `Family`s (`Sine` to `Bounce`), each `In`, `Out`, `InOut` or `OutIn`
   (Godot's `EASE_OUT_IN`), with `Linear`, `Smooth` (smoothstep) and
   `Steps(n)` (CSS's `steps(n, jump-end)`). `Out(f)` is
   `ONE - In(f)(ONE - p)` for every family; `InOut` follows easings.net,
   whose `Back` and `Elastic` take constants of their own there. The
   module docs carry the table of each engine's name.
5. **Curves and tweens are data, not a format.** They derive serde and
   sit inside other RON files (a cinematic, 2330's `.ui.ron`, a game's
   data): `InOut(Cubic)`, `(from: 0, to: 10, duration: 300)` with the
   rest defaulted. No asset kind and no extension, so ADR-0007 and
   ADR-0019 are untouched.
6. **A sequence orders the steps of one value; a timeline orders the
   shots of a cinematic.** A sequence of tweens (2135's `Sequence`)
   runs one value through steps one after another;
   `cinematic::Timeline` says which shot a time falls in. A shot may
   hold tweens for what moves inside it. Neither replaces the other.
7. **A skin that works in floats eases through
   `shuyi_ui::ease::ease_f32`,** which turns `p` into 1/65536, eases it
   by the integer curve and turns it back, so the curve is the same on
   every machine. `cinematic::ease_in_out` keeps its body, so nothing
   the game draws changes; 2136 moves the engine's own motions.

The public types follow ADR-0039: `Ease`, `Family` and `Repeat` are
`#[non_exhaustive]` enums, and `Tween` is `#[non_exhaustive]` config
with `Tween::new(from, to, duration)` and `const fn` builders
(`with_delay`, `with_ease`, `with_repeat`, `with_yoyo`). A `const fn`
can't drop a part of a generic value on stable Rust, so the builders
other than `with_ease` ask for `T: Copy` and `E: Copy`. Every `Lerp`
type is `Copy`, and so is `Ease`; a tween along a curve that isn't
(2135's `Points`) calls them before `with_ease`, or assigns the field.

With `yoyo`, every second play runs backwards in time, as CSS's
`alternate` and DOTween's `Yoyo` do: the curve is read at `ONE - p`, so
`In` going out is `Out` coming back.

## Consequences

- A simulation, a view and a skin share one set of curves, under the
  names agents know, and each is one line (`Tween::new(0, -6,
  300).with_ease(Ease::Out(Family::Back))`). Parked work (2362's camera
  blends, 2363's sound by distance, 2364's keyframed tracks) and After
  MVP work (2339, 2343) take their curves from here.
- Integer curves run on targets with no floating-point unit, and a
  test pins them: every curve at every `p` within 1/65536 of its
  formula (the polynomials and steps) or 2/65536 (the rest), and a
  golden table at 65 points.
- The engine's own motions stay written by hand until 2136, which keeps
  each to today's numbers.
- `Rgb` has two ways between colours: `Rgb::lerp` in `f32`, and
  `Lerp::mix` in integers. They agree at the ends; 2136
  says why the `f32` one stays.
- A curve that isn't in `Ease` is a type of the game's own implementing
  `Easing` (2135's `Points` is the engine's).
- `shuyi_core::ease` keeps its own quarter-wave table of 257 entries:
  2335's planned `Angle` table of 65 entries is too coarse for the
  bounds above (between entries a sine is off by up to about five
  units).

## Alternatives considered

- **Curves in `f32` in `shuyi-ui`, beside `ease_in_out`.** What there
  was. A simulation can't use them, they may differ in the last bits
  between machines, and a target with no floating-point unit pays for
  them in software.
- **Curves in `Fx`, 1119's fixed point.** 1119 is parked, and a
  view or a skin has no reason to know `Fx`. Plain `i32` in 1/65536 is
  `Fx`'s unit too, so 1119 implements `Lerp` for `Fx` when it lands.
- **A tween that stores its progress and is updated each frame**
  (Godot's `Tween`, DOTween). It breaks replays and rollback, needs the
  frame's time in the simulation, and can't be drawn at any moment
  without playing it there.
- **A time type (`Millis`, `Steps`).** A tween would need a version per
  unit, or a trait, for no check a game needs: the caller already knows
  its clock. ADR-0024's `Millis` stays the backend's.
- **Tables of 65 entries.** Smaller, but a sine is off by up to about
  five units between entries, outside the bounds in the tests.
- **CSS's `cubic-bezier` curves.** None of the engines compared names
  its curves by them, and they need a solver; a follow-up if a game
  asks.
- **An outside crate** (`keyframe`, `simple-easing`, `interpolation`).
  Each is `f32`, so none fits a simulation.
