---
id: "2120"
title: "Parked: Field of view and line of sight on the grid: symmetric shadowcasting, line checks, and remembered tiles"
type: feature
milestone: "Engine: roguelikes"
model: reasoning
effort: medium
status: blocked
blocked_by: []
nick_input: none
completed:
---

# 2120 — Field of view and line of sight on the grid

> **Parked** (Nick, 2026-10-20, choosing roguelikes as a kind of game the engine serves; Visions of Shuyi is "the first game I want to be launched with the engine"): like real-time strategy, visual novels and detective games, this waits until after Visions of Shuyi launches. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6,
Unreal 5.8 and Construct 3. Each answers "can A see B" its own way:

- Construct 3 has a Line of sight behavior, with a range, a cone and
  obstacles;
- Unreal's AI Perception has a sight sense;
- Godot (`RayCast2D`, `intersect_ray`), Unity (`Physics2D.Linecast`)
  and Bevy (through physics crates) cast rays in free space. None of
  them has a field of view on a grid. The roguelike libraries do:
  libtcod, rot.js and bracket-lib.

What Shuyi has today: no sight at all. Near it:

- `Grid`, `Pos` and `TileSet`, a bitset of tiles whose `union_with`
  merges one set into another (`crates/core/src/grid.rs`);
- `reachable` and `flow_field`, the grid kit's answer to "where can it
  go" (`crates/core/src/path.rs`);
- `MapScene` shows ranges by name (`tint`, `RangeLayer`), and a tile
  whose `terrain` is `None` gets no terrain drawn by the glyph skin
  (`crates/ui/src/grid/scene.rs`; `draw_tiles` in
  `crates/ui/src/grid/glyph/style.rs`).

Planned work comes near it but doesn't cover it:

- 2713 (parked) stamps sight circles for real-time strategy fog, and
  leaves out line of sight blocked by terrain;
- 2124 (parked) casts rays in free movement, and 3105 (parked) in 3D;
- 1146 (parked) lets a referee keep each player's view, but computes
  none.

Why it matters:

- **The game first:** Visions of Shuyi has no line of sight or fog in its
  code, and whether it has fog is undecided (its ticket 0053, through
  1146). Nick's rules of 2026-10-20 (`docs/decisions.md`) put what the
  game needs first: a piece it asks for through its `engine-change`
  skill is split out and unparked on its own. If the game asks for
  walls that block sight, or fog with walls, this ticket is that piece,
  and Nick unparks it for the game.
- **Other 2D games:** roguelikes (added by Nick on 2026-10-20) need it
  every turn: what the hero sees, what a foe sees, what the map
  remembers. So do stealth games, and tactics games with fog and walls.

It lives in `shuyi-core`, beside `path.rs`, not in `shuyi-procgen`
(2118): it serves any grid game, needs only `Pos` and `TileSet`, and is
small. It needs nothing that isn't done, so `blocked_by` is empty. The
roguelike example (2735) waits on it.

The method is Albert Ford's symmetric shadowcasting
(https://www.albertford.com/shadowcasting/). It scans the four quarters
around the viewer (north, east, south and west) row by row, and keeps
slopes as fractions of whole numbers, which suits the core's rule of no
floats. It promises:

- symmetry: if A sees B, B sees A (for tiles that don't block);
- walls are shown as far as any part of them shows;
- shadows widen behind pillars;
- no blind corners.

The article's code is licensed under CC0, so ours may follow it
closely. Write it in Rust; the article has no radius, which this ticket
adds.

## Nick input

None.

## Scope

**In:**
- **`shuyi_core::sight`** (`crates/core/src/sight.rs`), `no_std`,
  integers only:
  - `field_of_view(width: u16, height: u16, origin: Pos, radius: u16,
    blocks: impl Fn(Pos) -> bool) -> TileSet`:
    - the origin is always in the set, when it is on the map. An origin
      off the map gives an empty set;
    - a tile that doesn't block is in the set when its centre is inside
      its row's lit sector (Ford's `is_symmetric`);
    - a tile that blocks is in the set when any part of it shows (the
      columns of Ford's `Row.tiles()`, with `round_ties_up` and
      `round_ties_down`);
    - slopes are pairs of integers (`2·col − 1` over `2·depth`, and
      `-1` and `1` for the first row). Comparisons multiply across in
      `i64`, and a row's first and last columns round with integer
      floor division (`div_euclid`). No floats anywhere;
    - a tile is within the radius when `dx² + dy² ≤ radius² + radius`,
      computed in `u64`, so radius 1 is the eight neighbours and circles
      come out round. A tile past the radius is never in the set. The
      scan reads no row deeper than `radius`, and a tile it reads that
      is outside the circle still blocks or not as `blocks` says;
    - a tile off the map blocks and is never in the set. `blocks` is
      asked only about tiles on the map, as `path.rs`'s cost function
      is;
    - the scan keeps a stack of rows and does not recurse, so a large
      radius can't overflow a small stack (the Playdate's, 3028);
  - `sees(width: u16, height: u16, from: Pos, to: Pos, blocks) -> bool`:
    the same answer as `field_of_view(width, height, from, r,
    blocks).contains(to)` for any `r` that reaches `to`. It scans only
    the quarters that hold `to` (two when `to` is on a diagonal), and
    only the rows up to it. It is `false` when either tile is off the
    map, and `true` for `sees(a, a)` on the map;
  - `line(from: Pos, to: Pos) -> Vec<Pos>`: the tiles a thrown thing
    passes, both ends included. There is one tile per step along the
    longer axis: the one whose centre is nearest the true line. A tie
    goes to the tile with the smaller `(y, x)`, so `line(b, a)` is
    `line(a, b)` reversed. It knows no map: skipping tiles off the map
    is the caller's. Deltas are worked in `i64`. A game checks the
    tiles itself, for example
    `line(a, b).into_iter().find(|&p| blocks(p))`;
  - **remembered tiles** need no new type. The module docs show a game
    keeping a `TileSet` of the tiles seen so far, and merging each
    turn's view into it with `TileSet::union_with`;
  - the module docs also say that `sees` does not walk `line`: a seen
    tile can have a blocking tile on its `line`, so a game picks which
    one its rules use for a shot.
- **Exports and docs:**
  - `pub mod sight;` and `pub use sight::{field_of_view, line, sees};`
    in `crates/core/src/lib.rs`, and name sight in its doc comment
    ("the tactics kit (a square grid, paths over it, ...)");
  - add sight to `shuyi-core`'s row in `README.md` ("the tactics kit:
    grid, paths, turns" becomes "grid, paths, sight, turns").
- **Fixture maps**, as text in `crates/core/src/sight/tests.rs`: an open
  room, a room with pillars (the article's widening shadow), and a
  corridor that turns a corner (the article's no blind corners), each
  at most 11 × 11, with its view from two places. A map's text has `#`
  for a tile that blocks, `.` for one that doesn't, and `@` for the
  viewer; its view is the same rows with a space for each tile not in
  the set. Work the views out by hand from Ford's rules, not from the
  code's output. CI's `test` job runs them on x64 and ARM64, and 2116's
  `determinism` job runs every `shuyi-core` test on wasm32, so they need
  no golden table, whichever ticket lands first.

**Out (do not do):**
- cones (a view limited to a facing): a follow-up when a stealth game
  asks;
- heights, light levels and tiles seen in part;
- drawing. Hiding unseen tiles and dimming remembered ones is the
  game's scene: 2735 shows it with blank tiles and `MapScene::tint`. A
  debug view channel (2315) is a follow-up if a developer asks;
- fog of war per team (2713), 2D lights (2328, which leaves its own
  shadows and occluders to a follow-up), ray casts in free movement
  (2124) and in 3D (3105), and per-player views on a server (1146);
- eight-way moves and A* paths (2117), and hex grids (2121): this
  ticket is square grids only;
- pixel lines for drawing: 2336's width-1 line and 2315's gizmo lines
  are Bresenham's in pixels, with their own rule. `line` here is in
  tiles and symmetric;
- the roguelike example (2735).

## Implementation steps

1. Read Ford's article, and `crates/core/src/path.rs` with
   `crates/core/src/path/tests.rs` for the grid kit's conventions: sizes
   as `u16`, the closure asked only on the map, determinism notes in the
   module docs, and tests in a `tests.rs` beside the module
   (`#[cfg(test)] mod tests;`).
2. Write `line` and its unit tests. `n` is the larger of `|dx|` and
   `|dy|`, and `d` the other axis's delta. At step `i` the other
   coordinate is the start's plus `ceil((2·d·i − n) / (2·n))`, worked
   with `div_euclid`, which rounds half down, so a tie takes the smaller
   coordinate. When
   `from == to` it is `vec![from]`.
3. Write `field_of_view`:
   - a `Row { depth, start, end }` with slopes as `(num, den)` pairs,
     `den > 0`, and Ford's `Row(1, -1, 1)` first in each quarter;
   - Ford's transform from `(depth, col)` to a `Pos` around the origin
     `(x, y)`: north `(x + col, y − depth)`, south `(x + col,
     y + depth)`, east `(x + depth, y + col)`, west `(x − depth,
     y + col)`;
   - a `Vec<Row>` as a stack in place of Ford's recursive `scan`.
     Where Ford calls `scan(next_row)` on a floor-to-wall step, push
     the next row with the row's `start` as it stands then and `end`
     set to that tile's slope. After the last tile, if it was a floor,
     push the next row with the row's current slopes. The order rows
     are popped doesn't change the set;
   - stop at depth `radius`.

   Then `sees`, on the same quarter scan with a last depth and a target.
4. Write the property tests and the fixture maps. Every test's `blocks`
   panics when asked about a tile off the map, as `Costs` does in
   `path/tests.rs`.
5. Re-export, write the module docs (with the remembered tiles example
   as a doc test), and update the README row.
6. Run the gates.

## Acceptance criteria

- [ ] Symmetry: on random maps up to 24 × 24, for two random tiles that
      don't block, `b` is in `a`'s view exactly when `a` is in `b`'s, at
      the same radius (property test).
- [ ] With no walls, the view is every tile on the map within the radius
      (property test).
- [ ] Every tile in a view is within the radius (property test).
- [ ] Making one more tile block never adds any other tile to a view
      (property test).
- [ ] `sees` agrees with `field_of_view` (property test).
- [ ] `line` starts at `from`, ends at `to`, has `max(|dx|, |dy|) + 1`
      tiles, each one king's step from the one before, and `line(b, a)`
      is `line(a, b)` reversed (property test).
- [ ] No test's `blocks` is asked about a tile off the map (its closure
      panics if it is).
- [ ] The fixture maps' views match their text.
- [ ] A view of radius `u16::MAX` from `(1, 0)` on an open 3 × 65000
      map holds every tile, and finishes on the test thread's stack.
- [ ] `shuyi-core` still builds for `thumbv7em-none-eabihf`, and
      `grep -nE "f32|f64" crates/core/src/sight.rs` finds nothing.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `line` for the eight directions, a tie, and `from == to`; the
  origin alone at radius 0; the eight neighbours at radius 1; an origin
  off the map sees nothing; `sees` from a tile to itself, and to a tile
  off the map.
- Property: the six property criteria above.
- Fixture: the three maps' views, compared with the text written in
  the test.
- Doc test: the remembered tiles example.

## Completion notes

*(Filled in by the session that completes the ticket.)*
