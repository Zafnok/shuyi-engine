---
id: "2101"
title: "A deterministic, sandboxed script runtime: the shuyi-script module, and an ADR choosing its language"
type: feature
milestone: After MVP
model: opus-5.5
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2101 — A deterministic, sandboxed script runtime: the `shuyi-script` module

## Context

- **The game ticket:** game#1017
  ([`1017-scripting-runtime-and-scripted-skill-effects.md`](https://github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1017-scripting-runtime-and-scripted-skill-effects.md)).
  Nick asked for "a scripting DSL or something for easy modding
  support". The game's rules call scripts and turn what they return into
  effects it already knows (game#1017); its mods ADR (game#1011) says
  where in the game scripts may run. The runtime is this ticket.
- **The job:** let a modder write a small rule as text that the game
  runs, without the rule being able to break the game's determinism or
  reach outside it.
- **Friction:** no friction review: a missing feature, found when the
  game's tickets were split between the two repositories.
- **The generic shape:** a script runtime a game embeds. Scripts are
  compiled once, at load, from files in the bundle. A script reads a
  read-only view the game hands it and returns plain values the game
  converts into its own types; it never changes the game's state. It
  has no clock, no host randomness, no file or network access, integer
  maths only, and an operation and memory limit. The same script on the
  same view always returns the same values, on every platform, so
  replays, rewinds and lockstep stay exact. ADR-0005 §5 plans visual
  scripting (1135) to compile to it.
- **Outside tactics:** a farming game's modded crop that grows faster in
  rain; a platformer's level script that opens a door when three
  switches are on; a card game's card text.

What research for the split already found (2026-10-14): Rhai (MIT OR
Apache-2.0, pure Rust) builds for `wasm32-unknown-unknown`; `mlua`
supports only emscripten on the web, which the engine's web build
doesn't use (macroquad's loader, no `wasm-bindgen`).

## Nick input

None (technical).

## Scope

**In:**
- **An ADR** (the `write-adr` skill, the next free number) choosing the
  language. Compare at least: Rhai (sandboxed, operation limits, can
  drop floating point); Lua through `mlua` (a C library: check the
  Windows GNU toolchain and the web build); WebAssembly modules
  (`wasmi`, pure Rust); a small expression language of our own. For
  each: the license (ADR-0002 §8, quoted from its source), the size it
  adds to a native and a WASM build (measured, not guessed), determinism
  (no clock, no host randomness, integer maths only, bounded run time),
  and how a modder would write a rule. Default: Rhai, integer only, with
  an operation limit.
- **The module** `shuyi-script` in `crates/script`, as ADR-0009 §4 says:
  metadata in its `Cargo.toml`, a README in the fixed order, no default
  crate depending on it. It wraps the chosen runtime:
  - configured as the ADR says: no clock, no host randomness, no file or
    network access, integer maths only, an operation and a memory limit;
  - `compile(name, source)`: an error names the file and line (the
    bundle's display path);
  - a way for the game to register its read-only view types and to read
    back plain values (numbers, booleans, strings, lists and maps of
    them);
  - running a script that errors or hits a limit gives an error value
    and never panics; the caller decides what that means.
- The WASM build and `cargo deny check` pass with the module built.
- Without `std`: the ADR says whether the module builds for
  `thumbv7em-none-eabihf` without `std` (the target of a handheld such
  as the Playdate, ADR-0008). If it doesn't, the README says so, and a
  game that builds its core without `std` keeps scripting behind a
  feature of its own.

**Out (do not do):**
- Anything a game does with scripts: its view types, its effects, where
  it calls them (game#1017).
- The module's example (2102).
- Visual scripting (1135).
- Scripts that change state directly, or that run on a clock.

**For the game:** additive (a new module). The game's PR that uses it
(game#1017) moves its pin after this one.

## Implementation steps

1. Read ADR-0002 (layers, determinism, licenses), ADR-0005 §5 and
   ADR-0009 §4.
2. Spike the two strongest candidates for size and determinism; write the
   ADR and its index row.
3. Add `crates/script` with the chosen runtime's features cut to what the
   ADR allows; the metadata and README.
4. The API above, with its tests.
5. Check the WASM build and `cargo deny check`.

## Acceptance criteria

- [ ] The ADR compares the candidates as above, with measured sizes and
      quoted licenses, and the index lists it.
- [ ] The same script on the same view always returns the same values
      (property test).
- [ ] A script that loops forever stops at the limit with an error value
      (test).
- [ ] A script that fails to compile reports its file and line (test).
- [ ] A script can't read a clock, a random source, a file or the
      network (tests that each such call fails to compile or errors).
- [ ] The WASM build and `cargo deny check` pass.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: compile errors, limits, reading back values.
- Property: determinism over generated views.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
