---
id: "2304"
title: "The grid scene says where the map goes on past the view, and the grid skins let a game paint over the units and under the cursor"
type: feature
milestone: MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-10-21
---

# 2304 — The grid scene says where the map goes on past the view

## Context

- **The game ticket:** game#0449
  ([`0449-arrows-where-the-map-goes-on-past-the-view.md`](https://github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/0449-arrows-where-the-map-goes-on-past-the-view.md)).
  The game wants small arrows on each edge of its map view where the map
  goes on past it. What they look like is the game's (it asks Nick
  first); which edges have more map, and a place to paint them, is this
  ticket.
- **The job:** show the player that the map goes on past the edge of the
  screen.
- **Friction:** no friction review: a missing feature, found when the
  game's ticket was split between the two repositories.
- **The generic shape:** the grid kit's `MapScene` (`crates/ui/src/grid/
  scene.rs`) holds the view's `origin` and `size` but not whether the map
  goes on past each side. It gains that, as plain data. The kit's skins
  (`grid/glyph.rs`, `grid/sprite.rs`) call the game's `UnitPainter`
  between their layers; they gain one more call, for what lies over the
  units and under the cursor, so a game can paint edge marks (or any
  other mark of its own) there without drawing into the buffer itself.
- **Outside tactics:** a puzzle game or a roguelike whose level is bigger
  than the screen; a board game on a board larger than its view.

The order the skins paint in today: the ground and its tints,
`paint_under`, the path's line, the units (`UnitPainter::paint`), the
path's arrowhead, the cursor, then the threat arrows.

## Nick input

None.

## Scope

**In:**
- `MapScene` gains the sides the map goes on past (e.g. `more:
  MapEdges` with four `bool`s, `left`, `right`, `up`, `down`), all
  `false` from `MapScene::new`.
- A way to set it from the view and the map's size (e.g.
  `MapEdges::of_view(origin, size, map_size)`): `left` when
  `origin.x > 0`, `right` when `origin.x + size.0 < map width`, the
  same for up and down. A map smaller than the view (a negative origin,
  centred) has none.
- `MapScene::to_text` lists it in one line, only when some side is set,
  so existing text snapshots of maps that fit don't change.
- `UnitPainter::paint_over` (default: nothing), called by every kit skin
  after the units and the path's arrowhead and before the cursor.
- **The debug view (2315).** If 2315 is done, add the edges where the
  map goes on past the view to `grid::debug::layout_gizmos` (role
  `Note`). If it isn't, 2315 does this.

**Out (do not do):**
- Any look for the marks: the kit paints none of its own.
- Scrolling, the camera, or when the view moves.
- Any game's scene, skins or pictures.

**For the game:** additive (a new field with a default, a new trait
method with a default), except where a game builds `MapScene` with a
struct literal. The game's PR that moves its pin follows this one
(game#0449).

**Either order with 2013:** if 2013 is done, `MapScene` is
`#[non_exhaustive]`, so no game builds it by a struct literal and this
change is fully additive (say so in the PR), and `MapEdges` takes the
shape 2013's ADR gives a plain value (exhaustive, with public fields).
If 2013 isn't done, keep the exception above; 2013 makes `MapScene`
non-exhaustive afterwards.

## Implementation steps

1. Add `MapEdges` and the field; set it in `MapScene::new`; add the
   helper with its rules.
2. Add it to `to_text` (one line, only when set) and to every test that
   lists the scene's fields (search the kit's tests for the field
   lists, e.g. `every_field_is_listed`).
3. Add `UnitPainter::paint_over` and call it in the glyph skin's and the
   sprite skin's `paint` (with and without terrain pictures) at the
   place above.
4. Document the paint order in each skin's doc comment.

## Acceptance criteria

- [x] The helper gives the right sides at the left edge, in the middle
      and at the right edge of a map wider than the view, the same down
      a taller map, and none for a map smaller than the view (unit
      tests).
- [x] `to_text` shows the sides only when some side is set; existing
      snapshots are unchanged.
- [x] In each skin, a test painter's `paint_over` is drawn over the
      units and under the cursor (a test of a look, reading cells or
      items).
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the helper; `to_text`.
- Snapshot / integration: the paint order in the glyph and sprite skins.

## Completion notes

- `MapScene::more: MapEdges` (none from `MapScene::new`), with
  `MapEdges::of_view(origin, size, map_size)` and `left()`, `right()`,
  `up()`, `down()`, `any()`. `to_text` adds a `more right down` line after
  the first one, only when some side is set.
- `UnitPainter::paint_over` (default: nothing), called by the glyph skin
  and the sprite skin (with and without terrain tiles) after the units and
  the path's arrowhead, before the cursor. Each skin's `paint` doc says the
  order.
- **Deviation:** the ticket suggested four `bool` fields. Clippy's pedantic
  `struct_excessive_bools` denies that (and a gate can't be allowed away),
  so `MapEdges` is two axes, `x` and `y`, each an `Ends { before, after }`,
  with `left()`/`right()`/`up()`/`down()` to read them. Both are plain
  values with public fields, as 2013 asks of plain values.
- 2013 isn't done, so a game that builds `MapScene` by a struct literal
  must add `more`; everything else is additive.
- 2315 isn't done: the debug view's edge gizmos are 2315's.
- Game: the PR that moves the pin is game#0449's (not yet opened).
- Nick input: none.
