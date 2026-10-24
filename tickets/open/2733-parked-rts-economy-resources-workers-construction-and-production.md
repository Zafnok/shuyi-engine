---
id: "2733"
title: "Parked: RTS economy: resources, workers that gather, construction, production and supply"
type: feature
milestone: "Engine: real-time strategy"
model: opus-5.5
effort: xhigh
status: blocked
blocked_by: ["2711"]
nick_input: none
completed:
---

# 2733 — RTS economy: resources, gathering, construction, production

> **Parked** (Nick, 2026-10-18: "make tickets parked for after visions of shuyi launches for all of these"): after Visions of Shuyi launches, which is later than the MVP in `docs/ROADMAP.md`. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

[ADR-0015](../../docs/adr/0015-real-time-strategy-at-scale-command-sims-columns-flow-fields-and-ai-in-layers.md)
§2 lists `Build`, `Gather`, `Produce` and `Rally` among the commands a
player can give, and §8 has the commander run "the economy: workers to
resources, production to the plan". 2707 validates and queues those
commands but leaves what they do in the world to this ticket. The AI's
economy (2715) and the example (2719) need it.

It builds on:

- 2707: the unit table, `UnitKind` (with `cost`, `build_time`,
  `footprint`, `produces`, `builds` and `gathers`), `RtsCommand`, order
  queues, buckets, spawns and removals;
- 2708: the spatial grid, for the nearest resource and drop-off;
- 2709: costs, where buildings mark their cells and bump
  `map_version`;
- 2711: moving to a point and arriving, which workers use.

The simulation stays pure, in `Fx` and integers, with no clock and no
floats (ADR-0002 §2, ADR-0003 §3). Every step reads the last state and
writes the next (ADR-0015 §3).

## Nick input

None.

## Scope

**In:**
- **Resources** (`shuyi_rts::economy`):
  - `RtsSettings.resources`: a list of resource names with a starting
    amount per player. A game names its own ("ore", "wood"); the engine
    names none.
  - `Stock`: per player, a `BTreeMap<ResourceName, i64>`, so its order
    and its hash are the same everywhere. It is part of `RtsSim`'s state,
    its hash and its text dump.
  - **Supply**: a kind's `supply` (what a unit uses) and
    `provides_supply` (what a building adds), both `#[serde(default)]`
    fields on `UnitKind`. A player's cap is the sum of what their
    finished buildings provide, up to `RtsSettings.max_supply`.
- **Resource nodes**: a kind with a `node: Some(NodeDef { resource,
  amount })` field is a resource node (speed 0, usually a footprint).
  Its remaining amount is a column (`node_left: i32`, 0 for other
  rows). It is owned by a neutral player id (`PlayerId::NEUTRAL`). When
  it reaches 0 it is removed and `ResourceDepleted` is emitted.
- **Gathering** (`Gather`):
  - a worker kind has `gather: Some(GatherDef { resources, carry,
    steps })` and `gathers: true` (2707's field);
  - a building kind has `drop_off: Vec<ResourceName>`;
  - the loop, as a small state machine in a `work` column: go to the
    node (2711's move), work `steps` steps beside it, take `min(carry,
    node_left)`, go to the nearest friendly drop-off for that resource
    (2708's nearest query, ties by row order), deposit into `Stock`,
    return to the same node, repeat;
  - if the node is gone, the worker looks for the nearest node of the
    same resource within `RtsSettings.regather_radius`, and goes idle
    with `WorkerIdle` if there is none;
  - at most `NodeDef.slots` workers work a node at once (default: no
    limit); the others wait beside it in arrival order;
  - **conservation**: a node's loss always equals what workers carry
    plus what was deposited. A property test checks it every step.
- **Construction** (`Build`):
  - `can_place(kind, cell, player, &impl PlaceQuery) -> Result<(),
    PlacementError>`, a pure function: every footprint cell is inside
    the map, not a wall for the kind's movement class, not marked by a
    building, and holds no unit (2708's rectangle query). `PlaceQuery`
    is a small trait with just what it reads (the cells, the walls of a
    movement class, the building marks and `units_in_rect`); `RtsSim`
    implements it over every unit. 2734 implements it for `TeamView`
    and calls the same function for the placement ghost, so the ghost
    and the simulation agree on all the team can see. Over a hidden
    enemy unit the ghost is valid and the simulation still refuses with
    `PlacementBlocked`.
  - When the builder arrives next to the footprint, placement is checked
    again, the cost is taken from `Stock` (or `NotEnough { resource }` is
    emitted and the order dropped), and the building spawns *under
    construction*: a `progress` column counts steps up to `build_time`,
    and its health grows with it from 1 to the kind's health.
  - Progress needs a builder beside it. Several builders add their
    steps (capped by `RtsSettings.max_builders`).
  - The footprint's cells are marked in 2709's costs when it spawns,
    and freed when it is removed, each bumping `map_version`.
  - Cancelling (a `Stop` on the building while under construction)
    removes it and refunds `RtsSettings.cancel_refund` percent, rounded
    down.
  - `Built { unit }` when done; only then does it provide supply or
    produce.
- **Production** (`Produce`):
  - a `ProductionQueue` per building, kept in a `BTreeMap<UnitId,
    ProductionQueue>` (buildings are few, so not a column), with a
    capacity (`RtsSettings.production_queue`, default 5);
  - queuing takes the cost at once (or `NotEnough`), and checks supply
    (or `SupplyBlocked`); a full queue is refused with an event;
  - the head counts steps to its `build_time`, then spawns the unit on
    the nearest free cell around the footprint (in a fixed clockwise
    order from the rally side), and gives it the rally order;
  - cancelling a queued item (`RtsCommand::CancelProduce { building,
    index }`, added to `RtsCommand`) refunds it in full.
- **Rally** (`Rally`): a building's rally point (a place, or a unit to
  follow), stored with its queue. A new unit gets `Move` to a place,
  `Gather` if the point is a resource node and the unit gathers, or
  follows a unit.
- **Events**, added to `RtsEvent`: `Deposited`, `ResourceDepleted`,
  `WorkerIdle`, `ConstructionStarted`, `Built`, `Produced`,
  `NotEnough { resource }`, `SupplyBlocked`, `PlacementBlocked {
  reason }`, `QueueFull`.
- **Buckets**: a gathering worker checks for a lost node or a nearer
  drop-off only in its bucket (ADR-0015 §3); moving and working run
  every step.
- **The fixtures**: three kinds of our own in `crates/rts/fixtures/`: a
  worker, a drop-off building that produces workers, and a resource
  node.
- **Either order with 2713** (vision): the AI and the view read only
  `TeamView` (2713's boundary test), so it must give the team's own
  economy. If 2713 is done, add it: `stock(player)` and
  `supply(player)` (used, cap) for the team's players, each own
  building's production queue and rally point, and in `SeenUnit`
  whether an own worker is idle; 2715 and 2734 read it there. If not,
  nothing to do (2713 adds it when it lands).

**Out (do not do):**
- the player's side: the placement ghost, the production panel, the
  resource bar, selecting idle workers (2734);
- the AI's economy decisions (2715);
- tech trees, upgrades and research (a game's own, through
  `RtsCommand::Game`, or a later ticket);
- walls and gates as special buildings;
- trade, markets, and resources that regrow.

## Implementation steps

1. Add the `UnitKind` fields with `#[serde(default)]` (`node`, `gather`,
   `drop_off`, `supply`, `provides_supply`), their load checks, and the
   new fixture kinds.
2. Add `Stock` and supply to `RtsSim`'s state, hash and dump.
3. Write `can_place` and construction, with the costs marking from
   2709.
4. Write the gathering state machine in the `work` column, with the
   node slots.
5. Write production queues, spawning around the footprint and rally
   orders, and `CancelProduce`.
6. Add the events, then the tests below.

## Acceptance criteria

- [ ] A fixture of one worker, one node and one drop-off: after N steps,
      the stock equals the round trips times `carry`, matching a
      checked-in number.
- [ ] Conservation holds every step in a property test with random
      workers, nodes and drop-offs.
- [ ] `can_place` refuses each case (outside the map, a wall, a
      building, a unit) with its own `PlacementError`, and accepts a
      free footprint.
- [ ] A building under construction provides no supply and produces
      nothing until `Built`.
- [ ] Production takes the cost when queued, refunds it in full on
      cancel, refuses past supply and past the queue's capacity.
- [ ] A produced unit follows its building's rally point.
- [ ] A fixture run replays from its `CommandRecord` to the same hash
      every step on native and web.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `can_place`; supply sums; refunds rounded down; spawn cells in
  their fixed order.
- Property: conservation of resources; no two buildings' footprints
  overlap; a building's cells are walls in 2709's costs exactly while
  it exists.
- Integration: a fixture economy of two players over 3 000 steps, its
  hash checked in; a replay test; a snapshot of the text dump.

## Completion notes
