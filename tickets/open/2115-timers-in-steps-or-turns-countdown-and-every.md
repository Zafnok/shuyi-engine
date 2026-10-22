---
id: "2115"
title: "Timers in steps or turns: Countdown and Every in shuyi_core::timer, and the step rate moved into the core"
type: feature
milestone: After MVP
model: sonnet-5
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2115 — Timers in steps or turns: Countdown and Every

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8, and the 2D engines GameMaker, Defold and Construct 3.
Each has a timer a game sets and forgets:

- Bevy's `Timer` (once or repeating) and `Stopwatch`, ticked by each
  frame's `Duration`
  (docs.rs/bevy/latest/bevy/time/struct.Timer.html);
- Godot's `Timer` node (`wait_time` in seconds, `one_shot`) and
  `SceneTree.create_timer`
  (docs.godotengine.org/en/stable/classes/class_timer.html);
- Unity's `Invoke`, `InvokeRepeating` and `WaitForSeconds`, and
  Unreal's `FTimerManager::SetTimer`, with a rate in seconds and a loop
  flag;
- Defold's `timer.delay`, Construct 3's Timer behavior, and GameMaker's
  alarms, which count frames.

They count seconds of a clock, or frames. Shuyi's rules can do
neither: a simulation has no clock and no floats (`AGENTS.md` rule 5,
ADR-0003 §2 and §3), and a frame is not a step (ADR-0024 §4). Its
timers count what its simulations count: steps, 60 a second, and
turns.

What Shuyi has today:

- **The time a screen sees.** `FrameInput::dt()`
  (`crates/ui/src/input/frame.rs:97`) is the frame's seconds as an
  `f32`. `FrameInput::ticks()` (line 114) is the fixed steps due this
  frame, counted by `TickClock` (`crates/platform/src/tick.rs`) and
  handed in by `Game::set_ticks` (`crates/ui/src/game.rs:241`,
  ADR-0024 §4). Outside tests, nothing in `crates/`, `examples/` or
  `templates/` reads `ticks()`.
- **The step rate** is `STEPS_PER_SECOND` in `shuyi_platform::tick`.
  A game's rules crate depends on `shuyi-core` and `shuyi-assets`, not
  on the platform (`templates/grid/crates/rules/Cargo.toml`), so it
  can't name it.
- **Clocks for presentation:** `HurryClock` (`crates/ui/src/hurry.rs`,
  `f32` seconds), `Ctx::clock_s` (`crates/ui/src/ctx.rs`, `f64`), and
  the grid cursor's pulse (`Cursor::tick(dt)`,
  `crates/ui/src/grid/cursor.rs`).
- **No timer type** in `shuyi-core` or `shuyi-ui`. The templates'
  `AGENTS.md` (`templates/blank`, `templates/grid`) say "no clock, no
  floats" for rules, but never how to time something in steps or turns.

The `dt` uses in `examples/hello-grid/src/screens/stage.rs` (line 289)
and `crates/ui/src/toy/grid.rs` (line 665) time a walk glide, the
cursor's pulse and an animation clock. That is presentation, which
ADR-0003 §2 allows ("Turn stages keep animating with `dt`"). They are
right, and they stay.

What is planned:

- 1119 (parked): `TickSim`, records and `TickDriver`, and no timers.
- 2706 (parked): `CommandQueue`, commands scheduled for a later step.
- 1127 (parked): coyote time and a jump buffer "at their exact step
  limits", which a controller would count by hand.
- 2712 (parked): an RTS weapon cooldown, a `u16` column of steps left.
- 1123, 2231, 2103 and 2104 key clips, particles and animation states on
  the step they started (`start_step`, `entered_step`). That is time
  since a start, which needs no timer.
- 2109 (After MVP): tweens drawn from the time alone.
- 2123 (parked): game speed, under which "timers in steps (2115)" are
  game time.
- 2364 (parked): `crossed`, the marks between two times, which goes in
  this ticket's `shuyi_core::timer`.
- 1134 (Engine: public): the skill pack, with a `shuyi-stage-grid` and
  a `shuyi-stage-field` skill (ADR-0005 §3). Its scope doesn't say yet
  how a rule is timed.

**Why it matters.** Visions of Shuyi is turn-based and counts its own
turns in its rules. Nothing in it changes. For other games, the end
goal of 2026-10-19 is the measure: an agent asked "make me a snake game
with Shuyi Engine" makes it for the web and the Playdate in few lines,
and every new API is judged against it (`docs/decisions.md`). Today the
only time a screen hands an agent is `dt`, so the agent writes
`acc += dt; if acc > 0.15 { ... }`. That ties the rules to the frame
rate (the Playdate holds 30 frames a second, ADR-0008; the web 60), and
replays, bots and the state hash stop agreeing. A small typed timer fed
by `ticks()` or by turns makes the deterministic way the short way.

**Its first users.** Until real time unparks, the agent check of the
end goal measures turn-based games (Nick, 2026-10-20,
`docs/decisions.md`). So the first users are turn games made with
`shuyi new grid`: a poison that lasts three turns, a skill that waits
two turns before it is ready again, a door that opens every fourth
turn. Then the real-time grid template and its snake example (2739,
parked), which moves every few steps with `Every`. This ticket lands
first, so that template never shows a rule timed by `dt`.

Nick, 2026-10-20 (`docs/decisions.md`): the features most 2D games
expect come after the MVP, not parked. This is one of them, and it
needs nothing parked: `ticks()` already works, and turns need no clock.

## Nick input

None.

## Scope

**In:**
- **`shuyi_core::timer`** (`crates/core/src/timer.rs`, its tests in
  `crates/core/src/timer/tests.rs` as `hash` and `history` keep
  theirs), `no_std`, with no float:
  - **`STEPS_PER_SECOND: u32 = 60`**, moved here from
    `shuyi_platform::tick`. `crates/platform/src/tick.rs` replaces its
    constant with `pub use shuyi_core::timer::STEPS_PER_SECOND;`, so
    the old path still works and there is one source. `shuyi-platform`
    reaches `shuyi-core` only through `shuyi-ui` today, so
    `crates/platform/Cargo.toml` gains `shuyi-core = { path =
    "../core", default-features = false }`, and its `std` feature adds
    `"shuyi-core/std"`. No new outside dependency. 2015's
    `tools/layers.ron` already lets `shuyi-platform` depend on
    `shuyi-core`, so `cargo xtask check-layers` passes.
  - **`steps_from_ms(ms: u32) -> u32`**, a `const fn`: whole steps at
    `STEPS_PER_SECOND`, rounded up, so any time above 0 is at least one
    step: `(ms * STEPS_PER_SECOND + 999) / 1000` in `u64`. The result
    always fits (`u32::MAX` ms is 257 698 038 steps). `u64::from` and
    `try_from` aren't `const`, so it widens and narrows with `as`, and
    the narrowing cast carries `#[expect(clippy::cast_possible_truncation,
    reason = "...")]` saying why it fits. It turns a length a designer
    wrote in milliseconds into steps, once, at load. A rule never sees
    milliseconds.
  - **`Countdown`**, one shot:
    - private fields `length: u32` and `left: u32`; derives `Debug`,
      `Clone`, `Copy`, `PartialEq`, `Eq`, `Hash`, `Serialize`,
      `Deserialize`, and `Default` (length 0, ready);
    - `new(length)` starts it (`left = length`). `new_ready(length)` is
      done already, for a cooldown whose first use goes through;
    - `advance(n) -> bool`: `left` goes down by `n` and stops at 0. It
      returns `true` only on the call that takes `left` from above 0 to
      0, so it fires once however far `n` overshoots, and `advance(0)`
      never fires. A length of 0 is ready at once and never fires;
    - `ready() -> bool` (`left == 0`), `remaining() -> u32`,
      `elapsed() -> u32` (`length - left`, saturating, for a bar that
      fills), `length() -> u32`, and `restart()` (`left = length`);
    - it is the cooldown too, and its docs show how:
      `if used && cooldown.ready() { ...; cooldown.restart(); }`, with
      `cooldown.advance(1)` each step or turn. There is no `Cooldown`
      type.
  - **`Every`**, repeating:
    - private fields `period: u32` and `since: u32`; the same derives
      as `Countdown` except `Default`;
    - `new(period)`. A period of 0 acts as 1, as 2109's `Steps(0)` acts
      as `Steps(1)`: `new` stores 1, and every method reads
      `period.max(1)`, so a 0 read from a file is safe too. Below,
      `period` means that;
    - `advance(n) -> u32`: how many times it fired. In `u64`,
      `total = since + n`; it fires `total / period` times and keeps
      `total % period`. The count saturates at `u32::MAX`;
    - `period() -> u32`, `until_next() -> u32` and `reset()`
      (`since = 0`). `until_next` is
      `period.saturating_sub(since).max(1)`: `period - since` in range,
      and 1 when `since` is out of range, because the next `advance(1)`
      fires then.
  - A value from a hand-edited save that is out of range (`left` above
    `length`, `since` at or above `period`) still behaves by the
    formulas above, and no method panics on it: every subtraction
    saturates. There is no check, and the docs say so.
  - The unit is the caller's: steps or turns in rules. A timer moves
    only when it is advanced, so a paused game's timers wait.
  - `shuyi_core` re-exports `Countdown`, `Every`, `STEPS_PER_SECOND`
    and `steps_from_ms` at its root, and the crate docs' first sentence
    names timers.
  - **Public types follow 2013** (`#[non_exhaustive]`, constructors) if
    2013 is done. If not, the private fields and constructors above
    already let fields be added without a break.
- **The module docs: two kinds of time.**
  - *Simulation time* is steps or turns. A tick game steps its rules
    once for each of `input.ticks()` and advances its timers by 1 in
    each step, so a timer fires on the same step at 30 and at 60 frames
    a second. A turn game advances by 1 where its turn or round ends,
    in its own `TurnRules::apply`. Timers live in the state the rules
    own, so they save, hash (`state_hash`) and replay with it.
  - *Presentation time* is `FrameInput::dt()`, `Ctx::clock_s` and
    `HurryClock`: glides, blinks, text typing out, a clock on screen.
    It may stay in floats (ADR-0003 §2 and §3).
  - The docs name `shuyi-ui`'s items (`FrameInput`, `Ctx::clock_s`,
    `HurryClock`) in plain backticks, not as intra-doc links:
    `shuyi-core` doesn't depend on `shuyi-ui`, so a link would break
    `cargo doc`.
  - Many timers: a `BTreeMap<K, Countdown>` in the state, never a
    `HashMap`, whose order changes the hash (`shuyi_core::hash`'s
    docs).
  - An event that happens later: a `Countdown` kept beside it, which
    sends it when it fires.
  - **Either order with 2109:** if 2109 is done, a line points at
    `shuyi_core::ease`: a timer says when, a tween says how far. If
    not, 2109 adds that line (its scope says so).
  - A doctest: something that moves every 8 steps, a loop `for _ in
    0..ticks` around `Every::advance(1)`, with `ticks` a plain `u32`
    standing in for `input.ticks()` (the doctest can't reach
    `shuyi-ui`, which depends on `shuyi-core`).
  - **Either order with 2364 (parked):** if 2364 is done,
    `crates/core/src/timer.rs` and its `timer/tests.rs` already hold
    its `crossed`: add to both files, keep `crossed`, and the module
    docs name it too. If not, write both files new; 2364 adds
    `crossed` to them.
- **The rustdoc of `FrameInput::dt()` and `FrameInput::ticks()`**
  (`crates/ui/src/input/frame.rs`): one sentence each. `dt` is for what
  only looks; rules count `ticks()` or turns, with
  `shuyi_core::timer`.
- **The templates' `AGENTS.md`** (`templates/blank/AGENTS.md`,
  `templates/grid/AGENTS.md`, and any other template under
  `templates/` when this is worked), a bullet under *The layers*: "Time
  a rule in turns or steps with `shuyi_core::timer` (`Countdown`,
  `Every`): advance it by one each turn in `apply`, or once per step of
  `input.ticks()`. Never time a rule with `input.dt()`, which is for
  what only looks." **Either order with 2501:** if 2501 is done, the
  bullet goes in the template part that writes `AGENTS.md` (`base`). If
  not, it goes in each template's file.
- **Either order with 1134** (the skill pack): if 1134 is done, its
  `shuyi-stage-grid` and `shuyi-stage-field` skills (ADR-0005 §3) say
  the same as the templates' bullet. If not, this PR adds a line to
  1134's scope: those two skills say to time rules with
  `shuyi_core::timer` (2115), never with `dt`.
- **Either order with 2802** (the book): if 2802 is done, chapter 6,
  *Rules as commands*, gets a short section on time in rules. If not,
  this PR adds "time in steps or turns (`shuyi_core::timer`, 2115)" to
  chapter 6's line in 2802's scope.
- **What changes for the game:** nothing. The API grows, and
  `shuyi_platform::tick::STEPS_PER_SECOND` keeps its path and value.
  The PR says so.

**Out (do not do):**
- a separate `Cooldown` type, or a `Stopwatch` (a `u32` the rules add
  to is one);
- a keyed `Timers<K>` collection: the docs show the `BTreeMap`;
- a generic queue of delayed events (`Delayed<E>`). 2706's
  `CommandQueue` (parked) schedules commands by step. If a game asks
  for a generic queue first, a follow-up ticket builds it, and 2706
  builds on it, not beside it;
- timers in seconds or milliseconds of a clock, in `shuyi-ui` or
  anywhere: presentation keeps `dt`;
- changing the `dt` uses in `examples/hello-grid` and
  `crates/ui/src/toy/grid.rs`;
- clips, particles and animation state machines: they keep counting
  from a start step (1123, 2231, 2103, 2104);
- `TickSim`, `TickDriver` and records (1119); game speed and hit-stop
  (2123); tweens (2109);
- the real-time grid template and its snake example (2739, parked);
- 2712's RTS cooldown, which stays a `u16` column for a thousand units;
- the platformer controller's coyote time and jump buffer (1127,
  parked), which may count with `Countdown` when it is worked;
- marks crossed between two times, `crossed` (2364, parked).

## Implementation steps

1. Read ADR-0003 §2 and §3, ADR-0024 §4,
   `crates/platform/src/tick.rs`, `crates/ui/src/input/frame.rs` and
   the module docs of `crates/core/src/hash.rs`.
2. Write `crates/core/src/timer.rs` (or add to it, if 2364 made it):
   `STEPS_PER_SECOND`, `steps_from_ms`, `Countdown`, `Every`, the
   module docs and the doctest. Add `pub mod timer;` (if it isn't
   there) and the re-exports to `crates/core/src/lib.rs`.
3. Add the `shuyi-core` dependency to `crates/platform/Cargo.toml`, and
   swap the constant in `crates/platform/src/tick.rs` for the
   re-export. Its tests pass unchanged.
4. Write the unit and property tests in
   `crates/core/src/timer/tests.rs`.
5. Write the frame-rate test in `crates/platform/src/app/tests.rs`,
   beside `a_second_of_frames_steps_sixty_times_and_a_late_one_four`.
6. Write the rustdoc sentences and the templates' bullet. Then, by
   whether each is done, the skill pack's lines or 1134's scope line,
   and the book's section or 2802's scope line.
7. Run the gates, the `no_std` build and `cargo xtask check-no-std`
   included.

## Acceptance criteria

- [ ] `shuyi_core::timer` has `Countdown`, `Every`, `STEPS_PER_SECOND`
      and `steps_from_ms`, re-exported at the crate root, with no float
      (2015's float ban passes).
- [ ] `shuyi_platform::tick::STEPS_PER_SECOND` is a re-export of the
      core's, and the existing `TickClock` tests pass unchanged.
- [ ] `steps_from_ms` gives 0 for 0, 1 for 1 and for 16, 2 for 17, 3
      for 50, 15 for 250, 60 for 1000, and 257 698 038 for `u32::MAX`.
- [ ] For any start value and any split of a total `n` into parts,
      advancing by each part ends in the same value as `advance(n)`.
      `Every` fires `(since + n) / period` times both ways. `Countdown`
      fires at most once, and exactly once when it was not ready and
      `n` is at least its `remaining()` (property tests).
- [ ] A `Countdown` restarted after it fired fires again after its
      length; `new_ready` is ready before any advance; a length of 0
      never fires. `Every::new(0)` acts as `Every::new(1)`, and so does
      an `Every` read from RON with a period of 0.
- [ ] An out-of-range value read from RON (`left` above `length`,
      `since` at or above `period`) panics in no method, and
      `until_next` is 1 for it.
- [ ] `Countdown` and `Every` read back equal from RON, and the
      `state_hash` of one fixed value of each equals a constant in the
      tests.
- [ ] A test in `crates/platform/src/app/tests.rs` runs two seconds of
      frames through `App::frame` at 60 frames a second (16, 17, 17 ms)
      and at 30 (33, 33, 34 ms). Each run steps 120 times, and an
      `Every::new(8)` advanced once per step fires on the same steps
      (8, 16, …, 120) in both.
- [ ] The doctest passes, and the rustdoc of `dt()` and `ticks()`, the
      module docs and every template's `AGENTS.md` say how to time a
      rule.
- [ ] 1134's two stage skills, or its scope, and 2802's chapter 6, or
      its scope, name `shuyi_core::timer`.
- [ ] `shuyi-core`, `shuyi-ui` and `shuyi-platform` still build for
      `thumbv7em-none-eabihf`, and `cargo xtask check-no-std` passes.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `steps_from_ms` at the values above; `Countdown::advance` at
  its edges (0, exactly `remaining()`, past it, after it fired),
  `restart`, `new_ready`, `elapsed` and `Default`; `Every` with `n`
  below, at and past the period, a period of 0, `until_next` and
  `reset`; an out-of-range value read from RON.
- Property: split advances equal one advance, for both types; the fire
  counts; the RON round trip.
- Integration: the 30 and 60 frames a second test through `App::frame`
  on the headless backend.
- Doctest: something that moves every 8 steps.

## Completion notes

*(Filled in by the session that completes the ticket.)*
