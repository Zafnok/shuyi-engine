---
id: "1122"
title: "Parked: Collision and movement kit: tile collision, boxes, pixel-stepping movers, triggers, a top-down controller"
type: feature
milestone: "Engine: real time"
model: opus-5.5
effort: high
status: blocked
blocked_by: ["1119"]
nick_input: none
completed:
---

# 1122 — Collision and movement kit

> **Parked** (Nick, 2026-10-14: "document the parked (i.e. after MVP/parity) tickets for networking, styles for later"): after the engine reaches parity with the game (the MVP in `docs/ROADMAP.md`). Unparked early only if the game's ticket 0060 says the army walks around freely (then 1119–1125 become game work). Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

ADR-0004 §4. Free movement and platformers need collision that is exact
under replay, so it uses `Fx` (1119), not floats. Movers step one pixel
at a time and stop at what they hit (the TowerFall and Celeste way). That
way they never tunnel through a wall and never overlap something they
then have to be pushed out of.

## Nick input

None.

## Scope

**In:**
- `shuyi_core::collide`:
  - `TileCollision` (solid, empty, one-way-up), built from a `Grid` and a
    rule per tile id;
  - `Aabb`;
  - a uniform-grid broad phase;
  - `Mover::move_x` / `move_y`, which keep the fractional remainder, step
    one pixel at a time, and return what they hit;
  - `Trigger` enter and leave events.
- `shuyi_core::controllers::top_down`: 8-way and analog movement with
  acceleration, top speed and friction, all as data (RON). It works with
  the `TopDown` perspective marker only (ADR-0009 §3, markers from
  1120), and a `compile_fail` doc test shows it can't be used with
  another one.
- Property tests:
  - a mover never ends inside a solid tile or box;
  - moving by `a` then by `b` equals moving by `a + b` when nothing is
    in the way;
  - the same inputs give the same positions.

**Out (do not do):** slopes; the platformer controller (1127); rigid
bodies (1128).

## Implementation steps

1. Write the types and the movers with unit tests on hand-drawn maps.
2. Add the property tests.
3. Use them in the tick example from 1119: the dot walks a little room.

## Acceptance criteria

- [ ] The property tests pass with 10,000 cases.
- [ ] The example room's replay hash is stable across runs and builds.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit and property, as above.

## Completion notes
