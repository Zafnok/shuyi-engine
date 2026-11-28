---
id: "3118"
title: "Parked: Baked and image-based light in 3D: environment maps and irradiance probes computed by shuyi bake, and lightmaps on a second UV set"
type: feature
milestone: "Parked: 3D"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: blocked
blocked_by: ["3102", "3103", "3104", "3111"]
nick_input: none
completed:
---

# 3118 — Baked and image-based light

> **Parked** (Nick, 2026-10-17: "I want to expand on this and add true 3D support (all kinds, first person, third, top down with 3D models), and HD-2D"): after the engine reaches parity with the game (the MVP in `docs/ROADMAP.md`) and after the 2D work already planned (`docs/decisions.md`, 2026-10-17). Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

3104's dynamic lights are few, and the web and low-end targets can't
afford many. Games there get good light by baking it. No ticket
mentions environment maps, irradiance, probes or lightmaps, and 3102's
**Out** leaves "more than one UV set", which lightmaps need. The Bevy
coverage audit of 2026-11-28 (`docs/ROADMAP.md`, *Bevy's game
coverage*) found them in Bevy (`crates/bevy_pbr/src/environment_map`,
`light_probe`, `lightmap`; `examples/3d/reflection_probes.rs`,
`irradiance_volumes.rs`, `lightmaps.rs`, `mixed_lighting.rs`).

Here the expensive part is bake-time and deterministic: `shuyi bake`
computes it in integers, and the run time only samples it.

## Nick input

None.

## Scope

**In:**
- **Environment maps:** a prefiltered cube computed at bake from a cube
  image or a 3115 scene, stored as an asset kind; the view names one.
- **Irradiance probes:** spherical-harmonic probes on a grid computed at
  bake from a scene in integers, stored as an asset kind; mesh draws
  sample the nearest probes.
- **Lightmaps:** glTF's second UV set (`TEXCOORD_1`) read by 3102's
  loader, and a lightmap image per mesh draw.
- Both painters sample them; the raster is the reference.
- The bake is tested equal between runs and targets (ADR-0007).

**Out (do not do):**
- computing lightmaps in the engine (they come from Blender or another
  tool);
- real-time global illumination (2911's research);
- blending between probe volumes.

**Estimated size:** about 1,300 lines (code 650, tests 450, docs 200).

**Split:** past 1,000 lines, split off lightmaps into a new ticket.

## Implementation steps

1. Read 3102, 3103, 3104, 3111 and 2204's bake.
2. Write the environment and probe bakes with golden outputs.
3. Read the second UV set and lightmap images.
4. Sample all three in both painters.
5. Write the docs page.

## Acceptance criteria

- [ ] A probe grid baked twice, and on wasm32, gives the same bytes.
- [ ] A lightmapped floor's raster picture equals its golden.
- [ ] A mesh without a second UV set and a lightmap named fails to load
  with its file.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] Guides in `docs/guides/` updated, if this adds a direction, a module or a move (ADR-0009 §5).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the harmonic projection, UV set reading.
- Snapshot / integration: bake goldens; raster goldens.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
