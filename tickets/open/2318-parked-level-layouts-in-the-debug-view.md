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

- `WorldLayer`s placed in world pixels;
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
  pixels through the layer's parallax and the camera's position and
  zoom, the same way the frame places that layer's sprites. It uses
  the one function the frame uses, so the two can never disagree.
  2317's `to_px` becomes a call to it.
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
    the world (`Bounds`).
- **The `sort` channel:** each sprite item in the world layers gets
  its sort key as a small label at its foot (`z,y,order`), and a
  `Cross` at the point its `y` is measured from. It is separate from
  `layout` because it is noisy.
- **Hot reload, if it is on (1124):** when a level is reloaded, the
  `layout` channel shows a `Note` label "reloaded <file>" for two
  seconds of frames.
- Call the helpers from the walkable town (1125) if it is done. If it
  isn't, call them from 1120's example, and 1125 calls them when it
  lands.

**Out (do not do):**
- isometric layouts (1126 adds its own projection to `push_world` if
  this ticket is done first; if not, this ticket supports it);
- editing levels (Tiled and LDtk are the editors, ADR-0004; the
  editor's choice is 2506's);
- 3D (2320).

## Implementation steps

1. Read 2315's ADR, 1120's `WorldLayer` and `Camera`, and 1124's level
   types.
2. Add `push_world` with tests that it matches where the frame draws a
   sprite at the same world point.
3. Write `debug/layout.rs` and the two channels.
4. Change 2317's `to_px` to use `push_world`, if 2317 is done.
5. Run the gates.

## Acceptance criteria

- [ ] A gizmo pushed at a sprite's world position lands on that
      sprite's console pixel, for parallax 1, ½ and 0, and zoom 1–4.
- [ ] With `layout` on, the example's snapshot lists the dead zone, the
      bounds and each object's outline and label.
- [ ] With `sort` on, every world sprite has a sort-key label.
- [ ] The replay hash is the same with the channels on and off.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `push_world` for each parallax and zoom; each helper on a small
  hand-made level, one from Tiled and one from LDtk (1124's fixtures).
- Property: for any camera position and zoom, `push_world` of a
  sprite's position equals the frame's own placement.
- Snapshot / integration: the example with `layout` on, and with `sort`
  on.

## Completion notes

*(Filled in by the session that completes the ticket.)*
