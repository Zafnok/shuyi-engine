---
id: "2327"
title: "Parked: A game's own shaders: WGSL effect passes and sprite materials as assets, with their parameters as data, and a per-sprite palette swap"
type: feature
milestone: "Engine: 2D effects"
model: opus-5.5
effort: high
status: blocked
blocked_by: ["2326"]
nick_input: none
completed:
---

# 2327 — A game's own shaders and sprite materials

> **Parked** (Nick, 2026-10-19, chose "Park until MVP" for the in-house platform layer, `docs/decisions.md`): a follow-up of the parked screen effects (2326), so it starts after 2326 is unparked and done. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Found by the 2026-10-20 comparison with Bevy 0.20 and Godot 4.6. Godot
has its canvas shader language; Bevy has `Material2d` with WGSL. 2326
gives built-in screen effects. Games also want their own: water that
ripples, a sprite that dissolves, an outline on the selected unit, one
sprite flashing white when hit, a team's colours on a shared sprite.

WGSL is wgpu's language ([ADR-0021](../../docs/adr/0021-the-platform-layer-in-house-clean-room-pieces-in-their-own-crates.md)
§1). Shaders are assets like any other (ADR-0007): text files, checked
with file and line, reloaded while the game runs. They change only the
look. Their inputs are plain data in the frame, including the time,
which the frame carries, so a picture of any moment is the same on
every run.

## Nick input

None.

## Scope

**In:**
- **`Shader`**, an asset kind read from `.wgsl` (ADR-0019), validated
  at load with `naga` (it comes with wgpu, MIT OR Apache-2.0; check it
  with `cargo deny`). Errors give the file, line and naga's message.
- **The interface**, fixed by the engine and documented in
  `docs/shaders.md`:
  - a **pass** shader gets the frame so far (texture and sampler),
    the frame's time in ms, and up to 16 `vec4<f32>` parameters;
  - a **material** gets the sprite's texture region, its palette role's
    colour, the frame's time, and up to 4 `vec4<f32>` parameters.
  A shader whose bindings don't match fails at load, naming what's
  wrong.
- **Passes:** 2326's `EffectChain` gains `Custom { shader: id, params:
  {name: [f32; 4]} }`.
- **Materials:** sprite items gain `material: Option<MaterialRef { id,
  params }>`. The snapshot line lists it.
- **A built-in `palette_swap` material:** a sprite drawn in one ramp of
  roles is drawn in another (`from: [roles]`, `to: [roles]`), for team
  colours. It has a CPU reference in `frame-png`.
- **`frame-png`** doesn't run a game's WGSL on the CPU. It draws the
  sprite or the frame without it and writes so in the snapshot
  (`material water (not drawn on cpu)`). With `--gpu`, where an adapter
  exists, it renders through the wgpu renderer.
- **Hot reload:** a changed `.wgsl` file reloads, as other assets do
  (ADR-0004 §1), and a broken one keeps the last good one with the
  error shown.
- **Targets without shaders:** materials are skipped, and a material
  may name a `fallback` sprite. If 3017 is done, a skin that needs a
  material on such a target fails its check.
- **The toy game** gets one pass (a wave) and one material (an outline).

**Out (do not do):**
- compute shaders;
- 3D materials (3102, 3104);
- a shader editor or graph;
- a shading language of our own.

## Implementation steps

1. Read 2326, the renderer, and naga's validation API.
2. Write the `Shader` kind and format, with the interface check.
3. Add `Custom` passes and sprite materials to the frame and renderer.
4. Write `palette_swap` with its CPU reference.
5. Add hot reload and the toy game's shaders.
6. Write `docs/shaders.md`.
7. Run the gates.

## Acceptance criteria

- [ ] A shader with a syntax error fails with its file and line; one
      with wrong bindings fails naming the binding.
- [ ] `palette_swap` on the GPU matches its CPU reference within 2326's
      tolerance.
- [ ] The toy game's wave and outline are reviewed with `frame-png
      --gpu` where an adapter exists, and the CPU snapshot says what was
      not drawn.
- [ ] Random bytes never panic the `.wgsl` format.
- [ ] `cargo deny check` passes.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the interface check; parameter packing; the material snapshot
  line; `palette_swap` on a small sprite.
- Property: random bytes into the format.
- Snapshot / integration: the toy game's frames, CPU and GPU.

## Completion notes

*(Filled in by the session that completes the ticket.)*
