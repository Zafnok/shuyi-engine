---
id: "3203"
title: "The grid kit's skins paint the scene's edge arrows, and a range as its outline"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3203 — The grid kit's skins paint the scene's edge arrows, and a range as its outline

## Context

A request from the game.

1. **The game ticket:** game#1213, which deletes the game's own painters
   once this lands
   (`github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1213-map-edge-arrows-and-range-outlines-from-the-kits-skins.md`).
   It is not a blocker of this one. The game orders game#1178 (the Playdate's own edge
   arrows) against game#1213 on its own side.
2. **The job in one sentence:** when a map is bigger than its view, an
   arrow at each side it goes on past says so, fading or steady, without
   a game writing the painter; and a range can show as a line round its
   edge instead of a fill.
3. **The friction numbers:** no friction review. It comes from the
   game/engine boundary audit of 2026-11-03. `MapScene::more`
   (`crates/ui/src/grid/scene.rs:32`, `MapEdges::of_view`) is data that
   no kit skin paints: only `to_text` reads it (`scene.rs:560`). So the
   game paints it from its unit painters' `paint_over` (game 0449),
   which takes about 100 production lines in three files: where the
   arrows go and their fade, a glyph arrow, and a sprite arrow above a
   unit's foot. Its one-bit skin also draws a range as its outline
   (42 lines) through `RangeLook::Fill`'s `skip` and `paint_under`.
4. **The generic shape:** a `GlyphStyle` option for arrows on the
   middle tile of each side in `more` (glyphs, a color name, steady,
   fading or blinking by `clock_ms`). The glyph style's skin paints
   them over the units and under the cursor. `SpriteSkin` gets the
   same from an image with four boxes. A `GlyphStyle` option outlines
   the named ranges: a line `px` thick in the range's color on each
   side of a range tile whose visible neighbor is outside it.
5. **Examples from games that aren't tactics:** a farming game's field
   bigger than the screen shows arrows where it scrolls on. A
   roguelike's dungeon view does the same. A city builder outlines a
   zone instead of tinting it, so the buildings stay readable.

## Nick input

None.

## Scope

**In:**
- `crates/ui/src/grid/scene.rs`: `MapScene::edge_tiles(&self) ->
  impl Iterator<Item = (Dir, Pos)>`: each side in `more`, in `Dir::ALL`
  order, with the middle tile of the view along it (the first of two
  middles). The side is `shuyi_core::Dir`.
- `grid/glyph/style.rs`: `GlyphStyle.edges: Option<EdgeArrows>`, where
  `EdgeArrows { glyphs: [char; 4] (in Dir::ALL order), color: &'static str,
  show: EdgeShow }` and `EdgeShow::{Steady, Fade { period_ms: u16,
  faintest: f32 }, Blink { period_ms: u16 }}` with
  `EdgeShow::strength(self, clock_ms) -> f32`. Fade is 1 at the start of
  a period, falls evenly to `faintest` at half and rises back. Blink is
  1 for the first half and 0 for the second. `PLAIN` sets `None`.
- The glyph skin paints them after `paint_over` and before the cursor.
  The arrow goes on the tile's right cell for `Right` and its left cell
  for the other sides. It replaces the cell's glyph and keeps its
  background. Its foreground is the background mixed toward the color
  by `strength` (`Rgb::lerp`). A strength of 0 paints nothing.
- `grid/sprite.rs`: `SpriteSkin::with_edge_marks(self, EdgeMarks)`.
  `EdgeMarks` is `#[non_exhaustive]` with `EdgeMarks::new(image: ImageId,
  boxes: [ImageRect; 4], inset_px: i32, show: EdgeShow)`. With terrain
  tiles, each arrow is a `Layer::Over` sprite (`inset_px` of the art in
  from the view's edge, centered along it, above the glyph skin's
  `foot_px`), zoomed, clipped to the grid's bounds, at `strength` as its
  opacity. On the glyph ground it paints its `glyph`'s `style.edges`.
- `GlyphStyle.outline: Option<RangeOutline>`, with `RangeOutline {
  ranges: &'static [RangeName], px: i32 }`. It is painted as
  `Layer::Under` overlays after the tiles, in `looks.range(kind)`, and
  never at the view's edge. Each listed range is drawn on its own,
  whatever the `RangeLook`.
- **Breaking:** `GlyphStyle` gains two fields (a game that writes it out
  adds `edges: None, outline: None`). `with_edge_marks` and
  `edge_tiles` are additive. Write the `Breaking:` line and its
  `migration` block. The game's PR that moves the pin (game#1213)
  follows this one.
- The book's `site/content/learn/book/frame-and-skins.md` and
  `skills/shuyi-stage-grid/SKILL.md`: a line on each option.

**Out (do not do):**
- When a game sets `more`, and the camera (the game's).
- The debug view's gizmo for `more` (2315 reads it on its own).
- Animated or eased arrows beyond `EdgeShow`.

## Implementation steps

1. Read `grid/scene.rs` (`MapEdges`, `of_view`), `grid/glyph.rs`
   (`GlyphSkin::paint`), `grid/glyph/style.rs`, `grid/sprite.rs`
   (`paint`, `paint_own_cursor`), and the game's `map_view/edges.rs`,
   `glyph.rs`, `sprite.rs` and `playdate/marks.rs` as the reference.
2. `edge_tiles` and `EdgeShow::strength`, with their tests.
3. The glyph arrows and the outline in `GlyphStyle`. Update every
   `GlyphStyle` literal in the engine and its examples.
4. `EdgeMarks` and `with_edge_marks`.
5. The docs, the `Breaking:` line and the migration block.

Expected size: about 150 production lines and 200 of tests. Past 600
lines, split: this ticket keeps the arrows, and a follow-up written in
the same PR takes `RangeOutline`. Name it in game#1213.

## Acceptance criteria

- [ ] A scene with `more` on all four sides gets an arrow on each
      side's middle tile in the glyph skin, and in the sprite skin both
      with and without terrain tiles. A scene that fits gets none (tests).
- [ ] `Fade` is full at 0 ms, `faintest` at half a period, and stays
      between them (a property test). `Blink` paints nothing in its
      second half.
- [ ] A sprite arrow never covers the `foot_px` pixels at its tile's
      foot (a test at zoom 1 and 2).
- [ ] An outlined range has a line on exactly the sides whose visible
      neighbor is outside it, and none at the view's edge (a test).
- [ ] Every existing snapshot is unchanged (`PLAIN` has neither option).
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `edge_tiles` (odd and even views), `strength`, cells and sprite
  items of the arrows, the outline's rectangles.
- Property: each skin paints only its area with `more` and an outline
  set.
- Snapshot: the glyph skin with arrows and an outlined range.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
