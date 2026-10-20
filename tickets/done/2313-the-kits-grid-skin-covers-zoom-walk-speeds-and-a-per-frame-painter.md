---
id: "2313"
title: "The kit's grid skin covers zoom, a held walk speed and a painter made each frame, and its glyph skin takes style options"
type: feature
milestone: After MVP
model: opus-5.5
effort: high
status: done
blocked_by: []
nick_input: none
completed: 2026-10-20
---

# 2313 — The kit's grid skin, enough for a game to use it directly

## Context

- **The game ticket:** game#1167
  ([`1167-the-map-skins-on-the-kits-grid-skin.md`](https://github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1167-the-map-skins-on-the-kits-grid-skin.md))
  waits for this one. Found by the game's first engine-friction sweep,
  game#1153.
- **The job:** "show the pieces on the kit's grid scene in three looks
  (glyphs, sprites, one-bit)".
- **Friction:** the game's map view and its three skins, 1266 lines
  against an estimate of 750 (1.7×), outcome 2 (engine ergonomics, in
  the tactics kit: another grid-tactics game would want it unchanged).
  About 190 lines of glue:
  - **A trait of its own beside `GridSkin`** (about 100 lines). The
    game's `MapSkin` repeats `GridSkin` (`crates/ui/src/grid/skin.rs`)
    method for method, plus zoom (`zoom`, `at_zoom`, a tileset id), a
    third walk speed (while a key is held to hurry the other side's
    turn), and `paint(ctx, scene, area, buf)`: its piece painter needs
    the game's context each frame, but `GridSkin::paint` takes a
    `&dyn UnitPainter<U>` built before the call, and the skins are kept
    as `dyn`. Each of its skins then forwards to the kit's.
  - **The one-bit skin copies the glyph skin** (about 90 lines):
    `GlyphSkin::paint`'s order, `draw_tiles` and `draw_terrain`
    (`crates/ui/src/grid/glyph.rs`), differing only in how a range is
    shown (a fill pattern instead of a tint), how a flash is shown
    (inverted instead of blended), and a side margin.
- **The generic shape:**
  - `GridSkin` gains optional zoom (`zoom_levels()`, `at_zoom(i)`,
    default: none) and `WalkSpeeds` a `held` speed;
  - `GridSkin::paint` takes the painter per call in a way a game can
    build from its frame's context (it already takes `&dyn
    UnitPainter<U>`; check whether a game can build the painter inside
    its own `draw` and pass it, and if so document that as the way and
    add what is missing, such as the zoom);
  - `GlyphSkin` takes a `GlyphStyle` (range: `Tint` or `Fill(Cell)`;
    flash: `Blend` or `Invert`; a side margin in cells), so a one-bit
    look reuses its paint order, tiles and terrain.
- **Outside tactics:** none needed: this is the tactics kit, and
  another grid-tactics game with a zoomable sprite map and a
  monochrome target writes the same code.

## Nick input

None.

## Scope

**In:**
- The zoom, walk speed and style additions, with defaults that keep
  every existing skin and snapshot unchanged.
- The kit's docs: how a game with its own piece painter and several
  looks uses `GridSkin` without a trait of its own.
- One example skin in the engine's test kit drawn with
  `GlyphStyle { range: Fill(..), flash: Invert, .. }`.

**Out (do not do):**
- One-bit output and patterns (3006): `Fill` takes a cell, whatever
  the frame does with it.
- Where the map goes past the view (2304).
- Outline paint for sprites (not filed: one game skin, no budget
  missed).
- Any game's skins.

**For the game:** additive if the defaults hold. The game's PR that
moves its pin follows this one and deletes its `MapSkin` and the
copied paint order (game#1167).

## Implementation steps

1. Read `grid/skin.rs`, `grid/glyph.rs`, `grid/sprite.rs` and their
   tests; read the game's `crates/ui/src/map_view/skin.rs` and
   `map_view/playdate.rs` at its `main` for what it needs.
2. Add zoom and the walk speed.
3. Add `GlyphStyle`; the default is today's look.
4. Docs and the example skin.

## Acceptance criteria

- [x] Every existing snapshot of the kit is unchanged.
- [x] A snapshot of the example skin shows a range as a fill and a
      flashed tile inverted.
- [x] A test asks a zoomable sprite skin for its levels and gets a skin
      at another zoom with a different `tile_size`.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit and snapshot, as above.

## Completion notes

**Done:**

- **Zoom.** `GridSkin` has `zoom_levels()`, `zoom()` and
  `at_zoom(zoom) -> Option<Rc<dyn GridSkin<U>>>`, none by default. A zoom
  is a factor (console pixels per pixel of the art), not an index.
  `SpriteSkin::with_zooms(&[1, 2])` gives a sprite skin its levels; it
  zooms only when its terrain tiles paint the ground.
- **Walk speeds.** `WalkSpeeds::held` was already there (ticket 1115):
  nothing to add.
- **The painter per frame.** `GridSkin::paint` already takes a
  `&dyn UnitPainter<U>` per call, so a screen can build its painter in its
  own `draw`. The painter is told the skin's `Grid`, which gives the
  zoom. `grid::skin`'s module docs say how a game with its own painters
  and several looks uses the kit with no skin trait of its own, with a
  compiled example.
- **`GlyphStyle`** (`grid::glyph::style`), on a new `GlyphSkin::style`
  field: ranges `Tint` or `Fill { glyphs, skip }`, flashes `Blend` or
  `Invert`, the cursor `Pulse` or `Flat`, threat marks `Highlighted` or
  `Plain { edge }`, and `side_cells`. `GlyphStyle::PLAIN` is today's look,
  and every existing snapshot is unchanged.
- **The example** `toy::grid::ONE_BIT`, with a snapshot
  (`one_bit_example_snapshot`) and unit tests for each part of the style.

**Deviations:**

- **`Fill` isn't `Fill(Cell)`.** One cell for every range would make all
  the ranges look the same. The one-bit look shows each range as its own
  pattern (move as dots, attack as lines). So a fill uses the range's own
  color as the background, keeps the glyphs and colors them with a named
  palette color. `skip` names ranges a game marks some other way: the
  game's danger zone is an edge drawn by its painter in `paint_under`.
- **More than range, flash and margin.** The game's
  `map_view/playdate.rs` also draws its cursor differently (flat marks
  that flip pixels, never two in one pixel, clipped to the whole area)
  and its threat marks (arrow-colored, on a named edge). Without these in
  the style, game#1167 couldn't remove its copied paint order, so they
  are `CursorLook::Flat` and `ThreatLook::Plain`.
- **No tileset id on `GridSkin`.** The game's `MapSkin::tileset_id` is
  the game's own data. The docs say to keep it, and whatever else the
  painter needs, next to the skin in a small struct of the game's own.

**For the game (game#1167):** `GlyphSkin` has a new public field, so a
struct literal of it no longer compiles. Use `GlyphSkin::new(colors,
walk, foot_px)`, or add `style: GlyphStyle::PLAIN`. The game's `GLYPH`
in `map_view/glyph.rs` needs that one-line change when game#1167 moves
the pin. Its one-bit skin becomes `GLYPH.styled(GlyphStyle { range:
RangeLook::Fill { glyphs: "map_marked_glyph", skip: &[danger] }, flash:
FlashLook::Invert, cursor: CursorLook::Flat, threat: ThreatLook::Plain {
edge: "map_threat_edge" }, side_cells: 1 })`, with the skin's threat
color `"map_threat"`. The game's PR moving the pin is game#1167 (not
opened yet).

**Follow-ups:** none.
