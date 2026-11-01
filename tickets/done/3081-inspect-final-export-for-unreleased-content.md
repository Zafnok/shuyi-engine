---
id: "3081"
title: "Inspect each final export against a game's disclosure manifest"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: done
blocked_by: ["3080"]
nick_input: none
completed: 2026-11-01
---

# 3081 — Inspect the final export for unreleased content

## Context

Shuyi's asset baker keeps unplanned bundle files as raw bytes (crates/assets/src/pack/bake.rs), and a game may embed files into its program. A pack equality test proves fidelity, not release safety. Build an auditable list of the bytes distributed to a player. A web build already holds only what it made (3080). **From 2912 (done):** all four intentionally present pilot payloads were extracted byte-exact from a ZIP, a loose file and a native binary overlay; that smoke test did not inspect a real Shuyi export. This ticket tests the real final packages.

## Nick input

None.

## Scope

**In:**
- A game-authored disclosure manifest of allowed paths and forbidden paths or canary strings for an export.
- A machine-readable exposure report for final desktop, web and Playdate packages: files, pack entries, loose assets, feature set and debug companions.
- A failing export when declared forbidden material or undeclared package files are found.

**Out (do not do):**
- A claim that arbitrary game semantics are absent from machine code.
- Per-recipient watermarking, DRM, or changes to game rules.

## Implementation steps

1. Define the manifest under shuyi.ron with exact path matching and clear errors; keep it independent of target profiles.
2. Inspect the final package and pack index after all target-specific copies, and scan bounded binary/Wasm strings for author-declared canaries.
3. Write an exposure report with hashes and the resolved build inputs; make a failed gate leave no publishable archive.

## Acceptance criteria

- [x] A harmless future-content file in a pack, loose folder or adjacent download fails with its source and package path.
- [x] A declared canary embedded in a binary or Wasm fails the gate.
- [x] An allowed fixture game exports on every supported target and has a reproducible report.
- [x] The book's chapter is updated (2802), or the completion notes have a Book: line saying why not.
- [x] All gates in the run-gates skill pass.

## Tests required

- Integration: canaries in raw packs, loose files, embedded strings, debug sidecars and final archives.
- Property: report ordering and hashing are deterministic for identical inputs.

## Completion notes

Every `export` now reads its final package back and writes
`exposure-report.json`: the folder, the archive entry by entry, every pack entry
(a pack a program embeds too), the debug file and anything else beside the
package, with hashes and the resolved build inputs. A `disclosure` part in
`shuyi.ron` holds it to a list; what breaks it refuses the export with `SH3012`,
names the file and its source, and leaves the report and no package.
[ADR-0070](../../docs/adr/0070-each-final-export-is-read-back-into-an-exposure-report-and-held-to-a-games-disclosure-part.md)
has the choices.

Checked on real packages of Hello, Grid, whose `shuyi.ron` is now the allowed
fixture, on this machine: `windows`, `web` and `playdate` export with `--release`,
each twice with byte-identical reports. A canary set to a text the game really
holds refused all three, and left only the report; on the Playdate only
`pdex.elf` showed it, as `pdex.bin` is compressed. `playdate` isn't in CI (no
SDK), and neither `linux` nor `macos` has run here (each builds on its own
system): CI's `export` job runs `linux` on each PR and `macos` on a push to
`main`, with a new step that exports twice and compares the reports. The
integration tests use stand-in programs through the real packagers.

Deviations and additions the ticket didn't name:

- Reproducible reports took two package changes: `objcopy` now runs with
  `SOURCE_DATE_EPOCH`, and the Simulator's DLL links with `--no-insert-timestamp`.
  `pdc` has no such setting, so `pdxinfo`'s hash leaves out its `buildtime=` line,
  and the Playdate zip's size and hash are of its files; both entries say so in
  `varies`. Linux and macOS have no such handling, and rely on CI's comparison.
- A refused web export also empties `web.dist`, since it holds the same bytes.
- Canaries are never printed, in the report or the errors, so a report can be
  shared without naming what is coming.
- Forbidden paths are also searched as text in programs, with either slash:
  `include_dir!` keeps its paths there. An embedded pack that doesn't read
  refuses the export, which a pack split by `wasm-opt` would do.
- SHA-256 is written in the CLI, not taken from `sha2`, to add no crate to a new
  game's tree (`tools/compile-budget.ron`).
- `shuyi_assets::pack::length` is new, for finding a pack inside a program.
- The report's shape is a contract schema, `export.exposure.json`, so the
  contract moved to version 33.

It finds what is written plainly: a pass isn't proof nothing unreleased is there.
`shuyi web` alone isn't inspected. The part is one per project, and editions are
handed to 3082 in its text.

No break: `Config` and the new types are `#[non_exhaustive]`, `Exporter::shipped`
has a default, and no existing public item changed shape. A game with no
`disclosure` part is refused nothing; it gains the report file.

Book: `targets-and-export.md`, *What a package gives away*.

The game needs nothing to move its pin.
