---
id: "2327"
title: "Parked: A game's own shaders: WGSL effect passes and sprite materials as assets, with their parameters as data, and a per-sprite palette swap"
type: feature
milestone: "Engine: 2D effects"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: blocked
blocked_by: ["2220", "2326", "2337"]
nick_input: none
completed:
---

# 2327 — A game's own shaders and sprite materials

> **Parked** (Nick, 2026-10-19, chose "Park until MVP" for the in-house platform layer, `docs/decisions.md`): a follow-up of the parked screen effects (2326), so it starts after 2326 is unparked and done. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Found by the 2026-10-20 comparison with Bevy 0.20 and Godot 4.6. Godot
has its canvas shader language; Bevy has `Material2d` with WGSL. 2326
gives built-in screen effects. Games also want their own: water that
ripples, a sprite that dissolves, an outline on the selected unit, a
team's colours on a shared sprite. A sprite flashing in one colour is
`Paint::Solid` today, and a tint, an additive glow or a multiply shadow
is `Paint::Tinted` or `Blend` (2337) on every renderer, so a material is
for what those can't do. Palette swap stays the team-colour tool,
because a multiply tint over full-colour art looks muddy.

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
    colour, its tint (`Paint::Tinted`'s colour, white without one), the
    frame's time, and up to 4 `vec4<f32>` parameters.
  A shader whose bindings don't match fails at load, naming what's
  wrong.
- **Passes:** 2326's `EffectChain` gains `Custom { shader: id, params:
  {name: [f32; 4]} }`.
- **Materials:** sprite items gain `material: Option<MaterialRef { id,
  params }>`. The snapshot line lists it. A sprite with a material keeps
  its `blend` (2337) as the pipeline's blend state, and its
  `Paint::Tinted` colour is an input of the material, not applied after
  it.
- **A built-in `palette_swap` material:** a sprite drawn in one ramp of
  roles is drawn in another (`from: [roles]`, `to: [roles]`), for team
  colours. It has a CPU reference in `frame-png`.
- **`frame-png`** doesn't run a game's WGSL on the CPU. It draws the
  sprite or the frame without it and writes so in the snapshot
  (`material water (not drawn on cpu)`). With `--gpu`, where an adapter
  exists, it renders through the wgpu renderer.
- **Hot reload:** a changed `.wgsl` file reloads through 2220's `Hot`
  values, as other assets do (ADR-0004 §1), and the renderer rebuilds
  the pipeline of each pass or material that uses it. A broken one
  keeps the last good one, with the error shown as 2220 shows it.
- **Targets without shaders:** materials are skipped, and a material
  may name a `fallback` sprite. **Either order with 3017:** if 3017 is
  done, give its `Capabilities` a `shaders: bool` if it has none yet
  (false on `playdate`, `gba`, `nds` and `dsi`), and a skin's `Needs` a
  `materials` need, so a skin that needs a material on such a target
  fails its check and an optional one goes in `left_out`; if not,
  nothing to do (3017 adds them).
- **The toy game** gets one pass (a wave) and one material (an outline).
- **Either order with 2331** (the render graph): if 2331 is done, a
  `Custom` pass runs as a node of its `RenderGraph`, as 2326's passes
  do, and the frame's node draws sprites with their materials; if not,
  nothing to do (2331 moves them into nodes).

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
