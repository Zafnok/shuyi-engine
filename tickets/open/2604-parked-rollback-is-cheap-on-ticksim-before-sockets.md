---
id: "2604"
title: "Parked: Rollback is shown to be cheap on TickSim before any socket exists: save, load and resimulation measured per step, and a check a game's tests run"
type: feature
milestone: "Parked: networking"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: blocked
blocked_by: ["3525"]
nick_input: none
completed:
---

# 2604 — Rollback is shown to be cheap on TickSim before any socket exists

> **Parked** (Nick, 2026-10-14: "document the parked (i.e. after MVP/parity) tickets for networking, styles for later"), like 1140–1150. It may start when Nick unparks networking, or when his answer to decision ticket 3525 puts a real-time core or real-time online play into 1.0. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

A blind-spot audit of the engine (2026-11-23) found that rollback, the
sync model ADR-0006 gives tick games (§1, the *Sync model* row), is
only research, and late research. 1147 spikes GGRS and a rollback of
our own, but it is blocked by 1145 (sessions and lobbies), which waits
for the relay (1141) and the LAN (1143). So nothing checks that
`TickSim` can roll back cheaply until the whole turn-game network stack
exists. If it can't, `TickSim` changes shape after games use it.

Rollback asks three things of a simulation:

- **save**: keep a copy of the state at each step;
- **load**: put an older copy back when a remote input turns out
  different from the guess;
- **resimulate**: step again from there to now, inside one frame.

None of these needs a socket. They need only `TickSim`
(`crates/core/src/tick.rs:187`, ticket 2137, done) and a state that
clones.

What is there today:

- `TickSim` has `step` and `hash` and no save or load
  (`crates/core/src/tick.rs:187-195`). 1147 assumes "save and load as
  clone, which the arena makes cheap"; the arena (`Arena<T>`) is parked
  ticket 1119's, and no code measures either.
- `replay` (`tick.rs:309`) steps a record from the start and returns
  each step's hash; nothing rewinds a sim and compares.
- `shuyi_ui::tick::TickDriver::rewind_to` (2137) sets the driver's steps
  back and hands no event on twice.
- The only toy tick sim is one `Dot` (`crates/core/src/toy.rs:60`), too
  small to measure.
- The targets' frame budgets are in instructions:
  `frame_instructions: 20_000_000` for the desktop
  (`targets/desktop.ron:32`, the web, Steam Deck, Android and iOS too)
  and `2_000_000` for the Playdate (`targets/playdate.ron:30`).
- The engine already counts one function's instructions with
  Callgrind's `--toggle-collect` and its allocation calls with
  `shuyi_alloc_count::global` in a test
  (`crates/assets/tests/lang_load.rs:104`, ticket 2266).

The audit also found that `docs/modules.ron:67-68` points the online
answers `rollback` and `referee` at 1140. 1140 builds neither: its
scope is lockstep over `TurnRules` (its Out leaves even command-driven
tick lockstep to 2720), rollback is 1147's research, and the referee
server is 1146. This ticket fixes those two rows.

ADR-0003 (tick sims make rollback possible), ADR-0006 §1 and §6,
ADR-0039 (new public types), ADR-0081 (the bench counts instructions,
never wall time).

## Nick input

None.

## Scope

**In:**
- `shuyi_core::toy::Crowd`, behind the `toy` feature: a `TickSim` of N
  movers in a box, with integer positions and velocities, a `SimRng` in
  its state, and seat 0's input pushing every mover, so a wrong guess of
  the input changes the state. A mover hitting a wall is an event.
- `shuyi_core::tick::check_rollback`, public, for a game's own tests:
  it runs the inputs straight, then again with rollbacks (guess each
  step's input by repeating the last one, then load the save from
  `window` steps back and step again with the real inputs), and says
  whether every step's hash matched the straight run. Its error type
  `RollbackMismatch` is new and may grow, so it is `#[non_exhaustive]`
  with accessors (ADR-0039). Adding it breaks nothing.
- `TickSim`'s doc contract gains a **Rollback** bullet: a sim that rolls
  back is `Clone`, and keeps its state in containers whose `clone_from`
  reuses their memory (a `Vec`, an arena), so a warm save allocates
  nothing.
- A cost test that measures, for a `Crowd` of 100, 1,000 and 10,000
  movers: one save, one load and a rollback of 8 steps (one load, then 8
  steps each saved again), as instructions (Callgrind, once, written
  down) and as allocation calls (asserted in the test).
- A research note, `docs/research/2604-rollback-cost.md`, with the
  numbers and their verdict against the budget below.
- `docs/modules.ron`: the `rollback` row's tickets become
  `["2604", "1147"]`, and the `referee` row's `["1146"]`.

**The budget**, from the requirement, not from the code: a rollback
frame still steps the newest step and draws, so the rollback itself (one
load and 8 steps, each saved) for 1,000 movers takes at most half the
desktop's `frame_instructions`, 10,000,000 instructions. 8 steps is
GGPO's usual prediction window (about 133 ms at 60 steps a second). The
Playdate's numbers are reported, not budgeted: no online play is planned
there.

**Out (do not do):**
- any transport, socket, session, input queue or desync exchange (1140,
  1147);
- choosing GGRS or our own rollback, and what happens to the events of
  resimulated steps (1147's ADR; `TickDriver::rewind_to` stays as it
  is);
- easing a correction in the drawn view (2361 and 1147);
- seeking a session by replay (2357, which replays from the start and
  takes no snapshot);
- measuring the arena, columns or an ECS at scale (2705);
- making `Clone` a supertrait of `TickSim`: the check takes
  `S: TickSim + Clone`, and only 1147's ADR may tighten the trait;
- a wall-clock measurement (ADR-0081).

**Either order with 1119:** if 1119 is done, `Crowd` keeps its movers
in `Arena<Bouncer>` with `Fx` positions, and the note gives the arena's
save cost; if not, `Crowd` keeps a `Vec<Bouncer>` of `i32` fields and the
note says the arena is a `Vec` of slots that clones the same way, and
nothing is left for 1119.

**Estimated size:** about 700 lines (code 250, tests 300, docs 150).

## Implementation steps

1. **`Crowd`.** In `crates/core/src/toy.rs` (or a `toy/crowd.rs` beside
   it), add `Crowd::new(movers: u32, seed: u64)`, its `Bouncer` (position
   and velocity, integers only: the float ban of ADR-0002 and
   `tools/layers.ron` holds), and `impl TickSim for Crowd`: each step
   seat 0's held directions add to every mover's velocity, movers move,
   and one that crosses a wall bounces and makes a `CrowdEvent::Bounced`.
   `hash` is `state_hash(self)`'s, as `Dot`'s is. `Crowd` derives
   `Clone`. Follow 1119 if it is done (the clause above).
2. **`check_rollback`.** In `crates/core/src/tick.rs`:
   `pub fn check_rollback<S: TickSim + Clone>(start: &S, inputs: &[TickInput], window: usize) -> Result<(), RollbackMismatch>`.
   Run `inputs` straight from a clone of `start`, keeping each step's
   hash. Then run again: at each step past the first `window`, save into
   a ring of `window + 1` states with `clone_from`, step with the guessed
   input (the last step's), then load the save from `window` steps back
   and step to now with the real inputs. Events of the resimulated steps
   are dropped (the driver's job, 2137). The first step whose hash
   differs is the error: `RollbackMismatch` with `step()`, `straight()`
   and `rolled()`. Document it with a doctest on `Dot`.
3. **The contract.** Add the **Rollback** bullet to `TickSim`'s doc
   (`tick.rs`, the `# Contract` list), naming `check_rollback`.
4. **The cost test.** `crates/core/tests/rollback_cost.rs`, after
   `crates/assets/tests/lang_load.rs`: three `#[inline(never)]`
   functions, `save`, `load` and `rollback` (one load then 8 saved
   steps), and `shuyi-alloc-count` with `global` as a dev-dependency.
   Dev-dependencies are free in `tools/layers.ron`, but `shuyi-core`'s
   tests also run without `std`: gate the file with
   `required-features = ["std"]` (or put it in a crate with `std`, such
   as `shuyi-ui`'s tests) so that run stays as it is. The test warms the
   ring, then asserts that a save and a load make no allocation call
   for each crowd size, and that `rollback` makes none beyond what
   `step` returns (an empty event list allocates nothing).
5. **Count the instructions.** On Linux (Valgrind runs there only: CI,
   WSL or a container), run the test under
   `valgrind --tool=callgrind --toggle-collect='rollback_cost::save'`,
   then `load` and `rollback`, for each crowd size. Write the counts,
   the command and the toolchain into the research note.
6. **The note.** `docs/research/2604-rollback-cost.md`: the counts per
   phase and per mover, the budget and the verdict, what a game's state
   must avoid to stay cheap (a `Box` or `Rc` per entity, a `HashMap`
   that rebuilds, a big `String`), and what it means for 1147's spikes.
   If the 1,000-mover rollback is over budget, the note says why, and a
   ticket is written (`write-ticket`) for what `TickSim` or the toolkit
   must change, parked like this one.
7. **Fix `docs/modules.ron`** lines 67-68 as the Scope says, and check
   `cargo xtask check-modules`.
8. **The book.** The tick part of
   `site/content/learn/book/rules-as-commands.md` (it introduces
   `TickSim`, line 184) gets a short paragraph: a sim that will roll
   back is `Clone`, and its tests call `check_rollback`.

## Acceptance criteria

- [ ] `Crowd` exists behind `toy`, derives `Clone`, holds no float, and
      its golden or snapshot test shows a run of 60 steps of 100 movers.
- [ ] `check_rollback` passes on `Dot` and on `Crowd` with a window of
      1, 2 and 8 steps, over inputs that change direction at least
      every 5 steps.
- [ ] `check_rollback` returns `RollbackMismatch` at the right step on a
      test sim whose `clone` forgets a field (its `SimRng`, say).
- [ ] With the ring warm, a save and a load of a `Crowd` of 100, 1,000
      and 10,000 movers make no allocation call, asserted in the test.
- [ ] `docs/research/2604-rollback-cost.md` gives the instruction count
      of a save, a load and an 8-step rollback for each crowd size, how
      they were counted, and the verdict against 10,000,000 instructions
      for 1,000 movers.
- [ ] If that verdict is over budget, a parked follow-up ticket exists
      and the note names it.
- [ ] `docs/modules.ron`'s `rollback` row lists `2604` and `1147`, its
      `referee` row lists `1146`, and `cargo xtask check-modules` passes.
- [ ] `TickSim`'s doc has the **Rollback** bullet.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `check_rollback` passing on `Dot` and `Crowd`; failing at the
  right step on a sim whose clone drops a field; a window of 0 and one
  longer than the inputs.
- Property: for any seed, crowd size up to 200 and input sequence,
  `check_rollback` with any window from 1 to 8 passes.
- Snapshot / integration: `Crowd`'s run (a snapshot of its text, or a
  row in `shuyi-core`'s golden table, `shuyi_assets::golden`, so CI's
  `determinism` job checks it on wasm32 too); the allocation test of
  step 4.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
