---
id: "2372"
title: "A module's debug menu entries: declared in install, listed after the game's, and in run --json"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: ["2358"]
nick_input: none
completed: 2026-10-23
---

# 2372 — A module's debug menu entries

## Context

2358 gave modules a way to join a running game
([ADR-0040](../../docs/adr/0040-modules-join-a-running-game-a-library-first-a-module-trait-with-declared-hooks-and-state-by-type-on-the-context.md),
`crates/ui/src/module.rs`): a `Module` declares its hooks and its state
on `Ctx` in `install`, `Game::modules()` lists what each declared, and
`run --json` lists the modules (`ModuleEntry`). ADR-0040 §4 also gives
a module debug menu entries. 2358 left them here, by its size rule.

Today `DebugMenuScreen::new(ctx, entries)` (`crates/ui/src/debug.rs`)
lists only the entries the game hands it, so a module's tools (a
physics module's "show bodies", a script module's "reload scripts") can
reach the menu only if every game adds them by hand.

## Nick input

None.

## Scope

**In:**
- **`Install::debug_entry(&mut self, make: fn() -> DebugEntry<G>)`**
  in `crates/ui/src/module.rs`: an entry for the debug menu, made each
  time the menu opens. It records the entry's label (read at install
  with `DebugEntry::label` and the `Ctx` that `Install` holds) and keeps
  `make` on the context.
- **The makers on `Ctx`**: `module::States` (the private field
  `Ctx::modules`) gains the makers, in install order, beside the
  states. Its `Debug` keeps listing the states by type name.
- **`InstalledModule::debug_entries: Vec<String>`**: the labels, in
  the order declared. `InstalledModule` is `#[non_exhaustive]`
  (ADR-0039), so the field is additive.
- **The debug menu:** `DebugMenuScreen::new(ctx, entries)` lists the
  game's entries, then each module's, made from the makers on `ctx`, in
  install order. With no module installed it is as today. A game whose
  `GameHooks::debug` is `None` has no menu, so its modules' entries
  don't show. Its signature doesn't change.
- **The contract:** `ModuleEntry::debug_entries: Vec<String>` in
  `crates/cli/src/contract/session.rs`, from `InstalledModule`; raise
  `contract::VERSION` by one, regenerate the schemas (`cargo shuyi
  schema --out crates/cli/schemas`), add the version's line to
  `crates/cli/schemas/versions.json`, set the example answer's
  `contract` in `docs/contract.md`, and say in `run`'s row that it
  lists each module's debug entries. `shuyi run`'s text for people
  needn't show them.
- **For the game:** additive. `DebugMenuScreen::new` keeps its
  signature; `ModuleEntry` and `InstalledModule` are
  `#[non_exhaustive]`.

**Out (do not do):**
- option rows from modules (a player setting needs a place in the saved
  settings; ADR-0040, *Consequences*);
- screens a module pushes over a running game on its own (2367); an
  entry may push a screen over the debug menu, as any entry does;
- `BasicGame` taking modules, and `Module` in the prelude (2003).

## Implementation steps

1. Read ADR-0040, `crates/ui/src/module.rs`, `ctx.rs`, `debug.rs` and
   `crates/cli/src/contract/session.rs`.
2. Add the makers to `module::States`, `Install::debug_entry` and
   `InstalledModule::debug_entries`.
3. Extend `DebugMenuScreen::new` with the modules' entries; update its
   doc and `DebugMenuScreen`'s.
4. Add `ModuleEntry::debug_entries`; raise the contract's version,
   regenerate the schemas, and update `docs/contract.md`.
5. Write the tests; run the gates.

## Acceptance criteria

- [x] `Install::debug_entry` records the entry's label in
      `Game::modules()`, in the order declared.
- [x] `DebugMenuScreen::new` lists a module's entries after the game's;
      choosing one runs it; with no module its menu is as before.
- [x] A toy game with a module that declares an entry, its Debug key
      pressed in the Harness, shows the entry last on the menu.
- [x] `run --json` lists each module's debug entries; the schema test
      passes at the new version.
- [x] `examples/hello-grid` and the templates build with no change.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `Install::debug_entry` (`crates/ui/src/module/tests.rs`); the
  debug menu's order of entries (`crates/ui/src/debug/tests.rs`).
- Integration: the Harness test above in `crates/ui/tests/it/modules.rs`;
  `ModuleEntry`'s field in `crates/cli/src/contract/run/tests.rs` and
  `session/tests.rs`.

## Completion notes

- `Install::debug_entry(make: fn() -> DebugEntry<G>)` records the
  entry's label (read at install from the `Ctx` it holds) in
  `InstalledModule::debug_entries` and keeps `make` on the context:
  `module::States` became `States<G>`, with the makers beside the
  states, in install order. Its `Debug` still lists only the states.
- `DebugMenuScreen::new(ctx, entries)` appends the modules' entries,
  made from the makers on `ctx`, after the game's. Its signature is
  unchanged; with no module the menu is as before.
- `ModuleEntry::debug_entries` in the contract; `contract::VERSION` is
  15, schemas regenerated, `versions.json` and `docs/contract.md`
  updated (the example answer and `run`'s row).
- Deviation: the Harness test reads the menu from the frame's snapshot
  (the Gallery row above the module's), not through `Harness::screen`,
  since `DebugMenuScreen` doesn't opt in to being found by type; the
  menu's labels and running an entry are checked in the unit test.
- Docs: the rustdoc of `module`, `Install::debug_entry`,
  `DebugMenuScreen` and `ModuleEntry`, and the contract page; ADR-0040
  already describes the entries (§4).
- Settled ticket 2101's clause that named this ticket.
- No break: `InstalledModule` and `ModuleEntry` are `#[non_exhaustive]`,
  `module::States` is crate-private.
