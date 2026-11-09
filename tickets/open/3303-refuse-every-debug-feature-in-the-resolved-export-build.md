---
id: "3303"
title: "Refuse every debug feature in the resolved export build, not the live link alone"
type: bug
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: ["3085", "3099"]
nick_input: none
completed:
---

# 3303 — Refuse every debug feature in the resolved export build

## Context

Two tickets refuse debug features in an export, each half of the way:

- 3085 ([ADR-0096](../../docs/adr/0096-an-export-asks-cargo-what-each-build-turns-on-and-refuses-the-live-link-before-anything-is-built.md))
  resolves each build with `cargo tree` (`crates/cli/src/export/live_link.rs`,
  `refuse`) and refuses the live link however Cargo reaches it: the
  package's `default` list, a feature that lists it, a dependency asking
  for `shuyi-app/link` or `shuyi-platform/link`.
- 3099 ([ADR-0094](../../docs/adr/0094-release-exports-ship-no-symbols-no-debug-features-and-no-state-on-demand.md))
  refuses all four debug features, `export::DEBUG_FEATURES` (`link`,
  `debug-tools`, `capture`, `sound-probe`), but only by the build's
  literal arguments (`export::no_debug_features`).

So a game whose `default = ["debug-tools"]`, or whose
`devtools = ["shuyi-ui/debug-tools"]` is listed in `export.features`,
ships the debug menu, `SHUYI_PERF` traces, session recording, the crash
record in release and the log's info and debug lines. Each needs a
mistake in the game's manifest, not an attacker.

## Nick input

None.

## Scope

**In:**
- `live_link::violations` (or a module renamed for it) watches every
  feature of `DEBUG_FEATURES`, each in the package built (depth 0) and
  in the engine's crates that have it: `link` in `shuyi-app` and
  `shuyi-platform`; `debug-tools` in `shuyi-ui`; `capture` in
  `shuyi-app` and `shuyi-platform`; `sound-probe` in `shuyi-app`. The table of which engine
  crate has which is data beside `DEBUG_FEATURES`, and a test reads each
  engine crate's `Cargo.toml` features to keep it equal.
- The refusal names the feature with `DEBUG_FEATURES`' words for it,
  and how it was reached, as 3085's does for the live link.
- The same dev-dependency rule as 3085's: a crate that turns on its own
  `capture` in its tests isn't refused.

**Out (do not do):**
- A runtime check in the shipped game.
- Changing what any debug feature does.

## Implementation steps

1. Read `crates/cli/src/export/live_link.rs` and its tests
   (`live_link/`), and `export::DEBUG_FEATURES`.
2. Replace the single `FEATURE` and `ENGINE` with a table of
   `(feature, engine crates)` for the four features; `violations`
   checks each.
3. Keep `no_debug_features` as the arguments' quick check before the
   resolution, as 3085 keeps it.
4. Tests with fixture `cargo tree` outputs for each feature reached by a
   default list, an alias and a dependency, and one for a dev-dependency
   that isn't refused.
5. Update `docs/contract.md`'s `export` row, `docs/targets/desktop.md`
   and ADR-0096's successor note if the design changes (a new ADR if it
   does; none if it only widens the table).

## Acceptance criteria

- [ ] An export of a package with `default = ["debug-tools"]` is refused,
      naming `debug-tools` and the `default` list.
- [ ] A dependency turning on `shuyi-ui/debug-tools` or
      `shuyi-app/capture` is refused, naming the path.
- [ ] Hello, Grid and both templates' games still export.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `violations` over fixture trees for each feature and each way it
  is reached, and a dev-dependency's that isn't.
- Unit: the table equals the engine crates' `[features]`.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
