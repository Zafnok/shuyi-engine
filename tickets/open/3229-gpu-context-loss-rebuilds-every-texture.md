---
id: "3229"
title: Losing the GPU context or surface rebuilds every texture from its source
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3229 — Losing the GPU context or surface rebuilds every texture from its source

## Context

A browser can take a page's WebGL context away at any time: a phone
browser with the tab in the background, a GPU driver reset, too many
contexts open at once. The page gets `webglcontextlost`, and later
`webglcontextrestored` only if it called `preventDefault` on the first.
After a restore every GL object is gone: textures, buffers, shaders,
programs, uniform locations, vertex arrays and framebuffers.

Nothing in the engine handles this today. A grep of `crates/`, `docs/`
and `tickets/` for `webglcontextlost`, `context lost`, `device lost`
and `surface lost` finds no plan (the blind-spot audit of 2026-11-22,
checked again for this ticket). `crates/cli/web/mq_js_bundle.js`
(macroquad's loader, vendored) never listens for either event. So a
lost context leaves a black or frozen canvas, and the game can't come
back without a reload, which loses whatever the player hadn't saved.

What the renderer holds, and why it can't simply start over:

- `Renderer` (`crates/app/src/render.rs`) keeps the font atlas texture,
  `images: ImageTextures<Texture2D>`, `recoloured` copies and
  `painted: PaintedTextures<Texture2D>`. `Renderer::with_files` uploads
  the atlas PNG and then drops its bytes, so the atlas has no source to
  rebuild from.
- `ImageTextures` (`crates/app/src/image_textures.rs`, ADR-0128 §5,
  ticket 3317) makes each texture the first frame a sprite draws it and
  keeps it for the run. It also keeps each texture's file in `sources`,
  so it can make them again.
- `PaintedTextures` (`crates/app/src/painted.rs`, ADR-0078) uploads a
  picture only in a frame whose revision or size differs from the last
  upload. After a loss, a picture whose revision hasn't moved would
  never be uploaded again.
- `MacroquadBackend` (`crates/app/src/backend.rs`) keeps the loading
  card's picture as `card_picture: Option<(String, Option<Texture2D>)>`.
- macroquad itself keeps GL objects we never see: its batcher's
  shaders, buffers and its own textures. They live in the bundle's `GL`
  tables (`GL.textures`, `GL.buffers`, `GL.programs`, `GL.shaders`,
  `GL.uniforms`, `GL.vaos`, `GL.framebuffers`, `GL.renderbuffers`), by
  id. Rust holds only the ids.

`crates/cli/web/README.md` says the vendored files stay as vendored. So
the web part is a plugin of the engine's own, like `storage.js` and
`gamepad.js`. In `register_plugin(importObject)` it wraps the bundle's
`gl*` imports to record what it needs, and on a restore it makes every
object again under the same id. The Rust part is a backend-neutral
"forget every texture" path. The macroquad research (3034, PR #532)
writes in its ADR-0136 §15 that the wgpu backend makes textures and
pipelines again after a surface is lost. That backend can call the same
path when wgpu reports a lost surface or device.

Tests read pixels: the canvas after a loss and a restore must equal the
canvas before it. The render card (ticket 2392, ADR-0052 §6) draws every
kind of thing a frame draws, and on the web it keeps showing the card
after `shuyi-capture: done` (`crates/app/examples/render_card.rs`). CI's
`web-render` job (ticket 2019) already builds it for the browser and
runs `tools/web-check/check.py capture` on it. Chromium's
`WEBGL_lose_context` extension (`loseContext()`, `restoreContext()`)
makes a real loss and restore on demand, under SwiftShader as on a GPU.

**From 2222 (done):** an atlas's pages are painted pictures of the
frame (ADR-0153, `shuyi_ui::frame::atlas`), uploaded with the other
painted pictures (`PaintedTextures`, `crates/app/src/painted.rs`): the
forget path drops them with those, and they are uploaded again from the
frame's pixels on the next frame. Nothing to pack again.

**Either order with 2337:** if 2337 is done, its two materials are
programs like macroquad's own, so the plugin replays them with the rest,
and the render card's blended items are part of the canvas check; if
not, nothing to do.

**Either order with 3507:** if 3507 is done, `check.py lose` takes
`--browser` as the other modes do, and the `web-browsers` job runs
`lose` on the render card's page and fails its copy without the plugin,
as `web-render` does; if not, nothing to do (3507 makes every
`check.py` mode take `--browser` and runs them in `web-browsers`).

## Nick input

None.

## Scope

**In:**
- `Renderer::forget_textures(&mut self)` in `crates/app/src/render.rs`:
  drops every texture `Renderer` owns, keeps every source, and lets the
  next frame make each texture again as it is drawn. It covers the font
  atlas (its PNG is kept now), `images`, `recoloured`, `painted` and the
  backend's `card_picture`.
- `ImageTextures::forget_all` and `PaintedTextures::forget_all`. After
  `forget_all`, `ImageTextures::texture` makes a texture from its kept
  source first, and goes to `read` only for an image it never had. That
  way a picture saved under `--watch` comes back as saved, not as first
  read.
- `crates/cli/web/gpu-context.js`, a miniquad plugin of the engine's own:
  - It listens for `webglcontextlost` on `#glcanvas`, calls
    `preventDefault`, and marks the context lost.
  - It records, by id, what each GL object needs to be made again:
    - each shader's type and source;
    - each program's shaders, attribute bindings and the uniform names
      looked up;
    - each buffer's target, size and usage;
    - each texture's target, size, format and parameters;
    - each vertex array, framebuffer and renderbuffer's setup.
  - It keeps the pixels of a texture only when its last full upload was
    4,096 bytes or less (macroquad's own small textures, such as its
    white one). `forget_textures` makes the rest again from their
    sources.
  - On `webglcontextrestored` it makes every recorded object again in
    the bundle's `GL` tables under the same ids, and looks up the
    uniform locations again. Then it marks the context restored.
- The Rust side of the plugin in `shuyi-app`, for `wasm32` only. It
  follows the shape of `gamepad.js`'s Rust side: one import that says
  whether the context is lost, and whether it came back since the last
  call. While the context is lost the backend skips drawing (WebGL calls
  on a lost context do nothing, but no texture is made). On the frame it
  comes back, the backend calls `forget_textures` before it draws.
- The shell ships the plugin: a row in `SHELL` and `SHELL_NOTICES`
  (`Origin::Engine`) in `crates/cli/src/web.rs`, its `<script>` in
  `crates/cli/web/index.html` after `mq_js_bundle.js`, and the file in
  the list in `crates/cli/src/export/tests.rs`.
- `check.py lose <dir>`, a mode in `tools/web-check/check.py`. It waits
  for `shuyi-capture: done` and saves the canvas. Then it calls
  `loseContext()`, waits for the plugin's console line
  `shuyi-gpu: context lost`, calls `restoreContext()`, waits for
  `shuyi-gpu: context restored` and some frames, and saves the canvas
  again. It fails on a console or page error, on a missing line, or
  unless the two canvases are equal pixel for pixel.
- A step in CI's `web-render` job that runs `lose` on the render card's
  page it already builds.
- A copy of the render card's page without the plugin's `<script>`,
  which `lose` must fail, as the job already does for a copy of Hello,
  Grid's page without `var register_plugin;`.
- Docs: `crates/cli/web/README.md` names the file and says what it
  records and why. A short ADR (the `write-adr` skill) records the
  pattern: an engine plugin that wraps the vendored bundle's imports
  instead of editing the bundle.
- No public API changes: `forget_textures` and `forget_all` are
  `pub(crate)`. The new mode and the new shell file add nothing a game
  calls.

**Out (do not do):**
- Editing `mq_js_bundle.js` or `sapp_jsutils.js`: they stay as vendored.
- A loss on the desktop under miniquad. miniquad reports no lost GL
  context there, and a Windows driver reset is rare. Android and iOS
  have no backend yet.
- wgpu's lost surface and lost device. ADR-0136 §15 (PR #532) asks that
  of the wgpu backend, and this ticket only gives it `forget_textures`.
- Saving or reloading the page on a loss. The game keeps running,
  because the simulation never touches the GPU.
- Freeing textures under memory pressure (ADR-0128 §7 keeps them for the
  run).

**Estimated size:** about 1,200 lines (code 700, tests 350, docs 150).

**Split:** past 1,000 lines, split off steps 5–6 (the plugin's replay of
macroquad's own objects) into a new ticket. This PR then ships the
forget path, the plugin's loss and restore events, a skipped draw while
lost, and a console line on restore. The canvas check of the acceptance
moves to the new ticket.

## Implementation steps

1. In `render.rs`, keep the atlas PNG in `Renderer` (a `Cow<'static,
   [u8]>` or the decoded `Image`). Let `remake`'s `Remake::Font` branch
   replace it too, so a font saved under `--watch` is the one rebuilt.
2. Add `ImageTextures::forget_all`. It clears `textures` and keeps
   `sources` and `undecodable`, and `texture` reads `sources` before
   `read`. Add `PaintedTextures::forget_all`, which clears `textures`
   (the next `texture` call finds no entry, so it is stale and made).
   Keep the test builds' upload counters counting.
3. Add `Renderer::forget_textures`. It remakes the atlas texture from
   the kept PNG, calls both `forget_all`s, clears `recoloured`, and
   leaves `warned`, `warned_images` and `remade` alone. In `backend.rs`,
   a `forget_textures` on the backend calls it and clears the texture in
   `card_picture`, so the next frame decodes the card again.
4. Write `gpu-context.js`'s loss and restore listeners, its console
   lines (`shuyi-gpu: context lost`, `shuyi-gpu: context restored`), and
   the import Rust polls. Add the `wasm32` Rust side and call it once a
   frame in the backend's draw.
5. In `register_plugin`, wrap the bundle's `importObject.env` entries
   that make or set up objects: `glCreateShader`, `glShaderSource`,
   `glCreateProgram`, `glAttachShader`, `glBindAttribLocation`,
   `glLinkProgram`, `glGetUniformLocation`, `glGenBuffers`,
   `glBufferData`, `glGenTextures`, `glTexImage2D`, `glTexSubImage2D`,
   `glTexParameteri`, `glGenVertexArrays`, `glGenFramebuffers`,
   `glFramebufferTexture2D`, `glGenRenderbuffers`,
   `glRenderbufferStorage`, and each matching `glDelete*`. Confirm first
   that a plugin's `register_plugin` runs after the bundle has filled
   `env`. If it runs before, wrap the functions at the first frame
   instead, and say so in the README.
6. Write the replay. For each recorded object, in dependency order
   (shaders, programs and their uniforms, buffers, textures with kept
   pixels, vertex arrays, renderbuffers, framebuffers), make it with
   `gl` and store it under its old id in the `GL` table. Read the
   bundle's `glGetUniformLocation` and `GL.programInfos` to see how
   uniform ids map, and rebuild that map. The restored context is the
   same `gl` object, so `gl` itself is not replaced.
7. Ship the file (`web.rs`, `index.html`, `export/tests.rs`).
8. Add the `lose` mode and the CI steps (the page and its copy without
   the plugin). Run it by hand on
   Hello, Grid's page as well, and say in the completion notes what it
   showed.
9. Write the README section and the ADR.

## Acceptance criteria

- [ ] `PaintedTextures`: after `forget_all`, a frame that draws a picture
      whose revision hasn't moved uploads it once, and the next frame
      uploads nothing.
- [ ] `ImageTextures`: after `forget_all`, each image drawn is made again
      from its kept source without calling `read`. An undecodable image
      stays given up. An image replaced under `--watch` comes back with
      the replaced bytes.
- [ ] `Renderer::forget_textures` drops and remakes the atlas texture
      from the kept PNG, and after a `--watch` font save it remakes the
      new one.
- [ ] `check.py lose` on the render card's web page passes: the canvas
      after a loss and a restore equals the canvas before it, pixel for
      pixel, with no console or page error.
- [ ] `check.py lose` fails on the copy of the page without the plugin,
      and names why.
- [ ] While the context is lost the game keeps stepping. No Rust panic
      and no console error is logged.
- [ ] `mq_js_bundle.js` and `sapp_jsutils.js` are unchanged
      (`git diff --stat` shows neither).
- [ ] The `web-render` job runs the new step and uploads its canvases.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `forget_all` on both caches with the existing fake textures
  (`Fake` in `painted.rs`, the byte vectors in `image_textures.rs`):
  upload counts before and after, sources used rather than `read`, and
  `undecodable` kept.
- Unit: `render.rs` has no tests today, and `Renderer` uploads through
  macroquad, so it can't be built without a window. Split what
  `forget_textures` decides (what is dropped, what is kept, the atlas
  bytes after a `--watch` save) into a function over the caches' generic
  `T`, and test that.
- Integration: `check.py lose` on the render card in CI, and the copy
  without the plugin that must fail.
- Snapshot: none. The canvas comparison is the check.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
