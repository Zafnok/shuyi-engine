---
id: "3105"
title: "Parked: 3D collision in Fx12: shapes, static meshes and heightfields, a grid broad phase, ray and shape casts, triggers, and a character mover"
type: feature
milestone: "Parked: 3D"
model: opus-5.5
effort: xhigh
status: blocked
blocked_by: ["3101", "1122"]
nick_input: none
completed:
---

# 3105 — 3D collision and the character mover

> **Parked** (Nick, 2026-10-17: "I want to expand on this and add true 3D support (all kinds, first person, third, top down with 3D models), and HD-2D"): after the engine reaches parity with the game (the MVP in `docs/ROADMAP.md`) and after the 2D work already planned (`docs/decisions.md`, 2026-10-17). Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

[ADR-0011](../../docs/adr/0011-3d-fixed-point-space-a-3d-view-in-the-frame-and-hd-2d-as-a-skin.md)
§6. A 3D game needs to walk, stand on ground, climb steps, and aim, and
all of it must replay exactly. So collision is a small kit in `Fx12`
with a fixed order for everything it returns, the 3D form of 1122's
movers. Rigid bodies in floats are a separate module later
(`rapier3d`, as 1128 does with rapier2d); they are not this ticket.

Gameplay never reads a bone (ADR-0011 §4): hits use these shapes only.

It builds on 3101 (`Fx12`, `FxVec3`, `FxTransform`, `Angle16`, `sqrt`)
and 1122 (the 2D kit's names and rules: `Mover`, `Trigger`, the grid
broad phase, settings in RON).

3102 (meshes) and this ticket can land in either order. If 3102 has
landed, add `TriMesh::from_mesh(&Mesh, &FxTransform)` here. If not,
build static meshes from triangle lists here, and 3102 adds
`from_mesh`.

## Nick input

None.

## Scope

**In:**
- **Shapes** in `shuyi_3d::collide`:
  - `Sphere { centre, radius }`, `Capsule { a, b, radius }`,
    `Aabb3 { min, max }`;
  - `TriMesh`: static triangles in world space (placed with an
    `FxTransform` when built), each with its normal and box, from
    triangle lists (and from a `Mesh` if 3102 is done);
  - `Heightfield { origin, cell, w, h, heights: Vec<Option<Fx12>> }`,
    built from a mesh at load time by casting a ray down at each grid
    point; a point with no hit is a hole.
- **`World3`**: static colliders and moving shapes, in an `Arena`
  (1119), with a uniform grid of cells for broad phase. The cell size
  is data (default 2 m). Each cell lists triangles and colliders in
  ascending id order.
- **Narrow phase**, in integers (products in `i64`, sums in `i128`):
  sphere and capsule against triangles (closest point on a triangle),
  boxes, spheres and capsules.
- **Casts:**
  - `ray_cast(origin, dir, max) -> Option<Hit>` against every shape;
  - `shape_cast(shape, from, delta) -> Option<Hit>` for spheres and
    capsules: steps of at most half the radius until the first overlap,
    then 8 rounds of bisection. Documented, and exact under replay;
  - every query returns hits sorted by (distance, collider id, triangle
    index), so ties never depend on memory order.
- **The character mover**, `CharacterMover`, with settings in RON
  (`radius`, `height`, `step_height`, `max_slope: Angle16`, `skin`,
  `iterations` up to 4), through the format registry (2201) as asset
  kind `character_mover`:
  - `move_by(&mut self, world: &World3, delta: FxVec3) -> MoveResult {
    position, grounded, ground_normal, hits }`;
  - slide: cast the capsule along what is left of the move, stop a skin
    short of the hit, and project the rest onto the hit plane (or along
    the crease of two planes);
  - steps: when blocked low, try up by `step_height`, forward, then
    down;
  - slopes: a surface steeper than `max_slope` is a wall;
  - ground: a short cast down after each move;
  - other movers are obstacles, taken in id order. There is no pushing.
  - Gravity and speed are the game's: the mover only takes a delta.
- **Triggers**: `Trigger3 { shape, id }`, with enter and leave events
  per step, sorted by (trigger id, body id).
- **A text dump** of a `World3` and of a `MoveResult`, one line each.

**Out (do not do):**
- rigid bodies and `rapier3d` (a module of its own when an example needs
  it, ADR-0009 §7);
- rotated boxes and moving platforms;
- pushing other movers;
- cameras (3106).

## Implementation steps

1. Write the shapes, the narrow phase and ray casts, with unit tests on
   hand-placed shapes.
2. Write `TriMesh`, `Heightfield` and the grid broad phase. Test that a
   query over the grid returns the same hits as a brute-force query.
3. Write `shape_cast`.
4. Write the mover and its RON settings, with a fixture room: a floor,
   a wall, a step of 0.2 m and one of 0.5 m, a slope of 30° and one of
   60°.
5. Write triggers and the dumps.
6. Add the property tests.

## Acceptance criteria

- [ ] The grid broad phase returns exactly the brute-force hits, in the
      same order (property test, random scenes).
- [ ] A mover never ends overlapping a static shape by more than its
      skin, from any start outside them (property test, 10 000 cases).
- [ ] With nothing in the way, moving by `a` then `b` ends where moving
      by `a + b` ends (property test).
- [ ] In the fixture room, with `step_height` 0.3 m and `max_slope` 45°,
      the mover climbs the 0.2 m step and the 30° slope, and is stopped
      by the 0.5 m step and the 60° slope (unit tests).
- [ ] The same moves in the same world give the same dump on native and
      `wasm32`.
- [ ] Each bad RON fixture gives an error naming the file and line.
- [ ] `clippy::float_arithmetic` finds nothing in `shuyi_3d::collide`.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each shape pair, casts, the fixture room, triggers.
- Property: broad phase against brute force; no overlap; additivity;
  determinism.
- Fixture: the dump on native and `wasm32`; the RON errors.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
