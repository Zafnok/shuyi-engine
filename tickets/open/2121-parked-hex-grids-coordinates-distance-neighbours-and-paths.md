---
id: "2121"
title: "Parked: Hex grids in the grid kit: axial coordinates, offset storage, distance, neighbours, rings and paths, and an ADR amending ADR-0003 §6"
type: feature
milestone: "Engine: hex grids"
model: reasoning
effort: high
status: blocked
blocked_by: ["2013", "2015", "2117"]
nick_input: none
completed:
---

# 2121 — Hex grids: coordinates, distance, neighbours and paths

> **Parked** (Nick, 2026-10-20, choosing hex grids as a kind of game the engine serves; Visions of Shuyi is "the first game I want to be launched with the engine"): like real-time strategy, visual novels and detective games, this waits until after Visions of Shuyi launches. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8. Each of them has hex maps:

- Godot's TileSet has a hexagon tile shape beside square, isometric and
  half-offset square
  (https://docs.godotengine.org/en/stable/tutorials/2d/using_tilesets.html);
- Unity's Tilemap has hexagonal grids, point top and flat top;
- Unreal's Paper2D has a `HexagonalStaggered` projection;
- on Bevy, `bevy_ecs_tilemap` draws hexagonal maps, and the `hexx`
  crate gives hex coordinates.

What Shuyi has today: no hex in its code or ADRs. The only tickets that
name hex are 2117, which leaves room for it in `Topology`, 2360's
*Out*, which points here and at 2740, and 2740 itself. The grid kit is
square:

- `crates/core/src/grid.rs`: `Pos`, `Grid<T>` (a `width × height`
  rectangle of cells, row-major), `TileSet` and the four-way `Dir`;
- `crates/core/src/path.rs`: `reachable` and `flow_field`. 2117 (before
  this) adds `Topology` (`Square4`, `Square8`), `reachable_on`,
  `flow_field_on` and `find_path`, and marks `Topology`
  `#[non_exhaustive]` for this ticket's variant;
- ADR-0003 §6 says the grid kit "works in tile coordinates and so works
  unchanged under any projection". That holds for top-down, isometric
  and side view, which are square grids drawn three ways. It doesn't
  hold for hex: a hex tile has six neighbours. So this ticket's ADR
  amends §6.

Why it matters:

- **The game first:** Visions of Shuyi uses square tiles and needs none
  of this. Nick parked hex grids until after its launch (2026-10-20,
  `docs/decisions.md`).
- **Other 2D games:** wargames, hex strategy games, board games on a
  hex board, hex puzzle games. Hex maps are common in turn games after
  tactics and roguelikes, and the grid kit is the engine's strongest
  part.
- It goes in `shuyi-core` beside `path.rs`, not in a module, as 2120's
  field of view does: it is pure integer maths over the same `Grid` and
  `TileSet`, it is small, and every search of the kit works over it
  through `Topology`. `Topology::Hex` has to be a variant of the core's
  own enum, which a module crate can't add. Drawing hex maps is 2740.

It builds on three tickets, all done first:

- 2117 (after the MVP): `Topology`, `reachable_on`, `flow_field_on`,
  `find_path` and `Grid::neighbors8`;
- 2013 (MVP): the ADR on public types that can grow. Under it `Hex`,
  `HexDir` and `HexLayout` are plain values and `Orientation` and
  `Offset` are closed sets, so all five stay exhaustive;
- 2015 (MVP): the float ban in `shuyi-core`.

The maths follows Red Blob Games' guide to hex grids
(https://www.redblobgames.com/grids/hexagons/): axial coordinates for
the maths, offset coordinates for storage. Write ours in Rust from the
guide's description. `hexx` (MIT OR Apache-2.0) was considered. Its
coordinates are integers, but its layouts and much of its API are in
`f32` through `glam`, and the core takes no floats (2015). What this
ticket needs is a few hundred lines.

## Nick input

None. Nick chose hex grids as a kind of game, parked until after the
launch, on 2026-10-20.

## Scope

**In:**
- **An ADR** (the `write-adr` skill, the next free number): hex grids
  in the grid kit.
  - A hex map is stored in the same `Grid<T>` and `TileSet`. Its `Pos`
    is an offset coordinate (column, row); the maths is in axial
    coordinates.
  - `Topology::Hex` is the kit's third topology, so `reachable_on`,
    `flow_field_on` and `find_path` work on it unchanged.
  - It amends ADR-0003 §6: the grid kit works unchanged under any
    projection of a square grid. A hex map is another topology, drawn by
    a projection of its own (2740).
  - It adds `hex grid` to the Perspective answers of ADR-0009 §2,
    parked (2121, 2740), served by the default kits rather than a
    module, as ADR-0011 and ADR-0015 added theirs.
  - The cursor's four moves stay valid: in every offset layout the
    steps `(±1, 0)` and `(0, ±1)` are neighbours of every tile, so
    `Cursor::step` (`crates/ui/src/grid/cursor.rs`) and `steer`
    (`crates/ui/src/grid/steer.rs`) need no change and every tile can
    be reached with a d-pad.
  - ADR-0003's `Status:` line gets a note that §6's sentence on the grid
    kit is amended by it, as the line already notes ADR-0011. The
    `write-adr` skill never rewrites an accepted ADR's decision, so §6
    itself stays as it is.
- **The catalog:** 2003 put the `hex-grid` answer ("hex grid") in the
  Perspective concern of `docs/modules.ron`, by the default engine,
  parked, naming 2121 and 2740. It stays parked here; 2740 sets it to
  ready.
- **`shuyi_core::hex`** (`crates/core/src/hex.rs`), `no_std`, integers
  only:
  - `Hex { q: i32, r: i32 }`, axial, with `Pos`'s derives, `Hex::new`,
    `Add` and `Sub` (both saturate, so no input panics), `s() -> i64`
    (`-q - r`, in `i64` so it can't overflow) and `Hex::distance(a, b)
    -> u32`: `(|dq| + |dr| + |ds|) / 2`, worked in `i64` and
    saturating;
  - `HexDir`, the six directions: a newtype over `0..6` with a private
    field and `Dir`'s derives (serde as its index, checked on load with
    `#[serde(try_from = "u8", into = "u8")]`), `HexDir::ALL`
    (counter-clockwise from axial `+q`, Red Blob's numbering: `(+1, 0)`,
    `(+1, −1)`, `(0, −1)`, `(−1, 0)`, `(−1, +1)`, `(0, +1)`), `delta()`,
    `opposite()` and `rotate(steps: i32)` (positive steps go on in
    `HexDir::ALL` order, counter-clockwise, modulo 6). Named constants
    for each orientation, all aliases of the six:
    - pointy top: `POINTY_EAST`, `POINTY_NORTH_EAST`,
      `POINTY_NORTH_WEST`, `POINTY_WEST`, `POINTY_SOUTH_WEST`,
      `POINTY_SOUTH_EAST`;
    - flat top: `FLAT_SOUTH_EAST`, `FLAT_NORTH_EAST`, `FLAT_NORTH`,
      `FLAT_NORTH_WEST`, `FLAT_SOUTH_WEST`, `FLAT_SOUTH`;
  - `Hex::neighbor(self, dir)`, and `Hex::neighbors(self) -> [Hex; 6]`
    in `HexDir::ALL` order;
  - `Hex::ring(center, radius: u32) -> impl Iterator<Item = Hex>`: the
    `6 × radius` tiles at exactly `radius` (the centre alone at 0). It
    starts at `center` plus `radius` steps of `HexDir::ALL[4]`, then
    takes `radius` steps in each of the six directions in order;
  - `Hex::within(center, radius: u32) -> impl Iterator<Item = Hex>`:
    every tile at most `radius` away, ring by ring from the centre,
    `3r(r + 1) + 1` tiles. An attack range, say;
  - `HexLayout { orientation: Orientation, offset: Offset }` with
    public fields, and `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`,
    `Hash`, `Default` and serde (what 2117's `Topology` derives needs).
    `Orientation` is `Pointy` or `Flat`; `Offset` is `Odd` or `Even`.
    Pointy shifts the odd (or even) rows half a tile right (odd-r,
    even-r); flat shifts the odd (or even) columns half a tile down
    (odd-q, even-q). The default is pointy and odd;
  - `HexLayout::to_hex(self, pos: Pos) -> Hex` and
    `HexLayout::to_pos(self, hex: Hex) -> Pos`, with Red Blob's
    formulas (odd-r: `q = col − (row − (row & 1)) / 2`, `r = row`;
    even-r adds `row & 1` instead; odd-q and even-q swap the roles of
    column and row). `row & 1` is the parity for negative rows too, so
    they invert each other on negative rows as well. They work in `i64`
    and saturate, so no input panics;
  - `HexLayout::neighbors(self, pos) -> [Pos; 6]` in `HexDir::ALL`
    order (some may be off the grid), and
    `HexLayout::distance(self, a: Pos, b: Pos) -> u32`;
  - `Grid::neighbors6(&self, pos, layout)` in `grid.rs`, beside
    `neighbors4` and `neighbors8`: the in-bounds ones, in that order.
- **`Topology::Hex(HexLayout)`** in 2117's `Topology`
  (`crates/core/src/path/topology.rs`):
  - `steps` gives the six neighbours of `HexLayout::neighbors`, in
    `HexDir::ALL` order, each of weight 1, and uses `open` as it does
    for `Square4` (hexes have no corners, so no corner rule).
    `step_weight` is `Some(1)` for those six and `None` otherwise;
  - `distance` is `HexLayout::distance`, so `find_path`'s heuristic is
    admissible on hex too;
  - nothing else in the searches changes. The module docs of `path.rs`
    name it.
- **Exports and docs:**
  - re-export `Hex`, `HexDir`, `HexLayout`, `Orientation` and `Offset`
    from `crates/core/src/lib.rs`, and name hex grids in its doc
    comment;
  - `shuyi-core`'s row in `README.md`;
  - a doc test in `hex.rs`: a 7 × 5 odd-r map, the tiles within 2 of a
    unit, and a path round a wall with `find_path` and `Topology::Hex`.
- **A pinned result:** a unit test pins one hex `find_path` (its tiles
  and cost) and one `within` list. CI's `test` job runs them on x64 and
  ARM64. **From 2116 (done):** its `determinism` job also runs them on
  wasm32 with the rest of `shuyi-core`'s tests, with nothing more to
  add.
- **Tiled hex maps. Either order with 1124:** 1124 keeps a Tiled map's
  stagger axis and stagger index. If 1124 is done, add one function
  beside its `TileMap` (`shuyi_assets::tilemap`) that gives a
  hexagonal map's `HexLayout`: the stagger axis gives its orientation
  (`y` is `Pointy`, `x` is `Flat`) and the stagger index its offset
  (`odd` is `Odd`, `even` is `Even`); any other map gives `None`. Test
  it on a hexagonal map made in Tiled. If not, nothing to do: 1124
  adds it.
- **What changes for the game:** nothing. New items, and a variant on
  2117's `#[non_exhaustive]` `Topology`. Say so in the PR.

**Out (do not do):**
- drawing, picking, the example, the guide `docs/guides/hex-grid.md`,
  and the catalog's answer set to ready (2740);
- a line between two hexes, field of view and line of sight on hex
  (2120 is square only), rotation and reflection of regions, and cube
  coordinates as a type: each a follow-up when a hex game asks;
- maps of other shapes than the rectangle of offset rows (a hexagon or
  a rhombus): a game leaves tiles out with its cost function;
- the `.map` format naming a layout, and importing Tiled hex maps
  (1124; LDtk has no hex maps);
- hex autotiling and isometric hex;
- a hex module crate: this is part of the default kit, as above.

## Implementation steps

1. Read Red Blob Games' guide (axial and offset coordinates,
   distances, neighbours, rings), 2117's `Topology`
   (`crates/core/src/path/topology.rs`), the module docs of `path.rs`,
   `crates/core/src/path/tests.rs` (its random cost maps and
   brute-force references), and 2013's ADR.
2. Write the ADR (with its row in `docs/adr/README.md`), and note it on
   ADR-0003's `Status:` line. The catalog's `hex-grid` answer is
   already there (2003) and stays parked.
3. Write `Hex`, `HexDir`, `ring` and `within`, with tests.
4. Write `HexLayout`, its conversions and neighbours, and
   `Grid::neighbors6`, with property tests in all four layouts.
5. Add `Topology::Hex`, and run the property tests of
   `crates/core/src/path/tests.rs` over it in all four layouts, with a
   brute force over `HexLayout::neighbors`.
6. Write the docs and the doc test, add the Tiled hex function if 1124
   is done, and run the gates.

**Size:** close to 600 changed lines. If the diff runs long,
`Topology::Hex` (step 5, its search tests and the doc test's path)
moves to a parked follow-up ticket (the `write-ticket` skill, blocked by
this one). Add that ticket to 2740's `blocked_by` and say so in the
completion notes.

## Acceptance criteria

- [ ] The ADR is in the PR with its row in `docs/adr/README.md`,
      ADR-0003's `Status:` line points at it, and the `hex grid` answer
      stays in `docs/modules.ron` as parked, naming 2121 and 2740.
- [ ] `to_pos(to_hex(p)) == p` and `to_hex(to_pos(h)) == h` for every
      position within `±(1 << 29)`, negative ones included, in all four
      layouts; at the extremes of `i32` nothing panics (property tests).
- [ ] Neighbours are symmetric: `b` is a neighbour of `a` exactly when
      `a` is one of `b`, and each is at distance 1, in all four layouts
      (property test).
- [ ] `distance` is zero only for a tile and itself, is symmetric, keeps
      the triangle inequality, and equals the steps of a breadth-first
      search on an open map (property tests).
- [ ] `ring(c, r)` has `6r` tiles for `r ≥ 1`, each at distance `r`,
      none twice; `within(c, r)` has `3r(r + 1) + 1` (property tests).
- [ ] With `Topology::Hex`, the costs of `reachable_on` and
      `flow_field_on` equal a brute force over `HexLayout::neighbors`,
      and `find_path`'s cost equals `reachable_on`'s, with each tile of
      its path a hex neighbour of the one before (property tests).
- [ ] In every layout, `(±1, 0)` and `(0, ±1)` from any tile are its
      neighbours (property test: the ADR's claim about the cursor).
- [ ] `shuyi-core` still builds for `thumbv7em-none-eabihf`, and 2015's
      float ban passes on the new code.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the six deltas and the orientation constants; `opposite` and
  `rotate`; `ring` at radius 0, 1 and 2 against the guide's pictures;
  the conversions on a hand-checked 4 × 4 map in each layout; the
  pinned path and range.
- Property: the criteria above.
- Doc test: the 7 × 5 map in `hex.rs`.

## Completion notes

*(Filled in by the session that completes the ticket.)*
