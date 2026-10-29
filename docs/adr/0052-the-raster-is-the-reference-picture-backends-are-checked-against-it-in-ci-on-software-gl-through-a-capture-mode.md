# ADR-0052: The raster is the reference picture: backends are checked against it in CI on software GL, through a capture mode

- **Status:** Accepted
- **Date:** 2026-10-29
- **Related tickets:** 2018 (this); 2332 (the raster); 2019 (the web
  build's capture); 2392 (the render card); 2566 (one input script);
  2333 (scale modes); 3034, 2326 and 3104 (later painters)
- **Extends:** ADR-0011 §5 (GPU output is compared with the software
  reference only by eye or with a tolerance), ADR-0008 §2 (only
  backends touch devices)

## Context

- `shuyi-app`'s renderer (`crates/app/src/render.rs`) is what players
  see. No test drew with it, and the `coverage` job leaves it out.
- The raster (`shuyi_ui::frame::raster`, ticket 2332) is what
  `frame-png`, the tool contract and MCP show agents. It is a software
  copy of the renderer, and `raster::compare` with a `Tolerance` was
  written to hold the renderer to it.
- A renderer bug, a macroquad upgrade or a driver change could make the
  window differ from the raster while every gate stayed green. Agents
  would then judge a game by a picture players never see (Nick's end
  goal, 2026-10-19: what an agent sees must be what players see).
- Others (2026-10-20 comparison): Bevy runs its examples on Linux under
  `xvfb-run` with Mesa, saves screenshots at set frames and compares them
  with `main`'s through Pixel Eagle (`example-run.yml`); Unreal compares
  screenshots with ground truth within a tolerance.

## Decision

### 1. The raster is the reference picture of a frame

Every painter is compared with it: `shuyi-app` now, and later wgpu
(3034, 2326), the 3D painter (3104) and a console's. The comparison is
`raster::compare` with the plan's `Tolerance`. The exact gates stay on
the snapshot text and the raster's own pictures (ADR-0011 §5 holds for
2D too); `frame-png` stays a tool, never a gate.

### 2. Real backends run in CI on software GL

The desktop runs under Xvfb with Mesa's llvmpipe
(`LIBGL_ALWAYS_SOFTWARE=1`), in CI's `render` job; the web runs in
headless Chromium (2019). No GPU runner: llvmpipe is the same on every
run, and a GPU runner costs money. The job prints `glxinfo -B`, so the
log names the renderer and Mesa's version.

### 3. A capture mode is a cargo feature of a backend

`shuyi-app`'s `capture` feature (`crates/app/src/capture.rs`), never on
in a shipped build. With it, `run` reads `SHUYI_CAPTURE` on native,
the path of a plan; unset, nothing changes. Set:

- **exact frames:** `App::frame_after(&mut platform, capture::FRAME_DT)`
  from the first frame, so the time, the fixed steps and the game's seed
  are the same on every run;
- **only the plan plays:** the backend's `Platform` is wrapped
  (`Capturing`): `poll_input` gives the plan's events, the system events
  are the close button alone, no controller is opened;
- **nothing outside:** storage is `MemoryStorage`, audio is `Silent`, no
  voice manifest is read, `Macroquad::start` doesn't run;
- **a capture:** after `present` draws a frame the plan names, the
  raster paints the same `buffer`, `background` and `look` at the scale
  `console::layout` gives the window, with pictures decoded as the
  renderer decodes its textures. The window is read through the live
  link's one read (`readback::window`), turned upright, cut out at the
  layout's offset and compared;
- **what it writes,** under `SHUYI_CAPTURE_OUT` (`target/capture`):
  `<name>.window.png`, `<name>.raster.png`, `<name>.diff.png` when one
  fails, and `capture.txt`, a line per capture: name, size, scale,
  pixels over the tolerance, the worst difference, the first such
  pixel, and the verdict;
- **exit codes:** 0 when every capture passes, 1 when one fails, 2 when
  the plan can't run (it doesn't parse, the game fails to load, a
  capture falls on a frame that isn't the game's, the window can't be
  read or its files written).

On wasm32 the feature builds and does nothing (2019 does the web).

### 4. A plan is a test input of the backend, not an asset kind

`shuyi_platform::capture::Plan` (the `capture` feature, `no_std`), RON
read by `Plan::parse`: `steps` (`keys`, `pad` and `wait` as
`frame-png`'s, and `capture("name")`, which names the frame before it)
and `tolerance` (`Tolerance::default()` unless it says). ADR-0007 is
about what a game loads; a plan is never loaded by a game. `parse` checks
every key and button name, so a typo is an error before the first frame.
A wait is `capture::wait_frames`: the seconds at 60 a second rounded up,
a product within a thousandth of a whole number counting as that number.
The frames of a key or button press are the link's (`press`, one copy).

### 5. The tolerance

The default is `Tolerance::default()`: 2 in a colour channel (a GPU's
rounding) and no pixel past that. A plan names a looser one only when a
backend measurably needs it, never above 3104's for 3D (2 % of a
capture's pixels over 24 in a channel), and never so loose that a
rectangle drawn one window pixel to the right passes.

Measured (Hello, Grid's plan, three captures of 768 × 768 at scale 2):

| Where | Pixels over | Worst channel difference |
|---|---|---|
| Windows 11, a desktop GPU (the author's) | 0 | 0 |
| CI, llvmpipe under Xvfb | (filled in by the PR's runs) | |

With `Item::Rect` drawn one window pixel to the right, two captures
fail: 232 and 488 pixels over, worst 188 and 189.

### 6. What a new frame item or paint does

A new `Item`, `Paint` or `Layer` variant goes on the render card (2392)
in the PR that adds it, and a game's plans capture the screens that use
it.

## Consequences

- The renderer is tested: a break of `render.rs`, a macroquad upgrade or
  a Mesa change that moves pixels fails `render`, with the diff
  uploaded.
- Agents' pictures (the raster) are held to the players' (the window).
- A backend has one more item on its checklist
  (`docs/platform-backends.md`): a capture mode, the render card and a
  game's plans, in CI where its device can run there.
- llvmpipe is not a player's GPU: a driver bug on real hardware isn't
  caught. The comparison catches the engine's own mistakes, which are
  the ones we can fix.
- `render` costs a Linux runner's build of `hello-grid` per PR.

## Alternatives considered

- **Compare with `main`'s screenshots** (Bevy's Pixel Eagle) — needs a
  hosted service and says only "changed", not "wrong"; the raster says
  what the frame should be.
- **GPU runners** — cost money and differ between runs and vendors.
- **A plan as an asset kind with a format plugin** (ADR-0007) — a game
  never loads it, and it would ship a test input in a game's data.
- **The capture mode always compiled in** — a shipped build would read
  one more environment variable and carry the raster and a PNG encoder.
- **Starting the plan when the game has loaded** (as the link's plays
  wait) — a plan counted from the first frame is the same run every
  time, and a capture on the loading card is a plain error.
