---
id: "2544"
title: "One operation plays a game's bots: `bots` folded into `playtest`, which answers each try's commands and record for `replay`"
type: feature
milestone: After MVP
model: opus-5.5
effort: high
status: todo
blocked_by: ["2532", "2533"]
nick_input: none
completed:
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
leave. Visions of Shuyi implements neither `GameTools::bots` nor
`GameTools::playtest` (its `crates/xtask/src/game_tools.rs`), so the game
is not affected.

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
  entrant it is what `bots` was. The text output for that case is one
  line per try and the counts, as `bots`' is.
- **`bots` goes:** `Op::Bots`, `contract/bots.rs` (`Spread` moves to
  `playtest` if it is still needed), `GameTools::bots`,
  `crates/cli/schemas/bots.*.json`, the `bots` subcommand and its usage
  text, its MCP tool (from `Op::ALL`), and its rows in `docs/contract.md`
  and the root `README.md`. A breaking change: the completion notes
  carry a `Breaking:` line for `shuyi-cli` (`GameTools::bots` and the
  `bots` operation go; a game gives its bot as a playtest plan with
  one entrant and no checks). The game implements neither hook.
- **Either order with 2005** (the first crates.io release): if 2005 is
  done, the break also takes what its version policy asks of a break
  after the release (a version bump and a `CHANGELOG.md` line,
  `work-ticket` §3); if not, the `Breaking:` line is enough.
- **The grid template plays its bot through `playtest`**
  (`templates/grid/crates/game/src/tools.rs`): one entrant, its stage's
  plan, no checks. Its test
  `the_bots_play_to_an_end_and_their_records_replay`
  (`crates/game/tests/it/main.rs`) calls `playtest` and still replays
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
- the game's use of `playtest` (game#1180).

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

- [ ] `shuyi bots` is gone: the binary's usage has no `bots`, and
      `crates/cli/schemas/` has no `bots.*.json`.
- [ ] `cargo shuyi playtest --tries 20 --json` in a game made from the
      grid template answers 20 tries, each with `commands` and a
      `record` that `replay` plays to its outcome.
- [ ] Hello, Grid counts the foe's trips in one place.
- [ ] `grep -rn '`bots`' tickets/open` finds no ticket that means the
      operation.
- [ ] All gates in the `run-gates` skill pass, the `templates` job too.

## Tests required

- Unit: a plan with no checks judges nothing and passes; a try's record
  and commands in the answer (`contract/playtest.rs`' tests with the
  fake game, `contract/fake.rs`).
- Integration: the grid template's record test through `playtest`; the
  binary's usage (`crates/cli/tests/it/binary.rs`); the MCP tool list
  (`crates/cli/tests/it/mcp.rs`).

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
