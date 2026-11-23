---
id: "3341"
title: The renderer counts draw calls, texture switches and uploads, and CI gates them on a stress scenario
type: infra
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3341 — The renderer counts draw calls, texture switches and uploads, and CI gates them on a stress scenario

## Context

Found by the blind-spot audit of 2026-11-23 (performance, "the painting
backend is outside every gate"). Checked against the code that day.

What is measured today, and what isn't:

- ADR-0008 §6 gates performance on numbers that come out the same on
  every run: instructions per frame, the heap's peak and the package's
  size. `shuyi bench` (ADR-0081) plays scenarios headless on the
  Harness, whose `present` only keeps the frame. So no gate ever sees
  what `shuyi-app` asks the GPU to do.
- CI's `render` job (`.github/workflows/ci.yml`, the `render:` job)
  runs the capture mode (ADR-0052) under Xvfb and Mesa's software GL. It
  compares pixels with the raster and nothing else.
- `crates/app/src/render.rs` draws every quad through macroquad's
  `draw_rectangle` and `draw_texture_ex`: the cells' fills and glyphs in
  `Renderer::draw_layers` (line 238), the items in `draw_items`
  (line 301), `draw_sprite` (line 343) and `draw_painted` (line 414). It
  makes textures in `texture_of` (line 499), `recoloured` (line 512) and
  `draw_painted`'s `make` and `update` (`Texture2D::from_rgba8`,
  `update_from_bytes`). Nothing counts any of it.
- A grep of the tickets, the ADRs and the crates for "draw call",
  "texture switch" and "batch" finds only parked work (2361, 3034) and
  ADR-0078. 2718 (parked) is the only stress scenario, and it is an RTS
  simulation gated on instructions per step, not on drawing.

How macroquad 0.4.16 batches (`src/quad_gl.rs`, `QuadGl::geometry`): a
quad joins the last draw call unless the texture, the clip, the
viewport, the camera's matrix or the pipeline differ, or the call would
pass `draw_call_vertex_capacity` (10,000) or `draw_call_index_capacity`
(5,000). `shuyi-app` sets neither, so the defaults hold (`src/lib.rs`,
`Conf`). A quad is 4 vertices and 6 indices, so one call holds at most
833 quads, whatever the textures. macroquad's own `telemetry::drawcalls`
can't be used: `track_drawcall` makes a 128 × 128 render texture for
every call it records.

The research of 3034 (done in PR 532, `docs/research/3034-replacing-macroquad.md`)
counted one game's frames once, by hand, with a present hook: the
busiest frame was 3,551 quads (14,204 vertices), and at most 58 texture
switches a frame with one texture per image. Its "draw calls" column
counted texture switches only. By the capacity rule above, 3,551 quads
are at least 5 draw calls even on one atlas page. That measurement is
not repeatable and gates nothing.

Why it matters now: sprite atlases (2222), the wgpu renderer the
research plans, particles (2103, parked) and animated lights (3228) each
claim to change these numbers. Without a counted, gated number a drop in
batching after a change goes unseen, and none of them can show it
helped. This ticket lands the counter and the stress scenario first.

## Nick input

None.

## Scope

**In:**
- A counter in `shuyi-app` that every quad and every texture upload the
  `Renderer` makes goes through. It models macroquad's batching rule
  above, pinned to macroquad 0.4.16.
- `DrawCounts` and `DrawBudget` in `shuyi_platform::capture`: plain
  data, backend-neutral, so another backend can fill the same counts.
  Both are new public types that may grow: `#[non_exhaustive]`, with a
  constructor (ADR-0039).
- `Renderer::counts(&self) -> DrawCounts`, the last frame's: an
  additive public method.
- The capture report (`capture.txt` and each capture's logged line)
  gives each captured frame's counts, and the run's peaks and totals.
- A stress example, `crates/app/examples/render_stress.rs`, with frames
  built in code (no file of any game), each compared with the raster and
  each count checked against numbers worked out by hand.
- The `render` job runs it.
- An optional `draws` budget in a capture plan, and Hello, Grid's plan
  with one.
- An ADR (the `write-adr` skill) extending ADR-0052 and ADR-0008 §6.

**Out (do not do):**
- Wall-clock frame times on devices, and any gate on a wall time
  (3340). This ticket gates counts only; it times the stress example
  only as the clause with 3340 says.
- Sprite atlases (2222). Its PR can read and lower these counts;
  nothing here waits for it.
- Culling and chunked maps (3343, parked).
- Counts in the debug view, in `run --perf`'s trace, or in
  `shuyi bench` (headless: no renderer runs there).
- Draw budgets in the target profiles' `budgets`: the counts depend on
  the scene, not the target, so a budget belongs to a scenario.
- The `web-render` job: the counter is the same code in the web build,
  but this ticket runs the stress example natively only.
- Counting in any other backend, and the loading card that
  `crates/app/src/backend.rs` draws outside the `Renderer` (line 381).
- Changing how anything is drawn. The counts describe today's renderer.

**Either order with 3228:** if 3228 is done, the `lights` frame of the
stress example draws its images through 3228's light material: its
uploads are 0 once the plain images are up, and the counter counts the
material's pipeline as a reason a draw call starts. If not, the frame
records today's recoloured copies (128 uploads, below), its budget says
so in a comment, and 3228's PR lowers it.

**Either order with 3340:** if 3340 is done, time the stress example
too. It is an example, not a game, so `cargo shuyi bench --wall` can't
play it: build it in the release profile and record each frame's
interval and busy time as 3340's measure mode does, summed up with
`shuyi_ui::perf::spread`. Write the frame interval's and the busy
time's p99 for each frame kind on `docs/targets/desktop.md`, beside the
counts, with the machine. They are recorded, never a gate, and native
only (the web is Out). If not, nothing to do (3340 measures the stress
example among its scenarios).

**Estimated size:** about 1,400 lines (code 650, tests 500, docs 250).

**Split:** past 900 lines, split off steps 7–8 (the plan's `draws`
field and Hello, Grid's budget) into a new ticket.

## Implementation steps

1. **`DrawCounts`** in `crates/platform/src/capture.rs`, with
   `Default`, `Clone`, `PartialEq` and `Debug`: `quads`, `vertices`,
   `draw_calls`, `texture_switches` (draw calls started because the
   texture differs from the last one's; the first call of a frame isn't
   one), `uploads` (textures made or updated), `upload_bytes`, and
   `live_textures` (the renderer's own, at the frame's end). Its
   `Display` is one line: `quads 3201, draw calls 5, switches 1, ...`.
2. **`DrawBudget`**, every field an `Option`: `quads`, `draw_calls`,
   `texture_switches`, `uploads`, `upload_bytes`, `live_textures`.
   `DrawCounts::over(&self, &DrawBudget) -> Vec<String>` names each
   count past its budget, as `draw calls 2001 > 64`.
3. **The counter**, `crates/app/src/draw_count.rs` (private): a
   `DrawCounter` with `quad(&mut self, key: BatchKey)` and
   `upload(&mut self, bytes: usize)`. `BatchKey` holds the texture
   (`Option<miniquad::TextureId>` from `Texture2D::raw_miniquad_id`,
   `None` for a plain rectangle), the pipeline and a camera generation.
   A quad starts a new draw call when its key differs from the last
   call's, or when the last call has 4,994 indices or more
   (5,000 − 6), or 9,996 vertices or more: `QuadGl::geometry`'s rule.
   A doc comment names macroquad 0.4.16 and `src/quad_gl.rs`, so a bump
   of macroquad re-checks it.
4. **Route `render.rs` through it.** Each `draw_rectangle` and
   `draw_texture_ex` in `Renderer::draw`, `draw_backdrop`,
   `draw_layers`, `draw_items` (gizmo runs too), `draw_sprite` and
   `draw_painted` calls the counter beside it; each `set_camera` and
   `set_default_camera` moves the camera generation. Each texture made
   or updated counts an upload of `width × height × 4` bytes:
   `texture_of`, `recoloured`, `draw_painted`'s `make` and `update`,
   and the font atlas in `Renderer::new`, `with_files` and `remake`
   (counted into the next frame drawn). `Renderer::draw` resets the
   frame's counts first; `Renderer::counts` returns the last frame's.
   The counter is always on: a few additions a quad.
5. **The capture report.** `crates/app/src/capture.rs`'s `Report` adds
   the captured frame's `DrawCounts` to its line, and `capture.txt`
   ends with the run's peaks (quads, draw calls, switches, per frame),
   totals (uploads, upload bytes) and the live textures at the end.
6. **The stress example**, `crates/app/examples/render_stress.rs`
   (`--features capture`), built as `render_card.rs` is: its pictures
   are PNGs encoded in code, 16 images of 16 × 16 pixels, and its
   console is 80 × 40 cells with every cell in the clear colour (no
   fills drawn) unless the frame says. One `Renderer` draws the frames
   in this order, each a few frames and then captured and compared with
   the raster:
   - `sorted`: 2,000 sprites, 125 of each image, drawn image by image;
   - `interleaved`: the same 2,000 sprites, each image after a
     different one;
   - `text`: every cell a non-space, non-bold glyph, no sprites;
   - `lights`: the 16 images each drawn in 8 different `Paint::Shaded`
     lights, then the same frame again;
   - `painted`: one 64 × 64 painted picture whose revision moves every
     frame.

   The expected counts sit in the example as constants, each with its
   sum in a comment. It exits 1 when a count differs, as a failed
   capture does. Its budget is the expected count itself: any change in
   batching moves a number and fails, and the PR that changes the
   renderer changes the constant and says why.
7. **The plan's `draws` field.** The plan file
   (`crates/platform/src/capture.rs`, `struct File`) gains
   `#[serde(default)] draws: Option<DrawBudget>`; `Plan::draws()`
   returns it. The capture mode checks it at each capture: the frame's
   quads, draw calls and switches; the uploads and upload bytes so far
   in the run; the live textures. A count past it fails the capture
   (exit 1), and the line names the count.
8. **Hello, Grid's budget.** `examples/hello-grid/capture/frames.ron`
   gets a `draws` budget. The values and where they come from (the
   counts on the day, rounded up, and why that headroom) go in a comment
   beside it.
9. **CI.** In the `render` job, after the render card: build and run
   `render_stress` under Xvfb into `target/capture/stress`. Its
   `capture.txt` joins the summary step's loop already. The two Hello,
   Grid runs' count lines must be the same: `cmp` their `capture.txt`
   counts, as the pictures are compared.
10. **Docs.** The ADR: counts, not times; the model of macroquad's
    batching and why its telemetry isn't used; budgets per scenario,
    not per target. A section in `docs/targets/desktop.md` beside its
    bench table: what the render job counts and the stress example's
    numbers.

## Acceptance criteria

- [ ] The counter's unit tests reach each rule: 833 quads on one
      texture are one draw call and the 834th starts another; a
      texture change, a pipeline change and a camera change each start
      one; a plain rectangle after a sprite starts one.
- [ ] The stress example's counts equal these, worked out from the
      rule (each frame also counts the clear rectangle, 1 quad, with no
      texture):
  - `sorted`: quads 2,001, draw calls 17, switches 16;
  - `interleaved`: quads 2,001, draw calls 2,001, switches 2,000;
  - `text`: quads 3,201, draw calls 5 (the clear rectangle, then
    3,200 glyphs at 833 a call: 833, 833, 833, 701), switches 1;
  - `lights`, first frame (the plain images already up from `sorted`):
    uploads 128 (16 images × 8 lights), upload bytes 131,072
    (128 × 1,024); the same frame again: uploads 0. If 3228 is done:
    uploads 0 on both (the clause above);
  - `painted`: uploads 1 and upload bytes 16,384 on every frame after
    its first.
- [ ] Every frame of the stress example matches the raster within the
      default tolerance.
- [ ] A plan whose `draws` budget a capture goes past fails with exit 1,
      and its line names the count and the budget; a plan without
      `draws` runs as before.
- [ ] Two runs of Hello, Grid's plan give the same counts.
- [ ] The `render` job runs the stress example and its counts appear
      in the job's summary.
- [ ] The ADR is written and linked from ADR-0052 and ADR-0008.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `DrawCounter`'s rules (the acceptance cases above);
  `DrawCounts::over` names each count past its budget and none under
  it; a plan with `draws` parses, one with an unknown field in `draws`
  is refused.
- Integration: `render_stress` under Xvfb in the `render` job; the
  capture mode's tests (`cargo test -p shuyi-app -p shuyi-platform
  --features shuyi-app/capture`) cover a budget passed and a budget
  missed.
- Determinism: Hello, Grid's two runs' counts compared in CI.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
