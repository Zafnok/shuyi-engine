---
name: shuyi-balance-bots
description: Bots that play a turn-based Shuyi Engine game and report on it, from a plain bot run (`shuyi bots`) to named bots judged by bands in a data file (`shuyi playtest`) and the autobalance search whose knobs change a stage until each bot lands in its band (`shuyi autobalance`). Use when a game needs to know how hard a stage is, whether it can be won, whether a change made it easier, or which setting of a stage puts the bots where the designer wants. Not for writing a game's rules (shuyi-stage-grid), for tests of exact outcomes (shuyi-test), or for a record that replays differently (shuyi-debug-replay).
license: MIT OR Apache-2.0
metadata:
  version: "1.0"
---

# Bots that play a stage and report

A bot is a player made of code. It plays the game's rules (`TurnRules`)
to the end, so a stage is measured, not guessed: how often a careful
player wins, how often a random one does, how many turns it takes. All
of it is in whole numbers and from seeds, so the same input gives the
same report on every machine and every thread count.

## When to use it

- A stage should be winnable, or about this hard: say so as data and
  check it on every change.
- A change to the rules or a map moved the difficulty, and you need to
  see how far.
- The designer wants a stage tuned until its bots land in their bands.

## When not to

- Writing the rules themselves: `shuyi-stage-grid`.
- A test that one script gives one exact state: `shuyi-test`.
- A record that plays differently twice: `shuyi-debug-replay`.

## Steps

1. **A plain run.** `cargo shuyi bots 20` plays 20 games with the
   game's bot, from seeds 0 to 19: how each ended, its turns, the game's
   measures, and a record of each (`/result/plays/<n>/record`) that
   `cargo shuyi replay` replays. A game made with `shuyi new --template
   grid` gives one, the engine's `RandomBot` (its `GameTools::bots`:
   each game by `shuyi_core::play`, summed up by `tally` from
   `shuyi_cli::contract::bots`).

2. **Name the bots.** A playtest's entrants are bots with names. Each is
   an `Entrant<S, R>` (`shuyi_core::bots::playtest`): its `name`, the
   stage as it plays it from a seed (`start`), one bot per side
   (`bots`), and a `Measure` of a try (`observe` each command, `finish`
   with the try's named measures, such as `won` 1 or 0 and `turns`).

3. **Give a bot that plans a copy to plan on.** The rules implement
   `Determinize`: a copy of the state as one side knows it, the luck to
   come redrawn from a seed (`SimRng::new(seed)` in place of the
   state's), anything the other sides hide dealt again. Never clone the
   state for a bot: it would see the rolls to come. Then:
   - `Rollout` tries each of its first commands (`Order`, or
     `Unordered`) on copies, plays each on with a `Playout` (any `Bot`),
     and keeps the one whose copies ended best (`Evaluate`, a score in
     whole points). `RolloutSettings::new(candidates, samples, horizon,
     commands, temperature)` says how hard it thinks, in work units,
     never time.
   - `TurnPlan` (with `PlanSettings`) plans a side's whole turn of
     several commands the same way.
   - Either one's `Order` decides what it tries at all: see *Candidate
     orders* below.

4. **Write the checks as data**, a file of `Check`s the tools read
   (keep it outside `files/`: it is the tools' data, not the game's):

   ```ron
   [
       Rate(entrant: "rollout", success: [(measure: "won", min: Some(1))],
            band: (min: 80, max: 100), floor: Some(50)),
       Median(entrant: "rollout", measure: "turns", among: [(measure: "won", min: Some(1))],
              want: Lower, than: "random"),
   ]
   ```

   A `Rate` is the share of an entrant's tries that meet every
   `success` condition: inside its `band` (✓), under it (✗ too hard) or
   over it (⚠ too easy). Under its `floor` the run is blocked, and that
   alone fails `playtest`. A `Median` compares a measure between two
   entrants.

5. **Give the plan.** The stage's `GameTools::playtest_plan` returns a
   `PlaytestPlan::new(title, entrants, checks, play)`: the entrants by
   name (and a label each, if any), a function from the names that play
   to the checks that name only them, and one try of one entrant from
   one seed, most simply `play_tries(&stage, &entrant, seed..seed + 1,
   turns)`. `groups` names sets of entrants, and `default_entrants` the
   ones that play when none is asked for. A game with several stages
   lists their ids in `playtest_stages`, and changes the defaults (100
   tries from seed 0, to turn 200) in `playtest_defaults`. Its `summary`
   gives each entrant (in `entrants` order) the numbers a designer reads
   first, over its measures, as `playtest::Summary` items that may live
   in a data file: `Sum(measure: "won")` prints `won 83/100`,
   `Spread(measure: "deaths", up_to: 3)` `deaths per try 0: 30  1: 21
   2: 25  3+: 24`, `Most(prefix: "fell ", skip_suffix: Some(" turn"))`
   `most: scout 41×`, and `Median` and `Max` `items median 1`. Each takes
   a `label`; a missing measure counts as 0.

6. **Run it:**

   ```bash
   cargo shuyi playtest
   cargo shuyi playtest --tries 20 --seed 0 --turns 200
   cargo shuyi playtest --entrant rollout --report playtest.json
   cargo shuyi playtest --every-stage
   ```

   `--stage <id>` picks a stage (needed when the game has several),
   `--entrant` picks entrants or a group, and `--threads` the cores to
   play on. The tries run on every core, and the report never depends on
   how many. The answer's `result` holds one report per stage: each
   entrant's tries with their measures, each check with what it found
   (`found.Rate.verdict` is `Within`, `Below` or `Above`), `blocked`,
   and each entrant's seconds per try, the one number that changes with
   the machine, and its `summary` lines.

   Every run is kept in a history, one JSON line per run in
   `target/playtest-history/<stage>-<entrants>[-<options>].jsonl`
   (`--history <dir>` keeps it elsewhere, `--no-history` keeps none).
   The next run of the same stage, entrants and options says what each
   entrant was then, after its line (`· was 47/100 succeed`, from its
   first rate check, else its first summary item), and ends with
   `previous <date> · commit <hash>`; in JSON, each entrant's `was` and
   the report's `previous`. Change a stage, run again, and read the
   `was`. `--brief` leaves out the line per try.

   In CI, the engine's reusable workflow plays every stage when a lever
   changed (`uses: Zafnok/shuyi-engine/.github/workflows/playtest.yml@<commit>`;
   a `grid` game has it in `.github/workflows/playtest.yml`). Its inputs:
   `paths` (patterns that play, `!pattern` that don't), or `levers`, a
   script of the game's that reads the changed paths and prints
   `play=true` or `play=false`; `working-directory`, `system-packages`,
   `build`, `command`, `allow-no-plan` and `timeout-minutes`. It fails
   only on a floor or a broken playtest. A new `grid` game passes
   `allow-no-plan: true`, since it has no plan: drop it once the plan is
   written. Give the caller the `concurrency` that never
   cancels a push (`group: playtest-${{ github.event_name == 'push' &&
   github.sha || github.ref }}`, `cancel-in-progress: ${{
   github.event_name != 'push' }}`), or a push's lever goes unplayed.
   The engine's `docs/contract.md`, *The playtest in CI*, has the rest.

7. **Tune the stage, never the bots.** `cargo shuyi autobalance` changes
   the stage with the game's knobs until every rate is in its band, then
   says the smallest change it found in the knobs' own words, why it
   stopped, and the reports before and after:

   ```bash
   cargo shuyi autobalance --budget 100
   cargo shuyi autobalance --stage <id> --entrant rollout --time 60 --threads 4
   cargo shuyi autobalance --apply
   ```

   `--budget` caps the evaluations (100), `--time` stops after that many
   seconds with the best setting seen (what it finds then depends on the
   machine, and a limit says so), and `--apply` has the game write the
   changed stage to its files. The search plays the same seeds for
   every setting, so settings compare on the same luck. A game gives it
   an `AutobalancePlan` (its `GameTools::autobalance_plan`): the
   entrants and checks as its playtest has them, and a
   `Tunable::new(stage, knobs, play, write)` over the game's own stage
   type `S`. Each knob is a `Knob<S>`: a `name`, a `range` of integer
   steps holding 0 (the stage as authored), a `stride`, `apply` (the
   stage moved that many steps, or `None` where it can't) and
   `describe`. The game writes no command, thread or time limit of its
   own. A new `grid` game has no plan: [references/autobalance.md](references/autobalance.md)
   writes one, with a knob of the wanderer's start.

## Targets by tier, and personas as data

- **Targets by tier.** A game with tiers of stages (easy, hard) writes
  what each entrant should achieve once, as
  `shuyi_core::bots::playtest::Targets`: `success` per entrant, `bands`
  per tier and entrant, `floors`, and `medians`.
  `Targets::checks(&tier, &entrants)` gives a stage's checks, and
  `unknown_entrants` names a file's typos.
- **Personas.** A kind of player is data (`shuyi_core::bots::weigh`): a
  `Persona` holds its `PlanSettings`, its weights by the game's feature
  names, and the game's own extras. `Weighted { weights, counts }` turns
  the game's counts of its features into an `Evaluate` for the bots that
  plan. A weight that names no feature of the game is an
  `UnknownFeature` error.

## Threat areas: bots that read the danger zone

A bot that plays like a person sees what a player sees: which tiles each
foe can strike next turn, and the forecasts. All in `shuyi_core::threat`:

- **Areas.** `strike_area(&stops, &ranges)` is the tiles a piece strikes
  from the tiles it can stop on (most often its `reachable(..).tiles()`
  less the tiles others hold), with `ranges` its inclusive Manhattan
  `(min, max)` pairs; it threatens its stops too. A piece that never
  moves uses `strike_area_from(w, h, at, &ranges)`.
- **Threats.** `Threats::new(vec![Threat::new(id, area), ..])`:
  `safe(tile)` is whether no foe reaches it, `reaches(tile)` names the
  foes that do, `zone()` is the whole danger zone.
- **Danger.** `threats.danger(tile, |id| forecast)` adds up the game's
  `Strike::new(expected, all_hit, struck_back)` for every foe that
  reaches the tile (expected values in hundredths: hit chance in percent
  times damage; `None` for a foe that wouldn't strike). Every threat
  counts: never guess which foe the other side picks. Caps (at most the
  target's health) are the game's, on the `Danger` it gets back.
- **Goals.** `GoalDistance::new(w, h, goals)` and `distance(kind, from,
  cost)` give the walk from a tile to the nearest goal for each movement
  kind, built once per kind (so `cost` must not change for a kind). A
  goal's tile is free to enter, as a foe to reach stands on it.

Feed these to a persona's feature counts (`Weighted`): tiles left in
danger, expected damage taken, steps to the goal.

## Candidate orders: what a planner tries first

`TurnPlan` and `Rollout` try only the first `candidates` commands of
their `Order`. The legal order cut at ten holds one piece's attack in
every flavour and nothing else, and the bot stalls. Give it a ranked
order instead (ADR-0044):

- **Any game:** `ranked_order(items, last, cap)` from
  `Ranked::new(command, group, value, key)`: groups ascending, the best
  `value` first, one per `key` first (all firsts, then all seconds), and
  `last` (the end of the turn) always within `cap`. A deck-builder's:
  damage plays in group 0 keyed by target, draws in group 1, the end
  last.
- **Grid games:** `TacticsOrder::new(hooks, cap)` with the game's
  `OrderHooks`: `prepare` builds the state's `Threats` and
  `GoalDistance`, `classify` tags each command (`Kind::Strike`,
  `Extra`, `Step`, `End`, `Drop`, `Other`), `pick` is the game's AI's
  own command, and `at`, `distance`, `safe` answer from them. Its eight
  steps: each piece's best strike on each target, the pick, the extras,
  each piece's advance (its step nearest the goals by walking, if
  nearer than where it stands), each piece's hold (its safe step
  nearest the goals, if not its advance), the other ways of striking,
  the rest, and the end.

Give the order the planner's own `candidates` as its `cap`, or the end
can fall past the planner's cut. What a strike or a heal is and what it
is worth stay the game's, in `classify`.

## When it fails

Branch on the answer's `code`, never its message:

- `SH3001`: the game's tools don't offer it: give the stage's
  `GameTools` a `bots`, `playtest_plan` or `autobalance_plan`.
- `SH3002`: the game's tools failed: a checks file that doesn't read, a
  check naming an entrant the game doesn't have, a stage that doesn't
  build. The answer's `error` says which.
- `SH0001`: a wrong option, or a `--stage` or `--entrant` the game
  doesn't have: the answer's error says which.
- `SH3004`: `playtest`'s history can't be written: the error names the
  file. The reports are in the answer still.
- `SH3005`: the game's tool program didn't build.

A `playtest` that ran and is `ok: false` has a rate under its floor
(`blocked`): the stage can't be won as the checks want. `autobalance`
is `ok: false` while a rate is outside its band.

## Worked example

[`example.ron`](example.ron) runs on a new `grid` game: `bots 5` plays
it; `playtest` answers `SH3001`; then the rules get a `Determinize`, the
stage's tools a plan with a random bot and a rollout bot, and a
`playtest.ron` their bands. 20 tries each put both rates in their bands,
with the rollout bot faster, then one entrant plays alone into a report
file. A band moved out of reach, with a floor, blocks the run. `lint`,
`fmt` and clippy pass. It leaves `autobalance` out: the reference
shows it.
