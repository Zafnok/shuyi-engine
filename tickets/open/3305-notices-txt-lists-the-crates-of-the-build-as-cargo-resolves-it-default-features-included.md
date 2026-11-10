---
id: "3305"
title: "NOTICES.txt lists the crates of the build as Cargo resolves it, default features included"
type: bug
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3305 — NOTICES.txt lists the crates of the build as built

## Context

Every package carries `NOTICES.txt`, made by `notices::crates`
(`crates/cli/src/notices.rs`) from `cargo metadata --filter-platform <triple>`
with the build's features as `--features <package>/<feature>`. It never says
whether the build keeps the package's default features, and `cargo metadata`
resolves the whole workspace with dev-dependencies, so the list is a superset
of what the program is made of.

3082 made that matter
([ADR-0100](../../docs/adr/0100-editions-named-builds-in-shuyi-ron-a-content-root-handed-to-the-build-and-default-features-always-said.md),
*Consequences*): an edition with `default_features: false` builds without
the package's defaults, but its `NOTICES.txt` still lists the crates a default
feature would bring. A public demo can name a crate only the full game uses
(a crate of the full game's story, say), and a demo's canary in that name
refuses the export through the disclosure gate (ADR-0070).

The resolved-build guard already resolves each planned build exactly
(`export::live_link::resolve`: `cargo tree` with the build's own flags, normal
and build edges, the exporter's Rust flags).

## Nick input

None.

## Scope

**In:**
- The crates `NOTICES.txt` lists for an export are those of the build as Cargo
  resolves it: its package, target, feature flags (`--no-default-features`
  included) and Rust flags, normal and build dependencies only.
- Desktop, web and Playdate exports, through the build arguments each
  exporter plans (`Exporter::cargo_args`).
- `shuyi web` (no export) keeps working.

**Out (do not do):**
- Changing what a notice says about a crate, or the engine's files and
  game's texts (`notices::engine_files`, `game_texts`).
- Editions themselves (ADR-0100).

## Implementation steps

1. Read `notices::crates` and `crates_of`, and `live_link::resolve` and
   `tree_args`.
2. Give the notices the resolved package set (names and versions from the
   build's `cargo tree`), and keep `cargo metadata` for each crate's license
   and files; a public function whose signature changes needs a `Breaking:`
   line, so prefer a new one beside it.
3. Call it from `ExportCx::native_notices`, the web build's `run` and the
   Playdate's `export` with each build's arguments.

## Acceptance criteria

- [ ] An export with `--no-default-features` lists no crate only a default
      feature brings, in a stand-in workspace like
      `export/tests/editions.rs`'s.
- [ ] A dev-dependency of the game is never listed.
- [ ] Hello, Grid's packages list the same crates or fewer than before.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the crates of a resolved tree mapped to their notices.
- Integration: a stand-in workspace with a dependency behind a default
  feature, exported with and without its defaults.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
