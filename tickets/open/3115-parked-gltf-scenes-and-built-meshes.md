---
id: "3115"
title: "Parked: glTF scenes as 3D levels and meshes a game builds: the node tree flattened with extras as placements, shape primitives and a mesh builder in Fx12, and 3D lines"
type: feature
milestone: "Parked: 3D"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: blocked
blocked_by: ["3102", "3103"]
nick_input: none
completed:
---

# 3115 — glTF scenes and meshes a game builds

> **Parked** (Nick, 2026-10-17: "I want to expand on this and add true 3D support (all kinds, first person, third, top down with 3D models), and HD-2D"): after the engine reaches parity with the game (the MVP in `docs/ROADMAP.md`) and after the 2D work already planned (`docs/decisions.md`, 2026-10-17). Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

3102 loads meshes, materials, skeletons and clips from glTF, and its
**Out** leaves "glTF's node tree and scenes (a level with many placed
meshes)", saying to write a ticket if 3108 needs one. 2753 and 3107
build meshes privately, and no ticket gives a game a mesh builder or
shape meshes. The Bevy coverage audit of 2026-11-28 (`docs/ROADMAP.md`,
*Bevy's game coverage*) found the 3D games these leave out: levels made
in Blender with spawn points and doors, voxel and procedurally built
worlds, and 3D board games built from primitives. Bevy loads glTF
scenes with node extras (`crates/bevy_gltf/src/assets.rs`;
`examples/gltf/load_gltf_extras.rs`), builds shape meshes
(`crates/bevy_mesh/src/primitives/dim3/`; `examples/3d/3d_shapes.rs`,
`generate_custom_mesh.rs`) and draws lines (`examples/3d/lines.rs`).

## Nick input

None.

## Scope

**In:**
- **A scene kind,** `level.glb#scene/Main`: nodes flattened in file
  order into `{ name, world FxTransform, mesh, material override,
  extras }`, floats to Fx12 by bits as 3102 does. Extras parse into a
  small typed map the game reads as its own placements (spawn points,
  triggers, doors), as 1124 reads map objects. A text dump and hash; a
  fuzz target and a golden row (ADR-0007).
- **Static collision** from named nodes into 3105's static meshes.
  **Either order with 3105:** if 3105 is done, build them; if not,
  nothing to do (3105 reads collision nodes from this kind).
- **Shapes:** `shuyi_3d::mesh::shapes` (cuboid, sphere, cylinder,
  capsule, cone, torus, plane) building 3102's `Mesh` in Fx12 with
  normals and texels, equal on every target.
- **A mesh builder:** push vertices, triangles and quads with per-vertex
  colours, for voxel chunks and procedural levels; 2753 and 3107 use it.
- **Lines:** `Line3 { points, colour, width }` items in the 3D view,
  painted by both painters.
- A game rebuilds a mesh from its state in the view layer, never in the
  rules.

**Out (do not do):**
- glTF instancing extensions (3119);
- scenes that inherit other scenes (2294 does it for ECS scenes);
- editing a level in the engine.

**Estimated size:** about 1,500 lines (code 700, tests 550, docs 250).

**Split:** past 1,000 lines, split off shapes, the builder and lines
into a new ticket.

## Implementation steps

1. Read 3102's loader, 3103's view, 3105's static meshes and 1124.
2. Add the scene kind, its flattening and extras.
3. Add collision nodes as the clause says.
4. Write the shapes and the builder; then `Line3` in both painters.
5. Write the docs page: a level with doors from extras.

## Acceptance criteria

- [ ] A fixture `.glb` with three placed meshes and a spawn point loads
  to its golden dump, equal on wasm32.
- [ ] A sphere of radius 1 has every vertex at distance 1 within one
  Fx12 unit.
- [ ] A builder-made cube equals the `cuboid` shape's mesh.
- [ ] A `Line3` appears in the raster's frame-png golden.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] Guides in `docs/guides/` updated, if this adds a direction, a module or a move (ADR-0009 §5).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: flattening, extras, each shape.
- Property: fuzzing the scene loader.
- Snapshot / integration: golden rows; frame-png.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
