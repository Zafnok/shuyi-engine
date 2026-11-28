---
id: "3328"
title: "Parked: The window facts miniquad lacks, on the winit backend: the largest whole scale that fits the work area, a saved place on a connected monitor, a minimum size, maximized, a chosen monitor, exclusive fullscreen, cursor images, and leaving fullscreen on Linux"
type: feature
milestone: "Engine: in-house platform"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: blocked
blocked_by: ["3326"]
nick_input: none
completed:
---

# 3328 — The window facts miniquad lacks

> **Parked** (Nick, 2026-11-22, `docs/decisions.md`): written by research 3034, whose research alone he unparked. The build waits for the MVP with the rest of the in-house platform layer (2026-10-19), and for 3079 through 3326.

## Context

3044 keeps the window's size and mode across launches, but miniquad
0.4.11 can't do the rest of what a desktop game's window is expected to
do, so 3044 left it out:

- open at the largest whole multiple of the console that fits;
- put a saved place back only on a monitor still connected;
- a minimum size;
- the maximized state;
- choosing a monitor;
- exclusive fullscreen;
- the game's own cursor image;
- leaving fullscreen on Linux (miniquad's X11 code can't go back to
  windowed).

[ADR-0136](../../docs/adr/0136-replacing-macroquad-winit-wgpu-and-wasm-bindgen.md)
§12 does them on the winit backend (3326). winit gives no monitor's work
area on any system (its issue 2494), so that one fact comes from each
system's own call.

## Nick input

None.

## Scope

**In:**
- **The work area:**
  - on Windows, `MonitorFromWindow` and `GetMonitorInfoW`'s `rcWork`, in
    `window/win32.rs` beside its other calls (`winapi`'s `winuser`,
    there already);
  - on macOS, `NSScreen`'s `visibleFrame` through `objc2-app-kit`;
  - on Linux, the monitor's size less 48 pixels top and bottom, a
    margin for panels, since X11 and Wayland give no portable answer.

  Check each new crate's license (ADR-0021 §3, `cargo deny`). An
  `unsafe` call goes in the platform module with a `// SAFETY:` comment,
  as `win32.rs`'s do.
- **The first opening** (no record): the largest whole scale of the
  console, with the window's margin, that fits the work area of the
  monitor the system opens it on. `Window::console`'s scale becomes the
  most it opens at, never more than fits.
- **The record, version 2** (`window.rs`):
  - It gains the outer position, the monitor's name and position, and
    whether the window was maximized.
  - A version-1 record still reads (its size and mode), and is written
    back as version 2.
  - A saved place is put back only when a connected monitor has that
    name and position, and the window would be at least half on it.
    Otherwise the window is centred on the primary monitor, or the first
    one on Wayland, which has no primary.
- **A minimum size:** the console at scale 1 plus its margin
  (`with_min_inner_size`).
- **Maximized:** opened maximized if it was left so (`with_maximized`).
- **A monitor chosen:** the game's `Window` may name the monitor to open
  on (`Monitor::Primary`, `Monitor::Named(..)`), falling back to the
  system's choice. It is a new public field: additive, on a
  `#[non_exhaustive]` type (ADR-0039).
- **Exclusive fullscreen:**
  - a new `Fullscreen` setting the player may pick on a desktop
    (`Borderless`, the default, or `Exclusive`), applied with
    `Fullscreen::Exclusive` at the monitor's current video mode;
  - borderless on Wayland and the web, which have none;
  - a `Behaviour:` line, and the options screen's entry where the engine
    shows the setting.
- **The game's cursor image:** an image of the game's files with its hot
  spot, made with `CustomCursor::from_rgba`, on the desktop.
  **Either order with 3330:** if 3330 is done, set it on the web too,
  with the same call; if not, nothing to do (3330 sets it there).
- **Leaving fullscreen on Linux:** `set_fullscreen(None)` on X11 and
  Wayland restores the last windowed size. Check on both.

**Out (do not do):**
- The macroquad backend: it keeps 3044's behaviour.
- Several windows, and moving the window from the game: 3355 (parked).

**Estimated size:** about 1,300 lines (code 750, tests 400, docs 150).

**Split:** past 1,000 lines, split off exclusive fullscreen and the
cursor image into a new ticket.

## Implementation steps

1. Read ADR-0136 §12, 3044's `window.rs` and `docs/platform-backends.md`'s
   *The window and the cursor (desktop)*, and winit's `monitor` and
   `window` docs.
2. The work area per system, behind one function `work_area(monitor)`.
3. The first opening, as a pure function of the work area, the console
   and the margin.
4. The record's version 2, and the placement on a connected monitor, as
   pure functions with their tests.
5. The minimum size, maximized, the chosen monitor, exclusive
   fullscreen, the cursor image.
6. The docs: *The window and the cursor (desktop)*, and the book's
   chapter on the window if one says how big it opens.

## Acceptance criteria

- [ ] An 80 × 40 console of 8 × 16 cells (640 × 640 pixels) with a
      16-pixel margin opens at scale 1 on a 1920 × 1032 work area (scale
      2 needs 1,312 pixels of height), and at scale 3 on a 2560 × 2112
      one. The pure function's cases cover each scale up to the largest
      that fits, and a work area where none fits, which opens at 1.
- [ ] A record whose monitor isn't connected opens centred on the
      primary monitor; one whose monitor is connected opens at its
      place (tests over the pure function, and a manual run with a
      second monitor unplugged, in the notes).
- [ ] A version-1 record opens at its size and mode.
- [ ] The window can't be made smaller than the console at scale 1.
- [ ] Exclusive fullscreen changes the video mode on Windows, and falls
      back to borderless on Wayland (manual, in the notes).
- [ ] Leaving fullscreen on X11 returns to the windowed size (manual).
- [ ] A game's cursor image shows over its window on Windows, Linux and
      macOS (manual, in the notes).
- [ ] The `render` job stays green on both backends: the capture mode
      keeps opening at its `Window`'s size, never the work area's, and
      reads no record (a test).
- [ ] The book's chapter is updated (2802), or the completion notes have
      a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the first opening's scale; version 1 and 2 records; the
  placement rule with each monitor case; the minimum size.
- Manual: as above, on Windows, Linux (X11 and Wayland) and macOS.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
