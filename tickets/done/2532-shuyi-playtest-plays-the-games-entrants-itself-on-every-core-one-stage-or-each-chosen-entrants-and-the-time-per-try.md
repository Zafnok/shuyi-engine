---
id: "2532"
title: "shuyi playtest plays the game's entrants itself: on every core, one stage or each, the entrants chosen, the game's defaults, and the time per try"
type: feature
milestone: After MVP
model: opus-5.5
effort: high
status: done
blocked_by: []
nick_input: none
completed: 2026-10-22
---

# 2532 — `shuyi playtest` plays the tries itself, on every core

## Context

A request from the game.

1. **The game ticket:** game#0506, the persona playtest bots
   (`github.com/Zafnok/visions-of-shuyi/blob/main/tickets/done/0506-persona-bots-casual-normal-hardcore.md`).
   The game's follow-up that moves onto this ticket is game#1180.
   Neither is a blocker of this one.
2. **The job in one sentence:** a game says what its playtest is (its
   stages, its named bots, its checks, how one try is played), and
   `shuyi playtest` does the rest: picks the stage and the bots asked
   for, plays the tries on every core, judges them once, times them and
   exits 1 when a bot falls under its floor.
3. **No friction review triggered:** game#0506 measured 1.8×. Nick
   (2026-10-22) asked that the generic parts of the game's playtesting
   move into the engine, so another game gets a playtest command out of
   the box; this is one ticket of that sweep (game
   `docs/engine/friction.md`, row "0506 (sweep)"). The game doesn't use
   2126's `shuyi playtest`: `GameTools::playtest` returns a finished
   report, so the engine can't spread the tries over threads, and a
   persona try of a real battle takes 2.5–45 s of one core (game
   ADR-0095). So the game wrote its own `cargo xtask playtest`
   (`crates/xtask/src/playtest.rs`, about 532 production lines):
   options, a bot choice with an "all", a thread pool with each try
   timed, the JSON file, a history with `was …` numbers and summary
   lines from named measures. This ticket and 2533 hold the generic
   parts; with both, the game keeps about 60 lines (its plan: the
   battle from its id, the personas, the targets).
4. **The generic shape:** `GameTools` gives a **playtest plan** instead
   of a report: the stages it has (or one unnamed stage), and for a
   stage its entrants by name (with named groups like "all"), its
   checks, an optional label per entrant (a mode), and a `Sync`
   function that plays one try of one entrant from one seed. The
   command splits the (entrant, seed) pairs over threads, times each
   try, gathers them in seed order and judges them with 2126's `judge`,
   so the report never depends on the thread count. A game can give its
   own defaults for `--tries`, `--seed` and `--turns`.
5. **Examples from games that aren't tactics:** a puzzle game's solver
   bots over its 40 levels (`--every-stage`, a level a stage); a
   platformer's input bots over its rooms; a deck-builder's personas
   over its fights. Each has slow bots and many cores, and each would
   write this same thread pool and options parser.

## Nick input

None.

## Scope

**In:**
- `crates/cli/src/game.rs`: `PlaytestPlan`, `PlaytestStage`,
  `GameTools::playtest_stages`, `GameTools::playtest_plan` and
  `GameTools::playtest_defaults`, replacing `GameTools::playtest` (a
  breaking change, fine before the first crates.io release, 2005;
  the game never implemented it).
- `crates/cli/src/contract/playtest.rs`: the new options, the thread
  pool, the timing, the answer's new part; its usage text; its tests.
- `crates/cli/src/contract/fake.rs`, hello-grid's `src/tools.rs` and the
  templates' `crates/game/src/tools.rs` (if they implement `playtest`)
  move to the plan.
- `docs/contract.md`: the `playtest` row and the game's parts.
- The answer's JSON schema (`crates/cli/schemas/playtest.output.json`
  and `versions.json`, as the contract's schema test asks).

**Out (do not do):**
- The history, `was …` lines and summary lines from measures: 2533.
- Targets by tier: 2129 (the game builds its checks with it).
- A CI workflow: 2021.
- `shuyi autobalance`: 2541, which reuses this ticket's thread pool, so
  keep the pool in a module of its own (not inside the playtest
  operation).
- Stopping a try early, or slicing a try over frames (3009).
- The game's use of it (game#1180).

**The MCP server (1133, done):** its `playtest` tool takes the
operation's input schema, so regenerating `crates/cli/schemas/` as
`docs/contract.md` says gives the tool the new input with no other
change.

## Implementation steps

1. **Read** `crates/cli/src/contract/playtest.rs`, `game.rs`,
   `contract/bots.rs` (`MAX_GAMES`, `game_arg`), hello-grid's
   `src/tools.rs` (`playtest`, `playtest_run`, `ENTRANTS`), 2126's
   `play_tries` and `judge`, and the game's `crates/xtask/src/playtest.rs`
   (`play_on_threads`, `Options`, `Bot`) for what to keep.
2. **The plan** (`game.rs`):
   ```rust
   pub struct PlaytestPlan {
       /// Shown at the top of the report: the stage and its tier, say.
       pub title: String,
       /// Every entrant, in report order: its name and a label (a mode), if any.
       pub entrants: Vec<(String, Option<String>)>,
       /// Named groups of entrants (`"all"`), for `--entrant`.
       pub groups: BTreeMap<String, Vec<String>>,
       /// The entrants played when `--entrant` isn't given (all if empty).
       pub default_entrants: Vec<String>,
       /// The checks (2126's), over the entrants that play.
       pub checks: Box<dyn Fn(&[&str]) -> Vec<Check> + Send + Sync>,
       /// One try: entrant index, seed, the turn it stops at.
       pub play: Box<dyn Fn(usize, u64, u32) -> Result<Tried, String> + Send + Sync>,
   }
   ```
   `GameTools::playtest_stages(&self) -> Vec<String>` (default empty:
   one unnamed stage); `playtest_plan(&self, stage: Option<&str>, input: &PlaytestInput) -> Result<PlaytestPlan, String>`
   (default: unsupported, as today); `playtest_defaults(&self) ->
   PlaytestInput` (default `PlaytestInput::default()`). 2126's
   `play_tries` over a one-seed range is how a game writes `play`.
3. **Options** (`parse_args`), all optional: `--stage <id>` (must be one
   of `playtest_stages`; required when there are several, unless
   `--every-stage`), `--every-stage` (each stage in order, one report
   each), `--entrant <name>` (repeatable; a group name expands; unknown →
   an error listing the names), `--threads <n>` (default
   `std::thread::available_parallelism`), `--report <path>` (also write
   the JSON answer to a file), and the existing `--tries`, `--seed`,
   `--turns`, game options and `--json`. Unset numbers come from
   `playtest_defaults`.
4. **The pool**: the work list is every (entrant, seed) pair; thread `w`
   takes items `w, w + n, w + 2n …` (`std::thread::scope`), times each
   try with `Instant`, and returns `(entrant, Tried, Duration)`. A
   panic in a worker is resumed on the main thread; a `play` error stops
   the command with "<entrant>: try <seed>: <error>". Gather per
   entrant, sort by seed, `judge` once. The clock lives in the CLI only
   (the core stays free of it).
5. **The answer**: `PlaytestOutput { title, report: PlaytestReport,
   entrants: Vec<Played> }` with `Played { name, label, seconds_per_try:
   f64 }`; the text is the title, 2126's table, then one line per entrant
   "`<name> · <label> · <s> s per try`". `ok` is `!report.blocked`, so
   the exit code is 1 when a rate fell under its floor (as today).
   `--every-stage` answers with a list, and is `ok` only if every stage
   is.
6. **hello-grid** moves to the plan (its two entrants, `playtest.ron`'s
   checks, one stage); its `tests/it/playtest.rs` gains a test that
   `--threads 1` and `--threads 4` give the same report.
7. **Docs**: `docs/contract.md` and the usage text.

## Acceptance criteria

- [x] `shuyi playtest` on hello-grid gives the same report as before
      this ticket (its existing test), and the same with `--threads 1`
      and `--threads 4`.
- [x] `--entrant walk` plays only `walk`, and its checks are only those
      `checks(&["walk"])` returns; `--entrant nobody` fails listing the
      names.
- [x] A fake game with two stages: `--stage` missing fails naming both;
      `--every-stage` answers two reports and exits 1 when either is
      blocked.
- [x] A fake game whose `playtest_defaults` sets `seed: 1, turns: 60`
      plays from seed 1 to turn 60 when neither flag is given.
- [x] `--report out.json` writes the same JSON as `--json` prints.
- [x] Every entrant has a `seconds_per_try` in the answer and its line.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `parse_args` (every option, bad values, unknown names), the pool
  on a fake game with a slow `play` (order by seed whatever the thread
  count), the answer's text.
- Integration: hello-grid's `tests/it/playtest.rs` through the binary.
- Schema: the contract's schema test with the new output.

## Completion notes

- **The plan.** `GameTools::playtest` is gone; a game gives
  `playtest_stages`, `playtest_plan` and `playtest_defaults`, and
  `shuyi_cli::PlaytestPlan` is the ticket's struct (its two closures named
  as the types `PlaytestChecks` and `PlayTry`, which clippy's
  `type_complexity` asks for).
- **The pool** is its own module, `shuyi_cli::pool::run`, for 2541 to
  reuse. It hands back the answers in work order. Once an item fails no
  thread starts a later one, but every earlier one still runs, so the
  error reported is the first failing try in seed order whatever the
  thread count. A worker's panic is resumed on the main thread.
- **The answer** is `Answer<Vec<PlaytestOutput>>`: a list on every run, one
  report a stage, with `stage`, `title`, `report` and `entrants`
  (`Played { name, label, seconds_per_try }`). One shape whether or not
  `--every-stage` is given. Contract version 11.
- **Deviations:**
  - `playtest_defaults` returns the core's `Tries` (count, seed,
    turns), not a `PlaytestInput`. The input's `tries`, `seed` and
    `turns` are now `Option`s, so "not given" can be told apart, and the
    defaults are three plain numbers, `DEFAULT_TRIES` (100 from seed 0
    to turn 200) unless the game has its own. `playtest_plan` gets the
    input with the numbers filled in.
  - `--report` is a command-line option like `--json`, not a field of the
    input: an MCP caller already gets the JSON. The path is read from the
    project's folder, like `bake --out`. If the file can't be written,
    the answer keeps its result and fails with `SH3004`.
  - `shuyi_core::bots::playtest::Check::entrants` is now public, so a
    game can keep the checks whose entrants all play (hello-grid does).
  - Before any try is played, a check that names an entrant who doesn't
    play fails the run (judging no tries says so).
- **hello-grid** gives a plan (`tools::playtest_plan`): one stage, its two
  entrants, and `playtest.ron`'s checks filtered to the entrants that
  play. Its tests through `main_with` cover `--threads 1` and `4`,
  `--entrant walk`, `--entrant nobody` and `--report`.
- **Tickets that named this one:** 1134's either-order clause drops 2532,
  2541's step 4 names the pool's module, and 2533's history record names
  one stage's `PlaytestOutput`.
- **The game:** it never implemented `GameTools::playtest`, so its next
  pin bump builds as it is. game#1180 moves its `cargo xtask playtest`
  onto this (no game PR yet).
- No follow-up tickets.
