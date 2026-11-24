---
id: "3325"
title: "Parked: shuyi-gpu, the 2D renderer over wgpu: textures, batched quads in one shader, blends, clips, targets and read-back, tested headless"
type: feature
milestone: "Engine: in-house platform"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: blocked
blocked_by: ["3079"]
nick_input: none
completed:
---

# 3325 — shuyi-gpu, the 2D renderer over wgpu

> **Parked** (Nick, 2026-11-22, `docs/decisions.md`): written by research 3034, whose research alone he unparked. The build waits for the MVP with the rest of the in-house platform layer (2026-10-19), and for 3079 (quad-snd removed), so it never closes the way back on sound (ADR-0068).

## Context

[ADR-0136](../../docs/adr/0136-replacing-macroquad-winit-wgpu-and-wasm-bindgen.md)
(3034's research, [report](../../docs/research/3034-replacing-macroquad.md))
replaces macroquad with winit for the window and our own 2D renderer over
wgpu for the graphics ([ADR-0021](../../docs/adr/0021-the-platform-layer-in-house-clean-room-pieces-in-their-own-crates.md) §1).
This is the first piece: the renderer, a crate of its own (ADR-0021 §4)
that no backend uses yet.

The research measured the game's six bench scenarios at this engine's
revision: the busiest frame is about 3,550 quads (about 14,200
vertices), drawing from at most 22 textures with at most 58 texture
switches in the painter's order. So the renderer is a batcher of
textured, coloured quads with one WGSL shader and one pipeline per blend
state (ADR-0136 §1–§2).

## Nick input

None.

## Scope

**In:**
- **`crates/gpu`, `shuyi-gpu`**, depending on no `shuyi-*` crate
  (ADR-0021 §4), MIT OR Apache-2.0, `unsafe_code = "forbid"`. Its line
  in `tools/layers.ron` (`devices: true`: it reaches the GPU), its name
  in `crates/xtask/src/modules.rs`'s `NOT_MODULES` beside
  `shuyi-sound-out` (a backend's part, not a module a game turns on),
  and its `README.md`: what it is for and not for, its dependencies and
  their licenses, and its measured `.wasm` size.
- **wgpu** at the major ADR-0136 §4 pins (30 on 2026-11-22; on the day,
  the newest major Bevy's latest release uses, if newer), with
  `default-features = false` and only the features §4 lists. Check its
  license and every new crate's with `cargo deny check` before using it.
- **The public API names no wgpu type** (ADR-0119): plain types of its
  own. The engine's own crates reach wgpu through `#[doc(hidden)] pub mod
  raw`, whose doc says semver doesn't cover it (ADR-0136 §2):
  - `Gpu`: the device and its queue. `Gpu::headless(&Wants)` (tests and
    tools) and `raw::for_surface(target, size, &Wants)` (a backend's
    window), both `async`, trying the backends in ADR-0136 §3's order and
    returning `NoGpu` (each backend tried, and why it failed) when none
    starts. `Wants` asks for optional features (timestamps, ray query,
    compute) that are taken only if the adapter has them;
    `Gpu::has(Feature)` says what was taken. `Gpu::info()` is the
    adapter's name, backend and driver as strings.
  - `TextureId`: `Gpu::texture(width, height, rgba, Sampling)`,
    `update(id, rect, rgba)` and `drop_texture(id)`. RGBA, 8 bits,
    `Rgba8Unorm` (never sRGB), nearest sampling by default. A texture
    over the device's `max_texture_dimension_2d` is refused with an
    error, never cut.
  - `Frame`, a list of `Pass`es, each into the screen or a `TargetId`
    (`Gpu::target(width, height)`, sampled by a later pass as a
    texture), with an optional clear colour and its quads.
  - `Quad`: destination rectangle in the target's pixels (`f32`), source
    rectangle in the texture's texels, texture, colour (RGBA, multiplied
    into the texel), `flip_x`, `Blend` (`Over`, `Add`, `Multiply`) and
    an optional clip rectangle in whole pixels.
  - `Gpu::draw(&Frame)` draws a frame and presents it;
    `Gpu::read(target) -> Rgba` reads a target back, top row first.
- **The batcher:** the frame's quads in order, into one vertex buffer
  and one `u32` index buffer per frame (grown by doubling, never
  shrunk), four vertices and six indices a quad, 20 bytes a vertex
  (position, texel coordinate, colour). Consecutive quads with the same
  texture, blend and clip are one draw call. Pure code over plain data,
  tested without a GPU.
- **One shader**, `src/quad.wgsl`, from the WGSL specification: the
  texel times the vertex colour. Pipelines: one per blend state and
  target format, made at start. Over is `SrcAlpha`, `OneMinusSrcAlpha`;
  Add is `SrcAlpha`, `One`; Multiply is `Dst`, `Zero` over a colour the
  shader mixes toward white by the alpha. The screen's format is a
  non-sRGB one the surface offers (`Bgra8Unorm` or `Rgba8Unorm`), so
  blending is in the same values as the raster's integer rules
  (ADR-0136 §10). Where a surface offers only sRGB formats and no
  non-sRGB view format (WebGL 2 offered the research's prototype only
  `Rgba8UnormSrgb`), the frame is drawn into a non-sRGB target and a
  last pass copies it to the screen with the encoding undone in the
  shader. A pixel test draws that last pass into an `Rgba8UnormSrgb`
  target and reads it back: its bytes are the frame's.
- **Read-back** for the capture mode and the link's picture: the frame is
  drawn again into a target of the window's size and read (no
  `COPY_SRC` on a surface), rows aligned to wgpu's 256 bytes.
- **CI:** the pixel tests run in the `render` job on lavapipe
  (`mesa-vulkan-drivers`, as wgpu's own CI) and again with
  `WGPU_BACKEND=gl` on llvmpipe; the crate builds for
  `wasm32-unknown-unknown` in the job that builds the examples for the web.

**Out (do not do):**
- Using it from `shuyi-app`, a window or the engine's frame (3326 paints
  through it).
- A game's own shaders, materials and screen passes (2326, 2327), lights
  (2328), the render graph (2331), GPU frame times (2382) and the 3D
  painter (3104): ADR-0136 §6–§9 leaves them room, and they build it.
- Recolouring in the shader: the backend keeps today's recoloured copies
  (ADR-0136 §2).
- Reading wgpu's source to copy it; its documentation and examples are
  fair to read (ADR-0021 §2). Never miniquad's.

**Estimated size:** about 1,700 lines (code 1,050, tests 500, docs 150).

**Split:** past 1,000 lines, split off the Add and Multiply blends and
the targets a later pass samples (steps 5 and 6, but the screen-sized
target the read-back uses) into a new ticket.

## Implementation steps

1. Read ADR-0136, ADR-0021 §2–§6, ADR-0119 and the research's report.
   Check wgpu's latest major and its changelog against ADR-0136 §4; if
   the names it uses changed (wgpu renames between majors), follow the
   new names and say so in the completion notes.
2. Add `crates/gpu` with the workspace's lints (`unsafe_code = "forbid"`),
   its `layers.ron` line, and wgpu in the root's `[workspace.dependencies]`
   with the features ADR-0136 §4 lists.
3. `Gpu`, `Wants`, `NoGpu` and the backend order: a pure function from
   the platform to the ordered list of backends (ADR-0136 §3), then one
   instance per entry, `request_adapter` with the surface (when there is
   one) and `request_device` with `Limits::downlevel_webgl2_defaults()`
   raised to the adapter's limits, the next entry on any failure, the
   fallback adapter last. `WGPU_BACKEND` (wgpu's own variable,
   `Backends::from_env`) narrows the list when set.
4. `TextureId` and its calls; the batcher (`batch.rs`) from `Frame` to
   the buffers and draw ranges.
5. `quad.wgsl` and the pipelines; draw a `Frame`'s passes in one command
   buffer, the clip as a scissor rectangle.
6. Targets, and `read`.
7. Tests (below), the `render` job's steps, the wasm build, the README.

## Acceptance criteria

- [ ] `shuyi-gpu` depends on no `shuyi-*` crate and has its line in
      `tools/layers.ron`; `cargo xtask check-layers` and `cargo deny
      check` pass.
- [ ] Its public API names no outside type (`cargo xtask api`); wgpu is
      reached only through `raw`, hidden.
- [ ] A frame of 3,600 quads in the order of a game's frame (cell fills,
      glyphs, sprites from 22 textures) is one vertex and one index
      buffer and as many draw calls as texture switches, by a test.
- [ ] On lavapipe and on llvmpipe's GL, in the `render` job: nearest
      sampling, the colour multiply, `flip_x`, a clip, each blend, a
      target drawn into and sampled, and read-back give the expected
      pixels within one step per channel.
- [ ] `NoGpu` names each backend tried: a test over the pure order
      function, and one where every backend is ruled out.
- [ ] The crate builds for `wasm32-unknown-unknown`, and its README gives
      its measured `.wasm` size.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the backend order per platform; the batcher (merging, the
  break on a texture, a blend or a clip, the vertices of a flipped
  quad, the buffers' growth).
- Pixel: the cases above, on a software adapter, in the `render` job.
  Outside it, a test that finds no adapter says so and passes; the job
  sets `SHUYI_GPU_TESTS=required`, under which finding none fails.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
