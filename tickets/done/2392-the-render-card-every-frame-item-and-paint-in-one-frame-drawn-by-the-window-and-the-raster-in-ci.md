---
id: "2392"
title: "The render card: every frame item and paint in one frame, drawn by the window and by the raster in CI's render job"
type: infra
milestone: After MVP
model: opus-5.5
effort: medium
status: done
blocked_by: ["2018"]
nick_input: none
completed: 2026-10-29
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

- [x] The render card example exits 0 under Xvfb in the `render` job,
      and its `capture.txt` is in the step summary.
- [x] The card holds every `Item`, `Paint` and `Layer` variant: the match
      with no `_` arm compiles, and a test counts them.
- [x] A broken window is caught: a temporary commit drawing `Item::Rect`
      one pixel to the right fails the job and uploads the card's diff;
      it is reverted.
- [x] ADR-0052's table has the card's numbers on llvmpipe.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the card's variant coverage and count; its size.
- Integration (CI): the `render` job's card step.

## Completion notes

**Done (2026-10-29), PR Zafnok/shuyi-engine#172.**

- **The card** (`shuyi_ui::frame::raster::card`, behind `raster`):
  `build() -> RenderCard` and `RenderCard::painter`, as the ticket's
  scope says; also `COLUMNS`, `ROWS`, `TEST_CARD_PATH` and
  `MISSING_PATH`. One frame of 32 × 12 text cells (256 × 192 pixels):
  a band of four backgrounds and five glyph colours, glyphs on the clear
  colour and the glyph the atlas lacks (`Z`, the fallback checkerboard
  in `MISSING_COLOR`), a gallery of fourteen test-card sprites at 1×
  (every `Paint`, both layers, opacity 128 and 255, brightness 102 and
  255, mirrored, the missing picture) and four at 2× (under glyphs, cut
  by a `clip` on odd pixels, mirrored and cut, `Solid` and cut), `Rect`s
  in both layers (one a single pixel wide), see-through cells with no
  backdrop behind them, and a backdrop at zoom 2 behind ten by nine
  cells, its scene (cells, a sprite and a rectangle of its own) offset by
  half a scene pixel, with solid cells over it, one in the clear colour.
  Its atlas is six 8 × 16 cells of boxes and bars.
- **Tests** (`raster/card/tests.rs`): the `match` on `Item`, `Paint` and
  `Layer` with no `_` arm and a count of each on the card (each at least
  twice); its size; its sprites' settings; its cells; its backdrop's
  zoom and offset; the painter's missing colour; the same card every
  time; the atlas as a text picture; and the frame's text snapshot
  (cells, items and the backdrop's scene), added when CI's diff mutants
  found the card's layout and atlas unpinned.
- **The example** (`crates/app/examples/render_card.rs`,
  `required-features = ["capture"]`): a window of 768 × 576 (three times
  the card), the card drawn by `render::Renderer` for three frames, the
  fourth captured through `shuyi_app::capture` with
  `Tolerance::default()`, and the exit codes of a plan's run. Its
  pictures are encoded to PNG once with `image` and kept in a
  `OnceLock`.
- **CI**: the `render` job runs it before Hello, Grid's plan, into
  `target/capture/card`, so the summary step and the artifact already
  take it.
- **Measured**: on llvmpipe 21.1.8 (Mesa 26.0.8) at scale 3, 0 pixels
  over the default tolerance, worst channel difference 1; on the
  author's Windows desktop (scale 2 there, from the display's scaling),
  0 over, worst 1. The default holds; no tolerance of its own.
- **The one-pixel break**: commit 9cfe8f3 drew `Item::Rect` one pixel
  to the right; its `render` run (run 40823038120) failed on the card (891
  pixels over, worst 237) and uploaded its pictures (the `render-captures` artifact, the card's diff among them). On the
  author's desktop: 1 184 over, worst 237. It is reverted.
- **Docs**: the rustdoc of `raster` and `card`, `shuyi_app::capture`'s
  module doc, ADR-0052's table, the backend checklist in
  `docs/platform-backends.md` and the `run-gates` skill.
- **Either order:** none of 1120, 2222, 2301, 2334, 2335, 2336, 2337,
  2360 and 2391 is done, so nothing of theirs is on the card; each one's
  clause now says it adds its part to the card. 2019's ticket names what
  the example leaves it (it builds on native only).

**Deviation: the table is made with `ImageTable::new`, not
`from_files`.** The ticket assumed only `from_files` makes an
`ImageId`; `ImageTable::new` (public, `const`) does too, from the sizes,
as the raster's own tests do, so no PNG header is written.

No break: the card and its constants are additions to `shuyi-ui`, and
`shuyi-app` gains an example.

Book: none, the render card is a backend's check of its own pixels
(`docs/platform-backends.md`), not something a game uses or names.

Nick input: none.
