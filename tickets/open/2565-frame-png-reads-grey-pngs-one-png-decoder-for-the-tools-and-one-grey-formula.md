---
id: "2565"
title: "frame-png reads grey PNG files: one PNG decoder for the tools, and one grey formula"
type: bug
milestone: After MVP
model: implementation
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
    `frame-png` decodes every sprite's image with it
    (`frame_png::decode_image`, `crates/cli/src/frame_png.rs`), and the
    raster that paints it reads four bytes a pixel
    (`shuyi_ui::frame::raster::Image::texel`, from 2332, indexes
    `rgba[i..i + 4]`), so a grey PNG in a game's
    bundle is painted from the wrong bytes, and `frame-png` panics with
    an index out of bounds once a sprite reads a pixel in the picture's
    lower half. `glyph-tiles` has the same bug, found again on
    2026-10-28: `glyph_tiles::render_files`
    (`crates/cli/src/glyph_tiles.rs`) decodes the font's `atlas.png`
    with `decode_png` and reads each pixel's cover at `* 4 + 3`, so a
    grey atlas (one drawn or re-saved by hand) gives the wrong cover,
    silently: a byte past the end reads as 0, with no panic. The tests
    only decode PNG files the tools wrote themselves, which are RGBA.
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
  calls `png_pixels` instead:
  - `frame_png::decode_image` (`crates/cli/src/frame_png.rs`);
  - `glyph_tiles::render_files` (`crates/cli/src/glyph_tiles.rs`);
  - the unit tests of `font_atlas` (`crates/cli/src/font_atlas.rs`),
    `frame_png` (`frame_png/tests.rs`) and `glyph_tiles`
    (`glyph_tiles/tests.rs`);
  - the cli's integration tests `crates/cli/tests/it/mcp.rs` and
    `crates/cli/tests/it/toy.rs`;
  - Hello, Grid's `examples/hello-grid/tests/it/files.rs`, as
    `shuyi::assets::bitmap::convert::png_pixels` (it reaches
    `shuyi-assets` through `shuyi`, and its `tools` feature's
    `shuyi-cli` turns `png` on).

  No template calls it. `decode_png` goes; `encode_png` stays, and
  `codec.rs`'s docs say it only encodes.
- **Either order with 2234:** if 2234 is done, `decode_image` sends a
  PNG to `codec::decode_png` and any other picture to the registry
  (`Formats::decode::<image::Image>`): replace only the PNG branch's
  call with `png_pixels` and keep the registry branch; if not, nothing
  to do (2234 decodes the other formats beside `png_pixels`).
- **Either order with 2811:** if 2811 is done, `codec.rs` also holds
  `decode_apng`, which reads back only the RGBA8 APNGs `encode_apng`
  writes: it stays, and `codec.rs`'s docs say it encodes PNG and APNG
  files and reads back only its own APNGs, any other PNG being read
  with `png_pixels`; if not, nothing to do (2811 writes those docs).
- **From 3072 (done):** `app_info::icon` (`crates/cli/src/app_info.rs`,
  the icon `check --target` checks and `app-info` downscales) decodes
  with `codec::decode_png`: move it onto `png_pixels` with the other
  callers. **Either order with 3049:** if it is done, its `icons` in
  packaging decodes the icon too (through `app_info::icon`, or
  `decode_png` if it calls one itself): the same move; if not, nothing
  to do (3049 decodes with `png_pixels`).
- **Either order with 2391:** if 2391 is done, `frame_png::decode_image`
  and `glyph_tiles::render_files` build a `shuyi_assets::image::Image`
  from the decoder's `(width, height, rgba)`: swap `decode_png` for
  `png_pixels` there and keep the rest; if not, nothing to do (2391
  builds its `Image` from whichever decoder it finds).
- `shuyi_assets::palette::grey(rgb: [u8; 3]) -> f32`: the Rec. 601
  grey, 0 to 255, documented once. `Light::apply`, the converter and
  `Rgb::dimmed` call it. Their results don't change (same sum, same
  order of operations: check each test still passes unchanged).
- **For the game:** `shuyi_cli::codec::decode_png` is public and goes,
  a `Breaking:` line (ADR-0038 §6). The game doesn't call it (it
  calls `Painter`, `Image` and `decode_image` through `frame_png`, which
  keep their signatures). `palette::grey` is
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
2. Replace `decode_png` with `png_pixels` at every caller Scope lists:
   `frame_png.rs`, `glyph_tiles.rs`, the three unit test modules,
   `tests/it/mcp.rs`, `tests/it/toy.rs` and Hello, Grid's
   `tests/it/files.rs`. Delete `decode_png` and its test, keep
   `encode_png`'s.
3. Add `palette::grey` with its test; call it from `lighting.rs`,
   `bitmap/convert.rs` and `frame/color.rs`.
4. Run the gates.

## Acceptance criteria

- [ ] `frame-png` paints a sprite from a grey PNG and from a grey PNG
      with alpha with the right pixels (a test that writes both kinds
      with the `png` crate, lays them in the test bundle, and compares
      the painted pixels with the source's).
- [ ] `glyph-tiles` draws the same tileset from a font atlas saved as
      grey with alpha as from the same atlas saved as RGBA (a test that
      writes the atlas both ways with the `png` crate and runs
      `render_files` on each).
- [ ] `grep -rn "decode_png\|0\.299" crates examples templates` finds
      only `palette::grey`.
- [ ] Every existing test of lights, dimmed colours, one-bit conversion
      and `frame-png` passes unchanged.
- [ ] The clauses with 2234, 2811 and 2391 are settled.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`shuyi-assets`): `grey` of black, white and pure red, green and
  blue.
- Integration (`shuyi-cli`): the grey-sprite test above, in
  `crates/cli/src/frame_png/tests.rs`.
- Unit (`shuyi-cli`): the grey-atlas test above, in
  `crates/cli/src/glyph_tiles/tests.rs`.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
