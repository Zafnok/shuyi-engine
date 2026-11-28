---
id: "3123"
title: "Parked: A game's own 3D materials and decals: WGSL material functions over the standard lit inputs with a fallback, and images projected onto meshes"
type: feature
milestone: "Parked: 3D"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: blocked
blocked_by: ["3116", "2327"]
nick_input: none
completed:
---

# 3123 — A game's own 3D materials, and decals

> **Parked** (Nick, 2026-10-17: "I want to expand on this and add true 3D support (all kinds, first person, third, top down with 3D models), and HD-2D"): after the engine reaches parity with the game (the MVP in `docs/ROADMAP.md`) and after the 2D work already planned (`docs/decisions.md`, 2026-10-17). Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

3116 gives 3D materials fixed shading models. 2327 (parked) lets a game
write WGSL effect passes and 2D sprite materials. The Bevy coverage
audit of 2026-11-28 (`docs/ROADMAP.md`, *Bevy's game coverage*) found
two 3D surface looks no ticket mentions: a game's own material shader
(animated water, a stylised look; Bevy's
`crates/bevy_pbr/src/extended_material.rs`,
`examples/shader/extended_material.rs`, `examples/3d/animated_material.rs`),
and decals that project an image onto meshes for marks, footprints and
signs (`crates/bevy_pbr/src/decal`; `examples/3d/decal.rs`,
`clustered_decals.rs`).

## Nick input

None.

## Scope

**In:**
- **Custom materials:** `Custom { shader, params, fallback }`, a WGSL
  fragment function over 3116's standard lit inputs, with parameters as
  data, following 2327's asset and parameter design and its hot reload.
  Only the GPU painter runs it; the raster draws the fallback material
  and lists it in `limits`; target profiles (3017) say which targets run
  it.
- **Decals:** `Decal { image, box: FxTransform, order }` in the 3D view
  as plain data, projected onto depth in integers by the raster as the
  reference and by a box pass on the GPU, capped per target and counted
  in `limits`. Rules keep marks as state; the view builds decals.

**Out (do not do):**
- vertex shaders and compute;
- clustered decals with normal and roughness maps.

**Estimated size:** about 1,000 lines (code 500, tests 300, docs 200).

## Implementation steps

1. Read 3116, 2327, 3104 and 3017.
2. Add the custom material kind and its fallback path.
3. Add decals in both painters.
4. Write the docs page: water and footprints.

## Acceptance criteria

- [ ] A custom material on a target without the GPU painter draws its
  fallback and appears in `limits`.
- [ ] A decal on a floor appears in both painters' pictures, matching
  within the measured tolerance.
- [ ] An edited shader reloads in a running debug build.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] Guides in `docs/guides/` updated, if this adds a direction, a module or a move (ADR-0009 §5).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: parameter parsing, the decal box.
- Snapshot / integration: raster goldens; render-job comparisons.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
