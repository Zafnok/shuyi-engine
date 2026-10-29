---
id: "2392"
title: "The render card: every frame item and paint in one frame, drawn by the window and by the raster in CI's render job"
type: infra
milestone: After MVP
model: opus-5.5
effort: medium
status: in-progress
blocked_by: ["2018"]
nick_input: none
completed:
---

# 2392 — The render card

## Context

2018 checked the real window in CI ([ADR-0052](../../docs/adr/0052-the-raster-is-the-reference-picture-backends-are-checked-against-it-in-ci-on-software-gl-through-a-capture-mode.md)):
`shuyi-app`'s `capture` feature runs a plan, paints the frames it names
with the raster (`shuyi_ui::frame::raster`, 2332) too, and compares the
window with it; CI's `render` job runs Hello, Grid's plan under Xvfb and
llvmpipe. 2018's diff ran well past 600 lines, so its own size rule moved
the render card here.

Hello, Grid draws cells, sprites in `Paint::Image` and `Paint::Solid` at
opacity 191, and `Item::Rect`s (the Tiles look's path and cursor). It
never draws `Paint::Dimmed` or `Paint::Shaded`, a brightness under 255,
`flip_x`, a clipped sprite, a missing image or glyph, or a backdrop. A
renderer break in one of those passes every gate today. The card is one
frame with every kind of thing the frame draws, so the window is held to
the raster for all of them, with no game behind it.

What 2018 left for this card:

- `shuyi_app::capture` has the public pieces a capture run uses:
  `window_layout`, `read_window` (the live link's one read of the window,
  `readback::window`, turned upright and cut out by the pure `cut_out`),
  `Report` (the files and `capture.txt`) and the exit codes `PASSED`,
  `FAILED` and `CANNOT_RUN`;
- the `render` job's summary step already writes every
  `target/capture/*/capture.txt`, and its artifact uploads
  `target/capture/`;
- ADR-0052 §6: a new frame item or paint goes on this card in the PR that
  adds it.

## Nick input

None.

## Scope

**In:**
- **The render card** (`shuyi_ui::frame::raster::card`, behind `raster`):
  `card::build() -> RenderCard`, with `RenderCard { buffer: GlyphBuffer,
  atlas: FontAtlasDef, atlas_image: Image, table: ImageTable, images:
  BTreeMap<ImageId, Image>, clear: Rgb }` (a plain value, exhaustive:
  ADR-0039 lets a value whose fields are all public stay exhaustive) and
  `RenderCard::painter(&self) -> raster::Painter`. One fixed frame of at
  most 320 × 240 pixels with every kind of thing the frame draws:
  - cells: backgrounds in several colours, see-through cells, glyphs in
    several colours, and a glyph the atlas lacks (the fallback, in
    `MISSING_COLOR`);
  - `Item::Rect` and `Item::Sprite` in both `Layer`s; sprites in every
    `Paint` (`Image`, `Solid`, `Dimmed`, `Shaded`), at opacity 255 and
    128, at brightness 255 and 102, with `flip_x`, cut by their `clip`,
    and one whose image is missing (in `table`, not in `images`);
  - a backdrop at zoom 2, its scene offset by half a scene pixel;
  - its pictures made in code: the test card (`raster::test_card`) and a
    small font atlas of boxes and bars (a `FontAtlasDef` and its
    `Image`). Only an `ImageTable` makes an `ImageId`, so `table` comes
    from `ImageTable::from_files` over a 24-byte PNG header per picture
    (the signature, `IHDR`, the width and the height: all that
    `png_size` reads). No PNG is encoded in `shuyi-ui`;
  - a test with a `match` on `Item`, `Paint` and `Layer` with no `_` arm,
    so a new variant doesn't compile until the card shows it, and a test
    that counts them on the card.
- **`crates/app/examples/render_card.rs`** (`required-features =
  ["capture"]`): opens a window three times the card's size, so
  `console::layout` gives scale 3 (an odd scale). It draws the card with
  `render::Renderer` (made from `atlas`, the atlas picture, `clear` and
  `table`), captures it through `shuyi_app::capture` (`window_layout`,
  `read_window`, `Report`) with `Tolerance::default()` (or the one step 4
  sets), into `SHUYI_CAPTURE_OUT` (`target/capture` unless set), and
  exits with the same codes as a plan's run. The renderer reads files
  through a `fn`, so the card's pictures are encoded to PNG once with
  `image` and kept in a `std::sync::OnceLock`.
- **The `render` job** runs the example under `xvfb-run -a -s "-screen 0
  1280x1024x24"` with `SHUYI_CAPTURE_OUT=target/capture/card`, before
  Hello, Grid's plan.
- **ADR-0052's table** gains the card's measured numbers on llvmpipe.
- **Either order with 2335:** if 2335 is done, its rotated and
  `flip_y` sprites go on the card; if not, nothing to do (2335 adds them
  when it lands).
- **Either order with 2336:** if 2336 is done, a shape of each kind goes
  on the card; if not, nothing to do (2336 adds them).
- **Either order with 2337:** if 2337 is done, its tinted paint and blend
  modes go on the card; if not, nothing to do (2337 adds them).
- **Either order with 2301:** if 2301 is done, a line of wide and narrow
  glyphs goes on the card; if not, nothing to do (2301 adds it).
- **Either order with 2222:** if 2222 is done, an atlas sprite and an
  inserted one go on the card; if not, nothing to do (2222 adds them).
- **Either order with 1120:** if 1120 is done, its test scene's world
  layers go on the card (two parallax layers of y-sorted sprites, and an
  inset viewport); if not, nothing to do (1120 adds them).
- **Either order with 2334:** if 2334 is done, a text run with an outline
  and a shadow goes on the card; if not, nothing to do (2334 adds it).
- **Either order with 2360:** if 2360 is done, a small tile-layer item
  goes on the card; if not, nothing to do (2360 adds it).
- **Either order with 2391:** if 2391 is done, the card's pictures are
  its one picture type; if not, nothing to do (2391 moves them).

**Out (do not do):**
- the web build's card (2019 shows the card in the browser);
- the plans and the capture mode (2018, done);
- 2703's and 2323's scenes (parked or blocked; their own clauses say how
  they join the card).

## Implementation steps

1. Read ADR-0052, `crates/app/src/capture.rs`, `crates/app/src/render.rs`,
   `crates/ui/src/frame/raster.rs` and `raster/test_card.rs`, and
   `shuyi_assets::image::ImageTable::from_files`. Check which of 1120,
   2222, 2301, 2334, 2335, 2336, 2337, 2360 and 2391 are done.
2. Write `raster::card` and its tests (the variant match, the count, the
   card's size at most 320 × 240).
3. Write the example, and run it on your own desktop: it exits 0.
4. Add its step to the `render` job, run the job, and record the numbers
   in ADR-0052. If the window differs from the raster, name the smallest
   tolerance that passes, never above 3104's for 3D (2 % of the pixels
   over 24 in a channel), and never so loose that the break in step 5
   passes.
5. Show the break with a temporary commit that draws `Item::Rect` one
   pixel to the right in `draw_items` (`crates/app/src/render.rs`), then
   revert it.
6. Run the gates.

## Acceptance criteria

- [ ] The render card example exits 0 under Xvfb in the `render` job,
      and its `capture.txt` is in the step summary.
- [ ] The card holds every `Item`, `Paint` and `Layer` variant: the match
      with no `_` arm compiles, and a test counts them.
- [ ] A broken window is caught: a temporary commit drawing `Item::Rect`
      one pixel to the right fails the job and uploads the card's diff;
      it is reverted.
- [ ] ADR-0052's table has the card's numbers on llvmpipe.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the card's variant coverage and count; its size.
- Integration (CI): the `render` job's card step.

## Completion notes

*(Filled in by the session that completes the ticket.)*
