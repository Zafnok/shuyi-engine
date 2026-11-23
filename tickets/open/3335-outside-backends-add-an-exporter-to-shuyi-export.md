---
id: "3335"
title: "A backend outside this repository adds an exporter to shuyi export: a shuyi-export-<id> program on PATH, with the CLI's own checks run around it"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3335 — A backend outside this repository adds an exporter to shuyi export

## Context

A console backend lives in a private repository and depends on the
engine's crates like any game
([ADR-0008](../../docs/adr/0008-export-targets-profiles-as-data-platform-backends-controls-that-fit-one-bit-and-budgets.md)
§2, [`docs/platform-backends.md`](../../docs/platform-backends.md),
*Console backends stay private*). That covers the runtime: `Platform`
and `App::frame`. It does not cover the tools. `shuyi export`, its
`--plan` (the matrix `release.yml` builds, ticket 3050), the disclosure
gate, the path search of a release build and the run step all go
through one closed list:

```rust
// crates/cli/src/export.rs:210
static EXPORTERS: [&dyn Exporter; 5] = [
    &desktop::WINDOWS, &desktop::LINUX, &desktop::MACOS, &web::WEB, &playdate::PLAYDATE,
];
```

`export::find`, `export::ids` and `export::exporters` read only that
list (`crates/cli/src/export.rs:218-234`), and
`contract::export::export` answers `no-exporter` (`SH3010`) for any
other target (`crates/cli/src/contract/export.rs:447-454`). A game may
add its own target profile (`shuyi.ron`'s `target_profiles`,
`crates/cli/src/project.rs:135`), so `check --target switch` can pass
today, and then nothing can package it.

The `Exporter` trait is already public (`pub mod export` in
`crates/cli/src/lib.rs:42`), so a private crate can write one. It just
has no way in. Its only choice is to fork `shuyi-cli`, and lose the
checks that tickets 3080, 3081, 3099 and 3300 built (the disclosure
gate, the path search, debug features refused, sealed packages). Or it
rebuilds them by hand. ADR-0040's Consequences (line 170) defer
"importers, bake steps and contract operations" from modules to later
tickets, and say nothing of exporters. 3014 (parked: Switch) plans a
private backend and never says how its repository gets
`shuyi export switch`.

The seam must be public before anyone signs a console's agreement: it
can't be designed inside the private repository. Source: the blind-spot
audit of 2026-11-22 (*Platforms*: "private console backends have no way
into the closed exporter list"; *Architecture*: ADR-0040's deferred
extension points).

The shape, as `cargo` finds `cargo-<name>` and `git` finds
`git-<name>`: a target with no exporter built in is looked up as a
program `shuyi-export-<id>` on `PATH`. That program speaks a small,
versioned JSON protocol, and the CLI runs its own checks before and
after it. `shuyi-cli` gives a `serve` helper, so the private repository
writes an `Exporter` as the built-in ones are and a three-line `main`.
A test exporter in this repository keeps the seam working, so nothing
private is needed to test it.

The game is unaffected. It pins engine `50472bbc`, and
`gh search code --repo Zafnok/visions-of-shuyi 'shuyi_cli::export'`
finds only `shuyi_cli::export::playdate::audio`
(`crates/tools/src/playdate_audio.rs`). It names neither `find`,
`exporters` nor `Exporter`.

## Nick input

None.

## Scope

**In:**
- **An ADR** (the `write-adr` skill), amending ADR-0008 §2 and
  ADR-0040: an exporter outside the engine is a program
  `shuyi-export-<id>`. It covers:
  - how it is found;
  - the protocol and its version;
  - which checks the CLI keeps (it always keeps them: the program is
    never trusted to run them);
  - what an outside exporter can't do yet;
  - the option weighed and refused: exporters registered from the
    game's tool program through `GameTools`. `export --plan` runs
    without that program (`in_tools`, `crates/cli/src/lib.rs:279`), so
    release CI would never see them. And a backend would be tied into
    every game's tool program, not installed once.
- **Finding it.** Built-in ids are never looked up on `PATH`: a
  `shuyi-export-windows` on `PATH` is ignored, and `export` says so in a
  limit. For any other target, `export` and `export --plan` look for
  `shuyi-export-<id>` (`.exe` on Windows) on `PATH`. A target with no
  built-in exporter and no program still answers `no-exporter`. Its
  message then also names the program it looked for.
- **The protocol, version 1.** It is JSON on stdin and stdout, one
  request and one answer per run, and its messages are contract types
  with schemas in `crates/cli/schemas/` (ADR-0022):
  - `describe`: the protocol version, the engine version it was built
    against, and the facts the trait gives today: `id`, `profile`,
    `host`, `container`, `ci`, `triples`, `rustflags` of each triple,
    `builds_a_program`,
    `takes_export_part`, `strips_symbols`, `panic`, whether it has a
    run step, and `bakes_pack` (whether the CLI bakes the game's pack
    for it first).
  - `cargo-args`: given the project's root and the `ExportInput`, the
    `cargo` arguments of each build (`Exporter::cargo_args`).
  - `export`: given the same, the `ExportReport` and the `Shipped`
    list.
  - `run`: given the export's folder and its report, the run step.
- The program builds its own `ExportCx` with `ExportCx::of`, from the
  project's root and the `ExportInput` the CLI hands it. So `ExportCx`
  and its fields need no serde. Only `Shipped` gains `Serialize`,
  `Deserialize` and `JsonSchema` (additive).
- A program whose protocol version isn't 1 is refused before anything
  is built, with a new problem code (the next free `SH30xx`, its page in
  `crates/cli/errors/`, ADR-0028). So is one that exits non-zero or
  answers with JSON that doesn't parse. An engine version that differs
  from the CLI's is a warning in the answer's limits, not a refusal.
- **The checks the CLI runs around it**, in its own process, in the
  order `contract::export::export` runs them for a built-in exporter:
  - `check --target <profile>` first;
  - the disclosure part's problems;
  - `live_link::refuse` on the program's `cargo-args` (no debug
    feature);
  - `ExportCx::bake_pack` with the game's tools when `bakes_pack` is
    true (the CLI has them; the program does not);
  - the program's `export`;
  - then `disclosure::gate` on its `Shipped` list;
  - for a `--release` build, the path search (`paths::check_file`, and
    `paths::check_pack` on a pack) on every file in `Shipped`;
  - then `export-report.json` written by the CLI.
  An outside program can't skip a check by leaving it out.
- `export --plan` lists an outside exporter's `PlannedExport` from its
  `describe`, for each target in `shuyi.ron` with no built-in exporter.
- **`shuyi_cli::export::outside::serve(exporter: &dyn Exporter) -> u8`**,
  the program's side: it reads the request, answers it with the
  exporter's methods and returns the exit code. A private backend
  writes an `Exporter` and
  `fn main() -> ExitCode { serve(&MY_EXPORTER).into() }`.
- **The `&'static str` returns.** `Exporter`'s `id`, `profile`,
  `container`, `triples` and `rustflags`, and `InCi::CheckOnly`'s
  reason, are `&'static str`. A proxy built from
  `describe` has owned strings. The proxy (`OutsideExporter`, which
  implements `Exporter`) keeps them in a process-wide cache, one entry
  per program found. The MCP host lives long, so it caches; it never
  leaks per call. Changing the trait's returns instead is a break: if
  the ADR chooses that, write a `Breaking:` line (ADR-0038). New public
  types are `#[non_exhaustive]` with constructors (ADR-0039).
- **A test exporter** in this repository, `crates/cli/examples/export-fixture.rs`
  (no required feature, so `cargo test` builds it): an `Exporter` that
  writes a small folder and archive from the game's files, run through
  `serve`. A test copies it into a temporary folder as
  `shuyi-export-fixture` and puts that folder first on `PATH`.
- Docs: a section *Exporting from a private backend* in
  `docs/platform-backends.md`, and the operation's prose in
  `docs/contract.md` (the `document` skill).

**Out (do not do):**
- Recipient overlays (`export --recipient`) for an outside exporter.
  `overlays()` is `false` for it and `repackage` is refused. A backend
  that needs them adds them to the protocol in a ticket of its own.
- A `prepare` step other than the pack's bake: the program can't call
  the game's tools. If it needs more, it runs `shuyi bake` or other
  commands itself.
- Any console's exporter, SDK or name. A private backend's work
  happens in its own repository (`AGENTS.md` rule 4, ADR-0008 §2). 3014
  plans the Switch.
- Contract operations, importers, bake steps and settings rows from
  modules, and a backend conformance harness: 3519.
- The extension traits' semver policy before 2005 freezes them: 3518.
  This ticket versions its own protocol, and that is all.
- Installing an exporter with `shuyi add`. 2526 adds module crates, not
  programs. An outside exporter is a program the developer puts on
  `PATH`, as with a cargo subcommand.
- Changing any built-in exporter's behaviour.

**Estimated size:** about 1,400 lines (code 650, tests 450, docs 300).

**Split:** past 1,000 lines, split off steps 6 and 7 (`export --plan`
listing outside exporters, and the run step) into a new ticket.

## Implementation steps

1. Write the ADR (the `write-adr` skill), as the Scope says.
2. Add `crates/cli/src/export/outside.rs`:
   - the request and answer types (`Describe`, `Request`, `Answer` of
     each operation, with serde and `JsonSchema`);
   - `PROTOCOL: u32 = 1`;
   - `find(id) -> Result<Option<OutsideExporter>, ExportError>`, which
     searches `PATH` with `std::env::split_paths` and adds `.exe` on
     Windows;
   - the `OutsideExporter` proxy implementing `Exporter`, with its
     process-wide cache of described programs;
   - `serve`.
3. Derive `Serialize`, `Deserialize` and `JsonSchema` on `Shipped`
   (`crates/cli/src/export/disclosure.rs:91`) and on what it holds
   (`FileKind`). Regenerate the schemas.
4. In `contract::export::export`, use the built-in exporter if there is
   one, else `outside::find`. For an outside one, run the checks the
   Scope lists in that order, then the program's `export`. Then run
   `disclosure::gate` on the `Shipped` it answered, and the path search
   on a release build. Keep `export::find`'s signature for the
   built-ins.
5. Add the problem code and its page, and change the `no-exporter`
   message to name `shuyi-export-<id>`.
6. `contract::export::plan`: for each target in `config.targets` with
   no built-in exporter, ask `describe` of `shuyi-export-<id>` if there
   is one, and list it. Keep the limit for a target with neither.
7. The run step: `Exporter::run` on the proxy sends `run` when
   `describe` says there is one. Otherwise it gives today's
   `NoRunStep`.
8. Write `crates/cli/examples/export-fixture.rs` and the tests below.
9. Write the docs (the `document` skill).

## Acceptance criteria

- [ ] With `shuyi-export-fixture` on `PATH` and a game whose
      `target_profiles` hold a `fixture` profile,
      `shuyi export fixture --json` answers `ok: true`, writes the
      fixture's package and `export-report.json`, and the report names
      the files the fixture shipped.
- [ ] Without the program on `PATH`, the same export answers `SH3010`,
      and its message names `shuyi-export-fixture`.
- [ ] A `shuyi-export-windows` on `PATH` is never run (a fixture that
      fails if it is run proves it). `export windows` builds with the
      built-in exporter, as before.
- [ ] A fixture whose `cargo-args` turns on a debug feature is refused
      before its `export` runs (it writes a marker file if it runs, and
      the test finds none).
- [ ] A fixture that ships a file the game's `disclosure` part denies
      gets it removed and answers `SH3012`, as a built-in exporter does.
- [ ] A `--release` fixture that ships a file holding the project's
      absolute path is refused with `BuildPath`'s code.
- [ ] A fixture answering protocol version 2, one exiting non-zero, and
      one printing text that isn't JSON each get the new problem code.
      Nothing is built in any of them.
- [ ] An engine version in `describe` that differs from the CLI's
      gives a limit, and the export still runs.
- [ ] `export --plan` lists the fixture's `PlannedExport` (host,
      container, ci) for a game with a `fixture` target, and still
      gives the "not packaged" limit for a target with no exporter at
      all.
- [ ] `export fixture --run` runs the fixture's run step.
- [ ] The schemas of the protocol's messages are in
      `crates/cli/schemas/`, and the schema check passes.
- [ ] `docs/platform-backends.md` says how a private backend ships an
      exporter, with the three-line `main`.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `outside::find` on a made-up `PATH` (found, not found, `.exe`
  on Windows, a built-in id never looked up); each request and answer
  round-trips through JSON; the proxy's cache gives the same `&'static
  str` for a second lookup of one program.
- Integration (`crates/cli/tests/it/`): each acceptance case above,
  with the `export-fixture` example copied under the names it needs,
  on the toy project or a fixture project with a `fixture` target
  profile.
- Snapshot: the protocol's schemas.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
