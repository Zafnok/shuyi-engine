---
id: "3305"
title: "NOTICES.txt lists the crates of the build as Cargo resolves it, default features included"
type: bug
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-10
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

- [x] An export with `--no-default-features` lists no crate only a default
      feature brings, in a stand-in workspace like
      `export/tests/editions.rs`'s.
- [x] A dev-dependency of the game is never listed.
- [x] Hello, Grid's packages list the same crates or fewer than before.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the crates of a resolved tree mapped to their notices.
- Integration: a stand-in workspace with a dependency behind a default
  feature, exported with and without its defaults.

## Completion notes

- `notices::built_crates` (new, beside `notices::crates`, which is kept)
  lists the crates of a build as Cargo resolves it: `cargo tree` with the
  build's own package, target and feature flags (`--no-default-features`
  included) and its Rust flags in `CARGO_ENCODED_RUSTFLAGS`, then each of
  those crates' license and files from `cargo metadata`, as before. A crate
  of the build that `cargo metadata` doesn't list is an error, never left
  out unseen.
- `live_link::tree_args` is split: `live_link::build_flags` turns a build's
  arguments into `cargo tree`'s, shared by the debug-feature check and the
  notices; `live_link::encoded_flags` gives the Rust flags both hand Cargo.
- `ExportCx::native_notices` (desktop), the Playdate's `export` and
  `web::run` (`shuyi web` and `export web`) call it with the arguments of
  the build they run (`ExportCx::cargo_args`, the Playdate's `cargo rustc`
  arguments, `web::cargo_args`).
- Tests: a resolved tree mapped to its notices on the fixture's metadata
  (and the unseen-crate error); the build's features handed to the
  metadata; a stand-in workspace whose `story` comes only through the
  default `full` feature, with a dev- and a build-dependency, built with
  and without its defaults; Hello, Grid's web and Linux builds list a
  subset of what the whole-graph walk did.
- Deviation: only **normal** edges are listed, not normal and build as the
  Scope's first line says. A build-dependency runs on the build machine and
  isn't compiled into the program, which is what `NOTICES.txt` lists (as
  before this ticket); listing them would have made Hello, Grid's list
  longer, against the third criterion. The debug-feature check still
  resolves normal and build edges.
- Docs: `crates/cli/web/README.md`'s notices and `docs/targets/desktop.md`
  say how the list is made. ADR-0100's *Consequences* line about the
  superset is now history; accepted ADRs aren't edited.
- No break: the public items are additions.

Book: none, the book's `targets-and-export.md` says `NOTICES.txt` holds the licenses of every crate the build carries, which is now exactly so; the new functions are the CLI's own.
