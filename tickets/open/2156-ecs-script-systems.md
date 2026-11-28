---
id: "2156"
title: "Script systems over a shuyi-ecs world: a .rhai system reads and writes registered components by name inside the schedule, and declares components of its own"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["2107", "2108"]
nick_input: none
completed:
---

# 2156 — Script systems over a shuyi-ecs world

## Context

`shuyi-script` (2101, done) runs sandboxed rhai with step limits, and a
game calls it from its own Rust. 2107 builds `shuyi-ecs`, where every
system and component is Rust. The Bevy coverage audit of 2026-11-28
(`docs/ROADMAP.md`, *Bevy's game coverage*) found that moddable
strategy and roguelike games, and data-driven rules games, need a third
thing: a script that is a system, reading and writing components by
name, and a mod that adds a component of its own.

Bevy does this with dynamic components and dynamic queries
(`examples/ecs/dynamic.rs`: `ComponentDescriptor`, `QueryBuilder`;
`crates/bevy_ecs/src/component/info.rs`,
`crates/bevy_ecs/src/query/builder.rs`), which its scripting crates
build on. Here values cross the boundary through ADR-0084 (serde and
JSON Pointer, no reflection registry), by the names 2221's
`SceneRegistry` gives components. 2904's ADR wins where it differs.

## Nick input

None.

## Scope

**In:**
- **A script system:** a `.rhai` file with a header that declares what
  it reads and writes, for example
  `// reads: Position, Speed` and `// writes: Position`. The schedule
  places it like a Rust system, so the parallel schedule (2108) keeps
  its hash.
- **Values by name:** components and resources cross as script maps
  through serde, by their registered names; a write the header didn't
  declare is an error with the file and line.
- **Components a script declares:** `// component Mana { now: 0, max:
  10 }`, a component of plain script values (integers, strings, bools,
  lists and maps; no floats, as 2101's sandbox already refuses them).
  It saves, dumps and hashes like any other.
- **The sandbox:** 2101's step limit applies per system run; running out
  is the system's error, handled as 2155 handles a failing system.
  **Either order with 2155:** if 2155 is done, a script that runs out of
  steps fails through its `EcsError` policy; if not, the command fails
  with the script's error and 2155 maps it when it lands.
- `ScriptSystemError` is `#[non_exhaustive]` (ADR-0039).

**Out (do not do):**
- a game's whole rules as a script (2146, parked);
- reloading a script system when its file changes: not this ticket's
  job (2145 makes a script an asset kind);
- scripts as observers of triggered events (2154's observers are Rust);
- floats in script components.

**Estimated size:** about 1,100 lines (code 550, tests 400, docs 150).

**Split:** past 900 lines, split off components a script declares into
a new ticket.

## Implementation steps

1. Read 2904's ADR, 2101's `shuyi-script`, 2107's world and the table
   that names components (2221's `SceneRegistry`, or 2107's own).
2. Parse the header; build an access set the schedule reads.
3. Run the script once per matching entity with its components as maps;
   write back the declared ones through serde.
4. Add script-declared components as a dynamic column keyed by name,
   saved, dumped and hashed.
5. Map step-limit and write errors to the system's error.
6. Write the docs page: a mod that adds a `Mana` component and a regen
   system.

## Acceptance criteria

- [ ] A script system that moves `Position` by `Speed` gives the same
  hash as the same system in Rust, in the serial and parallel schedules.
- [ ] A write to an undeclared component fails with the file and line.
- [ ] A script-declared component survives a save and load and is in the
  dump.
- [ ] A script past its step limit fails its run, and the world is
  rolled back in a turn game.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] Guides in `docs/guides/` updated, if this adds a direction, a module or a move (ADR-0009 §5).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: header parsing, access errors, value round trips.
- Property: a random script-written world hashes the same on native and
  wasm32 (the determinism job).
- Snapshot / integration: the `Mana` mod fixture's dump.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
