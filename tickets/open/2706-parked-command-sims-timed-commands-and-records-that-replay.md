---
id: "2706"
title: "Parked: Command sims: CommandSim, timed commands and records that replay, in shuyi-core"
type: feature
milestone: "Engine: real-time strategy"
model: opus-5.5
effort: high
status: blocked
blocked_by: ["1119"]
nick_input: none
completed:
---

# 2706 — Command sims: timed commands and records that replay

> **Parked** (Nick, 2026-10-18: "make tickets parked for after visions of shuyi launches for all of these"): after Visions of Shuyi launches, which is later than the MVP in `docs/ROADMAP.md`. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

[ADR-0003](../../docs/adr/0003-game-styles-stages-two-kinds-of-simulation-cameras-and-projections.md)
§2 has two shapes of simulation: turn rules driven by commands, and tick
simulations driven by inputs per step (`TickSim`, built by 1119). A
real-time strategy game is a third shape: it steps 60 times a second,
but its input is the orders players give, not the keys they press.
[ADR-0015](../../docs/adr/0015-real-time-strategy-at-scale-command-sims-columns-flow-fields-and-ai-in-layers.md)
§2 adds that shape to `shuyi_core::tick`.

It lives in the core, not in the RTS module, because lockstep (2720)
needs it and `shuyi-net` must not depend on a module (ADR-0015,
*Consequences*). It is generic: any game whose input is orders can use
it. 2707 is its first user.

It needs 1119: `Fx`, `TickSim`, `TickRecord`, `replay`, `TickDriver`
and the Harness's `steps(n)`.

## Nick input

None.

## Scope

**In:**
- **In `shuyi_core::tick`**, beside 1119's `TickSim`:
  - `Timed<C> { step: u64, player: PlayerId, command: C }`. `PlayerId`
    is the one 1119's `TickInput` uses; if 1119 has none, add
    `PlayerId(u8)` here.
  - `trait CommandSim` as ADR-0015 §2 writes it: `type Command: Clone +
    Eq + Serialize + DeserializeOwned`, `type Event`, `fn step(&mut
    self, due: &[Timed<Self::Command>]) -> Vec<Self::Event>`, `fn
    hash(&self) -> u64`.
  - `CommandRecord<C> { format_version: u32, made: Made, seed: u64,
    steps: u64, commands: Vec<Timed<C>> }`:
    - `made` is 2111's stamp: the engine version, the game's stamp and
      the content fingerprint. `replay_commands` takes the build's
      `Made` too, and refuses a record whose `made` differs from it,
      naming both (`Made::check`). **Either order with 2111:** if 2111
      isn't done, leave `made` out; 2111 adds it when it lands after
      this;
    - `steps` is the run's length, so steps after the last command
      replay too;
    - `commands` are kept ordered by step, then player, then the order
      they were sent. `push` inserts after every entry with the same
      step and player, so the sent order survives;
    - `due(step) -> &[Timed<C>]` returns one step's commands, found by
      binary search;
    - serde (RON), with `format_version: 1` and `deny_unknown_fields`.
      A record is a run's log, not an asset kind, so it gets no
      `Format`.
  - `CommandQueue<C>`: the commands scheduled for later steps.
    - `schedule(timed) -> Result<(), TooLate>`: a command for a step
      that has already run is refused with `TooLate { step, now }`.
      Local play schedules for the next step; lockstep (2720) schedules
      further ahead.
    - `take_due(step) -> Vec<Timed<C>>`, in record order, also appended
      to the queue's own `CommandRecord`.
  - `replay_commands<S: CommandSim>(sim: &mut S, record:
    &CommandRecord<S::Command>) -> Vec<u64>`: steps `record.steps`
    times, feeding each step its `due` slice, and returns the hash after
    each step. It has the same shape as 1119's `replay`: the caller
    builds `sim` from `record.seed`.
  - A **text dump** of a record: a header line with the seed and the
    step count, then one line per command, `step player command`, the
    command in compact RON.
  - Nothing here reads a clock, a file or a float.
- **In `shuyi_ui::tick`**, beside 1119's `TickDriver`:
  - `CommandDriver<S: CommandSim>`: steps the sim `FrameInput::ticks()`
    times, as `TickDriver` does. The accumulator is `App`'s `TickClock`
    (ADR-0024 §4), so it gets the same cap and, once 2123 is done, the
    game speed. Share the stepping code with `TickDriver`; don't copy
    it.
  - `issue(player, command)` schedules a local command for `now + delay`
    steps. `delay` is a setting, `1` by default (the next step); 2720
    sets lockstep's.
  - `record()` returns the `CommandRecord` so far.
- **The Harness:**
  - `harness.command(player, command)` schedules for the next step;
  - `harness.commands(&[Timed<C>])` schedules a list;
  - `harness.steps(n)` steps a `CommandDriver` screen as it does a
    `TickDriver` one;
  - `harness.command_record()` returns the record.
- **A tiny example sim** in the engine's tests, `Beacons`: a few dots on
  a line. Commands are `MoveTo { dot, x }` and `Stop { dot }`. Each step
  a dot moves one `Fx` step toward its target. It emits `Arrived(dot)`.

**Out (do not do):**
- lockstep over command turns (2720);
- the RTS module, its commands and its units (2707);
- computer players issuing commands (2714, 2715);
- a `replay` operation for command records in the tool contract. If
  1129 has a `replay` operation when this lands, write a follow-up
  ticket for it;
- a soak of command sims: `shuyi_core::soak` (2527) checks `TurnRules`
  games only. When this lands, write a follow-up ticket for the same
  reload-per-step check over a `CommandSim`, as for the contract's
  `replay` above;
- a generic queue of delayed events: 2115 left `Delayed<E>` out of
  `shuyi_core::timer`. If a ticket adds one before this starts,
  `CommandQueue` builds on it rather than beside it.

## Implementation steps

1. Add `Timed`, `CommandSim`, `CommandRecord` and `TooLate` to
   `crates/core/src/tick.rs` (or a `tick/command.rs` submodule if
   `tick.rs` passes 400 lines), with docs and a doc example.
2. Add `CommandQueue` and `replay_commands`.
3. Add the text dump and serde, with a format version.
4. Add `CommandDriver` in `crates/ui/src/tick.rs`, sharing the
   stepping code with `TickDriver`.
5. Add the Harness methods.
6. Write `Beacons` and its tests.

## Acceptance criteria

- [ ] Replaying a record twice gives identical hash lists (property
      test over random commands).
- [ ] The hashes of a live Harness run equal `replay_commands` over the
      record the run produced (property test).
- [ ] Commands pushed for one step in any order of players come out
      ordered by player, and two commands from one player keep the order
      they were sent (property test).
- [ ] Scheduling a command for a step that has run returns `TooLate`.
- [ ] A record round-trips through RON unchanged.
- [ ] A snapshot of `Beacons`' record dump after 120 Harness steps
      passes.
- [ ] `shuyi-core` still builds with no floats in `tick` (2015's
      `#![cfg_attr(not(test), deny(clippy::float_arithmetic))]` in
      `crates/core/src/lib.rs` passes), and for
      `wasm32-unknown-unknown`.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `CommandRecord::push` and `due`; `TooLate`; the text dump.
- Property: replay determinism; live run equals replay; command order
  within a step; serde round trip.
- Integration: the Harness drives `Beacons` through `CommandDriver`.
- Snapshot: the record dump.

## Completion notes
