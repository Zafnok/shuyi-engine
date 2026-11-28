---
id: "2287"
title: "Pixel edits on an Image: crop, a grid cell, blit and alpha over, flip, key out a colour, recolour, tint by brightness, HSV and halve"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2287 — Pixel edits on an Image

## Context

A request from the game.

1. **The game ticket:** game#1406, the art importers on these edits
   (`github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1406-importers-on-the-engines-image-edits.md`).
   It is not a blocker of this one.
2. **The job in one sentence:** cut, copy, mirror and recolour the
   pixels of a bought picture while importing it, without writing the
   pixel loops again in every importer.
3. **The friction numbers:** the game's third engine-friction sweep
   (game#1397, 2026-11-27). `shuyi_assets::image::Image` has only
   `filled`, `pixel` and `info`, so each of the game's five importers
   rolls its own buffer and loops, about 200 production lines in all:
   - `crates/tools/src/portrait_import.rs`: a `Picture {width, height,
     rgba}` that repeats `Image` (124–149), its `crop`, and
     `split_sheet` cutting grid cells (171–199);
   - `title_art.rs`: `type Picture = (u32, u32, Vec<u8>)` (line 102);
   - `tileset_import.rs`: `Sheet::tile` (a grid cell with a key colour),
     `mirrored`, `paired` (alpha over), `tint` and `brightness`,
     `recolour` by colour pairs, `halved`, and three hand blits;
   - `battler_import.rs`: `key_out` (375–386), `to_hsv`/`to_rgb`
     (388–428) and a hue-range `recolour` (430–451);
   - `map_sprite_import.rs`: grid cells, mirror and re-layout
     (303–343).
   With these edits the game keeps about 30 lines of calls: about 7×.
   The engine has a private blit of its own (`atlas.rs`,
   `Page::blit`) and private `texel`/`blend`/`fill` in
   `crates/ui/src/frame/raster.rs`.
4. **The generic shape:** methods on `Image` (or a small `image::edit`
   module over it), each pure, each returning a new `Image` or editing
   in place:
   - `crop(rect) -> Image`, `cell(col, row, size) -> Image` (a grid
     cell, with an optional margin and spacing);
   - `blit(&src, x, y)` (copy) and `over(&src, x, y)` (alpha over);
   - `flip_x()`, `flip_y()`;
   - `key_out(rgb)` (one colour becomes transparent);
   - `map_colours(&[(from, to)])`, and a `map_pixels(f)` for the rest;
   - `tint_by_brightness(rgb)` (a grey picture tinted, keeping its
     light and dark);
   - `to_hsv`/`from_hsv` for one pixel, and a hue-range recolour on top;
   - `halve()` (every 2×2 block to one pixel, the alpha-weighted mean).
5. **Examples from games that aren't tactics:** a platformer importing
   a bought character sheet (cut its frames, key out the magenta
   background, mirror the left-facing walk); a farming game recolouring
   one crop's sprite into three seasons.

## Nick input

None.

## Scope

**In:**
- The edits above, in the assets crate (no new dependency), with docs
  and doc examples.
- `atlas.rs` uses the public `blit` in place of its private one.
- New methods on an existing public type are additive (ADR-0039).

**Out (do not do):**
- Edits at run time on the screen (rotation, tint and blend modes of
  sprites): 2335 and 2337. These are offline edits of a picture's bytes.
- Resizing by any factor but a half, filtering, or colour quantising.
- The game's use (game#1406).

## Implementation steps

1. **Read** `crates/assets/src/image.rs`, `atlas.rs` (`Page::blit`)
   and `crates/ui/src/frame/raster.rs` (`texel`, `blend`).
2. Write each edit with a test, starting with `crop`/`cell`/`blit`,
   which the others build on.
3. Move `Page::blit` onto `Image::blit`.
4. Docs: one module doc listing the edits, as a recipe ("cut a sheet's
   frames, key out its background").

## Acceptance criteria

- [ ] Each edit has a unit test on a small picture built in the test,
      checked pixel for pixel.
- [ ] `crop` or `cell` outside the picture is an error (or `None`), not
      a panic; `blit`/`over` past the edge clip.
- [ ] `halve` of a 2×2 block with one transparent pixel weights the
      mean by alpha.
- [ ] The atlas tests pass unchanged after `Page::blit` moves.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] Guides in `docs/guides/` updated, if this adds a direction, a module or a move (ADR-0009 §5).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each edit.
- Property: `flip_x` twice is the picture; `crop` of the whole picture
  is the picture; `key_out` of a colour not in it changes nothing.
- Snapshot / integration: none.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
