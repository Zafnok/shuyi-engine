---
id: "3227"
title: The frame's colour space, blending space and alpha mode are decided in an ADR
type: research
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: medium
status: todo
blocked_by: ["3034"]
nick_input: none
completed:
---

# 3227 — The frame's colour space, blending space and alpha mode are decided in an ADR

## Context

Found by the engine blind-spot audit of 2026-11-23. No ADR or doc says
which colour space the frame's colours are in, whether blending happens
on encoded values or in linear light, or whether alpha is straight or
premultiplied. Each painter does something today, and they agree only
because nobody changed it.

What the code does today (checked 2026-11-23):

- `crates/ui/src/frame/color.rs`: `Rgb` is three `u8` channels, with no
  alpha and no colour space named.
- `crates/ui/src/frame/glyph_buffer.rs:280`: a sprite's `opacity` is a
  `u8`.
- `crates/assets/src/lighting.rs:107`: `Light::apply` multiplies the
  encoded bytes.
- `crates/ui/src/frame/raster.rs:103-121`: the raster, the reference
  picture of ADR-0052, blends `src * alpha + dst * (1 - alpha)` on the
  encoded values, in floats rounded once, with straight alpha, and
  writes alpha 255.
- `crates/assets/src/bitmap/convert.rs:363-367` (`png_pixels`): a PNG is
  expanded and cut to 8 bits a channel; its `gAMA`, `cHRM`, `sRGB` and
  `iCCP` chunks are ignored. `docs/formats.md` says the QOI colour-space
  tag is ignored and JPEG XL is rendered to sRGB. Nothing says the PNG's
  tags are ignored.
- `shuyi-app` draws through macroquad 0.4.16 and miniquad 0.4.11.
  Textures are `GL_RGBA8` (miniquad `src/graphics/gl.rs:78`), not sRGB.
  The pipeline's blend is `SourceAlpha, OneMinusSourceAlpha`
  (macroquad `src/quad_gl.rs:402-406`) with no separate alpha blend, so
  the same factors apply to the destination's alpha. Over an opaque
  pixel, a sprite at alpha 0.5 leaves alpha 0.75 in the framebuffer.
- miniquad's web loader asks for the canvas with `getContext("webgl2")`
  and no attributes (`js/gl.js:91`), so the canvas has an alpha channel
  and the browser composites it over the page. The engine's shell page
  is black (`crates/cli/web/index.html:19`), so this shows nowhere
  today. A game hosted on a page of another colour may see it. This is
  a lead to check, not a measured bug.
- The render card (2392, done) already draws sprites at opacity 128 and a
  picture whose bottom row is half clear (`crates/ui/src/frame/raster/card.rs:330-401`).
  ADR-0052 §5 measured it within the tolerance of 2 per channel, so the
  window and the raster blend in the same space today.

Why decide it now:

- PR 532's ADR-0136 §10 (3034's research, on branch
  `t3034-replacing-macroquad-research-v2`) already picks non-sRGB
  formats for the wgpu renderer (3325): `Bgra8Unorm` or `Rgba8Unorm`
  surfaces, `Rgba8Unorm` textures, and a last pass that undoes the
  encoding where a surface offers only `Rgba8UnormSrgb` (WebGL 2). It
  says this for one renderer. It does not say it is the rule every
  painter keeps, nor what alpha is, nor what a future linear-light
  effect must do.
- A wgpu port that took the sRGB surface wgpu offers first would blend
  in linear light. A white sprite at half opacity over black would come out
  about 188 where the raster has 128: far past the tolerance, on every
  translucent pixel.
- 2337 (tint and blend modes, todo) writes integer reference rules
  (`shuyi_ui::frame::blend`, not on main yet). Its Context names
  premultiplied alpha only as a mode Godot has. 2326 and 2328 (screen
  effects and 2D lights, parked) and the smooth 2D research filed in
  this batch (3236) will each need to know the space they blend in.
- ADR-0061 already premultiplies alpha when it downscales an icon, and
  says why (a clear pixel's colour must not bleed in). That is the one
  place the engine has decided alpha, for one tool.

The deliverable is the ADR. It changes no pixel. If it finds a painter
that breaks the rule it writes, it writes a ticket for the fix.

Builds on ADR-0007 (formats), ADR-0052 (the raster is the reference),
ADR-0061 (premultiplied downscaling), ADR-0128 (textures) and ADR-0136
§10, which lands with 3034.

## Nick input

None.

## Scope

**In:**
- One ADR (the `write-adr` skill), extending ADR-0052 and ADR-0136 §10,
  that decides:
  - **What a colour is.** Every `Rgb`, palette entry, texel and file
    pixel is an sRGB-encoded 8-bit value, shown as is. No painter
    converts it.
  - **The space blending happens in.** Opacity, `Light::apply`, tints,
    fades and 2337's blend modes work on encoded values, as the raster
    does. The ADR says what a later pass in linear light (a bloom, a
    light) must do to opt in: name it in its own ADR, and give the
    raster the same maths so the comparison still holds.
  - **The alpha mode.** Straight alpha in files, in `Painted` and in
    frame items. Whether a renderer may premultiply inside itself, and
    the rule for filtered sampling (premultiply before filtering, as
    ADR-0061 does), so a future linear filter or downscale has no dark
    fringes.
  - **What a window shows.** The output is opaque: a renderer writes
    alpha 255 (or masks alpha writes), and a web canvas does not let the
    page show through. The ADR checks today's macroquad web build
    against this and says what it found.
  - **Formats.** Surfaces, render targets and textures are non-sRGB
    (Unorm) views on every backend, as ADR-0136 §10 says for wgpu, with
    its last-pass rule for a surface that offers only sRGB.
  - **Files' colour tags.** A PNG's `gAMA`, `cHRM`, `sRGB` and `iCCP`
    chunks and QOI's tag are ignored; JPEG XL is rendered to sRGB. Wide
    gamut and HDR are named as not supported.
- A test that proves the render card catches a renderer blending in
  linear light: from the card's own frame, blend its translucent pixels
  in linear light and assert that `raster::compare` with the default
  tolerance fails. If no translucent pixel of the card sits where the
  two blends differ by more than the tolerance, add one to the card.
- A line in `docs/formats.md` that a PNG's colour chunks are ignored,
  and a short paragraph in the doc the `document` skill names for the
  frame's colours (the book's frame chapter, if that is where it lives).
- A ticket for each painter the ADR finds breaking its rule (the web
  canvas's alpha, if it shows through).

**Out (do not do):**
- Changing any painter's output, the raster's maths or the tolerance.
  A fix the ADR finds is its own ticket.
- Designing a linear-light pass, HDR, wide gamut or colour management.
- The cost of animated lights (3228). The ADR says only which space a
  light's multiply is in, wherever it runs.
- The smooth tier's filtering (3236). The ADR states only the alpha rule
  any filtering keeps.
- Editing ADR-0136 or the tickets PR 532 writes. The new ADR names them;
  their text stays as it is.

No public type changes, so ADR-0039 does not apply.

## Implementation steps

1. Read ADR-0052, ADR-0061 (its "Straight alpha" note), ADR-0128 and ADR-0136 §10
   on main, and 2337's Scope.
2. Check each painter against the rules in the Scope, and write what
   each does, with file and line: the raster (`crates/ui/src/frame/raster.rs`),
   `shuyi-app` over macroquad (its textures, its blend state, its alpha
   writes), the web build (the canvas's context attributes in
   miniquad's `js/gl.js` and `crates/cli/web/`), `frame-png`
   (`crates/cli/src/frame_png/`), the one-bit renderer
   (`crates/ui/src/frame/onebit/`) and Playdate if it reads colours.
3. On the web build, check whether a translucent sprite leaves alpha
   below 255 in the canvas: run an example with a translucent sprite in
   a page whose background is not black, and read the result. Record it
   in the ADR. If it shows through, write the fix's ticket.
4. Write the ADR with the `write-adr` skill: the decisions in the Scope,
   each with its reason, and a table of the painters from step 2.
5. Add the test in `crates/ui/src/frame/raster/` (beside the card's
   tests): a linear-light blend of the card's translucent pixels fails
   `raster::compare` with `Tolerance::default()`. Add a translucent
   sprite over a mid-tone backdrop to the card only if the test needs
   one; then update the card's captures as ADR-0052 says.
6. Add the `docs/formats.md` line and the frame doc's paragraph
   (`document` skill).

## Acceptance criteria

- [ ] An ADR in `docs/adr/` decides colour encoding, blending space,
      alpha mode, opaque output, surface and texture formats, and files'
      colour tags, each with its reason.
- [ ] The ADR has a table of every painter, saying what it does today
      and whether it keeps the rule, with file and line.
- [ ] The ADR says what the web canvas does with a translucent pixel,
      as measured in step 3.
- [ ] A test shows that blending the render card in linear light fails
      `raster::compare` at the default tolerance.
- [ ] `docs/formats.md` says a PNG's colour chunks are ignored.
- [ ] Each painter the ADR finds breaking a rule has a ticket in
      `tickets/open/`, named in the completion notes.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the linear-light blend of the render card fails the comparison
  with the default tolerance, and the raster's own card passes it.
- Property: none.
- Snapshot / integration: the render card's captures, only if step 5
  adds a sprite.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
