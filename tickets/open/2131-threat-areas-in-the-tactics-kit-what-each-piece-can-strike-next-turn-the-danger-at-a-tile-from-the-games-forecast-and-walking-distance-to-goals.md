---
id: "2131"
title: "Threat areas in the tactics kit: what each piece can strike next turn, the danger at a tile from the game's forecast, and walking distance to the nearest goal"
type: feature
milestone: After MVP
model: sonnet-5
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2131 — Threat areas, danger and walking distance to goals

## Context

A request from the game.

1. **The game ticket:** game#0506, the persona playtest bots
   (`github.com/Zafnok/visions-of-shuyi/blob/main/tickets/done/0506-persona-bots-casual-normal-hardcore.md`).
   The game's follow-up that moves onto this ticket is game#1180.
   Neither is a blocker of this one.
2. **The job in one sentence:** for a bot that plays like a person who
   reads the danger zone, say which tiles each opposing piece can strike
   next turn, how much danger a tile stands in (the game's forecast of
   each threat that reaches it, added up), whether a tile is out of
   every threat, and how far a piece would walk from a tile to the
   nearest of some goals.
3. **No friction review triggered:** game#0506 measured 1.8×. Nick
   (2026-10-22) asked that the generic parts of the game's playtesting
   move into the engine; this is one ticket of that sweep (game
   `docs/engine/friction.md`, row "0506 (sweep)"). In the game's
   `crates/bots/src/persona.rs`, `threats`, `danger`, `Goals` (a flow
   field cached per movement type, goal tiles free to enter) and their
   helpers are about 165 production lines, of which about 40 are the
   game's own (which pieces never move, the forecast, what a lord is).
   The game's `trpg_core::threat_area` (`crates/core/src/movement.rs`)
   is the same union of attack ranges around a reach, written over the
   game's own map types. With this ticket the game keeps the hooks,
   about 40 lines. game ADR-0095 and `docs/design/playtest-bots.md`
   (the owner's rule: a bot knows the danger zone and the forecasts a
   player reads, never whom the other side will choose) are why the
   danger is a sum over every threat, not a prediction.
4. **The generic shape** (tactics kit, beside `path.rs`): from a piece's
   stop tiles (the kit's `Reach` from `path::reachable`, filtered by the game's own occupancy
   rule) and its strike ranges (inclusive Manhattan distance pairs), the
   tiles it can strike; for a piece that never moves, the tiles in range
   of where it stands. A list of threats (a game id and its area) answers
   "which threats reach this tile" and "is this tile safe". The danger
   at a tile adds up a game-supplied strike forecast of each threat that
   reaches it: expected damage (in hundredths), the damage if every
   strike hits, and the expected damage struck back. And a cache of the
   kit's `flow_field` per movement kind, with the goals' own tiles free
   to enter (a goal is often a tile a piece stands on), answering
   "how far from this tile to the nearest goal, walking".
5. **Examples from games that aren't tactics:** a grid roguelike that
   shows a monster's reach as a danger overlay (as Into the Breach and
   Dungeon Crawl Stone Soup do) and whose auto-explore keeps out of it;
   a tower-defense game's tower coverage (a fixed tile, a range band);
   a farming game's sprinkler or scarecrow coverage (the fixed-tile
   area alone); a puzzle game's guard sightlines on a grid. The first
   two are grid games, so this belongs in the tactics kit (ADR-0009's
   modules), not the bare core.

## Nick input

None.

## Scope

**In:**
- `shuyi_core::threat` (`crates/core/src/threat.rs`), re-exported where
  `path` is: `strike_area`, `strike_area_from`, `Threat<P>`,
  `Threats<P>`, `Strike`, `Danger`, `Threats::danger`, `GoalDistance<K>`.
- Integer-only, deterministic, no allocation in `Threats::reaches` and
  `Threats::safe`.
- Tests; module docs; the kit's docs page if there is one (the
  `docs/README.md` map says where the grid kit is documented).
- **From 1134 (done):** add a short section on threat areas to
  `skills/shuyi-balance-bots/SKILL.md` (`strike_area`, `Threats` with
  `safe`, `zone` and `danger`, and `GoalDistance`); `check-skills`
  passes.

**Out (do not do):**
- Line of sight, eight-way moves (2117, 2120): ranges are Manhattan.
- Who strikes whom, damage, hit chances: the forecast is the game's
  closure.
- Predicting the other side's choice (game ADR-0095: every threat
  counts).
- A candidate order for planners (2132 builds on this).
- The game's use of it (game#1180).

## Implementation steps

1. **Read** `crates/core/src/path.rs` (`Reach`, `reachable`,
   `flow_field`), `grid.rs` (`TileSet`, `Pos::manhattan`), and the game's
   `crates/bots/src/persona.rs` (`threats`, `danger`, `Goals`) and
   `crates/core/src/movement.rs` (`threat_area`, `attack_tiles`) for the
   shapes this generalizes.
2. **Strike areas**:
   - `pub type Ranges = [(u32, u32)]` (each inclusive, min ≤ max; a pair
     with min > max is empty).
   - `strike_area(stops: &TileSet, ranges: &Ranges) -> TileSet`: every
     tile of the set's grid within one of `ranges` of a stop tile, plus
     the stop tiles themselves when `ranges` is not empty (a piece that
     can strike threatens where it can stand, as the game's
     `threat_area` does). Empty `ranges`: an empty set.
   - `strike_area_from(width, height, at: Pos, ranges: &Ranges) -> TileSet`:
     the same for a piece that never moves (only `at` as a stop).
   - Write it as a diamond walk per stop tile, clipped to the grid; note
     its cost (stops × ring tiles) in the docs.
3. **Threats**: `Threat<P> { pub piece: P, pub area: TileSet }` and
   `Threats<P>(Vec<Threat<P>>)` with `reaches(&self, at) -> impl
   Iterator<Item = &P>`, `safe(&self, at) -> bool` and `zone(&self) ->
   TileSet` (the union, the classic danger zone).
4. **Danger**: `Strike { pub expected: i64, pub all_hit: i64, pub struck_back: i64 }`
   (expected values in hundredths of a point, as a hit chance in percent
   times damage) and `Danger { pub expected: i64, pub worst: i64, pub struck_back: i64, pub threats: u32 }`;
   `Threats::danger(&self, at, strike: impl FnMut(&P) -> Option<Strike>) -> Danger`
   adds up each threat that reaches `at` and returns `Some`. Rounding
   and caps (at most the target's health, a share of the attacker's) are
   the game's: say so in the docs.
5. **`GoalDistance<K: Ord + Copy>`**: `new(width, height, goals: Vec<Pos>)`;
   `distance(&mut self, kind: K, from: Pos, cost: impl Fn(Pos) -> Option<u32>) -> Option<u32>`
   builds `flow_field` for `kind` on first use (the cost closure then
   wrapped so a goal's tile costs 0 to enter) and reads it. `None`
   for no goals, or no walk. Document that `cost` must be the same for
   every call with one `kind` (the field is cached).
6. **Docs**: module docs with a 5×5 picture in a doc comment (a piece,
   its stops, a 1–2 range, the area), and the "every threat counts"
   rule as the reason `danger` adds instead of predicting.

## Acceptance criteria

- [ ] `strike_area` of a single stop with ranges `[(1, 1)]` is the stop
      and its four neighbours; with `[(2, 2)]` the stop and the ring of
      8 at distance 2; clipped at a grid corner.
- [ ] `strike_area` equals the brute force ("every tile whose Manhattan
      distance to some stop is in some range, plus the stops") on random
      grids, stops and ranges (property test).
- [ ] `Threats::safe` is false exactly on `zone()`'s tiles.
- [ ] `danger` at a tile two threats reach, with a forecast closure that
      returns `None` for one of them, adds only the other.
- [ ] `GoalDistance` gives 0 on a goal, the walking distance around a
      wall (not the Manhattan one), `None` behind an uncrossable river,
      and builds one field per kind (a counting cost closure is called
      only on the first `distance` of each kind).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the criteria above, in `crates/core/src/threat/tests.rs`.
- Property: `strike_area` against the brute force.
- Determinism: no `HashMap` iteration order anywhere (`BTreeMap` for
  the cache).

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
