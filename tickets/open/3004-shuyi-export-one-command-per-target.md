---
id: "3004"
title: "shuyi export <target>: one command that checks a game and packages it for a target, starting with Windows, Linux, macOS and the web"
type: feature
milestone: After MVP
model: sonnet-5
effort: medium
status: todo
blocked_by: ["3001", "1129"]
nick_input: none
completed:
---

# 3004 — `shuyi export`

## Context

[ADR-0008](../../docs/adr/0008-export-targets-profiles-as-data-platform-backends-controls-that-fit-one-bit-and-budgets.md).
Each target needs a package:

- a zip per desktop OS;
- a folder for the web;
- a Steam Deck build (3012);
- a `.pdx` for the Playdate (3010).

One operation of the tool contract (ADR-0005), `export`, runs the
target's checks and then its exporter, so an agent or a player gets the
same result from the CLI, MCP and skills. This ticket builds the command
and the exporters that need no new backend. The target tickets add
theirs.

## Nick input

None.

## Scope

**In:**
- **`shuyi export <target> [--out <dir>] [--release]`:**
  1. it runs `shuyi check --target <target>` (3001, and 3002 once it
     lands) and refuses on any error;
  2. it builds with the target's Rust triple and the backend's crate;
  3. it collects the assets, packed if the game asks for it (2204,
     optional);
  4. it writes the package and a `export-report.json` (target, version,
     sizes, the check's warnings).
- **The `Exporter` trait** in `shuyi-cli`: `id()`, `triples()`,
  `export(&ExportCx) -> Result<ExportReport, ExportError>`. Exporters
  are registered by target id. A missing tool (a missing Rust target,
  the Playdate SDK) gives an error that says how to install it.
- **Exporters in this ticket:**
  - **`windows`.** `x86_64-pc-windows-gnu`, then a zip with the exe,
    `LICENSES/` and the third-party notices. MSVC isn't needed.
  - **`linux`.** `x86_64-unknown-linux-gnu`, then a `tar.gz`. It is
    built in CI, not on Nick's machine.
  - **`macos`.** It builds in CI only, on macOS runners. A `.app` folder,
    unsigned; signing is a later ticket and needs Nick's Apple account.
  - **`web`.** It reuses the existing web build, and the output folder
    holds `index.html`, the wasm and the JS plugins.
- **License notices.** Every package carries the engine's licenses and
  the third-party notices, generated from `cargo deny` /
  `cargo about`-style data (pick a tool under ADR-0002 §8, check its
  license, and note it here).
- **CI.** Export every example for `windows`, `linux` and `web` on every
  PR. Upload the packages as artifacts for 7 days.

**Out (do not do):**
- signing and notarising (macOS, Windows), which need Nick's accounts;
- store uploads (Steam is the game's 0903);
- the Steam Deck (3012), Playdate (3010) and mobile (3015) exporters.

## Implementation steps

1. Add the `export` operation to the contract, with its JSON shape.
2. Write the `Exporter` trait and the four exporters.
3. Add the notices generation.
4. Add the CI job.

## Acceptance criteria

- [ ] `shuyi export windows` on an example, on Nick's machine (GNU
      toolchain), gives a zip whose exe runs.
- [ ] `shuyi export playdate` on a game that fails `check --target
      playdate` refuses, and prints the check's errors.
- [ ] A missing Rust target gives "run `rustup target add …`", not a
      cargo error dump.
- [ ] Each package has the licenses and notices (test).
- [ ] CI exports the examples for `windows`, `linux` and `web`.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the report, and the refusal on failed checks.
- Integration: export a fixture game for `web` and `linux` in CI, and
  check the package's file list against a snapshot.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
