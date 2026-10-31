---
id: "2125"
title: "A rollout bot over any TurnRules game: Determinize, and flat Monte Carlo on copies that never see the luck to come"
type: feature
milestone: After MVP
model: reasoning
effort: high
status: done
blocked_by: []
nick_input: none
completed: 2026-10-21
---

# 2125 — A rollout bot: Determinize and flat Monte Carlo

## Context

A request from the game, split out of 2113 on 2026-10-21.

1. **The game tickets:** game#0506, the persona playtest bots
   (`github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/0506-persona-bots-casual-normal-hardcore.md`),
   and through it game#0509 (autobalance) and game#0512 (the bots play
   all of Act 1). They are the game's Act 1 critical path (its
   `docs/ROADMAP.md`, Lane C), and they now wait for this ticket. They
   are not blockers of this one.
2. **The job in one sentence:** a computer player that tries each of a
   few good-looking moves many times on copies of the game where the
   dice and the hidden things are redrawn, plays each copy out a little
   way with simple players, and picks the move whose copies ended best.
3. **No friction review:** a missing feature. Nick wants the engine "as
   capable as possible to support our game" (2026-10-21), and the game's
   0506 already described exactly this bot as game code. The generic
   parts move here; the game keeps its personas and their scoring.
4. **The generic shape:** `Determinize` (a copy of the state as one side
   knows it) and a flat Monte Carlo bot over `TurnRules + Sides`: the
   candidates come from `legal_commands`, capped and ordered by the
   game; each is played out `samples` times on determinized copies by
   game-supplied playout players up to a horizon; a game-supplied score
   is averaged; a base-2 softmax picks, from the bot's own seed. All in
   integers, limits in work units.
5. **Not only tactics:** a deck-builder's "which card do I play" bot
   (each card played out over a few redrawn shuffles), a board game with
   dice (backgammon-style rollouts), a roguelike's playtest bot choosing
   between stairs and a fight. Every turn game whose branching is too
   wide for a full tree search (2113) but whose moves can be played out
   by a simple player.

Why not 2113: its minimax and full MCTS search a tree of every side's
replies. A wide map with many pieces per side branches far too much
for that; 2113 says so itself ("a wide tactics map branches too much
for a game-tree search"). Flat Monte Carlo searches one level: the
bot's own next command, each judged by playouts. 2113 waits for 2112
and 3009 (which waits for 3008's benchmarks); this ticket waits for
nothing, because a playtest bot runs on a PC. 2113 now builds on this
ticket's `Determinize` and `Evaluate`.

Shuyi today:

- `crates/core/src/bots.rs`: `Sides`, `Bot`, `play`, `play_observed`
  and `RandomBot`. Its docs say a bot must never learn the luck to
  come; nothing enforces it (a bot that clones the state sees the rolls
  kept in the state's `SimRng`).
- `crates/core/src/rng.rs`: `SimRng` (`new`, `next_u32`, `roll_below`).
- `examples/hello-grid/src/rules.rs`: `Stage` (its luck in a `SimRng`;
  the foe may trip, `TRIP_PERCENT`), `Chase` (the foe's mind) and
  `distances_to_hero`. Its hero is a `RandomBot` in `tests/it/bots.rs`
  and in `src/tools.rs`'s `bot_play`.
- Nick's rule of 2026-10-18 (`docs/decisions.md`): a computer opponent
  sees only what its side sees, unless its difficulty settings say
  otherwise; it thinks in work units, never time.

Placement (ADR-0009 §1 and §7): it stays in `shuyi-core` beside `bots`,
like 2112's solver. It is generic, adds no dependency, is `no_std`, and
a game that never names it pays nothing. Hello-grid shows it.

## Nick input

None.

## Scope

**In:**
- **`shuyi_core::bots::determinize`** (`crates/core/src/bots/determinize.rs`):
  - `pub trait Determinize: Sides + Clone { fn determinize(&self, side: Self::Side, seed: u64) -> Self; }`:
    a copy of the state as `side` knows it, with everything it can't
    see redrawn from `seed`: the luck to come (a new
    `SimRng::new(seed)`), and the other sides' hidden things (a hand
    dealt again from what is left, a piece that hasn't shown up yet
    removed). A game with no luck and nothing hidden returns
    `self.clone()` and says so in a comment. A game whose difficulty
    lets a bot see more says so in its `determinize`. **The bots that
    plan on copies take only games that implement it**, so every game
    chooses what its bots may see.
  - The copy keeps the real state's `legal_commands` for `side` at the
    root (same list, same order). The rollout bot checks it and panics
    with a message naming `determinize` when it doesn't (a bug in the
    game's `determinize`).
  - `pub trait Evaluate<R: Sides> { fn score(&self, start: &R, end: &R, side: R::Side) -> i32; }`:
    how good `end` is for `side`, higher is better, in whole points,
    reached from `start` (the state the decision began in; a score that
    only needs the end ignores it, one that counts what was lost on the
    way compares the two). A finished game scores its outcome. A
    blanket impl for `Fn(&R, &R, R::Side) -> i32`, so a closure works.
- **`shuyi_core::bots::rollout`** (`crates/core/src/bots/rollout.rs`,
  tests in `bots/rollout/tests.rs`):
  - `pub trait Order<R: Sides> { fn order(&mut self, rules: &R, commands: Vec<R::Command>) -> Vec<R::Command>; }`:
    the game's candidate list, from the real state's `legal_commands`:
    it may drop commands (one the game never wants tried) and puts the
    most promising first. A blanket impl for `FnMut(&R, Vec<R::Command>) -> Vec<R::Command>`.
    `Unordered` keeps `legal_commands` as they are.
  - `pub trait Playout<R: Sides> { fn command(&mut self, rules: &R) -> R::Command; }`:
    how a copy is played on after the candidate, for whichever side is
    to move (a game matches on `side_to_move` to use its baseline for
    one side and its enemy mind for another). A blanket impl for every
    `Bot<R>`. Its randomness, if any, comes from the copy (whose luck
    was redrawn), so it needs no seed of its own.
  - `RolloutSettings { candidates: u32, samples: u32, horizon: Horizon, commands: u32, temperature: u32 }`,
    with `Serialize` and `Deserialize` (a persona's settings are data),
    and `Horizon` an enum: `Sides(u32)` (a playout stops after the side
    to move has changed that many times) or `Turns(u32)` (after
    `Sides::turn` has gone up that many). A `samples` or `candidates`
    of 0 counts as 1. `commands` caps a playout's length, so a playout
    player that never ends its side's move can't run forever.
  - `Rollout<O, P, E>`, built by `Rollout::new(order, playout, eval, settings, seed)`,
    implements `Bot<R>` for `R: Determinize` with
    `R::Command: Clone + PartialEq`, `O: Order<R>`, `P: Playout<R>`,
    `E: Evaluate<R>`. A decision:
    1. the bot's side is the side to move;
    2. the candidates are `order(rules, rules.legal_commands())`, the
       first `candidates` of them;
    3. `samples` seeds are drawn from the bot's own `SimRng`, each from
       two `next_u32` draws (the first is the high half), **once per
       decision and shared by every candidate** (each candidate is
       judged on the same redrawn luck, which makes the comparison
       fairer at the same cost);
    4. for each candidate and each seed: `copy = rules.determinize(side, seed)`,
       apply the candidate (a refusal panics naming `determinize`),
       then the playout player plays the copy until it ends, the
       horizon is reached, or `commands` commands have been applied (a
       command the copy refuses ends that playout where it is and is
       counted, never retried); then `eval.score(rules, &copy, side)`;
    5. each candidate's sum of scores in `i64`; its mean is the sum
       divided by `samples` (`div_euclid`);
    6. the pick: with `temperature` 0, the best mean (ties go to the
       earlier candidate in the order); otherwise a softmax in base 2,
       where a candidate `temperature` points below the best is half
       as likely as the best: weight = `exp2_q16(-(best - mean) * 65536 / temperature)`,
       an integer `2^x` in 16.16 fixed point (whole part by shift, the
       fraction from a 16-entry table, a candidate more than 32 halvings
       down weighs 0), and one `roll_below(total weight)` draw from the
       bot's `SimRng`. No floats anywhere (`AGENTS.md` rule 5).
  - **Work units, never time.** A unit is one command applied to a copy
    (the candidate's own included). A decision does at most
    `candidates × samples × (commands + 1)` units, and nothing in it
    reads a clock.
  - **Ready for slicing, not waiting for it.** The decision's loop
    state lives in a value, `RolloutSearch<R>`, with
    `fn advance(&mut self, units: u32) -> Option<R::Command>`
    (`None` while working); `choose` calls `advance(u32::MAX)`. Every
    draw from the bot's `SimRng` happens at the decision's start (the
    seeds) or its end (the pick), so how the work is split can never
    change them. When 3009 lands, `impl Think for RolloutSearch` is a
    few lines there; this ticket doesn't wait for it and doesn't add
    `Think`.
  - Re-exports in `crates/core/src/lib.rs`: `Determinize`, `Evaluate`,
    `Order`, `Unordered`, `Playout`, `Rollout`, `RolloutSettings`,
    `RolloutSearch` and `Horizon`; the crate docs' line on `bots` names
    them.
- **An ADR** (the next free number in `docs/adr/`, with the `write-adr`
  skill): bots that plan see a copy from `Determinize`, never the state
  itself; scores, means and the softmax in integers (base 2, the
  temperature in points); one set of seeds per decision shared by the
  candidates; work units and draws only at a decision's ends. It
  records how the 2026-10-18 rule ("sees only what its side sees") is
  kept, and that 2113's search bots and 2126's playtest runner build on
  it. It extends ADR-0008 §6 (work units).
- **Hello-grid shows it** (`examples/hello-grid/`):
  - `Stage` implements `Determinize` in `src/rules.rs`: its `rng` is
    redrawn; nothing else is hidden;
  - `pub fn distances_to_goal(&self) -> Grid<Option<u32>>` (a
    `flow_field` from the goal, like `distances_to_hero`);
  - `pub fn score(start: &Stage, end: &Stage, side: Side) -> i32` in the
    same file: for the hero, `Won` is 1 000 and `Caught` is -1 000;
    between them, the foe's steps to the hero minus the hero's steps to
    the goal, with 99 for a tile no path reaches. The foe's score is
    the negation. `start` is unused (said in a comment);
  - `pub struct Walk;`: a `Bot<Stage>` for the hero that moves to the
    tile in reach closest to the goal (the first in `reach` order among
    equals), and the playout player `pub fn playout(stage: &Stage) -> Command`:
    `Walk` for the hero, `Chase` for the foe;
  - the hero's order: tiles in reach sorted by their distance to the
    goal, nearest first;
  - `tests/it/bots.rs`: a rollout hero (8 candidates, 4 samples,
    `Horizon::Turns(3)`, temperature 0) against `Chase` wins more of
    seeds 0 to 19 than `RandomBot` does (both counts in the Completion
    notes), and the hero's first command doesn't change when only the
    stage's seed does;
  - `README.md`: the `src/rules.rs` row names `Determinize`, the score
    and `Walk`, and a sentence after the tools block says
    `tests/it/bots.rs` plays a rollout hero against the foe;
  - the stage screen's foe stays `Chase`, and `bot_play`'s hero stays
    `RandomBot`, so records and snapshots don't change.
- **Either order with 1134:** if `skills/shuyi-balance-bots/SKILL.md`
  exists, add a short section on the rollout bot and `Determinize`.
- **Either order with 2015:** if its float ban in `shuyi-core` is in,
  `bots` passes it. If not, no `f32` or `f64` appears in `bots` outside
  tests.
- **What changes for the game:** nothing breaks; `Bot`, `play` and
  `RandomBot` stay as they are. game#0506's PR moves the pin and
  implements `Determinize`, `Order`, `Playout` and `Evaluate` for its
  battle.

**Out (do not do):**
- minimax and full MCTS (2113, which now builds on this ticket);
- `Think` and slicing over frames (3009). `RolloutSearch::advance` is
  the seam; don't add the trait;
- the playtest runner and pass bands (2126), autobalance (2127);
- the game's personas, scoring, measures or battle (game#0506);
- threads, time limits, transposition tables, progressive widening;
- one description of a game's types and how a view differs from the
  state (2907). If its ADR is in, this ticket's ADR says how
  `Determinize` fits it;
- moving hello-grid's stage screen or `bot_play` onto the rollout hero.

## Implementation steps

1. Read `bots.rs` and its tests, `rng.rs`, ADR-0008 §6, and the
   2026-10-18 entry of `docs/decisions.md`. If 2013's ADR on
   `#[non_exhaustive]` and constructors is in `docs/adr/`, follow it for
   `RolloutSettings`.
2. Turn `bots.rs` into a module with `bots/determinize.rs` and
   `bots/rollout.rs` (the existing `bots/tests.rs` stays).
3. Write `Determinize` and `Evaluate`, then the test fixtures in
   `bots/fixtures.rs` (`#[cfg(test)]`):
   - a dice race: two sides take turns; a side either steps 2 or rolls
     1 to 6 from the state's `SimRng` and moves that many; the first to
     reach 20 wins;
   - Nim (take 1 to 3, the last take wins), with no luck
     (`determinize` returns `self.clone()`).
4. Write `exp2_q16` and its table, `RolloutSettings`, `Horizon`, and
   `Rollout` with `RolloutSearch::advance`.
5. Write the tests below, then the ADR.
6. Add hello-grid's `Determinize`, `distances_to_goal`, `score`, `Walk`,
   `playout`, the order, the tests and the README lines. Keep the test's
   settings small so the 20 plays take a few seconds in a debug build.
7. Do the either-order parts that apply, and run the gates.

Size: about 450 changed lines (the rollout bot and `exp2_q16` about
200, the fixtures and tests about 150, hello-grid about 100). If the
diff runs well past 600, move hello-grid's part to a follow-up ticket
and keep the fixtures' tests here.

## Acceptance criteria

- [x] In Nim, at temperature 0 with `Horizon::Turns(20)` and a playout
      player that takes `pile % 4` (1 when that is 0), the bot takes
      the winning amount from every pile of 1 to 20 that isn't a
      multiple of 4.
- [x] In the dice race, two states that differ only in their `SimRng`
      get the same command (property test over pairs of seeds).
- [x] Slicing: `advance(1)` repeated, `advance(7)`, `advance(100)` and
      `advance(u32::MAX)` give the same command on every fixture
      (property test with random slice sizes).
- [x] The same bot seed and the same state give the same command twice.
- [x] `exp2_q16` equals a table of known values (0, -0.5, -1, -2.25,
      -31, -33 in 16.16), and temperature 0 always picks the best mean.
- [x] At a temperature, over 1 000 decisions with different bot seeds, a
      candidate `temperature` points below another is picked about half
      as often (between 40% and 60% of its count).
- [x] A `determinize` that changes the root's commands, or a candidate
      the copy refuses, makes the bot panic with a message naming
      `determinize`.
- [x] No decision applies more than `candidates × samples × (commands + 1)`
      commands (a counting `Playout` in the test).
- [x] Hello-grid's rollout hero wins more of seeds 0 to 19 against
      `Chase` than the random hero does.
- [x] Hello-grid's existing records and snapshots pass unchanged.
- [x] `shuyi-core` still builds for `thumbv7em-none-eabihf` (CI's
      `no-std` job).
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the order and the cap; `samples` and `candidates` of 0; each
  horizon; the `commands` cap; a refused playout command ends the
  playout; `exp2_q16`; ties by order; the panic on a bad `determinize`.
- Property: slice independence; luck independence; the same answer
  twice.
- Integration: hello-grid's `tests/it/bots.rs`.

## Completion notes

**Done.**

- `shuyi_core::bots::determinize`: `Determinize` and `Evaluate` (with
  the blanket impl for closures).
- `shuyi_core::bots::rollout`: `Order` (and `Unordered`), `Playout`,
  `Horizon`, `RolloutSettings` (serde), `Rollout` and `RolloutSearch`
  with `advance`, and `exp2_q16`. All re-exported from `shuyi_core`, and
  the crate docs name them. No float anywhere in `bots`, tests included.
- Fixtures in `bots/fixtures.rs` (a dice race and Nim, whose
  `determinize` can be made to lie) and 25 tests in
  `bots/rollout/tests.rs`, three of them property tests.
- [ADR-0027](../../docs/adr/0027-bots-that-plan-see-a-determinized-copy-integer-rollouts-shared-seeds-and-work-units.md).
- Hello-grid: `Stage`'s `Determinize` (the luck redrawn),
  `distances_to_goal`, `score`, `hero_order`, `Walk` and `playout`, the
  tests in `tests/it/bots.rs` and the README lines. The stage screen
  and `bot_play` are unchanged, and so are the records and snapshots.
- **The counts:** of stages 0 to 19 against `Chase`, the rollout hero
  (8 candidates, 4 samples, `Horizon::Turns(3)`, 12 commands,
  temperature 0) won **20**, the random hero **2**.

**Deviations.**

- `RolloutSearch` has three more type parameters and a lifetime than
  the ticket's `RolloutSearch<R>`: `RolloutSearch<'a, R, O, P, E>`. It
  borrows its bot, because the playout player, the score and the bot's
  `SimRng` (the pick at the end) live there. It keeps its own clone of
  the root, so the state is free while it thinks. `advance` has the
  ticket's signature. `Rollout::search(&rules)` starts one.
- `Bot` gains a blanket impl for `FnMut(&R) -> R::Command`. Without it
  the ticket's `pub fn playout(stage: &Stage) -> Command` is not a
  `Playout`: the blanket impl for every `Bot` takes the place of one for
  closures, and the two can't both exist. The game's `Bot` impls are
  all for its own structs, so nothing clashes (checked in the game's
  `crates/bots`).
- `exp2_q16` takes an `i64` (the 16.16 exponent; a positive one counts
  as 0) and is public in `bots::rollout`, not re-exported. Between the
  table's entries it reads straight from one to the next, so it never
  rises. -31 and -33 both give 0 in 16.16; the guard against shifting
  past 32 halvings is a `checked_shr`.
- When the weights' total doesn't fit `roll_below`'s `u32` (more than
  65 535 candidates), every weight is halved as often as it needs.
- The copy's side to move is checked too, with its commands.
- Size: about 1 450 lines, against the 450 guessed. Most of it is the
  tests and fixtures the ticket lists (about 700) and the docs.
  Hello-grid's part is about 170 lines, so moving it out would not bring
  the PR near 600, and it is done and tested; it stays.
- 1134's `skills/shuyi-balance-bots/SKILL.md` doesn't exist yet, and
  2015's float ban isn't in; neither either-order part applied.

**For the game.** Nothing breaks: `Bot`, `play` and `RandomBot` are as
they were, and only the blanket `Bot` impl for closures is new. The
game PR that moves the pin is game#0506's (it implements `Determinize`,
`Order`, `Playout` and `Evaluate` for its battle); it doesn't exist yet.

**Follow-ups.** None new: 2113, 2126, 2127 and 3009 already build on
this.

**Nick:** nothing to decide.
