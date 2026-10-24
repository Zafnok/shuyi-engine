---
id: "3104"
title: "Parked: The GPU 3D painter, shuyi-app-3d, over miniquad: depth, textures, skinning, lights, a shadow map and the named effects, on GL 3.3 and WebGL 2"
type: feature
milestone: "Parked: 3D"
model: opus-5.5
effort: xhigh
status: blocked
blocked_by: ["3103", "3003", "3017", "2018"]
nick_input: none
completed:
---

# 3104 — The GPU 3D painter, `shuyi-app-3d`

> **Parked** (Nick, 2026-10-17: "I want to expand on this and add true 3D support (all kinds, first person, third, top down with 3D models), and HD-2D"): after the engine reaches parity with the game (the MVP in `docs/ROADMAP.md`) and after the 2D work already planned (`docs/decisions.md`, 2026-10-17). Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

[ADR-0011](../../docs/adr/0011-3d-fixed-point-space-a-3d-view-in-the-frame-and-hd-2d-as-a-skin.md)
§5 and §7. The software rasterizer (3103) is the reference and the
picture for `frame-png`. A game in a window needs a GPU painter: depth,
textures, skinning, lights, a shadow map, and the named effects that
make HD-2D look like a model under a lens.

It lives in its own crate, `shuyi-app-3d` (`crates/app-3d`), the
macroquad backend's part for 3D (3003), over **miniquad**, the layer under macroquad. miniquad runs on
the web through macroquad's own loader, with no `wasm-bindgen` (game
ADR-0034). wgpu was set aside for that reason (ADR-0011,
*Alternatives*).

**miniquad:** version 0.4.11, MIT OR Apache-2.0, checked on crates.io
on 2026-10-17. It is already in the build under macroquad. Check that
the version macroquad pins is the one used, so there is only one copy.

It builds on 3103 (the 3D view, the module layer hook, the painter
registry), 3003 (`shuyi-app` as the macroquad backend of
`shuyi-platform`), 3017 (target capabilities: the web profile's
`three_d` capability records what the web can run) and 2018 (the
capture mode and the `render` CI job that compare a window with the
raster).

`shuyi-app` is a default crate, and 2003's `check-modules` fails when a
default crate depends on a module (ADR-0009 §4). So the painter is a
crate of its own that depends on `shuyi-app` and `shuyi-3d`, and
`shuyi-app` gains only a hook that knows no module (ADR-0011 §5). Only
`shuyi-app-3d` touches the GPU for 3D, so it counts as backend code
(`AGENTS.md` rule 5).

macroquad and miniquad are to be replaced by our own window and graphics
layer (Nick, 2026-10-19; research 3034, parked). If 3034's tickets are
done first, this painter is written over that layer instead of miniquad,
and 3034's ADR says how. If this is done first, 3034 plans its move.

## Nick input

None.

## Scope

**In:**
- **A GPU painter hook in `shuyi-app`**: a `GpuPainter` trait and
  `register_gpu_painter(kind, painter)` for 3103's module layer kinds.
  The painter is called at its layer's place with the GL context. It
  names no module.
- **The crate `shuyi-app-3d`** (`crates/app-3d`), with module metadata,
  a README in ADR-0009 §4's order and a catalog row. It depends on
  `shuyi-app` and on `shuyi-3d` with `draw`. The `3d` feature of `shuyi`
  turns on both `shuyi-3d` and `shuyi-app-3d`.
- **Access to miniquad** through macroquad's internal GL context
  (`get_internal_gl`), flushing macroquad's own batch before and after.
  The 3D view draws into an offscreen target with depth, which is then
  drawn into the frame at the layer's place, so 2D layers and cells sit
  over and under it as in `frame-png`.
- **The GPU painter**, registered for the `shuyi-3d` module layer
  (3103's `ModulePainter`, in its GPU form through the hook):
  - meshes and textures uploaded once per asset.
    **Either order with 2220:** if 2220 is done, upload them again
    when its module hook, `Module::reloaded`, names a file they were
    read from; if not, upload once, and 2220 adds the upload on
    reload (its clause says so);
  - `Fx12` turned into `f32` here, after the scene is built (ADR-0011
    §3);
  - textures: `FilterMode::Nearest` when the material's `pixel` flag is
    set, linear with mipmaps otherwise;
  - alpha: cutout in the shader; blended draws after opaque ones,
    back to front by their centres;
  - **skinning**: each clip sampled on the CPU in `f32` at the pose's
    time, up to 64 joint matrices a draw as uniforms, skinned in the
    vertex shader. A skin with more joints (3102 reads up to 256) is
    drawn in its bind pose, as 3103 draws every skin, and counted and
    listed in `shuyi run`'s `limits` (never silently). If games' rigs
    need more, a follow-up keeps the matrices in a texture;
  - **lights**: ambient, one directional, and up to 8 point lights a
    view. More are dropped, counted, and listed in `shuyi run`'s
    `limits` (never silently);
  - **one shadow map** for the directional light that has `shadows`
    set: 2048² on desktop, 1024² on the web, with 2 × 2 filtering.
    3103's `Billboard` gains `casts_shadow: bool` (off by default,
    printed on its snapshot line only when on), and only billboards
    with it on draw into the shadow map (3107's `shadow: Cast` sets
    it; `Blob` leaves it off);
  - **fog** in the main shader.
- **The named effects** of ADR-0011 §7, as a chain of passes over the
  offscreen colour and depth, in a fixed order:
  - `depth_of_field`: blur by depth around a focus distance, or by
    screen height around a band (tilt-shift, the default);
  - `bloom`: bright pass, downsampled blur, added back;
  - `light_shafts`: a radial blur from the sun's screen position,
    masked by depth;
  - `vignette`;
  - `colour_grade`: a 3 × 3 colour matrix with lift, gamma and gain;
  - each pass uses `RGBA8` targets only, so none needs float render
    targets on WebGL 2.
- **GL 3.3 and WebGL 2.** One shader source per pass with a `#version`
  line chosen from the backend miniquad reports. Check that miniquad
  0.4.11 can ask for GL 3.3 core and WebGL 2. If it can't, record what
  it gives, and which features that loses, in the Completion notes.
- **The web check.** Run every effect in the web build in the browser
  pane, one at a time, and note in the PR what runs and what doesn't.
  Record the result in the `web` profile's capabilities (3017's
  `three_d` field):
  - if WebGL 2 runs every effect, `web` stays `three_d: Full`;
  - if it drops any, `web` becomes `three_d: Hardware { .. }` with the
    effects that run, and triangle and texture limits set from 3008's
    budgets (or high, with a comment saying why);
  - an effect the web can't run is then a `shuyi check --target web`
    error unless the skin marks it `optional` (3107).
- **Look tests.** Through 2018's capture mode and `render` CI job (Xvfb,
  Mesa's llvmpipe): add 3103's fixture scene, with shadows and effects
  off, to its plans or render card. Compare it with the raster (2332,
  with 3103's painter registered) by `raster::compare`, at a plan
  tolerance of at most 2 % of pixels over 24 in any channel. It is
  never the exact gate; the exact gates stay on the software picture
  and on snapshot text (ADR-0011 §5). The same scene with each effect
  on is saved as a CI artefact for a look by eye.
- **The debug view (2320).** If 2320 is done, draw its `Gizmo3`s as a
  line pass, depth-tested unless `on_top`. If it isn't, 2320 does this.

**Out (do not do):**
- other backends' painters (the DS's is ADR-0012's);
- more than one shadow-casting light;
- colour grading from a lookup image;
- ray-traced and path-traced light (2911 researches it);
- screen-space reflections, ambient occlusion and other effects not in
  ADR-0011 §7;
- the diorama skin (3107);
- a CI job of its own for 3D: the look test runs in 2018's `render`
  job.

## Implementation steps

1. Add the hook to `shuyi-app` and the crate `shuyi-app-3d`, with its
   metadata, README and catalog row.
2. Draw an untextured mesh into an offscreen target and composite it
   into the frame. Confirm the GL version on desktop and in the web
   build.
3. Add textures, the `pixel` flag, alpha, and per-vertex lighting.
4. Add skinning, with a test that samples 3102's arm clip at three times
   and checks the joint matrices against hand-worked values.
5. Add the shadow map, then fog.
6. Add the effect passes, one at a time, each checked on desktop and in
   the web build.
7. Add the fixture scene and its tolerance to 2018's `render` job.
   Record the web's effects in its profile.

## Acceptance criteria

- [ ] `check-modules` passes with no exception added: `cargo tree -p
      shuyi-app` shows no `shuyi-3d` and no `shuyi-app-3d`.
- [ ] A game without the `3d` feature builds exactly as before.
- [ ] The look test passes within the tolerance in 2018's `render` job.
- [ ] The web build passes and draws the fixture scene in the browser
      pane, and the PR notes each effect's result on WebGL 2.
- [ ] The `web` profile's `three_d` matches what ran on WebGL 2. If it
      drops an effect, `shuyi check --target web` fails on a fixture
      skin that needs that effect.
- [ ] Too many point lights, and a skin of more than 64 joints, are
      counted in `limits`, never dropped silently (tests).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `Fx12` to `f32`; clip sampling; the light and joint limits.
- Integration: the look test in 2018's `render` job, under llvmpipe.
- Manual: each effect on desktop and on WebGL 2, noted in the PR.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
