# Action RPG

- **Covers:** `kind/action-rpg`
- **Status:** planned
- **Tickets:** 2756, 2764

A hero who fights in real time, with skills, packs of foes and loot, in
a town and areas made anew. A module, `shuyi-action`, will hold it
(ticket 2756), and an example and template, a small isometric action RPG
with a town, two generated areas, waypoints, skills, packs and loot
(ticket 2764), writes this guide in full.

## Until then

- It runs in real time, on fixed ticks: the [grid kit](top-down-grid.md)'s
  turn rules aren't for it.
- It waits for a real-time world, the movers and collision kit of
  [top-down free](top-down-free.md), and for the
  [isometric](isometric.md) projection. The `shuyi-stage-field` skill
  says how a game that can't wait keeps its world ready for them.
