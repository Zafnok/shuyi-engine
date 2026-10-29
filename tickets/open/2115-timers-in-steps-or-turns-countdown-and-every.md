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
- 1134 (done): the skill pack, `skills/`, with a `shuyi-stage-grid`
  skill and a `shuyi-stage-field` skill that waits for the tick
  simulation. Both say that rules count turns or steps and never read
  `dt`; this ticket names its timers there.

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
    wrote in milliseconds into steps, once, at load. A timer never
    counts milliseconds.
  - **`ms_from_steps(steps: u32) -> u64`**, a `const fn`, the other
    way: `steps * 1000 / STEPS_PER_SECOND` in `u64`, rounded down,
    widened with `as` as `steps_from_ms` is. At 60 steps a second that
    is 0, 16, 33, 50, … and exactly 1000 at step 60, so it never drifts
    (a whole `ms_per_step` of 16 or 17 drifts 4% or 2% a second). It
    never overflows (`u32::MAX` steps is 71 582 788 250 ms), and
    `steps_from_ms` of it gives `steps` back while it fits a `u32`.
    It is the engine's one rule from steps to ms: data timed in ms
    (a clip, a particle's life, a bone clip) is read at
    `ms_from_steps(step - start_step)`. Its docs say so, naming
    `frame_at`, `particles_at` and `pose_at` in plain backticks, as
    below for `shuyi-ui`'s items.
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
  - `shuyi_core` re-exports `Countdown`, `Every`, `STEPS_PER_SECOND`,
    `steps_from_ms` and `ms_from_steps` at its root, and the crate
    docs' first sentence names timers.
  - **Public types (ADR-0039):** the private fields and constructors
    above are its kind 8, so fields can be added without a break.
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
  - **Either order with 2364, 2104 and 2105 (all parked):** whichever
    of them is worked first writes `crossed` in this module (2104's
    and 2105's scopes write it when 2364 isn't done). If one of them
    is done, `crates/core/src/timer.rs` and its `timer/tests.rs`
    already hold `crossed`: add to both files, keep `crossed`, and the
    module docs name it too. If none is done, write both files new;
    the first of them to be worked adds `crossed` to them.
  - **Either order with 2103 and 2104 (both parked):** each reads its
    time since a start step at `ms_from_steps(step - start_step)` and
    takes no `steps_per_second` or `ms_per_step`. If one of them is
    done, it wrote `STEPS_PER_SECOND` (moved here, with the platform's
    re-export) and `ms_from_steps` in `crates/core/src/timer.rs` as this
    Scope says: keep them and their tests, and add the rest. If neither
    is done, nothing more to do (they use this ticket's).
  - **Either order with 2134 (parked):** if 2134 is done, nothing to
    do (it used the `ms_from_steps` of 2104, its blocker); if not,
    nothing to do (it uses this one).
  - **Either order with 2231 and 2105 (both parked):** `frame_at` and
    `pose_at` take a time in ms and leave the step to the caller. If
    2231 or 2105 is done, its function's docs gain a line: a tick game
    passes `shuyi_core::timer::ms_from_steps(step - start_step)`. If
    not, nothing to do (each writes that line).
  - **Either order with 1123 (parked):** if 1123 is done, nothing to do
    (it imports clips and times nothing); if not, nothing to do.
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
  what only looks." The bullet goes in the `base` part's `AGENTS.md`
  (`templates/shuyi-default/base`, 2501).
- **From 1134 (done)** (the skill pack): its `shuyi-stage-grid` and
  `shuyi-stage-field` skills (`skills/shuyi-stage-grid/SKILL.md`,
  `skills/shuyi-stage-field/SKILL.md`) say the same as the templates'
  bullet, in place of their line that rules count turns or steps and
  never read `dt`: `shuyi_core::timer`'s `Countdown` and `Every`,
  advanced by one each turn or once per step of `FrameInput::ticks()`.
  `check-skills` passes.
- **From 2802 (done):** the book's chapter 6, *Rules as commands*
  (`site/content/learn/book/rules-as-commands.md`), gets a short
  section on time in rules, with a snippet in `site/snippets/`.
- **Either order with 2004** (the guides): if 2004 is done,
  `docs/guides/top-down-grid.md`'s *In real time* section names
  `shuyi_core::timer`'s `Countdown` and `Every` where it says a turn
  game times its rules in turns. If not, nothing to do (2004 names
  them).
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
  from a start step (1123, 2231, 2103, 2104), and read it in ms with
  `ms_from_steps`;
- `TickSim`, `TickDriver` and records (1119); game speed and hit-stop
  (2123); tweens (2109);
- the real-time grid template and its snake example (2739, parked);
- the card-battle example (2737, parked), which counts weak turns with
  `Countdown` if this is done. **Either order with 2737:** if 2737 is
  done, its `u8` weak turns stay: nothing to do. If not, nothing to do
  (2737 uses `Countdown`, as its scope says);
- 2712's RTS cooldown, which stays a `u16` column for a thousand units;
- the platformer controller's coyote time and jump buffer (1127,
  parked), which may count with `Countdown` when it is worked;
- marks crossed between two times, `crossed` (2364, parked).

## Implementation steps

1. Read ADR-0003 §2 and §3, ADR-0024 §4,
   `crates/platform/src/tick.rs`, `crates/ui/src/input/frame.rs` and
   the module docs of `crates/core/src/hash.rs`.
2. Write `crates/core/src/timer.rs` (or add to it, if 2364, 2104 or
   2105 made it, or keep what 2103 or 2104 wrote there):
   `STEPS_PER_SECOND`, `steps_from_ms`, `ms_from_steps`, `Countdown`,
   `Every`, the module docs and the doctest. Then the docs lines of
   `frame_at` and `pose_at`, if 2231 or 2105 is done. Add
   `pub mod timer;` (if it isn't there) and the re-exports to
   `crates/core/src/lib.rs`.
3. Add the `shuyi-core` dependency to `crates/platform/Cargo.toml`, and
   swap the constant in `crates/platform/src/tick.rs` for the
   re-export. Its tests pass unchanged.
4. Write the unit and property tests in
   `crates/core/src/timer/tests.rs`.
5. Write the frame-rate test in `crates/platform/src/app/tests.rs`,
   beside `a_second_of_frames_steps_sixty_times_and_a_late_one_four`.
6. Write the rustdoc sentences, the templates' bullet and the two
   stage skills' lines. Then the book's section.
7. Run the gates, the `no_std` build and `cargo xtask check-no-std`
   included.

## Acceptance criteria

- [ ] `shuyi_core::timer` has `Countdown`, `Every`, `STEPS_PER_SECOND`,
      `steps_from_ms` and `ms_from_steps`, re-exported at the crate
      root, with no float (2015's float ban passes).
- [ ] `shuyi_platform::tick::STEPS_PER_SECOND` is a re-export of the
      core's, and the existing `TickClock` tests pass unchanged.
- [ ] `steps_from_ms` gives 0 for 0, 1 for 1 and for 16, 2 for 17, 3
      for 50, 15 for 250, 60 for 1000, and 257 698 038 for `u32::MAX`.
- [ ] `ms_from_steps` gives 0 for 0, 16 for 1, 33 for 2, 50 for 3,
      1000 for 60, and 71 582 788 250 for `u32::MAX`; and
      `steps_from_ms(ms_from_steps(s))` is `s` for every `s` up to
      257 698 037 (property test).
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
- [ ] The `shuyi-stage-grid` and `shuyi-stage-field` skills, and
      the book's chapter 6, name `shuyi_core::timer`, and
      `check-skills` passes.
- [ ] `shuyi-core`, `shuyi-ui` and `shuyi-platform` still build for
      `thumbv7em-none-eabihf`, and `cargo xtask check-no-std` passes.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `steps_from_ms` and `ms_from_steps` at the values above;
  `Countdown::advance` at its edges (0, exactly `remaining()`, past it, after it fired),
  `restart`, `new_ready`, `elapsed` and `Default`; `Every` with `n`
  below, at and past the period, a period of 0, `until_next` and
  `reset`; an out-of-range value read from RON.
- Property: split advances equal one advance, for both types; the fire
  counts; the RON round trip; `ms_from_steps` back through
  `steps_from_ms`.
- Integration: the 30 and 60 frames a second test through `App::frame`
  on the headless backend.
- Doctest: something that moves every 8 steps.

## Completion notes

*(Filled in by the session that completes the ticket.)*
