# Side view

- **Covers:** `perspective/side-view`
- **Status:** planned
- **Tickets:** 1122, 1127, 1128

The world seen from the side, with gravity: platformers, and games that
jump and climb. A body runs, jumps and lands every fixed step, against
tiles and slopes. The engine doesn't build it yet: the movers and
collision kit (ticket 1122) comes first, then a platformer controller and
a platformer example (ticket 1127), which writes this guide in full.
Rigid-body physics, `shuyi-physics` (ticket 1128), is a module for a game
that wants bodies to tumble.

## Until then

- Don't build a platformer on the [grid kit](top-down-grid.md): a
  piece there steps from tile to tile and waits for the player.
- Don't move bodies by the frame's time (`dt`) or with floats. A
  real-time world steps a fixed number of times a second, as `TickSim`
  does (`shuyi_core::tick`), and the `shuyi-stage-field` skill says how
  to keep a world ready for the kit.
