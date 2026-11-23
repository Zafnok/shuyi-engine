---
id: "3241"
title: "The capture mode compares a frame that fills the window with the raster, read from its render target"
type: infra
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: medium
status: todo
blocked_by: ["2333"]
nick_input: none
completed:
---

# 3241 — The capture of a filled frame

## Context

Split off 2333 at its step 7 (its PR passed about 600 counted lines
without it). 2333 made `shuyi-app` draw a `ScaleMode::Fit` frame at a
whole `draw_scale` into a render target (`Stretch` in
`crates/app/src/render.rs`), then stretch it into the window (ADR-0139).
The capture mode (`shuyi-app`'s `capture` feature,
`crates/app/src/capture.rs`, from 2018) compares the window with the
raster in CI's `render` job, and CI's `web-render` job runs the same
plan in headless Chromium (2019), but only at whole scales: a mistake in
the render-target path (the picture upside down, the backdrop's
viewport rows on the target) would pass CI today.

## Nick input

None.

## Scope

**In:**
- A capture plan may ask for `Fit` (how is this ticket's choice; a test
  of `Plan::parse` reads it), and the window is resized so the frame
  doesn't come out whole.
- That capture reads the render target's texture (its pixels before the
  stretch) and compares them with the raster at `draw_scale`, within the
  usual `Tolerance`; the stretched window is saved for a look only.
- Hello, Grid's plan (`examples/hello-grid/capture/frames.ron`) gains
  one capture in `Fit`, with a backdrop if hello-grid has a screen that
  shows one, so both the target's rows and the backdrop's viewport are
  covered, in the `render` and `web-render` jobs.

**Out:** `frame-png` and the raster stay at whole scales; the player's
pick (ticket 3240).

**Estimated size:** about 400 lines (code 200, tests 150, docs 50).

## Implementation steps

1. Read `capture.rs`, `readback.rs`, `render.rs`'s `Stretch`, and the
   plan format and its tests.
2. Let a plan ask for `Fit`; give `Stretch` a way to read its target's
   pixels under the `capture` feature.
3. Compare them with the raster at `draw_scale`.
4. Add the capture to hello-grid's plan; run both CI jobs.

## Acceptance criteria

- [ ] A `Plan::parse` test reads a `Fit` capture.
- [ ] CI's `render` and `web-render` jobs compare hello-grid's `Fit`
      capture with the raster and pass.
- [ ] A capture whose target is drawn upside down fails (checked once
      by hand with a flipped draw, written in the completion notes).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the plan's parse.
- Integration: the capture in both CI jobs.

## Completion notes

*(Filled in by the session that completes the ticket.)*
