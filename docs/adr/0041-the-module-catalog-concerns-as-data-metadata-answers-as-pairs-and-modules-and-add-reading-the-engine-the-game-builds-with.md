# ADR-0041: The module catalog: concerns as data, metadata answers as pairs, and `modules` and `add` reading the engine the game builds with

- **Status:** Accepted
- **Date:** 2026-10-23
- **Related tickets:** 2003 (this), 2501 (the composer), 2502 (the wizard),
  2526 (outside crates), 1134 (the skill pack), 3017 (`needs`)
- **Extends:** ADR-0009 §2, §4 and §6 (concerns, what a module ships with,
  turning one on), ADR-0040 §5 (formats are the game's call)

## Context

ADR-0009 says a module has metadata (`concern`, `answer`, `serves`,
`alternatives`, `works_with`), that a `shuyi` crate has a feature per
module, and that `shuyi modules` and `shuyi add` read the metadata. It
leaves open:

- **Where the concerns live.** Ticket 2003 asks for them as data,
  `docs/modules.ron`, read by the catalog and the check; 2121, 2502, 2707,
  2721 and 2740 already write into it (an answer set to ready, the `kind`
  concern with `alternatives: false`).
- **The metadata's shape.** One module may answer several concerns
  (`shuyi-3d`: three perspectives and *World drawing*), so `concern` and
  `answer` are a list of pairs. A module that adds formats can't add them
  at install (ADR-0040 §5): the game's `formats()` calls a function.
- **Where the tools find a module.** `shuyi-cli` is a binary a game runs
  on its own folder. The modules are crates of the engine the game builds
  with, at the revision it pins, and that engine may be a git checkout
  in cargo's cache or a folder on the same machine.
- **Where a module's skill is.** ADR-0009 §4 puts it in the engine's skill
  pack (1134, not built). 2526 copies the skill of a crate outside this
  repository, with a function that takes a package folder.
- **The check runs in the xtask**, which reads `cargo metadata` through
  2015's `layers.rs`. The xtask doesn't depend on `shuyi-cli`, and the
  CI jobs that run it build nothing else.

## Decision

### 1. The concerns are `docs/modules.ron`

A list of concerns, each with `id`, `name`, `alternatives` (`false` when
its answers can share a stage) and `answers`. An answer has `id`, `name`,
`by` (`default`, a kit of the default engine, or the module's crate,
`shuyi-physics`), `status` (`ready`, `planned`, `parked`) and `tickets`.
`shuyi_cli::catalog::Concerns` reads it, every field required.

The catalog, `docs/modules.md`, is written by hand: a section per concern,
then format plugins and backends, a row per crate whose first cell is its
name in backticks. Its rows are the place for a module's serving and
tickets; the RON file is the place for what answers what.

### 2. A module's metadata

```toml
[package.metadata.shuyi]
answers = [{ concern = "bodies", answer = "rigid-body" }]
serves = ["side-view", "top-down"]
alternatives = ["shuyi-procanim"]   # optional
works_with = []                     # optional
formats = "shuyi_physics::formats"  # optional: the game's formats() calls it
```

`shuyi_cli::catalog::ModuleInfo` reads it with `deny_unknown_fields`,
from a `Cargo.toml` or from `cargo metadata`'s JSON (2526). A module's
feature of `shuyi` is its crate's name without `shuyi-`.

### 3. `cargo xtask check-modules` reads only what it checks

`crates/xtask/src/modules.rs` reads the members' `metadata.shuyi` and
features from `layers.rs`'s `Graph` (which gains both), `docs/modules.ron`
for ids and `alternatives`, and the catalog's rows. It checks the
metadata's keys against `ModuleInfo`'s and reads the fields it uses. It
doesn't depend on `shuyi-cli`. A module is a member under `crates/`
other than the default six, `shuyi` and `xtask`. `check-layers` counts
`shuyi` as an engine crate, with its own entry in `tools/layers.ron`.

### 4. `modules` and `add` read the engine the game builds with

`shuyi_cli::catalog::engine_root` finds the checkout from the game's root
`Cargo.toml`: the folder above the `shuyi` crate it names by `path`, or
else where `cargo metadata` resolved `shuyi` (a git checkout in cargo's
cache holds the whole repository). `Catalog::read` then reads that
checkout's `docs/modules.ron` and every `crates/*/Cargo.toml` with
`[package.metadata.shuyi]`. Nothing of the catalog is embedded in the
binary, so a game sees its own engine's modules at its own pin.

The game's `shuyi` dependency is read and edited under
`[workspace.dependencies]` (a game made by `shuyi new`), else under
`[dependencies]`, with `toml_edit` keeping its layout. Only the root
`Cargo.toml` is read; the answer says so in its limits. A game with no
`shuyi` dependency fails with `SH1004` (`engine-dependency`). A name no
module of the engine has fails with `SH0003` (`unknown-module`), naming
the answers a planned one would give.

### 5. A skill comes from the package, or from the skill pack beside it

`catalog::skill_of(package, name)` takes the package's own `skill/`
folder, or else `skills/<name>` two folders up (the engine's skill pack,
1134), whichever holds a `SKILL.md`. `catalog::copy_skill` copies its
regular files to the game's `.claude/skills/<name>`, only when the game
keeps that folder, and never over one already there.

### 6. What `add` leaves to the game

It never edits the game's code. It gives:

- the line to add when no Rust file of the game names
  `shuyi::enabled_modules` (the templates already do);
- the call for the game's `formats()` when `formats` is set;
- the alternatives already on, with the README's section on using both,
  `## Beside its alternatives` (anchor `beside-its-alternatives`, the
  fifth section of ADR-0009 §4).

### 7. `BasicGame` installs modules

`BasicGame::modules(fn() -> Vec<Box<dyn Module<BasicData<A, T>>>>)` names
the modules its `game(ctx)` installs, and `game` returns
`Result<Game, ModuleError>`; `start` makes the error the load's.

## Consequences

- A module's ticket adds its crate, metadata, row, `layers.ron` entry, the
  feature of `shuyi` and its line in `enabled_modules`; the check fails
  on any one missing.
- 2501's composer calls `catalog::turn_on`; 2526 reads `ModuleInfo` from
  `cargo metadata`'s JSON and copies a package's skill with the same
  functions; 3017 adds `needs` to `ModuleInfo` and the xtask's key list.
- `modules` and `add` on a game pinned by git run `cargo metadata`, which
  may fetch the engine once.
- The concerns are read in two places, the xtask's and `shuyi-cli`'s; the
  check's key list and `ModuleInfo` must stay equal, which a test of each
  holds against this repository.
- `BasicGame::game`'s signature changes: a break for a game that calls it.

## Alternatives considered

- **The catalog embedded in the binary** (`include_str!`, as the
  templates are): a game would see the binary's catalog, not its own
  engine's at its pin, and the module crates' READMEs and skills
  aren't embedded anyway.
- **`modules.md` generated from `modules.ron`**: one source, but the
  tickets already written edit the rows by hand, and a row carries what
  the RON file needn't (a format's serving, a backend).
- **The xtask depending on `shuyi-cli`** for one reader: every xtask job
  would build the engine.
- **`add` editing the game's code** to install the module: code is the
  developer's; the templates install `enabled_modules` once instead.
- **`concern` and `answer` as two strings**: `shuyi-3d` answers four.
