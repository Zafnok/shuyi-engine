---
id: "2318"
title: "Parked: Level layouts in the debug view: world layers, the camera's dead zone and bounds, sort keys, and Tiled and LDtk objects"
type: feature
milestone: "Engine: real time"
model: sonnet-5
effort: medium
status: blocked
blocked_by: ["2315", "1120", "1124"]
nick_input: none
completed:
---

# 2318 — Level layouts in the debug view

> **Parked** (Nick, 2026-10-14: "document the parked (i.e. after MVP/parity) tickets for networking, styles for later"): a follow-up of the parked world layers (1120) and map import (1124), so it starts after both are unparked and done, and after the debug view (2315). Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Nick, 2026-10-19 (`docs/decisions.md`): see "level layouts ... in
realtime (debug mode)". 2315's debug view has a `grid` channel for the
grid kit, which is enough for turn-based maps. A free-moving world
needs more. 1120 gives it:

- `WorldLayer`s placed in world pixels, each with an optional viewport
  (split screen, insets);
- a `Camera` with a dead zone, bounds and shake;
- parallax;
- sort keys `(z, y, order)`.

1124 loads levels from Tiled and LDtk, with their object layers.
When a camera snaps oddly, a sprite sorts behind a wall, or an object
sits in the wrong place, a developer wants to see these numbers drawn
over the world.

This ticket adds:

- the camera transform that gizmos in world pixels need;
- a `layout` channel;
- a `sort` channel.

2317 (collision) and 2319 (skeletons) use the same transform.

## Nick input

None.

## Scope

**In:**
- **World gizmos:** `Gizmos::push_world(channel, role, gizmo, &Camera,
  &WorldLayer)`. It maps a gizmo given in world pixels to console
  pixels through the layer's parallax, its viewport (1120) and the
  camera's position and zoom, the same way the frame places that
  layer's sprites, and drops what falls outside the viewport. It uses
  the one function the frame uses, so the two can never disagree.
  That function is public too, as `world_to_px(&Camera, &WorldLayer,
  point)`: a world point to a console pixel. 2317's `to_px` becomes a
  call to it, since 2317's helpers map points before they push.
  **Either order with 1126:** if 1126 is done, `push_world` on an
  `Isometric` layer maps the gizmo through 1126's
  `Projection::Isometric`, with a test that it lands where the frame
  draws a sprite at the same tile; if not, nothing to do (1126 adds
  its projection to `push_world`).
  **Either order with 2122:** if 2122 is done and its
  `shuyi_ui::debug::curve::gizmos` takes an offset from the caller,
  make it push through `push_world` too; if not, nothing to do (2122
  pushes through `push_world`).
- **The `layout` channel**, helpers in `shuyi_ui::debug::layout`:
  - the camera's dead zone (`Note`), and its bounds where they are in
    view (`Bounds`);
  - its target, as a cross (`Note`);
  - each world layer's name and parallax factor, as a label in a
    column at the left;
  - the outline of every object from a Tiled object layer or an LDtk
    entity layer, with its name or type as a label (`Note`), from
    1124's level types;
  - the level's own bounds, and, for LDtk, each level's rectangle in
    the world (`Bounds`);
  - **either order with 2362:** if 2362 is done, each room's rectangle
    (`Bounds`) with its index as a label, the look-ahead point, and
    during a blend the blend's goal, as crosses (`Note`). If 2362 isn't
    done, 2362 adds them to this channel;
  - **either order with 2360:** if 2360 is done, each tile-layer item's
    map bounds and its `visible` tile rectangle (`Bounds`). If 2360
    isn't done, 2360 adds them to this channel when it lands.
- **The `sort` channel:** each sprite item in the world layers gets
  its sort key as a small label at its foot (`z,y,order`), and a
  `Cross` at the point its `y` is measured from. It is separate from
  `layout` because it is noisy.
- **Hot reload. Either order with 2220:** if 2220 is done, when a
  `Hot<TileMap>` reloads, the `layout` channel shows a `Note` label
  "reloaded <file>" for two seconds of frames; if not, 2220 adds it to
  this channel when it lands.
- **Either order with 2361:** 2361 draws a smoothed world layer
  between the last two steps, through a blended camera. If 2361 is
  done, a gizmo on a smoothed layer goes through the layer as drawn
  (2361's `StepViews::drawn`): say so in `push_world`'s doc, and test
  that a gizmo at a tile's world point lands where the frame draws
  that tile. If not, nothing to do (2361 does it).
- Call the helpers from the walkable town (1125) if it is done. If it
  isn't, call them from a test scene of this ticket's own: 1120's world
  layers and camera over one of 1124's fixture levels, with its
  objects. 1125 calls them when it lands. "The example" below means
  the town, or that test scene.

**Out (do not do):**
- isometric layouts beyond `push_world`, whose clause with 1126 is in
  Scope;
- editing levels (Tiled and LDtk are the editors, ADR-0004; the
  editor's choice is 2506's);
- 3D (2320).

## Implementation steps

1. Read 2315's ADR, 1120's `WorldLayer` and `Camera`, and 1124's level
   types.
2. Add `push_world` with tests that it matches where the frame draws a
   sprite at the same world point.
3. Write `debug/layout.rs` and the two channels.
4. Change 2317's `to_px` to call `world_to_px`, if 2317 is done.
5. Do the either-order parts for 1125, 1126, 2220, 2360, 2361 and
   2362.
6. Run the gates.

## Acceptance criteria

- [ ] A gizmo pushed at a sprite's world position lands on that
      sprite's console pixel, for parallax 1, ½ and 0, and zoom 1–4.
- [ ] With `layout` on, the example's snapshot lists the dead zone, the
      bounds and each object's outline and label.
- [ ] If 2362 is done, with `layout` on, the example's snapshot lists
      each room and the look-ahead point.
- [ ] If 2360 is done, with `layout` on, the example's snapshot lists
      each tile layer's map bounds and `visible` rectangle.
- [ ] If 2220 is done, a reloaded map shows its "reloaded <file>" note
      in the `layout` channel.
- [ ] With `sort` on, every world sprite has a sort-key label.
- [ ] The replay hash is the same with the channels on and off.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `push_world` for each parallax and zoom, and in a viewport that
  isn't at (0, 0), where a gizmo outside the viewport is dropped; each
  helper on a small hand-made level, one from Tiled and one from LDtk
  (1124's fixtures).
- Property: for any camera position, zoom and viewport, `push_world` of
  a sprite's position equals the frame's own placement.
- Snapshot / integration: the example with `layout` on, and with `sort`
  on.

## Completion notes

*(Filled in by the session that completes the ticket.)*
