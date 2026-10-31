---
id: "3034"
title: "Parked: Research: replacing macroquad with winit for the window and input, and our own 2D renderer over wgpu (Vulkan, Metal, Direct3D 12, OpenGL and GLES, WebGL 2, WebGPU), and the tickets to build it"
type: research
milestone: "Engine: in-house platform"
model: reasoning
effort: xhigh
status: blocked
blocked_by: ["3003", "2018", "2019"]
nick_input: none
completed:
---

# 3034 — Research: replacing macroquad

> **Parked** (Nick, 2026-10-19, chose "Park until MVP" for the in-house platform layer, `docs/decisions.md`): after the engine reaches parity with the game (the MVP in `docs/ROADMAP.md`), and after sound, controllers, web saves and file embedding (Nick's order; it needs only 3003, so the order is the roadmap's, not a blocker). Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Nick, 2026-10-19 (`docs/decisions.md`): "eventually - macroquad replace",
and "we should have a wide compat with all GFX libs". Once sound,
controllers, web saves and file embedding are replaced (2902, 2903, 3033,
2215), macroquad does four jobs in `shuyi-app`:

- the window, the run loop and the system's events;
- keyboard, mouse and text input (`crates/app/src/keys.rs`);
- drawing the frame (`crates/app/src/render.rs`, which paints the
  engine's plain-data frame);
- the web loader (`crates/cli/web/mq_js_bundle.js`, vendored), which the
  web build uses instead of `wasm-bindgen` (game ADR-0034).

miniquad, under macroquad, draws with OpenGL 2 and 3, GLES 2 and 3, WebGL 1
and Metal, and its README rules out Vulkan and DirectX 12 (checked
2026-10-19). Nick wants every graphics API, and he doesn't want us writing
a backend per API: the graphics go through `wgpu`, and our own 2D renderer
sits on top of it ("I don't wanna rewrite graphics bindings... so I think
wgpu is safe", `docs/decisions.md`). The window goes through `winit`,
as in Bevy ("we can use winit since it's not really something I wanna
reimplement").

[ADR-0021](../../docs/adr/0021-the-platform-layer-in-house-clean-room-pieces-in-their-own-crates.md)
sets the rules: wgpu for graphics and winit for the window and its input
(§1); what we write on top is written from specifications and
documentation (§2), in crates of their own (§4). **From 3003 (done,
ADR-0024):** the replacement implements `Platform`
(`crates/platform/src/platform.rs`) and is driven by `App::frame`.
2018 and 2019 (the desktop window and the web build compared with the
raster in CI) must be done first: they are the net the switch is
checked against.

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
- **The 2D renderer over wgpu.** Decide, and record in an ADR:
  - the renderer the engine's frame needs on top of wgpu: textures and
    atlases, one or two pipelines, batching, the skins' render targets,
    and the WGSL shaders;
  - which wgpu backend each platform asks for, and the fallback (for
    example Vulkan, then GL, on Linux and the Steam Deck; WebGPU, then
    WebGL 2, in the browser), and what happens on a machine where none
    starts;
  - which wgpu version to pin, how the engine moves to the next one (wgpu
    makes a breaking release about every three months), and which of its
    default features to turn off;
  - the build size: wgpu's added size, natively and in the `.wasm`, against
    today's macroquad build;
  - the 3D plans (ADR-0011, the GPU painter in 3104) and whether the
    renderer must leave room for them now. **Either order with 3104:**
    if 3104 is done, the plan has a ticket that moves `shuyi-app-3d`
    (the `GpuPainter` hook in `shuyi-app`, its passes and its look test
    in 2018's `render` job) off miniquad, before the one that removes
    macroquad; if not, the ADR says how 3104 is written over the new
    layer, as 3104 asks;
  - room for traced light (2911, parked research): wgpu's ray-query
    feature and compute passes. **Either order with 2911:** if it is
    done, its ADR and the line it added to this ticket say what traced
    light needs from the renderer; leave room for it. If not, record
    wgpu's ray-query status per backend in the report and the ADR, for
    2911 to build on;
  - room for the 2D effects that wait on this renderer: a chain of
    screen passes (2326), a game's own WGSL passes and sprite materials
    (2327), 2D lights with normal maps (2328), and the render graph
    that composes them as passes (2331). When this research writes the
    renderer's ticket, add its id to the `blocked_by` of 2326 and 2331,
    and the winit backend's ticket to the `blocked_by` of 3035
    (screen reader output through AccessKit, which joins a winit
    window).
  - GPU frame times for 2325's profiler (2382): what the renderer's ADR
    must leave room for, and whether wgpu's timestamp queries give them
    on each backend (a row per backend in the report, which 2382's
    table in `docs/platform-backends.md` starts from). When this
    research writes the renderer's ticket, add its id to the
    `blocked_by` of 2382 too, and the id of the ticket whose backend
    first runs the renderer in a window, if that is another one.
  - what the frame already asks of every renderer (2337):
    `Paint::Tinted` as the draw colour with no copy of the image,
    overlay opacity, and `Blend::Add` and `Blend::Multiply` as blend
    states, within one step per channel of `shuyi_ui::frame::blend`'s
    integer rules;
  - text: whether the renderer draws only glyphs from atlases that the
    engine lays out (the cell font, and 2224's baked fonts in 2334's
    runs), or also shapes text at run time through a library (for
    example `parley` or `cosmic-text`), and what each adds to the
    `.wasm`. **Either order with 2908:** if it is done, its ADR says
    which scripts need run-time shaping; if not, the report records the
    options for 2908 to use.
- **The window and input over winit.** Decide, and record in the ADR:
  - the run loop: how winit's event loop drives `App::frame` (3003) on
    desktop and the web;
  - **From 2511 (done):** the short path's entry,
    `shuyi_app::run_game(game)` (ADR-0036 §7, also
    `shuyi::prelude::run_game`), calls macroquad's `#[doc(hidden)]`
    `Window::from_config`, and a long-form game's `main` is
    `#[macroquad::main]`. The plan gives the new backend a `run_game`
    of the same signature, so a short-path game changes nothing at the
    switch, and says how a long-form game's entry moves;
  - keyboard and mouse to the engine's `RawInputEvent`s
    (`crates/app/src/keys.rs` today);
  - text input: the IME as winit gives it (its composing text with a
    caret, then the committed text) as `RawInputEvent`s, the name box
    drawing the composing text underlined, and the IME turned on and
    placed from `Frame::typing` and the buffer's text caret (2346) on
    Windows, macOS, Linux and the web (miniquad 0.4.11 does it on
    Windows only). 1149's chat box is built on the name box's typing
    and takes its composing text from these tickets (**either order
    with 1149:** if 1149 is done, the ticket that draws the composing
    text draws it in the chat box too; if not, that ticket draws it in
    every text box built on the name box's typing, as 1149's will be);
  - the system keyboard (3003's `TextInput`);
  - high-DPI, full screen and the Steam Deck's screen;
  - suspend and resume (3003's `SystemEvent`), and focus and visibility
    as 3042's ADR maps them: `WindowEvent::Focused` to `FocusLost` and
    `FocusGained`, and `WindowEvent::Occluded` to `Suspend` and
    `Resume` where it is reliable, which miniquad 0.4.11 can't give;
  - the window facts miniquad 0.4.11 lacks, which 3044 leaves out:
    opening at the largest whole multiple of the console that fits the
    monitor's work area; putting a saved window place back only on a
    connected monitor (3044 keeps the size and mode across launches,
    not the place); a minimum window size; the maximized state;
    choosing a monitor; exclusive fullscreen; custom cursor images
    (miniquad 0.4.11 has only the system's cursors); and leaving
    fullscreen on Linux (miniquad's X11 code has a TODO saying it can't
    go back to windowed). The winit backend keeps 3044's window record
    and cursor rule (`docs/platform-backends.md`, *The window and the
    cursor (desktop)*);
  - the cursor grab, which miniquad 0.4.11 already has
    (`set_cursor_grab`: the cursor held inside the window on desktop,
    pointer lock on the web), kept through winit's own grab.
    **Either order with 3106:** if 3106 is done, its pointer lock in
    `shuyi-app` moves to the winit backend with its tests green; if
    not, the ADR says how a game asks for the grab, and 3106 uses that;
  - which winit version to pin, and how the engine moves to the next
    one.
- **The web build moves to `wasm-bindgen`.** wgpu reaches the browser
  through `wasm-bindgen` and `web-sys`, so macroquad's loader
  (`mq_js_bundle.js`) and the rule against `wasm-bindgen` (game ADR-0034,
  `AGENTS.md` *Environment*) go with macroquad. Plan:
  - how `cargo shuyi web` builds with it (`wasm-bindgen-cli` at a pinned
    version, installed with `cargo install --locked`, matching the crate's
    version);
  - how our plugins (`gamepad.js`, 3043's `lifecycle.js`, 3031's
    `sound.js`, 3033's `storage.js`) move over: each becomes `web-sys`
    calls or a small `wasm-bindgen` import, whichever is less code.
    `lifecycle.js` runs a frame inside the browser's `visibilitychange`
    and `pagehide` events with macroquad's `wasm_exports.frame()` (or
    the export 3043 chose in its place); the plan says how the new web
    build runs the game's save inside those events. 3052's `shell.js`
    (the loading screen and errors) is not a plugin: it wraps
    `window.fetch`, `WebAssembly.instantiate` and
    `WebAssembly.instantiateStreaming` from outside the loader, and
    `index.html` starts it before `load(...)`; the plan says how the
    new loader's start goes through it, and that its `data-shuyi` state
    keeps working;
  - **Either order with 2909** (video playback research): if 2909 is
    done, its report plans the video on the macroquad path (the
    browser's `<video>` through a miniquad JS plugin of ours) and names
    what changes under this plan: move that plugin over with the ones
    above, and plan the video through `web-sys` and wgpu's textures. If
    not, nothing to do here: 2909 covers the video on this plan's web
    build itself.
  - **Either order with 2601's web backend** (parked; a follow-up
    ticket 2601 writes): if it is done, its `http.js` (and
    `crates/app/src/http/web.rs`) is one more plugin to move, and the
    plan says whether it becomes a crate of its own (2601 leaves that
    to the tickets this writes); if not, nothing to plan for it: the
    ticket that switches the web build moves every plugin
    `crates/cli/web` holds by then;
  - the log sinks (3041) keep working without macroquad: the browser
    console sink reaches `console.error` to `console.debug` through
    miniquad's macros today, and moves to `web-sys`'s `console` or a
    small import of ours; macroquad's `log-rs` feature goes with
    macroquad; the file and stderr sinks stay in the new backend, with
    the seconds on each line read from its clock in place of
    `miniquad::date::now()`;
  - the ADR supersedes the web part of game ADR-0034 for the engine, and
    the PR that switches the web build updates `AGENTS.md`.
- **Linux's system libraries.** List in the report the libraries winit
  and wgpu link and open on Linux (Wayland, X11, Vulkan's loader, GL).
  The tickets this research writes change doctor's Linux table
  (`crates/cli/src/contract/doctor/linux.rs`) and
  `scripts/setup-linux.sh` (2513) to match, in the PR that removes
  macroquad.
  **Either order with 3071:** if 3071 is done, that PR also checks
  `crates/cli/export/libraries.ron` (3071's library audit) for every
  target against what winit and wgpu load, and updates it; if not, the
  ticket that removes macroquad says the same in a clause with 3071,
  and 3071 names it back.
- **Android and iOS later** (ADR-0008): what the design must not rule out.
- **The order of the build**, so the game keeps working at every step:
  for example, a new backend crate beside `shuyi-app` behind a feature,
  one platform at a time, then the switch, as sound and controllers did.
- **The regression net.** The new backend keeps 2018's capture mode (a
  `capture` feature, `shuyi_platform::capture` plans,
  `raster::compare`), and it passes 2018's `render` and 2019's
  `web-render` jobs before the switch. Each ticket this research writes
  says how its piece keeps those jobs green.
- **Outputs:**
  - a report, `docs/research/3034-replacing-macroquad.md`, with the
    measurements and the options weighed;
  - an ADR (the `write-adr` skill) with the decisions above;
  - `Parked:` tickets, each one PR, for every piece, with `blocked_by`
    in order, and their rows in `docs/ROADMAP.md`'s *Parked* section.

**Out (do not do):**
- Building any of it.
- Writing graphics backends of our own, per API (Nick, 2026-10-19).
- Writing a window layer of our own (Nick, 2026-10-19: winit).
- Reading the source of miniquad or macroquad to copy how they draw
  (ADR-0021 §2). wgpu and winit are used, so their documentation and
  examples are fair to read (§2).

## Implementation steps

1. Measure the frame (scope, first bullet) in the game at this engine's
   revision, and write the numbers in the report.
2. Read what the macroquad backend does today that the new one keeps,
   and note which of these tickets are done: 2018's and 2019's capture
   mode and jobs, 2337's blend rules, 2346's IME calls, 3041's sinks,
   3042's ADR, 3043's `lifecycle.js`, 3044's window record and cursor
   rule, 3052's `shell.js`, 2513's Linux table, 2908's ADR, 3104's
   `shuyi-app-3d` and 3106's pointer lock.
3. Read wgpu's and winit's documentation and examples, and the WGSL
   specification, and list what was used in the report.
4. Prototype only what the decision needs, in a throwaway branch or
   scratch folder: for example the engine's frame painted through wgpu on
   each backend, natively and in a browser, to check the renderer's shape
   and the build size. Delete it after, and say
   what was learned.
5. Write the ADR, then the tickets, then the roadmap rows.

## Acceptance criteria

- [ ] The report has the frame measurements and the list of documents
      used.
- [ ] The ADR decides every bullet in the *2D renderer*, *window* and *web build*
      parts of the scope.
- [ ] Every ticket written is one PR's worth, `Parked:`, with its
      `blocked_by` chain, and passes the ticket lint.
- [ ] The tickets say how each piece keeps 2018's `render` and 2019's
      `web-render` jobs green, and the one that removes macroquad
      changes doctor's Linux table, `scripts/setup-linux.sh` and 3071's
      `libraries.ron` (by a clause with 3071 if 3071 is still open).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- None shipped (research). The tickets it writes say their tests.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
