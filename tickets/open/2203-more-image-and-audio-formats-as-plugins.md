---
id: "2203"
title: "More image and audio formats as plugins: JPEG, WebP, QOI, JPEG XL, FLAC, MP3, each behind a feature"
type: feature
milestone: After MVP
model: sonnet-5
effort: high
status: todo
blocked_by: ["2201"]
nick_input: none
completed:
---

# 2203 — More image and audio formats

## Context

ADR-0007 §3. The engine always loads PNG, WAV and OGG. Developers also
bring JPEG photos and backdrops, WebP, QOI (very fast to decode) and
JPEG XL (often much smaller than PNG for the same pixels, lossless or
lossy). For music they bring FLAC and MP3. The game's own music already
has MP3 and FLAC files (game ADR-0026). Each format here is a plugin
behind its own cargo feature, so a game pays only for what it uses.

Known trap (ADR-0004 §6, ADR-0004 §7): `symphonia`, the common Rust audio
decoder, is MPL-2.0 and denied. Check every crate's license from its
`Cargo.toml` and repository on the day, not from memory.

## Nick input

None.

## Scope

**In:**
- Image `Format`s:
  - JPEG (for example `zune-jpeg`);
  - WebP (for example `image-webp`);
  - QOI;
  - JPEG XL decoding (for example `jxl-oxide`).

  Use the named crates only if their licenses pass ADR-0002 §8 and they
  build for `wasm32-unknown-unknown` without `wasm-bindgen`. Pick another
  or leave the format out (with a note) if not.
- Audio `Format`s:
  - FLAC (for example `claxon`);
  - MP3 (a pure-Rust decoder under an allowed license; leave it out with
    a note if none qualifies).
- **Each format has:**
  - a feature;
  - fixtures (our own files, CC0);
  - a decode test against a checked-in pixel or sample hash
    (determinism, ADR-0007 §5);
  - a fuzz-style test.
- **A size table.** `docs/formats.md` gets the WASM size each feature
  adds.

**Out (do not do):** encoding (only decoders); the optimized pack (2204);
anything learned or neural (2901).

## Implementation steps

1. Check each crate's license, WASM build and maintenance. Record what
   you found in `docs/formats.md`.
2. Write the formats one by one, with their tests.
3. Measure and record the size table.

## Acceptance criteria

- [ ] Every format that passed step 1 decodes its fixtures to the
      checked-in hashes on native and web (or the CI equivalent).
- [ ] `cargo deny check` passes with every feature on.
- [ ] `docs/formats.md` has the license and size table.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Fixture, determinism and property tests per format.

## Completion notes
