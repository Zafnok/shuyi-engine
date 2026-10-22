---
id: "2127"
title: "An autobalance search: the game's knobs change a stage, never the bots, until every bot lands in its band"
type: feature
milestone: After MVP
model: opus-5.5
effort: high
status: done
blocked_by: ["2126"]
nick_input: none
completed: 2026-10-22
---

# 2127 — An autobalance search over knobs

## Context

A request from the game, written on 2026-10-21 beside 2125 and 2126.

1. **The game ticket:** game#0509, autobalance a battle with the
   persona bots
   (`github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/0509-autobalance-battles-with-bots.md`).
   Every Act 1 battle of the game is balanced with it (game#0848–0854,
   game#1071, game#1072, and game#0512's check of the whole act). It
   waits for this ticket; it is not this ticket's blocker.
2. **The job in one sentence:** change a level a little at a time (one
   more enemy, a weaker trap, a later ambush) and replay it with the
   bots until each bot succeeds about as often as the designer wants,
   and say what was changed.
3. **No friction review:** a missing feature. The game's designer (Nick,
   in the game's 0033) wants battles balanced by the tool, not by hand:
   "the point of automated balancing is I don't want to manually do
   this stuff". The search is generic; the game keeps its knobs.
4. **The generic shape:** a set of knobs the game defines, each a
   bounded integer step that changes a stage definition, and a
   deterministic search over the knobs' steps that scores each setting
   by 2126's playtest runner (the distance of every rate check to its
   band) until all are inside or a budget of evaluations runs out. It
   reports the smallest change set it found, in the knobs' own words,
   with the runner's report before and after.
5. **Not only tactics:** a puzzle game's move limit and number of
   pieces; a roguelike floor's monster count and potion drops; a
   platformer's enemy speed and gap widths, judged by its bots (1127,
   parked). Any game with bots, bands (2126) and levels made of data.

Shuyi today:

- 2126 (blocks this): `shuyi_core::bots::playtest`: `Entrant<S, R>`
  (an entrant starts the stage definition `S`), `Tries`, `Check`
  (`Rate` with a `Band`, `Median`), `run` and `PlaytestReport`.
- `examples/hello-grid/src/rules.rs`: `Stage::new(map, seed)` and the
  constant `TRIP_PERCENT` (25), the chance the foe trips.

**The bots never change.** Only the stage does. A search that tuned
the bots until they passed would prove nothing (the game's design rule,
`visions-of-shuyi/docs/design/playtest-bots.md`), so the search takes
the entrants as they are and has no way to reach their settings.

Placement (ADR-0009 §1 and §7): `shuyi-core`, beside 2126's runner. It
is generic, adds no dependency, is `no_std`, and a game that never
names it pays nothing. Hello-grid shows it.

## Nick input

None.

## Scope

**In:**
- **`shuyi_core::bots::balance`** (`crates/core/src/bots/balance.rs`,
  tests in `bots/balance/tests.rs`):
  - `pub trait Knob<S>`:
    - `fn name(&self) -> &str;`
    - `fn range(&self) -> RangeInclusive<i32>;` the steps it may take,
      0 being the stage as authored (the range holds 0);
    - `fn stride(&self) -> u32 { 1 }` the first step size the search
      tries (a coarse knob, such as every enemy's level, starts larger);
    - `fn apply(&self, stage: &S, step: i32) -> Option<S>;` `stage`
      with this knob moved `step`, or `None` when it can't apply there
      (no free tile for one more piece). Knobs apply in the order
      given, each to the result of the one before;
    - `fn describe(&self, step: i32) -> String;` the change in plain
      words, such as `"the foe trips 10% less often"`.
  - `BalanceSettings { tries: Tries, quick: u32, evaluations: u32 }`,
    with `Serialize` and `Deserialize`: `tries` is 2126's full run (it
    decides "in band"); `quick` the try count for the search's own
    evaluations (same first seed); `evaluations` the budget, counted in
    runs of the playtest runner, never time.
  - **The distance** of a report to its bands: over every
    `Check::Rate`, how far its success share lies outside its band, in
    integers (`max(0, min * count - successes * 100, successes * 100 - max * count)`),
    summed. 0 when every rate is inside. `Check::Median` doesn't count:
    it is about the bots, not the stage.
  - `pub fn autobalance<S: Clone, R: Sides>(stage: &S, knobs: &[&dyn Knob<S>], entrants: &[&dyn Entrant<S, R>], checks: &[Check], settings: &BalanceSettings) -> Result<Balanced<S>, PlaytestError<R>>`.
    The search, all deterministic:
    1. **Before:** run the authored stage at `tries`. If its distance
       is 0, stop (`Stop::AlreadyInBands`) with no change.
    2. **Pattern search** at `quick` tries, from every knob at 0, each
       knob's stride its `stride()`. In knob order, try `+stride` then
       `-stride` (clamped to the range; skip a step that doesn't move,
       or whose `apply` is `None` for any knob). Take the first that
       lowers the distance and start the knob order again. When no
       knob lowers it, halve every stride above 1; when every stride
       is 1 and none lowers it, stop (`Stop::Stuck`).
    3. **Confirm:** when a setting reaches distance 0 at `quick`, run
       it at `tries`. If that is 0 too, go to 4; if not, carry on the
       pattern search at `tries` from there.
    4. **Prune:** for each changed knob in order, try it at 0, then
       nearer 0 one step at a time, keeping each change that stays at
       distance 0 at `tries`. What is left is the smallest change set
       this search found (not proved the smallest: said in the docs).
    5. **Budget:** every run of the playtest runner counts one
       evaluation. When `evaluations` runs out, stop
       (`Stop::Budget`) with the best setting seen.
    The same seeds play every evaluation, so settings are compared on
    the same luck, and the same inputs always give the same changes.
  - `Balanced<S>`: the changed stage, the change set (`(knob name,
    step, describe(step))` for every knob not at 0, in knob order),
    the `PlaytestReport` before and after (at `tries`), `stop: Stop`
    and the evaluations used. `impl Display`: why it stopped, the
    changes one per line, then the two reports' tables.
  - Re-exports in `crates/core/src/lib.rs`: `Knob`, `BalanceSettings`,
    `autobalance`, `Balanced` and `Stop`.
- **Hello-grid shows it:**
  - `Stage::with_tuning(map, seed, tuning)`, where
    `Tuning { trip_percent: u8, foe_waits: u8 }` (the foe skips its
    first `foe_waits` turns); `Stage::new` keeps `TRIP_PERCENT` and no
    wait. The tuning serializes only when it isn't the default, so the
    state hashes of existing records don't change;
  - two knobs in `src/tools.rs` over `(map, Tuning)`: `trip`, five
    points of trip chance a step, from 0% to 75%, stride 2; and
    `head start`, the foe's wait, 0 to 3 turns;
  - `tests/it/balance.rs`: with 2126's hello-grid entrants and a band
    on `walk` that the stage at trip 0% misses (measured once and
    written down), `autobalance` from trip 0% lands every rate inside
    its band, its changes are in the knobs' words, the same inputs give
    the same changes twice, and no knob was applied outside its range.
- **What changes for the game:** nothing breaks. game#0509's PR moves
  the pin, writes the battle's knobs, and makes `cargo xtask autobalance`
  a thin wrapper that loads the battle, calls `autobalance` and writes
  the file back with `--apply`.

**Out (do not do):**
- changing an entrant, a bot or a check (the bots are never tuned to
  pass);
- writing a stage back to its file (the game's, as its format allows);
- a `shuyi autobalance` contract operation (a follow-up when a game
  asks for it);
- generating a stage from nothing (the game's 0510, and parked
  2119's generators);
- random restarts, annealing or any search that isn't deterministic;
- threads, time limits.

## Implementation steps

1. Read 2126's `bots/playtest.rs` and its tests.
2. Write `Knob`, `BalanceSettings`, the distance, `Stop`, `Balanced`
   and `autobalance` in `bots/balance.rs`.
3. Unit tests with a fixture game in `bots/balance/tests.rs`: a
   one-side "coin race" whose stage is a target score and whose bot
   wins with a share that falls as the target rises, so the right
   setting is known by hand.
4. Hello-grid's `Tuning`, `with_tuning`, the knobs and
   `tests/it/balance.rs`, and a line in `README.md`.
5. Run the gates.

Size: about 400 changed lines (the search about 200, its tests about
100, hello-grid about 100).

## Acceptance criteria

- [x] On the coin race, a target set too high comes back to the
      setting worked out by hand, and an authored stage already inside
      its bands stops with `AlreadyInBands` after one evaluation.
- [x] The prune step removes a change that isn't needed (a fixture with
      a second knob that does nothing).
- [x] With a budget of 3 evaluations, the search stops with `Budget`
      and the best setting seen.
- [x] The same inputs give the same `Balanced` twice.
- [x] Hello-grid's `tests/it/balance.rs` passes, and its existing
      records and snapshots pass unchanged.
- [x] `shuyi-core` still builds for `thumbv7em-none-eabihf` (CI's
      `no-std` job).
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the distance; each stop; strides halving; a knob whose `apply`
  is `None`; the prune.
- Property: every setting tried stays inside every knob's range; the
  distance is 0 iff every rate check is `Within`.
- Integration: hello-grid's `tests/it/balance.rs`.

## Completion notes

What was done:

- `shuyi_core::bots::balance` (`crates/core/src/bots/balance.rs`): `Knob`,
  `BalanceSettings`, `distance`, `Stop` (`AlreadyInBands`, `InBands`,
  `Stuck`, `Budget`), `Change`, `Balanced` with `Display`, and
  `autobalance`: before, the pattern search at `quick` tries, the
  confirm at `tries` (carrying on at `tries` when it fails), the prune,
  and the budget, as the ticket says. Every setting's distance is cached
  by its steps and try count, so no setting plays twice at one count.
  `Knob`, `BalanceSettings`, `autobalance`, `Balanced` and `Stop` are
  re-exported at the crate root. 14 tests on a coin race whose right
  setting is worked out by hand (target 9 to target 4, in 8
  evaluations), two of them property tests.
- Hello-grid: `Tuning { trip_percent, foe_waits }`, `Stage::with_tuning`,
  `Stage::tuning` and `Stage::retuned`; the tuning is written only when
  it isn't the default, so every record, hash and snapshot is unchanged.
  The foe's waiting turns give a new `Event::Waited`. The knobs `Trip`
  (five points a step, 0% to 75%, stride 2) and `HeadStart` (0 to 3
  turns) in `src/tools.rs`, and `tests/it/balance.rs`.

Measured on seeds 0–49 (turn limit 200): the walker wins 50 of 50 at
every tuning (trip 0% to 75%, any head start), and the foe stumbles at
least once in its tries 0, 11, 16, 20 and 30 times out of 50 at trip 0%,
5%, 10%, 15% and 20%. From trip 0% with `playtest.ron`'s checks and a
band of 50–70% on that, the search lands on trip +4 (20%, 30 of 50).

Deviations:

- **The band is on the walker's `foe trips`, not on its wins.** The
  walker wins every try at every tuning, so no band on its wins can be
  missed at trip 0%. The random hero's wins do rise with the trip chance
  (about 11% at 0% to 43% at 75% over 400 tries), but they dip just above
  0% (37 of 400 at 10% against 47 at 0%), so the pattern search sticks
  at the stage as authored. The check is "the foe stumbles at least once
  in 50–70% of the walker's tries", still a band on `walk`.
- **The knobs are over `Stage`, not `(map, Tuning)`.** 2126's entrants
  start a `Stage`, and a stage at turn 1 is its map and its tuning, so a
  knob retunes the stage (`Stage::retuned`) and the entrants stay as they
  are. Each knob holds the authored value it counts its steps from.
- A change is a `Change { knob, step, what }` struct, not a tuple. The
  stop of a search that lands is `Stop::InBands`.
- A knob at 0 isn't applied. A search stopped by `Stuck` or `Budget`
  keeps its setting only when it plays nearer its bands at `tries` than
  the stage as authored, and the quick runs keep one evaluation back for
  that full run, so `evaluations` is never passed.
- The strides halve as a shift by a pass number, each pass makes at most
  one move per evaluation, and the prune walks a finite range, so no
  mutant of a comparison or a counter can stall the search (what 2128's
  mutation shards ran into).

Follow-up tickets: none. 1134's search clause now says the autobalance
search is done.

For the game: nothing breaks. `Stage` (hello-grid) is the only type that
changed, and `shuyi-core` only gains items. The game PR that moves the
pin is game#0509's (not opened yet): it writes the battle's knobs and
makes `cargo xtask autobalance` a thin wrapper over `autobalance`.

Nick input: none.
