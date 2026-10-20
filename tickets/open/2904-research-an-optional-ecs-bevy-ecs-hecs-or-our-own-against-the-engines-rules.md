---
id: "2904"
title: "Research: an optional ECS: bevy_ecs, hecs or our own, measured against the engine's rules, and the ADR for the shuyi-ecs module"
type: research
milestone: After MVP
model: opus-5.5
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2904 — Research: an optional ECS

## Context

Nick, 2026-10-20 ([`docs/decisions.md`](../../docs/decisions.md)): every feature
on Bevy's homepage, including an "ECS system (perhaps optional)". Bevy's
homepage says of its ECS: components are plain Rust structs, systems are
plain Rust functions, with queries, global and local resources, change
detection, and a lock-free parallel scheduler.

[ADR-0003](../../docs/adr/0003-game-styles-stages-two-kinds-of-simulation-cameras-and-projections.md)
§4 chose plain structs in an arena over an ECS, for determinism, state
readable as text, and compile time, and left an ECS open behind a
measured spike. Nick's decision keeps the arena as the default and adds
an ECS as an **opt-in module** (ADR-0009). This ticket picks how.

2705 (parked, the real-time strategy module) plans to time an arena,
columns and `hecs` on thousands of units. **Either order:** if 2705 is
still parked when this runs, this ticket's benchmark harness is written
so 2705 can reuse it; if 2705 ran first, use its numbers.

## Nick input

None. This is technical.

## Scope

**In:**
- **The rules**, each a pass or a measured number:
  1. **Determinism:** the same world and systems give the same hash on
     every run and platform, including after despawns and respawns
     (entity ids, archetype moves and iteration order all count);
  2. **State as text:** the world dumps to readable text and loads back
     equal (serde), for tests, agents and saves;
  3. **`no_std` with `alloc`**, so it builds in 3005's
     `thumbv7em-none-eabihf` gate (the Playdate);
  4. **The web build** without `wasm-bindgen` (game ADR-0034,
     `AGENTS.md`, *Environment*);
  5. **Compile time:** the rebuild of a small game using it, measured
     with 2009's tool if done (else `cargo build --timings`);
  6. **Size:** the web build's `.wasm` with and without it;
  7. **Speed:** a benchmark of 1 000 and 10 000 entities with three
     systems (move, collide on a grid, count), single-threaded, and
     parallel where it can be;
  8. **The API:** components as plain structs, systems as plain
     functions, queries with filters, resources, change detection
     (`Added`, `Changed`), deferred commands, an explicit schedule;
  9. **License** (ADR-0002 §8), default features (ADR-0004 §7),
     maintenance on the day.
- **The candidates:**
  - `bevy_ecs` alone, without the rest of Bevy (check its `no_std`
    support and what its `bevy_platform` and `bevy_tasks` pull into
    `wasm32-unknown-unknown`);
  - `hecs` (with a schedule and change detection of our own on top);
  - our own small ECS in `shuyi-ecs`: sparse sets or archetypes, as the
    measurements say.
- **Writes:**
  - an ADR (the `write-adr` skill) that amends ADR-0003 §4: the arena
    stays the default; `shuyi-ecs` is an opt-in module; which candidate
    and why, with the numbers; how determinism is kept (the iteration
    order rule, how the parallel scheduler gives the same hash);
  - the measurements in `docs/research/ecs.md`. The benchmark code lives
    in `research/ecs/`, a crate listed under `exclude` in the root
    `Cargo.toml` (as 2705 plans for its own), so no candidate enters the
    workspace's `Cargo.lock`. It stays until 2705 is done, so 2705 can
    reuse it;
  - changes to 2107 and 2108 to match the choice.

**Out (do not do):**
- building the module (2107, 2108);
- moving any existing code onto an ECS.

## Implementation steps

1. Write the benchmark and the determinism test as a small harness over
   a trait each candidate implements.
2. Run it for each candidate, and the `no_std`, web, size and compile
   time checks.
3. Write `docs/research/ecs.md` and the ADR.
4. Update 2107 and 2108.

## Acceptance criteria

- [ ] Every rule has a result for every candidate in
      `docs/research/ecs.md`.
- [ ] The ADR names the choice and amends ADR-0003 §4.
- [ ] 2107 and 2108 match the ADR.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- The harness's determinism test (same hash twice, and on the web build
  in CI's wasm job).

## Completion notes

*(Filled in by the session that completes the ticket.)*
