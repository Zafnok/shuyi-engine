---
id: "2126"
title: "A playtest runner: K tries of each named bot over a stage, the game's measures, and pass bands as data"
type: feature
milestone: After MVP
model: reasoning
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-10-21
---

# 2126 — A playtest runner with pass bands

## Context

A request from the game, written on 2026-10-21 beside 2125.

1. **The game tickets:** game#0506, the persona playtest bots
   (`github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/0506-persona-bots-casual-normal-hardcore.md`),
   whose report checks each bot's success rate against a band, and
   game#0509 (autobalance, through 2127). They wait for this ticket; they
   are not its blockers.
2. **The job in one sentence:** let several bots each play a level many
   times, measure every play, and say per bot whether it succeeded
   about as often as the designer wants: ✓, ✗ too hard, or ⚠ too easy.
3. **No friction review:** a missing feature. The game's 0505 built a
   runner of its own (`crates/bots`, `cargo xtask playtest`); its
   generic half (tries, seeds, bands, verdicts, the table) moves here,
   and the game keeps its measures, success rules and band values.
4. **The generic shape:** a runner over any `Sides` game: named
   entrants (each says how it starts the stage and which `Bot` plays
   each side), K tries each from seeds `seed`, `seed + 1`, …, the
   game's measures of every try by name, and checks as data: the share
   of tries meeting a condition within a band (with an optional floor
   that fails the run), and a median of a measure compared between two
   entrants. A report as data and as a table.
5. **Not only tactics:** a puzzle game checking that a "beginner" bot
   solves level 3 in 60–90% of tries and an "expert" bot in fewer moves;
   a card game checking that a deck wins 45–55% against the house AI; a
   platformer's level-gen bot later (1127). Any game with bots and a
   difficulty it wants to hold.

Shuyi today:

- `crates/core/src/bots.rs`: `Sides`, `Bot`, `play`, `play_observed`,
  `Report`, `PlayError` and `RandomBot`.
- `crates/cli/src/contract/bots.rs`: the `bots` operation: the game
  plays N games through `GameTools::bots`, `tally` sums the outcomes and
  measures (`BTreeMap<String, i64>` per play, a `Spread` each). It has
  no notion of success, bands, or more than one bot.
- `examples/hello-grid/src/tools.rs`: `bot_play` (a `RandomBot` hero,
  `Chase` foe, the measure "foe trips").

It accepts any `Bot`: the bots are the game's (game#0506's personas), a
`RandomBot`, or 2125's rollout bot. **It doesn't need 2125**; the two
can land in either order (see Scope).

Placement (ADR-0009 §1 and §7): the runner, the checks and the report
are pure and go in `shuyi-core` beside `bots` (generic, no dependency,
`no_std` with `alloc`, nothing a game pays for unless it names them).
The `shuyi playtest` operation goes in `shuyi-cli` beside `bots`.
Hello-grid shows both.

## Nick input

None.

## Scope

**In:**
- **`shuyi_core::bots::playtest`** (`crates/core/src/bots/playtest.rs`,
  tests in `bots/playtest/tests.rs`):
  - `pub type Measures = BTreeMap<String, i64>;` the game's measures of
    one try, by name. A success or a loss is a measure too (`"won"` 1
    or 0), so every check reads measures only.
  - `pub trait Measure<R: Sides>`: `observe(&mut self, rules: &R, command: &R::Command, events: &[R::Event])`
    for every applied command, then
    `finish(self: Box<Self>, report: &Report<R>, end: &R) -> Measures`.
  - `pub trait Entrant<S, R: Sides>`: one named bot as the runner plays
    it, over the stage definition `S` (the data a stage is built from,
    so 2127 can change it):
    - `fn name(&self) -> &str;`
    - `fn start(&self, stage: &S, seed: u64) -> R;` the stage as this
      entrant plays it (a game with modes picks this entrant's), its
      luck from `seed`;
    - `fn bots(&self, seed: u64) -> Vec<(R::Side, Box<dyn Bot<R>>)>;`
      one bot per side for the try of `seed`;
    - `fn measure(&self) -> Box<dyn Measure<R>>;` a fresh measure for
      a try.
  - `pub struct Tries { pub count: u32, pub seed: u64, pub turns: u32 }`:
    K tries from seeds `seed` to `seed + count - 1`, each stopped at
    turn `turns` if it hasn't ended. Every entrant plays the same seeds.
  - `pub fn run<S, R: Sides>(stage: &S, entrants: &[&dyn Entrant<S, R>], tries: &Tries, checks: &[Check]) -> Result<PlaytestReport, PlayError<R>>`,
    where `R::Command: Clone` and `R::Outcome: Debug`. Each try is
    `play_observed` with the entrant's bots and its measure. A refused
    command stops the run with the `PlayError` (a bug in a bot, never
    skipped).
  - `run` is two public halves, so a caller with threads (a game's
    tool, on `std`) can play seed ranges on threads of its own and
    judge them once: `play_tries(stage, entrant, seeds: Range<u64>, turns) -> Result<Vec<Tried>, PlayError<R>>`
    and `judge(tries: Vec<(String, Vec<Tried>)>, checks) -> Result<PlaytestReport, PlaytestError>`
    (the entrant's name and its tries in seed order; `judge` sorts
    them by seed, so the report never depends on how the seeds were
    split).
  - **The checks**, `Serialize` and `Deserialize`, so a game keeps them
    in a data file:
    - `Condition { measure: String, min: Option<i64>, max: Option<i64> }`,
      both ends inclusive; a try meets a list of conditions when it
      meets all of them; a missing measure fails its condition;
    - `Check::Rate { entrant: String, success: Vec<Condition>, band: Band, floor: Option<u32> }`,
      `Band { min: u32, max: u32 }` in percent of tries, both ends
      inclusive, compared in integers (`successes * 100` against
      `min * count` and `max * count`). Verdict `Within` (✓), `Below`
      (✗ too hard) or `Above` (⚠ too easy). `floor`: a run in which the
      share is under `floor` percent is **blocked**;
    - `Check::Median { entrant: String, measure: String, among: Vec<Condition>, want: Want, than: String }`,
      `Want` is `Lower` or `Higher`: the median of `measure` over the
      entrant's tries that meet `among`, against the same median of
      entrant `than`. Verdict `Pass` (✓), `Fail` (✗) or `Unknown` (–,
      when either side has no such try). The median of an even count
      is the lower middle value (integers; said in the docs);
    - a check naming an entrant that isn't in the run is an error
      before any try is played (`run` returns it as a
      `PlaytestError::UnknownEntrant`; make the error type an enum that
      also wraps `PlayError`).
  - `PlaytestReport`, plain data with `Serialize`: per entrant its
    name and every try (`seed`, `outcome` as `Option<String>` from the
    outcome's `Debug`, `turns`, `measures`); per check its numbers and
    verdict; `blocked: bool`. `impl Display`: the table below, the
    layout the game's report already uses
    (`visions-of-shuyi/docs/design/playtest-bots.md`, *What the report
    shows*), without the game's own lines:

    ```
    100 tries (seeds 1–100)

    casual    · 64/100 succeed    band 60–85%   ✓
    normal    · 47/100 succeed    band 50–75%   ✗ too hard
    hardcore  · 52/100 succeed    band 50–75%   ✓
    hardcore  · turns median 10 (normal 14)     lower ✓

    try 17 · normal · Some(Lost) · turns 9 · items 2 · deaths 2
    …
    ```

    A game adds its own lines (histograms, who fell most, wall time)
    from the report's data.
- **`shuyi playtest`** in `shuyi-cli`, beside `bots`
  (`crates/cli/src/contract/playtest.rs`):
  `GameTools::playtest(&self, input: &PlaytestInput) -> Result<PlaytestReport, String>`,
  unsupported by default like `bots`; `PlaytestInput { tries: u32, seed: u64, turns: u32, start: Vec<GameArg> }`
  with `bots`' limits (`MAX_GAMES`, the seed overflow check); the
  answer's `ok` is `!blocked`, so the exit code is 1 only when a floor
  is missed. Its JSON schema, the row in `docs/contract.md`, and the
  MCP tool if 1133 is done (either order: if not, 1133's list gains
  `playtest`).
- **Hello-grid shows it:**
  - measures `won` (1 or 0), `turns` and `foe trips`, as a `Measure`
    in `src/tools.rs` (the existing `bot_play` keeps its output);
  - two entrants: `random` (a `RandomBot` hero) and `walk` (a hero that
    moves to the tile in reach closest to the goal). **Either order
    with 2125:** if 2125 is done, `walk` uses its `Walk`; if not, add
    `Walk` and `distances_to_goal` to `src/rules.rs` as 2125 describes
    them, and 2125 then uses these;
  - `files/playtest.ron`: a rate check per entrant with bands that hold
    for seeds 0–49 (measured once and written down; say the numbers in
    the Completion notes), a `floor` of 1 on `random`, and `walk`'s
    median turns among won tries lower than `random`'s;
  - `GameTools::playtest` in `src/tools.rs`, and a line in
    `README.md`'s tools block;
  - `tests/it/playtest.rs`: the report's verdicts on seeds 0–49, and
    the same report twice.
- **What changes for the game:** nothing breaks. game#0506's PR moves
  the pin and puts its runner on this one (its `cargo xtask playtest`
  becomes a thin wrapper).

**Out (do not do):**
- the rollout bot (2125) and autobalance (2127);
- wall time, or any clock (the game times its tries);
- a history of runs over time (the game keeps its own);
- running tries on several threads (the caller may, through
  `play_tries` and `judge`);
- moving `bots`/`tally` onto this runner (they stay; a follow-up may
  merge them if both are kept);
- any game's success rules or band values.

## Implementation steps

1. Read `bots.rs`, `contract/bots.rs`, `game.rs` (`GameTools`), and
   `docs/contract.md`.
2. Turn `bots.rs` into a module if 2125 hasn't, and write
   `bots/playtest.rs`: the traits, `Tries`, `Condition`, `Band`,
   `Want`, `Check`, `run`, the verdicts and `PlaytestReport` with
   `Display`.
3. Unit tests on hand-made tries (no game needed): each verdict, the
   band's edges, the floor, the even-count median, a missing measure,
   an unknown entrant.
4. `shuyi playtest`: the input, the operation, `GameTools::playtest`,
   the schema and `docs/contract.md`.
5. Hello-grid's measure, entrants, `files/playtest.ron`, tools and
   tests.
6. Run the gates.

Size: about 500 changed lines (the runner and checks about 250, the
operation about 100, hello-grid about 150). If it runs well past 600,
move `shuyi playtest` and hello-grid's `GameTools::playtest` to a
follow-up ticket and keep the library and its hello-grid test here.

## Acceptance criteria

- [x] Unit test: a rate of 59, 60, 85 and 86 successes in 100 against
      band 60–85 gives `Below`, `Within`, `Within`, `Above`.
- [x] Unit test: a floor of 20 blocks a run with 19 successes in 100
      and not one with 20.
- [x] Unit test: the median of `[3, 1, 4, 2]` is 2; `Unknown` when an
      entrant has no try meeting `among`.
- [x] Unit test: a check naming a missing entrant fails before any try.
- [x] `judge` over seeds 0–49 played as one range, and as the ranges
      0–16, 17–33 and 34–49 given in reverse order, gives the same
      report as `run`.
- [x] The same stage, entrants, tries and checks give the same report.
- [x] `cargo shuyi playtest --tries 50` in `examples/hello-grid` prints
      the table with ✓ for every check, and exits 0.
- [x] `docs/contract.md` lists `playtest`, and its schema is in the
      schema set.
- [x] `shuyi-core` still builds for `thumbv7em-none-eabihf` (CI's
      `no-std` job).
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the checks and verdicts above, the report's `Display` on a
  small hand-made run (compared with the expected text; `shuyi-core`
  has no `insta`).
- Property: any band and any successes give exactly one verdict, and
  `Within` iff the share is inside the band.
- Integration: hello-grid's `tests/it/playtest.rs`; the contract test
  for `playtest` beside `bots`'.

## Completion notes

What was done:

- `shuyi_core::bots::playtest` (`crates/core/src/bots/playtest.rs`):
  `Measures`, `Measure`, `Entrant`, `Tries`, `Tried`, `Condition`,
  `Band`, `Want`, `Check` (`Rate`, `Median`), `run`, and its halves
  `play_tries` and `judge`, the verdicts (`RateVerdict`,
  `MedianVerdict`), and `PlaytestReport` with `Display` (the table, then
  every try). 12 tests, among them a property test of the band.
- `shuyi playtest [--tries N] [--seed N] [--turns N]` in `shuyi-cli`
  (`contract/playtest.rs`), `GameTools::playtest` (unsupported by
  default), `bots`' limits, `ok` is `!blocked`. Contract version 6 and
  its schemas; `docs/contract.md` lists it. 1133 isn't done, so its
  ticket now names `playtest` among the tools.
- Hello-grid: `StageMeasure` (`won`, `turns`, `foe trips`), the entrants
  `random` (a `RandomBot` hero) and `walk` (2125's `Walk`, already
  there), `playtest.ron`, `GameTools::playtest`, `tests/it/playtest.rs`.

The numbers on seeds 0–49 (turn limit 200): `random` won 5 of 50, `walk`
50 of 50; the median turns of won tries were 4 for `walk` and 28 for
`random`. So the bands are `random` 5–20% with a floor of 1%, and `walk`
90–100%. On seeds 0–99 (the default 100 tries) `random` wins 14 and the
bands still hold.

Deviations:

- The checks are in `examples/hello-grid/playtest.ron`, beside
  `shuyi.ron`, not in `files/`: `bake` packs every file of `files/`, so
  the tools' data would have shipped inside the game.
- `PlaytestError::Play` carries the entrant's name beside the
  `PlayError`, and `judge` returns `UnknownEntrant` (a struct that
  `PlaytestError` wraps), so `judge` needs no game type.
- The report's JSON schema needs `JsonSchema` on the core's types: they
  derive it behind a new optional `schema` feature of `shuyi-core`
  (`schemars`, already a dependency of `shuyi-cli`), which only
  `shuyi-cli` turns on. A `no_std` build doesn't see it.
- A try's line in the table writes its outcome as the game's `Debug`
  (`Won { turn: 36 }`), or `unfinished`, rather than `Some(…)`.
- Hello-grid's `playtest` refuses the game's start options (`--stage`,
  `--tiles`): they choose how the window starts, which a playtest
  doesn't use.
- `bots.rs` lends `playtest` two helpers (`options_text`, `game_arg`);
  `bots`' behaviour is unchanged.

For the game: nothing breaks; `GameTools` gains a method with a default.
The game PR that moves the pin is game#0506's (not opened yet), which puts
its runner on this one.

Nick input: none.
