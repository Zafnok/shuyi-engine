---
id: "3220"
title: "Threat arrows as arcs, with a frame on the threatened tile, beside today's straight ones"
type: feature
milestone: MVP
model: opus-5.5 / gpt-6.1-sol
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-17
---

# 3220 — Threat arrows as arcs, and a frame on the threatened tile

## Context

1. **The game ticket:** `game#0908`
   (`github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/0908-polish-pass-sparks-and-threat-arrows.md`).
   Shown three rounds of renders on 2026-11-17, Nick picked arcs as the
   default look of the threat arrows, with arcs plus a frame on the
   threatened tile, and today's straight arrows, as a player's options.
2. **The job:** draw the arrow from each unit that threatens a tile as a
   curve that bows up the screen over what lies between, instead of a
   straight line; optionally frame the threatened tile too.
3. **Friction:** no friction review: a missing feature.
4. **The generic shape:** the tactics kit's threat arrows
   (`crates/ui/src/grid/threat.rs`, game ticket 0448, engine 1115) get a
   shape a grid skin is built with: `Straight` (today's, the default),
   `Arc`, or `ArcAndFrame`. Nothing of the game: the arrows already
   know only a tile and the tiles threatening it.
5. **A game that isn't this one:** any grid tactics game (Fire Emblem:
   Three Houses and Engage draw these lines as arcs, Triangle Strategy
   too); a board game's "who can capture this square" hint.

## Nick input

None (the look was Nick's pick in the game ticket).

## Scope

**In:**
- `threat::Shape` (`Straight`, `Arc`, `ArcAndFrame`, non-exhaustive),
  `threat::threat_overlays_shaped` and `threat::paint_shaped` taking it;
  `threat_overlays` and `paint` stay as they are (the straight shape).
- `GlyphSkin::with_threat_shape`; the glyph skin and the sprite skin (via
  its glyph skin) paint the shape they were built with.
- The arc: a quadratic curve from the arrow's start to the tile's middle,
  its control point above the middle of the two by 0.4 of their distance;
  a 2-pixel line with the 1-pixel dark edge; stopping 4 px short of the
  middle; a filled arrowhead (6 px, about 34° each side), edged dark.
- The frame: a 1-pixel ring in the arrow's color 1 pixel inside the
  threatened tile, edged dark each side.
- Both start where the straight arrow does (`Start`).

**Out (do not do):** animation, see-through color or glow (the game's
round 2: Nick said they "sacrifice readability"); the marks over the
units (unchanged); an option screen (the game's).

## Implementation steps

1. `threat.rs`: `Shape`; `arc` (the curve's pixels joined by `line`, made
   2 px thick); `head` (a filled triangle as one-pixel rows); `frame`;
   `threat_overlays_shaped`, every dark edge first, then the color, then
   the marks as today.
2. `glyph.rs`: a `threat` field on `GlyphSkin`, `with_threat_shape`, used
   in `paint`; `sprite.rs` paints its glyph skin's.
3. Tests; the book's grid chapter if it shows the arrows.

## Acceptance criteria

- [x] An arc starts where the straight arrow starts, bows above both
      ends, and ends 4 px short of the tile's middle under a filled head.
- [x] `ArcAndFrame` adds the ring on the threatened tile, and only there.
- [x] `Straight` (and a skin built without a shape) draws exactly what it
      draws today.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the arc's ends, its bow, its tip, its edge under the color; the
  frame; a skin's shape reaching the overlays (glyph and sprite skins).

## Completion notes

- `threat::Shape` (`Straight`, the default; `Arc`; `ArcAndFrame`),
  `threat_overlays_shaped` and `paint_shaped`; `threat_overlays` and
  `paint` draw the straight shape as before. `GlyphSkin::with_threat_shape`
  and its `threat_shape` field; the sprite skin draws its glyph skin's.
- The arc's numbers are the game's mockup that Nick picked (game ticket
  0908, round 3): bow 0.4 of the distance, 2-pixel line, 4 px short of
  the middle, a 6-pixel filled head at about 34°; all *tunable* constants.
- Checked on the game's Chapter 1 battle at the pin plus this change, both
  looks (the pictures stay local: the map is the game's private content).
- The engine's `main` is past the game's pin; the game moves the pin in
  its own PR.

No break.

Book: none, the book's grid chapter doesn't show the threat arrows; the
new items are documented in their rustdoc.

Game: game#0908 moves the pin and gives its players the three shapes as
an option (arcs by default).
