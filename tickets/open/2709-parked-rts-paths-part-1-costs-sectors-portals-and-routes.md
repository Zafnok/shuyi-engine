---
id: "2709"
title: "Parked: RTS paths, part 1: costs per movement class, sectors and portals, and routes over the portal graph"
type: feature
milestone: "Engine: real-time strategy"
model: reasoning
effort: xhigh
status: blocked
blocked_by: ["2707", "2116"]
nick_input: none
completed:
---

# 2709 — RTS paths, part 1: costs, sectors, portals and routes

> **Parked** (Nick, 2026-10-18: "make tickets parked for after visions of shuyi launches for all of these"): after Visions of Shuyi launches, which is later than the MVP in `docs/ROADMAP.md`. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

[ADR-0015](../../docs/adr/0015-real-time-strategy-at-scale-command-sims-columns-flow-fields-and-ai-in-layers.md)
§5. A* per unit costs O(units × map) when an army moves. The ADR splits
pathfinding in two:

- **this ticket:** the costs, the map cut into sectors with portals
  between them (HPA*, Botea, Müller and Schaeffer, 2004), and a route as
  an A* search over the portal graph;
- **2710:** flow fields built only over the sectors on a route, shared
  by every unit going to one goal.

It builds on 2707's units, kinds (`movement`, `footprint`) and
`RtsSim`. Units keep moving in straight lines until 2710.

## Nick input

None.

## Scope

**In:**
- **Terrain, `shuyi_rts::paths::TerrainGrid`:**
  - a width × height grid of `TerrainClass(u8)`, with a cell size in
    `Fx`. It becomes the source of the map's size: if 2708 is done,
    remove its `RtsSettings::map_size` and read the terrain grid's size;
    if not, 2708 reads the terrain grid's size when it lands;
  - built from code: `TerrainGrid::from_fn(w, h, cell, f)`;
  - **either order with 1124** (Tiled and LDtk maps): if 1124 is done,
    also `TerrainGrid::from_layer(&map, layer, property)`, which reads a
    tile layer whose tiles carry an integer custom property (for
    example `terrain`; 1124's tilesets keep their tiles' properties),
    with a fixture map. If 1124 isn't done, code only: 1124 adds
    `from_layer` (its own clause says so).
- **Movement classes:**
  - `MovementClass` from `*.movement.ron`: a name, a cost per terrain
    class (`Cost(1..=254)` or `Wall`), and `blocked_by_buildings`
    (true for ground, false for air);
  - the format (ADR-0007): `movement-class`, extension `.movement.ron`,
    registered by `shuyi_rts::register`, with a fuzz-style test and a
    determinism fixture (each fixture's decoded hash as a row of
    `shuyi-rts`'s golden table, 2116's `shuyi_assets::golden`, which
    2707 runs on wasm32 in CI's `determinism` job);
  - `UnitKinds` resolves each kind's `movement` name, with an error
    naming the file and the kind when it is unknown.
- **Costs, `CostGrid`**, one per movement class: a `u8` per cell, 255 a
  wall. Buildings (kinds with a `footprint`) mark their cells as walls
  for classes with `blocked_by_buildings`. `RtsSim` marks them when such
  a unit spawns and clears them when it is removed. Every change bumps a
  `map_version: u64`.
- **Moves between cells:** 8 directions, straight cost 10 and diagonal
  14, times the terrain cost. A diagonal is allowed only when both
  straight neighbours are open (no cutting corners). **Either order
  with 2117:** if 2117 is done, take the eight directions, the step
  costs and the corner rule from `shuyi_core` (`Dir8`,
  `Topology::Square8(Diagonals::OCTILE)`, `Topology::steps` and
  `step_weight`), and use `Topology::distance` as `route`'s octile
  heuristic, rather than writing them again in `shuyi_rts::paths`. If
  not, write them here, and 2117 moves them onto `shuyi_core` when it
  lands (its clause says so).
- **Sectors and portals:**
  - sectors of 16 × 16 cells by default (a setting);
  - along each border between two sectors, each longest run of cells
    open on both sides is one portal. It gives two nodes, one on each
    side, at the run's middle cell, joined by the cost of that step;
  - inside a sector, every pair of its nodes is joined by the cost of
    the cheapest path between them inside the sector (a Dijkstra per
    node), or not at all;
  - node ids are given in a fixed order (sector, border, run start), so
    the graph is the same on every machine;
  - one `PortalGraph` per movement class.
- **Changes rebuild locally:** `PathWorld::cells_changed(rect)` rebuilds
  the portals on the borders of every sector the rect touches, and the
  inside joins of those sectors and their neighbours. Nothing else.
- **A route:** `route(class, from: CellPos, to: CellPos) ->
  Option<Route>`:
  - joins `from` and `to` to their sectors' nodes by a search inside the
    sector (and directly when both are in one sector);
  - A* over the portal graph, with the octile distance as the heuristic,
    in integers, ties broken by node id;
  - `Route { sectors: Vec<SectorId>, waypoints: Vec<CellPos>, cost }`.
    2710 reads `sectors`.
- **A text dump** of a sector's portals and joins, for tests and agents.

**Out (do not do):**
- flow fields, the request queue, the cache, and moving units along a
  route (2710);
- steering and pushing (2711);
- reading maps if 1124 isn't done (see above);
- a navigation mesh (ADR-0015, *Alternatives*); 2106 builds one for a
  few free movers, not for armies;
- what places and removes buildings in play (2733, the RTS economy; see
  2707's *Out*). Here, spawns and removals of footprint kinds are
  enough.

## Implementation steps

1. Write `TerrainGrid`, `MovementClass` and its format, and `CostGrid`
   with building marks and `map_version`.
2. Write the sector cut, the portal finder and the inside joins.
3. Write `route` with A* and its tie-breaks.
4. Write `cells_changed` and the comparison with a full rebuild.
5. Write `from_layer` if 1124 is done.
6. Write the cell-grid BFS reference in the tests.

## Acceptance criteria

- [ ] On random maps, `route` returns a route exactly when a BFS over
      the open cells finds a path (property test, 2 000 maps).
- [ ] Every waypoint of a route is an open cell, and consecutive
      waypoints are joined inside one sector (property test).
- [ ] After random buildings are added and removed, the graph from
      `cells_changed` equals a graph built from scratch (property test).
- [ ] Two runs over the same map give the same graph dump and the same
      routes.
- [ ] Each movement-class fixture decodes to its row in a golden table
      (2116's `shuyi_assets::golden`), checked on x64, ARM64 and wasm32
      by CI's `test` and `determinism` jobs; bad fixtures name the file;
      arbitrary bytes never panic.
- [ ] If 1124 is done, `TerrainGrid::from_layer` reads the fixture map.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: portals on hand-made sectors (an open border, a border with a
  wall in the middle, a closed border); corner cutting; tie-breaks.
- Property: routes against BFS; local rebuild against full rebuild.
- Fuzz-style: arbitrary bytes into the `movement-class` format.
- Fixture and determinism: the movement-class fixtures' golden rows, on
  x64, ARM64 and wasm32 (CI's `test` and `determinism` jobs).

## Completion notes
