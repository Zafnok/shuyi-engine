---
id: "2215"
title: "Parked: In-house file embedding: the shuyi-embed crate writes a game's file table at build time on every target, and include_dir is removed"
type: feature
milestone: "Engine: in-house platform"
model: sonnet-5
effort: medium
status: blocked
blocked_by: []
nick_input: none
completed:
---

# 2215 — In-house file embedding, and include_dir removed

> **Parked** (Nick, 2026-10-19, chose "Park until MVP" for the in-house platform layer, `docs/decisions.md`): after the engine reaches parity with the game (the MVP in `docs/ROADMAP.md`). Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title. In the roadmap's order it comes with 3033, after controllers, but it needs nothing else, so it can start once unparked.

## Context

Nick, 2026-10-19 (`docs/decisions.md`): "the two small ones (web saves,
asset packing)" go in-house. A game puts its files in its binary in one of
two ways (`crates/assets/src/bundle.rs`):

- `Files::Dir`: `include_dir!` (`include_dir` 0.7.4, last release June
  2024), only with `std`;
- `Files::Table`: a sorted table of `(path, bytes)` that a build script
  writes, for builds without `std` (the Playdate).

The table already does everything the directory does, on every target. So
the fix is to make the table easy and drop the directory. The work is
ours: a build-script helper in a crate of its own
([ADR-0021](../../docs/adr/0021-the-platform-layer-in-house-clean-room-pieces-in-their-own-crates.md)
§3), written from Cargo's build-script documentation, never from
include_dir's source (§1).

Related: 2204 (the optimized pack) is a different way to ship files and
doesn't change. 2207 adds layers from outside the binary, and either order
works: 2207 doesn't touch how embedded layers are built.

## Nick input

None.

## Scope

**In:**
- **The crate** `crates/embed` (`shuyi-embed`), MIT OR Apache-2.0, no
  `shuyi-*` dependency, used as a build dependency:
  - `shuyi_embed::Table::new("files").dir("files").write()`, called from a
    game's `build.rs`: walks the folder, sorts paths (`/`-separated,
    relative to the folder), and writes
    `$OUT_DIR/shuyi_embed_files.rs` holding a
    `&'static [(&'static str, &'static [u8])]` of `include_bytes!` with
    absolute paths;
  - prints `cargo::rerun-if-changed` for the folder and every file and
    sub-folder, so adding, changing or removing a file rebuilds;
  - `.skip(pattern)` for files to leave out, which also lists them for
    `Layer::left_out`;
  - an error (the build fails with the path) for non-UTF-8 paths, and for
    two paths that differ only by case, since they collide on Windows and
    macOS;
  - `shuyi_embed::files!("files")`, a `macro_rules!` that expands to
    `include!(concat!(env!("OUT_DIR"), "/shuyi_embed_files.rs"))`.
- **`shuyi-assets`**: remove `Files::Dir` and `Layer::dir`, and the
  `include_dir` dependency from `crates/assets/Cargo.toml` and the
  workspace. `Files::Table` and `Layer::table` stay as they are.
- **`examples/hello-grid`** moves to `shuyi-embed`.
- **Docs**: the bundle's docs and any guide that shows `include_dir!`
  show `shuyi-embed` instead.
- **For the game:** this breaks the game's API use: `Layer::dir` is gone.
  The PR says so, and the game's PR that moves the pin follows it,
  replacing `include_dir!` with a `build.rs` call (a few lines per layer).
  Write that game ticket (`write-ticket`, *A request from the game*, in the
  other direction) and link it.

**Out (do not do):**
- Compression or packing (2204).
- Changing what a layer is or how it's searched.

## Implementation steps

1. `crates/embed` with `src/lib.rs` (the builder, the writer, the macro).
2. Tests: the builder on a temporary folder writes the table you expect;
   the generated file compiles (an integration test crate with a
   `build.rs` that uses it).
3. Move `hello-grid`, then remove `Files::Dir`, `Layer::dir` and
   `include_dir`.
4. Build the game against this working copy (`run-gates`, *The game at the
   next pin*) with its own change applied in its checkout, so the game's
   PR is known to work.
5. In the completion notes: the game ticket and the documents used.

## Acceptance criteria

- [ ] `hello-grid` builds and runs natively and on the web with its files
      from `shuyi-embed`.
- [ ] Changing, adding or removing a file under `files/` rebuilds the
      table on the next `cargo build`.
- [ ] Two files differing only by case fail the build with both paths.
- [ ] `cargo tree --workspace` shows no `include_dir`.
- [ ] The game ticket for the pin move exists and is linked.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: path sorting and `/` separators on Windows; skip patterns; the
  case collision; non-UTF-8 paths.
- Integration: the test crate (step 2).

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
