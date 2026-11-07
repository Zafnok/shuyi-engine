# ADR-0088: One operation plays a game's bots: `playtest` keeps each try's commands and record, and the entrant gives the record

- **Status:** Accepted
- **Date:** 2026-11-06
- **Related tickets:** 2544 (2126, 2532, 2533; game#1240)

## Context

The tool contract had two operations that play a game's stage with its
bots ([`docs/contract.md`](../contract.md)):

- `bots <games>` (ticket 1129): `GameTools::bots` played the games and
  `contract::bots::tally` summed them: each play's outcome, turns,
  commands, measures and record for `replay`, the outcomes counted and
  the spread of each number.
- `playtest` (2126, rebuilt by 2532 and 2533): the game gives a
  `PlaytestPlan`, its named bots play K tries each on every core, and
  the game's checks judge them.

Their inputs said the same thing in different words, one play was
`bots::Play` in one and `Tried` in the other, Hello, Grid counted its
foe's trips twice, and a third operation (2527) planned to copy `bots`'
input again. The game (Visions of Shuyi) implements `GameTools::bots`
by copying the core's `play_tries` loop only to keep each play's
commands for its record, since `Tried` drops them (game#1240).

The judging in the core (`shuyi_core::bots::playtest::judge`) and its
`Tried`, which every report and history holds, were out of the
ticket's scope.

## Decision

1. **`bots` goes.** `Op::Bots`, `contract::bots`, `GameTools::bots`,
   the `bots` subcommand, its MCP tool and its schemas are removed. A
   game's bot run is a `PlaytestPlan` with one entrant and no checks.
2. **A plan with no checks is a plain bot run.** `playtest` judges
   nothing then, and passes when every try played. Its text adds each
   entrant's outcomes counted and the spread of its turns and measures
   after the line per try, as `bots` printed them.
3. **A try keeps its commands and its record.** A plan's one try
   (`PlayTry`, and `autobalance`'s `StagePlay`) returns the core's
   `PlayedTry { tried, commands, record }`. The answer's `Played` (one
   per entrant) gains `outcomes` (by the outcome's name: its `Debug` up
   to its fields, `unfinished` for a turn-limit stop), `turns` and
   `measures` (`playtest::Spread`: `min`, `max`, `total`, `mean`), and
   `tries` (`TryKept`: `seed`, `commands`, `record`, left out when the
   plan keeps none). `Tried` and the report don't change; the history
   keeps no try's commands or record.
4. **The entrant gives the record.** The core gains
   `playtest::play_try(stage, entrant, seed, turns)`, which plays one
   seed as `play_tries` does and keeps how many commands were applied
   and `Entrant::record(seed, commands)`: the game's record of the try
   from its seed and commands, a provided method that keeps none. A
   game that plays its tries itself builds the `PlayedTry` with
   `PlayedTry::new` and sets its `record`.
5. `shuyi_core::bots::playtest::UNFINISHED` names a try the turn limit
   stopped, in the report's try line and in the counts.

## Consequences

- One input and one answer for every bot run; 2527's `soak` takes
  `playtest`'s fields and limits.
- The game moves its bot run onto a plan whose entrant writes the
  record (game#1240) and drops its copy of the loop.
- A playtest's JSON answer grows with a record per try; the history,
  which the next run only compares, keeps none of them.
- Breaking for `shuyi-cli` before the first crates.io release:
  `GameTools::bots`, `contract::bots` and the `bots` operation go, and
  a plan's try returns `PlayedTry`. The core's additions break nothing.
- Counting outcomes by name reads Hello, Grid's `Won { turn: 12 }` as
  `Won`; a game whose outcomes differ only in their fields sees them
  counted together.

## Alternatives considered

- **Add `commands` and `record` to the core's `Tried`.** Every report,
  history line and check would carry the records, and the judging the
  ticket left alone would change with them.
- **A record hook on the plan, from the seed and the commands.** The
  CLI doesn't know a game's command type, so the hook would take them
  as text or the plan would need a type parameter; the entrant already
  knows them.
- **Keep `bots` beside `playtest`.** The two copies were what the
  ticket was written to end.
- **Count outcomes by their whole `Debug`.** A game whose outcome
  carries its turn would count each turn apart.
