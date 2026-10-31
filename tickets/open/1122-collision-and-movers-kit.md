---
id: "1122"
title: "Parked: Collision and movement kit: tile collision, boxes, pixel-stepping movers, triggers, a top-down controller"
type: feature
milestone: "Engine: real time"
model: reasoning
effort: high
status: blocked
blocked_by: ["1119", "1120"]
nick_input: none
completed:
---

# 1122 — Collision and movement kit

> **Parked** (Nick, 2026-10-14: "document the parked (i.e. after MVP/parity) tickets for networking, styles for later"; 2026-10-20: "unless these real-time work tickets overlap with visions of shuyi they should be parked until visions of shuyi is complete"): after Visions of Shuyi launches, not only after the MVP (`docs/decisions.md`, 2026-10-20). Unparked early only if the game needs it: its ticket 0060 says the army walks around freely (then 1119–1125 become game work), or it asks for a piece of this through its `engine-change` skill, which is then split out and unparked alone. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

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
  - `Trigger` enter and leave events;
  - `CollisionLayers { layer: u32, mask: u32 }`, bits: the layers a
    thing is in and the layers it meets. `CollisionLayers::DEFAULT` is
    layer `1`, mask `u32::MAX`. Every box the broad phase holds, every
    `Trigger`, every `Mover`, and each `TileCollision` (one value for
    all its tiles) carry one. Two things meet when
    `a.layer & b.mask != 0 && b.layer & a.mask != 0`, the rule of
    rapier's `InteractionGroups` (1128). A mover stops only at what it
    meets, a trigger reports only what it meets, and the broad phase
    pairs only what meets. A game that sets none gets the same
    behaviour as with no layers;
  - every box in the broad phase has an id that a mover's hit and a
    trigger's event name, and the broad phase can list the boxes of one
    cell in id order (2124's queries walk its cells).
- `shuyi_core::controllers::top_down`: 8-way and analog movement with
  acceleration, top speed and friction, all as data (RON). It works with
  the `TopDown` perspective marker only (ADR-0009 §3, markers from
  1120), and a `compile_fail` doc test shows it can't be used with
  another one.
- Property tests:
  - a mover never ends inside a solid tile or box whose layers it
    meets;
  - moving by `a` then by `b` equals moving by `a + b` when nothing is
    in the way;
  - the same inputs give the same positions;
  - with every layer `DEFAULT`, the same moves give the same positions
    and hits as with layers ignored;
  - a mover never stops at a box or tile whose layers it doesn't meet,
    and a trigger never reports one.
- The new-game wizard (ADR-0013 §3–§4). Set the `top-down` answer of
  the perspective concern in `docs/modules.ron` (2003 wrote it parked,
  with 1120 and 1122) to ready, with no tickets. If 2503 is done, add
  the part `stage-top-down` to the built-in template: a stage with the
  top-down controller, generic over `TopDown`, and its test. If 2503
  isn't done, 2503 adds it.
- **Either order with 1124:** if 1124 is done, add a function beside
  its `TileMap` (in `shuyi-assets`, which reaches `shuyi_core`; the core
  can't reach it) that builds a `TileCollision` from a named tile layer
  and a rule per tile id, tested on 1124's Tiled fixture. If 1124 isn't
  done, 1124 adds it.
- **Either order with 2231:** the `movers` answer of the bodies concern
  in `docs/modules.ron` (2003 wrote it parked) is this kit's movers and
  2231's clips. If 2231 is done, set it to ready; if not, nothing to do
  (2231 sets it).
- **Either order with 2122:** 2122 adds a second dot to 1119's tick
  example, patrolling a curve, beside the dot that walks this kit's
  room. If 2122 is done, keep that patrol and its snapshot as they are
  when the room goes in (a follower meets no wall); if not, nothing to
  do (2122 puts its patrol in the room).
- **From 1134 (done):** its `shuyi-stage-field` skill
  (`skills/shuyi-stage-field/SKILL.md`: a tick stage, movers,
  collision, camera) has no worked example until 1119, 1120 and this
  ticket are done, and this ticket lands last of the three. Write that
  skill's worked example, `skills/shuyi-stage-field/example.ron` (a
  script of contract operations that CI's `skills` job runs on a fresh
  `shuyi new` game, as it runs the pack's others), check the skill's
  text against the kit and the top-down controller as built, and
  remove its `waits_for` and the note that it waits. `check-skills`
  passes.
- **Either order with 2728** (investigation hotspots, parked): if 2728
  is done, nothing to do: world hotspots on this ticket's `Trigger`s
  are a parked follow-up blocked by this ticket, which 2728's
  Completion notes name; if not, nothing to do (2728 builds them on
  this ticket's `Trigger`s).

**Out (do not do):** slopes; the platformer controller (1127); rigid
bodies (1128); queries that move nothing: segment casts, point and
overlap queries, and mover probes (2124); code over a store. The kit is
plain values and pure functions that name neither `shuyi_core::arena`
(`Arena<T>`, `Id<T>`) nor `shuyi-ecs`'s `World` or `Entity` (the kit
rule of 2904's ADR). The broad phase gives each box its own id (the
`BoxId` 2124 reads), `Trigger` events name boxes by that id, and a game
maps it to its things in whichever store it uses.

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
