---
id: "2204"
title: "An optimized pack format for shipping: shuyi bake, one indexed compressed file, proven equal to its sources"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: done
blocked_by: ["2201", "1129"]
nick_input: none
completed: 2026-10-20
---

# 2204 — An optimized pack format for shipping

## Context

ADR-0007 §4. Nick: "if we want to introduce a highly optimized file
format, this should be easier than ever" (`docs/decisions.md`). This is
the first optimized format and the proof of the rule. Sources stay text.
`shuyi bake` turns them into one packed file the game loads faster. A
test proves that every asset decoded from the pack equals the same asset
decoded from its source.

Today the game embeds its assets with `include_dir!` (game ADR-0004) and
parses text at start-up. The pack is an alternative a game can choose,
not a replacement.

## Nick input

None.

## Scope

**In:**
- **The pack format** (`.shuyipak`):
  - a header with a version;
  - an index of asset paths and kinds with offsets;
  - per-asset blobs, each already decoded into the engine's binary form
    (a `bincode`-like encoding with serde, versioned), compressed with a
    pure-Rust compressor under an allowed license (for example
    `lz4_flex`).

  The pack is itself a bundle layer (game ADR-0040's layers), so a game
  can put a pack under its loose files.
- **A `no_std` reader.** The code that reads a pack builds with
  `default-features = false` as `no_std` with `alloc`, because the
  Playdate loads only packs (ADR-0008 §5, ticket 3010). Pick an encoding
  and a decompressor that work without `std`: for example `postcard` and
  `lz4_flex` with `default-features = false`, licenses checked. Add the
  reader to 3005's `thumbv7em-none-eabihf` CI build if that ticket has
  landed. If it hasn't, 3005 adds it.
- **A `--target` hook.** `shuyi bake --target <id>` passes the target
  to each format's encoder, and picks the target's bundle layer. This
  ticket only adds the parameter; 3007 uses it for 1-bit images.
- **`shuyi bake`.** Sources → pack. It refuses a source with errors, and
  prints sizes and times.
- **Equality.** For every asset in a game, decode(source) ==
  decode(pack). This is a test in `shuyi-assets` over fixtures, and
  `shuyi check --baked` over a whole game.
- **Measurements.** On the game's public assets, load time and size,
  loose files against the pack, native and web. They go in
  `docs/formats.md`.

**Out (do not do):**
- encryption or obfuscation;
- streaming parts of a pack (write a ticket if measurements show a need).

## Implementation steps

1. Write the format and its version rule. An older pack is refused with a
   message to re-bake it, never migrated.
2. Write `bake` and the equality checks.
3. Do the measurements, and record them in the docs.

## Acceptance criteria

- [x] The equality test passes on every fixture kind.
- [x] `shuyi check --baked` passes on an example game.
- [x] A truncated or changed pack gives an error, never a panic (property
      test).
- [x] The measurements are recorded.
- [x] The pack reader builds with `--no-default-features`.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Property: round trip and corruption.
- Integration: bake and load an example game.

## Completion notes

- **The format** is `shuyi_assets::pack` ([ADR-0023](../../docs/adr/0023-the-pack-one-indexed-file-of-baked-assets-proven-equal-to-its-sources.md)):
  - a header (magic, `pack::VERSION`, a checksum, the index's length);
  - a postcard index (stamp, target, each file's path, kinds, codec,
    offset and sizes);
  - each file's bytes, LZ4-compressed when that is smaller.
- **Baked files keep their source's path.** A baked file holds one
  postcard section per kind it was read as. `Packed<K>` finds it by its
  magic bytes, so loaders don't change. Every other file (PNG files, sounds)
  keeps its own bytes.
- **A pack is a bundle layer** (`Pack::into_layer`). Stored files borrow
  the pack's bytes.
- **The reader is `no_std`.** It builds in the existing
  `thumbv7em-none-eabihf` job (3005's job had landed), with `postcard`
  and `lz4_flex`, both without `std` and under MIT / Apache-2.0.
- **Baking and the proof.** `Bake` (`asset::<K>(id)`, `under::<K>(dir)`,
  a stamp), `Bake::bake`, `Bake::check` (decode(source) == decode(pack)
  for every baked asset; every other file byte-equal) and `Bake::load`.
- **Versions.** An old or foreign pack is refused with "bake it again":
  another `VERSION`, another stamp, cut short or changed. An `insta`
  snapshot pins the binary form of every engine kind, so a change forces
  a `VERSION` bump.
- **The engine's kinds got serde** in both directions (and `CinematicFile`
  got `PartialEq`). The fixtures found one real bug: a map's look skipped
  its default light when serialized, which postcard can't read back. Maps
  now write their look whole (`grid_map::look_in_full`).
- **The tools.**
  - `shuyi bake [--target <id>] [--out <file>]` is a contract op. It
    prints sizes, the bake time, and the load time from the sources and
    from the pack.
  - `shuyi check --baked` runs the proof.
  - Both go through `GameTools::bake`.
  - The contract goes to version 3 (after 3001's 2), with new schemas.
  - `--target` is passed to the game as it is: the game picks its layers
    and kinds by it.
- **Hello, Grid** now loads through the registry (`Hello::load_from`),
  bakes its text (`hello_grid::bake`) and loads from a pack
  (`Hello::from_pack`). Its tests prove the game loaded from the pack
  equals the game loaded from its files, and that `check --baked`
  passes. Its errors now name files as `files/<path>`.
- **Measurements** are in `docs/formats.md`, native and web:
  - Hello, Grid loads 2.5× (native) to 4× (web) faster from its pack, at
    half the size.
  - The game's public assets: the pack is 74% of their size, and the
    kinds the engine can bake without the game's types load 25% faster.
  - The first design (copying every file and a byte-at-a-time checksum)
    made the game's pack no faster. Borrowing stored files and a word-wide
    checksum fixed that before merging.
- **Deviations.**
  - The ticket's measurement "on the game's public assets" covers sizes
    for every file. Load times cover only the engine kinds among them:
    the game's own tables need its types to derive serde, which is the
    game's change.
  - The pack's bytes differ between a 64-bit and a 32-bit baker (LZ4).
    The decoded assets don't.
- **API changes for the game's next pin.**
  - `CheckInput` has a field (`baked`).
  - The contract is version 3.
  - `GameTools` has a new method with a default, so nothing breaks.
  - Hello, Grid's `Hello::load` error names changed.
  - No game call site breaks. Adopting the pack is the game's own
    ticket, through its `engine-change` process.
- **Rebased onto 3001** ([Zafnok/shuyi-engine#29](https://github.com/Zafnok/shuyi-engine/pull/29)):
  - the contract is version 3, after 3001's 2;
  - `CheckInput` has both `target` and `baked`; `check` takes none,
    `--baked`, or `--target <id | all>`;
  - `bake --target` refuses a target with no profile, the engine's or the
    game's;
  - `TargetProfile` derives `Serialize` and is baked in the fixtures;
  - `Button` is written by its name, as 3001 reads it.
- No follow-up tickets. Nick input: none.
