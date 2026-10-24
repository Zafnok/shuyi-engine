---
id: "2326"
title: "Parked: Screen effects as skin data: a chain of passes over the painted frame (CRT, bloom, palette remap, vignette) on the wgpu renderer, with reference pictures in frame-png"
type: feature
milestone: "Engine: 2D effects"
model: opus-5.5
effort: xhigh
status: blocked
blocked_by: ["3034", "2018", "2340"]
nick_input: none
completed:
---

# 2326 — Screen effects as skin data

> **Parked** (Nick, 2026-10-19, chose "Park until MVP" for the in-house platform layer, `docs/decisions.md`): it needs the 2D renderer over wgpu that 3034's research plans, and that is parked. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Found by the 2026-10-20 comparison with Bevy 0.20 and Godot 4.6. Godot
has canvas shaders, `BackBufferCopy` and glow for 2D; Bevy has bloom on
2D cameras and post-processing through its render graph. Shuyi's 3D
plans have effects (3103's `effects` list, the diorama skin's depth of
field and bloom in 3107), but the 2D frame has none.

A text-first engine is a good home for a CRT look, and games use a few
screen effects again and again: bloom, a palette remap (day and night,
a sepia memory), a vignette, a flash when something is hit (2340 draws
a whole-frame flash on every painter, the one-bit one included, through
the frame's veil from 2339).

Graphics are a skin (ADR-0002 §4, `AGENTS.md` rule 9). Logic says
"hurt"; the skin picks the look. Effects are data the skin hands in
with the frame, so tests of what happened never read them, and tests of
a look do.

[ADR-0021](../../docs/adr/0021-the-platform-layer-in-house-clean-room-pieces-in-their-own-crates.md)
§1: graphics go through wgpu, and our own 2D renderer sits on it. 3034
is the research that plans that renderer and writes its tickets. This
ticket runs on that renderer, so when 3034 writes the renderer's
ticket, it adds it to this one's `blocked_by`.

## Nick input

None.

## Scope

**In:**
- **`EffectChain`**, an asset kind read from RON (`effects.ron` in a
  skin, ADR-0007, ADR-0019): passes in order, each with its numbers and
  two flags:
  - `Crt { scanlines: u8, curvature: u8, mask: Aperture | Slot | None }`;
  - `Bloom { threshold: u8, strength: u8, radius: u8 }`;
  - `Remap { to: palette id }`: every colour of the game's palette
    becomes the same role's colour in another palette (day to night);
  - `Vignette { strength: u8 }`;
  - `optional`: a target that can't run it drops it.
    **Either order with 3017:** if 3017 is done, add its `shaders`
    capability and the `screen_passes` need as 3017 describes them for
    this ticket (a pass is required unless marked `optional`); if not,
    nothing to do here (3017 adds them);
  - `flashing`: dropped when the player turns flashing off (below).
- **The frame** carries the chain: `GlyphBuffer` gains `effects:
  Option<EffectsRef { chain id, amounts }>`, where `amounts` lets the
  skin fade a pass in and out (a vignette at 255, then 128, then 0). One
  snapshot line lists it; the format's doc comment says so.
- **The wgpu renderer** runs the chain after painting: one WGSL pass
  each, ping-ponging two render targets. Passes that look at pixels
  (bloom, remap, vignette) run at the frame's logical size;
  `Crt` runs at the window's size, last.
- **Either order with 2331** (the render graph, parked): if 2331 is
  done, each pass of the chain is a node of its `RenderGraph`, which
  `render.ron` turns on or off, while `effects.ron` keeps the passes'
  numbers and the frame's `effects` their amounts; if not, nothing to
  do here (2331 moves the chain's passes into nodes when it lands).
- **The raster** (`shuyi_ui::frame::raster`, 2332) runs a CPU reference
  of each pass, in integers where the pass allows, so `frame-png` shows
  an agent the effect without a GPU, and the window is compared with it.
- **GPU against CPU:** a test renders a test frame through both and
  compares, within the plan's `Tolerance` (the worker picks it on the
  test frames and writes why). It runs through 2018's capture mode, on
  the wgpu backend that 3034's tickets build, with
  `shuyi_platform::capture` plans and `raster::compare` (2332). Use
  2018's `render` CI job, with lavapipe as the software adapter. Each
  effect gets a test frame in a plan.
- **Player settings** (photosensitivity): `Settings::screen_effects`
  (on or off), saved, with an option row. `Settings::flashing` is
  2340's (with its row, its three-flashes-a-second rule and its gentle
  cap); this ticket reads it: a pass marked `flashing` is dropped when
  it is off. With effects off, only `Remap` runs (it can carry meaning,
  such as night).
- **The toy game** gets a CRT chain.

**Out (do not do):**
- a game's own shaders and sprite materials (2327);
- 2D lights (2328);
- 3D effects (3103, 3104, 3107);
- effects on targets without shaders (the Playdate, the GBA, the DS):
  `optional` passes are dropped there, and this ticket adds nothing for
  them;
- a whole-frame flash and shake, and transition looks, on every painter
  (2340, 2339).

## Implementation steps

1. Read 3034's ADR and the wgpu renderer, ADR-0002 §4, 2332's raster,
   2018's ADR and capture mode, 2340's settings, and 3017 if done.
2. Write `EffectChain`, its format and checks, and the frame's field and
   snapshot line.
3. Write the CPU reference of each pass in the raster, with unit tests.
4. Write the WGSL passes and the chain in the renderer.
5. Add a plan with a test frame per effect, run in 2018's `render` job.
6. Add the `screen_effects` setting, its row and the toy game's chain.
7. Run the gates.

## Acceptance criteria

- [ ] Each pass's CPU reference has a committed `frame-png` picture of
      the test frame, reviewed.
- [ ] The GPU output matches the CPU reference within the plan's
      tolerance, in 2018's `render` job on lavapipe.
- [ ] With flashing off, a `flashing` pass never runs (a test).
- [ ] A frame with no effects draws the same pixels as before (the
      existing snapshots don't change).
- [ ] Random bytes never panic the format; a bad chain gives errors with
      file and line.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each CPU pass on small images; amounts at 0 and 255; the
  `screen_effects` setting's saved form.
- Property: a pass at amount 0 leaves the image unchanged.
- Snapshot / integration: each pass's picture; GPU against CPU through
  the capture plans in the `render` job.

## Completion notes

*(Filled in by the session that completes the ticket.)*
