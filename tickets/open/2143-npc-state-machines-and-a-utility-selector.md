---
id: "2143"
title: A game writes NPC and enemy behaviour as a deterministic state machine, and picks among choices with a utility selector over weights from a file
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2143 — NPC behaviour as a state machine and a utility selector

## Context

Found by the blind-spot audit of 2026-11-23 (gameplay frameworks). No
friction review: a missing feature, not a request from the game.

Every game with enemies or townsfolk writes the same thing: a guard
that patrols, chases on sight, attacks in range and gives up after a
while; a shopkeeper that is open, closed or restocking; a platformer's
walker that turns at a ledge and is stunned when hit. Today an agent
writes that by hand each time, as a match over a state enum with a
counter beside it. The engine has nothing for it:

- `crates/core/src/lib.rs` has no state machine module. The only state
  machines the tickets plan are for animation (2104, 2134, parked), the
  graph panel (2508, parked) and visual scripting (1135, research).
- The engine's AI is search bots over `TurnRules` (`shuyi_core::bots`:
  rollout, turn planner, playtest, autobalance). 2113 puts "behaviour
  trees, utility AI ... and anything real-time" out (its **Out**).
- Utility scoring is planned only inside the parked RTS commander
  (2715, `shuyi_rts::ai::utility`), with its own response curves in
  `Fx`.
- `shuyi_core::bots::weigh` (2130, done) already scores named integer
  counts against weights read by name from a file (`Weights::from_named`,
  `Weights::score`, saturating, no floats). That is most of a utility
  scorer, and nothing uses it for NPCs.

Godot users reach for a state machine node or LimboAI, Unity 6 ships a
Behavior package, Bevy has `States`, Unreal has StateTree. The cheapest
of these that still keeps a game's code looking ordinary is a small
state machine over the game's own enum, plus a selector that picks the
best of a few choices.

Both must keep the core's rules (ADR-0002 §2, ADR-0032): pure, integer,
no clock, no floats. Time in a state is counted in steps or turns, as
`shuyi_core::timer` counts (2115). A machine is plain serde data, so it
lives in the game's state, its hash goes in `state_hash`, and a replay
or a save keeps it.

This works today in turn games and in tick games (`TickSim`,
`crates/core/src/tick.rs`; the `fixed-ticks` answer of
`docs/modules.ron` is ready). It needs none of the parked real-time
chain (1119, 1120, 1122).

## Nick input

None.

## Scope

**In:**
- **`shuyi_core::fsm`**, a new module of `shuyi-core` (no new
  dependency: serde only, already allowed by `tools/layers.ron`):
  - `Machine<S>`: the state a thing is in (`S`, the game's own enum or
    any `Clone + PartialEq` type), the steps it has been in it, and the
    state before. `Machine::new(initial)`, `state()`, `previous()`,
    `steps_in()`, `advance(n)` (adds `n`, saturating), `go(to)`
    (returns a `Switch { from, to }` and sets `steps_in` to 0, or
    `None` when `to` is the state it is in), and `restart()` (sets
    `steps_in` to 0, for an attack that starts again). Serialize and
    Deserialize; `#[serde(deny_unknown_fields)]`.
  - `Switch<S>`: what `go` and `Transitions::fire` return. A game reads
    it as its enter and exit: `from` is the state left, `to` the one
    entered. No callbacks: the game matches on it, so the order of what
    happens stays in the game's code.
  - `Transitions<S, E, C>`: the rules as data, tried in the order
    written. Each `Rule { from, when, to }`:
    - `from`: `In::State(S)` or `In::Any` (not named `From`, which is
      the standard trait);
    - `when`: a list of `When<E, C>` that must all hold: `On(E)` (one of
      this step's events equals it), `After(n)` (`steps_in() >= n`),
      `If(C)` and `Unless(C)` (a condition of the game's own type,
      answered by a function the game passes in). An empty list always
      holds.
    - `to`: the state to go to. A rule whose `to` is the state already
      in is skipped, so `Any -> Stunned` does not enter `Stunned` again
      each step.
  - `Transitions::fire(&self, machine, events: &[E], holds: impl FnMut(&C) -> bool) -> Option<Switch<S>>`:
    at most one rule fires per call, the first that matches. It
    allocates nothing.
  - `Transitions::shadowed(&self) -> Vec<usize>`: the rules that can
    never fire, because an earlier rule has the same `from` (or `Any`)
    and a subset of its `when`. A test or `shuyi check` of the game's
    own reads it; the engine never refuses such a file.
  - **The RON form** is the serde form of `Transitions<S, E, C>`, with
    the game's enums by variant name. A game reads it with
    `shuyi_assets::ron::Ron::<Transitions<S, E, C>>::named("fsm", &["fsm.ron"])`
    (or `load_ron`), so an unknown state, event or condition fails with
    `file:line:col` as every RON file does. No new asset kind: the
    table is generic over the game's types, like a personas file
    (2130), and `docs/formats.md`'s row "its own" already covers it.
- **`shuyi_core::utility`**, a new module:
  - `Selector<const N: usize>`: a `Weights<N>` (from `bots::weigh`,
    reused, not copied) and `stay: i32`, a bonus the current choice
    gets so an NPC does not flip between two close choices every step.
    `Selector::new(weights, stay)` and
    `Selector::from_named(names, by_name, stay)` through
    `Weights::from_named` (its `UnknownFeature` error).
  - `Selector::pick(&self, current: Option<usize>, options: &[[i64; N]]) -> Option<Pick>`:
    each option's counts scored by `Weights::score`, `stay` added to
    `current`'s (saturating), the highest wins, a tie goes to the lower
    index. `None` for no options. `Pick { index, score }`. It
    allocates nothing.
  - What a count means and how it is counted (distance to the player,
    health lost, ammo) stay the game's, as in `bots::weigh`. Response
    curves stay out: a game bends a count before it hands it over.
  - A selector file: `SelectorFile { weights: {name: i32}, stay: i32 }`,
    read through `Ron<T>` as above, `#[serde(deny_unknown_fields)]`.
- **Using both together** is the documented pattern: the selector picks
  which of a few states an NPC wants, and `Machine::go` takes it there;
  the rules handle what must happen whatever the scores say (stunned
  on a hit, back to patrol after a time).
- **New public types follow ADR-0039:** `Machine`, `Transitions`,
  `Selector` and `SelectorFile` are `#[non_exhaustive]` with the
  constructors above. `Rule`, `In`, `When`, `Switch` and `Pick` are
  plain data a game builds and matches: `When` is `#[non_exhaustive]`
  (more conditions may come), and `Rule`, `Switch` and `Pick` are
  `#[non_exhaustive]` with a `new`. Everything is additive: no break.
- **An ADR** (the `write-adr` skill) for the shape: rules as data over
  the game's enums, conditions answered by the game, at most one rule a
  step, enter and exit as a returned `Switch`, and why behaviour trees
  wait.
- **A golden row** in `shuyi-core`'s tests: a guard machine with a
  selector, stepped 300 steps over a scripted list of events, its
  `state_hash` every 30 steps, in `crates/core/tests/golden/fsm.ron`
  read by a test named `golden`. CI's `determinism` job runs
  `shuyi-core`'s integration tests on wasm32 already.

**Out (do not do):**
- Behaviour trees and GOAP. They wait until a game asks; the ADR says
  why.
- Animation state machines (2104, 2134, parked). They pick clips by
  parameters inside `shuyi-assets`; whether 2104 is later built on
  `Machine` is that ticket's choice when it is unparked.
- The graph panel over a state machine (2508, parked) and visual
  scripting (1135).
- The RTS commander's utility layer (2715, parked). It keeps its own
  response curves in `Fx`; it may use `Selector` when it is unparked.
- Search bots for turn games (2113). They play a whole game; this
  ticket is for things inside one.
- Story variables, event sequences and quests (2140, 2141), and stats
  and effects (2142). A game can drive a machine from them; none of them
  is needed here.
- Script-written rules (2146): a script may call these later, through
  the script crate's own ticket.
- Hierarchical or parallel states. A game nests by keeping two
  machines; a follow-up when a game asks.
- Steering, paths and movement (2106, 2122, 1122). The machine says
  what an NPC wants; how it moves is the game's or those tickets'.
- Any change to `bots::weigh`'s behaviour or its public items.

**Estimated size:** about 1,100 lines (code 450, tests 450, docs 200).

**Split:** past 900 lines, split off `shuyi_core::utility` (steps 5–6
and its tests) into a new ticket that names this one as done.

## Implementation steps

1. Write `crates/core/src/fsm.rs`: `Machine`, `Switch`, with
   `advance`, `go` and `restart`. Doc comments say steps or turns, as
   `timer` does.
2. Add `Rule`, `In`, `When` and `Transitions` in the same module,
   with `fire` (first match in order, skip a rule to the current state,
   at most one switch) and `shadowed`.
3. Write the module's doc: a guard with `Patrol`, `Chase`, `Attack`
   and `Stunned`, its rules in RON and in Rust, and the loop a game
   writes (collect events, `advance(1)`, `fire`, match the `Switch`).
   Make it a doctest.
4. Add `pub mod fsm;` to `crates/core/src/lib.rs` and a line for it in
   the crate's doc list. Its items stay in `fsm` (no root re-export,
   like `bots::playtest`).
5. Write `crates/core/src/utility.rs`: `Selector`, `Pick` and
   `SelectorFile`, over `bots::Weights`.
6. Add `pub mod utility;` and its doc line, with a doctest: a guard
   that picks between chase, flee and call for help from three counts.
7. Write `crates/core/tests/fsm.rs`: the RON form read with `ron`
   (already a dev-dependency), an unknown variant failing, and the
   combined guard of the Scope.
8. Write the golden table `crates/core/tests/golden/fsm.ron` and its
   test, as `tests/golden.rs` does for the toy dot.
9. Write the ADR.
10. Document with the `document` skill: where an agent making a game
    looks first for "enemy behaviour" (the shipped skill that fits,
    `skills/shuyi-stage-grid` or a new section of the one the skill's
    map names), and the book's chapter.

## Acceptance criteria

- [ ] `Machine::go` to the state it is in returns `None` and keeps
      `steps_in`; to another returns `Switch { from, to }` and sets
      `steps_in` to 0.
- [ ] `advance(u32::MAX)` twice leaves `steps_in` at `u32::MAX`.
- [ ] `fire` tries rules in the order written: with two rules that both
      match, only the first fires, and one call switches at most once.
- [ ] An `In::Any` rule to the current state is skipped and the next
      matching rule fires.
- [ ] `After(30)` holds at `steps_in() == 30` and not at 29.
- [ ] `On(e)` holds only when `e` is among the events passed;
      `If(c)`/`Unless(c)` read the game's function, called only for the
      rules tried.
- [ ] `shadowed` names a rule after an identical earlier one, and a
      rule after an `Any` rule whose conditions are a subset of its own;
      it names none in the guard's table.
- [ ] A table written in RON with the game's enums reads back equal to
      the one built in Rust; a misspelt state fails with its line and
      column.
- [ ] `Selector::pick` returns the highest score; a tie goes to the
      lower index; `stay` keeps the current choice when another is
      ahead by less than `stay`, and not when it is ahead by more;
      `None` for no options.
- [ ] `Selector::from_named` with a name the game didn't declare fails
      with `UnknownFeature`.
- [ ] `fire` and `pick` allocate nothing: an integration test counts
      the heap's calls with `shuyi-alloc-count` (a new dev-dependency of
      `shuyi-core`), as `crates/assets/tests/walk_alloc.rs` does, one
      test to its file.
- [ ] The golden table `fsm` passes natively and on wasm32 in CI's
      `determinism` job.
- [ ] `cargo xtask check-layers` passes with no new outside crate for
      `shuyi-core`, and the module builds without `std`
      (`cargo xtask check-no-std`).
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `Machine` (go, restart, advance saturating, previous);
  `fire` (order, `Any`, skip to self, each `When`, empty `when`);
  `shadowed`; `Selector::pick` (ties, `stay`, saturating scores, empty).
- Property: for random tables, events and condition answers, `fire`
  switches at most once and never to the state it is in; the same
  inputs from the same machine give the same `Switch` and the same
  `state_hash`.
- Snapshot / integration: `crates/core/tests/fsm.rs` (the RON form, the
  combined guard); the `fsm` golden table; the doctests of both
  modules.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
