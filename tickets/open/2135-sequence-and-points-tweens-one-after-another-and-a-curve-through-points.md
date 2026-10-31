---
id: "2135"
title: "Sequence and Points in shuyi_core::ease: the tweens of one value one after another, and a curve through points"
type: feature
milestone: After MVP
model: implementation
effort: high
status: todo
blocked_by: ["2109"]
nick_input: none
completed:
---

# 2135 — Sequences of tweens and curves through points

## Context

The first of 2109's two follow-ups. 2109 builds `shuyi_core::ease`:
integer ease curves (`Ease`, the `Easing` trait), `Lerp`, and
`Tween<T, E>`, one value from `from` to `to`, drawn from the time
alone. It split two things out to stay one PR's worth (its step 9).
This ticket is those two, with the spec 2109's Scope gave them. The
other follow-up, 2136, moves the motions written by hand onto `ease`.

What the engines have that 2109 doesn't:

- **One value, step after step.** Godot's `Tween` runs its
  `tween_property` calls one after another unless it is set parallel,
  with `tween_interval` for a hold
  (docs.godotengine.org/en/stable/classes/class_tween.html). DOTween's
  `Sequence` has `Append` and `AppendInterval`. Defold's `go.animate`
  takes a delay and a function called when it is done. With 2109
  alone, a title that drops in, overshoots, holds and leaves is three
  tweens, and a caller that adds up their start times by hand.
- **A curve of a game's own.** GameMaker's Animation Curves and Unity's
  `AnimationCurve` let a designer draw the shape: a squash that goes
  past the end and settles, a pulse that comes back to where it began.
  2109's `Ease` names Penner's curves and nothing else.

2109's ADR (*Easing and tweens*, its points as 2109's Scope lists
them) already places both:

- point 2: a value is a function of the fields and the time since the
  start, and no progress is stored. A `Sequence` keeps that rule, so a
  simulation stores it and the step it started on, as it stores a
  `Tween`;
- point 5: curves and tweens are data inside other RON files, not a
  format. So is a `Points` list;
- point 6: a sequence orders the steps of one value, and
  `cinematic::Timeline` orders the shots of a cinematic. Neither
  replaces the other.

**Why under `shuyi_core::ease`, not at the crate root.** The parked
MIDI module plans `shuyi_midi::sequence::Sequence`, a song's tracks of
notes (2205, ADR-0014). Two `Sequence`s at the root of a game's
imports would clash, so this one is always `ease::Sequence`. `Points`
stays beside it: alone at the root, the name says nothing.

**Who will use it.** Small games made by agents (the end goal of
2026-10-19, `docs/decisions.md`): a score that counts up then pops, a
menu that slides in then settles, a title that holds then leaves.
Later, perhaps, the parked bone tools: 2105's keys take an `Ease`, and
2216 (DragonBones) and 2217 (Spine) read a Bézier key as `Linear` with
a warning. Keys that take a `Points` curve, so a Bézier keeps its
shape, would be a follow-up of 2105 (see *Out*).

Nick, 2026-10-20 (`docs/decisions.md`): the features most 2D games
expect come after the MVP, not parked. This is one of them, and it
needs nothing parked.

## Nick input

None.

## Scope

**In:**
- **`Sequence<T, E = Ease>`** in `shuyi_core::ease`
  (`crates/core/src/ease/sequence.rs`, its tests in
  `crates/core/src/ease/sequence/tests.rs`), `no_std` with `alloc`, and
  no float outside its tests. 2109's spec, kept: the tweens of one
  value, one after another. `Sequence::new(start)`, then
  `.then(to, duration, ease)` (from where the step before ended),
  `.wait(duration)` (a hold) and `.repeat(Repeat)`. `value_at(t)`,
  `length()` (one pass, the sum of its steps), `done_at()` and
  `finished_between(from, to)`, meaning what `Tween`'s mean, and
  `step_at(t) -> Option<(usize, u32)>`. Values that move together are
  several tweens started together: no parallel step. In full:
  - **The steps:** `pub enum SequenceStep<T, E = Ease> { To { to: T,
    duration: u32, ease: E }, Wait(u32) }`. It is not named `Step`,
    which would read like `Ease::Steps` beside it, and like a
    simulation's steps. `ease` may be left out in RON
    (`#[serde(default)]`), as in `Tween`, with the serde bounds
    `Tween` uses for it;
  - **The fields** are private: `start: T`, `steps:
    Vec<SequenceStep<T, E>>` and `repeat: Repeat` (2109's, `Once` by
    default). `steps` and `repeat` may be left out in RON. A sequence
    reads `(start: 40, steps: [To(to: 0, duration: 300, ease:
    Out(Back)), Wait(800), To(to: -40, duration: 200)])`;
  - **The builder:** `new(start)` has no steps. `then` and `wait` push
    a step and `repeat` sets the rule; each takes and returns `self`
    and is `#[must_use]`. `start(&self) -> T` and
    `steps(&self) -> &[SequenceStep<T, E>]` read them back;
  - **One pass:** a `To` step runs from the value the step before it
    ended on (`start` for the first) to `to`, drawn by a 2109 `Tween`
    with the step's numbers (from that begin value, with its `to`,
    `duration` and `ease`) at `u` into the step, so a step and a tween
    never disagree. It ends on that tween's value at `duration`: `to`
    for every `Ease`, and `T::mix(begin, to, ease.ease(ONE))` for a
    curve that ends elsewhere. A `Wait` holds the value it starts on.
    A step of length 0 takes no time: at its time, its end value holds
    already;
  - `length()`: the steps' durations added, saturating at `u32::MAX`;
  - **Passes:** `Once`, `Times(n)` (`Times(0)` acts as `Once`) or
    `Loop`, as for `Tween`. Each pass starts again from `start`. There
    is no yoyo and no delay: a leading `wait` is the delay, and a way
    back is written as steps;
  - `value_at(&self, t: u32) -> T`: the value at `t` into the pass
    that holds `t`. At and after `done_at`, the value the last pass
    ended on. With `Loop`, `t` is taken modulo `length()`, and a `Loop`
    of length 0 holds its end value;
  - `done_at(&self) -> Option<u32>`: `length() * plays`, saturating;
    `None` for `Loop`;
  - `finished_between(&self, from: u32, to: u32) -> bool`: `Tween`'s
    rule (true when `from < d <= to`, where `d` is `done_at` raised to
    at least 1), and never true for `Loop`;
  - `step_at(&self, t: u32) -> Option<(usize, u32)>`: the index of the
    step that holds `t`, a `Wait` included, and the time into it. A
    step holds the times from its start up to its end, not including
    the end, so a step of length 0 is never given, and at a boundary
    the next step is at 0. `None` at and after `done_at`, and when
    `length()` is 0. With `Loop`, modulo `length()`. A screen uses it
    to show which part is playing, or to act when a step starts;
  - it derives `Debug`, `Clone`, `PartialEq` and serde, and `Eq` when
    `T` and `E` have it, as `Tween` does (no `Copy`: it holds a `Vec`).
    It saves and hashes (`state_hash`) through serde.
- **`Points`** in `shuyi_core::ease` (`crates/core/src/ease/points.rs`,
  its tests in `crates/core/src/ease/points/tests.rs`). 2109's spec,
  kept: a curve through points for a game's own shape (GameMaker's
  Animation Curves, Unity's `AnimationCurve`, with straight segments):
  `(x, y)` pairs in thousandths, as `crates/assets/src/lighting.rs`
  counts fractions; `x` rising strictly from 0 to 1000, `y` from -1000
  to 2000; `Points::new(Vec<(u16, i32)>) -> Result<Points,
  PointsError>`, serde through it (`try_from`) so a bad list fails at
  load and says which rule it breaks; `Default` is the straight line;
  it implements `Easing`, so `Tween<T, Points>` works. In full:
  - **The rules**, checked in this order, and the first one broken is
    the error: at least two points; the first `x` is 0; each `x` is
    above the one before it and each `y` is in `-1000..=2000`, point by
    point from the start; the last `x` is 1000. So a list holds at most
    1001 points. The first and last `y` are free: a curve may end where
    it began (a pulse) or past 1000 (a curve that stays over);
  - **`PointsError`**: `TooFew`, `FirstX(u16)`, `NotRising { index:
    usize, x: u16 }`, `YOutOfRange { index: usize, y: i32 }` and
    `LastX(u16)`, with `index` from 0. It implements `Display`, one
    sentence that names the rule and the point ("the point at index 2
    has y = 2500, outside -1000 to 2000"), and `core::error::Error`;
  - **serde:** `#[serde(try_from = "Vec<(u16, i32)>", into =
    "Vec<(u16, i32)>")]`, as `Grid` reads through `RawGrid`
    (`crates/core/src/grid.rs`). In RON a curve is its list:
    `[(0, 0), (300, 1200), (1000, 1000)]`;
  - **The curve:** `Easing::ease(&self, p)` clamps `p` to `0..=ONE`,
    as every curve of 2109 does, finds the two points around it
    (`partition_point` on `x`), and draws the straight line between
    them, scaled from thousandths to 1/65536. It computes in `i128`
    and rounds once, as `Lerp` rounds (to nearest, halves up). At
    `p = 0`, at `p = ONE`, and at each point whose `x` is a multiple of
    125 (it falls on a whole 1/65536), it gives that point's `y`,
    scaled and rounded once;
  - `Default`: `[(0, 0), (1000, 1000)]`, which equals `Ease::Linear` at
    every `p`. `Tween`'s defaulted `ease` needs it;
  - `points(&self) -> &[(u16, i32)]` reads the list back;
  - it derives `Debug`, `Clone`, `PartialEq`, `Eq` and `Hash`.
- **2109's `Tween` with a curve of a game's own.** 2109 says that after
  its last play a tween holds "the value that play ended on", which for
  a `Points` curve that ends at `y = 0` is `from`. Check that
  `Tween::value_at` computes that end value through the curve
  (`T::mix(from, to, ease.ease(ONE))`, and the mirror of it after a
  yoyo play), not as `to` written out. If it writes `to`, change it:
  every `Ease` gives exactly `ONE` there (2109's criteria), so no
  `Ease` value and no test of 2109 changes.
- **Public types (ADR-0039):** `Sequence` and `Points` keep their
  fields private (kind 8: `Points` keeps the rules above, and
  `Sequence` is built step by step). `SequenceStep` and `PointsError`
  are `#[non_exhaustive]` enums (kind 6), so a kind of step or a rule
  can be added later without a break.
- **Names.** `Sequence`, `SequenceStep`, `Points` and `PointsError` are
  `pub use`d in `shuyi_core::ease` from its private `sequence` and
  `points` modules, and not re-exported at the crate root (*Context*).
  No type here is named `Curve`: 2122 takes that name.
- **Docs:** `shuyi_core::ease`'s module docs gain two short sections,
  "One value, step after step" and "A curve of your own". Each names
  the engines' words beside ours (Godot's chained tweens and
  `tween_interval`, DOTween's `Sequence.Append` and `AppendInterval`;
  GameMaker's Animation Curves, Unity's `AnimationCurve`), checked
  against their docs on the day (the PR gives the date), and says what
  differs: no parallel step, and straight segments only. Two doctests:
  a title's y offset that drops in with `Out(Back)`, holds and leaves,
  as a `Sequence<i32>`; and a squash, a `Tween<i32, Points>` through
  `[(0, 0), (300, 1200), (1000, 1000)]`. The crate docs
  (`crates/core/src/lib.rs`) name sequences of tweens and curves
  through points beside ease curves and tweens.
- **From 1134 (done):** its `shuyi-animate` skill's section on ease
  curves and tweens (`skills/shuyi-animate/SKILL.md`, which 2109
  writes) names `Sequence` and `Points`, with the title that drops in;
  `check-skills` passes.
- **From 2802 (done):** the book's section on ease curves and tweens
  (chapter 4, `site/content/learn/book/frame-and-skins.md`, which 2109
  adds) shows them, with a snippet in `site/snippets/`.
- **What changes for the game:** nothing. The API grows (CI's `api`
  job lists the additions), and no `Ease` or `Tween` value changes.
  The PR says so.

**Out (do not do):**
- moving the motions written by hand onto `ease` (2136);
- a parallel step or a group: values that move together are several
  tweens or sequences started at the same time (2109's spec);
- a yoyo, a delay field or a speed for a whole sequence: a leading
  `wait` is the delay, and a way back is written as steps;
- callbacks or events inside a sequence: a screen or a simulation
  reads `step_at` and `finished_between`, and acts itself;
- smooth curves through points (Catmull-Rom, Unity's tangents,
  GameMaker's smooth and Bézier channels) and CSS `cubic-bezier`:
  straight segments only (a follow-up if a game asks);
- one sequence whose steps mix `Ease` and `Points`: a sequence has one
  curve type `E`, and a game that needs both writes its own `Easing`
  enum over the two;
- curves in space and following them (2122, parked);
- MIDI's `Sequence` of notes (2205, parked, ADR-0014), which this one
  never names or uses;
- keys of 2105's rigs that take a `Points` curve, and keeping a Bézier
  read by the DragonBones (2216) or Spine (2217) import as one: 2105's
  keys take an `Ease`, and 2216 and 2217 read a Bézier as `Linear`
  with a warning whether or not this ticket is done (a follow-up of
  2105 if a game asks);
- a file format or an asset kind: both are data inside other RON files
  (2109's ADR);
- re-exports at the crate root (*Context*).

## Implementation steps

1. Read 2109's ADR and `crates/core/src/ease.rs` (`Easing`, `Lerp`
   and how it rounds, `Tween`, `Repeat`), ADR-0039,
   `crates/core/src/grid.rs` (`Grid`'s `try_from`), ADR-0014's
   `Sequence`, and the docs of Godot's `Tween`, DOTween's `Sequence`,
   GameMaker's Animation Curves and Unity's `AnimationCurve`. Note the
   date you read them.
2. Check how `Tween::value_at` finds its end value (*Scope*), and fix
   it if it writes `to` out, with a test.
3. Write `crates/core/src/ease/points.rs`: `Points`, `PointsError`,
   the rules, `Easing`, `Default` and serde. Add `mod points;` and its
   `pub use` to `crates/core/src/ease.rs`.
4. Write `crates/core/src/ease/points/tests.rs`: the unit, exhaustive,
   property and RON tests below.
5. Write `crates/core/src/ease/sequence.rs`: `SequenceStep`,
   `Sequence`, the builder, `length`, `done_at`, `value_at`,
   `finished_between` and `step_at`, with each `To` step drawn by
   `Tween`. Add `mod sequence;` and its `pub use`.
6. Write `crates/core/src/ease/sequence/tests.rs`.
7. Write the module docs' two sections, the two doctests and the crate
   docs' words.
8. Write the skill's lines, and the book's (the clauses above).
9. Run the gates, the `no_std` build included. Expect about 600
   changed lines.

## Acceptance criteria

- [ ] Inside each `To` step, `Sequence::value_at` equals a `Tween` from
      the value the step before ended on, with the step's `to`,
      `duration` and `ease`, at the time into the step (a property
      test over random sequences of `To` and `Wait` steps, lengths of
      0 included).
- [ ] `value_at` is `start` at 0 when the first step takes time, holds
      through a `Wait`, and is the end value at and after `done_at`.
      With `Times(n)`, `value_at(k * length() + u) == value_at(u)` for
      every `k < n` and `u < length()`. With `Loop` and a length above
      0, `value_at(t + length()) == value_at(t)` for every `t`.
- [ ] `length()` and `done_at()` saturate at `u32::MAX`, and no method
      of `Sequence` or `Points` panics on any input.
- [ ] Over any split of time from 0 into windows one after another,
      `finished_between` is true in exactly one window for a sequence
      that ends (one of length 0 included), and in none for `Loop`.
- [ ] `step_at` gives each step that takes time at its start with time
      0, never gives a step of length 0, and gives `None` at `done_at`
      and for a sequence of length 0.
- [ ] `Points::default()` gives exactly `Ease::Linear`'s value at every
      `p` from 0 to `ONE`. For a three-point curve, `ease` is within
      one unit (1/65536) of its `f64` line at every `p`. For any valid
      list, it lies between the scaled `y`s of the two points around
      `p`.
- [ ] Each rule of `Points::new` has a list that breaks only it and
      gets its `PointsError`, and a `Tween<i32, Points>` in RON with
      that list fails to load with that error's sentence.
- [ ] A `Tween<i32, Points>` whose curve ends at `y = 0` holds `from`
      after its play.
- [ ] A full `Sequence`, one written with only `start`, a `Points` and
      a `Tween<i32, Points>` read back equal from RON, and the
      `state_hash` of one fixed `Sequence<i32>` equals a constant in
      the tests.
- [ ] `Sequence` and `Points` are reached as `shuyi_core::ease::…`,
      not at the crate root.
- [ ] `shuyi-core` still builds for `thumbv7em-none-eabihf`, and
      `cargo clippy -p shuyi-core --lib -- -D clippy::float_arithmetic`
      passes.
- [ ] The two doctests pass. The `shuyi-animate` skill names
      `Sequence` and `Points`, and so does the book.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: a sequence with no steps; `then`, `wait`, `then` at each
  boundary; a step of length 0 first, in the middle and last;
  `Times(2)` and `Times(0)`; `Loop`, and a `Loop` of length 0;
  `step_at` at the boundaries; `t = u32::MAX` with durations that
  saturate. Each `PointsError` and its sentence; `p` below 0 and above
  `ONE`; a pulse curve that ends at 0; the list of all 1001 points is
  valid.
- Exhaustive: `Points::default()` against `Ease::Linear`, and a
  three-point curve against its `f64` line, at every `p`.
- Property: `To` steps equal `Tween`s; `value_at` is pure (the same
  twice); passes repeat; `finished_between` fires once over random
  windows; `step_at` agrees with `value_at` (a `Wait` it names holds
  its value); a random valid `Points` lies between its neighbours and
  reads back equal from RON.
- RON: a full `Sequence` and one with its defaults left out, a
  `Points`, a `Tween<i32, Points>`, and each bad list.
- Doctest: the title that drops in, holds and leaves; the squash.

## Completion notes

*(Filled in by the session that completes the ticket.)*
