# Real-time strategy

- **Covers:** `kind/rts`
- **Status:** planned
- **Tickets:** 2707, 2719

Armies of hundreds or thousands of units, given orders and moving at
once, in real time. A module, `shuyi-rts`, will hold the units in
columns, their kinds and their orders (ticket 2707), and an example and
template, a small RTS with a thousand-unit battle (ticket 2719), writes
this guide in full.

## Until then

- Don't make each unit a piece of the [grid kit](top-down-grid.md)'s turn
  rules: an RTS runs on fixed ticks, and its units move by flow fields
  over the map, many at once.
- `flow_field` (`shuyi_core::path`) is already there, and the tick
  simulation, `TickSim`, is what an RTS will run on. Neither is a kit for
  armies yet.
