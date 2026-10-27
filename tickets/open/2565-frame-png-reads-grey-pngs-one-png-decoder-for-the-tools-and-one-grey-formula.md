---
id: "2565"
title: "frame-png reads grey PNG files: one PNG decoder for the tools, and one grey formula"
type: bug
milestone: After MVP
model: sonnet-5
effort: low
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2565 — `frame-png` reads grey PNG files: one PNG decoder, and one grey formula

## Context

Found by an analysis of the engine for areas that do the same job twice
(2026-10-27). Two small copies, one of them with a bug:

- **Two PNG decoders that both promise RGBA8:**
  - `shuyi_assets::bitmap::convert::png_pixels`
    (`crates/assets/src/bitmap/convert.rs`, the `png` feature): turns
    every kind of PNG (palette, grey, grey with alpha, RGB, 16 bits a
    channel) into four bytes a pixel by hand. The image table reads
    pictures with it (`crates/assets/src/image.rs`).
  - `shuyi_cli::codec::decode_png` (`crates/cli/src/codec.rs`): asks the
    `png` crate for `EXPAND | ALPHA | STRIP_16` and trusts the result to
    be RGBA. It isn't for grey pictures: in `png` 0.18 those
    transformations turn `Grayscale` and `GrayscaleAlpha` into
    `GrayscaleAlpha`, two bytes a pixel (`png::Reader::output_color_type`).
    `frame-png` decodes every sprite's image with it (`Image::decode`,
    `crates/cli/src/frame_png.rs`) and reads four bytes a pixel
    (`Image::texel` indexes `rgba[i..i + 4]`), so a grey PNG in a game's
    bundle is painted from the wrong bytes, and `frame-png` panics with
    an index out of bounds once a sprite reads a pixel in the picture's
    lower half. The tests only decode PNG files the tools wrote themselves,
    which are RGBA.
- **The Rec. 601 grey written three times in floats:**
  `0.299 r + 0.587 g + 0.114 b` in `Light::apply`
  (`crates/assets/src/lighting.rs`), in the one-bit converter's
  lightness (`crates/assets/src/bitmap/convert.rs`) and in
  `Rgb::dimmed` (`crates/ui/src/frame/color.rs`). Lights, dimmed units
  and one-bit pictures should agree on what grey a colour is.

## Nick input

None.

## Scope

**In:**
- `shuyi-cli` depends on `shuyi-assets` with the `png` feature
  (`crates/cli/Cargo.toml`), and every caller of `codec::decode_png`
  calls `png_pixels` instead: `frame_png::Image::decode`, and the tests
  of `font_atlas`, `frame_png` and `glyph_tiles`. `decode_png` goes;
  `encode_png` stays, and `codec.rs`'s docs say it only encodes.
- `shuyi_assets::palette::grey(rgb: [u8; 3]) -> f32`: the Rec. 601
  grey, 0 to 255, documented once. `Light::apply`, the converter and
  `Rgb::dimmed` call it. Their results don't change (same sum, same
  order of operations: check each test still passes unchanged).
- **For the game:** `shuyi_cli::codec::decode_png` is public and goes,
  a `Breaking:` line (ADR-0038 §6). The game doesn't call it (it
  calls `Painter` and `Image` through `frame_png`). `palette::grey` is
  additive.

**Out (do not do):**
- `roles::luminance` (`crates/ui/src/frame/onebit/roles.rs`): an
  integer form on purpose, so a one-bit frame is the same bits on every
  platform (ADR-0026).
- `ink_of`'s own brightness rule (`crates/ui/src/frame/onebit.rs`),
  which disagrees with `luminance`: 2389 removes `ink_of` with the inks.
- The private `wrap` in `crates/app/src/lib.rs`, beside
  `shuyi_assets::text::word_wrap`: it wraps error text for the log,
  keeps blank lines and never cuts a long word (a file path stays
  whole), where `word_wrap` drops blank lines and cuts words for text
  boxes. Different jobs; leave both.
- Painting a picture's pixels differently in any other way.

## Implementation steps

1. Turn on `shuyi-assets/png` in `crates/cli/Cargo.toml`.
2. Replace `decode_png` with `png_pixels` in `frame_png.rs` and the
   three test modules; delete `decode_png` and its test, keep
   `encode_png`'s.
3. Add `palette::grey` with its test; call it from `lighting.rs`,
   `bitmap/convert.rs` and `frame/color.rs`.
4. Run the gates.

## Acceptance criteria

- [ ] `frame-png` paints a sprite from a grey PNG and from a grey PNG
      with alpha with the right pixels (a test that writes both kinds
      with the `png` crate, lays them in the test bundle, and compares
      the painted pixels with the source's).
- [ ] `grep -rn "decode_png\|0\.299" crates` finds only `palette::grey`.
- [ ] Every existing test of lights, dimmed colours, one-bit conversion
      and `frame-png` passes unchanged.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`shuyi-assets`): `grey` of black, white and pure red, green and
  blue.
- Integration (`shuyi-cli`): the grey-sprite test above, in
  `crates/cli/src/frame_png/tests.rs`.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
