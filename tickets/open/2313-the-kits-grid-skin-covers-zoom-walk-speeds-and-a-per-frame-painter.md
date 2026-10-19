---
id: "2313"
title: "The kit's grid skin covers zoom, a held walk speed and a painter made each frame, and its glyph skin takes style options"
type: feature
milestone: After MVP
model: opus-5.5
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
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

- [ ] Every existing snapshot of the kit is unchanged.
- [ ] A snapshot of the example skin shows a range as a fill and a
      flashed tile inverted.
- [ ] A test asks a zoomable sprite skin for its levels and gets a skin
      at another zoom with a different `tile_size`.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit and snapshot, as above.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
