---
id: "3224"
title: "Ranges that spread out when opened, with a wave of light, one outline and relief"
type: feature
milestone: MVP
model: opus-5.5 / gpt-6.1-sol
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-18
---

# 3224 — Ranges that spread out, shimmer or glint, with one outline and relief

## Context

1. **The game ticket:** `game#0923`
   (`github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/0923-move-and-attack-ranges-new-look.md`).
   Shown three rounds of moving renders on 2026-11-18, Nick picked a
   look for a selected unit's move and attack ranges: they spread out
   from the unit, have one outline around them all, and move in one of
   four ways (a glint band over a raised slab by default; a shimmer; the
   glint alone; the glint with each tile raised), with today's still
   tints as a fifth choice, all a player's option.
2. **The job:** when a grid game opens a unit's ranges, show them
   spreading out from it ring by ring, then keep them gently alive (a
   wave of light), outlined as one area, optionally looking raised.
3. **Friction:** no friction review: a missing feature.
4. **The generic shape:** the tactics kit's range tints (the glyph
   skin's `RangeLook::Tint`, the sprite skin's washes, the one-bit
   fill) get a `RangeShow` a grid skin is built with: whether they
   spread, a `Wave` (`None`, `Shimmer`, `Glint`), one outline, and a
   `Relief` (`Flat`, `Tiles`, `Slab`). The scene says where and when the
   ranges were opened (`MapScene::ranges_shown`); everything else is the
   scene's clock. Nothing of the game: ranges are already named data.
5. **A game that isn't this one:** any grid tactics game (Advance Wars
   and Fire Emblem spread a unit's range out from it when selected;
   Fire Emblem's ranges shimmer); a board game's legal moves, a
   city-builder's zone of effect for a placed building.

## Nick input

None (the look was Nick's pick in the game ticket).

## Scope

**In:**
- `grid::ranges`: `RangeShow` (`TINT`, today's, the default;
  `spreading`, `with_wave`, `outlined`, `with_relief`), `Wave`, `Relief`,
  and what a skin asks of it: `strength` (a tile's tint now), `spread_at`
  and `overlays` (outline, raised edges, slab shadow, clipped).
- `MapScene::ranges_shown: Option<RangesShown>` (`from`, `at_ms`).
- `GlyphSkin::with_range_show` and its `range_show` field;
  `GlyphStyle::draw_tiles_shown` (`draw_tiles` draws `TINT` as before);
  the sprite skin shows its glyph skin's. A one-bit fill takes only the
  spread.
- The numbers are the game's renders that Nick picked: 45 ms a tile,
  a 90 ms fade, 0.6 under a shimmer of ±0.15, 0.55 under a glint of
  +0.2 reaching 2.5 tiles each side, a 2.4 s wave; all *tunable*.

**Out (do not do):** when a game opens ranges or which ones (the
game's); the danger zone's rules; an options screen (the game's).

## Implementation steps

1. `grid/ranges.rs` and its tests.
2. `scene.rs`: `RangesShown`, the field; the test scenes vary it.
3. `glyph/style.rs`: `draw_tiles_shown`; `glyph.rs`: the field and
   `with_range_show`; `sprite/ground.rs` and `sprite.rs`: the tints and
   overlays by it.
4. Tests of each skin.

## Acceptance criteria

- [x] `TINT` (and a skin built without a show) draws exactly what it
      draws today.
- [x] A spreading look shows nothing on a tile before the spread reaches
      it, then fades it in; a fill appears when reached.
- [x] The shimmer and the glint change a tile's tint over time; one
      outline goes round all ranges together, none between two.
- [x] Tiles relief lights the top and left edges and shades the others;
      a slab shadows below and right of the outline, inside the grid.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the spread's timing, the shimmer and the glint, the outline,
  each relief, the clip; a property that any scene's strengths are
  fractions and its overlays lie in the clip.
- Skin: the glyph skin at two moments, a one-bit fill waiting for the
  spread, the sprite skin showing its glyph skin's look.

## Completion notes

- `grid::ranges` (`RangeShow`, `Wave`, `Relief`, and their numbers as
  public *tunable* constants), `MapScene::ranges_shown` with
  `RangesShown`, `GlyphSkin::with_range_show`, `GlyphStyle::draw_tiles_shown`
  and `glyph::style::LIT`. A skin built as before draws as before.
- The outline and relief are overlays under the glyphs: 1 pixel on the
  glyph skin, 2 on the sprite skin, lighter there over the pictures.
- The sprite skin's fallback ground with no terrain tiles stays tinted.
- Checked on the game's Chapter 1 battle at the pin plus this change,
  both looks (the pictures stay local: the map is the game's private
  content).

No break: new items only (`MapScene` is non-exhaustive).

Book: none, the book's grid chapter doesn't show range tints in motion;
the new items are documented in their rustdoc.

Game: game#0923 moves the pin and gives its players the five looks as an
option (Slab by default).
