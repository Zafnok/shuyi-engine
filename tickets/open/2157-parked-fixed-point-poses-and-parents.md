---
id: "2157"
title: "Parked: Poses and parents in fixed point: Pose2 composed and inverted, and a parent table resolved in order, for things carried by other things"
type: feature
milestone: "Engine: real time"
model: opus-5.5 / gpt-6.1-sol
effort: medium
status: blocked
blocked_by: ["2139", "2335"]
nick_input: none
completed:
---

# 2157 — Poses and parents in fixed point

> **Parked** (Nick, 2026-10-20: "unless these real-time work tickets overlap with visions of shuyi they should be parked until visions of shuyi is complete"): free placement is real-time work, and this builds on the parked fixed-point numbers (2139), so it starts only after Nick unparks them. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

In a free-placement game things ride on other things: a turret on a
tank, a rider on a mount, a held weapon, a moon around a planet. Each
child has a position, angle and scale relative to its parent, and the
game needs it in world space each step, and the other way round (world
to local, for an aim). Bevy does this with `Transform`,
`GlobalTransform` and propagation down the hierarchy
(`crates/bevy_transform/src/`; `crates/bevy_math/src/isometry.rs`;
`examples/transforms/*.rs`). The Bevy coverage audit of 2026-11-28
(`docs/ROADMAP.md`, *Bevy's game coverage*) found no ticket for it:
2139 gives vectors and angles but no composed pose, and 2105 resolves
bones inside a rig only.

Here it is a pure table in `shuyi-core`, with no ECS and no change
detection: the simulation calls `resolve` each step, and a view reads
the result. 2105's bones and 2701's bodies may reuse it.

## Nick input

None.

## Scope

**In:**
- `Pose2 { pos: FxVec2, angle: Angle, scale: Fx }` in `shuyi_core`,
  beside 2139's numbers, with `then`, `inverse`, `apply(FxVec2)` and
  `apply_inverse`, worked in `i64` inside so composition never
  overflows within the documented range; angles through 2335's
  `Angle` and sine table.
- `Parents<Id>`: a plain table of (child, parent, local `Pose2`),
  checked acyclic with parents before children when built, and
  `resolve(&roots, out: &mut [Pose2])` in table order,
  deterministic and allocation-free.
- serde on both, and both in the world's hash when a game stores them.
- `Pose2` is a plain `Copy` struct with public fields; `ParentsError` is
  `#[non_exhaustive]` (ADR-0039).

**Out (do not do):**
- an ECS transform hierarchy: 2904's ADR says whether `shuyi-ecs` uses
  this table through 2153's relations;
- 3D poses (3101, parked);
- interpolating poses between steps for drawing (2361, parked).

## Implementation steps

1. Read 2139's and 2335's code.
2. Write `Pose2` and its operations with doc examples.
3. Write `Parents` with its build check and `resolve`.
4. Write the docs page: a turret on a tank.

## Acceptance criteria

- [ ] `a.then(a.inverse())` is the identity within one unit, for any
  pose in the documented range (property test).
- [ ] `resolve` equals a brute-force fold of each chain.
- [ ] A cycle or a child listed before its parent fails the build with
  `ParentsError`.
- [ ] Golden values of a three-level chain are equal on native and
  wasm32.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] Guides in `docs/guides/` updated, if this adds a direction, a module or a move (ADR-0009 §5).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each operation, the build check.
- Property: inverse and composition laws; resolve against the fold.
- Snapshot / integration: a golden row on wasm32.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
