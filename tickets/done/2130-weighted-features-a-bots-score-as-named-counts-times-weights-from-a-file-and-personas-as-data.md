---
id: "2130"
title: "Weighted features: a bot's score as the game's named counts times weights from a file, and named personas (planner settings and weights) as data"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-10-22
---

# 2130 — Weighted features and personas as data

## Context

A request from the game.

1. **The game ticket:** game#0506, the persona playtest bots
   (`github.com/Zafnok/visions-of-shuyi/blob/main/tickets/done/0506-persona-bots-casual-normal-hardcore.md`).
   The game's follow-up that moves onto this ticket is game#1180.
   Neither is a blocker of this one.
2. **The job in one sentence:** score how good a planned turn is for a
   kind of player as "each thing that changed, times what this player
   cares about it", with the things counted by the game and the weights
   in a file, and keep each kind of player (its thinking budget and its
   weights) as a named entry of a data file.
3. **No friction review triggered:** game#0506 measured 1.8× (1,060
   added production lines against 600). Nick (2026-10-22) asked that the
   generic parts of the game's playtesting move into the engine; this
   is one ticket of that sweep (game `docs/engine/friction.md`, row
   "0506 (sweep)"). In the game's `crates/bots/src/persona.rs`, `Weights`
   (16 named `i32` fields), `Persona`, `personas_from_ron`, `Tally` (the
   same 16 names as `i64` fields) and `Tally::score` (a 16-row table
   pairing them) are about 108 production lines, of which about 28 are
   the game's real logic (`Tally::between`: what changed). Each new
   count is written in four places. With this ticket the game keeps its
   feature list and the counting, about 35 lines.
4. **The generic shape:** a fixed list of feature names a game declares
   (`["won", "lost", "hp lost", …]`), weights read from data by name
   (an unknown name is an error, a missing one is 0), and a score that
   is the saturating integer sum of count × weight; an `Evaluate`
   adapter (2125's trait) that calls the game's function for the counts
   and weighs them. Beside it, a persona entry: 2128's `PlanSettings`,
   the weights by name, and a field the game fills with its own extras
   (a game mode, a difficulty), so `BTreeMap<String, Persona<X>>` is a
   personas file.
5. **Examples from games that aren't tactics:** a deck-builder's bots
   scoring "health", "enemy health", "cards in hand" and "energy left",
   a careful persona and a greedy one differing only in weights; a
   farming sim's bots weighing "gold", "crops watered" and "friendship";
   a racing game's AI drivers trading "position" against "tyre wear".
   Each writes the same struct pair and the same score table today.

## Nick input

None.

## Scope

**In:**
- `shuyi_core::bots::weigh` (`crates/core/src/bots/weigh.rs`),
  re-exported from `bots.rs`: `Weights<N>`, `UnknownFeature`,
  `Weighted<F, N>` and `Persona<X>`.
- Integer-only, no allocation per score (a planner calls it for every
  partial turn on every copy).
- Tests on the fixtures of `crates/core/src/bots/fixtures.rs`.
- Module docs, `bots.rs`'s list.

**Out (do not do):**
- Counting anything: what a feature means and how it is counted are the
  game's (`Evaluate`'s `start` and `end`).
- Reading files (no I/O in the core; a game reads RON, and
  `shuyi playtest` (2532) asks the game for its entrants).
- Floats, or learned weights (a trained bot is a game's research).
- The game's use of it (game#1180).

## Implementation steps

1. **Read** `crates/core/src/bots/determinize.rs` (`Evaluate`),
   `plan.rs` (`PlanSettings`, its serde), and the game's
   `crates/bots/src/persona.rs` (`Weights`, `Tally`, `Tally::score`) and
   `crates/bots/personas.ron`.
2. **`Weights<const N: usize>`**: `[i32; N]` in the order of the game's
   names.
   - `Weights::from_named(names: &[&str; N], named: &BTreeMap<String, i32>) -> Result<Self, UnknownFeature>`:
     each name of `named` must be in `names` (else
     `UnknownFeature { name, known: names.join(", ") }`, with `Display`
     "no feature \"x\" (there: a, b, c)"); names not in `named` weigh 0.
   - `score(&self, counts: &[i64; N]) -> i32`: the sum of
     `count × weight` in `i64` with saturating adds, clamped into `i32`.
   - `get(&self, name)`, for tests and reports.
3. **`Weighted<F, const N: usize>`** `{ pub weights: Weights<N>, pub counts: F }`
   where `F: Fn(&R, &R, R::Side) -> [i64; N]`, implementing
   `Evaluate<R>` as `self.weights.score(&(self.counts)(start, end, side))`.
4. **`Persona<X>`**: `{ pub settings: PlanSettings, pub weights: BTreeMap<String, i32>, #[serde(default)] pub game: X }`
   with `deny_unknown_fields` and serde bounds as 2129's `Targets`
   (`X: Default + Deserialize`), plus
   `Persona::weights::<N>(&self, names) -> Result<Weights<N>, UnknownFeature>`.
   Its RON entry reads:
   ```ron
   "careful": (
       settings: (width: 4, candidates: 10, depth: 6, samples: 2, temperature: 0),
       weights: {"won": 1000, "hp lost": -4},
       game: (mode: Classic),
   ),
   ```
5. **Fixture**: hello-grid's fixture game (or the bots fixtures) gets a
   three-feature `counts` function; two personas with opposite weights
   pick different first commands through `TurnPlan` at temperature 0.
6. **Docs**: module docs with the RON example above.

## Acceptance criteria

- [x] `Weights::from_named` on a map naming an undeclared feature fails
      with `UnknownFeature`, and its message lists the known names.
- [x] `score` saturates: counts of `i64::MAX` with weight 2 give
      `i32::MAX`, never a wrap.
- [x] A missing weight is 0.
- [x] Two `Persona`s parsed from one RON map, on the same fixture state,
      make `TurnPlan` pick different first commands.
- [x] `Weighted` is an `Evaluate` with no allocation per call (a test
      that counts with `#[global_allocator]` is not needed: `score` takes
      and returns no heap type).
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the criteria above.
- Property: `score` equals the `i128` sum clamped into `i32`, for any
  counts and weights.

## Completion notes

- **Done:** `shuyi_core::bots::weigh` (`crates/core/src/bots/weigh.rs`),
  re-exported from `bots`: `Weights<N>` (`from_named`, `score`, `get`),
  `UnknownFeature` (an `Error`, "no feature \"x\" (there: a, b, c)"),
  `Weighted<F, N>` (an `Evaluate`) and `Persona<X>` (serde,
  `deny_unknown_fields`, `weights` and `game` defaulted). Module docs carry
  the RON example; `bots.rs`'s list names the module.
- **Tests** (`weigh/tests.rs`, on the bots fixtures' Nim with three
  features "won", "lost", "taken"): the unknown feature and its message,
  a missing weight, saturation both ways, `Weighted` as an `Evaluate`,
  two personas parsed from one RON map with opposite "taken" weights
  picking 3 and 1 through `TurnPlan` at temperature 0 (the pile comes
  from the persona's own `game` extras), a persona with no weights or
  extras, unknown fields refused, and the property that `score` is the
  `i128` sum clamped into `i32`.
- **Deviations:**
  - `score` sums in `i128` (products of `i64 × i32` never overflow
    there), not in saturating `i64`: a saturating `i64` sum differs from
    the true sum when a huge term is later cancelled, so the ticket's own
    property ("equals the `i128` sum clamped") couldn't hold. Still
    integer-only, no allocation.
  - `get` takes the names too (`get(&names, "won")`): `Weights` holds
    only the numbers, as step 2 says.
  - The three-feature counts function lives in the module's tests rather
    than in `fixtures.rs`, its only user.
  - `Persona` has no JSON schema derive: `PlanSettings` has none.
- **Docs:** rustdoc only (the module docs); no contract operation, format
  or file of the engine changed.
- **Game:** game#1180 moves the game's `persona.rs` onto this and moves
  the pin. API is additive; nothing the game uses changed.
- **Follow-ups:** none.
