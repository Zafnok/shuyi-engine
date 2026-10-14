# ADR-0003: Game styles: stages, two kinds of simulation, cameras and projections

- **Status:** Accepted (Nick chose plan B, 2026-10-14; see `docs/decisions.md`). First proposed as the game repository's ADR-0064 (game ticket 1101)
- **Date:** 2026-10-14
- **Related tickets:** 1114 (turn rules), 1119–1125 (real time, the first
  free-movement example), 1126 (isometric), 1127 (side view), 0060 (free
  movement in our game)
- **Extends:** game ADR-0004 (determinism, time is an argument), game ADR-0017
  (screens), game ADR-0035 (one flow screen hosts the others), game ADR-0038 and
  game ADR-0054 (a scene, a skin), game ADR-0039 (a battle is its history)

## Context

Nick, 2026-10-14: the engine should support, without 3D:

1. top-down, grid-based (our game);
2. top-down, free movement;
3. isometric, free or grid movement;
4. side-scrolling;
5. moving between them as the game wants ("we ourselves might
   transition between 1 and 2 for open world section").

The audit (the game repository's [boundary map](https://github.com/Zafnok/visions-of-shuyi/blob/main/docs/engine/boundary.md), *What other game styles would
break*) found the engine-to-be assumes turns, integer tiles, square
orthogonal tiles, one unit per tile, a frame of integer console pixels
with two layers and no sort key, one backdrop, and a variable `dt` loop.
None of that is wrong for our game; all of it must become one choice
among several.

Things we will not give up, because they are why this codebase is fast to
work in and why an AI agent can work in it: the rules are deterministic
and replayable from a seed plus inputs; a test can drive the game
headlessly and read what happened as plain data; any frame can be
rendered to a PNG with no window (`frame-png`).

## Decision

### 1. A game style is a *stage*; moving between styles is moving between screens

- A **stage** is a screen (game ADR-0017) that owns a simulation, builds a
  plain-data scene from it each frame, and has a skin paint the scene
  (game ADR-0038). Our battle screen is a stage of the grid kind.
- **Changing style is a screen transition.** The game's flow screen
  (game ADR-0035) pushes or replaces stages and owns what persists between
  them (party, inventory, story flags). The engine never owns game state
  across stages; it gives the transition (fades, 0813) and the pattern.
  Example: a field stage (free movement) sees the player touch an enemy,
  returns "battle at this place with these units" to the flow; the flow
  pushes a grid battle stage built from that part of the field's tile map;
  when it ends, the flow pops it and gives the field the result.
- Each style ships with an **example game** in `examples/` that is
  also its integration test (driven by the Harness, a replay
  determinism test, a `frame-png` picture). The engine's README claims a
  style only when its example passes CI.

### 2. Two kinds of simulation, both deterministic

Both live in `shuyi-core`, pure, with no clock and no I/O (game ADR-0004).

- **Turn rules** (`TurnRules`, 1114): `check(&cmd)`, `apply(cmd) ->
  Result<Vec<Event>, Error>` (an error changes nothing),
  `legal_commands()`, `outcome()`. A match is its seed plus its command
  list (game ADR-0039). History, rewind, the bots, and lockstep networking
  (ADR-0006) are written against the trait. Our `BattleState` implements
  it.
- **Tick simulation** (`TickSim`, 1119): `step(&mut self, input:
  &TickInput) -> Vec<Event>` at a **fixed 60 steps a second**.
  `TickInput` holds, per player, the actions pressed, held and released
  this step, analog axes and the pointer, all as integers. A run is its
  seed plus its inputs per step (a `TickRecord`, run-length encoded).
  Replays, bots, headless tests and rollback networking use it.
- The engine's frame driver turns real time into steps for a tick stage
  (an accumulator, at most 4 catch-up steps a frame, then it slows down
  rather than spiral). Turn stages keep animating with `dt` as today.
  Drawing reads the latest state; there is no interpolation (pixel art at
  60 steps a second doesn't need it).

### 3. Numbers in a simulation are integers

- Positions and speeds in tick simulations are **fixed-point**:
  `shuyi_core::Fx`, an `i32` with 8 fractional bits (1/256 of a pixel),
  and `FxVec2`. Multiplication and division round toward negative
  infinity, written once in `Fx`. Angles use a 256-step table for sine
  and cosine.
- No `f32`/`f64` in any simulation or content type (as content already
  does: `TilePoint` is thousandths of a tile, game ADR-0060). Floats appear
  only in drawing, after the scene is built. This keeps native and web
  builds stepping identically, keeps `Eq` derives working, and is what
  makes replays, rollback and server checks possible.
- Rigid-body physics with floats is an optional crate outside this
  promise (ADR-0004).

### 4. Entities are plain structs in an arena, not an ECS

- A tick stage owns its things as plain structs (or an enum of kinds) in
  `shuyi_core::arena::Arena<T>`: generational ids, iteration in id order
  (deterministic), `serde` derives, and a text dump for tests and agents.
- No ECS, as game ADR-0004 decided for the rules. The examples are a town, a
  small dungeon, a platformer level: tens to a few hundred things. If an
  example needs thousands, or component combinations that structs make
  painful, 1119 records it and a spike compares `hecs` and `bevy_ecs`
  against these rules (determinism, WASM size, compile time, a readable
  dump).

### 5. The frame gains world layers; the console stays for text

The frame (`GlyphBuffer`, after 1108 configurable in size) keeps its
cells, rectangles and sprite items. It gains **world layers**, drawn
between the backdrop and the cells:

- A world layer has a **camera** (a position in `Fx`, a whole-number
  zoom 1–4 as today, follow with a dead zone, clamp to bounds, a shake
  given as data) and a **parallax factor** (a fraction; 1 is the world
  itself, ½ moves half as fast).
- Its sprite items are placed in world pixels and carry a **sort key**
  `(z, y, order added)`, so y-sorting and isometric depth are the
  engine's job, not each skin's.
- World items are never cut by cells (`cut_items` stays for UI items).
- Snapshots list world layers like the other items: one line per item.
  `frame-png` draws them.

Menus, text boxes and help bars stay glyph cells in every style
(game ADR-0038 §3).

### 6. Projections

`shuyi_ui::world::Projection` turns map coordinates into world pixels
and back. Every map scene and skin asks it instead of assuming
axis-aligned squares.

| Projection | Tile → world | World → tile (picking) | Depth key |
| ---------- | ------------ | ---------------------- | --------- |
| `TopDown { tile }` | `x·w, y·h` | divide | `y` |
| `Isometric { w, h }` | 2:1 diamond: `((x−y)·w/2, (x+y)·h/2)` | the inverse, then the nearest diamond | `x + y`, then height |
| `SideView { tile }` | `x·w, y·h`, with y down and gravity along +y | divide | layer order |

- The grid kit (pathfinding, ranges, cursor) works in tile coordinates
  and so works unchanged under any projection; only drawing and picking
  go through the projection. An isometric tactics game reuses the whole
  kit.
- Square corner autotiling (game ADR-0052) stays a top-down skin's feature;
  an isometric skin draws one picture per tile until an isometric
  autotile rule is wanted.

### 7. Collision and movement belong to the tick kit

Tile solidity (solid, one-way, later slopes), boxes, and movers that move
one pixel at a time and stop at what they hit, in `Fx` (the TowerFall and
Celeste way); triggers (a box that reports enter and leave). Details and
the optional physics crate: ADR-0004.

### 8. Input for real time

Real-time styles and editors need what the audit found missing: the
pointer (mouse or touch, in console and world coordinates) and analog
sticks as axes (today four digital directions). Both are added to the
engine's input (1121), quantized to integers in `TickInput`, and stay
rebindable through the game's `ActionSet` (game ADR-0063).

## Consequences

- A free-movement town that starts a grid battle is two stages and a few
  lines in the game's flow; both stages replay from their records and are
  testable headlessly.
- Fixed-point math is less convenient than floats for whoever writes a
  platformer controller. The engine gives `Fx` operators, the sine table
  and a tested mover, and the examples show the idioms.
- The frame grows one concept (world layers with cameras). Every effect
  that covers the whole frame (fades, game ADR-0060's brightness) must include
  them; 1120 says so.
- Our game is unaffected unless it adds a field stage (0060): the battle
  keeps its turn rules, `dt` animation and current skins.
- The styles beyond our game's (isometric, side view) are pure engine
  work. They cost what their tickets cost and are scheduled as engine
  investment (the game repository's [feasibility report](https://github.com/Zafnok/visions-of-shuyi/blob/main/docs/engine/feasibility.md)), never inside game tickets.

## Alternatives considered

- **One simulation kind for everything (all tick-based)** — our turn-based
  battle would turn into "nothing happens for 600 steps", its history
  would become step logs instead of commands, and lockstep would send far
  more. Turns and ticks are different shapes; two small traits are
  clearer.
- **Floats in simulations** — easier to write, but native and web builds
  can drift (fused multiply-add, `libm` differences), which breaks
  replays, rollback and server checks, and floats don't `Eq`. Integer
  fixed-point is what deterministic engines (and the NES) do.
- **An ECS for tick stages** — the norm in Bevy, but it costs
  determinism care, readability of the state as text, and compile time,
  for a scale our examples don't reach. Left open behind a measured
  spike (§4).
- **A separate renderer per style** — puts game knowledge in `app`, which
  nothing tests (game ADR-0038's reason), and multiplies the work.
- **Variable time step with `dt` for real time** — simple, but no two
  runs are the same, so no replays, no rollback, no bots that reproduce.
- **3D or 2.5D (height maps, a perspective camera)** — out of scope by
  Nick's word. Isometric height is only a depth and drawing offset.
