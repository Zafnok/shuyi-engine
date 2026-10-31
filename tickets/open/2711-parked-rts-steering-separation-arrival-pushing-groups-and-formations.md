---
id: "2711"
title: "Parked: RTS steering: separation, arrival, pushing with priorities, groups and formations"
type: feature
milestone: "Engine: real-time strategy"
model: opus-5.5 / gpt-6.1-sol
effort: xhigh
status: blocked
blocked_by: ["2708", "2710"]
nick_input: none
completed:
---

# 2711 — RTS steering: separation, arrival, pushing, groups, formations

> **Parked** (Nick, 2026-10-18: "make tickets parked for after visions of shuyi launches for all of these"): after Visions of Shuyi launches, which is later than the MVP in `docs/ROADMAP.md`. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

[ADR-0015](../../docs/adr/0015-real-time-strategy-at-scale-command-sims-columns-flow-fields-and-ai-in-layers.md)
§6. After 2710, units follow flow fields but pass through each other and
pile onto one point at the goal. Steering (Craig Reynolds, *Steering
Behaviors For Autonomous Characters*, 1999) makes them spread out, pass
each other and stop in an area. It is a weighted sum of small forces, in
`Fx`, read from the spatial grid (2708).

**Read the last state, write the next** (ADR-0015 §3) holds here too:
each unit computes its own velocity and its own push from its
neighbours' last positions, and writes only its own row. No unit moves
another. That keeps the result independent of the order units are
visited in, and ready for 2717.

## Nick input

None.

## Scope

**In:**
- **`Fx` roots:** steering needs `FxVec2::length` and `normalize`.
  **Either order with 2701:** if 2701 is done, use its `Fx::sqrt`,
  `FxVec2::length`, `length_squared` and `normalize`. If not, use the
  ones already in `shuyi_core::fx` (2106 and 2122 may have added
  `Fx::sqrt` and `FxVec2::length`), add the rest here exactly as
  2701's scope describes (floor of the exact root through `i64`; zero
  stays zero), with its property test, and note in 2701 that they
  exist.
- **Settings per kind**, a `steer` block in `*.unit.ron` with defaults:
  `separation_radius`, `separation_weight`, `flow_weight`,
  `arrival_radius`, `push_iterations` (the last one in `RtsSettings`).
- **`shuyi_rts::steer`**, each step, per unit, in `Fx`:
  - **the wanted direction:** the flow field's direction (2710), the
    straight line, or the unit's formation slot;
  - **separation:** for each neighbour within `separation_radius` (a
    `within_radius` query on 2708's grid), a push away, stronger the
    closer it is: `(r − d) / r` along the line between them. Two units
    on one point push apart along a fixed direction chosen from their
    ids, never a random one;
  - **arrival:** inside `arrival_radius` of the goal, speed falls in
    proportion to the distance;
  - the velocity is the weighted sum, capped at the kind's speed (and at
    the group's speed, below).
- **Groups:**
  - the units of one move command form a `Group` (`GroupId`, members,
    leader = the lowest `UnitId`, goal, optional formation);
  - a group moves at its slowest member's speed;
  - **the arrival area:** a circle around the goal whose area is the
    members' total area divided by a packing setting (default 0.6, as
    an `Fx`). A member stops when it is inside the area and touches a
    stopped member of its group, or reaches the goal point. So the
    first ones stop at the goal and the rest fill around them;
  - a group ends when every member has stopped or taken another order.
- **Formations:**
  - `*.formation.ron`: a name and slots, each an offset from the leader
    in `Fx`, turned with the leader's facing through `Fx`'s sine and
    cosine (1119's, from 2335's `shuyi_core::angle` table);
  - members take slots in a fixed order: by kind index, then by
    `UnitId`;
  - `Move` and `AttackMove` gain `formation: Option<String>`, with a
    serde default, so old records still load;
  - the format (ADR-0007): `formation`, extension `.formation.ron`, with
    a fuzz-style test and a determinism fixture (each fixture's decoded
    hash as a row of `shuyi-rts`'s golden table, 2116's
    `shuyi_assets::golden`, which 2707 runs on wasm32 in CI's
    `determinism` job).
- **Pushing** (ADR-0015 §6), after moving, for `push_iterations` rounds
  (default 2):
  - each unit sums the overlaps with its neighbours at the new positions
    and moves itself out by its share;
  - its share: an idle unit takes all of an overlap with a moving one;
    a unit takes all of an overlap with an enemy if it overlaps an ally
    too, so allies make way first; otherwise the two take half each;
  - a push never moves a unit into a wall cell of its class; it stops at
    the edge.
- **The tolerance**: a setting (default a quarter of the smaller
  radius). Units that start a step apart never end it overlapping more
  than that.

**Out (do not do):**
- RVO2, ORCA or any velocity obstacles (ADR-0015 §6; a later ticket if a
  game needs them);
- choosing targets, and fighting (2712);
- squads and their decisions (2714);
- threads (2717).

## Implementation steps

1. Add the `Fx` roots, or reuse 2701's.
2. Write separation and arrival over the spatial grid, reading last
   positions and writing next ones.
3. Write `Group`, its speed and the arrival area.
4. Write formations, the format and the command field.
5. Write pushing with priorities and the wall rule.
6. Write the README's steering section, with the settings and what each
   costs.

## Acceptance criteria

- [ ] From random starts with no overlaps and random move orders, no two
      units end any of 600 steps overlapping more than the tolerance
      (property test).
- [ ] 1 000 units ordered to one point all stop within 1 200 steps,
      inside a circle 1.5 times the arrival area's radius, with zero
      velocity (integration test).
- [ ] A group moves at its slowest member's speed (unit test).
- [ ] Members of a formation move take their slots in the documented
      order, and reach them on open ground (unit test).
- [ ] An idle unit yields to a moving one; a unit between an ally and an
      enemy moves before the enemy does (unit tests).
- [ ] A push never leaves a unit in a wall cell (property test).
- [ ] Visiting the rows in reverse order gives the same hashes (a test
      build flag reverses the passes), which proves no unit writes
      another's row.
- [ ] Two replays of a fixture give the same hash list; old records with
      no `formation` field load.
- [ ] Each formation fixture decodes to its row in `shuyi-rts`'s golden
      table (2116's `shuyi_assets::golden`), checked on x64, ARM64 and
      wasm32 by CI's `test` and `determinism` jobs; arbitrary bytes
      never panic.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: separation between two units; two units on one point; arrival
  slowing; group speed; formation slots; push shares.
- Property: overlap tolerance; walls; order independence.
- Integration: a thousand units to one point.
- Fuzz-style and fixture: the `formation` format.

## Completion notes
