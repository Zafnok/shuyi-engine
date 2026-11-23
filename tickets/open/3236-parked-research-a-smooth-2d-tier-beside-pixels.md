---
id: "3236"
title: "Parked: Research: a smooth 2D tier beside the pixel tier: filtering per image, fractional zoom and rotation, fonts at any size, and a raster checked with a tolerance"
type: research
milestone: "Parked: styles"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: blocked
blocked_by: ["3525", "3227"]
nick_input: none
completed:
---

# 3236 — Research: a smooth 2D tier beside the pixel tier

> **Parked** (the blind-spot audit, 2026-11-23): it may start when Nick unparks it in decision ticket 3525 (what 1.0 is for, among them "smooth 2D": is the engine for pixel art only, or also for smooth, hand-drawn and high-resolution 2D games?). Until then nobody starts it. If Nick answers "pixel art only", this ticket is closed as not planned, with his answer in its completion notes. Whoever unparks it sets `status: todo`, drops `Parked:` from the title, and sets the milestone 3525's answer gives.

## Context

A blind-spot audit of the engine (2026-11-23) found that every picture
the engine draws is pixel art. Nothing in the code or the tickets plans
a game whose art is smooth: painted or hand-drawn sprites, a vector
look, high-resolution art scaled down, or a camera that zooms smoothly.

**What the code does today** (checked 2026-11-23):

- Every texture `shuyi-app` makes is sampled nearest. `FilterMode::Nearest`
  is set in `crates/app/src/render.rs` at line 432 (painted pictures),
  line 501 (`texture_of`, every uploaded PNG) and line 524
  (`recoloured`). No other filter mode appears in `crates/app`.
- No texture has mipmaps, so a large picture drawn small shimmers.
- The raster (`shuyi_ui::frame::raster`, drawn by `frame-png`,
  `crates/cli/src/frame_png.rs`) is the reference picture, and each
  backend is checked against it with `Tolerance::default()`: 2 in a
  colour channel and no pixel past that (ADR-0052 §5). A filtered,
  rotated or fractionally scaled picture has no integer reference yet.

**What the open tickets plan, all of it pixel art:**

- **1120** (parked, real time): a camera with a whole zoom of 1 to 4.
- **2335** (after the MVP): rotation is integer and decided per console
  pixel by the nearest rule. Its **Out** names "smooth or filtered
  rotation, and RotSprite-style upscaling".
- **2224** (after the MVP): proportional fonts baked to atlases at one
  pixel size. Its **Out** names "rasterising at run time, SDF and MSDF".
- **2333** (after the MVP): the `Fit` scale mode draws "sharp bilinear"
  (a `FilterMode::Linear` render target), so each logical pixel's edge
  blends over one window pixel at most. That keeps pixel art sharp; it
  isn't a smooth tier.
- **2361** (parked, real time): world layers drawn between two steps,
  still on whole console pixels.
- **3325** (parked, in PR #532's research 3034, ADR-0136): `shuyi-gpu`
  takes a `Sampling` per texture, nearest by default, on `Rgba8Unorm`
  (never sRGB). So the renderer that replaces macroquad already has the
  seam a per-image filter needs.
- **3227** (after the MVP): an ADR on the frame's colour space, blending
  space and alpha mode. Filtering mixes texels, so its result depends on
  that answer. This ticket is blocked by it, and the smooth tier keeps
  its rule: straight alpha in data, premultiplied before any filtered
  sampling, and blending on sRGB-encoded values unless a pass opts into
  linear light by its own ADR. 3227 leaves the smooth tier's filtering
  to this ticket.
- **3231** (after the MVP): UI and text drawn at the window's own
  resolution over a scaled game frame. That is the UI and text half of
  "high resolution"; this ticket is the game's own images.

Godot (a project-wide default texture filter with an override per
`CanvasItem`, mipmaps, and MSDF fonts), Unity (a filter mode and mipmaps
per imported texture, a "pixel perfect" camera as an option) and
GameMaker (texture interpolation as a game setting) all treat pixel art
as one mode among others. Shuyi treats it as the only one.

The audit's recommendation: if Nick says non-pixel-art 2D is in scope,
write an ADR for a smooth tier (filtering per image, fractional zoom and
rotation, fonts at any size, and a raster checked with a tolerance) and
the tickets that build it. That is this ticket. It builds nothing that
ships.

Graphics are a skin (ADR-0002 §4, `AGENTS.md` rule 9). Smooth drawing
is the skin's choice: logic still builds the same views and scenes, the
rules never see a float (rule 5), and tests of what happened read the
scene, not the pixels.

## Nick input

None. The product question (pixel art only, or smooth 2D too) is asked
in 3525; this ticket starts only once he has said yes. Every question
below is technical, and the agent decides it.

## Scope

**In:** a report, `docs/research/3236-smooth-2d.md`, that answers each
question below with its options, a recommendation and the facts dated;
an ADR that decides them; and the parked or todo tickets that build the
tier (as 3525's answer says), each one PR's worth.

1. **Where the choice lives.** A filter per image (in its `.meta`
   import settings), a default per game (in its project file), a choice
   per world layer, or all three with an order of precedence. What an
   image with no setting gets (nearest, so every game today draws the
   same pixels).
2. **Filtering.** Nearest, bilinear, and mipmapped (trilinear) sampling.
   Who builds the mipmaps: the GPU at load, or `shuyi bake` (so the pack
   is checked equal, ADR-0007). Premultiplied alpha at the edges of a
   filtered sprite and in an atlas (2222's padding between packed images), as 3227
   decides the alpha mode.
3. **Fractional zoom and rotation.** How a camera zoom that isn't whole
   (1120) and a rotation drawn by the GPU (beside 2335's per-pixel rule)
   are described in the frame, without a float in a simulation: an
   integer ratio, an `Fx`, or the skin's own float. Sub-pixel positions
   for smooth motion, and how they meet 2361.
4. **Fonts at any size.** Text drawn sharp at sizes chosen at run time:
   several baked sizes (2224's bake run more than once), SDF or MSDF
   atlases baked by `shuyi bake`, or glyphs rasterised at run time. Each
   option's licence (ADR-0002 §8), its cost in the web `.wasm` and its
   fit with 3231's window-resolution text.
5. **The reference picture.** How the raster draws a smooth item: an
   integer reference for bilinear and mipmapped sampling, rotation and
   fractional zoom that a GPU matches within a tolerance, or a looser
   tolerance for smooth items only. The tolerance is measured as
   ADR-0052 §5 measures the default, and it still fails a rectangle
   drawn one window pixel to the right.
6. **Each painter.** What a smooth item becomes on the one-bit renderer,
   the Playdate and the GBA (drawn nearest, refused by
   `shuyi check --target`, or a fallback the target profile names,
   ADR-0008). What it costs on today's macroquad backend and on 3325's
   `shuyi-gpu`.
7. **Mixed frames.** A smooth world with pixel UI, and a pixel world
   with smooth UI: one frame, two tiers, and where 2333's scale modes and
   3231's window-resolution layer apply.

**Out (do not do):**
- the product question itself: 3525 asks Nick;
- the colour space, blending space and alpha mode: 3227 decides them,
  and this ticket builds on its ADR;
- UI and text at the window's own resolution: 3231. It draws baked
  fonts only, picked by `FontSizes`. What it leaves out is this
  ticket's: run-time rasterising of TTF and OTF, SDF and MSDF text, and
  filtered or rotated items in 3231's window layer (`WindowLayer`);
- effect passes and materials: 3237, 2326 and 2327;
- the wgpu renderer and its `Sampling` type: 3325;
- shaped and right-to-left text: 2908;
- building any part of the tier in this ticket: the tickets it writes
  do that. A spike on a throwaway branch is allowed, and none of it is
  committed;
- changing 2335's pixel rule, 1120's whole zoom or 2224's bake. The ADR
  says what each later ticket adds beside them.

## Implementation steps

1. Read 3525's answer in `docs/decisions.md` and 3227's ADR. If 3525
   says pixel art only, close this ticket as not planned and stop.
2. Read the code and tickets as built: `crates/app/src/render.rs`,
   `crates/app/src/painted.rs`, `crates/ui/src/frame/` (the raster and
   the one-bit renderer), `crates/cli/src/frame_png.rs`, ADR-0052,
   ADR-0007, and whichever of 1120, 2222, 2224, 2333, 2335, 2361, 3231
   and 3325 are done (each one's plan, if not).
3. Survey how Godot, Unity, GameMaker, Defold and LÖVE expose filtering,
   mipmaps, camera zoom and font sizes, with the facts dated.
4. Survey the font options (question 4): crates that rasterise fonts or
   bake SDF/MSDF atlases. For each, the licence checked against
   `deny.toml`, `no_std` or not, and the size it adds to
   `examples/hello-grid`'s `cargo shuyi web --release` `.wasm` with one
   call into it.
5. Spike on a throwaway branch: one painted sprite drawn bilinear, with
   mipmaps, rotated by a non-pixel angle and zoomed by 1.5, on today's
   macroquad backend and in a raster sketch. Capture both on Windows and
   on llvmpipe as ADR-0052 does, and measure the difference.
6. Write the report and the ADR (the `write-adr` skill). The ADR
   decides questions 1 to 7.
7. Write the tickets that build the tier, each one PR's worth, with
   their `blocked_by` chains, parked or todo as 3525's answer says.
8. Run the gates.

## Acceptance criteria

- [ ] The report answers questions 1 to 7, each with its options, a
      recommendation and the facts dated.
- [ ] Each font option has its licence checked against `deny.toml` and a
      measured `.wasm` size (or the error that stops it building).
- [ ] The PR shows the spike's captures beside the raster sketch, with
      the measured pixels over and worst channel difference, and no
      spike code is committed.
- [ ] The ADR decides questions 1 to 7, says that an image with no
      setting draws exactly as today, and `docs/adr/README.md` has its
      row.
- [ ] Every ticket written is one PR's worth, names 1120, 2224, 2333,
      2335 or 3231 where it adds beside them, and has its `blocked_by`
      chain.
- [ ] The book's chapter is updated (2802), or the completion notes have
      a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- None in code: this ticket commits a report, an ADR and tickets.
- The spike's captures are measured against the raster sketch with the
  same comparison ADR-0052 uses, and the numbers go in the report.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
