---
id: "3330"
title: "Parked: The web build on wasm-bindgen with the winit backend: cargo shuyi web with a pinned wasm-bindgen-cli, the page and the shell's start, WebGPU then WebGL 2, saves and the console through web-sys, and the web-render job on both backends"
type: feature
milestone: "Engine: in-house platform"
model: opus-5.5 / gpt-6.1-sol
effort: xhigh
status: blocked
blocked_by: ["3327"]
nick_input: none
completed:
---

# 3330 — The web build on wasm-bindgen

> **Parked** (Nick, 2026-11-22, `docs/decisions.md`): written by research 3034, whose research alone he unparked. The build waits for the MVP with the rest of the in-house platform layer (2026-10-19), and for 3079 through 3327.

## Context

wgpu and winit reach the browser through `wasm-bindgen` and `web-sys`.
So the web build leaves macroquad's loader (`mq_js_bundle.js`), and the
rule against `wasm-bindgen` (game ADR-0034's web part, `AGENTS.md`
*Environment*) goes with it.

[ADR-0136](../../docs/adr/0136-replacing-macroquad-winit-wgpu-and-wasm-bindgen.md)
§13 says how. This ticket builds and runs a game on the web with the
winit backend, behind the `winit` feature, and keeps 2019's `web-render`
job green on both backends. It moves saves and the console here;
3331 moves the other plugins.

## Nick input

None.

## Scope

**In:**
- **The winit backend on wasm32:**
  - `spawn_app`, or `run_app` on 0.31;
  - the canvas made by winit and placed in the page's `<canvas
    id="glcanvas">` (`WindowAttributesExtWebSys::with_canvas`), sized to
    its CSS size times `devicePixelRatio`;
  - `RedrawRequested` on each animation frame, with `Pacer::web_frame`
    as today;
  - the GPU's start, the voice manifest and the sound's start through
    `wasm_bindgen_futures::spawn_local`, into a slot the loop reads each
    frame;
  - no `App::frame` before the GPU is ready. On WebGL 2 the research's
    prototype took about 8 seconds to start in a hidden pane (6.7 of
    them making its one pipeline and its buffers, through ANGLE): measure
    the start on a shown page in Chrome and Firefox, write it in the
    notes, and make no pipeline the first frame doesn't need;
  - the canvas's drawing size set by the backend from its CSS size times
    `devicePixelRatio`, and again on each resize: the prototype's
    surface was 1 × 1 when nothing set it;
  - WebGPU when it gives an adapter, else WebGL 2 (ADR-0136 §3), and the
    shell's error state when neither does.
- **The build** (`crates/cli/src/web.rs`), when the game's `shuyi-app`
  has `winit` on (read from `cargo metadata`):
  - build for `wasm32-unknown-unknown` with `default-features = false`
    for `shuyi-app` (macroquad's exports must not be in the `.wasm`,
    ADR-0136 §12);
  - then `wasm-bindgen --target web --no-typescript --out-name <bin>`
    into the staging folder.
  - wasm-bindgen warns on a stripped `.wasm` (it can't read its
    features). The build keeps the `.wasm` unstripped for it, and
    passes `--remove-name-section`, which the research's prototype found
    as small.
  - The version is the one in the game's `Cargo.lock`. A missing CLI,
    or one of another version, fails with the command to run (`cargo
    install --locked wasm-bindgen-cli --version <v>`) as a problem code
    of its own (ADR-0028), and `shuyi doctor` checks the same.
  - `wasm-bindgen` is pinned exactly (`=`) in the workspace, and the
    template's game gets the same pin.
- **The page:** a second shell, `crates/cli/web/winit/index.html`, that
  imports the glue and calls `init({ module_or_path })`.
  - **Either order with 3052:** if 3052 is done, the start goes through
    its `shell.js`: it fetches the `.wasm` itself, counting bytes for its
    progress, and passes the `Response` to `init`, which streams it, and
    its `data-shuyi` states keep working, the GPU's error among them,
    with a test in `tools/web-check`; if not, nothing to do (3052 wraps
    this page's start too).
  - The pack's streaming (`shuyi_fetch_pack`) still starts before the
    `.wasm`, as today.
  - The focus and the keys inside a site's frame (3505) keep working.
- **Saves:** `storage.js` becomes `web-sys`'s `Storage`
  (`window.localStorage`), the same keys and values, under `winit`.
  `storage::conformance_suite` passes on it in the browser test.
- **The log's console sink** (3041) goes through `web-sys`'s `console`.
  Its seconds come from `performance.now()`.
- **The cursor image.** **Either order with 3328:** if 3328 is done,
  set its cursor image on the web too, with `CustomCursor::from_rgba`;
  if not, nothing to do (3328 sets it on every platform that has the
  backend then).
- **CI's `web-render` job** builds the fixtures and `hello-grid` a second
  time with `winit` and runs `tools/web-check/check.py` on them:
  - WebGL 2 on SwiftShader, as now;
  - WebGPU where the job's Chromium starts it (`--enable-unsafe-webgpu`
    on SwiftShader's Vulkan), or the job's notes saying why not.
  - Each compares the canvas with the raster and fails on a console
    error.

**Out (do not do):**
- The other plugins: controllers, sound output, the pack's chunks into
  the game, the page's files, the player's files and the lifecycle
  (3331). Until it lands, a winit web build has no sound and no
  controllers, which is fine behind a feature no game turns on.
- `wasm-opt`.
- Making `winit` the default (3332).

**Estimated size:** about 1,600 lines (code 950, tests 400, docs 250).

**Split:** past 1,000 lines, split off the saves and the console sink
(they can stay on the macroquad path a ticket longer) into a new ticket.

## Implementation steps

1. Read ADR-0136 §3, §12 and §13, game ADR-0034, `crates/cli/web/README.md`,
   `crates/cli/src/web.rs`, 2019's `tools/web-check/check.py`,
   wasm-bindgen's CLI reference and winit's `platform::web` docs.
2. The backend's web loop and its async start.
3. `web.rs`: the feature's detection, the build, the CLI's version
   check, the problem code and its page in `crates/cli/errors/`,
   `doctor`'s check.
4. The page; the clause with 3052; the pack's start.
5. Saves and the console; the cursor clause.
6. The `web-render` job's steps.
7. The docs: `crates/cli/web/README.md`, the book's chapter on the web
   build (what to install), `docs/platform-backends.md`'s table.

## Acceptance criteria

- [ ] `cargo shuyi web` on `hello-grid` with `winit` builds a page that
      plays in Chrome on WebGPU and in Firefox on WebGL 2 (manual, in
      the notes, after merging `main`).
- [ ] A missing or wrong `wasm-bindgen-cli` fails with its problem code
      and the command to install the right one (a test).
- [ ] The `web-render` job passes on both backends.
- [ ] Saves written by the macroquad build read back under `winit` (the
      same `localStorage` keys), by a browser test.
- [ ] The book's chapter is updated (2802), or the completion notes have
      a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the feature's detection from metadata; the version check.
- Browser (`tools/web-check`): the canvas against the raster on both
  backends; storage's conformance; no console error.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
