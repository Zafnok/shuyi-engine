---
id: "2137"
title: "Real time's first step: TickSim, records that replay, and TickDriver with its input latch"
type: feature
milestone: "Engine: real time"
model: reasoning
effort: xhigh
status: todo
blocked_by: ["2115"]
nick_input: none
completed:
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

- [ ] Replaying a record twice gives identical hash lists (property
      test over random inputs).
- [ ] The example record's hashes match their golden table in CI's
      `test` and `determinism` jobs (x64, ARM64, wasm32).
- [ ] The Harness steps the example sim headlessly; a snapshot of its
      state passes.
- [ ] Harness sweep: one script of presses and releases, each event
      delivered with the first frame at or after its time through
      `Harness::frame`, at 30, 60, 61, 144 and 240 Hz and with jittered
      frame times, gives at every rate the same presses and releases in
      the same order, each in exactly one step, and no press later than
      the first step run in or after the frame that read it.
- [ ] A tap read on a frame with no step is `pressed` in the next step
      and `released` in the one after; a press on a late frame that runs
      4 steps is `pressed` in the first only; a key repeat is never
      `pressed` in a step; an action handed in by `Game::act` reaches the
      next step.
- [ ] A press latched on a frame with no step, just before the stage is
      covered by another screen, doesn't reach a step after the stage is
      uncovered.
- [ ] Run the example sim for 10 steps, rewind it to its state at step
      5, and run it to step 10 again. The events of steps 6 to 10 are
      handed on once: the Harness's `audio_log` shows each cue once.
- [ ] The `shuyi-input` skill states the rule, and `check-skills`
      passes.
- [ ] `docs/modules.ron`'s `fixed-ticks` answer is ready and names 2137,
      and `cargo xtask check-modules` passes.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Property: replay determinism; for random frame times (0 to 100 ms)
  and random scripts of presses and releases, no press or release is
  lost or doubled.
- Integration: the Harness drives the example sim; the frame-rate sweep
  above; the rewind test above.
- Golden: the example record, natively and on wasm32 (2116).

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
