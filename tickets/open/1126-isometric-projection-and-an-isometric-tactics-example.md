---
id: "1126"
title: "Parked: Isometric: a projection for grid and free placement, depth sorting, picking, and a small isometric tactics example"
type: feature
milestone: "Engine: more styles"
model: reasoning
effort: xhigh
status: blocked
blocked_by: ["1125"]
nick_input: none
completed:
---

# 1126 — Isometric projection and example

> **Parked** (Nick, 2026-10-14: "document the parked (i.e. after MVP/parity) tickets for networking, styles for later"; 2026-10-20: "unless these real-time work tickets overlap with visions of shuyi they should be parked until visions of shuyi is complete"): after Visions of Shuyi launches, not only after the MVP (`docs/decisions.md`, 2026-10-20). Unparked early only if the game needs it: its ticket 0060 says the army walks around freely (then 1119–1125 become game work), or it asks for a piece of this through its `engine-change` skill, which is then split out and unparked alone. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

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
  projection. **Either order with 2405:** if 2405 is done, its
  `Projection::tile_at` (moved onto this `Projection`) and
  `GridSkin::tile_at` pick through this world to tile, so the pointer
  picks isometric tiles, and 2405's round-trip property test (every
  pixel of every drawn tile gives that tile) runs on the isometric skin
  too. If not, add to 2405's scope in this PR: "`tile_at` picks
  through every variant of `shuyi_ui::world::Projection` (1126), and
  the round-trip property test runs on the isometric skin too".
  **Either order with 1127:** if 1127 is done, its `SideView` marker
  already has a `SideView { tile }` projection: move it into this
  `Projection` beside `TopDown`, with its tests. If not, add
  `SideView { tile }` here, and 1127's marker uses it.
- An isometric grid skin: per-tile pictures, units y-sorted by depth,
  and a cursor drawn as a diamond.
- Free placement on an isometric map for tick stages, so a mover in
  tile-space `Fx` is drawn through the projection.
- Example `examples/iso-skirmish/`: a 10×10 isometric map with
  height offsets as drawing only, four pieces, and the kit's turns and
  bots. It uses CC0 isometric art (for example Kenney's isometric packs).
- The `Isometric` perspective marker (ADR-0009 §3), whose projection is
  `Projection::Isometric`. The grid kit takes any marker.
- Guides (ADR-0009 §5): write `docs/guides/isometric.md` in full. If 2004
  isn't done, create the file and 2004 adds it to its list.
- The new-game wizard (ADR-0013 §3–§4): set the `isometric` answer in
  `docs/modules.ron` (2003 put it there, parked) to ready, as
  `top-down-grid` is: `status: ready`, `tickets: []`. If 2503 is done,
  add the part `stage-isometric` to the built-in template: a small
  isometric grid stage and its test. If 2503 isn't done, 2503 adds it.
- **The debug view (2318).** If 2318 is done, add the isometric
  projection to its `Gizmos::push_world`, with its test. If it isn't,
  2318 supports isometric layers itself.
- **Moving to a pointed place. Either order with 2761:** if 2761 is
  done, `PointMove` (`shuyi_action::point_move`) turns the pointer
  into tile space through this projection's world to tile on an
  isometric stage, before it picks the pointed place and the body
  there, with a Harness test that clicks a tile and walks to it. If
  not, nothing to do: 2761 picks through this projection.

**Out (do not do):**
- isometric autotiling;
- isometric tile layers from Tiled maps through 2360's tile-layer item
  (tiles placed by `Projection::Isometric`, drawn back to front, culled
  through world to tile of the view's corners): a follow-up ticket,
  written when this lands (step 4). 2360 is done by then (it blocks
  1125, which blocks this ticket), draws orthogonal maps only and
  refuses an isometric one;
- real elevation rules;
- hex grids (2121, 2740, parked until after the launch): a hex map is
  another topology, not a projection of the square grid (2121's ADR
  amends ADR-0003 §6). If 2740 is done first, carry its
  `Projection::Hex` into `shuyi_ui::world::Projection` unchanged, with
  its tests, and give it the depth key 2740 names (the row for pointy
  top; for flat top, the row, then 1 on a shifted column and 0 on the
  others);
- 3D.

## Implementation steps

1. Write the projection with property tests. `world_to_tile(tile_to_world(t))
   == t` for all tiles. Every world pixel inside a diamond picks that
   diamond.
2. Make the grid kit use it, 2405's `tile_at` included if 2405 is done.
   The game's top-down snapshots must not change.
3. Build the skin and the example, with a Harness playthrough, a bot
   match and snapshots.
4. Write the follow-up ticket for isometric tile layers (Out) with the
   `write-ticket` skill.

## Acceptance criteria

- [ ] The projection property tests pass.
- [ ] The example's playthrough, bots and snapshots pass in CI. Its web
      build is on Pages.
- [ ] No snapshot of our game changes.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Property: the projection round trip and picking; 2405's tile round
  trip on the isometric skin, if 2405 is done.
- Integration: the example.

## Completion notes
