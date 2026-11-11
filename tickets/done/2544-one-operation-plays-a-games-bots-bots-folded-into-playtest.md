---
id: "2544"
title: "One operation plays a game's bots: `bots` folded into `playtest`, which answers each try's commands and record for `replay`"
type: feature
milestone: MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: done
blocked_by: ["2532", "2533"]
nick_input: none
completed: 2026-11-06
---

# 2544 — One operation plays a game's bots: `bots` folded into `playtest`

## Context

The tool contract has two operations that play a game's stage with its
bots, side by side:

- **`bots <games>`** (1129, `crates/cli/src/contract/bots.rs`): the game
  plays `games` seeded plays (`GameTools::bots`), and `tally` sums them:
  each play's seed, outcome, turns, commands, measures and record for
  `replay`, the outcomes counted, and the spread of turns and of each
  measure. The grid template implements it; it is the template's bot
  test.
- **`playtest [--tries N]`** (2126, `contract/playtest.rs`, over
  `shuyi_core::bots::playtest`): each of the game's named bots plays N
  tries from the same seeds, and the game's checks judge them. Hello,
  Grid implements it; the templates don't.

2126 kept `bots` on purpose ("moving `bots`/`tally` onto this runner
(they stay; a follow-up may merge them if both are kept)"), but no ticket
took the follow-up. A retro of PRs #38–#68 (2026-10-22) found what the
two copies cost:

- their inputs say the same thing in different words: `BotsInput {
  games, seed, turns, start }` and `PlaytestInput { tries, seed, turns,
  start }`, both a copy of core's `Tries { count, seed, turns }`; Hello,
  Grid maps one to the other by hand (`src/tools.rs`);
- one play is `bots::Play` in one and `playtest::Tried` in the other,
  near-twins, and they write an unfinished play differently (`bots`' `UNFINISHED`
  constant against the outcome's `Debug` and a literal);
- Hello, Grid counts its foe's trips twice, once in `bot_play` and once
  in its `StageMeasure`;
- 2527 (`shuyi soak`) plans a third operation whose "input keeps `bots`'
  fields and limits".

2532 rewrites how a game gives its playtest (`PlaytestPlan`,
`GameTools::playtest_plan`, `playtest_stages`, `playtest_defaults`,
replacing `GameTools::playtest`) and plays the tries on every core. 2533
adds a history and summary lines from the game's named measures. This
ticket comes after both, so `bots` is folded once into the shape they
leave.

**The game is affected now.** Since game#1193 Visions of Shuyi
implements `GameTools::bots` (`crates/xtask/src/game_tools.rs` and
`game_tools/plays.rs`), and the game/engine boundary audit of
2026-11-03 found its `bots` copying `play_tries`' loop only to keep each
play's commands for its record, since `Tried` drops them. Its follow-up
is [game#1240](https://github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1240-bot-play-records-from-the-engines-bots-operation.md):
after this ticket it gives its bot run as a plan whose try writes the
record, and drops `GameTools::bots`. The game ticket is not a blocker.
The game ticket game#1195 (stamping those records with the build that made them) says
where the stamp goes in either order with game#1240.
So a plan must be able to give a try's record from the try's seed and
commands (a hook on the plan or its entrant), and the PR that moves the
game's pin past this one follows it.

## Nick input

None.

## Scope

**In:**
- **`playtest` answers what `bots` answered.** Each try carries its
  `commands` (how many were applied) and its `record`, in the game's
  record format, for `replay`. A game whose plan gives no record leaves
  it out, as `bots` does today. The answer counts the outcomes per
  entrant, and has the spread of turns and of each measure per entrant
  (`bots::Spread`'s `min`, `max`, `total`, `mean`). 2533's summary items
  (`playtest::Summary`: `Sum`, `Spread`, `Most`, `Median`, `Max`) don't
  give it: they are the few numbers a game chooses per entrant, with no
  `min`, `mean` or outcome counts, so this answer adds those beside them.
- **A plan with no checks is a plain bot run.** With no checks the
  answer judges nothing and `ok` is true when every try played; with one
  entrant it is what `bots` was. The text output for that case is
  `playtest`'s line per try (`PlaytestReport::write_table`), then the
  outcome counts and the spreads that `bots`' text prints
  (`bots::text`, `crates/cli/src/contract/bots.rs`, which has no
  per-try line).
  **Either order with 2557** (a `turns` measure printed once): if 2557
  is done, every try line this ticket writes keeps its rule (a measure
  named `turns` equal to the try's own turns is left out), with its
  tests passing; if not, nothing to do (2557 applies it to every try
  line there is when it lands).
- **`bots` goes:** `Op::Bots`, `contract/bots.rs` (`Spread` moves to
  `playtest` if it is still needed), `GameTools::bots`,
  `crates/cli/schemas/bots.*.json`, the `bots` subcommand and its usage
  text, its MCP tool (from `Op::ALL`), and its rows in `docs/contract.md`
  and the root `README.md`. A breaking change: the completion notes
  carry a `Breaking:` line for `shuyi-cli` (`GameTools::bots` and the
  `bots` operation go; a game gives its bot as a playtest plan with
  one entrant and no checks). The game implements `GameTools::bots`;
  game#1240 moves it onto the plan.
- **From 2568 (done):** `game_arg` and `check_start` live in
  `crates/cli/src/game.rs`, and `bots.rs`'s `pub use
  crate::game::game_arg;` goes with the file (add
  `contract::bots::game_arg` to the `Breaking:` line). `check_start` runs
  at the top of `playtest` and `autobalance`; `bots::bots` goes with the
  file.
- **Before the first crates.io release:** 2005 waits for this ticket
  (Nick, 2026-11-05: planned breaks land before anyone installs), so the
  `Breaking:` line is enough; no version bump or `CHANGELOG.md` line.
- **The grid template plays its bot through `playtest`**
  (`templates/shuyi-default/stages/top-down-grid/crates/game/src/stages/top_down_grid/tools.rs`):
  one entrant, its stage's plan, no checks. Its test
  `the_bots_play_to_an_end_and_their_records_replay`
  (`stages/top-down-grid/crates/game/tests/stage.rs`) calls `playtest` and still replays
  each record to the outcome the bots saw. Its `AGENTS.md` table row
  and `.cargo/config.toml` comment say `cargo shuyi playtest --tries 20`.
  The blank template's test that `bots` is unsupported tests `playtest`.
  **Either order with 2133:** if 2133 is done, the entrant's bot keeps
  `RandomBot::new(bot_seed(seed))`, as `bot_play` has it; if not,
  nothing to do (2133 seeds it with `bot_seed`).
- **Hello, Grid** drops its `bots` and `bot_play`, and its
  `StageMeasure` is the one count of the foe's trips.
- **The problem codes' pages** that name `bots` or `GameTools::bots`
  (`crates/cli/errors/SH3001.md` at least: grep `crates/cli/errors`).
- **Open tickets that name the `bots` operation** (not the
  `shuyi_core::bots` module) say `playtest` instead: at least
  2112, 2113, 2356, 2501, 2518, 2524 and 2527 (grep
  `tickets/open` for `` `bots` `` and `GameTools::bots`, and read each
  hit). So does any skill under `skills/` that names it, which
  `check-skills` reports.
- **From 1134 (done):** the shipped skills (`skills/`) that name the
  `bots` operation say `playtest` instead, and a worked example
  (`skills/*/example.ron`) that runs `bots` runs `playtest`, so
  `check-skills` passes and CI's `skills` job runs them green. Grep
  `skills/` for `bots`.
- **Either order with 2524:** if 2524 is done, its eval asks `bots`
  in files outside `tickets/open`: move its `Replay` check
  (`crates/xtask/src/eval/grade.rs` and its test of the answer paths)
  onto `playtest --tries 1 --json` and its first try's `record`, the
  references' bots onto a plan as the grid template's bot, and the
  prompts in `evals/tasks/*.ron` and `evals/README.md` onto
  `cargo shuyi playtest`. If not, rewrite 2524's text with the
  others above.
- The contract's version, its schemas and `versions.json`, as
  `docs/contract.md`'s *The schemas* says.
- **Either order with 2527:** if 2527 is done, `soak` follows
  `playtest` in this PR: the `games` of `SoakInput` and `SoakOutput`
  becomes `tries`, its defaults and limits are `PlaytestInput`'s, its
  command line takes `--tries N` with `parse_args` and `usage` off
  `bots::parse_args` and `bots::usage`, and its schemas, its rows in
  `docs/contract.md` and the `cargo shuyi soak` lines in CI and the
  READMEs change with it; if not, nothing to do (2527 reads `bots` as
  `playtest` when it lands).

**Out (do not do):**
- `shuyi_core::play` and `shuyi_core::bots::playtest`'s judging: only the
  CLI's answer grows;
- the game's use of `playtest` (game#1180, game#1207) and its move off
  `GameTools::bots` (game#1240).

**`shuyi autobalance` (2541, done)** plays its tries through the same
helpers (`contract::playtest`'s `choose`, `checks_of` and `played`) and
an `AutobalancePlan` whose entrants and checks mirror the
`PlaytestPlan`'s: move it onto the plan as this ticket leaves it, with
its answer and its tests unchanged.

## Implementation steps

1. Read 2532's and 2533's completion notes and their code
   (`crates/cli/src/contract/playtest.rs`, `game.rs`), then
   `contract/bots.rs`, the grid template's `tools.rs` and tests, and
   Hello, Grid's `tools.rs`.
2. Add `commands` and `record` to a try in the playtest answer and the
   plan's play of one try. Add the outcome counts and the spreads, next
   to the plan's `summary` items (2533).
3. Move the grid template and Hello, Grid onto it, test first: the grid
   template's records still replay.
4. Remove `bots` (the op, the hook, the subcommand, the schemas, the MCP
   tool's tests), then regenerate the schemas and raise the version.
5. The docs (`docs/contract.md`, `README.md`, the templates'
   `AGENTS.md`, the errors' pages), then the `document` skill.
6. Rewrite the open tickets that name the `bots` operation, and the
   skills.

## Acceptance criteria

- [x] `shuyi bots` is gone: the binary's usage has no `bots`, and
      `crates/cli/schemas/` has no `bots.*.json`.
- [x] `cargo shuyi playtest --tries 20 --json` in a game made from the
      grid template answers 20 tries, each with `commands` and a
      `record` that `replay` plays to its outcome.
- [x] Hello, Grid counts the foe's trips in one place.
- [x] `grep -rn '`bots`' tickets/open` finds no ticket that means the
      operation.
- [x] All gates in the `run-gates` skill pass, the `templates` job too.

## Tests required

- Unit: a plan with no checks judges nothing and passes; a try's record
  and commands in the answer (`contract/playtest.rs`' tests with the
  fake game, `contract/fake.rs`).
- Integration: the grid template's record test through `playtest`; the
  binary's usage (`crates/cli/tests/it/binary.rs`); the MCP tool list
  (`crates/cli/tests/it/mcp.rs`).

## Completion notes

What was done:

- **Core** (`shuyi_core::bots::playtest`): `play_try(stage, entrant,
  seed, turns)` plays one seed as `play_tries` does and gives a
  `PlayedTry { tried, commands, record }`; the record comes from the new
  provided method `Entrant::record(seed, commands)` (default `None`).
  `UNFINISHED` names a turn-limit stop, in the try line and the counts.
  Additions only; `Tried` and the judging are unchanged.
- **`playtest`** (`crates/cli/src/contract/playtest.rs`): a plan's try
  (`PlayTry`, and `autobalance`'s `StagePlay` and `Evaluate`) returns a
  `PlayedTry`. Each entrant's `Played` gains `outcomes` (counted by the
  outcome's name, its `Debug` up to its fields), `turns` and `measures`
  (`Spread`, moved from `bots`), and `tries` (`TryKept`: `seed`,
  `commands`, `record`, left out of the JSON when the plan keeps none).
  A plan with no checks judges nothing and passes when every try
  played; its text adds the counts and spreads after the try lines, as
  `bots` printed them. The history keeps no try's commands or record.
  `MAX_TRIES` and `options_text` moved here.
- **`bots` is gone:** `Op::Bots`, `contract/bots.rs`,
  `GameTools::bots`, its schemas, subcommand, usage line and MCP tool.
  Contract version 38.
- **The grid template** plays its bot through `playtest`: one entrant,
  `random`, no checks, each try recorded (`playtest_plan`, re-exported
  from the stage's module). Its record test calls `playtest` and replays
  the first try to its outcome; its playtest workflow no longer passes
  `allow-no-plan`, since a grid game now has a plan. The blank
  template's test checks `playtest_plan` is unsupported.
- **Hello, Grid** drops `bots` and `bot_play`; its heroes play through
  `play_try` and keep a `StageRecord` each (`Hero::record`), and
  `StageMeasure` is the one count of the foe's trips. A new test replays
  each playtest try to its outcome. Its golden record is the `random`
  hero's try of seed 11, the same play `bots 1 --seed 11` made.
- **The tutorial** (`site/tutorial/*`): steps 09 and 10 give a
  `playtest_plan` (one bot, `line`, each try recorded) in place of
  `bots`; every step's test, `shuyi.ron` and `.cargo/config.toml`
  comment follows; the tutorial page and the book's *Testing* and
  *Tools for agents* chapters say `playtest`.
- The docs (`docs/contract.md`, `README.md`, Hello, Grid's README, the
  templates' `AGENTS.md`, the problem codes' pages SH0001, SH3001,
  SH3002 and SH3007, the reusable playtest workflow's input text), the
  shipped skills `shuyi-balance-bots`, `shuyi-debug-replay`,
  `shuyi-stage-grid` and `shuyi-test` and their worked examples, and 20
  open tickets that named the operation.
- [ADR-0088](../../docs/adr/0088-one-operation-plays-a-games-bots-playtest-keeps-the-commands-and-record-of-each-try-and-the-entrant-gives-the-record.md)
  records the choices (0087 is taken by an open PR).

Checked: `cargo shuyi playtest --tries 20 --json` in a game fresh from
the grid template answered 20 tries (18 caught, 1 won, 1 unfinished),
each with `commands` and a `record` that `cargo shuyi replay` played to
the try's outcome, with the same number of steps.

Deviations:

- The record hook is on the entrant (`Entrant::record`), with the
  core's `play_try` keeping the commands, rather than on the plan: the
  CLI never sees a game's command type, and with it the game drops its
  copy of `play_tries`' loop (game#1240).
- Outcomes are counted by name (`Won` for `Won { turn: 12 }`), so a
  game whose outcome carries its turn doesn't count each turn apart.
- The tutorial's ten steps and the book weren't named by the ticket;
  they used `bots` and had to move with it.
- The either-order clauses: 2557, 2133, 2524 and 2527 are open, so their
  "if not" branches hold; each ticket's text now says what holds (2524
  and 2527 read `playtest`, 2133 names the template's `random`
  entrant, 2557 the no-checks text's try lines).

Follow-ups: none new. The game's move onto a plan is game#1240, which
will move the pin past this change (no game PR exists yet).

Breaking: shuyi-cli: GameTools::bots, contract::bots (BotsInput, BotsOutput, Play, Spread, tally, UNFINISHED, MAX_GAMES, parse_args, usage, text, game_arg, options_text) and the `bots` operation and MCP tool: gone; a game gives its bot run as a `PlaytestPlan` with one entrant and no checks (`GameTools::playtest_plan`), and reads each try's `commands` and `record` from `playtest`'s answer.

```migration
since: 0.1.0
what: The bots operation is folded into playtest; a game's bot run is a playtest plan with one entrant and no checks.
old: "GameTools::bots(&self, input: &BotsInput) -> Result<BotsOutput, String>"
new: "GameTools::playtest_plan(&self, stage: Option<&str>, input: &PlaytestInput) -> Result<PlaytestPlan, String>"
before: |
  fn bots(&self, input: &BotsInput) -> Result<BotsOutput, String> {
      let plays = (input.seed..input.seed + u64::from(input.games))
          .map(|seed| bot_play(&map, seed, input.turns))
          .collect::<Result<_, _>>()?;
      Ok(tally(plays))
  }
after: |
  fn playtest_plan(&self, _: Option<&str>, _: &PlaytestInput) -> Result<PlaytestPlan, String> {
      let map = self.map()?;
      Ok(PlaytestPlan::new(
          "the stage, played by a random bot",
          vec![("random".to_owned(), None)],
          Box::new(|_| Vec::new()),
          Box::new(move |_, seed, turns| bot_try(&map, seed, turns)),
      ))
  }
find: ["fn bots(", "BotsInput", "tally("]
```

Breaking: shuyi-cli: PlayTry, StagePlay, Evaluate: a plan's one try returns `shuyi_core::bots::playtest::PlayedTry` (the try, the commands applied, its record), not `Tried`; most simply `play_try` in place of `play_tries` over one seed.

```migration
since: 0.1.0
what: A plan's one try gives the commands applied and its record besides the try.
old: Box<dyn Fn(usize, u64, u32) -> Result<Tried, String>>
new: Box<dyn Fn(usize, u64, u32) -> Result<PlayedTry, String>>
before: |
  Box::new(move |entrant, seed, turns| {
      let mut tries = play_tries(&stage, &ENTRANTS[entrant], seed..seed + 1, turns)
          .map_err(|e| e.to_string())?;
      tries.pop().ok_or_else(|| "no try".to_owned())
  })
after: |
  Box::new(move |entrant, seed, turns| {
      play_try(&stage, &ENTRANTS[entrant], seed, turns).map_err(|e| e.to_string())
  })
find: ["play_tries(", "seed..seed + 1"]
```

No version bump or `CHANGELOG.md` line: the first crates.io release
(2005) waits for this ticket.

Nick input: none.
