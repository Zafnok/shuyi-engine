---
id: "2549"
title: "Parked: A soak of tick sims: random inputs into a TickSim, a reload at every step against a copy never reloaded, and a failure shrunk to a short TickRecord"
type: feature
milestone: "Engine: real time"
model: opus-5.5
effort: high
status: blocked
blocked_by: ["1119", "2527"]
nick_input: none
completed:
---

# 2549 — A soak of tick sims: random inputs, reloaded at every step

> **Parked** (Nick, 2026-10-14: "document the parked (i.e. after MVP/parity) tickets for networking, styles for later"; 2026-10-20: "unless these real-time work tickets overlap with visions of shuyi they should be parked until visions of shuyi is complete"): a follow-up of the parked tick simulation (1119), and a follow-up of parked work is parked, so it starts after 1119 is unparked and done, and after 2527. If 1119 is unparked early because the game needs it (`docs/decisions.md`, 2026-10-20), this stays parked until Nick says it comes too. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

2527 adds `shuyi_core::soak` for turn games. A seeded bot plays random
legal commands. At every step one copy of the state goes through a
`Codec` (written and read back, as a save would be) and is compared
with a copy that was never reloaded. The game's invariants run on both
copies, and a failure shrinks by delta debugging to a short record. It
finds state that the serde form leaves out, such as a
`#[serde(skip)]` field or a cache. `replay` can't see that state,
because a replay rebuilds the state from the start.

Tick sims (1119,
[ADR-0003](../../docs/adr/0003-game-styles-stages-two-kinds-of-simulation-cameras-and-projections.md)
§2) can have the same bug, and nothing checks for it. 1119's tests
replay a record twice and compare the hashes, and a lost field passes
that test. A tick game saved in progress rests on the serde form, and
so does a state sent to a peer later
([ADR-0006](../../docs/adr/0006-networking-in-layers-transports-sessions-sync-models-services.md)).
A tick sim also has a second gap. `TickSim::hash` is the game's own
function, not `state_hash` of the serde form, so a hash that misses a
field hides a difference too.

1119's **Out** leaves this check to this ticket, and so does 2527's.
2527 checks `TurnRules` games only: it needs `legal_commands`, `check`
and `outcome`, and a `TickSim` has none of them. But a `TickSim` takes
any `TickInput`, so this soak needs no list of legal moves. Random
presses, holds and releases of the game's actions are all legal input.

It is cheap. It reuses 2527's `Codec`, `Checks`, `Check`, `Failure`,
`Shrunk` and delta debugging, and 1119's `TickInput`, `TickRecord` and
`replay`.

Why it matters: the end goal of 2026-10-19 (`docs/decisions.md`) names
tick games. An agent that writes a tick game's rules should get the
same short failing record that 2527 gives the author of a turn game.

## Nick input

None.

## Scope

**In:**

**`shuyi_core::soak::tick`** (`crates/core/src/soak/tick.rs`, tests in
`crates/core/src/soak/tick/tests.rs`), `no_std` with `alloc` (no
`std::` path; `cargo xtask check-no-std` checks it), beside 2527's turn
soak. No new dependency, and no float.

- `trait Inputs { fn next(&mut self, step: u32) -> TickInput; }`: the
  input stepped from step `step` to step `step + 1`, so the first call
  gets 0. A closure `FnMut(u32) -> TickInput` implements it, so a game
  can hand in a script of its own.
- `RandomInputs`, the random player:
  - `RandomInputs::new(seed: u64, actions: &[A], seats: u8)`, where `A`
    is the action id type `TickInput` holds. `seats` is clamped to 1 to
    4 (`TickInput`'s most), and the docs say so.
  - It draws from its own `SimRng::new(seed)`, in a fixed order: each
    step, each seat from 0, each action in list order. So a seed gives
    the same inputs on every platform. It never sees the state, so it
    never draws from the state's `SimRng`.
  - It keeps which actions each seat has down, and makes only lists
    that 1119's latch could make:
    - an action that is up is pressed one step in 8. One press in 4 is
      a tap: `pressed`, not `held`, and `released` in the next step.
      The rest are holds: `pressed` and `held`;
    - an action held stays `held`, or one step in 6 it is `released`
      (and not `held`);
    - an action released in a step isn't pressed again in that step.
  - Axes and the pointer stay at rest. A game's debug inputs are never
    sent.
  - It is `#[non_exhaustive]` with `new`
    ([ADR-0039](../../docs/adr/0039-public-types-that-can-grow-non-exhaustive-config-and-views-with-constructors-plain-values-left-open.md)),
    so rates can be added later without a break.
- **2527's types, reused:**
  - `Checks<'c, S, C>` (the codec and the game's invariants, a
    `fn(&S) -> Vec<String>`) serves a `TickSim` as it serves a
    `TurnRules` game. If 2527 put a `TurnRules` bound on the struct
    itself, move the bound to the functions that need it. Removing a
    bound breaks nothing.
  - `Check` gains `Record` (`record`): the play's `TickRecord`,
    written and read by the codec and replayed with `replay`, gives
    another hash at some step, or doesn't go through the codec. `Check`
    is `#[non_exhaustive]` (2527), so the variant is additive. The tick
    soak gives `hash`, `round-trip`, `reloaded`, `invariant` and
    `record` only, and each variant's docs say which soak gives it.
    `Check::ALL` lists `record`. If `ALL` is an array with its length
    in its type, the longer array is a break: write the `Breaking:`
    line (ADR-0038).
    - **Either order with 2772:** if 2772 is done, `Check::Record`
      and the shared ddmin are there: use both, add the tick soak to
      `Record`'s docs, and add nothing to `ALL`. If not, add them as
      written here, and 2772 uses them.
  - `Failure<TickInput>`: `commands` holds one `TickInput` per step,
    from the first step through the failing one, so replaying it
    reproduces the failure. `step` counts as 2527's does: 0 is the
    start, `n` is after the `n`th input. A failure found while input
    `n` is stepped is at step `n`.
  - `Shrunk<TickInput>` for a shrunk failure.
- `play<S, C>(start: &S, blank: &TickRecord, inputs: &mut dyn Inputs,
  steps: u32, checks: &Checks<S, C>) -> Result<TickRecord,
  Failure<TickInput>>`: steps `steps` times (a `TickSim` has no
  outcome), and checks every step. On a pass it returns the play's
  record: `blank` with every step's input added. `blank` is an empty
  record for `start`, made as 1119 makes one (its seed, and its `Made`
  stamp if `TickRecord` has one).
- `inputs<S, C>(start: &S, blank: &TickRecord, inputs: &[TickInput],
  checks) -> Result<(), Failure<TickInput>>`: a fixed list checked the
  same way, for a kept record and for shrinking. Every list plays,
  because `TickSim::step` takes any `TickInput`.
- `record(blank: &TickRecord, inputs: &[TickInput]) -> TickRecord`: the
  record a failure's inputs make, which 1119's `replay` reproduces. If
  1119's `TickRecord` has no public way to add one step's input, or to
  list the inputs it holds, add `TickRecord::push(&mut self, input:
  TickInput)` and `TickRecord::inputs(&self) -> Vec<TickInput>` in
  `crates/core/src/tick.rs`, run-length encoded as the driver writes
  it.
- `shrink<S, C>(start: &S, blank: &TickRecord, failure:
  Failure<TickInput>, checks, runs: u32) -> Shrunk<TickInput>`: 2527's
  delta debugging over the steps. A step is kept or dropped whole. A
  candidate counts only if `inputs` fails it with the same `Check`. A
  candidate may be a list the latch never makes (a release with no
  press before it); it still plays, and that is fine. At most `runs`
  candidates are run, and the shrink keeps the play's seed.
  - Share the delta debugging with 2527's `shrink`: move it into one
    private generic function, such as `fn ddmin<T: Clone>(items:
    Vec<T>, runs: u32, fails: impl FnMut(&[T]) -> bool) -> (Vec<T>,
    bool)`, which both shrinks call, unless 2527 already did. 2527's
    shrink tests pass unchanged.
- Bounds: `S: TickSim + Clone + Serialize + DeserializeOwned`, and
  `S::Event: PartialEq + Debug`. `TickInput` and `TickRecord` have
  serde already (1119). If `TickInput` or `TickRecord` take a type
  parameter (the game's debug inputs), the functions here take it too.
- No re-exports at the crate's root: a game writes
  `shuyi_core::soak::tick::play`.

**One step, exactly.** The module docs say it in this order. Copies A
and B start as clones of `start`.

1. Hash A: `TickSim::hash`, and `state_hash` of its serde form (`hash`
   if `state_hash` refuses). Run the invariants on A (`invariant`).
2. B goes through the codec. The copy's `TickSim::hash` and
   `state_hash` must equal B's (`round-trip`). The invariants run on
   the copy (`invariant`, "the reloaded copy"), and B becomes the copy.
3. The step's input (from `Inputs`, or the list's) is stepped on A and
   on B. Other events, or another `TickSim::hash` or `state_hash` after
   the step, is `reloaded`.

Both hashes are compared because each can miss something. `state_hash`
never sees a `#[serde(skip)]` field, and the game's `TickSim::hash` may
leave out a field that the serde form holds.

At the end of a play or a list, steps 1 and 2 run once more on the last
state. Then the play's `record(blank, inputs)` goes through the codec,
and `replay` runs it from a clone of `start`. It must give one hash for
each step played (and the start's, if 1119's `replay` returns one), and
each must equal A's `TickSim::hash` at that step (`record`). If 1119's
`replay` takes the build's `Made`, pass the record's own: the soak
tests the state, not the stamp.

**1119's example sim** (the dot moved by input with inertia, in the
engine's tests) is soaked. Its state derives `Clone`, `Serialize` and
`Deserialize` if it doesn't yet. A test beside its own tests runs
`play` for 50 seeds of 600 steps each (10 seconds of play), with
`RandomInputs` over its actions and one seat, the codec RON and no
invariants. It passes.

**Docs** (the `document` skill): the module docs teach the soak, with a
doc example on a tiny sim. The crate docs' line on `soak`
(`crates/core/src/lib.rs`) names tick sims too.

**Either order with 1134:** if 1134 is done, the soak section of
`skills/shuyi-test/SKILL.md` gains a short paragraph: a tick game soaks
its `TickSim` in its own tests with `soak::tick::play` and
`RandomInputs` over its actions, and keeps a shrunk `TickRecord` as a
regression test that calls `soak::tick::inputs`. `cargo xtask
check-skills` still passes. If not, nothing to do: 1134 adds the
paragraph when it lands.

**The public API.** `soak::tick` and its items are new, so they are
additive, and so is `Check::Record`. A `Breaking:` line is needed only
for `Check::ALL`, as above.

**Out (do not do):**
- turn games: 2527's `soak::play` checks them;
- command sims (2706): the same check over a `CommandSim` is 2772's;
- a `soak` op for tick games in the tool contract. 2527's op plays turn
  games through `GameTools::soak`. A tick game calls `soak::tick` from
  its own tests. The contract's tick replay is 2739's;
- `TickDriver`, frame rates and the input latch: the soak steps the sim
  straight, as `replay` does. 1119's Harness sweep tests the latch;
- random axes and pointer positions in `TickInput`. 1121 adds them to
  `TickInput`, and `RandomInputs` leaves them at rest. A game that
  reads them hands in its own `Inputs`;
- debug inputs (2356): `RandomInputs` never sends one;
- shrinking inside a step, such as dropping one press of several: the
  steps are the unit, as commands are 2527's;
- desyncs between several peers (2529);
- the same records on several platforms (2116);
- a game's own save files in the soak: a game whose saves aren't its
  state in RON writes its own `Codec`, as 2527 says.

## Implementation steps

1. Read 2527's `crates/core/src/soak.rs` and its tests, and 1119's
   `crates/core/src/tick.rs` (`TickSim`, `TickInput`, `TickRecord`,
   `replay`) and its example sim.
2. In `soak.rs`: share the delta debugging between the shrinks, move
   any `TurnRules` bound off `Checks`, and add `Check::Record` with its
   name, its place in `ALL` and its docs. Run 2527's tests.
3. Add `TickRecord::push` and `TickRecord::inputs` if they are missing.
4. Write `soak/tick.rs` and add `pub mod tick;` in `soak.rs`:
   `Inputs`, `RandomInputs`, and one private `Run` holding A, B, the
   hashes so far and the inputs so far, with `step(input)` and
   `finish()`. Then `play`, `inputs`, `record` and `shrink` over it.
5. Write the fixture sims and the tests below. The tests' codec is RON
   (the core's dev-dependency).
6. Soak 1119's example sim, write the docs, do the 1134 part if it
   applies, and run the gates.

Size: about 250 lines in the core and 300 in tests. The fixture sims
count in the tests.

## Acceptance criteria

- [ ] A fixture sim with a `#[serde(skip)]` field, set when one action
      is pressed and read at the start of a later step when another is
      pressed, fails `reloaded` and shrinks to exactly two steps: the
      one that presses the first action and the one that presses the
      second.
- [ ] Each of `hash`, `round-trip`, `invariant` and `record` has a
      fixture that fails it, and only it, at the step expected: a sim
      whose `Serialize` refuses; a sim whose own `TickSim::hash` reads a
      `#[serde(skip)]` field; invariants that fail on A, and invariants
      that fail on the reloaded copy only; a codec that refuses
      `TickRecord` (it tells the type by `core::any::type_name`). A
      failure on the start is at step 0.
- [ ] A correct fixture with two seats passes 50 seeds of 600 steps,
      and the record `play` returns gives A's hashes through `replay`.
- [ ] `RandomInputs` gives the same inputs for the same seed, fills
      every seat from 0 to `seats - 1`, and makes only lists the latch
      could make (each release after its press, a tap's release in the
      next step, no press of an action already held).
- [ ] Shrinking is deterministic, a complete shrink is 1-minimal (no
      single step can go), and with `runs` 0 a failure comes back whole
      with `complete: false`. 2527's shrink tests still pass.
- [ ] 1119's example sim passes 50 seeds of 600 steps.
- [ ] If 1134 is done, `skills/shuyi-test/SKILL.md` shows the tick soak,
      and `cargo xtask check-skills` passes.
- [ ] `shuyi-core` still builds for `thumbv7em-none-eabihf`, and its
      tests pass without `std` (CI's `no-std` job). `soak::tick` has no
      float (`deny(clippy::float_arithmetic)` in `lib.rs` passes).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit, on fixture sims in `soak/tick/tests.rs`: one per check (`hash`,
  `round-trip`, `reloaded`, `invariant` on each copy, `record`); a
  passing sim with one seat and with two; failures at step 0 and at
  step `n`; a codec that fails on the state; a closure as `Inputs`.
- Property: `RandomInputs` is deterministic and makes only lists the
  latch could make; `play` gives the same record twice; a passing play's
  record replays to A's hashes; a shrunk failure still fails the same
  check, is no longer than its input, and fails again through `inputs`.
- Integration: 1119's example sim soaked for 50 seeds.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
