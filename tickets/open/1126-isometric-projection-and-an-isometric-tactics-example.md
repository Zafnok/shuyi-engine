---
id: "1126"
title: "Parked: Isometric: a projection for grid and free placement, depth sorting, picking, and a small isometric tactics example"
type: feature
milestone: "Engine: more styles"
model: opus-5.5
effort: xhigh
status: blocked
blocked_by: ["1125"]
nick_input: none
completed:
---

# 1126 — Isometric projection and example

> **Parked** (Nick, 2026-10-14: "document the parked (i.e. after MVP/parity) tickets for networking, styles for later"): after the engine reaches parity with the game (the MVP in `docs/ROADMAP.md`). Unparked early only if the game's ticket 0060 says the army walks around freely (then 1119–1125 become game work). Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

This is ADR-0003 §6. `Projection::Isometric { w, h }` maps a tile to world
pixels on a 2:1 diamond, and back again for picking. It sorts by
`x + y`, then by height. The tactics kit works in tile coordinates, so
pathfinding, ranges and the cursor need no change. Only drawing and
picking go through the projection.

game ADR-0052's square corner autotiling stays a top-down feature. An
isometric skin draws one picture per tile.

## Nick input

None.

## Scope

**In:**
- `shuyi_ui::world::Projection` with `TopDown`, `Isometric` and
  `SideView`, and a method for each of: tile to world, world to tile
  (the nearest diamond), and depth key. 1115 left a hook for this in the
  grid kit's math. The grid kit's scene and skins now take a
  projection.
- An isometric grid skin: per-tile pictures, units y-sorted by depth,
  and a cursor drawn as a diamond.
- Free placement on an isometric map for tick stages, so a mover in
  tile-space `Fx` is drawn through the projection.
- Example `examples/iso-skirmish/`: a 10×10 isometric map with
  height offsets as drawing only, four pieces, and the kit's turns and
  bots. It uses CC0 isometric art (for example Kenney's isometric packs).

**Out (do not do):**
- isometric autotiling;
- real elevation rules;
- 3D.

## Implementation steps

1. Write the projection with property tests. `world_to_tile(tile_to_world(t))
   == t` for all tiles. Every world pixel inside a diamond picks that
   diamond.
2. Make the grid kit use it. The game's top-down snapshots must not
   change.
3. Build the skin and the example, with a Harness playthrough, a bot
   match and snapshots.

## Acceptance criteria

- [ ] The projection property tests pass.
- [ ] The example's playthrough, bots and snapshots pass in CI. Its web
      build is on Pages.
- [ ] No snapshot of our game changes.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Property: the projection round trip and picking.
- Integration: the example.

## Completion notes
