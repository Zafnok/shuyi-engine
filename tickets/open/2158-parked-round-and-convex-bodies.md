---
id: "2158"
title: "Parked: Circle, capsule and convex polygon bodies in fixed point: overlaps with contact normals, casts for every pair, and random points inside each shape"
type: feature
milestone: "Engine: real time"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: blocked
blocked_by: ["2124", "2139"]
nick_input: none
completed:
---

# 2158 — Round and convex bodies, and random points in shapes

> **Parked** (Nick, 2026-10-20: "unless these real-time work tickets overlap with visions of shuyi they should be parked until visions of shuyi is complete"): a follow-up of the parked collision queries (2124) and fixed-point numbers (2139), so it starts only after they are unparked and done. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

1122 and 2124 collide boxes against tiles and boxes, and 2124 asks
overlap questions with circles. Its **Out** lists "slopes, rotated
boxes, polygons, and circles as bodies" with no ticket, and 2756 keeps
hurtboxes as boxes for the same reason. 1128 offers rapier, which works
in floats behind its own determinism. The Bevy coverage audit of
2026-11-28 (`docs/ROADMAP.md`, *Bevy's game coverage*) found the games
this leaves out: bullet hell with round hitboxes, pool, pinball and air
hockey, top-down shooters with capsule units, and sloped convex pieces
of terrain.

Bevy has the shapes and their tests in `bevy_shape`
(`crates/bevy_shape/src/dim2.rs`: `Circle`, `Capsule2d`,
`ConvexPolygon`; `bounding/bounded2d`, `bounding/raycast2d.rs`;
`examples/math/bounding_2d.rs`), and it samples random points inside
and on the edge of each one (`crates/bevy_shape/src/sampling/`,
`examples/math/random_sampling.rs`), which games use for spawn areas,
scatter and shaped particle emitters. `crates/core/src/rng.rs` has
shuffles and weighted picks only, and 2103's emitter starts from a
point.

## Nick input

None.

## Scope

**In:**
- **Shapes**, in 2124's `shuyi_core::collide`: `Shape { Box, Circle,
  Capsule, Convex(ConvexId) }`. A convex polygon is checked when loaded
  (counter-clockwise, convex, at most 8 points) and fails with the
  file and line otherwise.
- **Pairs:** `overlap(a, b) -> Option<Contact { normal: FxVec2, depth:
  Fx }>` for every pair, by separating axes in `i64`, circle against
  polygon by the closest feature; `cast(shape, motion, against)` by
  conservative advancement in a fixed number of iterations, ties broken
  by id as 2124 breaks them.
- **Into the kit:** the new shapes sit in 2124's broad phase and layer
  filters; a top-down mover slides along the contact normal.
- **Random points:** a `sample` module over `RandomSource`:
  `in_disc`, `in_ring` (by area, through an integer square root),
  `in_rect`, `in_triangle`, `in_convex` (area-weighted triangle pick),
  `on_circle`, `on_rect_edge`, `angle`. Each takes a fixed number of
  draws, so a replay's stream stays aligned.
- `Contact` and `ConvexError` follow ADR-0039.

**Out (do not do):**
- concave polygons (a game splits them into convex pieces);
- rigid bodies, mass and restitution (1128, parked);
- 2103's emitter shapes: **Either order with 2103:** if 2103 is done,
  give `EmitterDef` a `shape: SpawnShape` (point, disc, ring, rect,
  line) through this module; if not, nothing to do (2103 takes its
  spawn shapes from this module).

**Estimated size:** about 1,400 lines (code 650, tests 600, docs 150).

**Split:** past 1,000 lines, split off random points (the `sample`
module and the emitter shape) into a new ticket.

## Implementation steps

1. Read 1122's and 2124's code and 2139's numbers.
2. Add the shapes and the convex check.
3. Write `overlap` for each pair; then `cast`.
4. Add them to the broad phase and the top-down mover.
5. Write `sample` with golden tables.
6. Add the emitter shape as the clause says.
7. Write the docs page: a pool table and a ring of spawn points.

## Acceptance criteria

- [ ] Every pair's `overlap` agrees with a brute-force test of pixel
  centres on random shapes (property test).
- [ ] A circle cast at a polygon stops within one unit of the true
  contact, in the fixed number of iterations.
- [ ] A concave or clockwise polygon fails to load with its file and
  line.
- [ ] `in_ring` points are uniform by area (a chi-square check over
  fixed seeds), and every sampler takes the same number of draws per
  call.
- [ ] Golden tables of overlaps and samples are equal on wasm32.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] Guides in `docs/guides/` updated, if this adds a direction, a module or a move (ADR-0009 §5).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each pair, each sampler, the convex check.
- Property: overlap against the pixel test; casts never pass through.
- Snapshot / integration: golden tables on wasm32.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
