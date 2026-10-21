---
id: "2328"
title: "Parked: 2D lights: ambient light, point lights from the scene, and normal maps beside sprites, on the wgpu renderer"
type: feature
milestone: "Engine: 2D effects"
model: opus-5.5
effort: high
status: blocked
blocked_by: ["1120", "2229", "2326"]
nick_input: none
completed:
---

# 2328 — 2D lights and normal maps

> **Parked** (Nick, 2026-10-19, chose "Park until MVP" for the in-house platform layer, `docs/decisions.md`): a follow-up of the parked screen effects (2326) and world layers (1120), so it starts after both are unparked and done. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Found by the 2026-10-20 comparison with Bevy 0.20 and Godot 4.6. Godot
has `PointLight2D`, `DirectionalLight2D` and normal maps on 2D sprites
(`CanvasTexture`); Bevy has no 2D lights of its own. A torch in a
cave, a lamp-lit town at night, a sprite that looks round because it is
lit from the side: 2D games use these often, and Shuyi has nothing.

Graphics are a skin (ADR-0002 §4). Logic says where the torch is; the
skin says how much light it gives. Lights are plain data in the frame,
on 1120's world layers. A light never changes the simulation: a game
that needs "can the guard see me" writes that as rules (as 2713 does
for RTS fog).

This runs on the wgpu renderer (3034) with 2326's render targets, and,
like 2326, has a CPU reference for `frame-png`.

## Nick input

None.

## Scope

**In:**
- **The frame:** a world layer (1120) gains `lit: bool`, `ambient: (role,
  level: u8)`, and `lights: Vec<Light>`. A `Light` is a position in
  world pixels, a radius, a colour role, an intensity (0–255), and an
  optional `flicker { seed, amount }`, computed from the seed and the
  frame's time (as particles are, 2103), so a picture of any moment is
  the same. UI and glyph layers are never lit. Snapshot lines list the
  lights.
- **Normal maps:** an image's import settings (2229) may name a normal
  map image beside it, in a `lit` section: `hero.png.meta` holding
  `(lit: (normal: "hero_n.png"))`, a type implementing
  `ImportSettings`, registered in `Formats::register_standard`. It is
  checked to be the same size.
  A sprite without one is lit as if it faced the viewer.
- **The wgpu renderer:** per-pixel light on lit layers: ambient plus
  each light, Lambert with the normal map, falloff to zero at the
  radius. Up to 16 lights per layer in one uniform array; more is an
  error in the check, naming the layer. An optional `bands: u8` steps
  the light into bands for a pixel-art look.
- **`frame-png`:** a CPU reference of the same lighting, in integers.
  2326's GPU-against-CPU test covers lights too.
- **Targets without shaders:** lit layers are drawn unlit. If 3017 is
  done, `lights` is a need a target can lack (optional).
- **The example:** 1119's tick example's room at night, with a torch
  the dot carries.

**Out (do not do):**
- shadows and occluders (a follow-up if a game asks);
- lights that change the simulation;
- 3D lights (3103, 3104);
- emissive maps (a follow-up).

## Implementation steps

1. Read 2326 and the renderer, 1120's world layers, and 2229's import
   settings.
2. Add lights to the frame, with snapshot lines and the checks.
3. Register the `lit` section (2229) and load normal maps through it.
4. Write the CPU reference, with unit tests.
5. Write the WGSL lighting, and extend the GPU-against-CPU test.
6. Add the example.
7. Run the gates.

## Acceptance criteria

- [ ] A light straight above a sprite with a flat normal map lights it
      fully; with the normal facing away, not at all (CPU tests).
- [ ] The GPU output matches the CPU reference within 2326's tolerance.
- [ ] The example's night room is reviewed with `frame-png`, and its
      replay hash is the same with lights on and off.
- [ ] A normal map of the wrong size fails with the file named.
- [ ] A frame with no lit layer draws the same pixels as before.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: falloff; ambient only; bands; flicker from seed and time.
- Property: brightness never rises with distance from a light.
- Snapshot / integration: the night room, CPU and GPU.

## Completion notes

*(Filled in by the session that completes the ticket.)*
