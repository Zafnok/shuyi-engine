---
id: "2109"
title: "Ease curves in integers in shuyi_core::ease, tweens drawn from the time alone, and an ADR that takes easing out of the cinematic module"
type: feature
milestone: After MVP
model: opus-5.5
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2109 — Ease curves and tweens drawn from the time alone

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8, and the 2D engines GameMaker, Defold and Construct 3.
Every one of them has named ease curves:

- Godot's `Tween`: 12 transitions (`TRANS_LINEAR` to `TRANS_SPRING`) by
  four eases (`EASE_IN`, `EASE_OUT`, `EASE_IN_OUT`, `EASE_OUT_IN`),
  chained or in parallel
  (docs.godotengine.org/en/stable/classes/class_tween.html);
- Defold's `go.animate` with 41 easings: linear, and ten families by
  four modes;
- Construct's Tween behavior and GameMaker's Animation Curves;
- Bevy's `EaseFunction` in `bevy_math`, Unity's `AnimationCurve` and
  Unreal's Timeline curves. Bevy and Unity leave the tweens themselves
  to outside crates and packages (`bevy_tweening`, DOTween).

Most of them name their curves after Robert Penner's equations, as
easings.net lists them (`easeInOutCubic`). Those are the names agents
reach for.

What Shuyi has today:

- **One curve.** `shuyi_ui::cinematic::ease_in_out(p: f32)`
  (`crates/ui/src/cinematic.rs:160`), a smoothstep that `Pan::centre`
  uses. It is `f32` and in `shuyi-ui`, so a simulation can't use it
  (`AGENTS.md` rule 5: no floats), and `shuyi-core` can't reach it.
- **A pointer that promises more.** ADR-0004 §3 says "Easing curves
  come from the cinematic module", which has one curve. 1123 and 2104
  once left tweens on sprites out because "the cinematic easing covers
  them"; their Out lines now point at this ticket's `Tween`.
- **Motions written by hand,** with time in `f32` seconds in some and
  whole milliseconds in others:
  - `Rgb::lerp(self, other, t: f32)` (`crates/ui/src/frame/color.rs:37`);
  - the grid cursor's pulse, a cosine over `f32` seconds the cursor
    carries (`blink_t`, `crates/ui/src/grid/cursor.rs`);
  - walk frames every `WALK_FRAME_S` (`f32` seconds) beside idle frames
    every `IDLE_STEP_MS` (`u64` milliseconds), in
    `crates/ui/src/grid/walk.rs`;
  - music fades in `f32` seconds (`MusicState::update`,
    `crates/ui/src/audio.rs:459`), while `AudioManifest` stores
    `music_fade_ms: u32` (`crates/assets/src/audio.rs:48`);
  - the text box's `type_on(dt: f32, ...)`
    (`crates/ui/src/dialogue/text_box.rs:154`).

No ticket owns tweens. Parked tickets already wait for this one: 2362's
camera blends, 2363's fall-off of sound by distance and 2364's
keyframed tracks take their curves from `shuyi_core::ease`. After MVP
tickets 2339 and 2343 may use it, in either order; 2341's music fades
(done) move onto it with 2136.

**Why it matters.** Visions of Shuyi asks for no tweens. Its curves
above keep working, and this ticket changes none of them. Most other 2D
games do need them: a score that pops, a menu that slides in, a title
that bobs, a card that flips. With this ticket each is one line, under
the names agents know. That serves the end goal of 2026-10-19 (an agent
makes a small game "in not too many LOC", `docs/decisions.md`), whose
check measures turn-based games until real time is unparked. Integer
curves work in a tick simulation, in a view, and on targets with no
floating-point unit (the parked GBA and DS, ADR-0012).

Nick, 2026-10-20 (`docs/decisions.md`): the features most 2D games
expect come after the MVP, not parked. This is one of them. It needs
nothing parked.

## Nick input

None.

## Scope

**In:**
- **An ADR** (the `write-adr` skill, the next free number): "Easing and
  tweens: integer curves in `shuyi-core`, drawn from the time alone". It
  says:
  1. easing lives in `shuyi_core::ease`, in integers, `no_std`. It
     supersedes one sentence of ADR-0004 §3, "Easing curves come from
     the cinematic module". The rest of §3 stands. ADR-0004's `Status:`
     line gains "its §3 sentence on easing is superseded by ADR-NNNN",
     and two rows of `docs/adr/README.md` change: ADR-0004's, and the
     new ADR's, which is added;
  2. a tween stores no progress. Its value is a function of its fields
     and the time since it started, as ADR-0004 §3 says of clips. A
     simulation stores a tween and the step it started on; a view, the
     millisecond;
  3. a tween's time is a plain `u32` whose unit the caller picks: whole
     milliseconds in a view, steps in a tick simulation (ADR-0003 §2),
     turns in a turn game. There is no time type. Screens get each
     frame's time in seconds (ADR-0024 §3), so a view counts its own
     milliseconds, as `GridScene::clock_ms` does
     (`crates/ui/src/grid/scene.rs`);
  4. curves are named as easings.net and Godot name them;
  5. curves and tweens are data, not a format. They derive serde and
     sit inside other RON files (a cinematic, 2330's `.ui.ron`, a game's
     data). No asset kind and no extension, so ADR-0007 and ADR-0019 are
     untouched;
  6. a sequence of tweens (2135's `Sequence`) orders the steps of one
     value; `cinematic::Timeline` orders the shots of a cinematic. A
     shot may hold tweens for what moves inside it. Neither replaces the
     other;
  7. a skin that works in floats eases through `shuyi_ui::ease::ease_f32`,
     which goes through the integer curve, so the curve is the same on
     every machine.
- **`shuyi_core::ease`** (`crates/core/src/ease.rs`, its tables in
  `crates/core/src/ease/tables.rs` and its tests in
  `crates/core/src/ease/tests.rs`, as `hash` and `history` keep
  theirs), `no_std`, with no float outside its tests:
  - progress in 1/65536: `pub const ONE: i32 = 65_536`, the unit of
    2335's `Angle::UNIT` too. Progress going in is clamped to
    `0..=ONE`. Progress coming out is signed, because `Back` and
    `Elastic` overshoot;
  - `pub trait Easing { fn ease(&self, p: i32) -> i32; }`;
  - `pub enum Family { Sine, Quad, Cubic, Quart, Quint, Expo, Circ,
    Back, Elastic, Bounce }`, easings.net's ten;
  - `pub enum Ease { Linear, Smooth, In(Family), Out(Family),
    InOut(Family), OutIn(Family), Steps(u16) }`, `Copy`, `Eq`, `Hash`,
    serde, with `Linear` as its `Default`. In RON: `Linear`,
    `InOut(Cubic)`, `Steps(4)`.
    - `Smooth` is smoothstep, `3p^2 - 2p^3`, the curve of
      `cinematic::ease_in_out`.
    - `In`, `Out` and `InOut` follow easings.net's formulas. `Out(f)` is
      `ONE - In(f)(ONE - p)` for every family. `InOut(f)` is `In` over
      the first half and `Out` over the second, except for `Back` and
      `Elastic`, whose `InOut` formulas on easings.net have constants of
      their own (`c2 = c1 * 1.525`, `c5 = 2 * PI / 4.5`).
    - `OutIn(f)`, which easings.net lacks, is `Out` over the first half
      and `In` over the second, as Godot's `EASE_OUT_IN`.
    - `Steps(n)`: `n` equal jumps, each at the end of its step (CSS
      `steps(n, jump-end)`). `Steps(0)` acts as `Steps(1)`, which holds
      the start until the end: the "stepped" key of bone tools.
  - **How it computes:**
    - the polynomials (`Smooth`, `Quad` to `Quint`, `Back`, `Bounce`)
      in `i128`, rounded once. Their constants are exact fractions:
      `Back`'s `c1 = 1.70158` is `170_158 / 100_000`, and `Bounce`'s
      `7.5625` and `2.75` are `121 / 16` and `11 / 4`;
    - `Sine`, `Expo` and `Elastic` through two tables of 257 entries in
      1/65536 (a quarter of a sine wave, and `2^x` for `x` from 0 to 1),
      with straight lines between entries. Fewer entries miss the
      bounds in the acceptance criteria: with 65, a sine is off by up to
      about five units between entries. `Elastic` takes the angle of its
      sine in 1/2^24 of a turn or finer, for the same reason;
    - `Circ` through `u64::isqrt` (stable since Rust 1.84; the pin is
      1.98.1);
    - each table is a `const` array in `tables.rs`. A test rebuilds it
      in `f64` and fails if one entry differs, printing the table to
      paste.
  - **Either order with 2335** (an integer `Angle` and a quarter-wave
    table of 65 entries in `shuyi-core`): that table is too coarse for
    the bounds here, so `ease` keeps its own, private, whichever ticket
    is done first. The Completion notes say so, which answers 2335's
    clause on this table.
- **`Lerp`**, in `shuyi_core::ease`:
  - `pub trait Lerp: Copy { fn mix(from: Self, to: Self, p: i32) -> Self; }`.
    It is not called `lerp`, because `Rgb` already has an inherent
    `lerp` in `f32`;
  - `from + (to - from) * p / ONE`, rounded to nearest with halves up,
    in `i128`, so no `p` overflows;
  - `mix(a, b, 0) == a` and `mix(a, b, ONE) == b` exactly;
  - past `from` or `to` (an overshoot) it goes on in a line and stops at
    the type's bounds. For `u8`, a colour channel, that is 0 and 255;
  - for `i32`, `i64`, `u8`, `u16`, `u32` and `(i32, i32)`;
  - **`Rgb`** gets it in `shuyi-ui` (`crates/ui/src/frame/color.rs`),
    channel by channel. The existing `Rgb::lerp(f32)` stays.
- **`Tween<T, E = Ease>`**, in `shuyi_core::ease`, for `T: Lerp` and
  `E: Easing`, deriving `Debug`, `Clone`, `PartialEq` and serde, and
  `Copy` and `Eq` when `T` and `E` have them:
  - fields `from: T`, `to: T`, `delay: u32`, `duration: u32`, `ease: E`,
    `repeat: Repeat`, `yoyo: bool`. In RON, `delay`, `ease`, `repeat`
    and `yoyo` may be left out (`#[serde(default)]`: 0, the default
    curve, `Once`, `false`);
  - `Repeat { Once, Times(u16), Loop }`, `Once` by default: how many
    plays in all (`Times(0)` acts as `Once`). The names are those of
    2231's planned `shuyi_assets::anim::Repeat`, so the two read alike.
    They stay two types, because a clip loops by default and a tween
    plays once. With `yoyo`, every second play runs from `to` back to
    `from`. The delay comes once, before the first play;
  - `value_at(&self, t: u32) -> T`, where `t` is the time since the
    tween started: `from` before the delay, the eased value during a
    play, and after the last play the value that play ended on (for
    every `Ease`, `to`, or `from` after an even number of yoyo plays).
    A `duration` of 0 jumps to that end value at the end of the delay;
    with `Loop` it then holds `to`;
  - `done_at(&self) -> Option<u32>`: `delay + duration * plays`,
    saturating; `None` for `Loop`;
  - `finished_between(&self, from: u32, to: u32) -> bool`: true when
    `from < d <= to`, where `d` is `done_at` raised to at least 1, so a
    tween done at 0 fires in the first window that holds any time. A
    screen or a simulation passes the previous frame's or step's time
    and this one's, starting from 0, and fires "it landed" once, with
    no flag kept. Never true for `Loop`;
  - the docs say how a caller with a `u64` clock calls it:
    `u32::try_from(now.saturating_sub(start)).unwrap_or(u32::MAX)`.
- **Public types (ADR-0039):** `Ease`, `Family` and `Repeat` are
  `#[non_exhaustive]`, and `Tween` is `#[non_exhaustive]` with
  `Tween::new(from, to, duration)` and `with_delay`, `with_ease`,
  `with_repeat` and `with_yoyo` (`const fn` where the compiler allows),
  as the ADR asks of config a game fills in. So a curve or a field can
  be added later without a break.
- **Names.** `shuyi_core` re-exports `Ease`, `Easing`, `Family`, `Lerp`
  and `Tween` at its root, and its crate docs (`crates/core/src/lib.rs`)
  name ease curves and tweens. `Repeat` stays under `shuyi_core::ease`.
  No type here is named `Curve` (2122 takes that name for curves in
  space).
- **The name table**, in `shuyi_core::ease`'s module docs: each `Ease`
  beside Godot's `TRANS_*` and `EASE_*`, Bevy's `EaseFunction`, Defold's
  `go.EASING_*`, easings.net's name, and CSS where one matches
  (`linear`, `steps(n, jump-end)`; CSS's named curves are cubic Béziers
  and match none). It is checked against their docs on the day (the PR
  gives the date). Godot's `TRANS_SPRING` and Bevy's `Elastic(omega)`
  have no match here, and the table says so. A doctest shows a score
  that pops: a `Tween<i32>` of a y offset with `Out(Back)`.
- **`shuyi_ui::ease`** (`crates/ui/src/ease.rs`, `pub mod ease;` in
  `crates/ui/src/lib.rs`): `ease_f32(ease: &impl Easing, p: f32) -> f32`,
  for skins that work in floats. `p` is clamped to 0 to 1 (not a number
  counts as 0), turned into 1/65536 (rounded with `crate::float`, as the
  rest of the crate rounds without `std`), eased by the integer curve,
  and turned back. `cinematic::ease_in_out` keeps its body and its
  tests, so nothing the game draws changes. Its doc comment points at
  `Ease::Smooth`.
- **Tickets this one changes**, in the same PR:
  - **either order with 1119** (parked; `Fx` and `FxVec2` in
    `shuyi_core::fx`): if 1119 is done, add `Lerp` for `Fx` and `FxVec2`
    here, rounding as `Fx` does. If not, nothing to do: 1119's Scope
    implements them when it lands (its clause on 2109);
  - **from 2115 (done):** the module docs of `shuyi_core::timer`
    (`crates/core/src/timer.rs`) gain a line that points at
    `shuyi_core::ease`: a timer says when, a tween says how far;
  - **either order with 2369** (parked; a light cycle of named lights
    in `shuyi-assets`): if 2369 is done, its cycle file's stops gain
    `ease: Ease` (default `Linear`): the blend's `permille` goes
    through the curve before `Light::mix`, with a test and the
    format's row in `docs/formats.md` updated. If not, nothing to do
    (2369 adds `ease` when it is built, as its clause says);
  - **from 1134 (done):** its `shuyi-animate` skill
    (`skills/shuyi-animate/SKILL.md`) gets a section on ease curves and
    tweens (`shuyi_core::ease`: `Ease`, `Tween`, `Lerp`,
    `finished_between`); `check-skills` passes;
  - **from 2802 (done):** the book's chapter 4 (*The frame and skins*,
    `site/content/learn/book/frame-and-skins.md`) gets a section on
    ease curves and tweens drawn from the time alone
    (`shuyi_core::ease`), its code a snippet in `site/snippets/` that CI
    builds and tests.
- **Two follow-up tickets**, written before this one is worked (After
  MVP), each blocked by it:
  - **2135:** `Sequence` and `Points` in `shuyi_core::ease`, the tweens
    of one value one after another and a curve through points for a
    game's own shape;
  - **2136:** the motions written by hand moved onto `ease` (the grid
    cursor's pulse, music fades, `Pan`, and 2339's
    `transition::progress` if 2339 landed first), each kept to today's
    numbers, so no `frame-png` or scene snapshot moves and the game
    sees no change. It says why `Rgb::lerp`, `type_on` and the idle and
    walk frames stay as they are.
- **What changes for the game:** nothing. The API grows, and
  `ease_in_out` and `Rgb::lerp` stay as they are. The PR says so.

**Out (do not do):**
- `Sequence` and `Points` (2135), and moving the motions written by
  hand onto `ease` (2136): the two follow-ups above;
- CSS `cubic-bezier` curves, steps that jump at the start, and Godot's
  `TRANS_SPRING` (a follow-up each, if a game asks);
- timers, countdowns and cooldowns in steps or turns (2115). A generic
  queue of delayed events is no ticket's yet: 2115 leaves it out, and
  2706's `CommandQueue` (parked) is the nearest;
- screen transitions (2339), shake, flash and the reduced-motion
  setting (2340), music crossfades (2341) and ducking (2343). They may
  ease with this when they are built;
- camera blends and rooms (2362, parked), sound that fades with
  distance (2363, parked) and keyframed tracks in the cinematic file
  (2364, parked), which build on this;
- clips of frames (2231, parked) and particles (2103, parked);
- curves in space and following them (2122, parked);
- a file format or asset kind of its own.

## Implementation steps

1. Read ADR-0004 §3, ADR-0024 §3, `crates/core/src/lib.rs`,
   `crates/ui/src/cinematic.rs`, `crates/ui/src/frame/color.rs`,
   ADR-0039 (2013's: public types that can grow), and easings.net's
   formulas. Note the date you read them.
2. Write the ADR, ADR-0004's `Status:` line and the two index rows.
3. Write `ONE`, `Family`, `Ease` and `Easing`: the `In` curves first,
   then `Out` as their mirror, then `InOut` and `OutIn`, then `Steps`.
   Generate the tables with their test.
4. Write the exhaustive test against `f64` and the golden snapshot. Add
   `insta` to `shuyi-core`'s dev-dependencies from the workspace.
5. Write `Lerp` and its impls, then `Tween` and `Repeat`, with RON
   round trips.
6. Write `shuyi_ui::ease::ease_f32` and `Lerp` for `Rgb`.
7. Write the name table and the doctest, and the crate docs' line.
8. Make the ticket edits above.
9. Run the gates, the `no_std` build included. Expect about 600
   changed lines, not counting `tables.rs`, the snapshot and the ticket
   files. `Sequence` and `Points` are split out (2135) to keep it
   there.

## Acceptance criteria

- [ ] At every `p` from 0 to 65536, every curve (`Linear`, `Smooth`,
      `Steps(1)`, `Steps(3)`, `Steps(16)`, and each `Family` in all four
      modes) is within one unit (1/65536) of its `f64` formula for
      `Linear`, `Smooth`, `Steps` and `Quad` to `Quint`, and within two
      units for `Sine`, `Expo`, `Circ`, `Back`, `Elastic` and `Bounce`.
      The PR lists the worst error per curve.
- [ ] Every curve gives exactly 0 at `p = 0` and exactly `ONE` at
      `p = ONE`, and `p` below 0 or above `ONE` gives the same as 0 or
      `ONE`.
- [ ] A golden snapshot of every curve at 65 points (every 1024) is
      committed (`insta`, a new dev-dependency of `shuyi-core` from the
      workspace: Apache-2.0, which `deny.toml` allows, and already used
      by `shuyi-ui` and `shuyi-assets`).
- [ ] `mix(a, b, 0) == a` and `mix(a, b, ONE) == b` for every `Lerp`
      type. At `p = -ONE / 10` and `p = 11 * ONE / 10`, signed values
      and pairs go past their ends, and `u8` and `Rgb` stop at 0 and
      255.
- [ ] `Tween`: `value_at` is `from` before the delay and the end value
      at and after `done_at`. With `Loop`, `value_at(t + duration) ==
      value_at(t)` for every `t` at or after the delay. Over any split
      of time from 0 into windows one after another, `finished_between`
      is true in exactly one window for a tween that ends (a tween done
      at 0 included), and in none for `Loop`.
- [ ] Every `Ease` and a `Tween` read back equal from RON, and a
      `Tween` written with only `from`, `to` and `duration` reads with
      the defaults.
- [ ] `shuyi-core` and `shuyi-ui` still build for
      `thumbv7em-none-eabihf`, and `cargo clippy -p shuyi-core --lib --
      -D clippy::float_arithmetic` passes (2015's float ban in its
      `lib.rs` passes too).
- [ ] `ease_f32(&Ease::Smooth, p)` is within 2/65536 of
      `cinematic::ease_in_out(p)` across 0 to 1, and no `frame-png` or
      scene snapshot in the repository changes.
- [ ] The ADR is written, ADR-0004's `Status:` line is updated, and
      `docs/adr/README.md` has the new row and ADR-0004's changed one.
- [ ] If 1119 is done, `Fx` and `FxVec2` implement `Lerp`, and `mix`
      hits both ends exactly.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each curve at 0, one half and 1; `Steps` at its jumps and
  `Steps(0)`; `p` out of range; `Lerp` overshoot for each type; a
  `Tween` with a delay, with `Times(2)` and `yoyo`, with a zero
  duration, with `Loop`, and at `t = u32::MAX`; `ease_f32` on not a
  number and on input out of range.
- Exhaustive: every curve against `f64` at every `p`; each table
  rebuilt in `f64`.
- Property: `value_at` is pure (the same twice); `Loop` repeats;
  `finished_between` fires once over random windows; `Lerp` hits its
  ends.
- Snapshot: the golden table.
- RON: every `Ease`, a full `Tween` and one with defaults left out.
- Doctest: the score that pops.

## Completion notes

*(Filled in by the session that completes the ticket.)*
