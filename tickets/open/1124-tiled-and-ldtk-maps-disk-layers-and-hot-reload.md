---
id: "1124"
title: "Parked: Tiled and LDtk maps, and hot reload of the folders on disk in debug builds"
type: feature
milestone: "Engine: real time"
model: opus-5.5
effort: high
status: blocked
blocked_by: ["1119", "2207"]
nick_input: none
completed:
---

# 1124 — Tiled and LDtk maps; hot reload

> **Parked** (Nick, 2026-10-14: "document the parked (i.e. after MVP/parity) tickets for networking, styles for later"): after the engine reaches parity with the game (the MVP in `docs/ROADMAP.md`). Unparked early only if the game's ticket 0060 says the army walks around freely (then 1119–1125 become game work). Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

ADR-0004 §1–§2. Developers draw maps in Tiled or LDtk. The `tiled` crate
(MIT) reads TMX/TSX in pure Rust, but only with default features off,
because its `zstd` feature builds C code. LDtk's crates are stale, but
LDtk publishes a JSON schema and generated serde types (MIT). Both
importers produce one `TileMap`:

- tile layers: a `Grid` of tile ids per tileset;
- object layers: named boxes and points with properties;
- the map's own properties.

Folders on disk become bundle layers in 2207 (a game's mods first:
game#1012 and game#1014); this ticket reloads them.

## Nick input

None.

## Scope

**In:**
- `shuyi_assets::tilemap::TileMap`, an asset kind (`AssetKind`,
  ADR-0019 §1), and the two importers as `Format`s of it (ADR-0007,
  ADR-0019; `docs/formats.md` says how to write one), registered with
  `Formats::register_standard`, each with its id, extension (`.tmx`,
  `.ldtk`), limit and a property test that random bytes never panic it:
  - Tiled: `tiled` with `default-features = false` (its TSX tilesets are
    a `Format` of the tileset kind, or files the TMX format reads through
    its `DecodeCx`);
  - LDtk: the vendored generated types with their MIT notice in
    `THIRD_PARTY_ASSETS.md`.
  Both have fixtures made with each tool's free editor.
- `TileCollision` from a named layer (1122).
- Disk layers are 2207's: `shuyi-app` reads a folder into a layer
  (only `shuyi-app` touches files, game ADR-0004).
- Hot reload in debug builds: `shuyi-app` checks the modification times
  of the disk layers' files once a second. It then calls a reload hook
  that re-runs the loaders for the changed files. Errors show in a debug
  overlay instead of crashing.
- `shuyi-cli import <map> --print` prints the map as our text.

**Out (do not do):** mods' folder and load order (1014); the web build's
disk layers (none on the web).

## Implementation steps

1. Write `TileMap` and the Tiled importer, with fixtures.
2. Add the LDtk types and importer, with fixtures.
3. Add hot reload of 2207's disk layers, with a test that changes a fixture
   file through a fake clock and file source.

## Acceptance criteria

- [ ] Both fixtures import to the same expected `TileMap` (a snapshot of
      its text dump).
- [ ] A broken fixture names the file and the place in it.
- [ ] The hot reload test passes.
- [ ] `cargo deny check` passes, and the WASM build passes with both
      importers.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Fixture and snapshot tests as above.
- Unit: reload with a fake file source.

## Completion notes
