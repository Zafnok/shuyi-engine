---
id: "2112"
title: "A level solver: breadth-first search over any TurnRules game, proving a level can be won and giving its shortest line"
type: feature
milestone: After MVP
model: sonnet-5
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2112 — A level solver over TurnRules

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8. None of them ships a search over a turn game's states.
Their AI tools are for real-time agents: Unreal's Behavior Trees,
StateTree and EQS, and Unity's Behavior package. Bevy and Godot have
none built in. So this is an edge, not a catch-up.

Shuyi today:

- `crates/core/src/turn.rs`: `TurnRules` (`check`, `apply`,
  `legal_commands`, `outcome`), the interface a search needs. Its
  contract lets a game leave commands out of `legal_commands` when there
  are too many to list.
- `crates/core/src/hash.rs`: `state_hash`, the same on every platform,
  which a search can use to skip states it has seen. It fails with a
  `HashError` when the state's own `Serialize` refuses.
- `crates/core/src/bots.rs`: `Bot`, `play`, `play_observed` and
  `RandomBot`. Nothing in the engine searches.
- `crates/core/src/path.rs`: Dijkstra over grid tiles, not over game
  states.
- `templates/grid/crates/game/src/tools.rs`: `check` returns the files'
  load errors only. Nothing proves the stage can be won.
- `crates/cli/src/game.rs:103`: `GameTools::check` returns the
  `ContentError`s of any validator the game runs, so a game can report
  an unwinnable level there with no change to the contract.

Plans that need it:

- 3009 slices "the tactics bot's search" over frames, but the carve
  brought only `Bot`, `play` and `RandomBot` into the engine
  (`docs/carve.md`, the game's 1116). The engine has no search to slice.
  This ticket gives it one, written so it already runs in slices, and
  3009 implements `Think` for it.
- 2113's minimax and MCTS bots go in the same module, after this ticket.
- 2803's step 9 is "a bot that solves level 1" of a box-pushing puzzle,
  which it would otherwise write by hand.

Why it matters. Visions of Shuyi doesn't ask for it: its enemy AI is its
own. It serves the engine's own plans first (3009, 2803, the grid
template), then every puzzle game: a level that an agent or a designer
writes is proved winnable in `shuyi check`, and the proof is a command
list that doubles as a replay. That serves the end goal of 2026-10-19,
which until the launch is measured on turn-based games
(`docs/decisions.md`, 2026-10-20): an agent asked for a puzzle game can
prove each level it writes.

## Nick input

None.

## Scope

**In:**
- **`shuyi_core::search`** (`crates/core/src/search.rs`, tests in
  `crates/core/src/search/tests.rs`), `no_std` with `alloc`, beside
  `bots` and `path`. It stays in `shuyi-core` and isn't a module: it
  adds no dependency, and its generic code is compiled only in a game
  that calls it, so a game that never names it pays nothing for it in
  size (ADR-0009 §1's test).
  - `Limits { states: u32, depth: u32 }`: the most distinct states
    kept, and the longest line in commands. `Default` is 100 000 states
    and no depth limit (`u32::MAX`).
  - `Solve<C>`:
    - `Solved { commands: Vec<C>, states: u32 }`: a shortest line from
      the start to an outcome the goal accepts;
    - `Exhausted { states: u32 }`: every line among the listed commands
      was tried, and none reaches the goal;
    - `LimitReached { states: u32 }`: a limit stopped the search first.
      It is never a pass.
  - `SolveError<C>`: `Hash(HashError)` (the state's own `Serialize`
    failed), or `Refused { line: Vec<C> }`: the game refused a command
    its own `legal_commands` listed, which is a bug in the game. `line`
    reaches the state and ends with the refused command, so replaying
    it shows the game's error.
  - `Solve` and `SolveError` derive `Debug, Clone, PartialEq, Eq`.
  - `Solver<R, G>`, where `R: TurnRules + Clone + Eq + Serialize`,
    `R::Command: Clone`, and `G: Fn(&R::Outcome) -> bool` says which
    outcomes are the goal:
    - `Solver::new(start: R, goal: G, limits: Limits) -> Self` does no
      work. The start is hashed and checked by the first `step`;
    - `Solver::step(&mut self, units: u32) -> Result<Option<Solve<R::Command>>, SolveError<R::Command>>`:
      expands up to `units` states, and returns `None` while it is not
      done. A unit is one state taken from the queue and expanded.
      `step(0)` does nothing. Once it has answered, `step` returns the
      same answer again and expands nothing. Its whole state (the
      queue, the states seen, the parent links, the answer) lives in
      the `Solver`, so 3009 can implement `Think` for it with no
      change.
  - `solve(start: &R, goal: G, limits: Limits) -> Result<Solve<R::Command>, SolveError<R::Command>>`:
    a clone of `start` in a `Solver`, stepped to the end.
- **The rules of the search**, written in the module docs:
  - breadth first, so the line found is a shortest one in commands;
  - children in `legal_commands` order, and the goal checked when a
    state is made, so among shortest lines the first in that order wins,
    every time;
  - a state seen before is skipped. Its `state_hash` finds it in a
    `BTreeMap<StateHash, Vec<u32>>` of indexes into the states kept,
    and full equality (`Eq`) confirms it, so a hash collision never
    drops a state;
  - `states` counts the start and every distinct state made after it.
    The search keeps every state it made (the `Eq` check needs them),
    so `Limits::states` bounds its memory too. When a new distinct
    state would pass the limit, the answer is `LimitReached`;
  - a start whose outcome the goal accepts is `Solved` with no
    commands. A finished state the goal refuses is a dead end, and so is
    a state with no listed commands;
  - a state `depth` commands from the start that isn't finished is not
    expanded, and counts as cut. If any state was cut, the answer is
    `LimitReached`, never `Exhausted`;
  - **`Exhausted` proves a level can't be won only if the game lists
    every command.** `TurnRules` lets a game leave some out, and such a
    game reads `Exhausted` as "not found among the listed commands";
  - **luck is part of the state.** The search plays the state as it
    stands, its `SimRng` included, so a line is a line for that seed.
    That is right for a puzzle level and its seed. It says nothing about
    other seeds;
  - **every command is the solver's.** It picks the commands of every
    side. For a game with an opponent, `Solved` means "some line of play
    reaches the goal", not "a side can force it" (2113's minimax answers
    that);
  - a state that keeps its turn number is never equal to a state of
    another turn, so the states kept grow with the depth, and a level
    whose pieces can walk forever never runs out of states. `Limits`
    bounds that, and the answer is then `LimitReached`.
- **The grid template proves its stage can be won.** In
  `templates/grid/crates/game/src/tools.rs`:
  - `pub fn prove_stage(map: &GridMap<TerrainId, Feature>, limits: Limits) -> Result<Vec<ContentError>, String>`
    solves `Stage::new(map, 0)` (seed 0, the seed `--seed` defaults to
    and the first stage's) with the goal `Outcome::Won { .. }`:
    - `Solved` gives no error;
    - `Exhausted` gives a `ContentError` on `stage.map`: "the stage
      can't be won on seed 0: every line among N states was tried";
    - `LimitReached` gives one too: "the stage isn't proved winnable on
      seed 0: the search stopped at N states";
    - a `SolveError` is a bug in the rules, not in the files, so it is
      an `Err`.
  - `check` returns the load errors as today when the files don't load.
    When they do, it returns `prove_stage(&data.stage_map,
    Limits::default())`.
  - The template's `legal_commands` lists every open step, so
    `Exhausted` is a proof there. Its `Stage` keeps the turn, so a goal
    walled off from a hero who can still walk gives `LimitReached`, and
    `Exhausted` comes only when every line ends caught or stuck.
  - If the shipped stage can't be won on seed 0, fix the stage in this
    PR.
- **Either order with 2803:** if `site/tutorial/` exists, its `09-test`
  step (and `10-ship`, which carries it) gets level 1's line from
  `solve` instead of a line kept in its files, a test proves its five
  levels winnable, and the step's page in `site/content/learn/tutorial/`
  shows the call. If 2803 isn't done, its step 9 already says to use
  `solve` when it exists.
- **Either order with 1134:** if `skills/shuyi-test/SKILL.md` exists,
  add a short section: proving a level winnable with `solve` in a game's
  `check`. If not, 1134 writes that section.
- **Either order with 2802:** if `site/content/learn/book/` exists,
  chapter 8 (Testing) gets a short section on `solve`, its code a
  snippet in `site/snippets/` that CI builds and tests. If not, 2802
  writes that section.

**Out (do not do):**
- bots that play one side against another, and hiding luck or hidden
  things from the searcher (`Determinize`): 2125 and 2113;
- a `Bot` that plays by `solve`: it would see the luck to come, which
  `Bot`'s docs forbid. A test plays the line `solve` returns instead;
- the `Think` trait (3009, which implements it for `Solver`);
- a `levels` hook in `GameTools`, or a new field in the contract's
  `check` answer. `check` already returns `ContentError`s, so no schema
  version changes;
- solving hello-grid's stage: it has a foe, whose commands the solver
  would pick for it;
- A*, heuristics, a key that leaves part of the state (such as a move
  counter) out when states are compared, or a stored table of
  positions: a follow-up if a game's levels pass the limit;
- the mystery fair-play check (2731, parked). It searches an abstract
  case graph, not `TurnRules` states.

## Implementation steps

1. Read `turn.rs`, `bots.rs`, `hash.rs` and `path.rs` for the house
   style, and `path/tests.rs` for the layout of tests in a folder. Follow
   ADR-0039 (`#[non_exhaustive]` and constructors) for `Limits`,
   `Solve` and `SolveError`.
2. Write `search.rs`: the module docs with the rules above, the types,
   `Solver::new`, `Solver::step` and `solve`. Keep the hash function a
   private field of the `Solver`, of type `fn(&R) -> Result<StateHash,
   HashError>` and set to `state_hash` by `new`, so a test in
   `search/tests.rs` can set it after `new` to give every state the
   same hash.
3. Add `pub mod search;` and the re-exports (`Limits`, `Solve`,
   `SolveError`, `Solver`, `solve`) to `crates/core/src/lib.rs`, and a
   line on the module in the crate docs.
4. Write the tests below, with small fixture games in the tests file.
5. Add `prove_stage` to the grid template's `tools.rs` and call it from
   `check`, with its tests in `tests/it/main.rs` (maps built with
   `rules::parse_map`). Add the proof to the module docs of `tools.rs`
   and to the `cargo shuyi check` row of `templates/grid/AGENTS.md`.
   `templates/` isn't in the workspace, so `cargo fmt --all` doesn't
   format it: run the template's gates by hand (`cargo run -p shuyi-cli
   -- new grid <dir> --engine-path <repo>`, then fmt, clippy, test and
   `cargo shuyi check` in `<dir>`), as CI's `templates` job does.
6. Do the either-order parts that apply, and run the gates.

Size: if the diff runs well past 600 changed lines, keep the solver
and the template's proof here, and move the either-order parts that
apply (with 2803, 1134 and 2802) to a follow-up ticket written in
this PR (the `write-ticket` skill, blocked by this one). A part
applies only when its ticket is done, and a done ticket's clause
can't carry it any more. A part whose ticket is still open needs
nothing: that ticket's own clause does it.

## Acceptance criteria

- [ ] `solve` returns a shortest line, and among shortest lines the
      first in `legal_commands` order (a fixture with two shortest
      lines).
- [ ] `Exhausted` comes only when no limit cut anything (a fixture whose
      goal lies past the depth limit gives `LimitReached`).
- [ ] With every state given the same hash, every fixture gives the same
      answer as with `state_hash`.
- [ ] Stepping by 1, by random slice sizes and by `u32::MAX` gives the
      same answer, and a `step` after the answer gives it again
      (property test).
- [ ] A listed command the game refuses gives `SolveError::Refused` with
      its line.
- [ ] In a game made from the grid template, `cargo shuyi check` passes
      on the shipped stage.
- [ ] The template's tests show `prove_stage` giving one `ContentError`
      on `stage.map` for a hero walled in alone (`Exhausted`) and for a
      goal walled off from a hero who can walk (`LimitReached`, with
      small limits).
- [ ] `shuyi-core` still builds for `thumbv7em-none-eabihf` (CI's
      `no-std` job).
- [ ] The book's chapter is updated (2802), or the completion notes say
      why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit, on fixture games in `search/tests.rs` (for example a counter
  that `+2` and `×3` commands move toward a target, and a small maze):
  known shortest lines; ties by `legal_commands` order; a start already
  won; a goal that can't be reached (`Exhausted`, with its exact state
  count); the depth and state limits; a refused listed command; a state
  whose `Serialize` fails (`SolveError::Hash`); every state given one
  hash; `step(0)`, and `step` after the answer.
- Property: on random small mazes, a `Solved` line replayed through
  `apply` ends in an outcome the goal accepts; no shorter line exists (a
  brute-force search to that depth); slicing doesn't change the answer;
  solving twice gives the same answer.
- Integration: the grid template's `check` on its stage, and
  `prove_stage` on a hero walled in alone and on a goal walled off.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
