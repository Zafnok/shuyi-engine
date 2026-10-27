---
id: "1119"
title: "Parked: Real time: a fixed-step tick simulation with fixed-point numbers, an entity arena and input records that replay"
type: feature
milestone: "Engine: real time"
model: opus-5.5
effort: xhigh
status: blocked
blocked_by: ["game#1131", "2116"]
nick_input: none
completed:
---

# 1119 — Tick simulation: fixed steps, fixed-point, records that replay

> **Parked** (Nick, 2026-10-14: "document the parked (i.e. after MVP/parity) tickets for networking, styles for later"; 2026-10-20: "unless these real-time work tickets overlap with visions of shuyi they should be parked until visions of shuyi is complete"): after Visions of Shuyi launches, not only after the MVP (`docs/decisions.md`, 2026-10-20). Unparked early only if the game needs it: its ticket 0060 says the army walks around freely (then 1119–1125 become game work), or it asks for a piece of this through its `engine-change` skill, which is then split out and unparked alone. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

ADR-0003 §2–§4. Top-down free movement, side-scrolling and real-time
isometric games need a simulation that steps in time. To keep what
makes this codebase fast to work in (replays, headless tests, bots, and
later rollback networking, ADR-0006), the step is fixed (60 a second),
numbers are integer fixed-point, and a run is its seed plus its inputs.

The 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6 and
Unreal 5.8 found a rule missing here: which step gets a frame's presses.
Bevy's `just_pressed` read in `FixedUpdate` misses or repeats presses, a
known pitfall its fixed-timestep example avoids by gathering input
before the fixed loop. Unity's Input System has fixed and dynamic update
modes for it, and Godot keeps physics-frame bookkeeping so
`is_action_just_pressed` works in `_physics_process`. Without the rule,
a turn pressed on a frame with no step is dropped: the first bug a
player of a tick game meets (a snake that doesn't turn, for the end goal
of 2026-10-19). A record stores what each step saw, so replay tests
can't catch it.

It needs the engine's code in this repository (the game's ticket 1131).

**Its first users.** The snake example and the arcade template part
(2739, parked with this ticket) need only `shuyi_core::tick`
(`TickSim`, `TickInput`, `TickRecord`, `replay`) and the driver, not
`Fx` or `Arena`. If Nick unparks snake before the game needs free
movement, split `tick` and the driver (with its latch) out of this
ticket into a ticket of their own first, and leave `Fx` and `Arena`
here.

## Nick input

None (technical). Scheduling: see above.

## Scope

**In:**
- `shuyi_core::fx`: `Fx` (i32, 8 fraction bits), `FxVec2`, with
  arithmetic that rounds toward negative infinity, saturating where
  documented, `from_px`, `floor_px`, and sine and cosine in `Fx` from
  2335's `shuyi_core::angle` (its table shifted from 16 fraction bits
  to `Fx`'s 8, rounding toward negative infinity), not a table of its
  own.
  - **Either order with 2335:** if 2335 isn't done when this starts,
    write `shuyi_core::angle` as 2335's scope says (the `Angle` type and
    its table only), and 2335 builds on it.
  - **Either order with 2109:** if 2109 is done, implement its
    `shuyi_core::ease::Lerp` for `Fx` and `FxVec2` here, rounding as
    `Fx` does, with a test that `mix` hits both ends exactly; if not,
    2109 adds them.
  - **Either order with 2118:** if 2118 is done, add a one-line
    `shuyi_procgen::noise::to_fx`, a noise value as an `Fx` (`n >> 8`),
    with a test; if not, nothing to do (2118 adds it).
- `shuyi_core::arena`: `Arena<T>` with generational `Id<T>`, insert,
  remove, get, iteration in id order, serde, and a text dump.
- `shuyi_core::tick`: `trait TickSim { type Event; fn step(&mut self,
  input: &TickInput) -> Vec<Self::Event>; fn hash(&self) -> u64; }`.
  `TickInput` holds, per player, the action ids pressed, held and
  released, axes as `i16`, and the pointer in world pixels (filled by
  1121; empty until then). `TickRecord` holds a `Made` (2111: the
  engine version, the game's stamp and the content fingerprint), a seed
  and run-length encoded inputs. `replay(sim, record) -> Vec<u64>`
  returns hashes per step.
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
- `shuyi_ui::tick::TickDriver` holds a screen's accumulator: at most 4
  steps a frame, then it slows down rather than spiral. The Harness can
  step exact counts (`harness.steps(n)`). *Since ticket 3003 the
  accumulator is `shuyi_platform::App`'s (`shuyi_platform::tick`,
  ADR-0024 §4): a screen reads the steps due as `FrameInput::ticks()`,
  so `TickDriver` steps the simulation that many times and keeps no
  clock of its own.*
- **Input reaches the steps.** `TickDriver` keeps what each frame read
  until a step takes it, so the frame rate never changes which presses
  a simulation sees. Today `FrameInput` (`crates/ui/src/input/frame.rs`)
  holds one frame's `actions` (presses, then key repeats, in one list),
  `held`, and `ticks()`, the steps due: 0 to 4 from
  `shuyi_platform::tick::TickClock`. At 120 Hz every other frame has no
  step, and frames of 16, 17 and 17 ms give some frames two
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
  - If 1119 is split for size, the latch ships in the same PR as
    `TickDriver`: a driver without it would save records with lost
    presses.
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
- A tiny example sim in the engine's tests: a dot moved by input with
  inertia. When the dot stops, that is an event, and the example screen
  turns it into a sound cue.
- **The debug view (2316).** If 2316 is done, its step runs one fixed
  simulation step here, and its readout names the step number. If 2316
  isn't done, 2316 does this.
- **Debug commands (2356).** If 2356 is done, a tick screen's
  `Screen::debug_command` queues the game's debug inputs for the next
  step: `TickInput` carries them (the game's own type, never a
  `DebugCall`), so a `TickRecord` holds them and `replay` reproduces
  them. Repeat 2356's replay-hash test on the example sim. If 2356 isn't
  done, nothing is added here, and 2356 adds it (its Out says so).
- **The skill pack. From 1134 (done):**
  `skills/shuyi-stage-field/SKILL.md` lists 1119 in its `waits_for`,
  and `check-skills` fails once this ticket is in `done/`: drop 1119
  from that list, and teach this ticket's work in the skill (fixed
  steps, `Fx`, `TickSim`, records and `replay`, and `TickDriver` with
  its latch). If that empties the list, write the skill's worked
  example, `skills/shuyi-stage-field/example.ron`, which CI's `skills`
  job runs on a fresh `shuyi new` game. Step 5's rule goes in
  `skills/shuyi-input/SKILL.md`, not in `input-actions`, which keeps
  only what holds in this repository (1134, *One source for what is
  true of every game*).
- **From 2003 (done):** the `fixed-ticks` answer of the time concern in
  `docs/modules.ron` is parked and names this ticket alone. Set it to
  ready.

**Out (do not do):**
- collision (1122);
- world layers (1120);
- overlays that let a stage keep running, such as a notice or a toast
  (2367, parked; its note says it is unparked together with this
  ticket's first tick stage). A pause menu pushed over a stage stays
  modal, the default, and stops it;
- networking;
- checking a tick game's submitted score on 1148's server (its
  `TickRecord` re-simulated). **Either order with 1148:** if 1148 is
  done, its PR wrote a parked follow-up ticket for that check, blocked
  by this one, and this PR adds nothing for it; if not, nothing to do
  (1148 checks a `TickRecord` itself);
- thousands of units (the parked 2705 measures an arena against columns
  and an ECS for the real-time strategy module, ADR-0015 §3);
- jump buffering and coyote time (1127);
- hold or toggle per action (2404);
- a game's queue of turns or directions (game or style-kit logic);
- axes and the pointer in `TickInput` (1121 fills them, with the latest
  values each step);
- command sims (2706's `CommandDriver` already schedules each command
  for a later step);
- game speed and hit-stop (2123): the speed scales the time `App`'s
  `TickClock` owes, so `TickDriver` keeps stepping `FrameInput::ticks()`
  times and needs nothing for it; hit-stop is simulation state (2123's
  `Freeze`);
- timers and the step rate: `shuyi_core::timer` (2115) has `Countdown`,
  `Every`, `steps_from_ms` and `STEPS_PER_SECOND` (moved there from
  `shuyi_platform::tick`, which re-exports it). `shuyi_core::tick` and
  the example sim time things with them and add no timer of their own;
- a tick replay in the tool contract
  (`shuyi_cli::contract::replay::ticks`, 2739);
- a soak of tick sims: `shuyi_core::soak` (2527) reloads a turn game's
  state at every step, compares it with a copy never reloaded and
  checks the game's invariants. 2549 (parked, blocked by this ticket
  and 2527) does the same for a `TickSim` and its records (a snapshot
  round trip per step).

## Implementation steps

1. `Fx` with property tests: the algebra holds within documented
   rounding; a golden table of a fixed list of operations,
   `crates/core/tests/golden/fx.ron`, through 2116's
   `shuyi_assets::golden` (a dev-dependency on `shuyi-assets` with
   `default-features = false, features = ["golden"]`, so the no-std run
   stays without `std`). CI's `test` job checks it on x64 and ARM64,
   and its `determinism` job on wasm32, which runs `shuyi-core`'s
   tests. The example sim's `TickRecord` and its hash per step go in a
   table too.
2. `Arena` with property tests (ids never reused while live; order
   stable). If 2904 is done, read its ADR first. `Arena<T>` follows what
   it says the arena shares with `shuyi-ecs`: kits name neither store,
   and scenes (2221) get an arena side only if the ADR calls for it, as
   a follow-up ticket written here (the `write-ticket` skill), not in
   this PR.
3. `TickSim`, records, replay, `TickDriver` with the latch (Scope,
   *Input reaches the steps*) and its events handed on once, and
   `FrameInput`'s presses without repeats and released actions; Harness
   support.
4. Decide, and write in the Completion notes, whether any example needs
   an ECS (ADR-0003 §4). Default: no. A game that wants one has the
   opt-in `shuyi-ecs` module (2107). Its tick adapter, `EcsTickSim`, is
   2108's either-order clause. If 2108 is done, its PR wrote a parked
   follow-up ticket for the adapter, blocked by this one, and this PR
   adds nothing for it. If 2108 isn't done, 2108 writes the adapter
   itself.
5. Add a rule to the `input-actions` skill
   (`.claude/skills/input-actions/SKILL.md`, *Rules*): "**Presses reach
   fixed steps once.** A tick stage reads input from `TickInput`, never
   from `FrameInput::actions`: `TickDriver` keeps a frame's presses and
   releases until a step runs, gives each to exactly one step, and never
   gives a key repeat to a step (1119)."

## Acceptance criteria

- [ ] Replaying a record twice gives identical hash lists (property
      test over random inputs).
- [ ] `Fx` results and the example record's hashes match their golden
      tables in CI's `test` and `determinism` jobs (x64, ARM64, wasm32).
- [ ] The Harness steps the example sim headlessly; a snapshot of its
      text dump passes.
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
- [ ] The `shuyi-input` skill states the rule, and `check-skills`
      passes.
- [ ] Run the example sim for 10 steps, rewind it to its state at step
      5, and run it to step 10 again. The events of steps 6 to 10 are
      handed on once: the Harness's `audio_log` shows each cue once.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Property: `Fx`, `Arena`, replay determinism; for random frame times
  (0 to 100 ms) and random scripts of presses and releases, no press or
  release is lost or doubled.
- Integration: the Harness drives the example sim; the frame-rate sweep
  above; the rewind test above.

## Completion notes
