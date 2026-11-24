# Isometric

- **Covers:** `perspective/isometric`
- **Status:** planned
- **Tickets:** 1126

A square grid seen at an angle, its tiles drawn as diamonds, for tactics
games and towns with height. The rules stay those of a square grid: only
the drawing, and which tile a click lands on, change. The engine doesn't
draw it yet: ticket 1126 builds the isometric projection and an
isometric tactics example, and writes this guide in full.

## Until then

- Write the rules over the [top-down grid](top-down-grid.md)'s kit, which
  an isometric stage will keep. Only the screen and the skin change.
- Don't work out a tile's place on the screen in the rules, or with
  floats. Where a tile is drawn is the skin's, and the projection will
  be a value the screen holds.
