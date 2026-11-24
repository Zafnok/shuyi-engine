# Roguelike

- **Covers:** `kind/roguelike`
- **Status:** planned
- **Tickets:** 2118, 2119, 2735

Floors made anew for each run from a seed, explored a turn at a time,
with what the player can see limited by walls. A module, `shuyi-procgen`,
will hold noise in integers (ticket 2118) and level generators over a
grid (ticket 2119). An example and template, a small roguelike with
generated floors, field of view and runs from a seed (ticket 2735),
writes this guide in full.

## Until then

- Its moves are the [top-down grid](top-down-grid.md)'s: write the rules
  over its kit, `TurnRules` and the seeded `SimRng`, and a generator
  later only makes the map.
- Don't draw a floor from the clock or an unseeded random source: a run
  is its seed, so a replay and a bug report can play it again.
