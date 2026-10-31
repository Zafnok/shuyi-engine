---
id: "2702"
title: "Parked: Procedural animation: limbs that reach (two-bone and FABRIK) and a gait that finds footholds on tiles"
type: feature
milestone: "Engine: procedural animation"
model: reasoning
effort: xhigh
status: blocked
blocked_by: ["2701"]
nick_input: none
completed:
---

# 2702 — Procedural animation: limbs and a gait

> **Parked** (Nick, 2026-10-16: "this will be very late addition since it won't tie into visions of shuyi game"): after the engine reaches parity with the game (the MVP in `docs/ROADMAP.md`) and after 1119 and 1122. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

[ADR-0010](../../docs/adr/0010-procedural-animation-bodies-limbs-and-strips.md)
§3–§4. Bodies exist (2701). This ticket adds what makes a creature look
alive on any terrain: limbs that reach for a point, and a gait that
plants feet on tile edges and moves them when they fall behind. It also
adds the look tier: the `Rig`, which steps after the simulation.

## Nick input

None.

## Scope

**In:**
- **`Limb`**: a chain of joints from an anchor (a point of a body) to an
  end, with segment lengths. Two ways to reach a target:
  - `reach_two_bone(anchor, target, bend_side)`: the exact solution,
    using `Angle::acos` from 2701. A target out of reach straightens the
    limb toward it;
  - `reach_fabrik(anchor, target, passes)`: for chains of any length,
    with a fixed number of passes.
- **`Gait`** (ADR-0010 §4):
  - per foot: a rest place relative to its anchor, a stride, a reach, a
    lift height, and the number of steps a move takes;
  - feet in named groups that take turns;
  - a grip search over `TileCollision`: solid tiles with an open
    neighbour, within reach of the anchor, scored by distance to the
    rest place, ties broken by the seeded RNG (seeded by the body's
    seed and the step), searched in a fixed order;
  - a planted foot holds its place, and a moving foot follows an arc
    over its steps;
  - with no grip in reach, the foot hangs: its chain becomes a body
    chain under gravity until a grip is found.
- **`Rig`**: the look tier (ADR-0010 §3). It holds limbs, gaits and
  look-only bodies (tails, antennae). `Rig::step(&body, &level)` runs
  after each simulation step. It has its own `hash()` and dump, apart
  from the simulation's.
- **RON**: limbs and gaits in the creature file of 2701, checked with
  file and line.
- **The README**: a section on limbs and the look tier.

**Out (do not do):**
- drawing (2703);
- the example game (2704);
- limbs that push the body. A game does that by reading planted feet and
  calling `push`; the README shows how.

## Implementation steps

1. Write `reach_two_bone` and `reach_fabrik`, with property tests.
2. Write the grip search, with tests on hand-drawn tile maps: a floor, a
   wall, a ceiling, and a gap too wide to reach.
3. Write `Gait` and `Rig`. A four-legged test creature, pushed by a
   scripted force, walks along a floor, up a wall and across a ceiling.
4. Add limbs and gaits to the RON, with error fixtures.
5. Write the README section.

## Acceptance criteria

- [ ] Every segment keeps its length within 1/256 px after a reach
      (property test).
- [ ] When the target is in reach, the end lands within 1 px of it
      (property test).
- [ ] A planted foot never moves while it stays within its stride
      (property test).
- [ ] No two feet of the same group are lifted at once (property test).
- [ ] The test creature reaches the end of the floor, wall and ceiling
      course in a fixed number of steps, with the same `Rig` hash list
      on every run.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Property: reach, planted feet, groups.
- Integration: the test creature's course, with snapshots of the `Rig`
  dump at three steps.

## Completion notes
