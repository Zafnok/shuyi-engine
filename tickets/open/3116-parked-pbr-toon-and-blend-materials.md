---
id: "3116"
title: "Parked: 3D materials beyond base colour: metallic-roughness, normal, occlusion and emissive maps, unlit, toon and PBR shading, blend modes and vertex colours"
type: feature
milestone: "Parked: 3D"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: blocked
blocked_by: ["3102", "3103", "3104"]
nick_input: none
completed:
---

# 3116 — PBR, toon and blend modes in 3D materials

> **Parked** (Nick, 2026-10-17: "I want to expand on this and add true 3D support (all kinds, first person, third, top down with 3D models), and HD-2D"): after the engine reaches parity with the game (the MVP in `docs/ROADMAP.md`) and after the 2D work already planned (`docs/decisions.md`, 2026-10-17). Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

3102's `Material` is a base colour, a texture, emission, alpha,
double-sidedness and a pixel flag; 3104 lights per vertex. That gives
retro and HD-2D looks. The Bevy coverage audit of 2026-11-28
(`docs/ROADMAP.md`, *Bevy's game coverage*) found the 3D games it
leaves out: realistic and stylised-realistic games (third-person
action, walking sims) with physically based materials, and toon-shaded
games outside the DS. Bevy's
`StandardMaterial` covers PBR, normal and parallax maps and blend modes
(`crates/bevy_pbr/src/pbr_material.rs`; `examples/3d/pbr.rs`,
`parallax_mapping.rs`, `blend_modes.rs`, `vertex_colors.rs`). No
ticket mentions them.

## Nick input

None.

## Scope

**In:**
- **Shading models:** 3102's `Material` gains `shading: Unlit |
  Lambert | Toon { bands } | Pbr { metallic: u8, roughness: u8 }`,
  optional normal, metallic-roughness, occlusion and emissive textures
  read from glTF, `blend: Over | Add | Multiply`, and vertex colours on
  `Mesh`. Tangents are generated at load in integers (MikkTSpace
  compatible), so hashes match on every target.
- **References:** 3103's raster gets a per-pixel Fx12 reference of each
  model; 3104's painter a shader per model, compared within a tolerance
  in 2018's render job.
- Target profiles (3017) name which models a target runs; the DS keeps
  its toon path (3025).

**Out (do not do):**
- clearcoat, anisotropy, transmission and other `KHR_materials_*`
  extensions (listed in `limits`);
- parallax mapping;
- a game's own WGSL materials and decals: 3123;
- 2D materials (2327, parked).

**Estimated size:** about 1,300 lines (code 650, tests 450, docs 200).

**Split:** past 1,000 lines, split off the toon model and blend modes
into a new ticket.

## Implementation steps

1. Read 3102, 3103, 3104 and ADR-0011.
2. Extend `Material` and the glTF reader; generate tangents.
3. Write each model's raster reference, then its shader.
4. Add blend modes and vertex colours.
5. Add target-profile limits; write the docs page.

## Acceptance criteria

- [ ] A PBR sphere fixture's raster picture equals its golden, and the
  GPU picture matches within the measured tolerance.
- [ ] Tangents of a fixture mesh equal on native and wasm32.
- [ ] An additive blend over a fixture equals its golden.
- [ ] A target profile without PBR draws Lambert and lists the material
  in `limits`.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] Guides in `docs/guides/` updated, if this adds a direction, a module or a move (ADR-0009 §5).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: material parsing, tangent generation.
- Snapshot / integration: raster goldens per model; render-job
  comparisons.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
