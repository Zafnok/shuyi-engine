---
id: "2734"
title: "Parked: RTS view: placing buildings, the production panel, the resource bar and idle workers"
type: feature
milestone: "Engine: real-time strategy"
model: sonnet-5
effort: high
status: blocked
blocked_by: ["2733", "2716", "2354", "2355"]
nick_input: none
completed:
---

# 2734 — RTS view: placing buildings, production, resources

> **Parked** (Nick, 2026-10-18: "make tickets parked for after visions of shuyi launches for all of these"): after Visions of Shuyi launches, which is later than the MVP in `docs/ROADMAP.md`. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

[ADR-0015](../../docs/adr/0015-real-time-strategy-at-scale-command-sims-columns-flow-fields-and-ai-in-layers.md)
§9: the view turns what the player does into commands, and never changes
the simulation. 2716 built the scene, selection, control groups and
the minimap, and left the economy's screens out. 2733 built the economy.
This ticket is the player's side of it, behind the crate's `view`
feature.

## Nick input

None.

## Scope

**In:**
- **Placing a building** (`shuyi_rts::view::place`):
  - an action starts placement for a kind the selected workers can
    build (`UnitKind.builds`);
  - a ghost of the footprint follows the pointer (1121), snapped to
    cells. With no pointer, the ghost starts by the selected workers
    and moves a cell at a time by the movement actions;
  - the ghost is marked valid or invalid by 2733's `can_place`, called
    with the local team's `TeamView` (2713) as its `PlaceQuery`. This
    ticket implements 2733's trait for `TeamView`, in `vision` beside
    it (2713's boundary test keeps `view/` off the unit table), with
    hidden enemy units, buildings among them, left out, so the player
    isn't told about them;
  - confirming sends `Build { units, kind, at, queued }`; the "add to
    queue" action keeps placement open for another;
  - cancelling leaves placement and sends nothing.
- **The production panel** (glyph cells, ADR-0003 §5):
  - for the selected building, one entry per kind it `produces`, with
    its cost, greyed when the stock or supply doesn't allow it;
  - the building's queue, with progress as a bar of cells;
  - choosing an entry sends `Produce`; choosing a queued item sends
    `CancelProduce`;
  - entries are reached by actions (move, confirm, cancel) as well as
    the pointer, so the panel works with only a d-pad. Its entries move
    on 2354's `GridCursor` if they form a grid, and the focus passes
    between the panel and the screen's other widgets with 2355's
    `Focus`, over the screen's layout (2329).
- **Setting a rally point**: an action, then a place, a resource node or
  a unit, sends `Rally`.
- **Gathering by the context order** (2716): `Order` on a resource node
  with workers selected sends `Gather { units, resource, queued }` for
  the selected units that gather.
- **The resource bar**: the local player's `Stock` and supply (used /
  cap), as glyph cells. Like the rest of the view, it reads them from
  the local team's `TeamView` (2713's boundary test covers `view/`),
  which also gives the queues, rally points and idle workers.
- **Idle workers**: a count in the resource bar, and an action that
  selects the next idle worker (from `TeamView`) and moves the camera
  to it.
- **Help text** names actions through the player's keymap, never a key
  (`AGENTS.md` rule 8, the `input-actions` skill).
- **The scene** gains the ghost, the panel, the bar and rally markers
  for the selection, each a line in the scene's text form (2716).

**Out (do not do):**
- the economy itself (2733);
- art for the example (2719);
- tech trees and upgrades in the panel (a game's own);
- a build grid overlay for every cell (the ghost is enough here).

## Implementation steps

1. Add placement mode to the view's player state, with the ghost in the
   scene, and `can_place` on `TeamView` through its `PlaceQuery`.
2. Add the production panel and the resource bar as glyph widgets, with
   the panel's moves on `GridCursor` and `Focus`.
3. Add rally and idle-worker actions to the view's `ActionSet` entries.
4. Write the scene text form lines and the tests below.

## Acceptance criteria

- [ ] A Harness test places a building with the pointer and another with
      only movement actions; both send the same `Build` command shape.
- [ ] The ghost is invalid over a wall and over a visible unit, and
      valid over a hidden enemy unit (the simulation then refuses with
      `PlacementBlocked`).
- [ ] Choosing an entry sends `Produce`; with too little stock the entry
      is greyed and choosing it sends nothing.
- [ ] Nothing in `shuyi_rts::view` writes to `RtsSim`; a test replays the
      commands the view sent and reaches the same hash.
- [ ] No help text names a key (the keymap check passes).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: ghost snapping; panel entry states from stock and supply; the
  `Gather` context order.
- Snapshot: the scene's text form with the ghost, the panel and the
  bar; one skin look test of the panel's cells.
- Integration: a Harness session that builds a drop-off, queues two
  workers, sets a rally point on a node, and selects an idle worker.

## Completion notes
