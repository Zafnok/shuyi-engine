---
id: "2117"
title: "Eight-way moves with a corner rule, and point-to-point paths by A* with partial paths, in the grid kit; four-way stays the default"
type: feature
milestone: After MVP
model: opus-5.5
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2117 — Eight-way moves and point-to-point paths in the grid kit

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8, and the 2D engines GameMaker and Construct 3. Grid
paths with diagonals are built in elsewhere:

- Godot's `AStarGrid2D` has diagonal modes (always, never, at least one
  walkable, only if no obstacles), Manhattan, Euclidean, Octile and
  Chebyshev heuristics, jumping, and a partial path when the goal can't
  be reached
  (https://docs.godotengine.org/en/stable/classes/class_astargrid2d.html);
- GameMaker's `mp_grid_path` takes an argument that allows diagonal
  moves, and Construct 3's Pathfinding behavior finds grid paths with or
  without diagonals;
- on Bevy, crates such as `bevy_northstar` add grid A* with
  neighbourhoods of 4 to 26 directions.

What Shuyi has today:

- `crates/core/src/grid.rs`: `Pos`, `Dir` with four values
  (`Dir::ALL`), `Pos::manhattan`, `Pos::step` and `Grid::neighbors4`.
  Nothing knows a diagonal.
- `crates/core/src/path.rs`: `reachable` (a Dijkstra within a budget,
  with `Reach::path_to`) and `flow_field`, both through one private
  `search`. Its module docs say "Only the four grid directions
  ([`Dir`]) are steps", and `search` steps through `Dir::ALL`. Nothing
  searches from one tile to another and stops at the goal, and nothing
  answers "get as close as you can" when the goal can't be reached.
- In the grid kit's drawing, `path_overlays`
  (`crates/ui/src/grid/path.rs`) draws `MapScene::path` as straight
  legs between four-way neighbours, `steer`
  (`crates/ui/src/grid/steer.rs`) appends a tile only when
  `Pos::manhattan(end, to) == 1`, and `Cursor::step`
  (`crates/ui/src/grid/cursor.rs`) takes a `Dir`.

Planned work near it:

- 2709 (parked, real-time strategy) moves in eight directions at cost
  10 and 14 with no corner cutting, with an octile A*, but over a
  portal graph inside `shuyi_rts`, not in the grid kit;
- 2106 (parked) runs A* over a navigation mesh for free movers, and
  1122 (parked) moves free movers eight ways in `Fx`;
- 2119 (parked) counts a tile's eight neighbours with `Grid::neighbors8`
  if this ticket is done; 2735 (parked) moves its hero eight ways if
  this ticket is done;
- 2121 (parked, hex grids) is blocked by this ticket: it adds
  `Topology::Hex` as a third topology, and 2740 (parked) draws it;
- 2112 and 2113 search over game states, not tiles.

Why it matters:

- **The game first:** Visions of Shuyi moves four ways and needs none
  of this. So four-way stays the default, and every existing function
  keeps its signature and its results. Nothing changes for the game at
  its next pin bump.
- **Other 2D games:** many tactics and board games move eight ways (a
  king's step). Roguelikes do too; Nick added them on 2026-10-20
  (`docs/decisions.md`), parked until Visions of Shuyi launches, and
  this ticket is the grid kit's half that 2119 and 2735 build on when
  they are unparked. Any grid game wants "walk from here to there" in
  one call: a farming game's villager walking to the shop, a puzzle
  game's hint. Today a game writes A* itself, or runs a full
  `reachable` to read one path.
- **The end goal** (2026-10-19): an agent making a grid game finds
  `find_path` in the docs and needs one line.

Integers only. A diagonal's cost comes from whole-number weights the
game picks (10 and 14 for octile, 1 and 1 for a king's step) times the
tile's cost. No square roots and no floats (2015's float ban).

## Nick input

None.

## Scope

**In:**
- **`shuyi_core::grid`** (`crates/core/src/grid.rs`), additions only.
  `Dir`, `Pos::step` and `neighbors4` don't change:
  - `Dir8`: `Right`, `DownRight`, `Down`, `DownLeft`, `Left`, `UpLeft`,
    `Up`, `UpRight`, with `Dir8::ALL` in that order (clockwise from
    `Right`, as `Dir::ALL` is), `delta()`, `is_diagonal()` and
    `From<Dir> for Dir8`. The same derives as `Dir`. It is a closed set,
    so it stays exhaustive (2013's rule);
  - `Pos::step8(self, dir: Dir8) -> Pos` and
    `Pos::chebyshev(a, b) -> u32`;
  - `Grid::neighbors8(&self, pos)`: the in-bounds neighbours in
    `Dir8::ALL` order, with no corner rule (it knows no costs). 2119
    counts a tile's neighbours with it.
- **`Topology`**, in a new file `crates/core/src/path/topology.rs`,
  re-exported from `path` and the crate root:
  - `enum Topology { Square4, Square8(Diagonals) }`, `Square4` the
    default. `Topology`, `Diagonals` and `Corners` each derive `Debug`,
    `Clone`, `Copy`, `PartialEq`, `Eq`, `Hash`, `Serialize` and
    `Deserialize`. `Topology` is `#[non_exhaustive]`, because 2121
    (parked) adds hex grids as a third;
  - `Diagonals { straight: u32, diagonal: u32, corners: Corners }`, with
    `Diagonals::new(straight, diagonal, corners)` and two constants:
    `OCTILE` (10, 14, `BothOpen`) and `KING` (1, 1, `BothOpen`). A
    weight of 0 counts as 1 wherever a weight is read, since a saved
    `Diagonals` can hold 0;
  - `Corners`: `BothOpen` (the `Default`: a diagonal step only when
    both straight tiles beside it are open, 2709's "no cutting
    corners"), `OneOpen` (at least one) and `Always`. A tile is open
    when the cost function answers `Some` for it. Four-way is
    `Topology::Square4`, so there is no `Never`;
  - `Topology::steps(self, from: Pos, open: impl Fn(Pos) -> bool) ->
    impl Iterator<Item = (Pos, u32)>`: each tile one step away that
    `open` answers `true` for, and the step's weight, in `Dir8::ALL`
    order. `Square4` gives only the straight steps, which is
    `Dir::ALL`'s order, each of weight 1. A diagonal also needs its two
    corner tiles open as its `Corners` rule says. Bounds are `open`'s
    job;
  - `Topology::step_weight(self, from, to, open) -> Option<u32>`: the
    weight of one step, or `None` when `to` isn't one step away, `open`
    answers `false` for it, or the corner rule forbids it, so it always
    agrees with `steps`. A game's rules check their own moves with it
    (a `Move(Dir8)` command, say);
  - `Topology::distance(self, a, b) -> u32`: the least total weight
    from `a` to `b` on an open map. `Square4` is Manhattan. `Square8` is
    `straight × (hi − lo) + min(diagonal, 2 × straight) × lo`, where
    `lo` and `hi` are the smaller and larger of `|dx|` and `|dy|`: that
    is Chebyshev with `KING` and octile with `OCTILE`. It saturates.
- **The searches take a topology** (`crates/core/src/path.rs`):
  - `reachable_on(topology, width, height, start, budget, cost) ->
    Reach` and `flow_field_on(topology, width, height, goals, cost) ->
    Grid<Option<u32>>`. `reachable` and `flow_field` call them with
    `Topology::Square4` and keep their signatures;
  - the private `search` takes the topology and walks `Topology::steps`
    instead of `Dir::ALL`, with `open` as "in bounds and `enter` answers
    `Some`". A step costs its weight times what it costs today
    (`leave(a) + enter(b)`), checked: a cost past `u32::MAX` is no step,
    as now. So a corner tile is open when `enter` answers `Some`, which
    is right for both callers (`reachable`'s `enter` is the cost
    function; `flow_field`'s is `cost(pos).map(|_| 0)`). A corner of two
    in-bounds tiles is always in bounds;
  - with `Square4` every weight is 1, so every result is today's: every
    test in `crates/core/src/path/tests.rs` passes unchanged;
  - `Reach::path_to` works for every topology (it follows `prev`).
- **Point-to-point paths**, in a new file `crates/core/src/path/find.rs`,
  re-exported from `path` and the crate root:
  - `find_path(width, height, from, to, options: PathOptions, cost) ->
    Option<TilePath>`;
  - `PathOptions { topology, least_cost: u32, partial: bool }`, whose
    `Default` is four-way, a least cost of 1 and no partial path, with
    `with_topology`, `with_least_cost` and `with_partial` each returning
    a changed copy. `cost` is the cost function of `reachable`;
  - `TilePath { tiles: Vec<Pos>, cost: u32, reached: bool }`: the
    tiles from `from` to the last one, both included, and the cost as
    `reachable_on` counts it;
  - A* with the heuristic `least_cost × topology.distance(tile, to)`.
    `least_cost` is the least any tile costs to enter (1 for most
    games). With 0 the search is a Dijkstra that stops at the goal. A
    game whose tiles cost less than its `least_cost` still gets a valid
    path, but maybe not the cheapest; the docs say so;
  - deterministic: the queue is ordered by `(f, h, y, x)`, smallest
    first, where `f = g + h` and both `h` and `f` saturate; a tile's
    predecessor changes only for a strictly cheaper `g`; each tile is
    expanded once. With `least_cost` at most every tile's cost the
    heuristic is consistent, so an expanded tile's cost is final. These
    rules go in the module docs beside the Dijkstra's. With `h = 0` and
    no goal, `(f, h, y, x)` is the Dijkstra's `(cost, y, x)`, so
    `find_path` may share `search`'s loop rather than copy it, as long
    as every result above holds;
  - it stops when `to` is expanded;
  - `None` when `from` is off the grid, or when `to` can't be reached
    and `partial` is off;
  - **a partial path:** with `partial` on and `to` out of reach (or off
    the grid), the path ends on the expanded tile nearest `to` by
    `topology.distance`, then the cheapest, then the smallest `(y, x)`,
    with `reached: false`. That may be `from` alone.
- **Docs:**
  - the module docs of `path.rs`: its *Rules* cover steps per topology,
    weights, the corner rule, A*'s order and partial paths, with a doc
    test of an eight-way `find_path` round a wall;
  - `crates/core/src/lib.rs`: its doc comment, and re-exports of
    `Dir8`, `Topology`, `Diagonals`, `Corners`, `PathOptions`,
    `TilePath`, `find_path`, `reachable_on` and `flow_field_on`;
  - `shuyi-core`'s row in `README.md` ("grid (four- or eight-way),
    paths, turns");
  - the docs of `MapScene::path` (`crates/ui/src/grid/scene.rs`) and
    `path_overlays` say they draw four-way steps. Docs only; drawing a
    diagonal is the follow-up under *Out*.
- **Public types (ADR-0037):** `Topology`, `Diagonals`, `Corners`,
  `PathOptions` and `TilePath` are `#[non_exhaustive]` (fields stay
  public to read), with the constructors, constants, `Default` and
  `with_*` methods above. `Dir8` stays exhaustive, a closed set.
- **A pinned path:** a unit test pins one eight-way `find_path` on a
  fixed map (its tiles and cost). CI's `test` job runs it on x64 and
  ARM64. If 2116 is done, its `determinism` job also runs `shuyi-core`'s
  tests on wasm32. If not, the native runs alone.
- **Either order with 2004** (guides): if 2004 is done, add a paragraph
  to `docs/guides/top-down-grid.md`: four-way by default, eight-way with
  `Topology::Square8`, and `find_path`. If not, 2004 writes it.
- **What changes for the game:** nothing. New items only. Say so in
  the PR.

**Out (do not do):**
- drawing a diagonal step in the path arrow, steering the cursor in
  eight directions (a diagonal must be an action, and a Playdate's
  d-pad has four: the `input-actions` skill), and an eight-way choice
  in `templates/grid` or the wizard (2501–2505). The session that
  completes this ticket writes one follow-up ticket for them (the
  `write-ticket` skill), in the 23xx block;
- jump-point search, inexact (weighted) heuristics, Euclidean distance,
  a cap on the tiles searched, and cached paths: each one when a
  measurement on a real map asks for it;
- hex grids (2121 and 2740, parked), field of view (2120, parked),
  level generators (2119, parked) and the roguelike example (2735,
  parked);
- paths for free movers (2106), eight-way free movers in `Fx` (1122)
  and routes over portals (2709);
- searches over game states (2112, 2113);
- changing `Dir`, `reachable`, `flow_field` or `neighbors4`.

## Implementation steps

1. Read `crates/core/src/grid.rs`, `crates/core/src/path.rs` and
   `crates/core/src/path/tests.rs` (its `Costs` maps and brute-force
   references), and ADR-0037.
2. Add `Dir8`, `Pos::step8`, `Pos::chebyshev` and `Grid::neighbors8`,
   with tests.
3. Write `Topology`, `Diagonals` and `Corners` with `steps`,
   `step_weight` and `distance`.
4. Make `search` take a topology, and add `reachable_on` and
   `flow_field_on`. Run the existing tests unchanged.
5. In `path/tests.rs`, add `brute_reach_on` and `brute_flow_on`: the
   brute-force references relaxed over `Topology::steps`. Leave
   `brute_reach`, `brute_flow` and the existing tests as they are, so
   they still run unchanged. Add the property tests below.
6. Write `find_path` and its tests.
7. Write the docs and re-exports, do the either-order items, and run
   the gates.

**Size:** about 600 to 750 changed lines with tests. If the diff runs
long, step 6 (`find_path`, `PathOptions`, `TilePath`, `find.rs` and
their tests) moves to a follow-up ticket of the same milestone (the
`write-ticket` skill), added to 2121's `blocked_by` (its hex paths use
`find_path`), and the completion notes say so.

## Acceptance criteria

- [ ] Every existing test in `crates/core` passes unchanged, and
      `reachable` and `flow_field` are `reachable_on` and
      `flow_field_on` with `Topology::Square4`.
- [ ] The costs of `reachable_on` and `flow_field_on` equal
      `brute_reach_on` and `brute_flow_on` for `Square4`, and for
      `Square8` with each `Corners` rule and both `OCTILE` and `KING`
      (property test).
- [ ] With `Corners::BothOpen`, no path cuts a corner: both straight
      tiles beside every diagonal step of a `path_to` or `find_path`
      result are open (property test).
- [ ] `find_path`'s cost equals `reachable_on(…, u32::MAX, …).cost(to)`
      whenever `least_cost` is at most every tile's cost, in both
      topologies, and its tiles are steps of the topology whose costs
      add up to its `cost` (property test).
- [ ] With `partial` on and the goal out of reach, the path ends on a
      reachable tile with `reached: false`, and no reachable tile is
      nearer the goal by `topology.distance`. With the goal in reach,
      `partial` changes nothing and `reached` is `true` (property test
      against `reachable_on`).
- [ ] The ties of a hand-made open room come out in the documented
      order, and the same call gives the same path twice.
- [ ] On an open 100 × 100 map, in both topologies, `find_path` from
      corner to corner asks the cost function about fewer distinct
      tiles than `reachable_on` does (a cost function that records the
      tiles it is asked about).
- [ ] `step_weight` agrees with `steps` for every pair of tiles within
      two steps of each other, and `distance` is never more than the
      cheapest path's cost when every tile costs 1 (property tests).
- [ ] `shuyi-core` still builds for `thumbv7em-none-eabihf`, and has no
      float in the new code (2015's float ban passes).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `Dir8`'s deltas and order; `neighbors8` at a corner and in the
  middle; each `Corners` rule on a diagonal between two walls, one wall
  and none; `distance` for `Square4`, `KING`, `OCTILE` and a diagonal
  weight over twice the straight one; `find_path` from a tile to
  itself, from off the grid, to off the grid with and without
  `partial`, and with `least_cost` 0; the pinned path.
- Property: the criteria above, over the random `Costs` maps of
  `path/tests.rs`.
- Doc test: the eight-way `find_path` in the module docs.

## Completion notes

*(Filled in by the session that completes the ticket.)*
