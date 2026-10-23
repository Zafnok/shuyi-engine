---
id: "2119"
title: "Parked: Seeded level generators over a grid: cellular caves, BSP rooms, a drunkard's walk, connectivity repair, and weighted tables as data"
type: feature
milestone: "Engine: roguelikes"
model: sonnet-5
effort: high
status: blocked
blocked_by: ["2118", "2114", "2116"]
nick_input: none
completed:
---

# 2119 — Seeded level generators and weighted tables

> **Parked** (Nick, 2026-10-20, choosing roguelikes as a kind of game the engine serves; Visions of Shuyi is "the first game I want to be launched with the engine"): like real-time strategy, visual novels and detective games, this waits until after Visions of Shuyi launches. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8:

- Unreal's PCG framework makes content from rules and a seed
  (https://dev.epicgames.com/documentation/en-us/unreal-engine/procedural-content-generation-overview);
- Godot and Unity ship no level generators in their core. They draw a
  generated map with terrain sets (Godot's `set_cells_terrain_connect`)
  and Rule Tiles (Unity's 2D Tilemap Extras);
- Bevy ships none;
- roguelike libraries carry them: rot.js has cellular caves
  (`ROT.Map.Cellular`) and rooms with corridors (`ROT.Map.Digger`),
  libtcod has a binary space partition toolkit, and the Rust roguelike
  tutorial on bracket-lib writes BSP rooms, cellular caves and a
  drunkard's walk. The algorithms are textbook ones. Nothing is copied:
  the code follows this ticket's rules.

What Shuyi has today:

- `Grid<T>`, `Pos`, `Dir::ALL` and `neighbors4`, four directions only,
  and `TileSet`, a bitset of a map's tiles (`crates/core/src/grid.rs`);
- `reachable` and `flow_field`, one Dijkstra over four directions
  (`crates/core/src/path.rs`);
- `RandomSource` with `roll_percent` and `roll_below`, `SimRng`, and
  `ScriptedRng` for tests (`new` scripts `roll_percent`, `with_below`
  scripts `roll_below`) in `crates/core/src/rng.rs`;
- `state_hash`, whose `StateHash` prints as 16 hex digits
  (`crates/core/src/hash.rs`);
- the `.map` format reads a `GridMap<T, F>` whose `tiles` are a
  `Grid<T>` (`crates/assets/src/grid_map.rs`);
- the grid kit draws any terrain grid. The corner rule
  (`Shown::of` in `crates/ui/src/grid/corners.rs`) picks pictures from
  the terrain around each tile every frame, and
  `crates/assets/src/tileset.rs` says "a terrain grid alone says what
  the map looks like". So a generated `Grid<TerrainId>` draws right with
  no extra step.

Planned:

- 2118 adds `shuyi-procgen`, noise, `Grid::map`, the `shuyi-procgen`
  skill (or its line in 1134's list), and the ADR on what a seed
  promises;
- 2114 adds RNG helpers on `RandomSource` (ranges, picks, shuffles,
  bags, and `weighted_index` over `&[u32]` and `pick_weighted` over
  `(u32, T)`), and named streams and forks of `SimRng`. Weighted tables
  as data, checked at load, are here; a game's loot design beyond them
  is the game's;
- 2116 adds golden tables (`shuyi_assets::golden`, the `golden!` macro,
  `cargo xtask golden --bless`) and the CI job `determinism`, which runs
  them on wasm32;
- 2117 adds eight-way moves and point-to-point A* to the grid kit,
  with `Grid::neighbors8`;
- 2120 adds field of view; 2735 is the example and template.

Why it matters:

- **The game first:** Visions of Shuyi needs no generated levels, which
  is why this waits for its launch. Nothing here changes for the game:
  it doesn't use `shuyi-procgen`.
- **Other 2D games:** Nick added roguelikes on 2026-10-20
  (`docs/decisions.md`). Each generator here is under a hundred lines,
  but a roguelike needs several, and each needs the same promises: pure,
  seeded, integers only, no panic at any size, the same on every device,
  and stable for a seed (2118's ADR). Tested once in the engine, they
  spare every game that wants them from writing and testing its own.
- **The end goal** (2026-10-19): an agent asked for a roguelike calls
  tested generators instead of writing them.

The comparison's reviewers cut two parts of the first proposal:

- **wave function collapse:** none of Bevy, Godot or Unity ships it in
  its core, and no game asks for it;
- **terrain rules at run time:** the corner rule already follows changed
  terrain (it reads the terrain and `TileView::becomes` each frame).
  Keeping the chosen pictures in map data would move a look into logic
  (`AGENTS.md` rule 9).

## Nick input

None.

## Scope

**In:**
- **`shuyi_procgen::carve`** (`crates/procgen/src/carve.rs`, with
  `carve/rooms.rs` if it passes about 300 lines), pure, integers only:
  - every generator takes `width: u16, height: u16` (as `reachable`
    does), its rules, and `rng: &mut impl RandomSource`;
  - it returns a `Grid<Cell>`, with `Cell::{Solid, Open}` (`Copy`,
    `Eq`, serde; a closed set, so exhaustive). A game turns cells into
    its terrain with `Grid::map` (2118);
  - the inner tiles are every tile but the outer ring. Every generator
    leaves the outer ring `Solid`;
  - a map with no inner tile (width or height under 3), or with no
    room that fits, gives an all-`Solid` grid, never a panic;
  - every loop has a bound from the rules or the map's size;
  - one private helper digs an L-shaped corridor between two tiles,
    the horizontal leg first or not. `rooms` and `repair` both use it.
    It stays inside the two tiles' bounding box, so it never opens the
    outer ring when both ends are inner.
- **Cellular caves**, `caves(width, height, &CaveRules, rng) ->
  Grid<Cell>`:
  - each inner tile, row-major, takes one `roll_percent()` and starts
    `Solid` when the roll is below `fill_percent`;
  - then `steps` rounds, each computed from the round before, over the
    inner tiles only. A tile counts how many of its eight neighbours are
    `Solid` (a tile off the map counts as `Solid`):
    - an `Open` tile turns `Solid` at `solid_at` or more;
    - a `Solid` tile stays `Solid` at `stay_solid_at` or more, and opens
      otherwise;
  - the defaults: `CaveRules { fill_percent: 45, steps: 4, solid_at: 5,
    stay_solid_at: 4 }`;
  - the eight neighbours: if 2117 is done, `Grid::neighbors8` (in-bounds
    only, so the neighbours it leaves out are off the map and count as
    `Solid`). If not, a private table of the eight offsets in
    `carve.rs`.
- **Rooms by binary space partition**, `rooms(width, height, &RoomRules,
  rng) -> Rooms`:
  - one depth-first pass, from the inner rectangle as the first leaf.
    Its rolls come in this order: a split's place, then its first part
    (left or top) in full, then its second part, then the corridor that
    joins them;
  - a leaf wider than tall takes a vertical cut, one taller than wide a
    horizontal cut, and a square a vertical cut. It is split when it is
    fewer than `max_depth` splits deep and the side being cut is at
    least `2 * min_leaf`. The cut leaves `min_leaf +
    roll_below(side - 2 * min_leaf + 1)` tiles in the first part, so
    both parts are at least `min_leaf` across;
  - each leaf that isn't split gets one room, with its rolls in this
    order: width `min_room + roll_below(max_room - min_room + 1)`, cut
    to the leaf's width less 2; height the same way; then `x` and `y`,
    each rolled so the room keeps one tile of margin inside the leaf on
    every side. A leaf less than `min_room + 2` across on either side
    gets no room and no rolls. So rooms never overlap or touch;
  - each split joins its two parts when both have rooms: of the pairs
    (a room of the first part, a room of the second), the one whose
    centres are nearest by Manhattan distance (ties: the first, by the
    order rooms were made, first part's room before second's) is joined
    by an L-shaped corridor between the two centres, the horizontal leg
    first when `roll_below(2) == 0`;
  - `Rooms { cells: Grid<Cell>, rooms: Vec<Room> }`, with
    `Room { origin: Pos, width: u16, height: u16 }` in the order rooms
    were made, and `Room::center()`, `origin + (width / 2, height / 2)`
    rounded down. The game places its stairs, foes and things by them.
    Both are serde, so `state_hash` takes them;
  - the defaults: `RoomRules { min_leaf: 8, min_room: 4, max_room: 10,
    max_depth: 5 }`.
- **A drunkard's walk**, `walk(width, height, &WalkRules, rng) ->
  Grid<Cell>`:
  - one walker starts at `start`, or at `(width / 2, height / 2)` when
    it is `None`, and opens its tile. A `start` outside the inner tiles
    is moved to the nearest inner tile (each coordinate clamped);
  - each step rolls one of `Dir::ALL` with `roll_below(4)`. The walker
    moves only when the next tile is an inner tile, and opens it. A step
    that doesn't move still counts;
  - it stops when `open * 100 >= inner * open_percent` (`open`: the open
    tiles, `inner`: the inner tiles), or after `max_steps` steps;
  - the defaults: `WalkRules { open_percent: 40, start: None,
    max_steps: 100_000 }`.
- **Settings as data** (ADR-0009 §4): `CaveRules`, `RoomRules` and
  `WalkRules` are serde, with `deny_unknown_fields`, `Default` and
  `#[serde(default)]` from the defaults above, so a game keeps them in
  its own data file:
  - the fields: `fill_percent`, `solid_at`, `stay_solid_at`, `steps`
    and `open_percent` are `u8`; `min_leaf`, `min_room` and `max_room`
    are `u16`; `max_depth` is `u8`; `start` is `Option<Pos>`;
    `max_steps` is `u32`;
  - reading checks a raw form through `#[serde(try_from = …)]`, as
    `Grid` does in `crates/core/src/grid.rs`. It refuses a percent over
    100, a neighbour count over 8, a `max_depth` over 16, a `min_room`
    of 0, `min_room > max_room`, and `min_leaf < min_room + 2`, each
    with a message naming the field;
  - code that builds rules in Rust gets the same checks from
    `check(&self) -> Result<(), RulesError>`, where `RulesError` names
    the field and the rule it broke. Each generator calls `check` first,
    and rules that fail it give an all-`Solid` grid (and no rooms), with
    no rolls.
- **`shuyi_procgen::regions`** (`crates/procgen/src/regions.rs`):
  - `regions(&Grid<Cell>) -> Vec<TileSet>`: the four-connected `Open`
    regions (through `neighbors4`), in the order of each region's first
    tile, row-major;
  - `repair(&mut Grid<Cell>, Repair)`, so every open tile reaches every
    other:
    - `Repair::KeepLargest` fills every region but the largest with
      `Solid` (ties: the first);
    - `Repair::Connect` digs, from each other region in order, an
      L-shaped corridor (the horizontal leg first) from its first tile
      to the tile of the largest region nearest that first tile by
      Manhattan distance (ties: the first, row-major);
    - `Repair` is `Copy` and serde;
  - after either, `regions(..)` has at most one region.
- **`shuyi_procgen::table`: weighted tables as data**
  (`crates/procgen/src/table.rs`). 2114 gives every `RandomSource`
  `weighted_index` over `&[u32]` and `pick_weighted` over `&[(u32, T)]`,
  which return `None` on a bad slice. This adds the table a game reads
  from a file and checks once, at load:
  - `WeightedTable<T>` holds its entries `(u32, T)`, weight first, as
    2114's `pick_weighted` takes them, plus a copy of the weights and
    their total, all private. `new(entries)` refuses an empty list, a
    weight of 0, and a total over `u32::MAX` with `TableError`
    (`Empty`, `ZeroWeight { index }`, `TooHeavy`), whose `Display`
    names the problem;
  - `pick_index(&self, rng: &mut impl RandomSource) -> usize` calls
    2114's `weighted_index` over the weights: one `roll_below(total)`,
    and the same index `pick_weighted` would give. It is never `None`
    for a table `new` accepted; `unwrap_or(0)` covers it, because the
    workspace warns on `unwrap` and `expect` and clippy runs with
    `-D warnings`;
  - `pick(&self, rng) -> &T` is the entry at `pick_index`. `entries()`
    and `total()` read the table;
  - serde as a list of pairs: `Deserialize` through
    `#[serde(try_from = "Vec<(u32, T)>")]`, so RON reads
    `[(3, Potion), (1, Scroll)]` and reading checks what `new` checks,
    and a short hand-written `Serialize` that writes the entries alone;
  - a game keeps its tables inside its own data table and reads that
    with `shuyi_assets::ron::Ron<T>`, as ADR-0019 §7 says a game's data
    tables are read. No new file format. A test in
    `crates/procgen/tests/table.rs` reads such a table with
    `shuyi_assets::parse_ron` (what `Ron<T>` calls), and shows that a
    bad table's error names its file and line. `shuyi-assets` is a
    dev-dependency only.
- **Golden hashes**, `crates/procgen/tests/golden/carve.ron`, through
  2116's `shuyi_assets::golden` (`golden!`, the `golden` feature on the
  dev-dependency), beside 2118's noise table:
  - rows `caves seed 1` to `caves seed 4`, `rooms seed 1` to
    `rooms seed 4` (the whole `Rooms`) and `walk seed 1` to
    `walk seed 4`: the `state_hash` of the output for `SimRng::new(n)`
    on a 48 × 32 map with the default rules;
  - a row `table picks seed 1`: the counts of 10 000 `pick_index`
    draws from `SimRng::new(1)` over the weights `1, 2, 3, 4`, written
    as four numbers;
  - the test's name has `golden` in it, and the table is written with
    `cargo xtask golden --bless`;
  - `shuyi-procgen` must be in the `determinism` job's wasm32 run
    (`.github/workflows/ci.yml`), as 2118's noise table needs too. If it
    is missing, add `-p shuyi-procgen` to that run;
  - the rows change only under 2118's ADR's rule for seeds: a PR that
    moves one carries a `Breaking:` line.
- **Public types (ADR-0037):** `CaveRules`, `RoomRules`, `WalkRules`,
  `Repair`, `RulesError` and `TableError` are `#[non_exhaustive]`,
  `Cell` and `Room` stay exhaustive (plain values), and `WeightedTable`
  keeps its fields private.
- **Docs:**
  - the crate's `lib.rs` doc and re-exports name the generators,
    `regions`, `repair` and `WeightedTable`, with a doc test of a cave
    from a seed, repaired and mapped to a game's terrain with
    `Grid::map`;
  - the README's "what it is for" section (2118 wrote it) names them;
  - if the `shuyi-procgen` skill exists (2118 writes it when 1134 is
    done), it lists them. If not, nothing: 1134's list already names the
    skill.

**Out (do not do):**
- wave function collapse: a ticket when a game asks;
- terrain rules at run time: the grid kit's corner rule draws from
  terrain already. For 1124's tile layers (Tiled terrain sets, LDtk
  auto-layer rules), a follow-up beside 1124 if a game asks;
- eight-way moves and point-to-point A* (2117);
- RNG conveniences: ranges, picks, shuffles, bags, weighted picks over a
  slice, named streams (2114);
- nested tables, "nothing" entries and guaranteed drops: a game writes
  them with its own `T` (an enum of a thing or another table's name;
  `Option<Thing>` for nothing). A table asset kind of its own: none;
  the game's data table holds the tables;
- prefabs (hand-made rooms stamped into a generated map): a follow-up if
  a game asks;
- generators on hex grids (2121 adds hex grids, parked): a follow-up if
  a hex game asks;
- placing stairs, foes and things, and which generator each floor uses:
  the game's (2735 shows it);
- checking a generated level by search (2112);
- noise (2118), field of view (2120), and the example and template
  (2735).

## Implementation steps

1. Read 2118's ADR, crate and README, 2114's helpers and ADR, 2116's
   `shuyi_assets::golden` and ADR-0037. Check whether
   2117 is done.
2. Write `Cell`, the three rules types with their raw forms and checks,
   and the corridor helper, with unit tests.
3. Write `caves`, then `rooms`, then `walk`, each with its unit and
   property tests, in `crates/procgen/src/carve/tests.rs` (as
   `crates/core/src/path/tests.rs` sits beside `path.rs`).
4. Write `regions` and `repair`, with their tests.
5. Write `WeightedTable`, its tests and `crates/procgen/tests/table.rs`.
6. Write the golden test, bless `carve.ron` with `cargo xtask golden
   --bless`, and check the `determinism` job runs it.
7. Write the docs and the doc test. Run the gates.

If the diff passes about 700 changed lines without the golden table,
leave `WeightedTable` out with its tests and its golden row. Write a
parked follow-up ticket for it (the `write-ticket` skill, blocked by
this one), add it to 2735's `blocked_by`, fix 2114's and 2735's lines
that name 2119 for it, and say so in the completion notes.

## Acceptance criteria

- [ ] For random sizes from 0 × 0 to 64 × 64, random rules (passing
      `check` or not) and random seeds, no generator panics, and each
      leaves the outer ring `Solid` (property test).
- [ ] The same seed gives the same grid, and the golden table matches
      natively and in the `determinism` job on wasm32.
- [ ] `rooms`: no two rooms overlap or touch (their rectangles grown by
      one tile don't overlap), and the open tiles form at most one
      region (property test).
- [ ] `walk`: the open tiles form at most one region, and
      `open * 100 >= inner * open_percent` unless `max_steps` ran out
      (property test).
- [ ] `repair` leaves at most one region in either mode, and
      `KeepLargest` leaves exactly the largest region's tiles open
      (property test).
- [ ] Each rules type reads from RON with the defaults for left-out
      fields, and each refused value gives an error naming its field.
- [ ] `WeightedTable` refuses each of its three bad inputs, from code
      and from RON, and for every draw in `0..total` its `pick` gives
      what 2114's `pick_weighted` gives (a `ScriptedRng` per draw).
- [ ] `grep -rnE "f32|f64" crates/procgen/src` finds nothing (2015's
      float ban covers it if it is done), and `cargo build -p
      shuyi-procgen --no-default-features --target
      thumbv7em-none-eabihf` passes.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit:
  - caves with `steps: 0` against a `ScriptedRng`, and one round of the
    private step function on a drawn 5 × 5 grid;
  - one binary split traced with a `ScriptedRng`, room rolls and
    corridor roll included;
  - a walk boxed in by a 5 × 5 map, and a `start` on the outer ring;
  - `regions` and `repair` on a drawn map with two islands, in both
    modes;
  - table picks at the edges of each weight;
  - each rules type from RON, and its errors.
- Property: the criteria above; caves with `steps: 0` on 64 × 64 leave
  the inner tiles open within ten points of `100 - fill_percent`.
- Integration: `crates/procgen/tests/table.rs`, a game's data table
  with two weighted tables, read with `parse_ron`, and a bad one.
- Golden: `carve.ron`, natively and in the `determinism` job.
- Doc test: a cave from a seed, repaired and mapped to terrain.

## Completion notes

*(Filled in by the session that completes the ticket.)*
