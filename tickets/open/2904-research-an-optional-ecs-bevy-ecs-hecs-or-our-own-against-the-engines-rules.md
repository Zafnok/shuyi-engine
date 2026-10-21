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

The 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6 and
Unreal 5.8 (and Defold) adds a question for this ADR. Bevy, Godot and
Defold each keep one world model. Unity's GameObjects beside its
Entities, and Unreal's Actors beside Mass, are two models whose tools
each had to cover. Here the default store, `Arena<T>`, is 1119's, which
Nick keeps parked until Visions of Shuyi launches (2026-10-20,
`docs/decisions.md`). `shuyi-ecs` (2107, 2108) and its scenes (2221)
come after the MVP. So the opt-in store is built first.

Most tools already name no store:

- `state_hash` takes any `Serialize` (`crates/core/src/hash.rs`);
- saves go through `SaveFormat` over serde (`crates/core/src/save.rs`);
- the contract's `scene` answer is a `Scene { screen, kind, text, data }`
  that a game builds with `Headless::with_scene`
  (`crates/cli/src/contract/session.rs`);
- the planned debug view reads `Screen::debug_draw` (2315) and
  `Screen::debug_readout` (2316).

What still turns on the store is scenes with id remapping (2221), saves
by id, and what the editor's tree lists (2507). This ticket's ADR
settles those, so no second set of tools is ever built.

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
     maintenance on the day;
  10. **The description of a game's types** (2907): `bevy_ecs` 0.20
      turns on `bevy_reflect` among its default features. The ECS is
      opt-in and the arena stays the default, so the ECS's reflection
      can't be the only description of a game's types. **Either order
      with 2907:** if 2907 is done, say for each candidate how it fits
      2907's ADR (component names, the dump through serde, paths); if
      not, 2907's ADR weighs this research's candidates.
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
    order rule, how the parallel scheduler gives the same hash). The
    same ADR says how the two stores share the engine:
    - **The kit rule.** Kits are plain values and pure functions in
      `shuyi_core` (clips in `shuyi_assets::anim`), tied to no store:
      collision (1122, 2124), clips (2231, 1123), particles (2103),
      paths (2106), curves (2122), bodies (2701), and every kit after
      them. None names `Arena`, `Id<T>`, `World` or `Entity`. An id a
      kit hands out is its own (1122's box ids). A game keeps kit state
      in whichever store it uses, and wraps kit functions in its own
      systems if it uses `shuyi-ecs`.
    - **What the tools read.** `state_hash`, saves through `SaveFormat`,
      the contract's `scene` answer, `Screen::debug_draw` and
      `Screen::debug_readout` name no store and stay that way. Saves by
      id, entity inspection and the editor's tree (2507) read the same:
      a stage's dump and hash.
    - **Scenes for the arena.** With the line counts of both ways:
      whether 2221's scene file and its id remapping are written over a
      small trait that 1119's `Arena<T>` can implement later, or stay
      ECS-only while the arena keeps its serde dump. A shared store
      trait (one trait over `Arena<T>` and `World`) comes in only if the
      ADR finds a second real consumer for it. Building it is then a
      follow-up ticket blocked by 1119 and parked with it.
    - **The order with 1119.** Nick keeps 1119 parked until Visions of
      Shuyi launches (2026-10-20), so `shuyi-ecs` is the first general
      store the engine builds. The ADR says why that is acceptable: it
      is opt-in; a turn game keeps its state as a plain struct behind
      `TurnRules` (`crates/core/src/turn.rs`); tick games get no store
      until 1119; and since kits name no store, nothing built before
      1119 is rewritten for it. It also says what 2108's tick adapter
      does while 1119 is parked (2108's either-order clause leaves a
      line in 1119).
  - the measurements in `docs/research/ecs.md`. The benchmark code lives
    in `research/ecs/`, a crate listed under `exclude` in the root
    `Cargo.toml` (as 2705 plans for its own), so no candidate enters the
    workspace's `Cargo.lock`. It stays until 2705 is done, so 2705 can
    reuse it;
  - changes to 2107, 2108, 2221 and 1119 (parked: its text only) to
    match the choice. If the ADR words the kit rule differently from
    the one line each kit ticket carries (1122, 1123, 2231, 2103, 2106,
    2701), it updates those lines too.

**Out (do not do):**
- building the module (2107, 2108);
- moving any existing code onto an ECS;
- a shared store trait over `Arena<T>` and `World` built ahead of this
  ADR (the ADR decides whether one is needed; building it waits for
  1119);
- the real-time strategy module's columns (ADR-0015 §3, 2705, 2707),
  which stay inside that module;
- whether 1119 is unparked: Nick answered on 2026-10-20
  (`docs/decisions.md`), and only he changes it.

## Implementation steps

1. Write the benchmark and the determinism test as a small harness over
   a trait each candidate implements.
2. Run it for each candidate, and the `no_std`, web, size and compile
   time checks.
3. Write `docs/research/ecs.md` and the ADR, with its parts on the kit
   rule, what the tools read, scenes for the arena (count the lines of
   both ways) and the order with 1119.
4. Update 2107, 2108, 2221 and 1119, and the kit tickets' store line
   if the ADR changes it.

## Acceptance criteria

- [ ] Every rule has a result for every candidate in
      `docs/research/ecs.md`.
- [ ] The ADR names the choice and amends ADR-0003 §4.
- [ ] The ADR states the kit rule, what the tools read, its answer for
      scenes of the arena with the line counts behind it, and the order
      with 1119.
- [ ] 2107, 2108, 2221 and 1119 match the ADR.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- The harness's determinism test (same hash twice, and on the web build
  in CI's wasm job).

## Completion notes

*(Filled in by the session that completes the ticket.)*
