---
id: "2203"
title: "More image and audio formats as plugins: JPEG, WebP, QOI, JPEG XL, FLAC, MP3, each behind a feature"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: high
status: done
blocked_by: ["2201"]
nick_input: none
completed: 2026-11-22
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

- [x] Every format that passed step 1 decodes its fixtures to the
      hashes in 2116's `crates/assets/tests/golden/formats.ron` (pixels
      for an image, the header for a sound; decoded samples aren't
      pinned), checked by CI's `test` job (x64, ARM64) and
      `determinism` job (wasm32), with the formats' features added to
      `shuyi-assets`'s self dev-dependency.
- [x] `cargo deny check` passes with every feature on.
- [x] `docs/formats.md` has the license and size table.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Fixture, determinism (a golden row) and property tests per format.

## Completion notes

What was done:

- **Step 1, the crates** (checked 2026-11-22, recorded in
  `docs/formats.md`, *Formats behind a feature*): every named crate
  passed, so all six formats shipped. Licences from each `Cargo.toml`
  and the licence files in its package: `zune-jpeg` 0.5.15 (MIT OR
  Apache-2.0 OR Zlib), `image-webp` 0.2.4, `qoi` 0.4.1, `jxl-oxide`
  0.12.6 (its `brotli-decompressor`: BSD-3-Clause OR MIT), `nanomp3`
  0.2.0 (MIT OR Apache-2.0), `claxon` 0.4.3 (Apache-2.0). All with
  `default-features = false` (no `x86`/`neon` SIMD in `zune-jpeg`, no
  `rayon` in `jxl-oxide`, `nanomp3` with `alloc` only), all build for
  `wasm32-unknown-unknown` with no `wasm-bindgen` in the graph, and
  `cargo deny check` passes with every feature on. `jpeg`, `qoi` and
  `mp3` also build without `std` (`thumbv7em-none-eabihf`); `webp`,
  `jpeg-xl` and `flac` turn on `shuyi-assets`' `std`.
- **Features** `jpeg`, `webp`, `qoi`, `jpeg-xl`, `flac`, `mp3` of
  `shuyi-assets`, all in its self dev-dependency, so CI's `test` and
  `determinism` jobs build and check them. The six crates are in
  `tools/layers.ron`'s outside list.
- **Pictures** (`crates/assets/src/image/codecs.rs`): `image::{Jpeg,
  JpegInfo, WebP, WebPInfo, Qoi, QoiInfo, JpegXl, JpegXlInfo}`, each
  pair made by one macro over a small codec module (id, extensions,
  sniff, size, pixels). The pixels format checks the size first
  (`ImageInfo::checked`), then that the decoder gave `w × h × 4` bytes.
- **Sounds** (`crates/assets/src/audio/formats.rs`): `audio::{Flac,
  FlacTrack, Mp3, Mp3Track}`, headers only, mono or stereo at 44.1 kHz
  as WAV and OGG. A FLAC track's length is its `STREAMINFO` sample
  count; an MP3 track's is `nanomp3`'s scan of every frame, gapless.
  `ogg_length_ms`'s rounding moved to `samples_ms`, which the tracks
  share.
- **Registered** by `Formats::register_standard` under each feature
  (a `cfg` block each), after the PNG and OGG formats, so
  `ImageTable::load_with` sizes the files. 2234 asks for this; doing it
  here also makes 2116's `missing_rows` guard require their rows.
- **Fixtures** (`crates/assets/tests/fixtures/formats/`, ours, CC0, with
  a README and the two scripts that made them): a 20 × 12 RGBA picture
  as PNG, JPEG, lossy and lossless WebP, QOI, lossy and lossless JPEG
  XL, three reference decodes (libjpeg-turbo, libwebp, libjxl), and a
  50 ms tone as FLAC and MP3 (with and without a LAME tag). 8 KiB in
  all.
- **Tests** (`crates/assets/src/format/codec_tests.rs`): ids,
  extensions, limits and magic bytes; lossless fixtures equal their
  source; lossy ones within 4 (JPEG), 0 (WebP) and 1 (JPEG XL) of the
  reference decoders; sizes; a picture too big refused before its
  pixels; broken bytes; FLAC's and MP3's headers and lengths, a 48 kHz
  FLAC and one with no sample count; the registry and the image table.
  Golden rows: 12, `<kind>: <id>`, the lossy pictures' pixels. Fuzz:
  `no_bytes_panic_a_codec` in `builtin_tests.rs`, random bytes after
  each magic and each fixture with bytes changed and cut.
- **Size table** in `docs/formats.md`, with the note that `shuyi-app`
  and `frame-png` don't draw these yet nor macroquad play them (2234).
  Measured on a wasm32 release library that decodes every kind through
  `Formats::standard()`, before `wasm-opt` (not installed here): JPEG
  adds 141 KiB, WebP 187, QOI 10, JPEG XL 1,629, FLAC 17 and MP3 54;
  all six 2,024. The docs advise a web game with few JPEG XL files to
  convert them.

Deviations:

- **The ticket, checked against the code** (it was 39 days old): every
  type and file it names still exists under that name (`ImageTable::load_with`,
  `PngInfo`, `OggVorbisTrack`, `rows.format`, `missing_rows`, `cargo
  xtask golden --bless`); 2020 and 2901 are still open, so neither
  *Either order* item has anything to do. No fix to the ticket.
- **JPEG XL's determinism.** `jxl-render` picks AVX2 and FMA (x64) or
  NEON (ARM64) at run time for lossy pictures, with no feature to turn
  it off. The golden row pins the lossy fixture (ADR-0007 §5 asks a
  float decoder for exactly that), and it gave the same hash on x64
  (AVX2 and FMA) and wasm32 (scalar) here; `docs/formats.md` says that
  another lossy picture could round a pixel apart and that lossless is
  exact. The ticket named no fallback for this.
- **No size estimate up front.** The ticket was on
  `tools/unestimated-tickets.txt` and I wrote no estimate before the
  code; it came to 1,494 counted lines, under the limit after the
  pictures' two formats shared one macro arm. It is off the list now.
  Where it would have split cleanly: the four picture formats, then
  FLAC and MP3 (each part with its own fixtures, rows and docs).
- **Extra fixtures:** the three reference decodes, so the lossy
  decoders are checked against libjpeg-turbo, libwebp and libjxl, not
  only against a loose distance from the source.

Follow-ups: none. 2234 draws and plays these formats; its file now
says what 2203 shipped (**From 2203 (done)**).

Game: nothing to adopt, the features are off by default and the game
turns on none (`gh search code --repo Zafnok/visions-of-shuyi
'Formats::standard'`: `crates/content/src/formats/tests.rs` checks
only its dialogue ids; its `Cargo.toml` asks `shuyi-assets` for `png`
alone).

No break: the new types and features are additions, and with every
new feature off `Formats::standard()` and every other format are
unchanged.

Book: none, the formats are reference material in `docs/formats.md`;
the book's chapters load PNG, WAV and OGG, which don't change, and a
game can't draw or play the new formats until 2234.
