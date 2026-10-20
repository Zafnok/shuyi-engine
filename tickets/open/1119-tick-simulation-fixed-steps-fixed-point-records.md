---
id: "1119"
title: "Parked: Real time: a fixed-step tick simulation with fixed-point numbers, an entity arena and input records that replay"
type: feature
milestone: "Engine: real time"
model: opus-5.5
effort: xhigh
status: blocked
blocked_by: ["game#1131"]
nick_input: none
completed:
---

# 1119 — Tick simulation: fixed steps, fixed-point, records that replay

> **Parked** (Nick, 2026-10-14: "document the parked (i.e. after MVP/parity) tickets for networking, styles for later"): after the engine reaches parity with the game (the MVP in `docs/ROADMAP.md`). Unparked early only if the game's ticket 0060 says the army walks around freely (then 1119–1125 become game work). Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

ADR-0003 §2–§4. Top-down free movement, side-scrolling and real-time
isometric games need a simulation that steps in time. To keep what
makes this codebase fast to work in (replays, headless tests, bots, and
later rollback networking, ADR-0006), the step is fixed (60 a second),
numbers are integer fixed-point, and a run is its seed plus its inputs.

It needs the engine's code in this repository (the game's ticket 1131).

## Nick input

None (technical). Scheduling: see above.

## Scope

**In:**
- `shuyi_core::fx`: `Fx` (i32, 8 fraction bits), `FxVec2`, with
  arithmetic that rounds toward negative infinity, saturating where
  documented, `from_px`, `floor_px`, a 256-step sine/cosine table.
- `shuyi_core::arena`: `Arena<T>` with generational `Id<T>`, insert,
  remove, get, iteration in id order, serde, and a text dump.
- `shuyi_core::tick`: `trait TickSim { type Event; fn step(&mut self,
  input: &TickInput) -> Vec<Self::Event>; fn hash(&self) -> u64; }`.
  `TickInput` holds, per player, the action ids pressed, held and
  released, axes as `i16`, and the pointer in world pixels (filled by
  1121; empty until then). `TickRecord` holds a seed and run-length
  encoded inputs. `replay(sim, record) -> Vec<u64>` returns hashes per
  step.
- `shuyi_ui::tick::TickDriver` holds a screen's accumulator: at most 4
  steps a frame, then it slows down rather than spiral. The Harness can
  step exact counts (`harness.steps(n)`). *Since ticket 3003 the
  accumulator is `shuyi_platform::App`'s (`shuyi_platform::tick`,
  ADR-0024 §4): a screen reads the steps due as `FrameInput::ticks()`,
  so `TickDriver` steps the simulation that many times and keeps no
  clock of its own.*
- A tiny example sim in the engine's tests: a dot moved by input with
  inertia.
- **The debug view (2316).** If 2316 is done, its step runs one fixed
  simulation step here, and its readout names the step number. If 2316
  isn't done, 2316 does this.

**Out (do not do):** collision (1122); world layers (1120); networking;
thousands of units (the parked 2705 measures an arena against columns
and an ECS for the real-time strategy module, ADR-0015 §3).

## Implementation steps

1. `Fx` with property tests: the algebra holds within documented
   rounding; native and `wasm32` builds give identical results for a
   fixed list of operations (a test that runs under `wasm32-wasip1` in
   CI, or compares against a checked-in table).
2. `Arena` with property tests (ids never reused while live; order
   stable).
3. `TickSim`, records, replay, `TickDriver`, Harness support.
4. Decide, and write in the Completion notes, whether any example needs
   an ECS (ADR-0003 §4). Default: no. A game that wants one has the
   opt-in `shuyi-ecs` module (2107), whose tick adapter is 2108's: if
   2108 is done, check its adapter runs on this `TickSim`.

## Acceptance criteria

- [ ] Replaying a record twice gives identical hash lists (property
      test over random inputs).
- [ ] `Fx` results match the checked-in table on native and web.
- [ ] The Harness steps the example sim headlessly; a snapshot of its
      text dump passes.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Property: `Fx`, `Arena`, replay determinism.
- Integration: the Harness drives the example sim.

## Completion notes
