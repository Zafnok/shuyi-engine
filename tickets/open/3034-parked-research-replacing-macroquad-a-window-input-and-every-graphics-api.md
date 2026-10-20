---
id: "3034"
title: "Parked: Research: replacing macroquad with our own window, input and graphics on Vulkan, Metal, Direct3D 12, OpenGL and GLES, WebGL 2 and WebGPU, and the tickets to build it"
type: research
milestone: "Engine: in-house platform"
model: opus-5.5
effort: xhigh
status: blocked
blocked_by: ["3003", "3032", "2403", "3033", "2215"]
nick_input: none
completed:
---

# 3034 — Research: replacing macroquad

> **Parked** (Nick, 2026-10-19, chose "Park until MVP" for the in-house platform layer, `docs/decisions.md`): after the engine reaches parity with the game (the MVP in `docs/ROADMAP.md`), and after sound, controllers, web saves and file embedding are in-house (Nick's order). Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Nick, 2026-10-19 (`docs/decisions.md`): "eventually - macroquad replace",
and "we should have a wide compat with all GFX libs". Once sound,
controllers, web saves and file embedding are ours (3032, 2403, 3033,
2215), macroquad does four jobs in `shuyi-app`:

- the window, the run loop and the system's events;
- keyboard, mouse and text input (`crates/app/src/keys.rs`);
- drawing the frame (`crates/app/src/render.rs`, which paints the
  engine's plain-data frame);
- the web loader (`crates/cli/web/mq_js_bundle.js`, vendored), which the
  web build uses instead of `wasm-bindgen` (game ADR-0034).

miniquad, under macroquad, draws with OpenGL 2 and 3, GLES 2 and 3, WebGL 1
and Metal, and its README rules out Vulkan and DirectX 12 (checked
2026-10-19). Nick wants every graphics API.

[ADR-0021](../../docs/adr/0021-the-platform-layer-in-house-clean-room-pieces-in-their-own-crates.md)
sets the rules: written from specifications and the systems'
documentation (§1), raw bindings only (§2: `windows` for Win32 and
Direct3D 12, `objc2-*` for AppKit and Metal, `ash` for Vulkan, `glow` for
GL), crates of their own (§3), `unsafe` only in `sys` modules (§5). 3003
(the platform boundary) must be done first: the replacement implements
`Platform`.

This is research: it writes the plan, an ADR and the tickets. It builds
nothing that ships.

## Nick input

None. If the research finds a choice that changes what the game gets or
when (dropping a platform for a while, say), it asks Nick with the
`ask-nick` skill.

## Scope

**In:**
- **Measure what the engine draws.** The frame is a list of glyph cells,
  sprites, rectangles and text (`shuyi-ui`'s frame, painted by
  `render.rs`). Count draw calls, textures and vertices per frame in the
  game's busiest screen, at 60 fps. This sets how simple the renderer can
  be: a 2D batcher with one pipeline may be all it needs.
- **The graphics layer's shape.** Decide, and record in an ADR:
  - one small renderer interface for the engine's 2D needs (textures,
    one or two pipelines, batches, render targets for the skins), with a
    backend per API: Vulkan, Metal, Direct3D 12, OpenGL 3.3 and GLES 3,
    WebGL 2, and WebGPU in the browser;
  - which backend each platform uses by default, and the fallback (for
    example Vulkan, then GL, on Linux and the Steam Deck);
  - shaders: one source language and how each backend gets its form (our
    own small translator, hand-written per backend, or a
    permissively licensed offline compiler run at build time as a
    black-box tool). Check licenses on the day;
  - the 3D plans (ADR-0011, the GPU painter in 3104) and whether the
    interface must leave room for them now.
- **The window and input.** Win32, AppKit, and X11 and Wayland on Linux
  (both, or Wayland with XWayland covering X11), each from the system's
  documentation. Text input and IME (3003's `TextInput`), high-DPI,
  full screen, the Steam Deck's screen.
- **The web without `wasm-bindgen`.** Our own JS loader to replace
  `mq_js_bundle.js`: the canvas, events, WebGL 2 and WebGPU calls from
  wasm, and how the plugins we already have (`gamepad.js`, 3031's
  `sound.js`, 3033's `storage.js`) register with it.
- **Android and iOS later** (ADR-0008): what the design must not rule out.
- **The order of the build**, so the game keeps working at every step:
  for example, a new backend crate beside `shuyi-app` behind a feature,
  one platform at a time, then the switch, as sound and controllers did.
- **Outputs:**
  - a report, `docs/research/3034-replacing-macroquad.md`, with the
    measurements and the options weighed;
  - an ADR (the `write-adr` skill) with the decisions above;
  - `Parked:` tickets, each one PR, for every piece, with `blocked_by`
    in order, and their rows in `docs/ROADMAP.md`'s *Parked* section.

**Out (do not do):**
- Building any of it.
- Reading the source of miniquad, macroquad, winit, wgpu, SDL, sokol or
  any other window or graphics library (ADR-0021 §1). Their public
  documentation and feature lists may be read, to know what users expect.

## Implementation steps

1. Measure the frame (scope, first bullet) in the game at this engine's
   revision, and write the numbers in the report.
2. Read the specifications and documentation for each API and system, and
   list them in the report.
3. Prototype only what the decision needs, in a throwaway branch or
   scratch folder: for example a triangle on each graphics API from a
   single shader source, to check the shader plan. Delete it after, and say
   what was learned.
4. Write the ADR, then the tickets, then the roadmap rows.

## Acceptance criteria

- [ ] The report has the frame measurements and the list of documents
      used.
- [ ] The ADR decides every bullet in the *graphics layer* and *window*
      parts of the scope.
- [ ] Every ticket written is one PR's worth, `Parked:`, with its
      `blocked_by` chain, and passes the ticket lint.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- None shipped (research). The tickets it writes say their tests.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
