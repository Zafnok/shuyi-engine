# ADR-0067: Each final export is read back into an exposure report and held to a game's disclosure part

- **Status:** Accepted; extends ADR-0058 (what an export package carries)
- **Date:** 2026-11-01
- **Related tickets:** 3081 (this), 3080 (the web's clean staging), 2912 (the
  extraction measurement behind it), 3082 (named demo editions, which build on
  it)

## Context

- A pack keeps every file of its bundle that no format plans for as raw bytes
  (`crates/assets/src/pack/bake.rs`), and a game may embed files in its program
  (`include_dir!`, or a pack by `include_bytes!`, which the pack's docs
  recommend). A pack equality test proves fidelity, not that a release is safe
  to hand out.
- 2912 extracted its planted payloads byte for byte from a ZIP, a loose file and
  a binary overlay. Nothing inspected a real package.
- A hidden level is the game's to name: the engine can't guess what is secret
  (ADR-0002, rule 4). It can say exactly what a package holds and hold it to a
  list the game writes.

## Decision

1. **`export` inspects after the exporter, on every target**
   (`crates/cli/src/export/disclosure.rs`). It reads the package's folder, then
   its archive entry by entry, compared by hash, since the archive is what
   players download; the debug file; and every other file in the export's folder,
   since whoever uploads a folder uploads those too.
2. **Every export writes `exposure-report.json`**, with or without a manifest:
   each file's kind, source, size and SHA-256, each pack entry, the findings, and
   the build inputs (cargo arguments, resolved features, `rustc -V`, the hashes of
   `shuyi.ron` and `Cargo.lock`). It holds no path of this machine and is sorted,
   so the same input gives the same bytes. Its shape is a contract schema,
   `export.exposure.json`.
3. **The manifest is a `disclosure` part of `shuyi.ron`, one for all targets:**
   `allowed_files`, `allowed_pack_entries`, `forbidden_paths`, `canaries`. Paths
   match whole, and a pattern is refused before any build: a pattern that matches
   a new file is how a leak gets through. A game that lists nothing isn't held to
   anything.
4. **The exporter says what it writes** (`Exporter::shipped`): the program, the
   notices, the shell, the pack's file. These need no line, because the game didn't
   choose them and their names differ by target. Only the game's files and pack
   entries need one. A target that says nothing has only undeclared files, so a new
   exporter fails closed.
5. **What is searched.** Canaries as UTF-8 and UTF-16 in every file, and in what
   each pack entry unpacks to, since LZ4 can split a text. Packs are found by
   their magic bytes and version anywhere in a file, so a pack a program embeds is
   listed like one beside it, and one that doesn't read refuses the export.
   Forbidden paths are also searched as text in programs, with either slash:
   `include_dir!` keeps its paths there. A Playdate's `pdex.bin` is compressed, so
   the `pdex.elf` it is made from is read as a build input.
6. **A refusal leaves no package:** the folder, the archive, the debug file, the
   last `export-report.json` and the contents of `web.dist` go, and the report
   stays. A file beside the package that this export didn't make refuses the export
   but stays, since it isn't this export's to delete. A code of its own, `SH3012`.
7. **Canaries are never printed.** The report and the errors name
   `disclosure.canaries[n]`, so the report can go in a CI artifact or to a tester
   without naming what is coming. A forbidden path appears only when it was found.
8. **Reproducible by fixing the time, where a tool lets us.** `objcopy` honours
   `SOURCE_DATE_EPOCH`, set to the archives' day; the Simulator's DLL links with
   `--no-insert-timestamp`. `pdc` has no such setting, and whether a device reads
   `buildtime` isn't documented, so that line stays as written: `Shipped::varying`
   leaves it out of `pdxinfo`'s hash, the Playdate zip's size and hash are of its
   files, and both entries say so (`varies`).
9. **SHA-256 is written here,** in 60 lines tested against NIST's values, because
   it names bytes and guards no secret, and `sha2` would add crates to every new
   game's tree (`tools/compile-budget.ron`).

## Consequences

- A release has an auditable list, and CI's `export` job exports Hello, Grid twice
  on each target and compares the reports.
- A stale file in the export's folder, an old version's archive or a `.debug` from
  an earlier `--release` now refuses a game with a part until it is removed. That
  is the point, and the message says so.
- The Playdate lists differ from the others' (`audio/x.pda`), so a game's part
  holds the union; an allowed file that a target doesn't ship is not an error.
- A pack `wasm-opt` splits in a `.wasm` doesn't read, and refuses the export: safe,
  and the error's page says to ship that pack as a file.
- It finds what is written plainly. Bytes a game compresses or encrypts itself,
  images and sounds `pdc` converts, and what a compiler makes of a rule aren't
  searched. The book says so: a pass means no declared text was found, not that
  nothing unreleased is there. Omitting content is 3082's job.
- `shuyi web` alone isn't inspected. Its folder is emptied when an export refuses.

## Alternatives considered

- **Glob patterns in the lists** — shorter, but `maps/*` allows next year's file.
  The ticket asks for exact paths.
- **A list per target profile** — the ticket keeps it independent of profiles, and
  3082's editions are the axis that needs separate lists.
- **The author lists every file, the program too** — names like `pdex.dll` and the
  web shell aren't the game's to know, and a new shell file would break every game.
- **Inspecting only the folder** — the archive is what ships, and a difference
  would go unseen.
- **Rewriting `buildtime` to a fixed value** — a byte-identical `.pdx`, but it
  changes what the SDK's tool wrote in a way that can't be tested without knowing
  how a device reads it.
- **`sha2`** — the usual choice, at the cost of crates in every new game for one
  function of a tool.
- **Running `strings` or `binwalk`** — another tool on every machine, and neither
  reads a pack.
