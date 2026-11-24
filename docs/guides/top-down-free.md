# Top-down, free

- **Covers:** `perspective/top-down`
- **Status:** planned
- **Tickets:** 1120, 1122, 1125

A world seen from above where things walk anywhere, not tile by tile: a
town to explore, a top-down action game. Things move every fixed step,
slide along walls and bump into each other, and a camera follows the
player. The engine doesn't build it yet: it needs world layers and
cameras (ticket 1120), the movers and collision kit (ticket 1122), and
an example, a walkable town that starts a grid battle (ticket 1125),
which writes this guide in full.

## Until then

- Don't build a free world from the [grid kit](top-down-grid.md): its
  paths step a whole tile in four directions, and its rules wait for the
  player.
- Don't move things by the frame's time (`dt`) or with floats. A world
  that moves on its own steps a fixed number of times a second, as
  `TickSim` does (`shuyi_core::tick`); the `shuyi-stage-field` skill says
  how a game that can't wait keeps its world ready for the kit.
