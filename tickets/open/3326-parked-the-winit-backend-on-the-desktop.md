---
id: "3326"
title: "Parked: The winit backend on the desktop, behind shuyi-app's winit feature: the window, the loop and its pacing, the frame painted through shuyi-gpu, the window record and the capture mode"
type: feature
milestone: "Engine: in-house platform"
model: opus-5.5 / gpt-6.1-sol
effort: xhigh
status: blocked
blocked_by: ["3325"]
nick_input: none
completed:
---

# 3326 — The winit backend on the desktop

> **Parked** (Nick, 2026-11-22, `docs/decisions.md`): written by research 3034, whose research alone he unparked. The build waits for the MVP with the rest of the in-house platform layer (2026-10-19), and for 3079 through 3325.

## Context

[ADR-0136](../../docs/adr/0136-replacing-macroquad-winit-wgpu-and-wasm-bindgen.md)
§12 puts a winit backend in `shuyi-app` beside the macroquad one, behind a
feature, as sound's replacement sat behind `in-house-sound` (ADR-0068).
This ticket opens the window and runs the game in it on Windows, Linux
(the Steam Deck included) and macOS. It paints the engine's frame
through 3325's `shuyi-gpu` and keeps 2018's `render` job green on both
backends. The keyboard, the mouse, focus and the cursor are 3327's.

## Nick input

None.

## Scope

**In:**
- **Features of `shuyi-app`.**
  - macroquad becomes an optional dependency behind a default feature,
    `macroquad`.
  - `winit` turns on winit 0.30 (ADR-0136 §4's features), `shuyi-gpu`
    and `pollster`.
  - With both on, winit wins.
  - Both names stay for good (ADR-0068 §8; ADR-0136 §12). The feature
    docs in `crates/app/Cargo.toml` say so.
- **`src/winit/`** (the backend; its `Platform` is `Winit`), with
  `ApplicationHandler`:
  - The window and the `shuyi-gpu` surface are made in `resumed`.
  - `about_to_wait` sets `ControlFlow::WaitUntil` from the pacer and asks
    for a redraw when the next frame is due.
  - `RedrawRequested` runs `App::frame`, then draws.
  - The present mode is `Fifo` when `Window::vsync`, else `Immediate` or
    `Mailbox` if offered.
  - The pacer's rules stay (`crates/app/src/pacing.rs`,
    `docs/platform-backends.md`, *The frame cap*).
  - The voice manifest and the sound's start run with `pollster` between
    frames.
- **`src/paint.rs`** turns a `GlyphBuffer` into a `shuyi_gpu::Frame` in
  `render.rs`'s order:
  - the clear rectangle, then the backdrop under a clip;
  - the fills (`Fills::NotClear` or `All`), the `Under` items, the
    glyphs (wide and bold as `render.rs`), and the `Over` items;
  - `ImageTextures` and `PaintedTextures` over `TextureId`, with
    recoloured copies as today;
  - the loading card and the errors in the backend's own font, as the
    macroquad backend shows them.
- **The entries keep their signatures:** `run_app`, `run_game`,
  `run_game_checked`, `run_dialogue_preview` and the hidden
  `open_window`. Under `winit` they run this backend.
- **The window record** (3044) on winit, unchanged: its sizes are winit's
  physical sizes (`inner_size`), and macOS's points are converted, so a
  record written by either backend opens the same window.
- **The player's Fullscreen:** `Fullscreen::Borderless` on the window's
  monitor, and back.
- **The start, per backend:**
  - Measure each backend's time from the window to the first frame on
    Windows.
  - The research's prototype took 4.4 s to its DX12 device on an NVIDIA
    laptop GPU the first time (4.0 s of it the surface and the adapter),
    and 1.6 s on later runs, against 1.6 to 2 s on Vulkan and 0.3 s on
    GL.
  - Find why: the shader compiler's load, the adapters' enumeration on
    a machine with two GPUs.
  - Fix it, or put Vulkan before DX12 on Windows in `shuyi-gpu`'s order
    and amend ADR-0136 §3 in this PR.
- **No GPU** (ADR-0136 §3): the log file and stderr name each backend
  tried, and the process exits 1. On Windows a `MessageBoxW` says it,
  beside `window/win32.rs`'s calls.
- **The log sinks' seconds** come from the backend's clock
  (`std::time::Instant`) in place of `miniquad::date::now()`.
- **The capture mode** (`capture` feature, ADR-0052) on winit:
  - the plan's exact frames;
  - the window read back through `shuyi-gpu` (§2's second draw into a
    target);
  - the same report and exit codes.
  - The link's picture (`readback::window_png`) reads the same way.
- **CI's `render` job** runs the render card and `hello-grid`'s plan a
  second time, with `--features shuyi-app/winit`:
  - on lavapipe (`mesa-vulkan-drivers`);
  - again with `WGPU_BACKEND=gl` on llvmpipe;
  - each compared with the raster, as the macroquad run is.

**Out (do not do):**
- Keys, text, the mouse, touch, focus, the idle cursor, drops and the
  grab (3327); the window facts macroquad lacks (3328); the IME (3329).
- The web (3330 and 3331).
- Making `winit` a default (3332), or removing macroquad (3333).
- AccessKit (3035, which joins this window).

**Estimated size:** about 1,800 lines (code 1,100, tests 500, docs 200).

**Split:** past 1,000 lines, split off the window record on winit and
the player's Fullscreen into a new ticket, leaving the window at its
configured size.

## Implementation steps

1. Read ADR-0136, ADR-0024, ADR-0052, ADR-0076, ADR-0119,
   `docs/platform-backends.md` (*The loop*, *The frame cap*, *The window
   and the cursor (desktop)*) and `crates/app/src/lib.rs`'s `run`. Read
   winit's 0.30 docs and examples. If 0.31 is stable and Bevy's latest
   release is on it, use 0.31 (ADR-0136 §4) and say so.
2. The features and the optional macroquad, with `cfg`s on every
   macroquad module. `cargo build -p shuyi-app` (macroquad),
   `--features winit`, and `--no-default-features --features winit`
   all build.
3. The handler and the loop, with the pacer.
4. `paint.rs`, and its test against `render.rs`'s order.
5. The window record and the Fullscreen.
6. The capture mode, the link's picture and the `render` job's steps.
7. The docs: `docs/platform-backends.md`'s backends table and *The
   loop* (the winit backend's loop as the second example), and the
   crate's docs.

## Acceptance criteria

- [ ] `cargo run -p hello-grid --features shuyi-app/winit` opens the
      game on Windows (GNU toolchain) on DX12, and with
      `WGPU_BACKEND=vulkan` and `WGPU_BACKEND=gl`. The window,
      the scale, the letterbox and the colours look as on macroquad, by
      `frame-png` beside a screenshot, recorded in the completion notes.
- [ ] The `render` job passes on both backends, the winit one on
      lavapipe and on llvmpipe's GL, within the plan's tolerance.
- [ ] Frames are paced to the frame cap with vsync on and off, and the
      pacer's report reaches the game (an integration test over the
      handler with a fake clock, and a manual run's `SHUYI_PERF` file).
- [ ] A record written by the macroquad backend opens the same window
      size under `winit`, and back.
- [ ] With every backend ruled out (`WGPU_BACKEND` naming one this
      machine lacks), the log names it and the process exits 1.
- [ ] No public signature of `shuyi-app` changes (`cargo xtask api`).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `paint.rs` (the order, the fills' rule with and without a
  backdrop, the backdrop's clip, wide and bold glyphs, the missing
  colour), the record's sizes in both backends' units.
- Integration: the capture mode's tests on the winit backend in the
  `render` job.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
