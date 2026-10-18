# ADR-0015: Real-time strategy at scale: command sims, units in columns, flow fields, and AI in layers

- **Status:** Accepted
- **Date:** 2026-10-18
- **Related tickets:** 2705–2720, 2733 and 2734 (all parked until
  after Visions of Shuyi launches), 1119 (tick simulation), 1120 (world
  layers), 1121 (pointer input), 1124 (maps), 1134 (skills), 1140
  (lockstep), 2003 (the catalog), 2004 (guides), 2201 (formats), 2501
  (templates), 3008 (budgets), 3009 (bots think in slices), 3013 (the
  Steam Deck), 3017 (capabilities and needs)
- **Extends:** ADR-0003 §2 (two kinds of simulation) with a third shape,
  and answers ADR-0003 §4's open question for this module; ADR-0006
  (lockstep over a tick game); ADR-0008 §6 (budgets, thinking in work
  units); ADR-0009 (modules)

## Context

Nick, 2026-10-18 (`docs/decisions.md`): "I wanna be able to support high
unit count RTS games", and "I would imagine thousand+ unit count RTS
games need advanced optimized AI routines provided". Parked until after
Visions of Shuyi launches.

What a real-time strategy game with a thousand units or more needs that
the plans don't have:

- **Orders, not buttons.** A player selects 300 units and orders them to
  a place. The simulation's input is that order, not the keys pressed.
  ADR-0003 §2's `TickInput` carries actions, axes and the pointer per
  player. Selection is the player's own view and must not be in the
  simulation, or every peer in a networked match would need every other
  player's mouse.
- **Many units, cheaply.** ADR-0003 §4 keeps tick stages as plain
  structs in an `Arena<T>`, sized for "tens to a few hundred things",
  and says that if an example needs thousands, a spike compares `hecs`
  and `bevy_ecs` against the engine's rules (determinism, WASM size,
  compile time, a readable dump).
- **Pathfinding for armies.** A* per unit costs O(units × map). With a
  thousand units ordered at once, that is the whole frame and more.
- **Units that don't stack up.** Units moving together must spread out,
  pass each other, and not overlap at the goal.
- **Fighting, vision, and fog of war** for every unit, every step.
- **Computer opponents.** An economy, production, scouting, attacks and
  retreats, plus per-unit reactions (pick a target, focus fire, back off
  when hurt), for every unit of every computer player.
- **A screen that holds up.** Drawing, selecting and the minimap with a
  thousand units in play.

What other games and engines do (public talks and papers, read
2026-10-18):

- **Lockstep with timed commands** is how RTS multiplayer has worked
  since the 1990s ("1500 Archers on a 28.8", Age of Empires, GDC 2001).
  Each peer runs the same deterministic simulation and sends only
  commands, scheduled a few steps ahead.
- **Flow fields** (Elijah Emerson, *Crowd Pathfinding and Steering Using
  Flow Field Tiles*, Game AI Pro, 2013, used in Supreme Commander 2;
  also Planetary Annihilation) cost O(map) per goal, shared by every
  unit going there, and are built only over the sectors a high-level
  search picks.
- **Hierarchical pathfinding** (HPA*, Botea, Müller and Schaeffer,
  2004): sectors with portals, a search over the portal graph first.
- **Steering** (Craig Reynolds, *Steering Behaviors For Autonomous
  Characters*, 1999): separation, cohesion, alignment and arrival as
  sums of small forces. Reciprocal velocity obstacles (RVO2, ORCA) give
  smoother crowds, but solve a linear program per unit in floats.
- **Influence maps** and **utility AI** (Dave Mark, Kevin Dill, Game AI
  Pro) for strategic decisions; build orders as data in most RTS AIs.
- **Data laid out in columns** (struct of arrays) for anything touched
  per unit per step, so a pass over one field reads contiguous memory.

The engine's rules still hold: the simulation is pure and deterministic,
integer fixed-point (`Fx`, ADR-0003 §3), with no clock and no I/O; a run
replays from its record; tests read state and scenes, not pixels; bots
think in work units, never time (ADR-0008 §6).

## Decision

### 1. A module, `shuyi-rts`, under a new concern

- **The crate** `crates/rts` (`shuyi-rts`), with every module duty of
  ADR-0009 §4: metadata, a README in the fixed order, the `rts` feature
  of the `shuyi` crate, a catalog row, an example, a skill
  (`shuyi-rts`), and its `needs` (ADR-0012 §1).
- **A new concern, *Kind of game*** (`kind`), `pick: Many`, question
  "What kind of game is it?". It lists the kits built for one kind of
  game, which are not answers to the other concerns:
  - `tactics`: the grid-tactics kit of the default engine (ready once
    the carve is done);
  - `rts`: this module;
  - `visual-novel` and `mystery`: ADR-0016.

  Unlike the concerns of ADR-0009 §2, its answers are **not
  alternatives**: a strategy game can tell its story as a visual novel.
  The concern is marked `alternatives: false` in `docs/modules.ron`, and
  2003's check doesn't ask its modules to name each other. 2003 adds
  the concern with its parked answers; if 2003 is already done, the
  first of these tickets to land adds it.
- **It serves** the top-down free and isometric perspectives (ADR-0009
  §3): the simulation is in world `Fx` coordinates over a tile grid, and
  only drawing goes through the projection.
- **Its parts**, in submodules:

  | Part | Path | What |
  | ---- | ---- | ---- |
  | Units | `shuyi_rts::units` | the unit table (§3), unit kinds as data |
  | Orders | `shuyi_rts::orders` | commands, order queues per unit |
  | Space | `shuyi_rts::space` | the spatial grid (§4) |
  | Paths | `shuyi_rts::paths` | costs, sectors, portals, flow fields (§5) |
  | Steering | `shuyi_rts::steer` | moving together without overlap (§6) |
  | Combat | `shuyi_rts::combat` | targets, attacks, projectiles (§7) |
  | Economy | `shuyi_rts::economy` | resources, gathering, construction, production, supply (§12) |
  | Reactions | `shuyi_rts::react` | what every unit does by itself (§8, layer 1) |
  | Vision | `shuyi_rts::vision` | fog of war per team (§7) |
  | AI | `shuyi_rts::ai` | units, squads, commanders (§8) |
  | View | `shuyi_rts::view` (feature `view`) | the scene, selection, the minimap (§9) |

  The simulation parts depend on `shuyi-core` only, so a headless server
  or a balance-testing tool uses them without `shuyi-ui`. The module's
  formats (`*.unit.ron` and the others) are registered behind an
  `assets` feature, on by default, which adds `shuyi-assets`.

### 2. A third shape of simulation: ticks driven by timed commands

ADR-0003 §2 has turn rules (commands) and tick simulations (inputs per
step). An RTS is ticks driven by commands. `shuyi_core::tick` gains:

```rust
pub struct Timed<C> { pub step: u64, pub player: PlayerId, pub command: C }

pub trait CommandSim {
    type Command: Clone + Eq + Serialize + DeserializeOwned;
    type Event;
    /// Commands due at this step, in (player, order sent) order.
    fn step(&mut self, due: &[Timed<Self::Command>]) -> Vec<Self::Event>;
    fn hash(&self) -> u64;
}
```

- **A run is its seed plus its timed commands** (`CommandRecord`).
  Replays, `shuyi replay`, the Harness and bots work on it, as they do on
  `TickRecord`.
- **A command is what a player could order**: move, attack, attack-move,
  patrol, hold, stop, build, gather, produce, set a rally point, and a
  game's own. It names its units by id. Selection, control groups and
  the camera are the player's view, outside the simulation (§9).
- **Every player is equal.** A computer player issues the same commands
  a person does, through the same queue. Nothing in the AI writes to the
  simulation directly. So an AI can be swapped for a person, and a
  replay never needs the AI to run again.
- **Steps stay at 60 a second** (ADR-0003 §2), so drawing needs no
  interpolation. Work that needn't run every step is spread over steps
  instead (§3).
- **Lockstep over commands** (2720): peers exchange the commands for a
  *command turn* of N steps (default 4), scheduled a fixed delay ahead
  (default 2 turns, about 130 ms), and compare hashes each turn. This is
  ADR-0006's lockstep for a tick game: 2720 adds `CommandLockstep<S:
  CommandSim>` beside 1140's lockstep over `TurnRules`, and reuses
  1140's transport and session unchanged. The hidden-information rule stays: lockstep is for
  friends (ADR-0006 §4).

### 3. Units live in columns, not in an arena of structs

ADR-0003 §4 stays the rule for small tick stages. For this module, the
answer to its open question is a table of columns:

- **`UnitTable`**: one `Vec` per field (position `FxVec2`, velocity,
  facing, kind, owner, health, the current order's index, flags), all
  the same length, dense. A pass over positions reads positions only.
- **Ids are stable**: a generational `UnitId` maps to a row through an
  index; removing a unit swaps the last row into its place and updates
  the index. Iteration is in row order, which is deterministic because
  every insert and removal is.
- **Kinds are data**: `*.unit.ron` (speed, radius, health, armour,
  weapons, sight, cost, build time, movement class). A unit row holds
  only its kind's index and what changes per unit.
- **Buildings and resource nodes are units** whose kind has a
  `footprint` (cells) and speed 0, so one table, one spatial grid and
  one set of commands cover them.
- **Serde and a text dump** as `Arena<T>` has, so a test or an agent can
  read the whole state.
- **Staggered work.** Work that can wait (choosing a target, an AI
  decision, re-checking a path) runs for one *bucket* of units each
  step: a unit runs on steps where `step % k == slot % k`, where `slot`
  is its `UnitId`'s index, which a removal never changes. The bucket
  count `k` is a setting per kind of work. The order is fixed, so the
  result is deterministic.
- **The step reads the last state and writes the next.** Each pass reads
  columns from the step before and writes its own unit's row in the
  next, so the order units are visited in doesn't change the result.
  That keeps the step parallelisable later (§10) without changing it.
  Where one unit acts on another (a hit, a push), the acting unit
  writes into a buffer, and each unit gathers its own share in a later
  pass: hits are summed by their target, and each unit computes its own
  part of a push.
- **2705 measures first.** Before the table is written, a research
  ticket times an `Arena` of structs, the `UnitTable`, and `hecs` (and
  `bevy_ecs` if it builds for the web without `wasm-bindgen`) on 1 000,
  5 000 and 10 000 units, against the rules ADR-0003 §4 names. If an ECS
  wins on every rule, a new ADR supersedes this section before 2707
  starts.

### 4. Space: one uniform grid, rebuilt each step

- `SpatialGrid`: cells of a fixed size in `Fx` (a setting, default the
  largest unit's diameter), rebuilt every step by a counting sort of the
  unit rows. That is O(units) with no allocation after the first step,
  and the order inside a cell is row order.
- Queries: units within a radius, units in a rectangle (box selection,
  area damage), the nearest unit matching a filter. Results come in a
  fixed order.
- Quadtrees and bounding-volume trees lose: rebuilt or rebalanced each
  step for moving units, they cost more and are harder to keep
  deterministic.

### 5. Paths: costs, sectors and portals, flow fields shared by everyone

- **Costs.** A grid of terrain classes, from the map (Tiled or LDtk,
  1124, or code). Each *movement class* (ground, hover, air, a game's
  own) turns a class into a cost or a wall. Buildings mark their cells.
- **Sectors and portals** (HPA*). The grid is cut into sectors (default
  16 × 16 cells). Each run of open cells along a sector border is a
  portal. A sector's portals are joined by their costs inside it. A
  change of costs (a building placed or destroyed) rebuilds only the
  sectors it touches.
- **A route** is an A* search over the portal graph, from the unit's
  sector to the goal's.
- **Flow fields** are built only over the sectors on the route: an
  integration field (cost to the goal, Dijkstra over cells) and a
  direction per cell (one of 8, toward the cheapest neighbour). A unit
  reads its cell's direction; no unit holds a path.
- **Shared and cached.** Every unit of one movement class ordered to
  one goal cell shares one field. Fields are cached by (goal, class,
  map version), least recently used first out, with a size limit.
- **Budgeted.** Requests wait in a queue and are served in work units
  (cells expanded), a fixed number per step, never by time (ADR-0008
  §6). In a real-time simulation that number is a setting of the
  simulation, the same on every device, never a target profile's
  `think_units_per_frame` (3009): otherwise a field would be ready at a
  different step on a slower device, and replays and lockstep would
  part. A unit whose field isn't ready yet heads straight for the goal
  until it is. The queue's order is the order requests were made.
- A single unit far from others, or an air unit, may use a direct line
  or a portal path alone; the README says when.

### 6. Steering: move together, don't overlap, arrive

Each step, a unit's wanted velocity is a weighted sum, in `Fx`:

- the flow field's direction (or the straight line, or a formation
  slot);
- **separation** from neighbours in the spatial grid, stronger the
  closer they are;
- **arrival**: slowing inside a radius of the goal, and stopping when the
  group's area around the goal is full, so units don't fight over one
  point.

Then:

- **Pushing.** Overlaps left after moving are resolved by pushing, with
  priorities: an idle unit yields to a moving one, a unit of the same
  owner yields before an enemy. Units never end a step overlapping
  more than a set tolerance, a property test checks it.
- **Groups.** Units ordered together are a group: it moves at its
  slowest member's speed, and may hold a formation (slots relative to a
  leader, from a `*.formation.ron`).
- The weights and radii are settings per unit kind.

RVO2 and ORCA lose for now: smoother, but each unit solves a linear
program in floats, and doing that in `Fx` is a project of its own. A
later ticket can add it as a second steering answer if a game needs it.

### 7. Combat and vision

- **Targets.** A unit without an order to attack picks a target from the
  spatial grid in its bucket (§3): the nearest enemy it can see and hit,
  ties broken by row order, with a game's own rule allowed.
- **Attacks** have a range, a cooldown in steps and a damage, from the
  kind's data. **Projectiles** are rows in a table of their own, with
  the same column layout.
- **Vision.** Each team has a visibility grid on a coarser grid than the
  costs (a setting). A unit stamps its sight circle from a precomputed
  stencil per sight radius, in its bucket. A cell is *unseen*,
  *explored* or *visible*.
- **What a team knows.** `TeamView` reads only what a team can see. The
  scene for a person and the AI for a computer player both read the
  state through it (§8, §9). Whole-map vision is a setting, off by
  default, and named as a cheat in the AI's difficulty.

### 8. AI in three layers, all thinking in work units

The AI is the engine's, provided ready to use, with its behaviour as
data. Each layer runs on its own schedule. Squads and commanders
implement `Think` (3009), with a fixed number of work units per step
set in their settings (as in §5), so they decide at the same step on
every device.

1. **Units** (every unit, cheap, in buckets): reactions as data per kind,
   in `shuyi_rts::react`. They are simulation behaviour for every
   player's units, a person's too, not a computer player's commands:
   pick a target (§7), focus fire on what a squad marks, back off below
   a health share, hold or chase within a leash distance, return to a
   post. A small state machine per unit, with its state in the unit
   table.
2. **Squads** (a group under one goal): move as a group (§6), estimate a
   fight before it starts (the sum of each side's damage per step times
   its health, near the squad, through `TeamView`), engage, wait for
   help, or retreat by that estimate. Its thresholds are settings.
3. **Commanders** (one per computer player, every N steps):
   - **Influence maps** on a coarse grid: threat (enemy damage nearby),
     control (whose units hold an area) and value (resources,
     buildings). They are updated in buckets, never rebuilt whole.
   - **Utility scoring** of goals (expand, defend, attack a place,
     scout, build up), each a weighted sum of considerations read from
     the influence maps and the economy. The weights are in a
     `*.commander.ron`; a game ships several as personalities.
   - **Build orders** as data: a list of steps with conditions, followed
     until the scores say to leave it.
   - **The economy**: workers to resources, production to the plan.
   - It hands squads their goals. Squads and commanders issue commands
     only (§2), through a `CommandSource` in `shuyi_core::tick` that
     feeds the same queue as a person's commands.
- **Difficulty is settings**, never special code: how often each layer
  thinks, a delay in steps before reacting, a cap on commands a minute,
  and whether it sees the whole map.
- **Hooks for a game's own AI.** A game can replace any layer with its
  own `Think`, or add considerations to the commander, without forking
  the module.

### 9. The view: only what the camera sees

`shuyi_rts::view` (feature `view`, depends on `shuyi-ui`):

- **The scene** holds the units the camera sees (a rectangle query on
  the spatial grid), through the local team's `TeamView`, as sprite items
  on a world layer with sort keys (1120, ADR-0003 §5). A skin paints it.
- **Selection** is the player's state: a box from the pointer (1121),
  a click, all of a kind on screen, and control groups saved and
  recalled through actions in the player's keymap (`AGENTS.md` rule 8).
  It produces commands; it never changes the simulation.
- **The minimap** is a small layer drawn from the coarse vision and
  control grids, not from every unit.
- **Health bars, rally points and orders queued** are drawn for the
  selection only, so their cost follows the selection, not the army.

### 10. Faster on desktops, the same everywhere

- **Single-threaded first**, and it must meet the budgets (§11) that
  way.
- **Parallel later, bit for bit the same** (2717): a native-only `parallel`
  feature runs the per-unit passes over chunks with `rayon` (MIT OR
  Apache-2.0, checked on the day). §3's read-last-write-next rule makes
  the result independent of the split. A test runs every fixture with
  and without the feature and compares hashes every step. The web build
  and small targets never turn it on (3009: no threads).

### 11. Budgets: a thousand units is a test

ADR-0008 §6's budgets apply. The module adds bench scenarios to 3008
(2718), each a fixed `CommandRecord`:

| Scenario | Units | Gate |
| -------- | ----- | ---- |
| `rts-march-1k` | 1 000 ordered across a map | within `steam-deck`'s `frame_instructions` per step, in CI |
| `rts-battle-1k` | 2 × 500 fighting, both sides computer players | within `steam-deck`'s `frame_instructions` per step, in CI |
| `rts-battle-5k` | 2 × 2 500 | measured and reported in CI, not gated; on the Steam Deck p99 ≤ 16.7 ms is the goal |
| `rts-ai-think` | 4 commanders, 1 000 units | each commander's decision within its work-unit budget per step |

Every number is provisional until the Steam Deck device ticket (3013)
measures it. The module states its `needs` (ADR-0012 §1): it doesn't
fit the Playdate, the GBA or the DS at a thousand units, and
`shuyi check --target` says so; a smaller game on those targets is a
game's choice to test.

### 12. The economy

`shuyi_rts::economy` (2733, with the player's side in 2734):

- **Resources** are named by the game; each player has a stock, kept in
  a sorted map so it hashes the same everywhere. **Supply** caps the
  army: units use it, finished buildings provide it.
- **Resource nodes** are units of a neutral player with an amount left.
  **Workers** loop: go to a node, work, carry, go to the nearest
  drop-off, deposit. What a node loses always equals what is carried
  plus what is deposited, and a property test checks it.
- **Construction** takes the cost when a builder starts. One pure
  function, `can_place`, decides whether a footprint fits, for the
  simulation and for the player's placement ghost alike.
- **Production** is a queue per building that takes the cost when
  queued, refunds it on cancel, and sends new units to a rally point.

## Consequences

- A developer making an RTS gets pathfinding, steering, combat, fog and
  computer opponents from one module, and writes their game's units,
  rules and art as data.
- Every RTS made with the engine replays, runs headless, and can play in
  lockstep, because commands are the only input and the AI uses them
  too.
- `shuyi-core` gains `CommandSim`, `Timed` and `CommandRecord`. They are
  small and generic, and lockstep (1140, 2720) needs them in the core,
  because `shuyi-net` must not depend on a module.
- Column storage is less convenient to write than structs. The module
  hides it behind `UnitTable` methods and a `UnitRef` view, and keeps
  the arena rule for every other tick stage.
- Spreading work over buckets delays some reactions by up to `k` steps.
  That is a setting, and the README says what it costs.
- Flow fields cost memory per goal. The cache has a limit, and the
  bench scenarios measure it.
- Eighteen tickets of engine investment (2705–2720, 2733, 2734), parked
  until after the game launches. None touches the game.

## Alternatives considered

- **An ECS (`hecs`, `bevy_ecs`) for the whole module** — the common
  answer at this scale, but it costs readable state dumps, determinism
  care (archetype order, parallel schedules) and compile time. 2705
  measures before anything is built; if it wins on every rule, it
  replaces §3.
- **Units in `TickInput` as actions and the pointer** — puts every
  player's selection and mouse in the simulation and in every network
  message. Commands are smaller and are what RTS lockstep has always
  sent.
- **A* per unit** — simple, but O(units × map) when an army moves.
  Flow fields cost O(map) per goal, shared.
- **A navigation mesh** — fits open, irregular maps, but an RTS map is
  a grid of tiles and buildings that change it; a grid with sectors
  updates locally and stays integer.
- **RVO2 or ORCA for avoidance** — smoother crowds, but floats or a
  fixed-point linear program solver. Left for a later answer (§6).
- **A lower simulation rate (10–20 steps a second) with interpolation**
  — cheaper per second, but it breaks ADR-0003 §2's 60 steps without
  interpolation, and every drawing path would need to blend. Buckets
  give the same saving where it matters.
- **Behaviour trees or GOAP for the commander** — both work; utility
  scores over influence maps are data a designer can tune, cost a fixed
  amount per decision, and are easy to slice into work units. A game
  can still plug its own in (§8).
- **Time-limited AI ("think for 5 ms")** — the decision would depend on
  the device and break replays and lockstep (ADR-0008 §6).
