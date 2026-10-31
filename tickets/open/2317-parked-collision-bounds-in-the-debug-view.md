---
id: "2317"
title: "Parked: Collision bounds in the debug view: solid tiles, boxes, triggers and what each mover hit"
type: feature
milestone: "Engine: real time"
model: implementation
effort: medium
status: blocked
blocked_by: ["2315", "1122"]
nick_input: none
completed:
---

# 2317 — Collision bounds in the debug view

> **Parked** (Nick, 2026-10-14: "document the parked (i.e. after MVP/parity) tickets for networking, styles for later"): a follow-up of the parked collision kit (1122), so it starts after 1122 is unparked and done, and after the debug view (2315). Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Nick, 2026-10-19 (`docs/decisions.md`): see "collisions bounds ... in
realtime (debug mode)". 2315 built the debug view: gizmos as plain
data, channels turned on from the debug menu, and a read-only rule.
1122 built the collision kit in `shuyi_core::collide`:

- `TileCollision` (solid, empty, one-way-up);
- `Aabb`;
- a uniform-grid broad phase;
- `Mover::move_x` and `move_y`, which return what they hit;
- `Trigger` enter and leave events.

This ticket shows all of that as a `collision` channel.

`shuyi_core` has no gizmo types and must not get any (the core draws
nothing). So the helper lives in `shuyi_ui::debug::collide`. It reads
the core's types through `&` and turns them into gizmos.

## Nick input

None.

## Scope

**In:**
- **`shuyi_ui::debug::collide`**, with these helpers:
  - `tile_gizmos(&TileCollision, view, to_px, &mut Gizmos)`: an
    outline per solid tile in the view (`Solid`), and a line on the top
    edge of each one-way tile (`Solid`);
  - `box_gizmos(&[Aabb], to_px, &mut Gizmos)` (`Bounds`);
  - `trigger_gizmos(&[Trigger], to_px, &mut Gizmos)` (`Trigger`), with
    the trigger's name as a label when it has one;
  - `hit_gizmos(&MoveResult, to_px, &mut Gizmos)`: a cross on the
    contact point and a short line along the side that was hit
    (`Note`);
  - `cast_gizmos(from, to, hit: Option<&Hit>, to_px, &mut Gizmos)` for
    2124's casts: a `Line` from `from` to the hit point (or to `to`
    when nothing was hit), and at a hit a `Cross` and a short line
    along the side entered, all `Note`. **Either order with 2124:** if
    2124 is done, add it here; if not, 2124 adds it.

  `to_px` maps world pixels to console pixels:
  - if 2318 has landed, a call to its `world_to_px`, the function
    behind `push_world`;
  - before that, the identity, plus an offset the caller gives.
- **The broad phase**, as a channel of its own, `collision_grid`: the
  cells of the uniform grid in the view (`Bounds`), each cell with its
  box count as a label when the count is above 0. It is a separate
  channel because it is noisy.
- **Last hits:** `Mover` results are values, so the helper keeps no
  state. A stage that wants hits drawn keeps the last step's results
  next to its state. Show this in the tick example from 1119 and 1122:
  the dot's room.
- **The platformer**, if 1127 is done: its ground and wall probes as
  `Note` lines. If 1127 isn't done, 1127 calls the helpers in its
  example.
- Fixed-point to pixels: positions are `Fx`. Round to the nearest
  console pixel in one function (`debug::collide::px`), with a test,
  so every helper agrees. `px` and the `collision` channel are
  public: 2373 (parked) uses both from `shuyi-physics`.
- **Either order with 2106:** if 2106 is done, its `nav` helper
  (`shuyi_ui::debug::nav::gizmos`, if it is written yet) takes the
  caller's offset; round its points through `px` too. If not, nothing
  to do (2106 uses `to_px` and `px` from here).

**Out (do not do):**
- rigid-body shapes from 1128: 2373 (parked) draws them;
- 3D collision (2320);
- editing boxes or tiles (the editor, 2506–2508).

## Implementation steps

1. Read 2315's ADR, `shuyi_ui::debug::gizmo`, and 1122's
   `shuyi_core::collide` (and 2124's `collide::query`, if it is done).
2. Write `debug/collide.rs` and its tests on hand-drawn maps.
3. Register the `collision` and `collision_grid` channels, and call
   the helpers from the tick example's room.
4. Run the gates.

## Acceptance criteria

- [ ] With `collision` on, the example room's snapshot lists an outline
      for every solid tile in view, the dot's box, and a cross where it
      last hit a wall.
- [ ] The example's replay hash is the same with the channels on and
      off.
- [ ] `frame-png --debug collision` on the example is reviewed, and its
      snapshot is committed.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each helper on a 4 × 4 map, with a one-way tile and a trigger.
  `px` rounding at ±0.5. `cast_gizmos` with and without a hit, if 2124
  is done.
- Property: no gizmo falls outside the view's pixel bounds.
- Snapshot / integration: the example room with `collision` on, and
  with `collision_grid` on.

## Completion notes

*(Filled in by the session that completes the ticket.)*
