---
id: "2106"
title: "Parked: Paths for free movement: a navigation mesh built from a tile collision map, routes over it, and a path a mover follows"
type: feature
milestone: "Engine: real time"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: blocked
blocked_by: ["1122"]
nick_input: none
completed:
---

# 2106 — Paths for free movement: a navigation mesh

> **Parked** (Nick, 2026-10-14: "document the parked (i.e. after MVP/parity) tickets for networking, styles for later"): a follow-up of the parked collision kit (1122), so it starts after 1122 is unparked and done. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Found by the 2026-10-20 comparison with Bevy 0.20 and Godot 4.6. Godot
has `NavigationServer2D`, `NavigationRegion2D` and `NavigationAgent2D`;
Bevy uses crates such as `vleue_navigator`. Shuyi has:

- `shuyi_core::path`: Dijkstra over a square grid in four directions,
  for turn games, where a walker steps from tile to tile;
- 1122's collision kit: free movers in `Fx` that stop at walls, and
  nothing that finds a way round them;
- 2709 and 2710 (parked until after the game launches): sectors,
  portals and flow fields shared by armies of a thousand units. Their
  ADR (ADR-0015) lists a navigation mesh among the alternatives it
  didn't take for armies.

A few free movers need a way round walls too: a villager walking to the
shop in a farming game, an enemy chasing the player in a top-down action
game, a follower in a platformer's overworld. This ticket gives them
that, on 1122's `TileCollision`, deterministic and in `Fx`.

The mesh is axis-aligned rectangles, because the walls come from tiles:
rectangles cover a tile map exactly, merge in a fixed order, and need no
triangulation library.

## Nick input

None.

## Scope

**In:**
- **`shuyi_core::nav`**, beside `path` (pure, `no_std`, in `Fx`):
  - `NavMesh::from_tiles(&TileCollision) -> NavMesh`:
    - walkable tiles are covered by rectangles, merged greedily in row
      order (widest run first, then as many rows down as fit), so the
      same map always gives the same mesh;
    - one-way tiles count as solid (they are a side-view thing;
      documented);
    - a **portal** is the shared edge of two touching rectangles.
  - `NavMesh::locate(point) -> Option<RectId>` and
    `nearest_walkable(point) -> Option<(Fx, Fx)>`.
  - `NavMesh::route(from, to, radius_px) -> Option<Path>`:
    - A* over rectangles; a step costs the distance between portal
      midpoints, and the heuristic is the straight distance. Ties break
      by `(f, g, rect id)`;
    - **clearance:** each portal end that meets a wall moves inward by
      the radius. A portal shorter than twice the radius is closed;
    - the corridor of portals becomes corner points with the funnel
      algorithm ("simple stupid funnel", Mikko Mononen, 2010, from
      its description);
    - `None` when `from` or `to` is not walkable or they aren't
      connected.
  - **Square roots:** the distances need `Fx::sqrt`. **Either order
    with 2701 and 2122:** if either is done, use its `Fx::sqrt`. If
    not, add it here as 2701 specifies (the floor of the exact root,
    through `i64`), and change 2701's and 2122's text to say it exists.
  - `Path { points: Vec<(Fx, Fx)> }` and `Path::follow(&mut self, pos,
    speed) -> (Fx, Fx)`: the motion for this step toward the next
    point, dropping a point when it is reached and slowing to stop
    exactly on the last one. A game passes the motion to 1122's
    `Mover`.
  - **Either order with 2122:** if 2122 is done, `Path` holds a
    polyline `shuyi_core::curve::Curve` (`Curve::lines(points, false)`)
    instead of its own `points`, and `follow` reads its corners from
    it. `Curve::lines` refuses two points in one place, so a route
    from a point to itself is handled before the curve is built. If
    2122 isn't done, write `Path` as above, and 2122 then writes a
    follow-up ticket that makes the change. `follow` stays this
    ticket's: it steers from where the mover really is, which
    collision can stop, and 2122's `CurveFollower` (which moves by
    distance, and nothing stops) doesn't do that.
- **Changes to the map:** rebuild the mesh. Document the cost
  (measured on a 256 × 256 map in a test), and nothing incremental.
- **The debug view:** if 2315 is done, a `nav` channel helper,
  `shuyi_ui::debug::nav::gizmos(&NavMesh, Option<&Path>, to_px,
  &mut Gizmos)`: rectangles (`Bounds`), portals and the path (`Path`).
  If 2317 is done, use its `to_px` and `debug::collide::px`; if not,
  take the caller's offset, as 2317 does, and 2317 moves the helper
  onto `px`. If 2315 isn't done, write a follow-up ticket for the
  helper, blocked by 2315.
- **The example:** in 1119's tick example (the dot's room), a second
  dot follows the first round a wall.

**Out (do not do):**
- polygon obstacles from Tiled or LDtk object layers: a follow-up
  after 1124 if a game asks (the mesh here is tile rectangles only);
- movers avoiding each other (steering; for armies, 2711);
- following a curve by distance, with loops and ping-pong (2122's
  `CurveFollower`);
- jumps, ladders and other links between areas;
- crowds and flow fields (2709, 2710);
- 3D navigation;
- code over a store: `NavMesh`, `Path` and `route` name neither
  `Arena<T>` nor `shuyi-ecs`'s `World` (2904's ADR), and a game keeps
  each follower's `Path` in whichever store it uses.

## Implementation steps

1. Read 1122's `collide`, `shuyi_core::path` (its determinism rules),
   2709's text for the A* tie-breaks it chose, and 2122's
   `shuyi_core::curve` if it is done.
2. Write the rectangle merge and portals, with tests on hand-drawn maps.
3. Write `route` with clearance and the funnel.
4. Write `Path::follow`.
5. Add the debug helper if 2315 is done, or write its follow-up ticket
   if not, and add the example.
6. Run the gates.

## Acceptance criteria

- [ ] Every rectangle covers only walkable tiles, and together they
      cover all of them, with no overlap (a property test).
- [ ] A route's segments, checked every pixel, keep `radius` clear of
      every solid tile (a property test on random maps).
- [ ] A route exists exactly when a flood fill of the eroded map joins
      the two points.
- [ ] The same map, start and goal give the same points on every run.
- [ ] A corridor narrower than twice the radius gives `None`.
- [ ] The example's replay hash is stable across runs and builds.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: a U-shaped wall; a corridor exactly as wide as the agent; a
  start inside a wall; a goal in a closed room.
- Property: cover and no overlap; clearance; determinism; `follow`
  never overshoots the last point.
- Snapshot / integration: the example, and the `nav` channel's snapshot
  if 2315 is done.

## Completion notes

*(Filled in by the session that completes the ticket.)*
