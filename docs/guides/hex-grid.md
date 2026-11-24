# Hex grid

- **Covers:** `perspective/hex-grid`
- **Status:** planned
- **Tickets:** 2121, 2740

A board of hexagons, where each tile has six neighbours: strategy and
war games. Parked until after Visions of Shuyi launches. Ticket 2121
builds the coordinates, distances, neighbours and paths; ticket 2740 the
drawing, picking, a skin and an example, and writes this guide in full.

## Until then

- Don't fake hexes on the square [grid kit](top-down-grid.md) by
  offsetting rows: its paths, distances and threats count four
  neighbours, and every rule built on them would change.
- A game that needs a board now uses the square grid, and keeps its
  rules apart from its drawing, as the grid kit does, so a later move to
  hexes changes the kit and the skin, not the game.
