---
id: "2137"
title: "Real time's first step: TickSim, records that replay, and TickDriver with its input latch"
type: feature
milestone: "Engine: real time"
model: opus-5.5 / gpt-6.1-sol
effort: xhigh
status: done
blocked_by: ["2115"]
nick_input: none
completed: 2026-11-04
---

# 2137 — TickSim, records that replay, and TickDriver with its input latch

## Context

Split out of 1119 on 2026-10-30, when Nick unparked the demo chain
(snake, the maze chase and the A/B against Bevy, `docs/decisions.md`,
2026-10-30). 1119's own text said to do this: snake (2739) needs only
`shuyi_core::tick` and the driver, not `Fx` or `Arena`, so if snake
came before the game's free movement, `tick` and the driver (with its
latch) go in a ticket of their own. 1119 keeps `Fx` and `Arena`, stays
parked, and waits for this one.

ADR-0003 §2–§4. A game that steps in time keeps what makes this
codebase fast to work in (replays, headless tests, bots, and later
rollback networking, ADR-0006): the step is fixed (60 a second), and a
run is its seed plus its inputs.

The 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6 and
Unreal 5.8 found a rule missing here: which step gets a frame's presses.
Bevy's `just_pressed` read in `FixedUpdate` misses or repeats presses, a
known pitfall its fixed-timestep example avoids by gathering input
before the fixed loop. Unity's Input System has fixed and dynamic update
modes for it, and Godot keeps physics-frame bookkeeping so
`is_action_just_pressed` works in `_physics_process`. Without the rule,
a turn pressed on a frame with no step is dropped: the first bug a
player of a tick game meets (a snake that doesn't turn). A record stores
what each step saw, so replay tests can't catch it.

**Its first users:** the snake example and the arcade template part
(2739), then the maze chase (2776).

## Nick input

None.

## Scope

**In:**
- `shuyi_core::tick`: `trait TickSim { type Event; fn step(&mut self,
  input: &TickInput) -> Vec<Self::Event>; fn hash(&self) -> u64; }`.
  `TickInput` holds, per player, the action ids pressed, held and
  released (axes and the pointer are 1121's, Out). `TickRecord` holds a
  `Made` (2111: the engine version, the game's stamp and the content
  fingerprint), a seed and run-length encoded inputs. `replay(sim,
  record) -> Vec<u64>` returns hashes per step. `TickInput` and
  `TickRecord` may grow, so each is `#[non_exhaustive]` with a
  constructor (ADR-0039).
  - **Players are seats (2406).** `TickInput` holds one entry per seat,
    indexed by seat number from 0 (at most 4). Until 2406 is done there
    is one entry, seat 0, filled from the shared input. **Either order
    with 2406:** if 2406 is done, `TickDriver` fills one entry per
    filled seat from `FrameInput::seat(n)`, and a seat that joins or
    leaves changes the entries from the next step, so a `TickRecord`
    replays it; if not, 2406 does. The core names no `shuyi-ui` type:
    the index is a plain number.
  - **Records are stamped (2111, done).** `replay` takes the build's
    `Made` (`shuyi_core::Made`) and refuses a record whose `Made`
    differs from it, naming both (`Made::check`).
- `shuyi_ui::tick::TickDriver`: a screen reads the steps due as
  `FrameInput::ticks()` (`shuyi_platform::App`'s `TickClock`,
  `crates/platform/src/tick.rs`, ADR-0024 §4, at most 4 a frame), and
  `TickDriver` steps the simulation that many times. It keeps no clock
  of its own. The Harness can step exact counts (`harness.steps(n)`).
- **Input reaches the steps.** `TickDriver` keeps what each frame read
  until a step takes it, so the frame rate never changes which presses
  a simulation sees. Today `FrameInput` (`crates/ui/src/input/frame.rs`)
  holds one frame's `actions` (presses, then key repeats, in one list),
  `held`, and `ticks()`: 0 to 4 steps. At 120 Hz every other frame has
  no step, and frames of 16, 17 and 17 ms give some frames two
  (`TickClock`'s tests). Stepping straight from `actions` would drop a
  press read on a frame with no step and repeat it on a frame with
  several. The rules, for each seat's entry in `TickInput`:
  - Each action press is `pressed` in exactly one step: the first one
    run in or after the frame that read it. On a frame with several
    steps that is the first of them, a late frame capped at
    `MAX_STEPS` included. A frame that runs no step, for any reason
    (less than a step's time passed; `Game::act`, which hands in an
    action with no time passing; a slowed or stopped game speed,
    2123), keeps its presses for the next step.
  - Each release is `released` in exactly one step after its press's.
    A press and its release with no step between them show `pressed`
    in one step and `released` in the next, so a tap is never lost.
  - `held` is the frame's: every step a frame runs sees the actions
    held at its end. A tap is `pressed` without being `held`.
  - Key repeats never reach a step. `InputState::update` returns a
    frame's actions with the repeats after them in one list, and
    nothing says which actions were let go, so `FrameInput` gains the
    frame's presses without repeats and the actions released, for
    `TickDriver` to read.
  - The latch empties when its stage is covered (the stage clears it
    from `Screen::covered`) and when the game resumes after a suspend
    (`SystemEvent::Resume`, handled in `App::on_system_event`,
    `crates/platform/src/app.rs`), so a press made before a pause
    screen or a suspend never fires after it. Suspended frames already
    drop their input (`App::run`).
  - The same action pressed twice before a step: the session decides
    whether the second press merges into the first or waits for the
    next step, and writes the choice in `TickDriver`'s docs and the
    Completion notes.
  - `TickRecord` stores what each step saw after the latch, so a replay
    gives the same steps whatever frame rate made the record.
- **A step's events are handed on once.** `TickDriver` hands each
  step's events to the screen together with the step's number, and it
  keeps the highest step number it has handed on. When the game loads
  an older state (a rollback, 1147), `TickDriver::rewind_to(step)` sets
  the step count back without lowering that mark. The steps that run
  again are stepped for their state, but their events are not handed on
  a second time. A simulation asks for sound only through events, so no
  sound plays twice. Catching up (up to `MAX_STEPS` steps in one frame)
  hands on every step's events, in order. A seek (2357) needs no
  rewind: it builds a new game by a silent replay from the start.
- A tiny example sim in the engine's tests, in integers only: a dot that
  moves a pixel a step while a direction is held, speeding up by one
  pixel a step every 8 steps held (an `Every`, 2115) up to 4, and
  stopping when let go. When the dot stops, that is an event, and the
  example screen turns it into a sound cue. 1119 later moves it onto
  `FxVec2` with inertia.
- **The debug view (2316).** **Either order with 2316:** if 2316 is
  done, its step runs one fixed simulation step here, and its readout
  names the step number; if not, 2316 does this.
- **Debug commands (2356).** **Either order with 2356:** if 2356 is
  done, a tick screen's `Screen::debug_command` queues the game's debug
  inputs for the next step: `TickInput` carries them (the game's own
  type, never a `DebugCall`), so a `TickRecord` holds them and `replay`
  reproduces them. Repeat 2356's replay-hash test on the example sim. If
  2356 isn't done, nothing is added here, and 2356 adds it (its Out says
  so).
- **The ECS (2108).** Decide, and write in the Completion notes,
  whether any example needs an ECS (ADR-0003 §4). Default: no. A game
  that wants one has the opt-in `shuyi-ecs` module (2107). **Either
  order with 2108:** if 2108 is done, its PR wrote a parked follow-up
  ticket for its tick adapter, `EcsTickSim`, blocked by this one, and
  this PR adds nothing for it; if not, 2108 writes the adapter itself.
- **The skill pack.** Add a rule to `skills/shuyi-input/SKILL.md`, not
  to `input-actions`, which keeps only what holds in this repository
  (1134, *One source for what is true of every game*): "**Presses reach
  fixed steps once.** A tick stage reads input from `TickInput`, never
  from `FrameInput::actions`: `TickDriver` keeps a frame's presses and
  releases until a step runs, gives each to exactly one step, and never
  gives a key repeat to a step (2137)." `skills/shuyi-stage-field`
  keeps 1119 in its `waits_for`: free movement needs `Fx`, so this
  ticket doesn't change it. The arcade skill that teaches the tick path
  is 2739's.
- **From 2003 (done):** the `fixed-ticks` answer of the time concern in
  `docs/modules.ron` is parked and names 1119. Set it to ready, naming
  this ticket.

**Out (do not do):**
- `Fx`, `FxVec2` and `Arena`: 1119 (parked, blocked by this ticket);
- collision (1122) and world layers (1120);
- overlays that let a stage keep running, such as a notice or a toast
  (2367, parked). A pause menu pushed over a stage stays modal, the
  default, and stops it;
- networking, and checking a tick game's submitted score on 1148's
  server (its `TickRecord` re-simulated). **Either order with 1148:**
  if 1148 is done, its PR wrote a parked follow-up ticket for that
  check, blocked by this one, and this PR adds nothing for it; if not,
  nothing to do (1148 checks a `TickRecord` itself);
- thousands of units (the parked 2705);
- jump buffering and coyote time (1127);
- hold or toggle per action (2404);
- axes and the pointer in `TickInput` (1121 fills them, with the latest
  values each step);
- command sims (2706's `CommandDriver` already schedules each command
  for a later step);
- game speed and hit-stop (2123): the speed scales the time `App`'s
  `TickClock` owes, so `TickDriver` keeps stepping `FrameInput::ticks()`
  times and needs nothing for it; hit-stop is simulation state (2123's
  `Freeze`);
- timers and the step rate: `shuyi_core::timer` (2115) has `Countdown`,
  `Every`, `steps_from_ms` and `STEPS_PER_SECOND`. `shuyi_core::tick`
  and the example sim time things with them and add no timer of their
  own;
- a tick replay in the tool contract
  (`shuyi_cli::contract::replay::ticks`, 2739);
- a soak of tick sims (2549, parked, blocked by this ticket and 2527).

## Implementation steps

1. Read ADR-0003 §2–§4, ADR-0024 §4, ADR-0039, `crates/platform/src/tick.rs`,
   `crates/ui/src/input/frame.rs`, 2115's `shuyi_core::timer` and
   2111's `Made`. Check which of 2108, 2316, 2356, 2406 and 1148 are done
   (the either-order clauses).
2. `TickSim`, `TickInput`, `TickRecord` and `replay` in
   `crates/core/src/tick.rs`, with property tests. The example sim's
   `TickRecord` and its hash per step go in a golden table,
   `crates/core/tests/golden/tick.ron`, through 2116's
   `shuyi_assets::golden` (a dev-dependency on `shuyi-assets` with
   `default-features = false, features = ["golden"]`, so the no-std run
   stays without `std`). CI's `test` job checks it on x64 and ARM64,
   and its `determinism` job on wasm32, which runs `shuyi-core`'s
   tests.
3. `FrameInput`'s presses without repeats and released actions, then
   `TickDriver` with the latch (Scope, *Input reaches the steps*), its
   events handed on once and `rewind_to`, and Harness support.
4. The example sim and its screen, the either-order clauses, the ECS
   note, the skill rule and `docs/modules.ron`.
5. The book (2802): a section on fixed steps in the chapter on time, or
   a `Book:` line.

## Acceptance criteria

- [x] Replaying a record twice gives identical hash lists (property
      test over random inputs).
- [x] The example record's hashes match their golden table in CI's
      `test` and `determinism` jobs (x64, ARM64, wasm32).
- [x] The Harness steps the example sim headlessly; a snapshot of its
      state passes.
- [x] Harness sweep: one script of presses and releases, each event
      delivered with the first frame at or after its time through
      `Harness::frame`, at 30, 60, 61, 144 and 240 Hz and with jittered
      frame times, gives at every rate the same presses and releases in
      the same order, each in exactly one step, and no press later than
      the first step run in or after the frame that read it.
- [x] A tap read on a frame with no step is `pressed` in the next step
      and `released` in the one after; a press on a late frame that runs
      4 steps is `pressed` in the first only; a key repeat is never
      `pressed` in a step; an action handed in by `Game::act` reaches the
      next step.
- [x] A press latched on a frame with no step, just before the stage is
      covered by another screen, doesn't reach a step after the stage is
      uncovered.
- [x] Run the example sim for 10 steps, rewind it to its state at step
      5, and run it to step 10 again. The events of steps 6 to 10 are
      handed on once: the Harness's `audio_log` shows each cue once.
- [x] The `shuyi-input` skill states the rule, and `check-skills`
      passes.
- [x] `docs/modules.ron`'s `fixed-ticks` answer is ready and names 2137,
      and `cargo xtask check-modules` passes.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Property: replay determinism; for random frame times (0 to 100 ms)
  and random scripts of presses and releases, no press or release is
  lost or doubled.
- Integration: the Harness drives the example sim; the frame-rate sweep
  above; the rewind test above.
- Golden: the example record, natively and on wasm32 (2116).

## Completion notes

- **`shuyi_core::tick`** (`crates/core/src/tick.rs`): `TickSim` as the
  Scope writes it; `ActionId(u16)`, an action by its place in
  `ActionSet::ALL`; `SeatInput` (`pressed`, `held`, `released`) and
  `TickInput` (one `SeatInput` per seat, at most `MAX_SEATS`, 4), each
  `#[non_exhaustive]` with `new` (and `TickInput::one` for seat 0);
  `TickRecord` (private fields: `new(made, seed)`, `push`, `inputs`,
  `runs`, `len`, `truncate`), run-length encoded as `Run { steps, input }`;
  and `replay`. Re-exported at the crate root but `replay`.
- **`replay(sim: &mut S, record, made) -> Result<Vec<u64>, Mismatch>`**:
  it takes the state the caller built from the record's seed (as 2739's
  contract `replay::ticks` expects), refuses a record whose stamp isn't
  `made` before anything steps (`Made::check`), gives one hash per step
  (none for the start) and leaves `sim` at the record's end.
- **The example sim** is `shuyi_core::toy::Dot`, behind a new `toy`
  feature of `shuyi-core` (`shuyi-ui`'s `toy` turns it on), not a file
  under `tests/`: core's golden table and ui's Harness tests both step it.
  Its directions are the sample actions' first four ids. Stopping is its
  event (`DotEvent::Stopped`); the toy game's stage
  (`shuyi_ui::toy::tick::DotStage`, new) plays it as the cue `stop`, added
  to the toy's manifest, and opens the toy's `Menu` on Cancel.
- **The golden table**, `crates/core/tests/golden/tick.ron`: the record's
  text (with a fixed stamp, so it never moves at a release), the hash
  before any step and after each of its 44 steps, and where the dot ends.
  Checked by `crates/core/tests/golden.rs`, with a dev-dependency on
  `shuyi-assets` (`default-features = false, features = ["golden"]`).
- **`FrameInput`** gains `presses()` (the frame's presses without the key
  repeats), `released()`, `held()`, `with_presses`, `resumed()` and
  `with_resumed`. `InputState` gains `presses()` and `released()`: an
  action is released when no key or button holds it any more, a chord
  button's own action given on its release is a tap (pressed and
  released at once), and `set_keymap` now reports what it let go as
  released. `FrameInput::new` treats its actions as presses, and the ones
  not held as taps, so `Game::act`'s action is a tap for a tick stage.
- **Resume:** `App::on_system_event` calls the new `Game::set_resumed` on
  `SystemEvent::Resume`, and the next frame's `FrameInput::resumed()` is
  true; `TickDriver` clears its latch on it.
- **`shuyi_ui::tick`** (`crates/ui/src/tick.rs`): `TickDriver<A>` with
  `new`, `recording(TickRecord)`, `update(&mut sim, &input) -> Vec<(u64,
  Event)>` (events with their step numbers, from 1), `clear`,
  `rewind_to`, `steps`, `handed`, `record` and `take_record`; and
  `ActionIds`, `id()` and `from_id()` for every `ActionSet`.
- **Pressed twice before a step: the second press waits.** It isn't
  merged. The next step has the first press, the one after has its
  release and the press again (a step may show an action released and
  pressed, never pressed and released), and so on: every press and
  release reaches a step, in order. Merging would lose a fast double tap
  in a late frame or in a slowed game. The cost: a press of an action
  that already has an edge waiting lands a step later than the first
  step after its frame. Written in `TickDriver`'s docs.
- **The latch works on actions, not keys:** a second key on an action
  already held presses nothing; a release of a press no step saw (a key
  held since before the stage opened, or across a `clear`) is dropped;
  a press a step had stays down across a `clear`, so its release still
  comes; and an action down but no longer held is let go.
- **Harness support:** `Harness::steps(count)` runs frames until exactly
  `count` steps were due, reading the new `App::ticks()` (the steps the
  last frame handed the game).
- **Tests:** core's unit and property tests (`tick/tests.rs`: records,
  text, `replay` and its stamp, the dot), the golden table; the latch's
  unit tests (`crates/ui/src/tick/tests.rs`); `InputState`'s presses and
  releases (`input/tests.rs`, `chord_tests.rs`); and the Harness tests in
  `crates/ui/tests/it/tick.rs`: the sweep at 30, 60, 61, 144 and 240 Hz
  and jittered frames, the property test over random frame times (0 to
  100 ms) and scripts, a tap on a frame with no step, a late frame of 4
  steps, key repeats, `Game::act`, cover, suspend, the rewind with
  `audio_log`, and an insta snapshot of the dot's state. Broken on
  purpose (a release in its press's step, presses dropped on frames with
  no step, no clear on cover), the sweep, the property test and the
  cover test each fail.
- **Either order:** 2108, 2316, 2356, 2406 and 1148 aren't done, so each
  takes its part; their clauses now say what holds ("From 2137 (done)"),
  as do 2357, 2549 and 2904. 1121 and 1147 name the latch and
  `rewind_to` where they now live. For 2356, `TickInput` isn't generic:
  its clause says carrying the game's debug inputs is its choice (a
  field or a type parameter).
- **The ECS (2108):** no example needs one. The dot is one struct;
  snake and the maze chase are tens of things. 2108's clause now has it
  write `EcsTickSim` over `TickSim`.
- **Docs:** rustdoc of every item; the `shuyi-input` skill's rule 8; the
  `shuyi-stage-field` skill no longer says fixed ticks are parked, and
  points at `TickSim`, `TickDriver` and `TickRecord` (its `waits_for`
  keeps 1119); `docs/modules.ron`'s `fixed-ticks` is ready and names
  2137; the book's chapter 6 has a section *Rules in fixed steps*, with a
  bee over the garden (`Bee`, `site/snippets/src/rules.rs`) and its
  record replayed.
- **Not checked through the MCP session on Hello, Grid:** it is a turn
  game, and nothing a player sees in it changed. The Harness tests drive
  the toy's tick stage instead.
- **Follow-up:** 2411, a key let go while the game is suspended stays
  held after the resume (found here: `App::run` drains a suspended
  frame's input, so `held` keeps the key and a tick stage keeps moving).
- For the game: nothing changes. The API only grows, and `set_keymap`'s
  new releases reach nothing it reads. No pin bump is needed.
- No break: no `Breaking:` line. The book is changed, so no `Book:` line.
