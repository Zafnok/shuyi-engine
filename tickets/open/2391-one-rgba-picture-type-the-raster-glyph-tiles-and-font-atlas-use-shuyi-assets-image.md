---
id: "2391"
title: "One RGBA picture type: the raster, glyph-tiles and font-atlas use shuyi_assets::image::Image"
type: infra
milestone: After MVP
model: sonnet-5
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2391 — One RGBA picture type

## Context

Found by an analysis of the engine for areas that do the same job twice
(2026-10-28). A picture of four bytes a pixel, row by row from the top,
is written four times, each as a width, a height and a `rgba: Vec<u8>`:

- **`shuyi_assets::image::Image`** (`crates/assets/src/image.rs:73`):
  the `Image` asset kind, what the `Png` format decodes (with the `png`
  feature) and what every image format of 2203 and 2234 decodes to. It
  derives `Default`, `Serialize` and `Deserialize`, and has `info()`.
- **`shuyi_ui::frame::raster::Image`** (`crates/ui/src/frame/raster.rs:61`,
  from 2332): the same three fields, with `filled(width, height, Rgb)`
  and `pixel(x, y) -> Option<[u8; 4]>` public, and `texel`, `blend` and
  `fill` private. `Painter` takes its atlas picture and its images in
  this type, `render` and `compare` give it, and `frame_png` re-exports
  it (`crates/cli/src/frame_png.rs:30`). The module docs say decoding
  is the caller's, so `frame_png::decode_image` (`frame_png.rs:323`)
  decodes a PNG itself and copies the result into this type. It can't
  take the `Image` a `Formats` registry decodes, though the two are the
  same data. `shuyi-ui` already depends on `shuyi-assets`
  (`tools/layers.ron`), so nothing stops it using that type.
- **`shuyi_cli::glyph_tiles::Tileset`** (`crates/cli/src/glyph_tiles.rs:26`):
  the same three fields again, with a public `pixel(x, y) -> [u8; 4]`
  (clear outside the picture) and private `index`, `set` and `stamp`.
  `glyph_tiles::render` takes the font atlas's picture as a loose
  `(atlas_width, atlas_rgba): (u32, &[u8])` pair (`glyph_tiles.rs:46`).
- **`shuyi_cli::font_atlas::Atlas`** (`crates/cli/src/font_atlas.rs:188`):
  the glyph map `def`, then `width`, `height` and `rgba` for its
  picture. `font_atlas::one_bit` takes the picture as `(width, rgba)`
  (`font_atlas.rs:239`).

The four pictures are the same thing, and none of them is different on
purpose. The things that look alike but are different stay out (see
**Out**): the one-bit pictures, which pack a bit a pixel, and
macroquad's `Image` in `shuyi-app`.

## Nick input

None.

## Scope

**In:**
- **`shuyi_assets::image::Image`** is the one RGBA picture. It gains
  two methods, moved from the raster's type, and stays a plain struct
  with public fields (not `#[non_exhaustive]`: the engine, the examples
  and the game build it with a struct literal):
  - `pub fn filled(width: u32, height: u32, color: impl Into<[u8; 4]>) -> Self`:
    every pixel `color`. An empty size gives an empty `rgba`.
  - `pub fn pixel(&self, x: u32, y: u32) -> Option<[u8; 4]>`: the pixel,
    or `None` outside the picture.
  No floats in either. Both build without `std` (the `no-std` gate).
- **`shuyi_ui::frame::color`:** `impl From<Rgb> for [u8; 4]`, the colour
  opaque (`[r, g, b, 255]`), so `Image::filled(w, h, Rgb::new(…))`
  still compiles everywhere it does today.
- **The raster** (`crates/ui/src/frame/raster.rs`): its `Image` struct
  goes, and `pub use shuyi_assets::image::Image;` takes its place, so
  `shuyi_ui::frame::raster::Image` and `shuyi_cli::frame_png::Image`
  name the same type as before. `texel`, `blend` and `fill` become
  private functions of the module that take the picture first
  (`texel(image: &Image, x, y)`, `blend(image: &mut Image, …)`,
  `fill(image: &mut Image, …)`), with the same bodies. The module docs
  say its pictures are `shuyi_assets::image::Image`, what the image
  formats decode.
- **`glyph-tiles`** (`crates/cli/src/glyph_tiles.rs`): `Tileset` goes.
  `render` and `render_files` return an `Image`, and `render` takes the
  atlas picture as `atlas_image: &Image` instead of the
  `(u32, &[u8])` pair. `index`, `set` and `stamp` become private
  functions over `&mut Image`; `stamp` reads the pixel under it with
  `image.pixel(x, y).unwrap_or([0; 4])`. Its integer `blend` stays as it
  is.
- **`font-atlas`** (`crates/cli/src/font_atlas.rs`): `Atlas` is
  `{ def: FontAtlasDef, image: Image }`, and `one_bit(def, image: &Image)`.
  Every reader of `atlas.width`, `atlas.height` and `atlas.rgba` reads
  `atlas.image.…`: `font_atlas::run` and its tests, `crates/cli/src/toy.rs`,
  `crates/cli/src/glyph_tiles/tests.rs` and
  `examples/hello-grid/tests/it/files.rs`.
- **Either order with 2565:** if 2565 is done, `frame_png::decode_image`
  and `glyph_tiles::render_files` decode with
  `shuyi_assets::bitmap::convert::png_pixels`: build the `Image` from its
  `(width, height, rgba)` as they do; if not, they decode with
  `codec::decode_png`: build the `Image` from its result the same way,
  and leave the decoder to 2565.
- **Either order with 2234:** if 2234 is done, `frame_png::Painter::new`
  decodes a picture that isn't a PNG with
  `formats.decode::<image::Image>(…)` and copies its fields into the
  raster's `Image`: drop the copy and hand the decoded `Image` to the
  painter as it is; if not, nothing to do (2234 hands it over without a
  copy).
- **Either order with 2350:** if 2350 is done, `build_atlas` takes an
  `&AtlasChars` and its callers pass one: keep that, and change only
  the picture fields of the `Atlas` it returns; if not, nothing to do
  (2350 changes `build_atlas`'s parameters and leaves the picture
  alone).
- **Either order with 2224:** if 2224 is done, `text-font`
  (`crates/cli/src/text_font.rs`) builds a page picture: if it keeps a
  width, a height and an RGBA `Vec<u8>` of its own, make it an `Image`;
  if not, nothing to do (2224 builds its page as an `Image`).
- **Either order with 2392:** if 2392 is done, its render card
  (`raster::card`) builds its pictures as `raster::Image`, which keeps
  its path: check the card still builds, and turn any picture it keeps
  as a loose width, height and `rgba` into an `Image`; if not, nothing
  to do (2392 builds the card's pictures in the one type).
- **For the game:** `shuyi_ui::frame::raster::Image` and
  `shuyi_cli::frame_png::Image` keep their path, their fields and their
  `filled` and `pixel`, and gain `Default`, `Serialize` and
  `Deserialize`: no break is expected there. If CI's `api` job reports
  one, write it as a `Breaking:` line. These break, each a `Breaking:`
  line (ADR-0038 §6): `glyph_tiles::Tileset` is gone (`render` and
  `render_files` return `Image`, and `Image::pixel` gives an `Option`),
  `glyph_tiles::render`'s atlas parameter, `font_atlas::Atlas`'s fields
  and `font_atlas::one_bit`'s parameters. The game's PR that moves the
  pin follows this one; its staleness tests of the atlas and the
  tileset read `atlas.image` and an `Image`, as Hello, Grid's do.

**Out (do not do):**
- The one-bit pictures: `shuyi_ui::frame::onebit::Image<'a>` and
  `shuyi_assets::bitmap::Bitmap<'a>` borrow packed planes a bit a pixel,
  another job.
- macroquad's `Image` in `shuyi-app` (`crates/app/src/render.rs`).
- Merging the two blends: the raster's float `blend` matches the GPU,
  and `glyph_tiles`' integer `blend` stamps glyph coverage onto a clear
  tile. They stay as they are.
- A new decode function, or changing which decoder any caller uses:
  that is 2565's.
- `codec::encode_png`'s signature (`width, height, &rgba`) and the
  sheets of 2514, which write `Image`'s public fields; both keep working.
- 3103's (parked) rasterizer names `shuyi_ui::frame::raster::Image`,
  which keeps its path: nothing to hand it.

## Implementation steps

1. Add `Image::filled` and `Image::pixel` to `crates/assets/src/image.rs`,
   with unit tests.
2. Add `impl From<Rgb> for [u8; 4]` to `crates/ui/src/frame/color.rs`.
3. In `crates/ui/src/frame/raster.rs`, delete the `Image` struct and its
   `impl`, add `pub use shuyi_assets::image::Image;`, and turn `texel`,
   `blend` and `fill` into private functions; update their callers in
   the module (`one_bit_picture`, `Painter`, `blit`, `compare`). Update
   the module docs.
4. In `crates/cli/src/glyph_tiles.rs`, delete `Tileset`, return `Image`,
   take `atlas_image: &Image`, and turn its methods into private
   functions. Update `crates/cli/src/glyph_tiles/tests.rs` (its
   `tiles.pixel(x, y)` checks compare with `Some(…)`, or unwrap to
   clear).
5. In `crates/cli/src/font_atlas.rs`, make `Atlas` `{ def, image }` and
   `one_bit` take `&Image`; update `run`, the tests, `toy.rs` and
   `examples/hello-grid/tests/it/files.rs`.
6. Settle the either-order clauses above with what is done.
7. Update the docs the `document` skill names for a changed public
   item, and write the `Breaking:` lines in the completion notes.
8. Run the gates.

## Acceptance criteria

- [ ] `grep -rn "pub rgba: Vec<u8>" crates` finds only
      `crates/assets/src/image.rs`.
- [ ] `grep -rn "pub struct Tileset\|pub struct Image {" crates` finds
      only `crates/assets/src/image.rs`.
- [ ] `shuyi_ui::frame::raster::Image` and
      `shuyi_cli::frame_png::Image` are `shuyi_assets::image::Image` (a
      raster test builds a `shuyi_assets::image::Image`, hands it to
      `raster::Painter::new` as an image, and reads it back painted).
- [ ] Every existing test of the raster, `frame-png`, `glyph-tiles`,
      `font-atlas` and Hello, Grid's files passes; only the lines that
      name `Tileset`, `atlas.width`, `atlas.height` or `atlas.rgba`
      change.
- [ ] The completion notes have a `Breaking:` line for each break the
      `api` job reports.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`shuyi-assets`): `Image::filled` of 0 × 0 and of 3 × 2;
  `Image::pixel` inside, on the last pixel, and one past each side.
- Unit (`shuyi-ui`): `[u8; 4]::from(Rgb::new(1, 2, 3))` is
  `[1, 2, 3, 255]`; the raster's existing `Image::filled` and `pixel`
  tests pass unchanged.
- Unit (`shuyi-ui`, `crates/ui/src/frame/raster/tests.rs`): a
  `shuyi_assets::image::Image`, named by that path, is a `Painter`
  image, and a sprite of it paints its pixels. Don't turn on
  `shuyi-assets/png` in `shuyi-ui`: only tools use it.
- Unit (`shuyi-cli`): `glyph_tiles` and `font_atlas` tests as today,
  over `Image`.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
