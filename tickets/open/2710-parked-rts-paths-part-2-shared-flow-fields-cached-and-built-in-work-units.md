---
id: "2710"
title: "Parked: RTS paths, part 2: flow fields shared by every unit going to one goal, cached and built in work units"
type: feature
milestone: "Engine: real-time strategy"
model: opus-5.5 / gpt-6.1-sol
effort: xhigh
status: blocked
blocked_by: ["2709", "2117", "3009"]
nick_input: none
completed:
---

# 2710 — RTS paths, part 2: shared flow fields, cached and budgeted

> **Parked** (Nick, 2026-10-18: "make tickets parked for after visions of shuyi launches for all of these"): after Visions of Shuyi launches, which is later than the MVP in `docs/ROADMAP.md`. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

[ADR-0015](../../docs/adr/0015-real-time-strategy-at-scale-command-sims-columns-flow-fields-and-ai-in-layers.md)
§5, the flow fields (Elijah Emerson, *Crowd Pathfinding and Steering
Using Flow Field Tiles*, Game AI Pro, 2013). A flow field costs O(map)
once per goal, and every unit going there shares it, so 300 units
ordered to one place cost one field, not 300 paths. It is built only
over the sectors on the route 2709 finds.

Building a field is the heaviest work in the module, so it is
budgeted: requests wait in a queue and are served in work units, a
fixed number per step, never by time
([ADR-0008](../../docs/adr/0008-export-targets-profiles-as-data-platform-backends-controls-that-fit-one-bit-and-budgets.md)
§6). The builder implements 3009's `Think`.

The number of work units per step is a simulation setting, the same on
every device. It must not come from the target profile (3009's
`units_per_frame`): a field ready at a different step on a slower
device would change the game, and replays and lockstep would break.

## Nick input

None.

## Scope

**In:**
- **`shuyi_rts::paths::FlowField`:**
  - its key: `(goal: CellPos, class, map_version)`;
  - the sectors it covers, from a `Route` (2709);
  - an **integration field**: per covered cell, the cost to the goal
    (`u32`, `u32::MAX` unreached), by Dijkstra from the goal over the
    covered cells only, with 2709's move costs. It is the engine's
    flow field, not a second Dijkstra: `shuyi_core::path::flow_field_on`
    (2117, a blocker) with `Topology::Square8(Diagonals::OCTILE)`, the
    eight steps and corner rule 2709 takes from 2117 too. "Covered cells
    only" is its cost function answering `None` outside the covered
    sectors. Its queue is ordered by `(cost, y, x)`, which is (cost,
    cell index) for cells numbered row by row, so ties are fixed;
  - **what the core lacks, added there:** a search that stops after a
    number of tiles expanded and resumes on the next call. Add it to
    `shuyi_core::path` as a resumable form of the one private search
    (a state that `step(n)` advances by at most `n` expansions and
    that says when it is done), and make `flow_field_on` run it to the
    end, so there is still one Dijkstra. The builder below holds that
    state. Additive in `shuyi-core`, with a test that a field built in
    slices equals `flow_field_on`'s;
  - a **direction field**: per covered cell, one of 8 directions toward
    the cheapest neighbour, or none at the goal and on walls. Ties go in
    a fixed order (N, NE, E, SE, S, SW, W, NW). No diagonal cuts a
    corner (2709's rule).
- **Requests and extensions:**
  - a move order (`Move`, `AttackMove`, `Patrol`, and `Attack` toward a
    far target) asks for the field of its goal cell and each unit's
    movement class. Units of one command from several sectors ask for
    one field over the union of their routes;
  - a unit in a sector its field doesn't cover asks for an extension:
    the field is rebuilt over the larger set of sectors, under the same
    key;
  - requests wait in a `FlowRequests` queue in the order they were made
    (the order commands were applied, then row order). A request
    already queued or cached is not added twice.
- **The builder** implements `Think` (3009): `Decision = FlowField`, a
  work unit = one cell expanded. Each step `RtsSim` gives it
  `RtsSettings::flow_cells_per_step` units (default 4 096; 2718 tunes
  it). When a field is done, it goes in the cache and the next request
  starts in the same step with the units left. 3009's `Progress::Done`
  carries no count, so the builder counts the cells it expanded in the
  step (a `used()` on the builder), and `RtsSim` gives the next request
  `flow_cells_per_step` minus that.
- **The cache, `FlowCache`:**
  - limits in fields and in cells (both settings);
  - least recently used out first. "Used" is the step a unit last read
    it; ties go to the lower key;
  - a field whose `map_version` is old is dropped when looked up, and
    asked for again.
- **Moving units:**
  - a unit whose field is ready reads its cell's direction and moves
    that way at its kind's speed. In the goal cell, it heads straight to
    the exact goal point;
  - a unit whose field isn't ready heads straight for the goal until it
    is (ADR-0015 §5);
  - air classes (no walls) always go straight; the README says when a
    lone far unit may use 2709's waypoints instead, and why.
- **A text dump** of a field: the integration costs and the directions
  as a grid of characters, for tests and agents.

**Out (do not do):**
- separation, arrival areas, pushing, groups and formations (2711);
- choosing targets and chasing them (2712);
- threads (2717), and any limit in time;
- RVO or ORCA (ADR-0015 §6).

## Implementation steps

1. Add the resumable search to `shuyi_core::path` under
   `flow_field_on`, with its slice test. Then write `FlowField` over
   it, and its directions over a set of sectors, as a resumable state
   that `Think::step` advances.
2. Write `FlowRequests`, extensions and `FlowCache`.
3. Wire requests into `RtsSim`'s command phase and the builder into the
   step, with `flow_cells_per_step`.
4. Switch unit movement to fields, keeping the straight line while
   waiting.
5. Write the README's paths section.

## Acceptance criteria

- [ ] Building one field in slices of 1, 7 and unlimited units gives the
      same field, equal to `flow_field_on`'s over the same covered cells
      (property test with random maps and random slice sizes).
- [ ] `shuyi_rts::paths` has no Dijkstra of its own for fields
      (`flow_field_on` and the resumable search are the only ones), and
      `shuyi_core::path`'s existing tests pass unchanged.
- [ ] From every reachable covered cell, following the directions
      reaches the goal in at most the number of covered cells, and never
      enters a wall (property test on random maps).
- [ ] The cache drops the least recently used field first, and drops
      fields with an old `map_version` (unit tests).
- [ ] A unit heads straight while its field is queued, then follows the
      field from the step it is ready (unit test).
- [ ] 300 units of one class ordered to one cell make one field
      (integration test counts builds).
- [ ] Two replays of a fixture give the same hash list.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the direction tie order; extensions; cache eviction and
  versions; the straight-line wait.
- Property: slice-size independence; following the field reaches the
  goal.
- Integration: one field shared by an army; replay determinism.
- Snapshot: the text dump of a fixture field.

## Completion notes
