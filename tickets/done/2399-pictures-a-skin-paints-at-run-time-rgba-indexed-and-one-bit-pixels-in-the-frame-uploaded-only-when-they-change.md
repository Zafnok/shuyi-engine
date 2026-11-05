---
id: "2399"
title: "Pictures a skin paints at run time: RGBA, indexed and one-bit pixels in the frame, uploaded only when they change, and dithered on a one-bit target"
type: feature
milestone: "Engine: demos"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: done
blocked_by: []
nick_input: none
completed: 2026-11-04
---

# 2399 — Pictures a skin paints at run time

## Context

Nick, 2026-11-03 ([`docs/decisions.md`](../../docs/decisions.md),
*Bad Apple and DOOM*): "anything that can display graphics must run
Bad Apple and anything that can run video game logic must be able to
run DOOM". Both need the same thing from the frame, and the engine
doesn't have it yet: a picture whose pixels the game makes while it
runs.

What the frame draws today (`crates/ui/src/frame/glyph_buffer.rs`):
`Item::Rect` (a solid rectangle) and `Item::Sprite` (part of an image
from the `ImageTable`, `crates/assets/src/image.rs`). Every image is a
file baked or embedded before the game starts. `shuyi-app` uploads each
one as a texture once (`crates/app/src/render.rs`, `images:
HashMap<ImageId, Texture2D>`). The raster renderer behind `frame-png`
(`crates/ui/src/frame/raster.rs`) and the one-bit renderer
(`crates/ui/src/frame/onebit.rs`, ADR-0026; drawn fast by 3056) read the
same table.

Who needs a picture painted at run time:

- **Bad Apple** (2779): each frame of a one-bit flipbook (2248) is
  decoded into one 400 × 240 one-bit picture;
- **DOOM** (2780): its software renderer writes a 320 × 200 picture of
  palette indices, 35 times a second, and flashes its palette
  (`PLAYPAL`) when the player is hurt or picks something up;
- other games: an emulator's screen, a paint program's canvas, a
  minimap drawn from fog of war, a plasma or fire effect, a
  procedurally drawn planet. The integer software rasterizer of 3103
  (parked) paints into one too.

This is still rule 9 (`AGENTS.md`, ADR-0002 §4): logic builds a plain
view and a **skin** paints the picture from it. Tests of what happened
read the view or the scene; only tests of a look read pixels.

## Nick input

None.

## Scope

**In:**
- **An ADR** (the `write-adr` skill, the next free number): "Pictures
  a skin paints at run time". It decides:
  - where the pixels live between frames, so that a picture changed
    every frame costs one upload and an unchanged one none: a table of
    painted pictures that outlives one frame (the frame's owner keeps
    it, or `Ctx` does), each with a **revision** that moves whenever
    its pixels or palette are borrowed for writing;
  - the three kinds of pixels: RGBA8 (four bytes a pixel), **indexed**
    (a byte a pixel and a palette of up to 256 `Rgb`), and **one-bit**
    (the layout of `onebit::Image`: rows of `stride` bytes, the highest
    bit the leftmost pixel, a set bit white, an optional mask);
  - how one is placed: like a sprite, a `src` in the picture, a `dest`
    and `clip` in console pixels, and a `Layer`. Scaled
    nearest-neighbour only: no smoothing, ever (a painted picture is
    pixel art);
  - how a one-bit target shows an RGBA or indexed picture: each pixel's
    `bitmap::dither::brightness` against the 8 × 8 `onebit::BAYER`
    matrix, in integers, at paint time. A one-bit picture passes
    through untouched;
  - alternatives considered: a new image added to the `ImageTable`
    each frame (its ids are `&'static str` and its textures are made
    once), and a stream of `Item::Rect`s (64,000 rectangles a frame for
    DOOM).
- **The types**, in `shuyi_ui::frame` (a new `painted` module beside
  `raster` and `onebit`):
  - `PaintedId`, the game's name for a picture (a `&'static str`, as
    `ImageId` is);
  - `Painted`, `#[non_exhaustive]` (ADR-0039), made with
    `Painted::rgba(w, h)`, `Painted::indexed(w, h, palette)` and
    `Painted::one_bit(w, h)`, with `size()`, `kind()`, `revision()`,
    `pixels()`, `pixels_mut()`, `palette()` and `palette_mut()` (the
    `_mut` ones move the revision);
  - `Item::Painted(PaintedSprite)`: `Item` isn't `#[non_exhaustive]`,
    so the new variant is a break. Make `Item` `#[non_exhaustive]` in
    the same change, so the next variant isn't one, and write the
    `Breaking:` line (below).
- **The snapshot text** (`crates/ui/src/frame/snapshot.rs`): a painted
  item is one line with its id, size, kind, `dest`, revision and the
  FNV hash of its pixels and palette (`shuyi_assets::fnv`), so a test
  can see that the picture changed without reading it.
- **Every painter draws it:**
  - `shuyi-app` (macroquad): a texture per painted picture, made the
    first frame it appears, updated (`Texture2D::update`) only in a
    frame whose revision differs from the one uploaded, dropped when a
    frame no longer has the picture. An indexed picture is expanded
    through its palette at upload, so a palette flash costs one upload
    too. `FilterMode::Nearest`;
  - the raster renderer (`frame-png`): equal, pixel for pixel, to a
    sprite of the same pixels from the image table;
  - the one-bit renderer (`frame-png --target playdate` and the
    Playdate backend, `crates/playdate/src/display.rs`): dithered as the
    ADR says, through the fast path of 3056 where the picture's `dest`
    is byte-aligned;
  - the headless platform: draws nothing, but the scene and snapshot
    carry it.
- **The docs** (the `document` skill): the book's chapter on the frame
  and skins (`site/content/learn/book/frame-and-skins.md`) gets a short
  part with a plasma effect painted from the step count, and the
  skill pack's skill that teaches skins says when to paint a picture
  and when to use sprites.

**Out (do not do):**
- Decoding anything: flipbooks are 2248, DOOM's renderer is DOOM's.
- Rotation, tint and blend modes on a painted picture (2335 and 2337
  own those for sprites; either may extend to painted pictures later).
- Shaders and GPU effects (2326, 2327, parked).
- A painted picture as a source for the sprite atlas (2222).
- Writing pixels from the rules: the core stays pure. A skin paints.

## Implementation steps

1. Read ADR-0002 §4, ADR-0026, ADR-0039,
   `crates/ui/src/frame/{glyph_buffer,raster,onebit,snapshot}.rs`,
   `crates/app/src/render.rs`, `crates/playdate/src/display.rs` and
   `crates/assets/src/bitmap/dither.rs`.
2. Write the ADR; add its row to `docs/adr/README.md`.
3. Add `shuyi_ui::frame::painted` with `PaintedId`, `Painted` and
   `PaintedSprite`; add `Item::Painted` and make `Item`
   `#[non_exhaustive]`. Fix every `match` on `Item` in the workspace and
   the templates.
4. Add the table that keeps painted pictures between frames, where the
   ADR puts it, and the way a skin reaches it while it paints.
5. Paint it in `raster.rs`, then `onebit.rs` (the dither in a function
   of its own, `dither_row`, unit-tested on its own), then the snapshot
   line.
6. Draw it in `shuyi-app`: the texture cache by `PaintedId`, keyed on
   revision. Count uploads in debug builds, so a test can read them.
7. Draw it on the Playdate (the one-bit renderer, already shared).
8. Add a tiny test game in `crates/ui/tests/` (or beside the harness's
   tests) that paints a 64 × 48 plasma from the step count, for the
   tests below. Hello, Grid doesn't change.
9. The docs, then the gates.

## Acceptance criteria

- [x] The ADR is written, with its row in `docs/adr/README.md`.
- [x] A skin paints an RGBA, an indexed and a one-bit picture, and
      each is drawn by `shuyi-app`, `frame-png`, `frame-png --target
      playdate` and the Playdate backend.
- [x] A picture whose revision didn't move is not uploaded again; a
      palette change is one upload (a test reads the debug counter).
- [x] The raster renderer draws a painted picture equal to a sprite of
      the same pixels.
- [x] An RGBA picture on a one-bit target is dithered in integers
      through `BAYER`, the same on native and wasm32 (a golden row of
      the one-bit renderer's table).
- [x] A painted item's snapshot line names its id, size, kind, place,
      revision and hash.
- [x] `Item` is `#[non_exhaustive]`. The completion notes have a
      `Breaking:` line for `Item` with its `migration` block (ADR-0038,
      `tickets/README.md`).
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `Painted`'s constructors, sizes and revisions; `pixels_mut`
  and `palette_mut` move the revision; `dither_row` against hand-worked
  rows.
- Property: for any size up to 400 × 240 and any pixels, the raster
  renderer's painted picture equals a sprite of the same RGBA; the
  one-bit dither of a picture whose every pixel is 0 or 255 equals the
  threshold of it.
- Snapshot / integration: the plasma test game's scene and snapshot
  over 10 steps; its `frame-png` and `frame-png --target playdate`
  pictures as golden PNG files; the upload count over 10 frames with 3
  changes (3 uploads).

## Completion notes

What was done:

- [ADR-0077](../../docs/adr/0077-pictures-a-skin-paints-at-run-time-kept-in-the-frames-buffer-with-a-revision-rgba-indexed-or-one-bit-and-dithered-at-paint-time.md):
  the pictures live in the frame's own `GlyphBuffer` (its `PaintedTable`),
  which `Game` keeps between frames, so no painter's signature changed.
  A skin reaches it with `buf.pictures_mut()`. `blit` and `set_backdrop`
  copy the pictures their painted items show into the receiving buffer
  (only when they differ), and every painter looks a painted item up in
  the frame's table, also for a backdrop's scene.
- `shuyi_ui::frame::painted`: `PaintedId`, `Painted` (RGBA, indexed with
  256 colours, one-bit with an optional mask), `PaintedKind`,
  `PaintedSprite` (with a `brightness` that `fade` lowers, as a
  sprite's), `PaintedTable` (`get_or_insert_with`, `insert`, `update`:
  puts a picture in only if it looks different, so the revision moves only
  then). `Item::Painted`, and `Item` is `#[non_exhaustive]`.
- The raster draws a painted picture through the sprite's own blit. The
  one-bit renderer and its reference draw a one-bit picture as a one-bit
  sprite and dither an RGBA or indexed one through `onebit::BAYER` at the
  screen's pixels, a frame byte at a time (`onebit::dither_row`, public);
  `render_changed` redraws a painted item whose picture changed. The
  Playdate backend draws through `render_changed_with`, so it needed no
  change. `shuyi-app` keeps a texture per `PaintedId` (`painted.rs`, made
  without a window), updated with `update_from_bytes` only when the
  revision moved, dropped when a frame doesn't draw it.
- The snapshot's line: `layer  painted x,y wxh  id WxH kind rN hash`.
  `shuyi_assets::fnv` is public for its hash.
- The render card has painted pictures of every kind (both layers, dimmed,
  clipped, part of one, missing, one in the backdrop's scene); the
  window matched the raster locally with 0 pixels over.
- The toy's plasma stage (`shuyi_ui::toy::plasma`) paints all three kinds
  from its step count; the toy's tool program has `--plasma` and now
  `frame-png --target playdate`. Its scene and snapshot over 10 steps
  (`crates/ui/tests/it/plasma.rs`), its two pictures as golden PNG files
  (`crates/cli/tests/it/toy.rs`), and the one-bit renderer's golden table
  (`crates/ui/tests/golden/onebit.ron`).
- Docs: the book's chapter 4 has *A picture the skin paints* with a
  plasma snippet and its test; the `shuyi-screen` skill says when to
  paint a picture and when to use sprites; `docs/platform-backends.md`
  says what a backend does with a painted item.

Deviations:

- The upload counter is counted in test builds (`cfg(test)`), not every
  debug build: in a game's debug build nothing reads it, so it would be
  dead code there. The test reads it as the ticket asks.
- `PaintedSprite` has a `brightness` the ticket didn't list, so a fade
  to black darkens a painted picture as it does a sprite.
- `PaintedTable::update` is added beside the listed API: the plain way
  for a skin that makes its picture afresh each frame.
- The Playdate backend wasn't run on a device: it draws through the
  one-bit renderer, whose fast path is held equal to its reference and
  whose redraw of what changed is held equal to a full draw, painted
  pictures included (`onebit::speed_tests`).

Follow-ups: none new. Handed over in 2335, 2337, 2222 and 2783 (a
*From 2399 (done)* note each).

Breaking: shuyi-ui: frame::glyph_buffer::Item: a new variant, `Painted`, and the enum is `#[non_exhaustive]`: a `match` on `Item` outside the engine adds a `_` arm.

```migration
since: 0.2.0
what: Item has a new variant for pictures a skin paints, and more may come.
old: match item { Item::Rect(o) => .., Item::Sprite(s) => .. }
new: match item { Item::Rect(o) => .., Item::Sprite(s) => .., _ => .. }
before: |
  match item {
      Item::Rect(o) => draw_rect(o),
      Item::Sprite(s) => draw_sprite(s),
  }
after: |
  match item {
      Item::Rect(o) => draw_rect(o),
      Item::Sprite(s) => draw_sprite(s),
      Item::Painted(p) => draw_painted(p),
      _ => {}
  }
```
