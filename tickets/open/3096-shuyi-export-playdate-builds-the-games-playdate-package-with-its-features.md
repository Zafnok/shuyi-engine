---
id: "3096"
title: "`shuyi export playdate` builds the game's Playdate package, with its features"
type: feature
milestone: MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3096 — `shuyi export playdate` builds the game's Playdate package, with its features

## Context

1. **The game ticket:** game#1170,
   [the Playdate front end on the engine's backend](https://github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1170-the-playdate-front-end-on-the-engines-backend.md).
   It moves Visions of Shuyi's Playdate build from its own exporter
   (`cargo xtask playdate`) to `shuyi export playdate`, and can't: the
   export builds the wrong package. It is not a blocker of this ticket.
2. **The job in one sentence:** a game whose Playdate library is a
   package of its own, beside its desktop program and its tool program,
   exports it with `shuyi export playdate`, with the features that build
   needs.
3. **No friction review:** a missing feature, found while starting 1170
   (2026-11-07).
4. **The generic shape.** The Playdate exporter builds
   `ExportCx::package`, which for every target but the web is
   `export.package`, else the `game` part's package (`package_of` in
   `crates/cli/src/export.rs`), and since the Playdate takes no `export`
   part (`takes_export_part` is `false`) it is always the `game` part's:
   the tool program's. Its `cargo rustc` turns on `playdate` and nothing
   else (`cargo_args` in `crates/cli/src/export/playdate.rs`). That fits
   Hello, Grid, one package that is the game, its tools and its Playdate
   library. It doesn't fit a game split into packages: Visions of Shuyi's
   tool program is `xtask` (`game: (package: "xtask")`), its desktop
   program `trpg-app` (`export.package`), and its Playdate library
   `trpg-playdate`, whose bought art is a cargo feature
   (`private-assets`) and whose measuring build is another (`bench`).
   What is missing: `shuyi.ron` naming, for one target, the package its
   export builds and the features it turns on, as `export` names them
   for the desktop; and an export turning on a feature for one build
   (a measuring build), as `--without` turns one off.
5. **An example that isn't tactics:** a platformer whose desktop
   program is `jumper-app`, whose tools are `jumper-tools`, and whose
   Playdate library is `jumper-playdate` with a `demo` feature for a
   one-level build: the same lines.

## Nick input

None.

## Scope

**In:**
- **A target's own export settings** in `shuyi.ron`'s `export` part:
  `targets: { "playdate": (package: "trpg-playdate", features:
  ["private-assets"]) }`, both optional. For that target, its `package`
  replaces the package `package_of` picks, and its `features` replace
  `export.features`. Reading it needs no `takes_export_part`: `beside`
  still doesn't apply to the Playdate. `ExportPart` gains the field
  (additive; it is `#[non_exhaustive]` or gets a constructor if it
  isn't, ADR-0039); the new type is `#[non_exhaustive]` with a
  constructor.
- **The Playdate's `cargo rustc`** turns on `playdate` and the features
  turned on (`ExportCx::turned_on`), and `prepare`'s check for the
  `playdate` feature reads that package's features. `NOTICES.txt`'s
  crates are those of that build (`notices::crates` with the same
  features).
- **`--with <feature>`** on `shuyi export` (CLI, `ExportInput::with`,
  the MCP tool's schema): turns on a feature of the game's package for
  this build, one the package has; another is refused naming it, as
  `--without` refuses one `export.features` doesn't list. For every
  exporter, not only the Playdate's.
- **`check --target playdate` and `bake`** are unchanged: they run the
  tool program, as today.
- **Docs:** `docs/targets/playdate.md` (*What a game adds*), the `export`
  part's fields where `docs/contract.md` and the book describe them.

**Out (do not do):**
- The game's change: game#1170 names the package and features in its
  `shuyi.ron` after this ticket's pin move.
- Any other part of `shuyi.ron` taking per-target settings.
- A pack made optional for a game that embeds its files: the game reads
  the pack (1170 decides how).

## Implementation steps

1. `crates/cli/src/project.rs`: the `targets` field of `ExportPart` and
   its type, read and refused as the rest of the part (unknown fields).
2. `crates/cli/src/export.rs`: `ExportCx::of` picks the target's package
   and features when the part names them, before `metadata`; `turned_on`
   takes the target's features and `--with`.
3. `crates/cli/src/export/playdate.rs`: `cargo_args` adds the turned-on
   features after `playdate`; `prepare` and the notices use them.
4. `--with` in the CLI's export command and `ExportInput`.
5. Tests, then docs.

## Acceptance criteria

- [ ] With `export.targets.playdate.package` set, every Playdate step
      (`cargo rustc`, the static library's path, `NOTICES.txt`) builds
      that package; without it, the package is today's (Hello, Grid's
      export plan is unchanged).
- [ ] The Playdate build's `--features` is `playdate` plus the target's
      features, less `--without`, plus `--with`.
- [ ] `--with` of a feature the package lacks is refused, naming it;
      `--without` of one the target's list lacks is refused, as today.
- [ ] The desktop exports read `export.features` as before when the
      target has no entry.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: reading the part with and without `targets`, an unknown field
  refused; `ExportCx::of`'s package and features for the Playdate with
  and without an entry; `turned_on` with `--with` and `--without`;
  the Playdate plan's `cargo rustc` arguments.
- Integration: none needs the SDK; the plan tests cover the commands.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
