---
id: "2319"
title: "Parked: Skeletons in the debug view: procedural bodies' points and rules, limbs, targets and footholds"
type: feature
milestone: "Engine: procedural animation"
model: sonnet-5
effort: medium
status: blocked
blocked_by: ["2315", "2318", "2701", "2702"]
nick_input: none
completed:
---

# 2319 — Skeletons in the debug view: bodies, limbs, footholds

> **Parked** (Nick, 2026-10-16: "this will be very late addition since it won't tie into visions of shuyi game"): a follow-up of the parked procedural animation module (2701, 2702), so it starts after both are unparked and done, and after the debug view's world gizmos (2315, 2318). Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Nick, 2026-10-19 (`docs/decisions.md`): see "skeletons in realtime
(debug mode)". In a 2D engine with no bones, the skeletons are the
procedural animation module's (ADR-0010):

- 2701's `Body`: points with radii, joined by the rules `Distance`,
  `Bend` and `Pin`;
- 2702's `Limb` chains and their reach targets;
- 2702's `Gait`, which plants feet on footholds it finds.

When a creature's leg flails or a foot grabs the wrong tile, a
developer needs to see the points, the rules, the targets and the
footholds over the drawn creature. Rain World's developer tools draw
exactly this.

This ticket adds that as a `skeleton` channel. The helpers go in
`shuyi_procanim`, behind its `view` feature (ADR-0009 §4: a module
brings its own pieces). They produce 2315's gizmos in world pixels
through 2318's `push_world`. 3D skeletons are 2320.

## Nick input

None.

## Scope

**In:**
- **`shuyi_procanim::view::debug`**, with these helpers:
  - `body_gizmos(&Body, camera, layer, &mut Gizmos)`:
    - each point as a circle of its radius (`Bone`);
    - each `Distance` rule as a line (`Bone`), a rope that is slack
      as a dashed line (every other 2-pixel run);
    - each `Pin` as a cross (`Solid`);
    - each `Bend` as a short arc mark at its middle point (`Note`);
  - `rig_gizmos(&Rig, camera, layer, &mut Gizmos)`:
    - each limb's joints as lines (`Bone`);
    - its reach target as a cross (`Note`);
    - a target out of reach as a line from the end to the target
      (`Note`);
  - `gait_gizmos(&Rig, &TileCollision, camera, layer, &mut Gizmos)`:
    - each planted foot's tile as an outline (`Solid`);
    - each moving foot's chosen foothold as a cross, with its arc as a
      line (`Path`);
    - each foot's rest place as a small cross (`Note`);
    - a hanging foot as a label `hang` (`Note`).
- **Channels:** `skeleton` (bodies and limbs) and `footholds` (gaits),
  so each can be on alone.
- The cave example (2704) calls them, if it is done. If it isn't, use
  2702's test creature in an integration test, and 2704 calls the
  helpers when it lands.
- The `shuyi-procanim` README gains a short section on these channels.

**Out (do not do):**
- 3D skeletons from glTF (2320);
- editing creatures from the view (the editor, 2506–2508; creatures
  stay RON files, edited as text or with hot reload);
- any change to `Body`, `Rig` or `Gait` beyond read-only accessors
  the helpers need.

## Implementation steps

1. Read ADR-0010, 2315's ADR, 2318's `push_world`, and 2701's and
   2702's code.
2. Add read-only accessors where the helpers need them (rule kinds,
   foot states).
3. Write `view/debug.rs` and its tests on the test creature.
4. Register the two channels, and call them from the example or test.
5. Run the gates.

## Acceptance criteria

- [ ] With `skeleton` on, the test creature's snapshot lists a circle
      per point, a line per distance rule, and a cross per pin and per
      reach target.
- [ ] With `footholds` on, a planted foot's tile and a moving foot's
      foothold are in the snapshot.
- [ ] The simulation hash and the `Rig` hash are the same with the
      channels on and off.
- [ ] If 2704 is done, `frame-png --debug skeleton,footholds` on the
      cave example is reviewed, and its snapshot is committed. If not,
      2704 does it.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each helper on a three-point body with one of each rule, and on
  a two-bone limb whose target is in reach and out of reach.
- Property: the number of gizmos equals the body's points plus its
  rules, for any body made from RON.
- Snapshot / integration: the test creature mid-step with both
  channels on.

## Completion notes

*(Filled in by the session that completes the ticket.)*
