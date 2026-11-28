---
id: "3250"
title: "A sprite piece painter in the grid kit: pictures taller than their tile, standing on its foot, drawn by row and never over the piece above"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3250 — A sprite piece painter in the grid kit

## Context

A request from the game.

1. **The game ticket:** game#1411, which moves its sprite skin's unit
   painting onto this once it lands
   (`github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1411-the-sprite-skins-units-on-the-kits-piece-painter.md`).
   It is not a blocker of this one.
2. **The job in one sentence:** stand each piece's picture on its tile,
   with its feet on the tile's foot and its head over the tile above.
   Draw the top row first. Never draw it over a piece standing on that
   tile above. Let it glide between two tiles while it walks, with an
   outline in its side's colour and a fade or a flash over it.
3. **The friction numbers:** these come from the game's third
   engine-friction sweep (game#1397, `docs/engine/friction.md`).
   - **Where:** the game's `crates/ui/src/map_view/sprite/units.rs`, at
     the game's pin `230080d`.
   - **Measured:** 292 production lines (code, without docs or tests), of
     which 162 are generic: `unit_dest`, `outline_dests`, the three
     opacities, `offset_px`, `Place`, `toward`, `place` (the clip mask)
     and `paint_pictures`.
   - **Estimate:** about 140 lines for what the game would keep: the
     lookup of a piece's picture, the side colours, the effect marks and
     the HP bar.
   - **Ratio:** 2.1×, with a signal: the kit's `UnitPainter` is
     implemented, but every generic step under it is the game's own
     code.
   - **Outcome:** 1, tactics kit.
   - The game's first sweep (1153) logged "a sprite outline is four
     sprite items" as an accept row; that stays as it is, and this
     ticket keeps four items.
4. **The generic shape:** `grid::pieces` in `shuyi_ui`, a painter of
   pictures standing on grid tiles. Each piece gives:
   - its tile;
   - its offset while between two tiles (in tiles, `-1..=1` each way);
   - its picture rectangle (the image, the source rect and its size);
   - an outline colour or none;
   - a paint (as is, or dimmed);
   - a fall share and a flash share in `0..=1`.

   The painter does the geometry, the row order, the clip mask and the
   sprite items. It knows nothing of what a piece is: no side, health
   or class.
5. **Not a tactics example:** a farming game's villagers on a tile map
   are taller than a tile and walk between tiles. They need the same
   row order and the same "never over the head of the one standing
   behind". A top-down RPG's party following a leader is the same. The
   **clip** rule is tactics-shaped, because it assumes one piece to a
   tile. So the painter lives in the grid kit (ADR-0063 §5's test:
   "another grid-tactics game would want it unchanged").

The kit has today:
- `UnitPainter` (`crates/ui/src/grid/skin.rs:295`);
- the toy's `Cards` (`crates/ui/src/toy/grid.rs`): one centred 16 × 16
  card per piece, which needs none of this.

Neighbouring tickets:
- engine#2359 (parked) is about frames as data (idle and walk clips),
  not painting. This ticket takes a frame already picked. **Either order
  with 2359:** if 2359 is done, `PieceLook::picture` may come from its
  clip at the scene's clock; if not, nothing to do (the game picks the
  frame).
- engine#1120 (parked) gives the frame generic sort keys for y-sorting
  in real time. It has no tile clip and no grid. Out.

## Nick input

None. The clip rule is the game's look (game tickets 0039 and 0436);
this ticket moves it unchanged and makes it a choice that is on by
default.

## Scope

**In:**
- `PieceLook` (`#[non_exhaustive]`, built by
  `PieceLook::new(tile: Pos, picture: PieceArt)` with setters):
  - `offset: (f32, f32)`;
  - `outline: Option<Rgb>`;
  - `paint: Paint`;
  - `fall: f32` (`0..=1`; anything that isn't a number counts as 0);
  - `flash: Option<(Rgb, f32)>`.

  `PieceArt` is the image, the source rect and its size in pixels. A
  new public type that may grow, per ADR-0039 and ADR-0151.
- `pieces::foot_dest(art_size, tile: PxRect) -> PxRect`: centred
  across, feet on `tile`'s foot, less a `foot_px` the caller gives (the
  strip a game keeps for a bar). This is the game's `unit_dest`.
- `pieces::outline_dests(dest) -> [PxRect; 4]`: one pixel up, down, left
  and right.
- `pieces::offset_px(offset, tile_px) -> (i32, i32)`: whole pixels, a
  tile at most each way, 0 for anything that isn't a number.
- `pieces::toward(tile, offset) -> Pos`: the tile a piece walks to.
- `pieces::opacities(look) -> Opacities { picture, outline, flash }`,
  with these rules, moved from the game:
  - the picture fades as the piece falls;
  - the outline is gone by half way through the fall;
  - the flash is at full strength, then fades out with the picture.
- `pieces::place(grid, pieces: &[PieceLook], index) -> Option<Placed>`:
  - `Placed` is `{ tile: PxRect, row: i32, open: Vec<PxRect>, under:
    bool }`, `#[non_exhaustive]`.
  - It applies the clip mask, moved from the game unchanged: in each
    column the piece's pixels are in, plus one on each side, nothing is
    drawn above its feet's row where another piece stands on the tile
    just above.
  - The tile a walking piece is on and the tile it walks to don't count.
  - The side columns are cut with their neighbour.
- `pieces::paint(grid, pieces, buf, PieceOptions)`:
  - Every outline and picture, by the row of the feet; within a row, the
    pieces between two tiles come last.
  - Each is cut to `place`'s `open` parts.
  - `PieceOptions { clip: bool }` (`#[non_exhaustive]`, default `true`)
    turns the clip mask off for a game that wants heads over heads.
- No API break: additions only. The game's PR that moves the pin follows
  this one (game#1411).

**Out (do not do):**
- picking the frame from a facing and a walking sheet's columns:
  the game's sheets are its own layout (frames as data are engine#2359);
- HP bars, effect marks and their bounce, side colours: the game's;
- y-sorting with sort keys in the frame (engine#1120);
- glyph and one-bit skins: they don't draw pictures taller than a cell.

**Estimated size:** about 700 lines (code 250, tests 350, docs 100).

## Implementation steps

1. Read the game's `crates/ui/src/map_view/sprite/units.rs` at the pin
   (`unit_dest`, `outline_dests`, `offset_px`, `toward`, `place`,
   `paint_pictures`) and its tests in the same file. They are the
   behaviour to keep, case by case.
2. Write `crates/ui/src/grid/pieces.rs` and re-export it from
   `grid.rs`.
3. Port the clip-mask cases as unit tests: a piece under another, a
   piece gliding sideways under two, the walking piece's own two tiles,
   the top row of the view, and a piece half out of the view.
4. Add a `pieces` demo to the toy grid's sprite skin
   (`crates/ui/src/toy/grid.rs`): a 16 × 24 picture on 16 × 16 tiles,
   two pieces in one column.
5. Book: a section in the grid kit chapter.

## Acceptance criteria

- [ ] `pieces::place` gives the game's `place` results on the game's
      test cases (ported as `place_*` unit tests).
- [ ] With `PieceOptions::clip` off, every piece's `open` is the whole
      view.
- [ ] A piece with `fall` at NaN paints as if standing.
- [ ] Pieces paint by their feet's row, and a piece between two tiles
      paints after those standing in its row (a test reads the sprite
      items' order).
- [ ] The toy's sprite skin shows two tall pieces in one column, the
      lower one's head not over the upper one's feet (a frame snapshot).
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] Guides in `docs/guides/` updated, if this adds a direction, a module or a move (ADR-0009 §5).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `place`, `offset_px`, `toward`, `opacities`, the paint order.
- Property: `place`'s `open` parts are inside the view and don't overlap.
- Snapshot / integration: the toy's frame with two tall pieces.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
