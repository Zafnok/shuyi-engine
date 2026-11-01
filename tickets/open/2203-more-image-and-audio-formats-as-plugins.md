---
id: "2203"
title: "More image and audio formats as plugins: JPEG, WebP, QOI, JPEG XL, FLAC, MP3, each behind a feature"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
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
  - JPEG XL decoding (for example `jxl-oxide`), in the engine, not in
    2901's separate repository. **Either order with 2901:** if 2901 is
    done, read its report's JPEG XL numbers when you pick the crate; if
    not, nothing to do (2901 measures this decoder as its baseline).

  Use the named crates only if their licenses pass ADR-0002 §8 and they
  build for `wasm32-unknown-unknown` without `wasm-bindgen`. Pick another
  or leave the format out (with a note) if not.
- Audio `Format`s:
  - FLAC: `claxon` (Apache-2.0), selected by 2902;
  - MP3: `nanomp3` (MIT OR Apache-2.0), selected by 2902. Recheck both
    manifests, repository licenses and the resolved graph on the day;
    use defaults off and only the std/alloc features the adapter needs.
    Keep unsupported no-std targets behind the format feature fence;
    neither a pure Rust implementation nor `no_std` alone proves a
    compressed track fits a handheld's runtime budget.
- **From 2201 (done):** an image is two kinds (ADR-0019 §6). Each
  image codec is a `Format` of `image::Image` (its RGBA pixels) and has
  a size-only twin of `image::ImageInfo`, as `Png` has `PngInfo`, so
  the image table (`ImageTable::load_with`) sizes the file. An audio
  codec is a `Format` of `audio::Sound` (the header), and of
  `audio::MusicTrack` for music, as `OggVorbis` and `OggVorbisTrack`
  are.
- **Each format has:**
  - a feature;
  - fixtures (our own files, CC0);
  - a golden row: the hash of what it decodes from its fixture (pixels
    for an image, the header for a sound) in 2116's
    `crates/assets/tests/golden/formats.ron`, named
    `<kind>: <format id>` as 2116 names its rows (determinism,
    ADR-0007 §5). Its feature
    joins `shuyi-assets`'s self dev-dependency, so CI's `test` job
    (x64, ARM64) and `determinism` job (wasm32) build and check it.
    **From 2116 (done):** the row is a `rows.format(…)` line in
    `crates/assets/src/format/golden_tests.rs`, blessed with
    `cargo xtask golden --bless`; its guard `missing_rows` already
    covers `Image`, `ImageInfo`, `Sound` and `MusicTrack`, so a format
    with no row fails it;
  - a fuzz-style test. **Either order with 2020:** if 2020 is done,
    the format's feature also joins the `fuzz` feature's list in
    `crates/assets/Cargo.toml`, so its fuzz registry runs it; if not,
    nothing to do (2020 adds the features this ticket has added).
- **A size table.** `docs/formats.md` gets the WASM size each feature
  adds.

**Out (do not do):** encoding (only decoders); the optimized pack (2204);
anything learned or neural (2901); pinning decoded audio samples (a
sound's asset is its header, as in 2116's Out); drawing and playing
these formats (2234, blocked by this one): `shuyi-app` and `frame-png`
decode only PNG pixels, and macroquad plays only WAV and OGG.
`docs/formats.md` says so beside the size table.

## Implementation steps

1. Check each crate's license, WASM build and maintenance. Record what
   you found in `docs/formats.md`.
2. Write the formats one by one, with their tests.
3. Measure and record the size table.

## Acceptance criteria

- [ ] Every format that passed step 1 decodes its fixtures to the
      hashes in 2116's `crates/assets/tests/golden/formats.ron` (pixels
      for an image, the header for a sound; decoded samples aren't
      pinned), checked by CI's `test` job (x64, ARM64) and
      `determinism` job (wasm32), with the formats' features added to
      `shuyi-assets`'s self dev-dependency.
- [ ] `cargo deny check` passes with every feature on.
- [ ] `docs/formats.md` has the license and size table.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Fixture, determinism (a golden row) and property tests per format.

## Completion notes
