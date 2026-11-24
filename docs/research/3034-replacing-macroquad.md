# Replacing macroquad: winit, wgpu and a 2D batcher of our own

Research for ticket 3034, checked **2026-11-22** at engine revision
`f51fb23a`. The plan and its reasons are
[ADR-0136](../adr/0136-replacing-macroquad-winit-wgpu-and-wasm-bindgen.md),
and the build is tickets 3325–3333. Every one of those tickets is parked,
and each waits for 3079. Nick unparked the research alone
([decision of 2026-11-22](../decisions.md)).

In short:

- **The frame is small.** It is a few thousand textured quads, so the
  renderer is one batcher with one WGSL shader over wgpu, in a crate of
  its own (`shuyi-gpu`).
- **The window and its input are winit 0.30,** inside `shuyi-app` behind
  a `winit` feature until it becomes the default.
- **The web build moves to wasm-bindgen,** with WebGPU and WebGL 2 in
  one `.wasm`.
- **winit lacks three things the plan needs:** the IME on the web, a
  monitor's work area, and `Occluded` on Windows. Each is covered in the
  plan, by our own code or a system's own call.
- **Size:** wgpu and winit add about 5.4 MB to a desktop program. The
  WebGL 2 fallback adds about 1 MB gzipped to the `.wasm`; WebGPU alone
  adds nothing measurable.

## What the engine draws

**How it was measured.** A throwaway program played the game's six
bench scenarios (`bench/*.bench.ron` in Visions of Shuyi, pinned at
`cc9798d8`) headless, on `shuyi_platform`'s Harness, with the engine
patched to this revision:

- It installed a present hook and counted, in every frame presented,
  what `render.rs` would draw, in its order.
- It ran twice: with the public placeholder art, as the bench gates run,
  and with the bought art embedded (`private-assets`), as players get
  it.
- Each frame is at 60 frames a second (`FRAME_DT`).

A **quad** is one rectangle drawn: the clear rectangle, a cell's fill, a
glyph (two for a bold glyph without its own shape), a rectangle, a
sprite, a painted picture or a run of a gizmo. Each quad is four
vertices and six indices. The two draw-call columns count texture
switches in the painter's order:

- **A**: one texture per image and per recoloured copy, as
  `render.rs` makes them;
- **B**: the font and every image in one atlas page (2222).

| Scenario | Art | Frames | Mean quads | Busiest frame: quads (vertices) | Its fills / glyphs / rects / sprites | Most textures | Most draw calls A / B |
| -------- | --- | -----: | ---------: | ------------------------------: | ------------------------------------ | ------------: | --------------------: |
| battle-attack | public | 372 | 3,045 | 3,366 (13,464) | 3,000 / 345 / 10 / 8 | 4 | 5 / 1 |
| battle-attack | bought | 372 | 1,917 | 2,395 (9,580) | 900 / 185 / 689 / 617 | 21 | 58 / 1 |
| battle-cursor-sweep | public | 59 | 2,478 | 2,690 (10,760) | 1,516 / 956 / 195 / 0 | 2 | 2 / 1 |
| battle-cursor-sweep | bought | 59 | 1,878 | 2,104 (8,416) | 900 / 349 / 222 / 609 | 20 | 49 / 1 |
| battle-enemy-turn | public | 596 | 2,776 | 3,366 (13,464) | 3,000 / 345 / 10 / 8 | 4 | 5 / 1 |
| battle-enemy-turn | bought | 596 | 2,404 | 3,367 (13,468) | 3,000 / 344 / 10 / 10 | 22 | 46 / 1 |
| battle-idle | public | 61 | 2,561 | 2,561 (10,244) | 1,516 / 952 / 70 / 0 | 1 | 1 / 1 |
| battle-idle | bought | 61 | 1,963 | 1,963 (7,852) | 900 / 345 / 82 / 612 | 19 | 48 / 1 |
| dialogue-portrait | public | 297 | 2,288 | **3,551 (14,204)** | 2,472 / 1,065 / 10 / 1 | 2 | 3 / 1 |
| dialogue-portrait | bought | 297 | 1,929 | 2,980 (11,920) | 1,992 / 585 / 10 / 390 | 6 | 7 / 1 |
| options-menu | public | 73 | 2,155 | 2,200 (8,800) | 1,620 / 566 / 10 / 1 | 2 | 6 / 1 |
| options-menu | bought | 73 | 2,300 | 2,348 (9,392) | 1,740 / 594 / 10 / 1 | 2 | 7 / 1 |

What this means for the renderer:

- **The console is 80 × 40 cells** (3,200) in every scenario.
- **No frame drew a painted picture, a gizmo or a backdrop**, so those
  paths are rare.
- **The bought art draws the battle map as about 610 tile sprites.**
  Nearly all of them are recoloured copies for the battle's light
  (`Paint::Shaded`), which is why the textures climb to 22.
- **The busiest frame is 14,204 vertices.** At 20 bytes a vertex that is
  about 280 KB a frame, or 17 MB a second at 60 frames.
- **At most 58 draw calls a frame** with one texture per image, and one
  with a shared atlas page. Neither is a cost on any GPU wgpu runs on.
- So the renderer needs one batcher, one shader and a pipeline per blend
  state, and nothing more (ADR-0136 §1). 2222's atlases would make a
  frame one draw call, but the plan doesn't need them.

## What the macroquad backend does today

`shuyi-app` calls macroquad through `macroquad::prelude::*` in most
files. `run_app` and `open_window` call the hidden
`macroquad::Window::from_config`, and `run` is an `async fn` stepping
on `next_frame().await`.

The web plugins are `extern "C"` imports in the miniquad plugin ABI:

| Plugin | Rust side | Engine's own lines | Becomes (ADR-0136 §13) |
| ------ | --------- | -----------------: | ---------------------- |
| `storage.js` | `storage/web.rs` | 133 | `web-sys` `Storage` |
| `gamepad.js` | `pads/web.rs` | 102 | `Navigator::get_gamepads` |
| `sound.js` | `shuyi-sound-out`'s `sys.rs` | 365 | a `#[wasm_bindgen(module)]` import of the same worklet code |
| `pack-file.js` | `pack_file.rs` | 161 | an import |
| `page-files.js` | `page.rs` | 143 | an import |
| `player-files.js` | `player_files/web.rs` | 128 | an import |
| `sapp_jsutils.js` | strings and bytes across | 186 (vendored) | gone: wasm-bindgen passes them |
| `mq_js_bundle.js` | the loader | vendored | gone: wasm-bindgen's glue |

The tickets the new backend keeps working, and their state on
2026-11-22:

- **Done:**
  - 2018 and 2019: the capture mode, the `render` and `web-render` jobs;
  - 3041: the log sinks;
  - 3042: ADR-0076's focus;
  - 3044: the window record;
  - 3314: the idle cursor;
  - 2513: doctor's Linux table and `setup-linux.sh`;
  - 2511 and 2087: `run_game` and `run_app` as plain functions;
  - 2405 and 2413: the mouse and touch, 2413 reading them in
    `shuyi-app`;
  - 2215: the game's files embedded without `include_dir`.
- **Open:**
  - 2337: blends and tints;
  - 2346: `Frame::typing` and the IME on Windows;
  - 3043: `lifecycle.js`;
  - 3052: `shell.js`;
  - 2908: shaped scripts;
  - 3104: the GPU painter;
  - 3106: the pointer lock;
  - 2909: video;
  - 2601: HTTP;
  - 3071: the library audit;
  - 1149: the chat box;
  - 2911: traced light.

  The ADR plans what it can for each, and the tickets carry an
  either-order clause where order matters.

## wgpu

Versions and policy:

- **wgpu 30.0.2** (2026-11-12) is MIT OR Apache-2.0, with MSRV 1.87, and
  naga 30.0.2 has the same license.
- **Majors:** 25.0 (2025-04-10), 26.0 (2025-07-10), 27.0 (2025-10-01),
  28.0 (2025-12-18), 29.0 (2026-03-19) and 30.0 (2026-07-01). The README
  says "a breaking version every three months".
- **MSRV:** since v29 it rises only for a real benefit, never above
  stable−3.
- **Bevy:** 0.20.1 (2026-10-28) uses `wgpu ^30`. Bevy moved to 30 in one
  PR (#24841, 38 files, 2026-07-16) and to 29 in #23277 (48 files).
- **The last two majors broke:**
  - v29: the surface texture is an enum, `InstanceDescriptor::default`
    is gone, and the depth-stencil fields became `Option`s.
  - v30: `VertexState.buffers` is `Option`s, `present()` moved to the
    queue, `SurfaceConfiguration` gained `color_space`, and
    `get_mapped_range` returns a `Result`.

**Backends** (README; `wgpu-types` 30.0.2):

| Backend | Where | Tier |
| ------- | ----- | ---- |
| Vulkan | Windows, Linux, Android (macOS and iOS through MoltenVK and `vulkan-portability`) | first class |
| Metal | macOS, iOS | first class |
| DX12 | Windows 10 and later | first class |
| GL | GL 3.3 on Windows (WGL), GLES 3.0 on Linux and Android (EGL); macOS through ANGLE only | best effort |
| WebGPU | browsers | first class |
| WebGL 2 | browsers | best effort |

- **Choosing backends.** `Backends::PRIMARY` is Vulkan, Metal, DX12 and
  WebGPU. `WGPU_BACKEND` (`Backends::from_env`) narrows the choice.
  Nothing chains a fallback for us: a backend's instance that gives no
  adapter is retried with the next (ADR-0136 §3).
- **WebGPU and WebGL 2 in one `.wasm`** works since 0.19. The instance
  chooses when it is made. `navigator.gpu` can exist with no adapter
  (Chrome on Linux), so `util::new_instance_with_webgpu_detection` checks
  for one, and WebGL 2 needs `compatible_surface`.
- **WebGL 2's limits** (`Limits::downlevel_webgl2_defaults`): 2D textures
  of 2048, no storage buffers, and no compute.
- **DX11 is gone.**
- **The `noop` backend** creates buffers but executes nothing.

**Cargo features (30.0.2).**

- The defaults are `std`, `parking_lot`, `dx12`, `metal`, `gles`,
  `vulkan`, `wgsl` and `webgpu`.
- A WGSL-only 2D engine turns off `glsl`, `spirv`, `naga-ir`, `serde`,
  `trace`, `counters`, `static-dxc`, `angle`, `vulkan-portability`,
  `drm` and `strict_asserts`.
- Turning off `wgsl` saves nothing natively: wgpu-core always enables
  it there (issue #7518).
- With every feature on, every license is permissive: MIT, Apache-2.0,
  Zlib, ISC, BSD-3-Clause (from `glsl` only), Unlicense and Unicode-3.0.
- `static-dxc` downloads a prebuilt DXC at build time and needs MSVC, so
  it stays off.

**DX12 on the GNU toolchain.**

- It builds: the backend's `cfg` is `target_os = "windows"`, and
  `static-dxc` is gated to MSVC (issue #8303, fixed by #8430 on
  2025-11-19).
- Shaders compile through `Dx12Compiler::Auto`: a `dxcompiler.dll` if
  one is present, else FXC from `d3dcompiler_47.dll`, both loaded at run
  time.
- wgpu's CI tests only MSVC. The prototype below ran it on GNU.

**Timestamp queries**, for 2382's table:

| Backend | `TIMESTAMP_QUERY` (a pass's start and end) | Inside encoders and passes |
| ------- | ------------------------------------------ | -------------------------- |
| Vulkan | yes, where `timestamp_valid_bits >= 36` | yes |
| DX12 | yes | yes |
| Metal | yes | AMD and Intel GPUs only, not Apple's |
| GL / GLES | with `GL_ARB_timer_query` | with `GL_ARB_timer_query` |
| WebGPU | yes, quantized to 100 µs unless cross-origin isolated (WebGPU §20.4; Chrome 121) | no (native only) |
| WebGL 2 | probably none: wgpu checks the GL extension's name (from the source, not documented) | no |

`Queue::get_timestamp_period` gives nanoseconds per tick: 0 where there
are none, and 1.0 on the web.

**Ray query**, for 2911:

| Backend | `EXPERIMENTAL_RAY_QUERY` |
| ------- | ------------------------ |
| Vulkan | yes (the only one its documentation names) |
| DX12 | turned on in 30.0.2's code for raytracing tier 1.1 and shader model 6.5 |
| Metal | turned on in 30.0.2's code on capable devices (acceleration structures since 29) |
| GL, WebGL 2, WebGPU | no |

- The feature is native only, and its acceleration structures (BLAS,
  TLAS) have been part of it since 27. 30 adds procedural AABB BLAS.
- naga's WGSL takes `ray_query` behind `enable wgpu_ray_query;`.
- Ray-tracing pipelines are a separate feature, Vulkan only.
- **On this machine** (the prototype, an NVIDIA RTX 5090 laptop GPU),
  Vulkan reported the feature and DX12 didn't.
- Compute works on Vulkan, DX12, Metal, WebGPU, and GL from ES 3.1 or
  GL 4.3, but not on WebGL 2.

**WebGPU in browsers** (gpuweb's implementation status, edited
2026-10-02):

- **Chrome and Edge:**
  - Windows, macOS and ChromeOS since 113, and Android 12 and later since
    121.
  - On Linux only Intel Gen12 and later (144) and NVIDIA on Wayland
    (147). The rest of Linux is behind a flag, AMD and so a Steam Deck's
    browser included.
- **Firefox:** Windows since 141 and macOS since 147. Linux is Nightly
  only.
- **Safari:** 26, on macOS, iOS and iPadOS.

So WebGL 2 is the fallback the web build needs.

**The specifications:**

- WGSL is a W3C Candidate Recommendation Draft of 6 November 2026.
- WebGPU's is of 16 November 2026.

**Headless tests.**

- wgpu renders offscreen and reads back (`copy_texture_to_buffer`,
  rows aligned to 256 bytes, `map_async`, `device.poll`), as its
  `render_to_texture` example does.
- Its own CI runs GPU tests on hosted runners with no GPU: lavapipe
  (Mesa) on Linux, WARP on Windows.

**The Steam Deck.** wgpu documents nothing about it. Its one Deck issue
(#4600, 2023) no longer reproduces. RADV issues are open (#9742,
#9905), and wgpu works around the memory-budget one.

## winit

Versions:

- **0.30.13** (2026-03-02) is the stable release, Apache-2.0, with
  `dpi` "Apache-2.0 AND MIT".
- **0.31.0-beta.4** (2026-10-24) is a pre-release, with 25 issues of its
  milestone still open.
- **Earlier:** 0.30.12 (2025-07-27) and 0.30.11 (2025-05-21), with
  0.31's betas from 2025-11-16 on.
- **MSRV:** 1.70 for 0.30.13; 1.88 at 0.31, says the README.
- **Bevy:** 0.20.1 uses `winit ^0.30`.

**The loop.**

- **0.30's `ApplicationHandler`:** windows and surfaces come after the
  first `resumed`. `ControlFlow` is `Poll`, `Wait` or `WaitUntil`, and
  `request_redraw` gives `RedrawRequested`, aligned with
  `requestAnimationFrame` on the web.
- **0.31's handler:** `can_create_surfaces` and `destroy_surfaces` take
  over the surfaces, `resumed` and `suspended` stay for mobile and the
  web, and windows are `Box<dyn Window>`.
- **On the web:** 0.30's `run_app` throws to never return, so
  `spawn_app` is the way. 0.31's `run_app` returns.
- **A window made before the loop runs** is deprecated in 0.30 and gone
  in 0.31.

**Keys.**

- `KeyEvent` gives `physical_key` (the key's position), `logical_key`,
  `text`, `repeat` and `location`.
- X11 and Windows make synthetic presses on a focus change.
- The modifiers arrive in their own event.

**The IME** (0.30.13 and 0.31 beta.4):

| Platform | `Ime` events (preedit with its cursor, commit) | Placing the candidates (`set_ime_cursor_area`) |
| -------- | ---------------------------------------------- | ---------------------------------------------- |
| Windows | yes | yes |
| macOS | yes | yes |
| X11 | yes | position only |
| Wayland | yes | yes; `set_ime_purpose` too |
| The web | **no** (issue #4424; EditContext PR #4428 open) | no |
| Android, iOS | no; `set_ime_allowed` shows the soft keyboard since 0.30.6 | no |

- The IME is off until `set_ime_allowed`, and `KeyboardInput` stops
  while it composes.
- 0.31 folds the setters into `request_ime_update`.
- So the web's IME is a hidden text area of our own (ADR-0136 §12).

**Focus and visibility.**

- `Focused` is reported everywhere.
- `Occluded` is not reported on Windows, Wayland or Android. On the web
  it comes from `visibilitychange` and an IntersectionObserver on the
  canvas.
- The web's `suspended` and `resumed` fire only on `pagehide` and
  `pageshow` with the back-forward cache, not on a tab switch.

**The window and its monitors.**

- `with_min_inner_size` works on the desktop.
- `set_maximized` and `is_maximized` don't work on mobile or the web.
- `outer_position` doesn't work on Wayland or Android.
- `MonitorHandle` gives the name, size, position, scale and video modes.
- **No work area on any platform** (issue #2494, open since 2022, "needs
  discussion"). ADR-0136 §12 takes it from the systems' own calls.
- **Fullscreen:** Wayland ignores `Exclusive`. The web has borderless
  only, after a user gesture. Leaving is `set_fullscreen(None)`, with
  nothing documented against it on X11 or Wayland.

**The cursor.**

| Mode | Windows | macOS | X11 | Wayland | The web |
| ---- | ------- | ----- | --- | ------- | ------- |
| `Confined` | yes | no | yes | yes | no |
| `Locked` | yes | yes | no | yes | yes (pointer lock) |

- `CustomCursor::from_rgba` works on the desktop and the web. Browsers
  often cap a cursor at 128 × 128.
- `set_cursor_visible` hides the cursor inside the window, or while it
  has focus on macOS.

**High DPI.**

- `inner_size` is in physical pixels.
- On the web, `scale_factor` is `devicePixelRatio`, which includes the
  zoom and can be below 1.
- 0.31 sends no `ScaleFactorChanged` at creation, so the scale is read
  then.

**Linux.**

- 0.30.13's default features are `rwh_06`, `x11`, `wayland`,
  `wayland-dlopen` and `wayland-csd-adwaita`.
- Everything is loaded with `dlopen`:
  - libX11, libXcursor, libX11-xcb and libXi (x11-dl);
  - libxcb (x11rb);
  - libxkbcommon and libxkbcommon-x11;
  - libwayland-client.
- 0.31 drops libXcursor and adds libXfixes.
- With the defaults a build needs no `-dev` headers, from the build
  scripts of x11-dl and wayland-sys.
- The Wayland title bar's default font path brings `ab_glyph` and
  `tiny-skia` (BSD-3-Clause). The `-notitle` feature leaves them out.
  Nothing copyleft either way.
- wgpu opens libvulkan.so.1 through ash, and libEGL.so.1 through
  khronos-egl, with GL through EGL. On Wayland it also opens
  libwayland-egl through wayland-sys (wgpu-hal 30.0.2's source).

**Mobile.**

- Android needs android-activity 0.6 (MIT OR Apache-2.0) and a
  `cdylib`.
- Surfaces are made and dropped with the lifecycle events.

**AccessKit (3035).**

- `accesskit_winit` 0.34.2 is Apache-2.0 and needs winit `^0.30.5`.
- Its adapter must exist before the window is first shown, so the
  window starts hidden.
- It has no web adapter.

## The web build

**wasm-bindgen.**

- **Version:** 0.2.130 (2026-10-30), MIT OR Apache-2.0, with
  `wasm-bindgen-cli` released the same day. MSRV is 1.81 for the library
  and 1.86 for the CLI.
- **The CLI must match exactly:** its error says the "schema versions
  must exactly match". The guide installs it at the version in
  `Cargo.lock`.
- **`--target web`** writes an ES module, `<name>.js`, beside
  `<name>_bg.wasm`, and `init({ module_or_path })` takes a `Response`,
  an `ArrayBuffer`, a `WebAssembly.Module` or a promise of one.
- **Streaming:** given a `Response`, it streams the `.wasm` with
  `instantiateStreaming`, and falls back to the array buffer if the
  server's MIME type is wrong. So 3052's shell can fetch the `.wasm`
  itself, count its bytes and pass the `Response` on.
- **Our own JS:** `#[wasm_bindgen(module = "/path.js")]` imports an ES
  module of our own (no `import` inside it).
- **wasm-opt:** wasm-bindgen doesn't run it. Binaryen is Apache-2.0.

**web-sys 0.3.107** has what the plugins need, each a cargo feature:

- `console`;
- `Storage` with `Window`;
- `Document` with `VisibilityState`;
- `PageTransitionEvent`;
- `Navigator` with `Gamepad`.

winit has no gamepads.

## Text, if a script ever needs shaping at run time

The renderer draws glyphs from atlases the engine lays out, and adds no
text library (ADR-0136 §11). These are the options for 2908:

| Crate | Version | License | What it does |
| ----- | ------- | ------- | ------------ |
| `parley` | 0.13.0 (2026-11-12) | Apache-2.0 OR MIT | layout: shaping through HarfRust, line breaking, bidi, alignment, a font database. Bevy's text (0.20.1) uses it with `swash` |
| `cosmic-text` | 0.19.0 (2026-04-22) | MIT OR Apache-2.0 | shaping (HarfRust), layout with bidi, fallback fonts, drawing through swash, editing |
| `swash` | 0.2.10 (2026-07-17) | Apache-2.0 OR MIT | font reading, complex shaping, glyph drawing; no layout |
| `harfrust` | 0.16.1 (2026-11-19) | MIT | HarfBuzz's shaping in Rust (matching 15.2.0); shaping only |
| `rustybuzz` | 0.20.1 (2024-11-12) | MIT | the older port HarfRust forked; slower |
| `skrifa` | 0.50.0 (2026-11-10) | MIT OR Apache-2.0 | outlines, hinting, metadata; no shaping |
| `fontdue` | 0.9.5 (2026-10-19) | MIT OR Apache-2.0 OR Zlib | drawing glyphs and a naive layout; no shaping |
| `ab_glyph` | 0.2.32 (2025-09-28) | Apache-2.0 | loading and drawing glyphs; no shaping |

None of their READMEs gives a `.wasm` size. 2908 measures the one it
picks.

## Build size

Built on 2026-11-22 on the GNU toolchain (Rust 1.100.0) with the
workspace's release profile: thin LTO, one codegen unit, stripped.

- **wgpu-quads** is winit 0.30.13 and wgpu 30.0.2 with
  `default-features = false`. It draws 7,400 textured quads a frame:
  80 × 45 cells, each a fill and a glyph, plus 200 sprites, from one
  generated atlas.
- **mq-quads** is macroquad 0.4.16 drawing the same quads.
- **hello-grid** is the engine's example as it ships today.

| Build | Raw | gzip -9 | Brotli 11 |
| ----- | --: | ------: | --------: |
| wgpu-quads, Windows, `dx12` + `vulkan` + `gles` | 6,254,592 | 2,634,527 | 1,991,090 |
| wgpu-quads, Windows, `vulkan` + `gles` (Linux's set) | 5,470,208 | 2,308,178 | 1,756,626 |
| mq-quads, Windows | 810,496 | 376,381 | 313,962 |
| hello-grid today, Windows | 5,326,848 | | |
| wgpu-quads `.wasm`, WebGPU only | 498,715 | 174,459 | 137,349 |
| its wasm-bindgen glue (`.js`) | 90,092 | 15,396 | 13,234 |
| wgpu-quads `.wasm`, WebGPU and WebGL 2 | 3,744,782 | 1,174,571 | 826,318 |
| its glue | 140,365 | 21,489 | 18,074 |
| mq-quads `.wasm` | 493,315 | 165,536 | 132,256 |
| macroquad's loader, `mq_js_bundle.js` | 37,421 | 10,274 | 8,934 |
| hello-grid today `.wasm` (files embedded) | 3,797,318 | 1,214,966 | 808,162 |

What this means:

- **Natively, wgpu and winit add about 5.4 MB** to a program, or about
  2.3 MB gzipped, against macroquad's drawing. That is most of what
  `hello-grid` weighs today again, so a desktop download about doubles.
  The three Windows backends cost 0.8 MB more than Linux's two.
- **On the web, WebGPU alone costs nothing measurable** over macroquad:
  the browser compiles the WGSL itself.
- **The WebGL 2 fallback adds about 3.2 MB raw** (1.0 MB gzipped,
  0.7 MB Brotli). It brings wgpu-core, wgpu-hal and naga's GLSL output.
  A game like `hello-grid` would go from 1.2 MB to about 2.2 MB
  gzipped.
- **Both fit the web target's package budget** (25 MB,
  `targets/web.ron`) many times over.
- ADR-0136 §5 ships one `.wasm` with both backends. WebGPU is off by
  default in Chrome on most Linux machines and in Firefox on Linux, and
  WebGL 2 is the fallback that keeps the game opening there.

The release builds took 102 s (all Windows backends), 143 s (Vulkan and
GLES), 87 s (`.wasm`, WebGPU) and 266 s (`.wasm`, both) from clean,
against 30 s and 24 s for macroquad's.

**`wasm-opt -Oz`** (Binaryen 133) cut the raw `.wasm` by about 14% but
left the Brotli size where it was, and made the WebGL 2 build's slightly
larger. On this GNU toolchain it crashed unless `BINARYEN_CORES=1`, so
the web build doesn't run it (ADR-0136 §13).

**wasm-bindgen warned on a stripped `.wasm`:** with no `target_features`
section it can't tell which wasm features are on. An unstripped build
passed through `wasm-bindgen --remove-name-section` was 483,913 bytes
(3330).

## The prototype

Built in a scratch folder (`wgpu-quads`, `mq-quads`) to check the
renderer's shape, the backends on this machine and the sizes above, then
deleted. It followed wgpu's and winit's documentation and examples only.

**On Windows (GNU toolchain).** The laptop has an NVIDIA GeForce RTX 5090
Laptop GPU and an Intel integrated GPU. Each backend was chosen with
`WGPU_BACKEND`, and each ran 600 frames of 7,400 quads in one draw call
under vsync (`Fifo`).

| Backend | Adapter | Start to the device (ms) | CPU to build and submit a frame (mean, ms) | `TIMESTAMP_QUERY` / inside encoders and passes | Ray query |
| ------- | ------- | -----------------------: | -----------------------------------------: | ---------------------------------------------- | --------- |
| DX12 | NVIDIA (driver 32.0.16.1742) | 4,380 | 0.41 | yes / yes | no |
| Vulkan | NVIDIA (617.42) | 1,625 | 0.29 | yes / yes | yes |
| GL | Intel (GL 4.6) | 335 | 0.70 | yes / yes | no |

- DX12 builds and runs on `x86_64-pc-windows-gnu`, the GNU toolchain
  wgpu's CI doesn't test.
- **DX12's start was slow the first time:** 4.0 s of its 4.4 s went to
  the surface and the adapter. Later runs took 1.6 s. Vulkan's instance
  alone took 1.3 to 1.45 s every time, perhaps the machine's implicit
  layers (not confirmed). 3326 finds why before keeping DX12 first.
- **With no `WGPU_BACKEND`, wgpu picked Vulkan on the NVIDIA GPU.** GL
  landed on the Intel GPU, whatever the power preference asked.
- **The vsync wait** fell in `get_current_texture` on DX12 and Vulkan,
  and in `present` on GL. The pacer's "never wait on top of vsync" rule
  covers both.
- Every native surface offered an sRGB format first, and the prototype
  took it. ADR-0136 §10 takes a non-sRGB one.
- **Rendering offscreen and reading back** worked on all three, with no
  window.
  - DX12's and Vulkan's pictures were byte for byte the same.
  - GL's differed from them in 29% of the pixels, by at most one step
    per channel: within the raster's tolerance (ADR-0136 §10).
- **Licenses:** `cargo deny check licenses` with the repository's
  `deny.toml` passed on every target (Windows, Linux with X11 and
  Wayland, both `.wasm`s), and so did the macroquad build. The licenses
  beyond MIT and Apache-2.0 were Zlib, BSD-2-Clause, ISC, Unicode-3.0,
  and on Linux Apache-2.0 WITH LLVM-exception and Unlicense, all on the
  allow list.
- **Dependencies:**
  - wgpu and winit: 85 packages on Windows, 105 on Linux, and 49 or 72
    in the `.wasm` (WebGPU only, or with WebGL 2);
  - macroquad: 29 to 31;
  - winit 0.30 brings `windows-sys` 0.52 beside wgpu's `windows` 0.62.

**In a browser.** The page ran in the desktop app's built-in browser
pane (Chromium), served from `localhost`.

| Path | Adapter | `TIMESTAMP_QUERY` | Inside encoders and passes | Surface format | Start (ms) |
| ---- | ------- | ----------------- | -------------------------- | -------------- | ---------: |
| WebGPU | (the browser gives no name) | yes | no | `Bgra8Unorm` | 532 |
| WebGL 2, the same `.wasm` with `navigator.gpu` hidden | ANGLE on Direct3D 11, Intel | no | no | `Rgba8UnormSrgb` only | 8,268 |

- **One `.wasm` chooses between them at run time**, as wgpu's changelog
  says.
- **WebGL 2 started slowly:** 6.7 of its 8.3 s went to its one pipeline
  and its buffers, through ANGLE. The pane was hidden, so this is a
  ceiling; 3330 measures it on a shown page.
- **WebGL 2 offered only an sRGB surface,** so 3325 keeps the raster's
  bytes there with a last pass (ADR-0136 §10).
- **The surface stayed 1 × 1** when nothing set the canvas's size, which
  answers the question winit's docs leave open. The backend sets it
  (3330).

**macroquad, through its public API** (`macroquad::telemetry`), on the
same 7,400 quads:

| Order of the draws | Draw calls | Frame interval (ms) |
| ------------------ | ---------: | ------------------: |
| a fill, then a glyph, cell by cell | 7,200 | 78.4 |
| every fill, then every glyph (`render.rs`'s order) | 10 | 4.3 |
| one texture only | 9 | 4.4 |

- macroquad starts a new draw call each time a solid rectangle follows a
  textured quad. `render.rs` already draws all the fills before the
  glyphs, which is why the engine's frames stay at a few calls.
- `shuyi-gpu`'s batcher draws fills from the font atlas's white texel
  (ADR-0136 §2), so the order matters less there.

## The documents used

- **wgpu:**
  - crates.io's API for `wgpu`, `naga` and `bevy_render`;
  - the published 30.0.2 crates of `wgpu`, `wgpu-types`, `wgpu-hal`,
    `wgpu-core` and `naga`;
  - <https://docs.rs/wgpu/30.0.2/wgpu/> and its features page;
  - <https://github.com/gfx-rs/wgpu>: README, CHANGELOG, `docs/testing.md`,
    the CI workflow and its Mesa and WARP actions, the v30 examples
    (`standalone/02_hello_window`, `render_to_texture`), and the wiki;
  - its issues 7518, 8303, 4600, 9075, 9713, 9742 and 9905, and PR 8430;
  - Bevy's PRs 24841 and 23277.
- **WGSL and WebGPU:**
  - <https://www.w3.org/TR/2026/CRD-WGSL-20261106/>;
  - <https://www.w3.org/TR/2026/CRD-webgpu-20261116/>;
  - <https://w3c.github.io/hr-time/>;
  - <https://github.com/gpuweb/gpuweb/wiki/Implementation-Status>;
  - MDN's WebGPU API page;
  - caniuse's `webgpu` data;
  - <https://developer.chrome.com/blog/new-in-webgpu-121>.
- **winit:**
  - <https://docs.rs/winit/0.30.13/winit/> (the application, event loop,
    window, monitor, event and platform modules, the 0.29 and 0.30
    changelogs);
  - <https://docs.rs/winit/0.31.0-beta.4/winit/>;
  - <https://github.com/rust-windowing/winit>: README, FEATURES.md,
    releases, milestones, issues 2494, 4424 and 1823, PRs 4428 and 4451,
    and the 0.30.13 X11, xkb and web sources for which libraries load and
    how the web reports visibility;
  - crates.io's API for `winit`, `bevy_winit`, `x11-dl`, `xkbcommon-dl`,
    `x11rb`, `wayland-sys`, `sctk-adwaita`, `android-activity` and
    `accesskit_winit`.
- **The web:**
  - <https://wasm-bindgen.github.io/wasm-bindgen/> (the CLI reference);
  - wasm-bindgen's README, CHANGELOG and 0.2.130 `cli-support` source
    for `init`;
  - <https://docs.rs/web-sys/0.3.107/web_sys/> (`console`, `Storage`,
    `Gamepad`, `PageTransitionEvent`, `Window`, `Document`, `Navigator`,
    `VisibilityState`);
  - Binaryen's license and releases.
- **Text:** crates.io's API and READMEs for `parley`, `cosmic-text`,
  `swash`, `harfrust`, `rustybuzz`, `skrifa`, `fontdue`, `ab_glyph` and
  `bevy_text`.
- **macroquad:** only its own README and docs.rs pages. Its source and
  miniquad's were not read (ADR-0021 §2).
