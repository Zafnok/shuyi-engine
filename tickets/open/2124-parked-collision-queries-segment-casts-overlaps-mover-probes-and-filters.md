---
id: "2124"
title: "Parked: Collision queries in fixed point: segment casts against tiles and boxes, point and overlap queries, mover probes, and filters by layer"
type: feature
milestone: "Engine: real time"
model: reasoning
effort: high
status: blocked
blocked_by: ["1122"]
nick_input: none
completed:
---

# 2124 — Collision queries: casts, overlaps, probes and filters

> **Parked** (Nick, 2026-10-20: "unless these real-time work tickets overlap with visions of shuyi they should be parked until visions of shuyi is complete"): this is real-time work, so it waits until Visions of Shuyi launches, unless the game needs it first (its ticket 0060 says the army walks around freely, so 1119–1125 become game work, and a scene where it walks must ask what stands in front of a walker or under the pointer; the game asks through its `engine-change` skill). Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8, and with GameMaker, Defold and Construct 3. Every one of
them can ask a 2D world a question without moving anything:

- Godot: `PhysicsDirectSpaceState2D` has `intersect_ray`,
  `intersect_point`, `intersect_shape` and `cast_motion`, and every
  query takes a collision mask and bodies to leave out
  (<https://docs.godotengine.org/en/stable/classes/class_physicsdirectspacestate2d.html>).
  Every body has collision layers and masks;
- Bevy: `bevy_shape` has `Aabb2d` and `BoundingCircle` with
  `RayCast2d`, `AabbCast2d` and `BoundingCircleCast`
  (<https://docs.rs/bevy_shape/latest/bevy_shape/>);
- Unity: `Physics2D.Raycast` and `Linecast` with a `LayerMask`, and
  `ContactFilter2D`;
- GameMaker: `place_meeting` accepts a tile map. Construct 3: the Line
  of sight behaviour casts a ray and returns the hit point and the
  normal. Defold: ray casts against its collision objects.

What Shuyi has today (checked in `crates/core/src/`): no collision code
at all. There is no `collide` module and no `Fx`. There is `grid.rs`
with `Pos` and `Dir` (`Right`, `Down`, `Left`, `Up`; `y` grows down),
which this ticket reuses.

What is planned:

- 1122 (parked): `shuyi_core::collide` with `TileCollision` (solid,
  empty, one-way-up), `Aabb`, a uniform-grid broad phase, pixel-stepping
  `Mover::move_x` and `move_y` that return what they hit, and `Trigger`
  enter and leave events. It has no query that doesn't move a body. With
  the edit this ticket asks of it, it also gets `CollisionLayers` (a
  layer and a mask on boxes, triggers, movers and tile maps), an id for
  every box, and a list of one cell's boxes in id order;
- 3105 (parked): `ray_cast` and `shape_cast` with hits sorted by
  distance then id, for 3D only. So 2D would have less than 3D;
- 2708 (parked): `within_radius`, `in_rect` and `nearest` over an
  army's unit rows, in `shuyi-rts`. No tiles, no rays;
- 2120 (parked): field of view and line checks on a turn game's grid,
  tile to tile, not in free space;
- 2317 (parked): draws tiles, boxes, triggers and mover hits, but no
  casts. 1127 (parked) names "ground and wall probes" with no API
  behind them.

**Why it matters.** Visions of Shuyi is a grid game and doesn't need
this now (see the note above). Other 2D games ask these questions every
step: "is there ground below", "did the shot hit, and not its shooter",
"can the guard see the player", "is there a ledge ahead", "what is under
the pointer". The core has no floats (`AGENTS.md` rule 5), so a game
can't pull in a float ray-cast crate without breaking its replay
hashes. Without this, every action game an agent makes writes its own
fixed-point ray cast, which works against the end goal (Nick,
2026-10-19: a game made "in not too many LOC").

**Choices made here,** from the two reviews of this gap:

- Layers and masks belong on 1122's types from the start, because they
  decide what stops a mover and what enters a trigger. Adding them later
  would break every game on the kit. So they are an edit to 1122, and
  this ticket only takes a filter on its queries.
- A swept box is a mover's dry run, not a separate solver. A continuous
  swept cast could disagree with the pixel-stepping mover by a pixel and
  say "clear" where the move then stops. `Mover::probe_x` and `probe_y`
  run the mover's own code on a copy, so a probe and a move always
  agree. "Is there ground below" is a `probe_y` of one pixel down.
- A hit reports where it is along the segment as a fraction, plus the
  point and an axis normal, not a Euclidean distance. That needs no
  square root (`Fx::sqrt` is 2701's or 2106's).

It is about 600 changed lines without fixtures and snapshots: roughly
350 in `query.rs` with its docs and the probes, and 250 of tests.

## Nick input

None.

## Scope

**In:**
- **`shuyi_core::collide::query`** (`crates/core/src/collide/query.rs`,
  tests in `collide/query/tests.rs`), pure, `no_std`, integers only.
  Its types are re-exported from `collide`, beside 1122's. Below,
  `BroadPhase` and `BoxId` stand for 1122's broad phase and the id it
  gives a box: use the names 1122 chose.
  - `QueryFilter { mask: u32, exclude: Option<BoxId> }`, with
    `QueryFilter::ALL` (mask `u32::MAX`, nothing left out) and
    `QueryFilter::skip(id)`. A box passes when `layers.layer & mask !=
    0` and it isn't `exclude`. A `TileCollision`'s tiles pass when its
    `layers.layer & mask != 0`. A query is a question, not a body, so it
    has a mask and no layer, as Godot's query parameters have
    `collision_mask` and `exclude` and no layer.
  - `Target`: `Tile(Pos)` or `Box(BoxId)`.
  - `Hit { target: Target, point: FxVec2, normal: Dir, t: Fx }`. The
    normal is the side the segment came in through, pointing back out
    (a segment moving down that enters a tile's top has `Dir::Up`). `t`
    is how far along the segment, `0..=1`, rounded toward negative
    infinity as `Fx` rounds.
  - `CollisionQuery<'a>`, built with `CollisionQuery::new(tiles:
    Option<&'a TileCollision>, boxes: Option<&'a BroadPhase>)`. It holds
    two references and nothing else, so a stage builds one whenever it
    asks. Its methods:
    - `cast(from: FxVec2, to: FxVec2, filter) -> Option<Hit>`: the
      first solid the segment enters;
    - `cast_all(from, to, filter, out: &mut Vec<Hit>)`: every solid it
      enters, in order (a shot that goes through);
    - `at_point(point: FxVec2, filter, out: &mut Vec<Target>)`;
    - `overlap_rect(rect: Aabb, filter, out: &mut Vec<Target>)`;
    - `overlap_circle(center: FxVec2, radius: Fx, filter, out: &mut
      Vec<Target>)`. A circle is a shape to ask with only, never a body.

    A ray is a segment: `to = from + dir × max`. The docs say so, and
    there is no separate ray function.
- **The rules,** in a `# Rules` section of the module docs, as
  `crates/core/src/path.rs` has, each pinned by a unit test:
  - **Touching is not hitting.** A cast hits a solid where the segment
    first goes into its open interior. A segment that slides along a
    face, or only touches a corner, hits nothing there. This is the rule
    1122's movers live by: they rest touching what stopped them.
  - **Each tile is its own box.** A segment that starts inside a solid
    (or inside several overlapping ones) ignores each solid it starts
    inside and hits the next one it enters, which may be the next tile
    of the same wall. "Am I inside" is `at_point`.
  - **One-way-up tiles** are hit only through their top side, by a
    segment moving down. They are never in `at_point` or overlap
    answers: they stop only what comes down onto them.
  - **Corners.** When the segment enters a solid through an exact
    corner, the normal is on the `x` axis. Where two solid tiles meet
    diagonally, a segment from one of the two empty tiles beside them to
    the other, through that exact point, touches both and enters
    neither, so it passes between them. Movers can't: they have a size.
  - **Order.** The place along the segment is kept exactly, as a
    fraction of two `i64` values, and two places are compared by
    multiplying across in `i128` (raw `Fx` differences can need 33
    bits). At the same place, tiles come before boxes, tiles in `(y, x)`
    order, boxes by id. `Pos`'s derived `Ord` is `(x, y)`, so compare
    `(p.y, p.x)` by hand, as `path.rs` does. Overlap and point answers
    use the same order without the place. So ties never depend on memory
    order.
  - **The hit point.** Its coordinate on the normal's axis is exactly
    the side entered. The other coordinate is `from + d × fraction`,
    rounded toward negative infinity.
  - **Points** are inside a box or tile when `min <= p < max` on both
    axes, so a point on a shared edge belongs to one tile.
  - **Overlaps** need the interiors to overlap (`a.min < b.max &&
    b.min < a.max` on both axes). A circle overlaps a box when the
    squared distance from its centre to the box's closest point is less
    than `radius²`, computed in `i128`.
  - **Off the map.** Tiles off the map count as 1122's `TileCollision`
    counts them for movers. Boxes are found wherever 1122's broad phase
    keeps them, including any it keeps outside its cells.
- **One entry test, a fast path and a reference:**
  - one function, `enter(from, to, rect) -> Option<(place, Dir)>` (the
    slab test on the open box), is used by everything below;
  - **tiles:** a grid traversal (Amanatides and Woo, "A Fast Voxel
    Traversal Algorithm for Ray Tracing", 1987, written from its
    description). The next `x` and `y` crossings are fractions compared
    by multiplying across. At an exact corner it steps both axes at
    once. A segment lying exactly on a grid line enters no tile beside
    it;
  - **boxes:** the same traversal over the broad phase's cells, calling
    `enter` on each box a cell lists. `cast` stops once the next cell
    starts past the best hit so far. `cast_all` sorts its `out` with
    `sort_unstable_by` and removes a box met in two cells with
    `dedup_by`. Both work in place (a stable `sort` allocates a
    buffer);
  - **the reference,** in the tests: `enter` on every solid tile in the
    segment's bounding rectangle (tiles off the map counted as the rule
    above counts them) and on every box, sorted by the order above.
    Every fast answer must equal it.
- **No allocation of its own.** `cast` allocates nothing. The `out`
  forms clear `out` and fill it, so they allocate only when `out` must
  grow. `query.rs` has no `Vec::new`, `vec!`, `Box::new`, `collect`,
  `to_vec` or stable sort outside its tests. (A counting allocator
  can't be written here: the workspace forbids `unsafe_code`.)
- **Mover probes,** in 1122's `Mover`: `probe_x(&self, …)` and
  `probe_y(&self, …)` take `move_x`'s and `move_y`'s arguments and
  return exactly what those would return, from a copy of the mover. The
  mover itself, its fractional remainder included, is unchanged. To
  probe both axes, a game probes `x` on a copy, then `y` from where it
  ended; the docs show it. A box that isn't a mover is probed through a
  mover made for it.
- **If 1122 is done without `CollisionLayers`** (the edit this ticket
  asks of it was missed), or without box ids and a per-cell listing in
  id order, stop. Write a ticket for them (the `write-ticket` skill),
  with the layer rule and tests 1122's edit describes, add it to this
  ticket's `blocked_by`, and say so. Adding them changes `Mover`,
  `Trigger` and the broad phase, which is a PR of its own.
- **Either order with 2317:** if 2317 is done, add
  `cast_gizmos(from, to, hit: Option<&Hit>, to_px, &mut Gizmos)` to
  `shuyi_ui::debug::collide`, in the `collision` channel: a `Line` from
  `from` to the hit point (or to `to` when nothing was hit), and at a
  hit a `Cross` and a short line along the side entered, all
  `GizmoRole::Note`. If 2317 isn't done, 2317 adds it.
- **Either order with 1127:** if 1127 is done, its ground and wall
  checks become `probe_y` and `probe_x` calls, with its tests unchanged.
  If its enemy that walks back and forth turns at ledges, that check
  becomes a `cast` of a short segment down ahead of its feet: no hit
  means no ground. If 1127 isn't done, 1127 uses them.
- **The module docs,** with doc tests:
  - a ground check (`probe_y`);
  - a shot that skips its shooter (`QueryFilter::skip`) and one that
    skips a whole layer (the mask);
  - a line of sight between two points: clear when `cast` finds
    nothing;
  - what is under the pointer: `at_point` at a point in world pixels,
    which 1119's `TickInput` carries once 1121 fills it.
- **A fixture table:** the hits of 12 fixed casts and 4 overlap queries
  on two hand-drawn maps with boxes and one-way tiles, one line per
  answer. A test compares the text with
  `crates/core/src/collide/query/fixtures.txt`, read with
  `include_str!` so the test opens no file, and prints the new text
  when they differ. CI's `test` job runs it on x64 and ARM64. **From
  2116 (done):** its `determinism` job runs `shuyi-core`'s tests on
  `wasm32-wasip1`, so it runs there too, with nothing more to add
  (2114's pinned values in `rng.rs` work the same way).

**Out (do not do):**
- slopes, rotated boxes, polygons, and circles as bodies;
- swept shapes other than movers' probes;
- rigid-body shapes and rapier's own queries (1128);
- grid sight for turn games (2120), vision and fog for armies (2713),
  and 3D casts (3105);
- closure filters and a nearest query: a game filters `cast_all` or
  overlap answers itself (2708 has `nearest` for armies);
- moving platforms that carry what stands on them (2122's Out says it
  is a ticket of its own, parked with 1127);
- the platformer controller (1127), pointer input (1121), and the tile
  under the pointer on a turn game's grid map (2405);
- `CollisionLayers` and box ids themselves (1122);
- an example of its own: the doc tests and the fixture table show it,
  and 1127's platformer uses it. 2738's shooter (a ticket 2738 writes)
  finds its hits with `overlap_rect` if this ticket is done, and with
  1122's broad phase if not: nothing to do here either way.

## Implementation steps

1. Read 1122's `shuyi_core::collide` (the names of the broad phase and
   the box id, its tile size, its rule for tiles off the map, and
   `CollisionLayers`), 1119's `shuyi_core::fx`, `crates/core/src/grid.rs`
   (`Pos`, `Dir`) and the module docs of `crates/core/src/path.rs`. If
   `CollisionLayers`, box ids or the per-cell listing are missing, stop
   as the Scope says.
2. Write `enter` and the fraction type, with unit tests on single boxes:
   each side, each corner, a graze along a face, a start inside, a start
   on a face.
3. Write the tile traversal and `cast` against tiles, then against
   boxes through the broad phase's cells, then `cast_all`.
4. Write `at_point`, `overlap_rect` and `overlap_circle`.
5. Write the reference and the property tests.
6. Add `probe_x` and `probe_y` to `Mover`.
7. Write the module docs and doc tests, the fixture table, and the 2317
   and 1127 parts if they are done.
8. Run the gates.

**Size:** if the diff runs past about 600 changed lines,
`overlap_circle` and the 2317 and 1127 parts move to a parked follow-up
ticket (the `write-ticket` skill, blocked by this one), which takes
over this ticket's either-order clauses with 2317 and 1127. Say so in
the completion notes.

## Acceptance criteria

- [ ] On random tile maps up to 16 × 16 with one-way tiles, up to 12
      random boxes on random layers, and random segments and filters,
      `cast` and `cast_all` equal the reference exactly, in order
      (property test, 10 000 cases). Boxes and segments may lie partly
      off the map and off the broad phase's cells.
- [ ] `cast` always equals the first element of `cast_all`, and is
      `None` when `cast_all` is empty (property test).
- [ ] When `cast` finds nothing, no point of the segment sampled at
      every 1/64 of its length (rounded toward negative infinity) is
      strictly inside (`min < p < max` on both axes) a solid the filter
      passes, other than solids it starts inside and one-way tiles
      (property test, a check of the reference itself).
- [ ] `at_point`, `overlap_rect` and `overlap_circle` equal a brute-force
      scan of every tile and box, in the same order (property test).
- [ ] With mask 0, every query answers nothing. `QueryFilter::skip(id)`
      removes exactly that box's answers (property test).
- [ ] A segment moving up through a one-way tile never hits it; one
      moving down into its top hits it with `normal == Dir::Up` (unit
      tests).
- [ ] Where two solid tiles meet diagonally, a 45° segment from one of
      the two empty tiles beside them to the other, through their shared
      corner, hits neither; a segment sliding along a floor's top hits
      nothing (unit tests).
- [ ] For random movers and moves, `probe_x(a)` equals what `move_x(a)`
      returns on a copy, the same for `y`, and the probed mover is
      unchanged (property test).
- [ ] The fixture table's test passes against
      `crates/core/src/collide/query/fixtures.txt`.
- [ ] `shuyi-core` still builds for `thumbv7em-none-eabihf`, and `grep
      -nE "f32|f64" crates/core/src/collide/query.rs` finds nothing.
- [ ] `grep -nE "Vec::new|vec!|Box::new|collect|to_vec|\.sort(_by|_by_key)?\(" crates/core/src/collide/query.rs`
      finds nothing outside the tests module.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `enter` on each side and corner, grazes, starts inside and on a
  face; the tile traversal on horizontal, vertical, 45° and shallow
  segments, and along a grid line; one-way tiles; a zero-length segment
  (no hit); a tile and two boxes entered at the same place (the tile,
  then the boxes by id); `overlap_rect` over several tiles, in `(y, x)`
  order; `at_point` on a shared edge; a circle touching a box's corner
  (no overlap).
- Property: the criteria above.
- Snapshot / integration: the fixture table; `cast_gizmos`' gizmo list,
  if 2317 is done.
- Doc tests: the four examples in the module docs.

## Completion notes

*(Filled in by the session that completes the ticket.)*
