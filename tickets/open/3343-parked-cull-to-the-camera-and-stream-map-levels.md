---
id: "3343"
title: "Parked: Skins skip world items outside the camera, and a map's levels load and drop as the camera moves"
type: feature
milestone: "Parked: real time"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: blocked
blocked_by: ["3525", "1120", "1124"]
nick_input: none
completed:
---

# 3343 — Cull world items to the camera, and stream a map's levels

> **Parked** (the blind-spot audit, 2026-11-23): it may start when Nick unparks it in decision ticket 3525 (what 1.0 is for, among them "world space" and "map import"). Until then nobody starts it. It also needs 1120 and 1124, both parked as real-time work (Nick, 2026-10-20). Whoever unparks it sets `status: todo`, drops `Parked:` from the title, and sets the milestone 3525's answer gives.

## Context

A blind-spot audit of the engine (2026-11-23) found no plan for a
world bigger than the screen:

- **The frame grows with the world.** Logic builds a plain-data frame
  each frame, and `shuyi-app` walks every item once for each layer
  (`Renderer::draw_items`, `crates/app/src/render.rs`, line 308:
  `buf.items().iter().filter(|item| item.layer() == layer)`). Nothing
  drops an item the camera can't see.
- **1120** (parked) gives world layers a camera, zoom, parallax and
  viewports. It never mentions culling: a grep for `cull` and
  `off-screen` in it finds nothing. Its placement of a world item in
  window pixels is one function in `shuyi-ui` that every painter calls.
- **2360** (parked) culls only tile layers: its `visible_tiles` works out
  the part of the world a layer shows by running 1120's placement
  backwards. Sprite items, shapes and text runs on a world layer get no
  such rectangle, and a skin has nothing to ask before it builds them.
- **The grid kit already culls**, for its own maps only: `MapScene`
  holds the visible tiles (`crates/ui/src/grid/scene.rs`, `origin`,
  `size` and `tiles`).
- **Levels load whole.** 1124's Out says "every level loads whole
  here", and ADR-0128 §3 says streaming a world's levels around a
  camera "is groups loaded and dropped as the camera moves, with each
  level a folder. A ticket when a game asks for it." No ticket was
  written. The pieces exist: `Formats::load_under`
  (`crates/assets/src/format.rs`, line 994), `Bundle::files_under` and
  `Bundle::arrived` (`crates/assets/src/bundle.rs`, lines 747 and 673),
  and 1124's `MapWorld` (each level's file, position and size).

Why it matters. A metroidvania, a top-down adventure or a large
platformer level is far bigger than the screen. Godot's `TileMapLayer`
draws in quadrants and culls; Bevy culls by visibility; Tiled worlds and
LDtk projects are split into levels and loaded a level at a time. On the
web and the Playdate (3008's budgets) a frame that holds the whole world
misses its budget long before the game is big.

Graphics are a skin (`AGENTS.md` rule 9): the view decides what is
drawn, and the rules never read the camera. Streaming here moves
pictures and map data the view draws; what the rules read stays theirs
(ADR-0002, ADR-0015 §2 keeps the camera out of the hash).

## Nick input

None (the unpark is 3525's).

## Scope

**In:**
- **The view rectangle.** `WorldLayer::world_view(&self, frame: PxRect)
  -> PxRect` in `shuyi_ui::frame`, pure and in integers: the part of the
  world, in whole world pixels, that the layer shows. It is 1120's
  placement function run backwards over the layer's viewport (or the
  frame), with its camera, zoom, parallax and shake, widened by one
  world pixel all round so a rounded edge never drops a visible pixel.
- **Culling in the world layer.** Where 1120 sorts a layer's items once
  per frame, an item whose bounds in world pixels don't touch
  `world_view` is dropped from what painters and snapshots read. Every
  painter (the reference raster, `frame-png`, `shuyi-app`) reads the
  same culled list; none tests visibility itself. A rotated sprite
  (2335) is tested by the box around its turned corners.
- **A query for skins.** `WorldView` (a `#[non_exhaustive]` plain value
  with a constructor, ADR-0039): the layer's `world_view`, with
  `contains(PxRect) -> bool` and `margin(px: u16) -> WorldView`. A skin
  asks it before it builds an item, so off-screen things are never
  built at all. `Ctx` gives a screen the `WorldView` of each of its
  world layers for the frame being drawn.
- **A snapshot count.** A world layer's snapshot line ends with
  ` culled=N` when N items were dropped (absent when 0), so a test reads
  that culling happened without reading pixels.
- **Levels that follow the camera.** In `shuyi_assets` beside 1124's
  `MapWorld`:
  - `levels_near(world: &MapWorld, view: PxRect, margin: u16) ->
    Vec<LevelId>`, pure: the levels whose rectangle touches the view
    widened by `margin` world pixels, in `MapWorld` order;
  - `LevelSet`: the levels loaded now. `LevelSet::update(&mut self,
    world, view, margin, keep: u16) -> LevelChange { load, drop }`
    loads what `levels_near` gives and drops a level only once it is
    more than `keep` pixels past the margin, so a camera at a border
    doesn't load and drop a level every frame;
  - the loading itself is ADR-0128's groups: each level is a folder of
    the bundle, read with `Formats::load_under`, and a level not yet
    `Bundle::arrived` is reported as waiting, drawn as nothing, and
    asked again next frame (the web streams its pack). A level dropped
    is its values dropped.
  - `LevelId`, `LevelSet` and `LevelChange` are `#[non_exhaustive]` with
    constructors.
- **What the rules read stays whole.** The rules never see a
  `LevelSet`. A test runs one tick game's replay with two window sizes
  and two `margin`s and gets the same `TickSim::hash` at every step.
  The skill says a game that needs per-level rules data loads it by the
  rules' own position, not the camera's.
- **A bench scenario. From 3008 (done):** a `.bench.ron` scenario
  (`docs/targets/README.md`) that scrolls a camera across a world of
  10,000 sprite items and 16 levels, so the frame's item count and
  instructions are shown to follow the view, not the world.
- **Either order with 2360:** if 2360 is done, its `view` for
  `visible_tiles` becomes `WorldLayer::world_view`, with its tests
  unchanged; if not, nothing to do (2360 calls `world_view`).
- **Docs:** the world-layer chapter of the book (2802) and 1120's
  section of the `shuyi-stage-field` skill (`skills/shuyi-stage-field/SKILL.md`)
  teach `WorldView` and `LevelSet`. Update ADR-0128 §3's streaming
  line to point at this ticket's API, and ADR-0003 §5 to say world
  layers are culled before any painter reads them.

**Out (do not do):**
- a spatial index for many entities' queries: 2708 (parked) builds the
  RTS's uniform grid. A skin here tests each item's box, which is cheap
  next to building it;
- chunk caching of tile layers (pictures of chunks rebuilt on change):
  2360's Out leaves it to a follow-up if 3008's numbers ask;
- culling in the grid kit: `MapScene` already holds only the visible
  tiles;
- splitting one Tiled or LDtk level into chunks of its own: a level is
  the unit here, as ADR-0128 §3 says;
- loading on a thread: 3344 (native worker pool) is its own ticket;
- the draw-call counter and stress scenario of the renderer: 3341.

**Estimated size:** about 1,400 lines (code 650, tests 550, docs 200).

**Split:** past 900 lines, split off steps 5 and 6 (`levels_near`,
`LevelSet` and their loading) into a new ticket, parked like this one
and blocked by it.

## Implementation steps

1. Read 1120's `WorldLayer`, `Camera`, sort and placement function, and
   1124's `MapWorld`. If 1124 moved `MapWorld` to a follow-up ticket
   (its *Levels and worlds*), add that ticket to `blocked_by` and stop
   until it is done.
2. Write `WorldLayer::world_view` as the inverse of the placement
   function, in `shuyi-ui`, with unit and property tests.
3. Cull in the world layer's per-frame sort; add ` culled=N` to the
   snapshot line; check the raster, `frame-png` and `shuyi-app` read
   only the culled list.
4. Add `WorldView` and hand it to screens through `Ctx`.
5. Write `levels_near`, `LevelSet` and `LevelChange` in `shuyi-assets`,
   pure, with tests.
6. Load and drop levels through `Formats::load_under` and
   `Bundle::arrived`, and test a level that hasn't arrived.
7. Add the bench scenario and the replay test of the hash.
8. Do 2360's part as its clause says, update the docs and ADRs, and run
   the gates.

## Acceptance criteria

- [ ] For any camera position, zoom 1 to 4, parallax and viewport,
      `world_view` holds every world pixel the layer draws and is at
      most the viewport's size over the zoom, plus 2, each way.
- [ ] An item one world pixel inside the view's edge is drawn, and one
      wholly outside is dropped and counted in ` culled=N`.
- [ ] The raster pictures of 1120's test scene don't change.
- [ ] On the bench scenario, the frame's item count stays below the
      view's share of the world at every frame, and the scenario's
      instruction count is committed.
- [ ] `LevelSet::update` loads the levels that touch the widened view,
      and a camera moving back and forth by one pixel across a border
      never drops and reloads a level.
- [ ] A level not yet arrived is reported waiting, draws nothing, and
      loads on a later frame without an error.
- [ ] A tick game's replay gives the same hash at every step under two
      window sizes and two margins.
- [ ] No snapshot of the game or of the examples changes.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `world_view` at the corners, past an edge, with a viewport and
  with parallax; culling of a rotated sprite; `levels_near` on 1124's
  LDtk fixture; `LevelSet`'s keep distance; the snapshot line.
- Property: `world_view` covers every drawn pixel and is bounded;
  `levels_near` holds every level touching the view and no other.
- Snapshot / integration: the bench scenario; the replay hash under two
  window sizes; a streamed web pack where a level arrives late.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
