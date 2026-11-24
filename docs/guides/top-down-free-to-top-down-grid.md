# Top-down free to top-down grid

- **Move:** `perspective/top-down` to `perspective/top-down-grid`
- **Status:** planned
- **Tickets:** 1125

A town the player walks freely, where meeting a foe starts a battle on a
grid, and the battle's end brings the player back. The game's flow
screen changes the stage: it carries what both stages share (the party,
the place, the foe met) across, and each stage builds its own world from
it. Ticket 1125 builds the example, a walkable town that starts a grid
battle, and writes this guide in full.

## Until then

- Keep each stage's rules apart, each in its own crate or module, and
  let only the flow screen pass plain data between them: never one
  stage's state inside the other's.
- The [top-down free](top-down-free.md) stage isn't built; the
  [top-down grid](top-down-grid.md) stage is.
