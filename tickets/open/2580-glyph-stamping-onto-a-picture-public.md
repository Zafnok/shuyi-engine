---
id: "2580"
title: "Glyph stamping onto a picture, public: glyph_tiles::stamp_glyphs with a rule for clear pixels"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: low
status: todo
blocked_by: ["2391"]
nick_input: none
completed:
---

# 2580 — Glyph stamping onto a picture, public

## Context

A request from the game.

1. **The game ticket:** game#1226, which deletes the game's own stamper
   once this lands
   ([`1226-the-test-tileset-stamps-glyphs-with-the-engine.md`](https://github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1226-the-test-tileset-stamps-glyphs-with-the-engine.md)).
   It is not a blocker of this one.
2. **The job in one sentence:** stamp a few font glyphs, blended by their
   coverage, at any place on a picture a tool is drawing.
3. **The friction numbers:** no friction review: found by the
   game/engine boundary audit of 2026-11-03. `glyph-tiles` was made "as
   the game's `test-tileset` does" (`crates/cli/src/glyph_tiles.rs`'s
   module doc), and the game still has its own copy (its
   `crates/xtask/src/test_tileset.rs:112-196`, about 60 production lines:
   `Canvas`, `stamp`, `stamp_at`, the same rounded integer `blend`),
   used by three of its tools. It can't call ours: `Tileset::set` and
   `stamp` are private, and 2391 keeps them private as functions over
   `Image`.
4. **The generic shape:** `pub fn stamp_glyphs(image: &mut Image, at:
   (u32, u32), glyphs: &[char], color: [u8; 3], atlas: &FontAtlasDef,
   atlas_image: &Image, clear: Clear) -> Result<(), String>`, glyphs side
   by side from `at`, each pixel blended by the atlas's alpha;
   `pub fn centred(origin, side, count, atlas) -> (u32, u32)`, where
   `render` puts them in a tile; and `Image::set_pixel`. `Clear` says
   what a clear pixel under a glyph becomes:
   - `Clear::Cover`: the glyph's color at its coverage (what `render`
     does today, so a piece drawn over the ground has soft edges);
   - `Clear::Keep`: it stays clear, only its color channels blend (the
     game's rule; its committed pictures depend on it).
5. **An example from a game that isn't tactics:** a card game's tool
   draws placeholder card faces, a rank glyph at two corners and a big
   suit glyph in the middle, before the art is bought.

## Nick input

None.

## Scope

**In:**
- In `crates/cli/src/glyph_tiles.rs`: `stamp_glyphs`, `centred` and
  `pub enum Clear { Cover, Keep }` (`#[non_exhaustive]`, ADR-0039: a
  rule may be added). It checks every glyph is in the atlas before
  drawing any, and fails with `the font has no '<glyph>'`.
- `render` calls them (`Clear::Cover`), so its pictures are unchanged;
  its error keeps its `tile {index}: ` prefix.
- `shuyi_assets::image::Image::set_pixel(&mut self, x, y, rgba: [u8; 4])`
  beside 2391's `pixel`: outside the picture it does nothing. No floats;
  it builds without `std`.
- Additive, no API break. The game's PR that moves the pin follows this
  one (game#1226).

**Out (do not do):**
- Changing the blend, the centring or `render`'s pictures: every
  existing `glyph_tiles` test and Hello, Grid's tileset stay byte for
  byte.
- Filling a rectangle or drawing shapes; `stamp_glyphs` draws glyphs
  only.
- Moving any of it out of `shuyi-cli`.

## Implementation steps

1. Read `glyph_tiles.rs` and its tests as 2391 left them, and the game's
   `Canvas::stamp_at` (`crates/xtask/src/test_tileset.rs` at its
   `main`).
2. `Image::set_pixel` in `crates/assets/src/image.rs`, with tests.
3. In `glyph_tiles.rs`, lift the glyph loop out of `render` into
   `stamp_glyphs`; the private `stamp` takes the `Clear`; add `centred`
   from `render`'s `left`/`top` sums. Tests first for `Clear::Keep`.
4. **Measure:** about 40 production lines and 60 test lines.

## Acceptance criteria

- [ ] Every `glyph_tiles` test passes unchanged.
- [ ] On a clear pixel, `Clear::Cover` gives `[r, g, b, cover]` and
      `Clear::Keep` gives the blended color with alpha 0; on an opaque
      pixel the two give the same bytes.
- [ ] A glyph the atlas lacks fails and leaves the picture as it was.
- [ ] `Image::set_pixel` outside the picture changes nothing.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`glyph_tiles/tests.rs`): both `Clear` rules on clear and opaque
  pixels; glyphs past the picture's edge clipped; `centred` equals the
  tile placement `render` uses.
- Unit (`shuyi-assets`): `set_pixel` inside, on the last pixel, past
  each side.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
