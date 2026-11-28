---
id: "3119"
title: "Parked: Large 3D places: draws culled to the camera, distance levels of detail, one mesh drawn many times, chunks streamed in steps, and draw budgets per target"
type: feature
milestone: "Parked: 3D"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: blocked
blocked_by: ["3103", "3104"]
nick_input: none
completed:
---

# 3119 — Large 3D scenes

> **Parked** (Nick, 2026-10-17: "I want to expand on this and add true 3D support (all kinds, first person, third, top down with 3D models), and HD-2D"): after the engine reaches parity with the game (the MVP in `docs/ROADMAP.md`) and after the 2D work already planned (`docs/decisions.md`, 2026-10-17). Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

3103 culls back faces and draws every mesh it is given. The Bevy
coverage audit of 2026-11-28 (`docs/ROADMAP.md`, *Bevy's game
coverage*) found the 3D games that need more: open worlds and large
places of thousands of meshes (Bevy's `examples/large_scenes/`:
`bevy_city`, `bistro`, `caldera_hotel`), with frustum culling,
visibility ranges, instancing and streaming
(`crates/bevy_camera/src/visibility/`, `range.rs`;
`crates/bevy_camera/src/primitives.rs`; `examples/3d/visibility_range.rs`).
3343 (parked) skips 2D world items outside the camera and streams a
map's levels; nothing does it for 3D.

The view stays plain data: the skin culls and picks levels, the rules
never read the camera, and the snapshot text shows what was drawn.

## Nick input

None.

## Scope

**In:**
- **Bounds** (an Fx12 sphere and box) computed per mesh at load and
  stored with it (and in 3111's pack).
- **Culling:** the view builder drops draws outside the camera's
  frustum by an integer test.
- **Levels of detail:** `lods: [(max_distance, mesh)]` on a draw, picked
  by integer distance.
- **Instancing:** `MeshInstances { mesh, material, transforms }` as one
  draw, painted instanced by 3104 and looped by the raster.
- **Streaming:** a place split into chunks, each an asset group loaded
  in steps (ADR-0097, ADR-0128), loaded and dropped by the camera's
  distance with hysteresis.
- **Budgets:** draws and triangles per frame in the target profile
  (3017), counted in `limits` and `shuyi perf`.

**Out (do not do):**
- occlusion culling and meshlets (until a measured scene needs them);
- 2D culling and level streaming (3343, parked);
- terrain generation.

**Estimated size:** about 1,400 lines (code 700, tests 450, docs 250).

**Split:** past 1,000 lines, split off streaming into a new ticket.

## Implementation steps

1. Read 3103, 3104, 3111, ADR-0097 and ADR-0128.
2. Compute and store bounds; add frustum culling.
3. Add levels of detail and instanced draws in both painters.
4. Add chunk streaming.
5. Add budgets; a bench scenario of a 10,000-mesh place per target.
6. Write the docs page.

## Acceptance criteria

- [ ] A mesh fully outside the frustum is absent from the snapshot text;
  one partly inside is present.
- [ ] A draw at a distance past its first level uses its second mesh.
- [ ] 1,000 instances of one mesh are one draw call on the GPU painter
  (3341's counts).
- [ ] Walking through a streamed place keeps at most the profile's
  chunks loaded.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] Guides in `docs/guides/` updated, if this adds a direction, a module or a move (ADR-0009 §5).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the frustum test, level picking, hysteresis.
- Property: culling never drops a draw whose bounds touch the frustum.
- Snapshot / integration: snapshot text; the bench scenario.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
