---
id: "3103"
title: "Parked: The 3D view: a world layer of plain data in the frame, its snapshot text, and an integer software rasterizer for frame-png"
type: feature
milestone: "Parked: 3D"
model: opus-5.5
effort: xhigh
status: blocked
blocked_by: ["3101", "3102", "1120", "2332"]
nick_input: none
completed:
---

# 3103 — The 3D view and the software rasterizer

> **Parked** (Nick, 2026-10-17: "I want to expand on this and add true 3D support (all kinds, first person, third, top down with 3D models), and HD-2D"): after the engine reaches parity with the game (the MVP in `docs/ROADMAP.md`) and after the 2D work already planned (`docs/decisions.md`, 2026-10-17). Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

[ADR-0011](../../docs/adr/0011-3d-fixed-point-space-a-3d-view-in-the-frame-and-hd-2d-as-a-skin.md)
§5. Logic builds a plain-data scene and a skin paints it (ADR-0002 §4).
In 3D that scene is a **3D view**: a world layer of a new kind, beside
1120's 2D world layers. Tests of what happened read its snapshot text.

Any frame must become a PNG with no window (`frame-png`). For 3D that
needs a painter that runs anywhere and gives the same picture on every
machine: `shuyi_3d::raster`, an integer triangle rasterizer. It is the
reference picture. The GPU painter (3104) is compared to it, never the
other way round.

No default crate may depend on a module (ADR-0009 §4). The frame lives
in `shuyi-ui`, a default crate, and the 3D view's types live in
`shuyi-3d`. So the frame gets a small, general hook: a **module layer**
that holds a module's plain data, prints its snapshot lines, and is
painted by a painter the module registers. This is a new pattern, so
it gets an ADR (the `write-adr` skill) extending ADR-0011 §5 and
ADR-0003 §5.

It builds on 3101 (`Fx12`, transforms, `Angle16`), 3102 (meshes,
materials, textures), 1120 (world layers, their order and snapshot
lines) and 2332 (the raster: `frame-png`'s painter moved into
`shuyi_ui::frame::raster`).

3106 (cameras) and this ticket can land in either order. If 3106 has
landed, add `From<CameraPose> for Camera3` here. If not, 3106 adds it.

## Nick input

None.

## Scope

**In:**
- **The module layer hook** in `shuyi_ui::frame`:
  - `trait ModuleLayer: Any + Send + Sync` with `module(&self) ->
    &'static str`, `snapshot_lines(&self, out: &mut Vec<String>)`,
    `as_any(&self) -> &dyn Any` and `clone_box`;
  - the frame's world layers gain a `Module(Box<dyn ModuleLayer>)`
    kind, ordered with the others, so 2D layers and glyph cells draw
    over and under it as today;
  - `trait ModulePainter` with `module()` and `paint(&self, layer: &dyn
    ModuleLayer, assets, target: &mut shuyi_ui::frame::raster::Image)
    -> PaintReport`, and a `Painters` registry in
    `shuyi_ui::frame::raster` (2332) that the raster `Painter` calls for
    module layers, so `frame-png` and a game painting with the raster
    both get it. Both sit behind `shuyi-ui`'s `raster` feature, as the
    raster does;
  - a module layer with no painter is drawn as a grey box labelled with
    the module's name, and `frame-png` lists it in its `limits`. It is
    never silently blank.
- **`shuyi_3d::view`** (behind `draw`), plain data, implementing
  `ModuleLayer`:
  - `View3d { camera, draws, billboards, lights, fog, effects }`;
  - `Camera3 { transform: FxTransform, projection: Perspective { fov_y:
    Angle16 } | Orthographic { height: Fx12 }, near: Fx12, far: Fx12 }`;
  - `MeshDraw { mesh, material: Option<AssetPath>, transform, pose:
    Option<Pose { clip, time_ms }>, order: u32 }`;
  - `Billboard { sprite, pos: FxVec3, size: [Fx12; 2], mode: Upright |
    FaceCamera, order: u32 }`, where `sprite` is the same atlas frame a
    2D sprite item names. Like 2D world items, billboards are never cut
    by cells;
  - `Light`: `Ambient { colour }`, `Directional { dir, colour,
    shadows: bool }`, `Point { pos, colour, range }`, each with an
    `Fx12` intensity;
  - `fog: Option<Fog { colour, start, end }>`: the `fog` effect of §7;
  - `effects: Vec<Effect>`: an enum of `DepthOfField`, `Bloom`,
    `LightShafts`, `Vignette` and `ColourGrade`, each with its settings
    as plain fields (3104 paints them, 3107 fills them from a skin
    file).
- **Snapshot text**, from `snapshot_lines`:
  - the camera on one line, then one line per draw, billboard and
    light, in the order they were added, then fog and the effects;
  - numbers are `Fx12` printed as exact decimals with trailing zeros
    dropped (1/4096 is `0.000244140625`), so a snapshot is exact and
    readable;
  - a golden test for one view with one of each.
- **`shuyi_3d::raster`** (behind `draw`, which turns on `shuyi-ui`'s
  `raster` feature; `no_std` with `alloc`):
  - `fn render(view: &View3d, assets: &Assets3, w: u16, h: u16) ->
    (Image, RasterReport)`, where `Image` is the raster's
    (`shuyi_ui::frame::raster::Image`, 2332) and `Assets3` looks up
    loaded meshes, materials and textures by `AssetPath`;
  - vertices to camera space with `FxTransform`, projection in
    integers, clipping against the near plane, back faces culled unless
    the material is double-sided;
  - edge functions with 4 bits of subpixel and the top-left fill rule;
  - a depth buffer of `i32` from interpolated 1/z, and UVs perspective
    correct (u/z, v/z, divided per pixel in `i64`);
  - textures: nearest when `pixel` is set, integer bilinear otherwise,
    no mipmaps; `Cutout` alpha is tested, `Blend` is drawn as cutout at
    128 and listed;
  - lighting: per-vertex when the mesh has normals, flat (per face)
    otherwise, from ambient, directional and point lights in `Fx12`;
  - billboards as quads, upright (turned about the vertical axis only)
    or facing the camera, depth tested and cut out;
  - linear fog by depth;
  - skinned meshes are drawn in their bind pose, and listed;
  - `RasterReport` lists what it skipped (shadows, each effect, blend,
    bind pose) and counts triangles and pixels drawn.
- **`frame-png`** registers the raster painter when the `3d` feature of
  `shuyi` is on, and writes the report's skipped list as one line of
  text in the picture's top-right corner and in its `limits`.

**Out (do not do):**
- the GPU painter, shadows and effects (3104);
- skinning in the rasterizer (draw the bind pose; a later ticket if
  pictures of poses are wanted);
- sorting blended triangles;
- 2D items placed in 3D depth (billboards are the way).

## Implementation steps

1. Write the ADR for the module layer (the `write-adr` skill). Add the
   hook to `shuyi_ui::frame`, and `ModulePainter` and the `Painters`
   registry to `shuyi_ui::frame::raster` (`frame-png` uses it through
   2332's wrapper), with a fixture module layer that prints two lines
   and paints a square.
2. Write `shuyi_3d::view`, its snapshot lines and the golden test.
3. Write the rasterizer one stage at a time, each with unit tests on
   tiny images (4 × 4 to 16 × 16): one flat triangle, two triangles
   sharing an edge (no pixel drawn twice, none missed), depth order,
   near clipping, a textured quad, lighting, billboards, fog.
4. Write the fixture scene: 3102's textured cube twice, a lit ground
   quad, two billboards and fog, at 320 × 180. Check in its PNG and the
   hash of its RGBA pixels.
5. Wire the painter into `frame-png` with the corner line and `limits`.

## Acceptance criteria

- [ ] A frame with the fixture module layer lists its lines in the
      snapshot and paints its square in `frame-png`.
- [ ] A module layer with no painter shows the labelled box and a
      `limits` entry.
- [ ] The golden snapshot of the 3D view passes.
- [ ] Two triangles sharing an edge cover every pixel of their union
      exactly once (property test over random triangles).
- [ ] The fixture scene's pixel hash is the same on Linux, Windows and
      macOS CI, and on `wasm32`.
- [ ] The fixture picture's corner line names shadows and the effects
      as skipped when the view asks for them.
- [ ] `clippy::float_arithmetic` finds nothing in `shuyi_3d::raster`.
- [ ] The ADR exists and is listed in `docs/adr/README.md`.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each raster stage on tiny images; the module layer hook.
- Property: shared edges; depth (a nearer triangle always wins).
- Golden: the snapshot text.
- Fixture: the scene's pixel hash on every CI machine and `wasm32`.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
