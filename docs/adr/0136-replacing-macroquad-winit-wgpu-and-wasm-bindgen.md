# ADR-0136: Replacing macroquad: shuyi-gpu, our 2D batcher over wgpu; winit in shuyi-app behind a feature; the web build on wasm-bindgen

- **Status:** Accepted
- **Date:** 2026-11-22
- **Related tickets:** 3034 (this research); 3325–3333 (the build); 3079, 3104, 3106, 2326–2328, 2331, 2337, 2346, 2382, 2908, 2911, 3035, 3043, 3052, 3071, 1149, 2008
- **Extends:** ADR-0021 (names the renderer's crate, §2; finds no `unsafe` needed in it, §2)
- **Amends:** ADR-0068 (3034's research runs before 3079; the tickets it writes wait for 3079)

## Context

Nick decided on 2026-10-19 that macroquad leaves the engine, with the
graphics through wgpu, our own 2D renderer on top and the window through
winit (`docs/decisions.md`, ADR-0021 §1). On 2026-11-22 he unparked the
research alone. The tickets it writes stay parked, and every one waits
for 3079, which removes quad-snd: the sound path on the web runs on
macroquad's loader, so replacing the loader first would end the way back
to quad-snd (ADR-0068).

After sound, controllers, web saves and file embedding, macroquad does
four jobs in `shuyi-app`: the window, the run loop and the system's
events; the keyboard, the mouse and text; drawing the frame
(`crates/app/src/render.rs`); and the web loader (`mq_js_bundle.js`).

The research ([report](../research/3034-replacing-macroquad.md)) measured
what the engine draws. It played the game's six bench scenarios headless
at this revision and counted every presented frame, once with the public
placeholder art and once with the bought art. The busiest frame was 3,551
quads: 14,204 vertices and 21,306 indices. It had five kinds of quad:
cell fills, glyphs, rectangles, sprites and painted pictures. No frame
used more than 22 textures. Keeping the painter's order and giving each
image (and each recoloured copy) its own texture, a frame needed at most
58 texture switches; with one atlas page it needed 1. Nothing in those
frames needs depth, instancing, compute or more than one shader.

The facts checked on 2026-11-22 (sources in the report):

- **wgpu 30.0.2** (2026-11-12) is MIT OR Apache-2.0, with MSRV 1.87. It
  makes a breaking release about every three months (25.0 in April 2025
  through 30.0 in July 2026). Bevy 0.20.1 (2026-10-28) uses `wgpu ^30`.
  - It has Vulkan, Metal and DX12 first class, and GL as "best effort":
    GL 3.3 on Windows, GLES 3 on Linux and Android.
  - On the web it has WebGPU, plus WebGL 2 as best effort. Since 0.19 one
    `.wasm` can hold both and choose at run time. A WebGPU-only `.wasm`
    leaves out wgpu-core and naga.
  - DX11 is gone. DX12 builds on `x86_64-pc-windows-gnu` and compiles
    shaders with a DXC or FXC it loads at run time, but wgpu's CI tests
    only MSVC.
- **winit 0.30.13** (2026-03-02) is Apache-2.0. **0.31** is at beta.4
  (2026-10-24), not stable, and Bevy 0.20.1 uses `winit ^0.30`.
  - It loads every Linux library with `dlopen`, and builds need no
    `-dev` headers with its default features.
  - Its IME events exist on Windows, macOS, X11 and Wayland, but not on
    the web, Android or iOS, in 0.30.13 or 0.31 beta.4.
  - `Occluded` is not reported on Windows, Wayland or Android.
  - No version gives a monitor's work area (issue #2494, open since
    2022).
- **wasm-bindgen 0.2.130** (2026-10-30) is MIT OR Apache-2.0.
  - `wasm-bindgen-cli` must match the crate's version exactly.
  - The `--target web` glue's `init` takes a `Response`, so a page can
    stream the `.wasm` itself.
  - wgpu and winit both need wasm-bindgen in the browser.
- **The bars this sets:**
  - ADR-0119: no public item names an outside crate's type.
  - ADR-0021 §4: each piece we write is a crate of its own.
  - 2018's `render` and 2019's `web-render` jobs compare the window and
    the canvas with the raster, and must stay green at every step.

## Decision

### 1. What the renderer is

The frame is a few thousand textured quads in the painter's order. So the
renderer is one batcher:

- one vertex and one index buffer per frame;
- one WGSL shader: the texel times the vertex colour;
- one pipeline per blend state and target format;
- a draw call wherever the texture, the blend or the clip changes.

There is no instancing, depth, compute or scene graph in it. The busiest
frame measured is about 280 KB of vertices a frame at 20 bytes a vertex,
which is not a cost worth designing around.

### 2. `shuyi-gpu`, the renderer's crate

It is a crate of its own, `crates/gpu` (ADR-0021 §4). It depends on no
`shuyi-*` crate, and the build is 3325.

- **Its public API names no wgpu type** (ADR-0119).
  - `Gpu` holds the device and queue, with `Wants` (the optional
    features) and `NoGpu` (each backend tried and why it failed).
  - `TextureId` is an RGBA8 `Rgba8Unorm` texture with nearest or linear
    sampling.
  - `TargetId` is a texture a pass draws into and a later pass samples.
  - `Frame` is a list of `Pass`es. Each pass goes into the screen or a
    target, has an optional clear colour, and holds `Quad`s.
  - A `Quad` has a destination in the target's pixels, a source in
    texels, a texture, a colour, `flip_x`, a `Blend` (`Over`, `Add`,
    `Multiply`) and a clip in whole pixels.
  - `Gpu::draw(&Frame)` draws and presents, and `Gpu::read(target)` reads
    a target back.
- **The engine's own crates reach wgpu through `#[doc(hidden)] pub mod
  raw`**, which semver doesn't cover (ADR-0119 §2). It is for making a
  surface from a backend's window, and for the passes in §6 to §8. A
  major version of wgpu is then never a breaking release of ours.
- **No `unsafe`.** wgpu's API is safe, and the surface comes from winit's
  window through wgpu's safe constructor. The crate is
  `unsafe_code = "forbid"`, which answers ADR-0021 §6's question.
- **`shuyi-app` turns the engine's frame into a `Frame`** in a new
  `paint.rs`, in `render.rs`'s order:
  - the clear rectangle;
  - the backdrop, under a clip that replaces macroquad's camera viewport;
  - the cell fills, the `Under` items, the glyphs, and the `Over` items.
  - It keeps `ImageTextures<T>` and `PaintedTextures<T>`, which are
    generic already, over `TextureId`.
- **Recolouring stays on the CPU.** `Paint::Dimmed`, `Shaded` and the
  silhouette are made as recoloured copies, cached as today, so the
  integer rules hold exactly. A shader can take them over later without a
  change to the API.
- **Read-back for the capture mode and the link's picture.** The same
  `Frame` is drawn again into a target of the window's size, and that
  target is read. No surface needs `COPY_SRC`, which not every backend
  offers.

### 3. The backends, in order, and what happens when none starts

| Platform | Tried in order | Last |
| -------- | -------------- | ---- |
| Windows | DX12, Vulkan, GL | the fallback adapter (WARP) |
| macOS | Metal | none |
| Linux, the Steam Deck | Vulkan, GL (EGL, GLES 3) | the fallback adapter (lavapipe or llvmpipe, if installed) |
| The web | WebGPU, when `navigator.gpu` gives an adapter (`util::new_instance_with_webgpu_detection`); else WebGL 2 | none |
| Android (later, 3015) | Vulkan, GLES 3 | none |
| iOS (later, 3015) | Metal | none |

How the list is tried:

- Each entry is one instance, then `request_adapter` with the window's
  surface and `request_device`. If that fails, the reason is logged and
  the next entry is tried.
- The device asks for `Limits::downlevel_webgl2_defaults()`, raised to
  what the adapter offers. Textures are refused, never cut, above its
  `max_texture_dimension_2d`, which is 2048 on WebGL 2 (2222's atlas
  pages are sized to fit).
- `WGPU_BACKEND`, wgpu's own variable, narrows the list when it is set.
  That serves the CI jobs and a player working around a bad driver.

When none starts:

- On the desktop, the log file and stderr name each backend tried and
  its error, and the process exits with code 1. On Windows a message box
  says the same, through `shuyi-app`'s Windows module.
- On the web, the page's shell shows it as its error (3052's
  `data-shuyi` state).

### 4. Versions, and moving to the next

- **wgpu 30**, with `default-features = false` and these features:
  - on every target: `std` and `wgsl`;
  - on Windows: `dx12`, `vulkan`, `gles`;
  - on Apple: `metal`;
  - on Linux and Android: `vulkan`, `gles`;
  - on wasm32: `webgpu`, `webgl`.

  Left off: `glsl`, `spirv`, `naga-ir`, `serde`, `trace`, `counters`,
  `static-dxc` (MSVC only, and it downloads a binary at build time),
  `angle`, `vulkan-portability`, `drm`, `noop`, `parking_lot` and
  `fragile-send-sync-non-atomic-wasm`.
- **winit 0.30**, with `default-features = false` and `rwh_06`, `x11`,
  `wayland`, `wayland-dlopen` and `wayland-csd-adwaita-notitle`. That last
  one avoids sctk-adwaita's title font and tiny-skia.
- **wasm-bindgen pinned exactly** (`=0.2.130` on 2026-11-22), with the
  CLI at the same version (§13).
- **Moving.** The engine follows the majors that Bevy's latest release
  uses, and is never more than one wgpu major behind the newest.
  - A major bump is a Dependabot PR worked with the `dependency-upkeep`
    skill (ADR-0037), green on the `render` and `web-render` jobs.
  - Its breaking changes land in `shuyi-gpu` and `shuyi-app` alone. None
    reaches a game (§2).
  - winit 0.31 changes the handler's methods: surfaces come in
    `can_create_surfaces` and go in `destroy_surfaces`. §12 is written so
    that both versions fit it. The first build ticket after 0.31 ships
    moves to it.

### 5. Build size

Measured with the prototype, the same 7,400 quads a frame, on the
workspace's release profile (the report, *Build size*):

| Build | Raw | gzip -9 |
| ----- | --: | ------: |
| winit + wgpu, Windows (DX12, Vulkan, GL) | 6.25 MB | 2.63 MB |
| macroquad, Windows | 0.81 MB | 0.38 MB |
| winit + wgpu `.wasm`, WebGPU only | 0.50 MB | 0.17 MB |
| winit + wgpu `.wasm`, WebGPU and WebGL 2 | 3.74 MB | 1.17 MB |
| macroquad `.wasm` | 0.49 MB | 0.17 MB |

- Natively, wgpu and winit add about 5.4 MB, which about doubles today's
  `hello-grid`.
- On the web, WebGPU alone adds nothing measurable. The WebGL 2
  fallback adds about 1.0 MB gzipped.
- **The web build ships both in one `.wasm`.** WebGPU isn't on by
  default in Chrome on most Linux machines, nor in Firefox on Linux.
  WebGL 2 is the price of the game opening everywhere it opens today.
- Every size is far under the web target's package budget (25 MB).
- A page that fetches a WebGPU-only build where WebGPU starts would save
  that megabyte. That is a second build to make and test, and nothing
  asks for it.

### 6. 3D, and the GPU painter (3104)

3104 is not done.

- **3104 is written over `shuyi-gpu`**, not miniquad, if 3326 is done
  first. Its `GpuPainter` hook in `shuyi-app` becomes a pass in the frame
  through `raw`:
  - a closure given the device, the queue, the frame's command encoder,
    the target's view, and a depth texture of the target's size that
    `shuyi-gpu` keeps when `Wants` asks for one;
  - it is drawn where the 3D view sits in the frame's passes.
- GL 3.3 and WebGL 2 stay its floor, as 3104 plans, through wgpu's GL
  backend.
- If 3104 is done over miniquad first, 3332 moves `shuyi-app-3d` to that
  pass before the switch, with its look test in the `render` job green.

### 7. Traced light (2911)

2911 is not done. What exists today:

- wgpu has `EXPERIMENTAL_RAY_QUERY`, native only, with acceleration
  structures (BLAS, TLAS) since 27.
- Its documentation names Vulkan only. 30.0.2's code turns it on for
  DX12 (raytracing tier 1.1, shader model 6.5) and for capable Metal
  devices too.
- naga's WGSL takes `ray_query` behind `enable wgpu_ray_query;`.
- WebGPU has no ray tracing, and WebGL 2 has no compute.

The room left for it:

- `Wants` lists optional features (ray query, timestamps, compute,
  depth). `shuyi-gpu` asks the device for each only when the adapter
  offers it, and `Gpu::has` says what it got.
- Compute and ray-query passes go through `raw`'s pass closure.

2911 builds on these.

### 8. The 2D effects that wait on the renderer (2326, 2327, 2328, 2331)

`Frame`'s passes into targets are the seed of every one of them. Each
builds its own piece:

- **A screen pass (2326):** a full-screen quad drawn from one target
  into the next with a WGSL fragment shader of the effect.
- **A game's WGSL passes and sprite materials (2327):** a material is a
  pipeline made from a game's WGSL, which naga checks when it loads. It
  carries extra textures and a uniform block of bytes, and a quad names
  it (the built-in shader when it names none).
- **2D lights with normal maps (2328):** a material whose second texture
  is the normal map, and a pass that gathers the lights into a target.
- **The render graph (2331):** it orders passes and targets, and
  `shuyi_gpu::Frame` is what it runs.

The built-in quad shader is itself a material, so none of these needs
the API rebuilt. 3325's ticket id joins the `blocked_by` of 2326 and
2331.

### 9. GPU frame times (2382)

Which backends give timestamps:

- `TIMESTAMP_QUERY`, written at a pass's start and end, works on Vulkan,
  DX12, Metal, GL with `GL_ARB_timer_query`, and WebGPU.
- In browsers it is quantized to 100 µs unless the page is cross-origin
  isolated.
- WebGL 2 probably gives none (wgpu checks the GL extension's name). The
  report has a row per backend.
- Queries inside encoders and passes are native only.

The room left: with `Wants::timestamps`, `shuyi-gpu` writes each pass's
start and end and reads them back one frame late. 2382 builds the timer
and the table in `docs/platform-backends.md` from the report. 3325 and
3326 join its `blocked_by`.

### 10. What the frame asks of every renderer (2337)

- **`Paint::Tinted`** is the quad's colour, with no copy of the image.
- **An overlay's opacity** is the colour's alpha.
- **`Blend::Add`** is `SrcAlpha, One`.
- **`Blend::Multiply`** is `Dst, Zero`, over a colour the shader mixes
  toward white by the alpha.
- **Every format is non-sRGB:** the surface is `Bgra8Unorm` or
  `Rgba8Unorm`, and textures are `Rgba8Unorm`. Blending then works on
  the same values as `shuyi_ui::frame::blend`'s integer rules, within one
  step per channel. The capture mode's comparison with the raster checks
  that.
- **A surface that offers only sRGB formats** gets a non-sRGB view
  format if it allows one. WebGL 2 offered the prototype only
  `Rgba8UnormSrgb`. Where neither exists, the frame is drawn into a
  non-sRGB target, and a last pass copies it to the screen with the
  encoding undone in the shader (each value decoded before the hardware
  encodes it again), so the bytes on screen are the raster's.

### 11. Text

The renderer draws glyphs only from atlases the engine lays out: the
cell font now, and 2224's baked fonts in 2334's runs. It shapes no text
and adds no text library to the `.wasm`.

2908 is not done, so the report records the options for it to use:

- `parley` 0.13, which Bevy's text uses;
- `cosmic-text` 0.19, `swash` 0.2 and `harfrust` 0.16;
- each with its license and what it does.

If 2908 finds that a script needs shaping at run time, the shaping runs
on the CPU in the engine's text layout. Its glyphs are baked into an
atlas page at run time (2222), and the renderer doesn't change.

### 12. The window, the loop and the system's events, over winit

The winit backend lives in `shuyi-app` beside the macroquad one, behind
a feature named **`winit`** (3326). macroquad becomes an optional
dependency behind a default feature named **`macroquad`**.

- With both on, winit wins, so a native game tries winit by adding the
  feature.
- A web build turns the default features off, because macroquad's
  exports must not be in a wasm-bindgen `.wasm`.
- 3332 swaps the default to `winit`, and `default-features = false,
  features = ["macroquad"]` undoes it.
- Like `in-house-sound` (ADR-0068 §8), both names stay for good, and
  each does nothing once winit is the only backend (3333).

- **The loop.** `shuyi-app` implements winit's `ApplicationHandler`.
  - On the desktop: `EventLoop::run_app`.
  - On the web: `spawn_app` in 0.30, `run_app` in 0.31.
  - The window and its surface are made in `resumed`
    (`can_create_surfaces` in 0.31) and dropped in `suspended`
    (`destroy_surfaces`), so Android and iOS fit later.
  - On native, `about_to_wait` sets `ControlFlow::WaitUntil` to the
    pacer's next due time and asks for a redraw when it is due.
    `RedrawRequested` runs `App::frame` and draws. `pacing.rs` keeps its
    rules (`docs/platform-backends.md`, *The frame cap*): with vsync
    (`PresentMode::Fifo`) the frame's wait is the present's own.
  - On the web, every `RedrawRequested` comes from
    `requestAnimationFrame`, and `Pacer::web_frame` decides as it does
    today.
- **Work that is async today** runs between frames:
  - the GPU's adapter and device, the voice manifest, the sound's start;
  - on native with `pollster::block_on`;
  - on the web with `wasm_bindgen_futures::spawn_local` into a slot the
    loop reads each frame. No `App::frame` runs before the GPU is ready,
    and the page's shell shows its loading screen until the first frame.
- **The entries stay.** `run_game`, `run_game_checked`, `run_app`,
  `run_dialogue_preview` and the hidden `open_window` keep their
  signatures, so neither a game on the short path nor a game of its own
  shape changes at the switch.
- **Keys.** `KeyboardInput`'s `physical_key` maps to the engine's `Key`,
  by position, as `keys.rs` maps macroquad's codes.
  - `text` becomes `RawInputEvent::Text`.
  - A synthetic press (X11 and Windows make them on focus changes) is
    dropped.
  - The Shift chord comes from `ModifiersChanged`.
  - 3327 checks one non-US layout against the macroquad build, as a
    black box, and writes any difference as a `Behaviour:` line.
- **The mouse and touch.**
  - `CursorMoved`, in physical pixels, maps to the console's pixels
    through the layout. `CursorLeft` becomes `PointerLeft`.
  - The buttons become `PointerDown` and `PointerUp`. The wheel's lines
    or pixels become whole `Wheel` steps.
  - A first touch is the pointer, as 2405 has it.
- **The clipboard.** winit has none, and nothing in the engine uses
  miniquad's today. 1149's chat box plans to, so whichever of 1149 and
  3332 lands second puts paste and copy on winit: `arboard` on the
  desktop, the browser's `paste` and `copy` events through `web-sys` on
  the web.
- **The system keyboard** (3003's `TextInput`): none on the desktop and
  the web, as today. On Android and iOS, `set_ime_allowed` shows the soft
  keyboard (winit 0.30.6), and 3015 takes it from there.
- **High DPI.**
  - The layout is computed in physical pixels, as `render.rs` does.
  - `ScaleFactorChanged` and `Resized` reconfigure the surface.
  - On the web the canvas's size follows its CSS size times
    `devicePixelRatio`.
- **Full screen and the Steam Deck.**
  - The player's Fullscreen is `Fullscreen::Borderless` on the window's
    monitor; leaving it is `set_fullscreen(None)`.
  - The Deck's profile opens full screen at 1280×800 on Vulkan (RADV).
- **Focus and visibility** (ADR-0076):
  - `Focused` gives `FocusLost` and `FocusGained` everywhere.
  - `Occluded` gives `Suspend` and `Resume` on macOS, X11 and the web
    (winit reads `visibilitychange` and an IntersectionObserver there).
  - On Windows, which reports no `Occluded`, a minimized window seen on
    `Resized` (`is_minimized()`) gives `Suspend`, and its restore gives
    `Resume`. 3327 checks this on the machine.
  - Wayland has no reliable signal, so it gets focus alone.
  - On mobile, `suspended` and `destroy_surfaces` give `Suspend`.
- **The window facts miniquad lacks** (3328):
  - The window opens at the largest whole multiple of the console that
    fits the monitor's work area. winit has no work area, so it comes
    from `GetMonitorInfoW`'s `rcWork` on Windows, from `NSScreen`'s
    `visibleFrame` on macOS, and from the monitor's size less a margin
    on Linux.
  - A saved place is put back only on a connected monitor. The record
    gains the place, the monitor's name and whether it was maximized
    (version 2 of 3044's record).
  - There is a minimum size, `with_min_inner_size`: the console at scale
    1 plus its margin.
  - The monitor can be chosen.
  - Exclusive fullscreen works on Windows, X11 and macOS. Wayland and
    the web fall back to borderless.
  - Custom cursor images come from `CustomCursor::from_rgba`.
  - Leaving fullscreen works on Linux.
  - 3044's record and 3314's cursor rule (`docs/platform-backends.md`,
    *The window and the cursor (desktop)*) carry over unchanged.
- **The cursor grab** (3106 is not done). A game asks for it through its
  frame, as it asks for fullscreen:
  - a `cursor_grab: bool` on the context, carried on `Frame` to the
    backend;
  - the backend applies it with `CursorGrabMode::Locked`, falling back
    to `Confined`. X11 has no `Locked` and macOS no `Confined`; on the
    web, `Locked` is pointer lock.
  - A locked cursor is never shown (3314). 3106 uses this, and until the
    winit backend replaces macroquad the same request maps to
    `set_cursor_grab`.
- **The IME** (3329). It is turned on while `Frame::typing` is set and
  placed at the buffer's text caret (2346):
  - winit's `set_ime_allowed` and `set_ime_cursor_area`
    (`request_ime_update` in 0.31) on Windows, macOS and Linux;
  - on the web, where winit has no IME, a hidden `<textarea>` at the
    caret. It is written from MDN and the UI Events specification
    (ADR-0021 §2), and reads `compositionstart`, `compositionupdate`,
    `compositionend` and `input`.
  - The composing text and its caret become a new
    `RawInputEvent::Composing { text, caret }` (the enum is
    `#[non_exhaustive]`). The committed text becomes `Text`.
  - The name box draws the composing text underlined. Every text box
    built on the name box's typing does too: 1149's chat box when it
    exists.

### 13. The web build moves to wasm-bindgen

macroquad's loader, `sapp_jsutils.js` and the rule against wasm-bindgen
(game ADR-0034's web part, `AGENTS.md` *Environment*) go with macroquad.
For the engine, this supersedes that part of game ADR-0034, and the PR
that makes winit the default (3332) updates `AGENTS.md`.

- **`cargo shuyi web`**, under the `winit` feature first (3330):
  - It builds the game for `wasm32-unknown-unknown`, then runs
    `wasm-bindgen --target web --no-typescript` into the staging folder.
  - The CLI must be the exact version in the game's `Cargo.lock`. The
    tool reads that version, and if `wasm-bindgen --version` differs or
    the CLI is missing, it says to run `cargo install --locked
    wasm-bindgen-cli --version <it>`.
  - `shuyi doctor` checks the same.
  - It doesn't run `wasm-opt`.
- **The page and the shell.**
  - `index.html` imports the glue, and the start goes through 3052's
    `shell.js`. `shell.js` fetches the `.wasm` itself, counting bytes for
    its progress, and passes the `Response` to `init({ module_or_path })`,
    which streams it.
  - Its `data-shuyi` states stay.
  - The pack's streaming (`pack-file.js`) still starts before the
    `.wasm`.
- **The plugins.** Each becomes `web-sys` calls or a small
  `#[wasm_bindgen(module = "/web/x.js")]` import, whichever is less code
  (3330, 3331):
  - `storage.js`: `web-sys`'s `Storage` (3330);
  - `gamepad.js`: `Navigator::get_gamepads` (3331);
  - `sound.js`: stays JS, as its AudioWorklet must, imported by
    `shuyi-sound-out` behind a feature of that crate that `shuyi-app`'s
    `winit` turns on (3331);
  - `pack-file.js`, `page-files.js` and `player-files.js`: small imports
    of the same code (3331).
  - `lifecycle.js`, if 3043 is done, becomes `visibilitychange` and
    `pagehide` listeners that run a frame, the game's save inside it,
    from Rust (3331).
  - 2601's `http.js` moves too, if it exists by then.
- **The log sinks** (3041):
  - The browser console sink goes through `web-sys`'s `console`.
  - macroquad's `log-rs` feature goes with macroquad.
  - The file and stderr sinks stay. Their seconds come from the
    backend's clock (`std::time::Instant` on native, `performance.now()`
    on the web) in place of `miniquad::date::now()`.

### 14. Linux's system libraries

- **winit opens with `dlopen`:**
  - for X11: libX11, libX11-xcb, libXcursor, libXi, libxcb, libxkbcommon
    and libxkbcommon-x11;
  - for Wayland: libwayland-client.
- **wgpu opens with `dlopen`:**
  - for Vulkan: libvulkan.so.1;
  - for GL: libEGL.so.1, with GL itself through EGL;
  - on Wayland, libwayland-egl as well.
- **A build needs no `-dev` package for either.** macroquad links libGL,
  libX11 and libXi at build time today.
- **3333 changes the lists to match:**
  - doctor's Linux table (`crates/cli/src/contract/doctor/linux.rs`);
  - `scripts/setup-linux.sh`;
  - 3071's `crates/cli/export/libraries.ron`, through a clause with
    3071.

  The table keeps the run-time libraries, and drops the GL and X11
  headers.

### 15. Android and iOS later (ADR-0008, 3015)

Nothing here rules them out:

- surfaces come and go with winit's events (§12);
- the loop never blocks;
- textures and pipelines are made again after a surface is lost;
- touch is the pointer, and the soft keyboard is `set_ime_allowed`.

Android needs a `cdylib` and android-activity; that is 3015's.

### 16. The order of the build

The game keeps working at every step, as sound did. Each ticket is one
PR, parked, and `blocked_by` chains from 3079:

1. **3325**: `shuyi-gpu`, tested headless.
2. **3326**: the winit backend on the desktop behind `winit`: the window,
   the loop, painting, the window record, the capture mode.
3. **3327**: input over winit, focus and the cursor.
4. **3328**: the window facts miniquad lacks.
5. **3330**: the web build on wasm-bindgen under `winit`.
6. **3331**: the other web plugins.
7. **3329**: the IME (after 2346).
8. **3332**: `winit` becomes the default feature, with macroquad one line
   away (`default-features = false, features = ["macroquad"]`). It
   updates `AGENTS.md`. The game moves its pin to it and runs on it
   before the next step.
9. **3333**: macroquad, miniquad, `sapp-jsutils` and the vendored loader
   removed, and the Linux lists changed (ADR-0021 §5).

### 17. The regression net

- **The capture mode is kept** in the new backend: the `capture` feature,
  `shuyi_platform::capture`'s plans and `raster::compare`.
- **Until 3333, both backends run in CI:**
  - The `render` job runs the render card and `hello-grid`'s plan on
    macroquad and again with `--features shuyi-app/winit`: on lavapipe
    (Vulkan), and with `WGPU_BACKEND=gl` on llvmpipe.
  - The `web-render` job runs both web builds: WebGL 2 on SwiftShader,
    and WebGPU where its Chromium starts it.
- Each build ticket says how its piece keeps both jobs green.

## Consequences

- The engine draws through Vulkan, Metal, DX12, GL and GLES, WebGPU and
  WebGL 2 with about a thousand lines of renderer of its own. No game
  changes at the switch, and no public signature names wgpu or winit.
- The engine follows two libraries that break often. wgpu's majors are
  Dependabot PRs held to the `render` and `web-render` jobs, and land in
  two crates. That work is scheduled every three months or so.
- The web build gets a second tool, `wasm-bindgen-cli`, pinned to the
  crate's version, which `doctor` checks. The `.wasm` grows (§5).
- The IME on the web is our own (a hidden text area), since winit has
  none there. It is the one input path written from browser specs
  rather than taken from winit.
- A machine with no adapter at all gets a reason in the log, and a
  message box on Windows, instead of a black window.
- Until 3333, `shuyi-app` builds two backends and CI renders twice.
- ADR-0068's line that 3034 must not start until the sound chain ends now
  holds for these tickets, not the research: each waits for 3079.

## Alternatives considered

- **Use wgpu directly in `shuyi-app`, with no renderer crate.** Lost: it
  couldn't move to its own repository (2008), and the 2D effects and the
  3D painter would each grow their own batching. ADR-0021 §4 asks for a
  crate.
- **Expose wgpu's types in `shuyi-gpu`'s API.** Lost: ADR-0119. Every
  wgpu major would be a breaking release of ours, about every three
  months.
- **One pipeline that does recolouring and blends in the shader now.**
  Lost for now: the CPU copies keep the integer rules exact, the
  measured frames need few of them, and the API leaves the door open.
- **Instanced quads instead of a vertex batch.** Lost: one batch of 3,500
  quads is cheap, and instancing would need two code paths on WebGL 2.
- **WebGPU only on the web.** Lost: WebGPU is off by default in Chrome on
  most Linux machines and in Firefox on Linux (gpuweb's implementation
  status, 2026-10-02), so the game would stop opening where it opens
  today.
- **The Emscripten target, with no wasm-bindgen.** Lost: winit needs
  wasm-bindgen on the web, and wgpu's Emscripten path has one GLES
  example from 2023.
- **winit 0.31 beta now.** Lost: it isn't stable, and Bevy is on 0.30. §4
  and §12 fit both.
- **Our own window layer for the facts winit lacks.** Lost: Nick ruled it
  out (2026-10-19). The work area alone comes from each system's own call
  (§12).
- **A separate crate for the winit backend** (`shuyi-app-winit`). Lost:
  games depend on `shuyi-app` and `shuyi::prelude::run_game`, and a
  feature in one crate kept sound's switch to one line (ADR-0068).
