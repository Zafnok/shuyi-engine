---
id: "2204"
title: "An optimized pack format for shipping: shuyi bake, one indexed compressed file, proven equal to its sources"
type: feature
milestone: After MVP
model: opus-5.5
effort: high
status: todo
blocked_by: ["2201", "1129"]
nick_input: none
completed:
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

- [ ] The equality test passes on every fixture kind.
- [ ] `shuyi check --baked` passes on an example game.
- [ ] A truncated or changed pack gives an error, never a panic (property
      test).
- [ ] The measurements are recorded.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Property: round trip and corruption.
- Integration: bake and load an example game.

## Completion notes
