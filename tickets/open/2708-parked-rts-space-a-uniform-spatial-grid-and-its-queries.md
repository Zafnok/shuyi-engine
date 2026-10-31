---
id: "2708"
title: "Parked: RTS space: a uniform spatial grid rebuilt each step, and its queries"
type: feature
milestone: "Engine: real-time strategy"
model: reasoning
effort: high
status: blocked
blocked_by: ["2707"]
nick_input: none
completed:
---

# 2708 — RTS space: a uniform spatial grid and its queries

> **Parked** (Nick, 2026-10-18: "make tickets parked for after visions of shuyi launches for all of these"): after Visions of Shuyi launches, which is later than the MVP in `docs/ROADMAP.md`. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

[ADR-0015](../../docs/adr/0015-real-time-strategy-at-scale-command-sims-columns-flow-fields-and-ai-in-layers.md)
§4. With a thousand units, "who is near me" must not cost a pass over
every unit. Steering (2711), combat (2712), vision (2713) and the view's
selection (2716) all ask it. The ADR picks one uniform grid, rebuilt
every step by a counting sort: O(units), no allocation after the first
step, and a fixed order. Quadtrees and bounding-volume trees lose
(ADR-0015 §4).

It builds on 2707's `UnitTable` and `RtsSim`.

## Nick input

None.

## Scope

**In:**
- **`shuyi_rts::space::SpatialGrid`:**
  - the cell size in `Fx`: a setting in `RtsSettings`, by default the
    largest radius of any loaded kind times two;
  - the bounds: the map's size in world `Fx`. If 2709 is done, from its
    terrain grid; if not, from a new `RtsSettings::map_size`, which 2709
    replaces.
    A unit outside the bounds is put in the nearest edge cell, so it is
    still found;
  - storage: `cell_start: Vec<u32>` (one more than the cell count) and
    `entries: Vec<u32>` (rows);
  - `rebuild(&UnitTable)`: count units per cell, prefix sums, then
    scatter rows in row order. Inside a cell, rows are in row order;
  - the vectors keep their capacity, so a rebuild allocates only when
    the unit count or the map grows.
- **Queries**, each writing rows into a caller's `&mut Vec<u32>` (cleared
  first), so a pass allocates nothing:
  - `within_radius(center, radius, out)`;
  - `in_rect(rect, out)`;
  - `nearest(center, max_radius, filter: impl FnMut(u32) -> bool) ->
    Option<u32>`: scans rings of cells outward and stops once a ring is
    farther than the best found. Ties go to the lower row.
  - **The order is fixed:** cells in row-major order (y, then x), rows
    inside a cell in row order. The docs say so.
  - Distances are compared squared, in `i64` from `Fx`'s raw value, so
    they never overflow and never need a root.
- **`RtsSim` rebuilds the grid** once per step, after the commands and
  before the first pass, from the last positions. `sim.space()` gives
  read access.
- **A text dump** of the grid: one line per non-empty cell, `x y: rows`.

**Out (do not do):**
- using the grid for steering (2711), targets and area damage (2712),
  vision (2713) or selection (2716);
- quadtrees, bounding-volume trees, or a grid per team (ADR-0015 §4);
- parallel rebuilds: 2717 keeps the rebuild on one thread, and a
  parallel one is a later ticket if 2718 shows it matters.

## Implementation steps

1. Write `SpatialGrid` and `rebuild` in `crates/rts/src/space.rs`.
2. Write the three queries with their order documented.
3. Wire the rebuild into `RtsSim::step` and add the cell size setting.
4. Write the brute-force reference in the tests, the property tests and
   the allocation test.

## Acceptance criteria

- [ ] For random units and random queries, each query returns exactly
      the rows a brute-force scan returns, in the documented order
      (property test, 10 000 cases).
- [ ] `nearest` with a filter returns what a brute-force scan returns,
      lower row on a tie (property test).
- [ ] Units outside the map are found by queries that cover the nearest
      edge cell (unit test).
- [ ] After the first step, 600 steps of a 1 000-unit fixture make zero
      allocations in `rebuild` and the queries (an integration test with
      a counting `GlobalAlloc`).
- [ ] Two replays of a fixture give the same hash list with the grid in
      the step.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the counting sort on a hand-made table; edge cells; a ring scan
  stopping early.
- Property: each query against brute force; tie-breaks.
- Integration: the no-allocation test, in its own test binary so the
  counting allocator touches nothing else.

## Completion notes
