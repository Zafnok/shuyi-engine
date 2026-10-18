---
id: "3107"
title: "Parked: HD-2D: shuyi-diorama, a skin that draws a 2D stage in 3D, with terrain from tiles and heights, billboard characters, picking and named effects"
type: feature
milestone: "Parked: 3D"
model: opus-5.5
effort: xhigh
status: blocked
blocked_by: ["3104", "1123", "1124", "3017"]
nick_input: none
completed:
---

# 3107 — HD-2D: `shuyi-diorama`, a skin that draws a 2D stage in 3D

> **Parked** (Nick, 2026-10-17: "I want to expand on this and add true 3D support (all kinds, first person, third, top down with 3D models), and HD-2D"): after the engine reaches parity with the game (the MVP in `docs/ROADMAP.md`) and after the 2D work already planned (`docs/decisions.md`, 2026-10-17). Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

[ADR-0011](../../docs/adr/0011-3d-fixed-point-space-a-3d-view-in-the-frame-and-hd-2d-as-a-skin.md)
§1, §2 and §7. HD-2D is a look: a low 3D world built like a diorama,
pixel-art characters standing in it, a tilted camera, and lights,
shadows, depth of field, bloom and fog. The game under it is a 2D game.
So HD-2D isn't a perspective. It is a **skin** that paints a
`TopDownGrid`, `TopDown` or `Isometric` stage in 3D. The stage's rules,
tests and replays don't change (`AGENTS.md` rule 9).

**Inspired by, nothing copied** (`docs/decisions.md`, 2026-10-17): no
name, art, asset or number from Square Enix's games goes into this
crate, its fixtures or its docs.

It builds on 3104 (the GPU painter, its shadow map and effects; through
it 3103's 3D view and rasterizer), 1123 (animation clips, so characters
are billboards of their clips), 1124 (`TileMap` from Tiled and LDtk,
its layers and objects, and hot reload) and 3017 (target capabilities,
and `shuyi check --target` failing on a need the target lacks).

## Nick input

None.

## Scope

**In:**
- **The crate** `crates/diorama` (`shuyi-diorama`), following ADR-0009
  §4. It depends on `shuyi-3d` with `draw`, `shuyi-ui` and
  `shuyi-assets`. It is a drawing part only, so it has no simulation
  half:
  - metadata: concern *World drawing*, answer diorama (HD-2D),
    alternatives `shuyi-3d`; `serves`: `top-down-grid`,
    `top-down-free`, `isometric`; `needs` (3017): `three_d: hardware`;
  - the `diorama` feature in `shuyi`; the catalog row set to building;
  - the README's first two sections (what it is for, what it is not
    for: a 3D game's rules, which are `shuyi-3d`'s). 3109 finishes the
    rest.
- **The skin file** (`*.diorama.ron`), through the format registry
  (2201) as asset kind `diorama_skin`, with a fuzz-style test and
  errors naming file and line. It holds:
  - `tile_size_m` (default 1) and `height_step_m` (default 0.5);
  - `heights`: a named layer of the map, and how its tile ids give a
    height (the id less the tileset's first id, or a table). If 1124
    imports LDtk IntGrid layers, those work too. A game may hand
    heights from its own map data instead (`Heights::from_grid`);
  - `tiles`: per tile kind, `Block { top, side }`, `Cliff { side }`,
    `Slope { dir, top }` or `Flat { top }`, where `top` and `side` name
    tiles of the map's tilesets;
  - `layers`: which world layers of the scene are `Actors`
    (billboards) and which object layer gives `Props`;
  - `props`: per object kind, a mesh (3102) or a billboard;
  - `shadow`: `Blob` (a dark quad on the ground) or `Cast` (the
    billboard casts into the shadow map);
  - `camera`: `tilt: Angle16`, `distance`, `fov_y`, and how the 2D
    zoom maps to distance;
  - `lights`;
  - `effects`: a list, each with its name (`depth_of_field`, `bloom`,
    `fog`, `light_shafts`, `vignette`, `colour_grade`), its settings,
    and `optional: bool`.
- **Terrain**: built from the map and the heights when the stage
  starts and again on hot reload, as one `Mesh` per chunk of 16 × 16
  tiles, with the tile images as textures and `pixel` on.
- **`DioramaSkin::build(scene, map, frame)`**: reads the 2D world scene
  (1120) and adds one `View3d` (3103) in place of the 2D world layers:
  - terrain draws;
  - props as mesh draws or billboards;
  - **characters**: each sprite item of an `Actors` layer becomes a
    billboard of the same clip frame (1123), upright, at its 2D
    position turned into metres and lifted to the terrain's height
    there, with its shadow;
  - **the camera** from the stage's 2D camera: the same target, follow
    and bounds, seen from the file's tilt, distance and field of view.
    Shake carries over;
  - the lights and effects from the file (`fog` fills the view's fog,
    3103).
- **Picking stays 2D.** `DioramaSkin` implements a `WorldPick` hook
  that the input layer uses for the pointer under this skin: a ray from
  the 3D camera through the pointer, walked over the tile grid in
  integers (a grid walk against block heights), gives the world pixel
  of the tile hit. `TickInput` stores that world position as before
  (1121), so a record replays without the skin. If 1121 has no such
  hook, add it in `shuyi-ui::input` here.
- **Effects and targets.** The skin file gives 3017's `Needs`:
  `three_d: Hardware`, the effects not marked `optional` as required
  effects, and the others in its optional list. So `shuyi check
  --target` fails on each required effect the target can't run, naming
  the skin file, the effect and the target, and an optional one is left
  out on that target and listed in `left_out`.
- **A fixture stage** in the crate's tests: a 12 × 10 grid map made for
  the tests (ours, CC0) with three heights, a slope and a cliff, two
  characters walking a scripted path, a flat 2D skin, and a diorama
  skin file.

**Out (do not do):**
- any change to the 2D kits' rules;
- a 3D stage's rules (that is `shuyi-3d`);
- the examples, templates and guides (3109);
- editing heights in a tool (Tiled or LDtk draw them);
- turning the camera around the scene by input (a later ticket if an
  example wants it).

## Implementation steps

1. Create the crate, its metadata, its feature, its README start and
   catalog row.
2. Write the skin file format, with good and bad fixtures.
3. Build the terrain from the fixture map, and check its snapshot text.
4. Write `build`: props, characters, camera, lights, effects.
5. Write the `WorldPick` hook and the grid walk, with tests.
6. Report needs to `shuyi check --target`, with fixture targets.
7. Write the acceptance test of flat against diorama.

## Acceptance criteria

- [ ] **The rules don't change.** The fixture stage's replay, under the
      flat skin and under the diorama skin, gives the same hash list and
      the same scene text at every step (an integration test).
- [ ] The fixture's 3D view snapshot matches its golden text, and its
      `frame-png` picture (3103's software painter) matches its pixel
      hash.
- [ ] Picking the pointer at the centre of each visible top face gives
      that tile, at every height (unit test over the fixture).
- [ ] A fixture skin with `depth_of_field` required fails
      `shuyi check --target` on a fixture target without it, and with
      `optional: true` passes and lists it.
- [ ] Each bad skin file gives an error naming the file and line.
- [ ] `cargo xtask check-modules` passes, with the row set to building.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the skin file; terrain pieces; picking; needs.
- Integration: flat against diorama on the same replay.
- Golden and fixture: the view snapshot and the picture hash.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
