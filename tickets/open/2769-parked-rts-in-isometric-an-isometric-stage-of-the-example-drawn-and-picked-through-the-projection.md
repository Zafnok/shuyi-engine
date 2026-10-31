---
id: "2769"
title: "Parked: RTS in isometric: an isometric stage of the RTS example, drawn and picked through the projection, and shuyi-rts claims isometric"
type: feature
milestone: "Engine: real-time strategy"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: blocked
blocked_by: ["2719", "1126"]
nick_input: none
completed:
---

# 2769 — RTS in isometric: an isometric stage of the example

> **Parked** (Nick, 2026-10-18: "make tickets parked for after visions of shuyi launches for all of these"): a follow-up of 2719, and a follow-up of parked work is parked: after Visions of Shuyi launches, which is later than the MVP in `docs/ROADMAP.md`. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

[ADR-0015](../../docs/adr/0015-real-time-strategy-at-scale-command-sims-columns-flow-fields-and-ai-in-layers.md)
§1: `shuyi-rts` serves the top-down free and isometric perspectives.
The simulation is in world `Fx` coordinates over a tile grid, and only
drawing goes through the projection. 2707 writes both directions in the
crate's `serves`, and `docs/modules.md` lists both.

[ADR-0009](../../docs/adr/0009-modules-a-small-default-engine-and-opt-in-crates.md)
§4: a module's README claims a direction only when an example of it
passes CI. 2719 builds `examples/rts` as a top-down free game and claims
top-down free only. Its **Out** leaves an isometric example to this
ticket, after 1126.

1126 writes `shuyi_ui::world::Projection::Isometric`, the `Isometric`
perspective marker, free placement of tile-space `Fx` points, depth
keys and picking
([ADR-0003](../../docs/adr/0003-game-styles-stages-two-kinds-of-simulation-cameras-and-projections.md)
§6). It uses them for the grid kit and its own example, not for the RTS
view. 2716 built the RTS view (the scene, selection, control groups and
the minimap) and left isometric drawing out. This ticket joins the two:
the RTS view draws and picks through the stage's projection, and
`examples/rts` gains an isometric stage that proves it.

ADR-0009 §3: the scene of a perspective stage is generic over
`P: Perspective`, and the drawing side reads `P`'s projection.

The work this builds on is done before 2719, so it is done by then:
2716's view, 2734's placement ghost, 2713's `TeamView`, and 1120's
world layers, cameras, sort keys and perspective markers.

Nick, 2026-10-18 (`docs/decisions.md`): "Inspired by, nothing copied."
No name, unit, map, number, art or sound comes from an existing strategy
game.

## Nick input

None.

## Scope

**In:**
- **The projection's inverse.** The RTS view turns pixels into map
  points, not only into tiles. If 1126's `Projection` has no world to
  map that keeps the fraction (its world to tile gives the nearest
  diamond), add `Projection::world_to_map(world) -> FxVec2`, the
  inverse of free placement's tile-space `Fx` to world, for every
  variant, with a property test. A new method on the type is additive.
- **The view through the projection** (`shuyi_rts::view`, behind the
  crate's `view` feature):
  - `RtsScene`, the picks of `Selection`, 2734's placement ghost and
    the minimap take the stage's perspective `P` (1120's trait for 2D
    markers) and ask `P`'s projection for every step between map and
    pixels. Where the view maps top-down by hand, that moves behind
    `P`, with `TopDown` as the marker of the existing stages. The
    top-down stage's scene, snapshots and pictures don't change;
  - a unit's map position, divided by the terrain grid's cell size, is
    its tile-space point. Its item is placed at the projection's tile
    to world of that point, with a sort key from the projection's depth
    key (1126);
  - projectiles, fog cells (drawn as diamonds), health bars and queued
    order markers are placed the same way;
  - **map bounds of a pixel rectangle:** one helper,
    `shuyi_rts::view::project::map_bounds(projection, world_rect)`, the
    bounding box in map space of the rectangle's four corners through
    `world_to_map`. The scene queries `TeamView::units_in_rect` with
    the camera's map bounds, then keeps the units drawn inside the
    camera's rectangle;
  - **a box drag** queries the same way and keeps the own units drawn
    inside the dragged rectangle;
  - **a click** turns the pointer into a map point through
    `world_to_map` and picks as on the top-down stage: the nearest own
    unit within its radius of its foot, through
    `TeamView::units_within`;
  - **the context order** sends `Move`, `AttackMove` and `Patrol` to
    that map point, and `Attack` on the enemy picked there;
  - **the placement ghost** snaps to the cell under the pointer
    through the projection's world to tile, and is drawn as diamonds.
    With no pointer, the movement actions move it one cell along the
    map's axes, as on the top-down stage;
  - **the minimap** of an isometric stage is drawn through the same
    projection, scaled down to fit its box, so the camera's rectangle
    stays a rectangle on it. A press on it maps back by the same scale
    and moves the camera there.
- **The isometric stage**, `examples/rts` stage `isometric`, beside
  `battle-1k` and picked the same way:
  - the same map, sides, kinds, commander and difficulty presets as
    the main stage, on the `Isometric` marker;
  - the example's skin gains an isometric form: one diamond picture
    per terrain class, drawn as one sprite item per cell in the
    camera's map bounds; units, buildings and the resource as 2719's
    shapes; fog as diamond overlays;
  - the same keymap file. No key is named anywhere else, help text
    included.
- **Art:** the diamond pictures, original placeholders made by us in
  2719's style (plain shapes and colours), released CC0-1.0, each
  listed in `THIRD_PARTY_ASSETS.md` as our own. No bought or downloaded
  art.
- **Tests:** the Harness test, the replay test, the property tests and
  the `frame-png` snapshots in *Tests required*.
- **The claim:**
  - `crates/rts/README.md` claims isometric beside top-down free, in
    section 1, and says what an isometric RTS changes: the stage's
    marker, and so drawing and picking only;
  - `docs/guides/rts.md` names the `isometric` stage under the example
    to read first;
  - `docs/guides/isometric.md` lists `examples/rts`'s `isometric`
    stage beside the isometric tactics example.
- **The `shuyi-rts` skill. From 1134 (done):** the skill
  (`skills/shuyi-rts/SKILL.md`, which 2707 writes and 2719 gives its
  worked example) says the module serves isometric, names the
  `isometric` stage of `examples/rts`, and says that only the stage's
  marker changes; `check-skills` passes.
- **Public API:** `Projection::world_to_map` (if added) and the
  perspective parameter on the view's types. A new public type follows
  ADR-0039: `#[non_exhaustive]` with a constructor if it may grow. The
  game doesn't use `shuyi-rts`, but if CI's `api` job reports a break,
  write a `Breaking:` line for it in the completion notes.

**Out (do not do):**
- new kinds, maps, sides or art beyond 2719's placeholders: the
  isometric pictures are the same plain shapes drawn as diamonds;
- isometric autotiling (ADR-0003 §6: an isometric skin draws one
  picture per tile);
- an isometric tile-layer item for Tiled and LDtk maps: 1126's
  follow-up ticket for isometric tile layers. This stage draws its
  terrain as one sprite item per cell in the camera;
- height: no cliffs, ramps or high ground (ADR-0003: isometric height
  is only a depth and drawing offset);
- picking by a sprite's drawn shape: a click picks at the unit's foot,
  as on the top-down stage;
- an isometric template part: 2719's `kind-rts` part stays top-down
  free;
- an isometric thousand-unit stage: `battle-1k` stays top-down;
- anything from Visions of Shuyi, and anything copied from existing
  strategy games.

## Implementation steps

1. Read 1126's `Projection` and its free placement. Add
   `world_to_map` and its property test if it is missing.
2. In `crates/rts/src/view/`, give the scene (`scene.rs`), the picks
   (`select.rs`), the ghost (`place.rs`) and the minimap
   (`minimap.rs`) the perspective parameter, and route every step
   between map and pixels through `P`'s projection. Add
   `view/project.rs` with `map_bounds`. Run the existing view tests:
   the top-down scene text and snapshots must not change.
3. Write the isometric minimap and the diamond fog and ghost.
4. Add the `isometric` stage and the skin's isometric form to
   `examples/rts`. Draw the placeholders and list them in
   `THIRD_PARTY_ASSETS.md`.
5. Write the Harness test, the replay test, the property tests and the
   snapshots.
6. Update the README, the two guides and the skill.

## Acceptance criteria

- [ ] The Harness test on the `isometric` stage box-selects three own
      units, moves them to a pointed place and orders an attack on a
      pointed enemy, as the scene and the command record show.
- [ ] The commands of a recorded easy match on the `isometric` stage,
      replayed on the top-down stage, give the same hash at every step.
- [ ] The isometric scene holds exactly the units a brute-force scan
      finds drawn inside the camera's rectangle that the team can see
      (property test).
- [ ] A box drag on the isometric stage selects exactly the own units
      drawn inside it (property test).
- [ ] `world_to_map` of the drawn point of any tile-space point gives
      that point back, to within one `Fx` step (property test), if this
      ticket adds it.
- [ ] The top-down stage's scene text, snapshots and `frame-png`
      pictures don't change.
- [ ] Both new `frame-png` snapshots pass, and the example's web build
      still builds in CI.
- [ ] Every new asset is listed in `THIRD_PARTY_ASSETS.md` as CC0-1.0,
      made by us.
- [ ] `cargo xtask check-keys` (or its successor) passes on the example.
- [ ] The README claims top-down free and isometric, both guides name
      the `isometric` stage, and `cargo xtask check-modules` passes.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: a click on the isometric stage picks the unit under the
  pointer; an `Order` on an enemy there sends one `Attack` with the
  selected ids; the ghost snaps to the cell under the pointer;
  `map_bounds` of a fixture rectangle.
- Property: `world_to_map` round trip (if added); the isometric scene
  against brute force; box selection against brute force.
- Integration: the Harness test on the `isometric` stage.
- Determinism: the isometric match's commands replayed on the top-down
  stage give the same hashes.
- Look: `frame-png` snapshots of the `isometric` stage at the start of
  the match, and mid-fight with a box selection drawn.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
