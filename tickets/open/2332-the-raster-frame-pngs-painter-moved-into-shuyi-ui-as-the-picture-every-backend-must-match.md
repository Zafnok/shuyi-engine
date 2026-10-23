---
id: "2332"
title: "The raster: frame-png's Painter moved into shuyi-ui behind an opt-in feature, as the picture every backend must match"
type: feature
milestone: After MVP
model: sonnet-5
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2332 — The raster: frame-png's Painter moved into shuyi-ui

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8. The engines that check their renderers keep one
reference picture and compare the real renderer with it: Bevy compares
its examples' screenshots in CI
(https://github.com/bevyengine/bevy/blob/main/.github/workflows/example-run.yml),
and Unreal compares screenshots with ground truth within a tolerance.

Shuyi paints the same frame (`GlyphBuffer`) three ways:

- `shuyi-app`'s `Renderer` (`crates/app/src/render.rs`), on the GPU
  through macroquad: what players see;
- `Painter` in `crates/cli/src/frame_png.rs`, in software: what
  `frame-png`, the tool contract, the MCP server (1133) and every game's
  PNG snapshots see (hello-grid's
  `tests/it/snapshots/it__snapshot__stage_*.snap.png`);
- the one-bit renderer (`crates/ui/src/frame/onebit.rs`), already held
  to `onebit::reference` (feature `onebit-reference`).

`frame_png.rs` says `Painter` is "a software copy of `shuyi-app`'s
renderer, pixel for pixel at whole scales". It lives in `shuyi-cli`, so
a game can't use it at run time, and nothing checks the claim.
ADR-0016 §4 already plans to move this code "into the crate that owns
the frame"; parked 2724 would do it for visual novel thumbnails.

This ticket does the move now and makes the result the reference:
`shuyi_ui::frame::raster`. 2018 and 2019 then compare the desktop and
web builds with it. The rendering tickets after it (2301, 2315, 2335 to
2340, and parked 2360 and 2361) change the reference in one place.

Why it matters:

- **The game first.** Visions of Shuyi's tool program paints its frames
  with `Painter::new`, `render` and `one_bit_picture`, and agents judge
  its screens by those pictures.
- **Other 2D games** can make a picture at run time (a save's
  thumbnail, a photo mode) without `shuyi-cli`.
- **The end goal** (2026-10-19): an agent judges the game it made by
  pictures, so the picture must be the one players see. This ticket is
  the first half of that; 2018 checks it.

## Nick input

None.

## Scope

**In:**
- **`shuyi_ui::frame::raster`** (`crates/ui/src/frame/raster.rs`),
  behind a new `raster` feature of `shuyi-ui`, off by default. ADR-0009
  §1 keeps the default engine small, and a Playdate, GBA or DS build
  doesn't need it. Gate it as `onebit::reference` is:
  `#[cfg(any(test, feature = "raster"))]`. Moved from
  `crates/cli/src/frame_png.rs` with their code unchanged:
  - `Image` (`width`, `height`, `rgba`, `pixel`; `filled` becomes
    public; `texel`, `blend` and `fill` stay private);
  - `Painter` and its drawing: `render`, `draw_backdrop`,
    `draw_layers`, `draw_items`, `draw_sprite`, `atlas_cell`; the free
    functions `blit` and `unit`; `MISSING_COLOR`, which becomes a
    public constant so a backend's tests can name the colour;
  - `one_bit_picture`.
- **`no_std` with `alloc`:**
  - `HashMap<ImageId, Image>` becomes a `BTreeMap` (`ImageId` is `Ord`,
    `crates/assets/src/image.rs`). The painter only looks images up, so
    the order changes nothing;
  - `Vec` and `vec!` come from `use crate::prelude::*`, as in
    `frame/draw.rs`;
  - `round` (in `Image::blend`) and `floor` (in `blit`): the same
    `use crate::prelude::*` brings in `crate::float::Float` without
    `std` only. With `std` the standard methods are called, so the
    numbers stay exactly what they were. `float.rs`'s tests hold the
    two equal.
- **No PNG in `shuyi-ui`.** `raster::Painter::new(atlas: FontAtlasDef,
  atlas_image: Image, images: BTreeMap<ImageId, Image>, clear: Rgb) ->
  Painter` takes decoded pictures. Decoding stays in `shuyi-cli`
  (`codec.rs`, the `png` crate).
- **`frame-png` keeps its API** (`crates/cli/src/frame_png.rs`):
  - `pub use shuyi_ui::frame::raster::{Image, one_bit_picture};`
  - `pub struct Painter(raster::Painter)`, with the same
    `new(atlas, atlas_png, images, read, clear) -> Result<Self, String>`
    (it decodes, then builds the raster's) and
    `render(&self, buf, scale) -> Image`, plus
    `raster(&self) -> &raster::Painter`;
  - `Image::decode` becomes `frame_png::decode_image(png) ->
    Result<Image, String>`: an inherent method can't stay on a type of
    another crate;
  - `shuyi-cli` turns on `shuyi-ui`'s `raster` feature in
    `crates/cli/Cargo.toml`.
- **The test card moves too:** `crates/cli/src/test_card.rs` (generated
  pixels, no PNG) becomes `shuyi_ui::frame::raster::test_card`, and
  `crates/cli/src/lib.rs` replaces `pub mod test_card;` with
  `pub use shuyi_ui::frame::raster::test_card;`, so
  `shuyi_cli::test_card` keeps its path.
- **`raster::compare`**, the one comparison every backend's pixels go
  through (2018, 2019; later parked 3104 and 2326):
  - `Tolerance { channel: u8, pixels: u32 }`, deriving `Debug`,
    `Clone`, `Copy`, `PartialEq`, `Eq`, `Serialize` and `Deserialize`
    with `#[serde(default, deny_unknown_fields)]` (2018's plan files
    name it), and an `impl Default` of `channel: 2, pixels: 0`;
  - `compare(window: &Image, reference: &Image, tolerance: Tolerance) ->
    Comparison`;
  - `Comparison { same_size: bool, over: u32, worst: u8, first:
    Option<(u32, u32)>, diff: Image, tolerance: Tolerance }`:
    - `over`: how many pixels differ by more than `tolerance.channel`
      in some colour channel (alpha is ignored: a window has none);
    - `worst`: the largest difference in any colour channel;
    - `first`: the first pixel in `over`, in row order;
    - `diff`: the reference with every colour channel halved and alpha
      255, and each pixel in `over` in `MISSING_COLOR`;
    - with different sizes, `same_size` is false and the other fields
      cover the top-left area both pictures share;
  - `Comparison::passes(&self) -> bool`: `same_size` and
    `over <= tolerance.pixels`.
- **The docs.** The raster's module docs take `frame_png.rs`'s
  paragraph on what the painter draws, and say the raster is the
  reference picture of a frame that `shuyi-app` must match (checked by
  2018). `frame_png.rs`'s module docs point at it. `README.md`'s
  hello-grid table row ("`frame-png`'s `Painter`, as binary snapshots")
  names the raster, and its stale `shuyi_ui::harness` becomes
  `shuyi_platform::harness`, where the Harness is.
- **CI.** The `no-std` job in `.github/workflows/ci.yml` also builds
  `shuyi-ui` with the feature for `thumbv7em-none-eabihf`. The
  `run-gates` skill's `no_std` block gets the same line.
- **For the game.** One API change: `Image::decode` becomes
  `frame_png::decode_image` (the game's tool tests call it once).
  `Painter::new`, `render`, `Image`, `one_bit_picture` and `test_card`
  keep their paths and signatures. Say so in the PR; the game's PR
  that moves the pin follows it.
- **Size.** About 440 lines only change place (the painter, the test
  card and one test), and about 300 are new (the wrapper, `compare` and
  the tests). The PR shows the move with `git diff --color-moved`.
- **Either order:**
  - with 2010 (MVP, a `Breaking:` line for each API break): if it is
    done, this ticket's completion notes carry `Breaking: shuyi-cli:
    frame_png::Image::decode: call frame_png::decode_image(png)
    instead`; if not, the PR body says the same;
  - `Tolerance`, `Comparison` and `raster::Painter` follow ADR-0039
    (`#[non_exhaustive]` and constructors);
  - with 2514 (frames over time from `frame-png`): its sheet and
    encoders stay in `shuyi-cli` either way; if it is done first, they
    take `raster::Image` through `frame_png`'s re-export;
  - with the tickets that add drawing to the `Painter` (2301, 2315,
    2335, 2336, 2337, 2338, 2339 and 2340, and parked 2360 and 2361):
    if one is done first, its drawing code and its painter tests in
    `frame_png.rs` move here with the rest, unchanged. 2301, 2315,
    2335 to 2340, 2360 and 2361 have the matching clause.

**Out (do not do):**
- comparing `shuyi-app` (2018) or the web build (2019) with the raster;
- the ADR that makes the raster the reference of every painter, and a
  render card of every item kind (both 2018);
- PNG encoding or decoding in `shuyi-ui`;
- changing any pixel: the move is byte for byte;
- the one-bit renderer (`onebit.rs` and `onebit::reference` stay);
- the `Painters` registry for module layers (3103, parked) and the 3D
  and screen-effect comparisons (3104 and 2326, parked);
- a thumbnail helper for games (2724, parked);
- frame sheets and animated pictures (2514).

## Implementation steps

1. Read `crates/cli/src/frame_png.rs`, `frame_png/tests.rs`,
   `test_card.rs`, `toy.rs` and `lib.rs`; `crates/ui/src/frame.rs`,
   `frame/onebit.rs` (how `reference` is gated), `lib.rs` (the
   `prelude` module) and `float.rs`.
2. Add the `raster` feature to `crates/ui/Cargo.toml`, commented like
   `onebit-reference`, and `pub mod raster;` in `frame.rs` behind
   `#[cfg(any(test, feature = "raster"))]`.
3. Move `Image`, `Painter`, `blit`, `unit`, `MISSING_COLOR` and
   `one_bit_picture` into `crates/ui/src/frame/raster.rs`, and
   `test_card.rs` into `crates/ui/src/frame/raster/test_card.rs` (its
   own tests go with it). Make them build without `std` (the In list
   above), and keep every line of drawing code as it was.
4. Write the wrapper, `decode_image` and the re-exports in
   `frame_png.rs` and `lib.rs`, and turn on the feature in
   `crates/cli/Cargo.toml`. `toy.rs`, `examples/hello-grid/src/tools.rs`
   and the templates' `crates/game/src/tools.rs` must compile unchanged.
5. Move the one test that calls private raster code
   (`empty_pictures_and_pixels_off_the_picture_are_harmless`, which
   calls `texel` and `blend`) to `crates/ui/src/frame/raster/tests.rs`.
   The other `frame_png` tests stay and pass unchanged through the
   wrapper (the one at line 387 calls `Image::filled`, now public).
6. Write `Tolerance`, `compare` and `Comparison` in `raster.rs`, with
   their tests in `raster/tests.rs`.
7. Add the `no-std` line to `ci.yml` and the `run-gates` skill; update
   the docs.
8. Run the gates. In the PR, show the move with `git diff
   --color-moved`, and name the API change for the game.

## Acceptance criteria

- [ ] `cargo build -p shuyi-ui --no-default-features --features raster
      --target thumbv7em-none-eabihf --locked` passes, in CI's `no-std`
      job and in the `run-gates` skill.
- [ ] `raster` is behind `#[cfg(any(test, feature = "raster"))]`,
      `shuyi-ui`'s `default` features don't list it, and only
      `shuyi-cli`'s `[dependencies]` turn it on (not `shuyi-app`'s, not
      `shuyi-platform`'s).
- [ ] No picture changes: hello-grid's
      `it__snapshot__stage_glyphs.snap.png` and
      `it__snapshot__stage_tiles.snap.png` pass with no
      `cargo insta accept`, and every `frame_png` test passes with its
      expected values unchanged.
- [ ] `shuyi_cli::frame_png::{Painter, Image, one_bit_picture}` and
      `shuyi_cli::test_card` keep their paths, and `Painter::new` and
      `render` their signatures: `toy.rs`, hello-grid's `tools.rs` and
      both templates build unchanged (CI's `templates` job).
- [ ] `compare` of a picture with itself gives `over == 0`,
      `worst == 0`, and passes. One pixel changed by `channel + 1` in
      one channel gives `over == 1`, `worst == channel + 1`, that pixel
      as `first` and in `MISSING_COLOR` in `diff`, fails at
      `pixels: 0` and passes at `pixels: 1`. A change of exactly
      `channel` gives `over == 0`. A change in alpha alone gives
      `over == 0`. Pictures of different sizes give
      `same_size == false` and never pass.
- [ ] `Tolerance` reads `(channel: 4)` from RON with `pixels: 0`, and
      refuses an unknown key.
- [ ] `cargo xtask check-no-std` passes with the new files.
- [ ] The API change for the game (`Image::decode` →
      `frame_png::decode_image`) is in the PR body, and in a
      `Breaking:` line if 2010 is done.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (in `shuyi-ui`, `crates/ui/src/frame/raster/tests.rs`):
  - `compare` on hand-made pictures (the cases above);
  - `Tolerance::default()` and reading it from RON;
  - `Image::filled` and `pixel`;
  - a buffer of 2 × 1 cells with a rectangle item and a test-card
    sprite, rendered at scale 2, with the expected pixels at a few
    points. Its pictures are made in code: the 24 × 16 atlas of
    `frame_png/tests.rs`'s `tiny_atlas` as an `Image` with its
    `FontAtlasDef`, and the test card from `test_card::pixels()`. The
    sprite's `ImageId` comes from an `ImageTable` literal, as
    `frame_png/tests.rs` builds one;
  - the moved `empty_pictures_and_pixels_off_the_picture_are_harmless`
    and the test card's own tests.
- Property: `compare(a, b)` and `compare(b, a)` give the same `over`,
  `worst` and `first`; rendering the same buffer twice gives equal
  pictures.
- Snapshot / integration: the existing `frame_png` tests and
  hello-grid's two PNG snapshots, unchanged.

## Completion notes

*(Filled in by the session that completes the ticket.)*
