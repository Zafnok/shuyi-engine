---
id: "2705"
title: "Parked: Research: units at scale: an arena of structs, columns, or an ECS at 1 000, 5 000 and 10 000 units"
type: research
milestone: "Engine: real-time strategy"
model: opus-5.5
effort: high
status: blocked
blocked_by: ["1119"]
nick_input: none
completed:
---

# 2705 — Research: units at scale, an arena, columns or an ECS

> **Parked** (Nick, 2026-10-18: "make tickets parked for after visions of shuyi launches for all of these"): after Visions of Shuyi launches, which is later than the MVP in `docs/ROADMAP.md`. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Nick, 2026-10-18 (`docs/decisions.md`): "I wanna be able to support high
unit count RTS games".
[ADR-0015](../../docs/adr/0015-real-time-strategy-at-scale-command-sims-columns-flow-fields-and-ai-in-layers.md)
§3 decides that the RTS module keeps its units in a table of columns
(`UnitTable`), not in an `Arena<T>` of structs. Its last bullet says this
ticket measures first.

[ADR-0003](../../docs/adr/0003-game-styles-stages-two-kinds-of-simulation-cameras-and-projections.md)
§4 left a spike open: if an example needs thousands of things, compare
`hecs` and `bevy_ecs` against the engine's rules (determinism, WASM size,
compile time, a readable dump). This ticket is that spike, at the scale
of an RTS.

It needs 1119 (`Fx`, `FxVec2`, `Arena<T>`, the core's seeded random
numbers and hash). The answer decides how 2707 is written.

## Nick input

None.

## Scope

**In:**
- **Four contenders**, each running the same workload:
  - **A:** `shuyi_core::arena::Arena<UnitStruct>` from 1119, one struct
    per unit;
  - **B:** a prototype column table as ADR-0015 §3 describes: one `Vec`
    per field, a generational id, an index from id to row, and removal
    by swapping the last row in. Write only what the workload needs;
  - **C:** `hecs`;
  - **D:** `bevy_ecs`, only if it builds for `wasm32-unknown-unknown`
    without `wasm-bindgen` (game ADR-0034). Check with
    `cargo tree --target wasm32-unknown-unknown -i wasm-bindgen` and a
    build. If it doesn't, record why and skip it.
  - Check each crate's license from its `Cargo.toml` and repository on
    the day (both are believed MIT OR Apache-2.0) and run `cargo deny
    check licenses` on the bench crate. Record the versions.
- **The workload**, the same for all four, in `Fx` only (no floats):
  - a 256 × 256 tile map, units placed by the core's seeded random
    numbers, half on each of two teams;
  - each step:
    1. **move:** each unit steers straight toward its own goal at its
       speed;
    2. **separation:** a naive uniform grid (a `Vec<Vec<u32>>` of cells
       is fine here; 2708 writes the real one) rebuilt each step; each
       unit sums a push away from neighbours inside a radius;
    3. **target pick:** each unit finds the nearest enemy within its
       range through the grid, ties broken by the unit's serial number;
  - every 10 steps, 1 % of the units are removed and as many spawned, so
    inserts and removals are measured too;
  - every unit has a `serial: u32` given at spawn, so the four contenders
    can be compared on equal terms whatever their ids look like;
  - sizes: 1 000, 5 000 and 10 000 units, 600 steps each.
- **What to measure**, per contender and size:
  1. **Step time, native**, release build: median and p99 over the 600
     steps, on one named machine (CPU, OS, toolchain). Wall time is fine
     here: this is research, not a gate (ADR-0008 §6).
  2. **Instructions per step:**
     - if 3008 is done, as a `.bench.ron` scenario through `shuyi bench`;
     - if not, with `iai-callgrind` (check its current name, version and
       license on the day) in a CI job or WSL, because valgrind doesn't
       run on Windows;
     - if neither is possible, a counted proxy: counters in the workload
       for field reads, field writes and neighbour checks. The report
       says which method was used.
  3. **WASM size added:** a release `cdylib` per contender for
     `wasm32-unknown-unknown` with the web build's profile, minus an
     empty baseline.
  4. **Compile time:** a clean `cargo build --release` of each
     contender's feature, three runs, the median, with
     `cargo build --timings` saved.
  5. **Peak heap**, through a counting `GlobalAlloc` in the bench binary.
  6. **Determinism:**
     - a hash of every unit, in serial order, after every step;
     - two native runs give the same 1 000-step hash list;
     - native and web give the same list (run the web build under
       `wasm32-wasip1` with `wasmtime`, as 1119 does for `Fx`);
     - for `hecs` and `bevy_ecs`, write down what it took to make the
       result independent of query and archetype order.
  7. **Readability of a text dump:** a dump of 10 units from each, put in
     the report. Judge: one line per unit, every field shown, in a fixed
     order; and how much code the dump took.
- **The report**, `docs/research/units-at-scale.md`:
  - a table per measure (contenders × sizes);
  - the machine, the versions, the licenses with where and when they
    were checked;
  - the exact commands to reproduce it;
  - the verdict, rule by rule.
- **The outcome**, one of two:
  - **ADR-0015 §3 holds:** the report says so, and 2707 starts as
    written;
  - **an ECS wins on every rule:** write a new ADR (`write-adr` skill)
    that supersedes ADR-0015 §3, and update 2707's scope in this PR.
- **Where the prototype code lives:** in a throwaway crate,
  `research/units-at-scale/`, listed under `exclude` in the root
  `Cargo.toml`, so `hecs` and `bevy_ecs` never enter the workspace's
  `Cargo.lock`. The last commit of the PR deletes the crate. The report
  links to the PR's commit before the deletion (GitHub keeps a PR's
  commits). 2707 writes `UnitTable` from scratch, with the report's
  lessons.

**Out (do not do):**
- the real `UnitTable` (2707) and the real spatial grid (2708);
- any change to `Arena<T>` or to other tick stages (ADR-0003 §4 stays
  the rule for small stages);
- parallel passes (2717);
- adding `hecs` or `bevy_ecs` to the workspace.

## Implementation steps

1. Write `research/units-at-scale/` with the workload as a generic
   function over a small `Units` trait (spawn, remove, iterate, read and
   write a field), one implementation per contender behind a feature.
2. Check that all contenders reach the same hash after 600 steps at
   1 000 units. Fix the workload until they do; until then the timings
   compare different work.
3. Run the measures, natively and for the web.
4. Write `docs/research/units-at-scale.md`.
5. Write the superseding ADR and update 2707, only if the verdict says
   so.
6. Delete the crate in the last commit.

## Acceptance criteria

- [ ] `docs/research/units-at-scale.md` has a table for each of the seven
      measures, with every contender (or the reason one was skipped), at
      1 000, 5 000 and 10 000 units where the measure depends on size.
- [ ] The report states which instruction-count method was used.
- [ ] All contenders end with the same hash after 600 steps at 1 000
      units, and the report shows it.
- [ ] Each contender's 1 000-step hash list is the same twice natively
      and on the web, or the report says why not.
- [ ] The licenses of `hecs` and `bevy_ecs` are recorded with their
      source and date.
- [ ] The verdict is written. If it goes against ADR-0015 §3, the new
      ADR exists and 2707 is updated.
- [ ] The workspace `Cargo.lock` has no `hecs` or `bevy_ecs`, and
      `research/units-at-scale/` is gone at the end of the PR.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Equivalence: the four contenders' hashes after 600 steps at 1 000
  units are equal.
- Determinism: native twice, and native against web, for each
  contender.
- No other tests: the code is deleted.

## Completion notes
