---
id: "2541"
title: "shuyi autobalance runs the search for a game's stage on every core, with a time limit, prints the changes and writes the stage back"
type: feature
milestone: After MVP
model: opus-5.5
effort: high
status: todo
blocked_by: ["2532"]
nick_input: none
completed:
---

# 2541 — `shuyi autobalance`

## Context

A request from the game.

1. **The game tickets:** game#0509, the game's autobalance
   (`github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/0509-autobalance-battles-with-bots.md`),
   and game#1183, the follow-up that moves it onto this ticket. Neither
   is a blocker of this one.
2. **The job in one sentence:** a game says what can change in a stage
   (its knobs), who plays it and what the bands are, and `shuyi
   autobalance` finds the smallest change that lands every bot in its
   band, playing the tries on every core within a time limit, prints the
   change in the knobs' words and, with `--apply`, has the game write the
   changed stage back.
3. **No friction review triggered: a missing feature, found by the
   game's retro of its last 50 PRs (2026-10-22).** 2127 gave the search
   (`autobalance`, `Knob`, `BalanceSettings`) and put Out "a `shuyi
   autobalance` contract operation (a follow-up when a game asks for
   it)" and "threads, time limits". The game is about to ask: game#0509
   plans a game-side `cargo xtask autobalance` (options, printing,
   `--apply`, about 120 lines) and, since a persona try of a real battle
   takes 2.5–45 s of one core (game ADR-0095), a search of up to 200
   runs at 30–100 tries needs a thread pool the game would write too. The
   game's persona playtest went the same way (game#0506 wrote its own
   command, threads and history; 2532, 2533 and 2021 are moving them
   back), and the game's retro asked that this one start in the engine.
4. **The generic shape:** `GameTools` gives an **autobalance plan** for
   a stage: the stage as authored (an opaque value the plan's closures
   take), its knobs (2127's `Knob` over that value), the entrants and
   checks (as 2532's playtest plan has them), `play` of one try of one
   entrant **on a given stage value**, and `write` of a changed stage
   value (for `--apply`). The command runs 2127's search with each
   evaluation's tries spread over threads (2532's pool), stops at
   `--budget` evaluations or `--time` seconds with the best setting
   seen, prints `Balanced` (the changes in the knobs' words, the reports
   before and after, why it stopped) and, with `--apply`, calls `write`.
   The result never depends on the thread count: the search is
   deterministic and each evaluation is judged once from tries gathered
   in seed order.
5. **Examples from games that aren't tactics:** a puzzle game tuning a
   level's move limit and starting pieces until its solver bots land in
   their bands; a platformer tuning a room's gap widths and enemy speeds
   for its input bots; a deck-builder tuning a fight's enemy deck.

## Nick input

None.

## Scope

**In:**
- `crates/core/src/bots/balance.rs`: the search takes its evaluations
  through a closure (`autobalance_by(stage, knobs, checks, settings,
  evaluate)`, where `evaluate(&stage_value, &Tries) -> Result<PlaytestReport, E>`);
  `autobalance` stays, as today, on top of it with `playtest::run`. The
  search itself, its order and its results don't change (2127's tests
  pass unchanged).
- A stop check between evaluations, for the time limit: the core has no
  clock (rule 5), so `autobalance_by` also takes `should_stop: impl
  FnMut() -> bool`, and stopping gives a new `Stop::Stopped` with the
  best setting seen (kept back one evaluation, as `Stop::Budget` is);
  the CLI's closure reads the clock.
- `crates/cli/src/game.rs`: `AutobalancePlan` and
  `GameTools::autobalance_plan(&self, stage: Option<&str>, input) ->
  Result<AutobalancePlan, String>` (default: unsupported), using 2532's
  `playtest_stages` for the stage names.
- `crates/cli/src/contract/autobalance.rs`: the operation: options
  `--stage`, `--entrant` (2532's), `--tries`, `--quick`, `--budget`,
  `--time <seconds>`, `--seed`, `--threads`, `--apply`, `--json`; the
  pool from 2532 (shared, not copied); the answer's text and JSON
  schema (`crates/cli/schemas/`), exit 0 when in every band, 1 when
  not.
- hello-grid implements the plan (a knob over its stage, e.g. a wall's
  count or an enemy's start), and its tests run it.
- `docs/contract.md`: the `autobalance` row.

**Out (do not do):**
- Changing the search (2127's rules).
- Writing a stage to a file (the game's `write`).
- Running autobalance in CI.
- The game's use of it (game#1183, or game#0509 if it comes after).

**The MCP server (1133, done):** its tools come from the operations'
schemas; regenerating `crates/cli/schemas/` as `docs/contract.md` says
gives the server an `autobalance` tool with no other change. Check that
`--apply` is either left out of the MCP tool or marked as writing files.

## Implementation steps

1. **Read** 2127's `bots/balance.rs` and its tests, 2532's ticket and
   code (the plan, the pool, the options), `crates/cli/src/game.rs`,
   `contract/playtest.rs`, `docs/contract.md`, and hello-grid's
   `src/tools.rs`.
2. **The core**: `autobalance_by` and `should_stop`, with 2127's tests
   unchanged and new tests for the closure (a fake evaluator) and for
   `Stop::Stopped`.
3. **The plan** in `game.rs`. The stage value is opaque to the CLI
   (`Box<dyn Any + Send + Sync>`, or a generic plan behind a trait
   object): pick the simplest shape that lets `play` and `write` take a
   changed stage without the CLI knowing its type, and say why in the
   PR.
4. **The operation**: options, the pool (move 2532's into a module both
   use if it isn't one), the clock for `--time`, the answer.
5. **hello-grid** and the fake game: a plan each; tests through the
   binary.
6. **Docs**.

## Acceptance criteria

- [ ] 2127's tests pass unchanged; `autobalance_by` with a fake
      evaluator gives the same `Balanced` as `autobalance`.
- [ ] `shuyi autobalance` on hello-grid finds a change set and prints
      it; the same with `--threads 1` and `--threads 4`.
- [ ] `--budget 3` stops with `Stop::Budget`; `--time 0` stops before the
      second evaluation with `Stop::Stopped`, keeping the best setting.
- [ ] `--apply` calls the plan's `write` with the changed stage (fake
      game test); without it, nothing is written.
- [ ] The JSON answer matches its schema; the contract's schema test
      passes.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `autobalance_by`, `should_stop`, the options parser, the answer.
- Integration: hello-grid's autobalance through the binary.
- Schema: the contract's schema test.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
