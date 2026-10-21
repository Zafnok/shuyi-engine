---
id: "2107"
title: "The shuyi-ecs module: entities, components as plain structs, queries, resources, change detection, commands and a deterministic schedule"
type: feature
milestone: After MVP
model: opus-5.5
effort: high
status: todo
blocked_by: ["2904", "2003", "2015"]
nick_input: none
completed:
---

# 2107 — The shuyi-ecs module

## Context

Nick, 2026-10-20 ([`docs/decisions.md`](../../docs/decisions.md)): an "ECS system
(perhaps optional)", as on Bevy's homepage. 2904 picks how it is built
(`bevy_ecs`, `hecs` with our own layers, or our own) and writes the ADR.
This ticket builds the module that ADR describes. **If 2904's ADR differs
from a line below, the ADR wins**, and 2904 updates this ticket.

It is a module (ADR-0009): off by default, a feature of the `shuyi`
crate (2003), listed in the catalog under its concern. The arena of
plain structs stays the default (ADR-0003 §4, as 2904 amends it).

## Nick input

None.

## Scope

**In:**
- **`crates/ecs/` (`shuyi-ecs`)**, `no_std` with `alloc`, in 3005's
  `thumbv7em-none-eabihf` gate:
  - `World`; `Entity`, a generational id that serializes and is stable
    across save and load;
  - components: any `'static` struct with `#[derive(Component)]`
    (a derive in `crates/ecs-macros` if 2904's choice doesn't bring
    one);
  - `Query<(&A, &mut B), (With<C>, Without<D>)>`, with `Option<&T>`,
    and `Entity` in the tuple;
  - resources: `Res<T>`, `ResMut<T>`, and a system's own state
    (`Local<T>`);
  - change detection: `Added<T>`, `Changed<T>`, ticks per system run;
  - `Commands`: spawn, despawn, insert, remove, applied at the end of
    the system;
  - systems as plain functions of these parameters;
  - `Schedule`: systems in the order they were added, with
    `.after(...)` and `.before(...)`, and a check that rejects cycles
    with the systems' names;
  - **deterministic iteration**, as 2904's ADR states it, and
    `World::hash()` through `shuyi_core::hash` over every component
    that opts in (`#[derive(Component, Hash)]`);
  - a text dump (`World::dump()`, RON) of every serializable component
    by entity, and `World::load` from it, for tests, agents and saves
    (scenes use it, 2221);
  - no floats in the module's own code (`AGENTS.md` rule 5); a game's
    components may hold `Fx` (1119) or integers. Hold it as 2015 holds
    `shuyi-core`: put
    `#![cfg_attr(not(test), deny(clippy::float_arithmetic))]` in
    `crates/ecs/src/lib.rs`.
- **Entries in `tools/layers.ron`** (2015) for `shuyi-ecs`, and for
  `shuyi-ecs-macros` if it exists: the engine crates each may use, its
  outside crates, `devices: false`, and `floats: false` for
  `shuyi-ecs`. `cargo xtask check-layers` fails on a `shuyi-*` crate
  with no entry.
- **Module metadata** (2003): concern "simulation", one line on what it
  is for ("many things with mixed parts, as in Bevy"), and what it
  conflicts with: nothing.

**Out (do not do):**
- running a schedule inside a game's stage, the parallel scheduler and
  the example (2108);
- scenes (2221);
- relations, observers, events beyond `Commands` (follow-ups if a game
  needs them).

## Implementation steps

1. Read 2904's ADR and research notes.
2. Write the crate's core: world, entities, storage, queries.
3. Add resources, change detection, commands and the schedule.
4. Add the hash, the dump and load, the module metadata, the float ban
   and the `tools/layers.ron` entries.
5. Run the gates.

## Acceptance criteria

- [ ] A system written as `fn step(mut q: Query<(&mut Pos, &Vel)>)` runs
      from a `Schedule`.
- [ ] `Added` and `Changed` see exactly the entities changed since the
      system's last run (unit tests per case).
- [ ] The same spawns, systems and despawns give the same hash in two
      runs and in the web build (a test in CI's wasm job).
- [ ] `World::dump` then `World::load` gives an equal world (property
      test over random worlds).
- [ ] A cycle in `.after`/`.before` is an error naming both systems.
- [ ] The crate builds for `thumbv7em-none-eabihf`.
- [ ] `cargo xtask check-layers` passes with the new crates' entries.
- [ ] All gates in the `run-gates` skill pass, mutation testing in CI
      included.

## Tests required

- Unit: each query filter; each command; change ticks; schedule order.
- Property: dump and load round trip; despawn then spawn never reuses a
  live id; hashes equal across two runs of random command lists.

## Completion notes

*(Filled in by the session that completes the ticket.)*
