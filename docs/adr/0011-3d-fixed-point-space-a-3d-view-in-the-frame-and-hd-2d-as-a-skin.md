# ADR-0011: 3D: a fixed-point space, a 3D view in the frame, four directions, and HD-2D as a skin

- **Status:** Accepted; its tickets are parked
- **Date:** 2026-10-17
- **Related tickets:** 3101–3109; they build on 1119 (tick simulation),
  1120 (world layers, perspective markers), 1121 (pointer and analog
  input), 1122 (collision), 1124 (maps), 1129 (`shuyi new`), 2003
  (modules), 2004 (guides), 2201 (formats), 3003 (the platform
  boundary), 3017 (target capabilities)
- **Extends:** ADR-0003 §2–§6 (stages, `Fx`, the frame, projections);
  ADR-0004 (assets in, data out); ADR-0007 (formats are plugins);
  ADR-0008 §1–§2 (profiles, backends); ADR-0009 (modules, directions,
  perspective markers)
- **Lifts:** ADR-0003's "without 3D" (its *Alternatives considered*:
  "3D or 2.5D … out of scope by Nick's word")

## Context

Nick, 2026-10-17 (`docs/decisions.md`): expand the templates "and add
true 3D support (all kinds, first person, third, top down with 3D
models), and HD-2D (like final fantasy remakes)". Not every target can
run every feature.

**HD-2D** is Square Enix's name for the look of Octopath Traveler
(2018), Triangle Strategy, and the Live A Live and Dragon Quest III
remakes. From their public trailers and interviews:

- the world is 3D: low, tiled terrain, buildings and props, often
  built like a diorama;
- characters and many props are 2D pixel-art sprites, standing upright
  and facing the camera;
- the camera looks down at a tilt, mostly fixed, and sometimes turns or
  zooms for effect;
- dynamic lights and shadows, strong depth of field (a tilt-shift
  look), bloom, fog and light shafts make it look like a model under a
  lens.

The gameplay under that look is a 2D game's: walking a map, and in
Triangle Strategy a grid battle with heights.

**What the engine has, or plans, that 3D must keep:**

- simulations are deterministic, in integers (`Fx`, ADR-0003 §3), so
  replays, rollback, bots and server checks work;
- logic builds a plain-data scene and a skin paints it (ADR-0002 §4);
  tests of behaviour read the scene, and only tests of a look read
  pixels;
- any frame renders to a PNG with no window (`frame-png`), and an agent
  reads what is on screen as data;
- the web build uses macroquad's own loader, not `wasm-bindgen` (game
  ADR-0034);
- modules: a game pays only for what it turns on, and types say what a
  piece is built for (ADR-0009).

**Facts checked on 2026-10-17** (crates.io):

- `gltf` 1.4.1 is MIT OR Apache-2.0;
- `miniquad` 0.4.11, under macroquad, is MIT OR Apache-2.0;
- `rapier3d` 0.36.1 is Apache-2.0.

## Decision

### 1. 3D is modules, and the default engine stays 2D

- **`shuyi-3d`** (`crates/3d`): the 3D space, meshes as data, the 3D
  scene and view, cameras, and the 3D collision kit. Its simulation part
  is pure, `no_std` with `alloc`, and in fixed-point. Its drawing part
  sits behind its `draw` feature (ADR-0009 §4).
- **`shuyi-diorama`** (`crates/diorama`): the HD-2D skin (§7). It
  depends on `shuyi-3d`.
- **`shuyi-app-3d`** (`crates/app-3d`): the GPU painter (§5), the
  macroquad backend's part for 3D. It depends on `shuyi-app` and
  `shuyi-3d`, and only it touches the GPU for 3D, so it counts as
  backend code (`AGENTS.md` rule 5).
- They follow ADR-0009 §4: README, example, skill, metadata, catalog
  row, and a feature of the `shuyi` crate (`3d` turns on `shuyi-3d` and
  `shuyi-app-3d`; `diorama` adds `shuyi-diorama`). No default crate
  depends on any of them.

### 2. Four new directions

ADR-0009 §2's *Perspective* concern gains three answers, and a new
concern holds HD-2D:

| Concern | New answers | Marker / module |
| ------- | ----------- | --------------- |
| Perspective | first person; third person; top-down 3D | `FirstPerson`, `ThirdPerson`, `Overhead3D` in `shuyi_3d::perspective`, implementing `shuyi_core::perspective::Perspective` |
| World drawing | flat 2D (default); diorama (HD-2D); full 3D | the default skins; `shuyi-diorama`; `shuyi-3d`'s `draw` |

- A 3D stage names its marker in its type (ADR-0009 §3). The
  first-person controller takes only `FirstPerson`, and so on.
- **HD-2D isn't a perspective.** It draws a `TopDownGrid`, `TopDown` or
  `Isometric` stage in 3D (§7). Its rules stay 2D, so the grid kit, the
  movers and every 2D kit work under it unchanged.
- Each direction is a template for `shuyi new` (`first-person`,
  `third-person`, `top-down-3d`, `hd-2d`), with a direction guide and an
  example (ADR-0009 §5–§6). `shuyi new` with no template asks which
  direction, as a menu, and the same choice is a flag for agents and
  scripts (ADR-0005: the CLI and MCP never need a prompt).

### 3. The 3D space is fixed-point, 20.12

- **`Fixed<const F: u32>`** in `shuyi_core::fx`: an `i32` with `F`
  fractional bits, with the operators, rounding and functions written
  once. `Fx` becomes `type Fx = Fixed<8>`, with no change of
  behaviour (its tests and fixtures prove it). `Fx12 = Fixed<12>` is
  the 3D type: one unit is a metre, a step is 1/4096 m (0.24 mm), and
  the range is ±524 km.
- **Why 20.12:** the precision 3D rotations need, the range a level
  needs, products in `i64`, and it is the DS geometry engine's own
  format, so the DS backend (ADR-0012) hands it over as is.
- **Vectors and rotations:** `FxVec3`, `FxMat3` (rotation and scale),
  `FxTransform` (a rotation and a translation), and `FxQuat` for
  turning smoothly. Normalising rounds in one documented way.
- **Angles:** `Angle16`, 65 536 steps a turn, so it wraps on overflow.
  Sine and cosine come from a 1 024-entry quarter-wave table with
  integer interpolation; `atan2` and square roots are integer
  algorithms written once. If 2701 has already added square roots and
  inverse trigonometry to `Fx`, 3101 makes them generic; if not, 3101
  writes them generic.
- **No floats in a 3D simulation**, as in 2D (ADR-0003 §3). Floats
  appear only in drawing, after the scene is built.

### 4. Meshes, materials and clips come from glTF

- **glTF 2.0** is the standard 3D format and always loads (ADR-0007),
  through `gltf` (MIT OR Apache-2.0). Both forms load: `.gltf` with its
  `.bin`, and `.glb`. Wavefront `.obj` is a second format plugin,
  behind a feature, for simple static props.
- **Asset kinds:** `Mesh` (positions in `Fx12`, normals, UVs, skin
  weights), `Material` (base colour, a texture, emission, alpha, and a
  `pixel` flag that turns off filtering for pixel-art textures),
  `Skeleton` and `Clip3` (bone keys over time).
- **Gameplay never reads a bone.** Clips are sampled in drawing only, in
  floats. Hits and triggers use the simulation's own shapes (§6). So
  animation can be smooth without making the rules depend on it.
- **An optimized mesh format** is derived at bake time (2204's pack)
  and tested equal to the glTF it came from (ADR-0007 §4).

### 5. The frame gains 3D views; a skin paints them

- **A 3D view** is a world layer of a new kind (ADR-0003 §5 has 2D
  ones). It holds plain data:
  - a camera: a transform, a projection (perspective with a field of
    view, or orthographic), near and far;
  - mesh draws: a mesh id, a material id, a transform, a pose (clip and
    time) if skinned;
  - billboards: a sprite in 3D, upright or facing the camera, with
    the same sort and cut rules as 2D sprite items;
  - lights: ambient, directional (casting shadows or not), and point
    lights with a range;
  - fog, and a list of effects by name with settings (§7).
- 2D world layers and glyph cells draw over and under 3D views as they
  do today. Menus and text stay cells (ADR-0003 §5).
- **Snapshots** list a 3D view as text: the camera, then one line per
  draw, billboard and light, with transforms in `Fx12`. Tests of what
  happened read this.
- **Three painters:**
  - **The software rasterizer** (`shuyi_3d::raster`, `no_std`): an
    integer, deterministic triangle rasterizer with a depth buffer,
    textures, flat and per-vertex lighting, and billboards. It is the
    reference for `frame-png`, so a 3D frame becomes the same PNG on
    every machine, with no window and no GPU. It skips shadows and
    effects, and the picture says so in a corner. The Playdate and other
    targets with no GPU can use it within their budgets.
  - **The GPU painter** in `shuyi-app-3d`, over miniquad (the layer
    under macroquad). `shuyi-app` gains only a small hook that lets a
    painter registered for a layer kind draw with the GL context at that
    layer's place, so it depends on no module. It runs wherever `shuyi-app`
    runs, the web build included, without `wasm-bindgen`. It does depth,
    textures, skinning, lights, a shadow map, and the effects of §7. It
    targets GL 3.3 and WebGL 2; 3104 checks each effect on WebGL 2 and
    lists what the web drops.
  - **Hardware painters** in other backends, such as the DS's (ADR-0012).
- **Pictures of a look** compare GPU output with the software reference
  only by eye or with a tolerance; the exact gate is on the software
  picture and on the snapshot text.

### 6. 3D collision and movement: a deterministic kit, physics optional

- In `shuyi-3d`, in `Fx12`:
  - shapes: spheres, capsules, boxes aligned to the axes, and static
    triangle meshes and heightfields built from a mesh at load time,
    with a grid of cells for broad phase;
  - a **character mover**: a capsule that slides along what it hits,
    climbs steps up to a height, holds to slopes up to an angle, and
    stops on ground (the 3D form of 1122's movers);
  - triggers (enter and leave) and ray and shape casts, for aiming,
    line of sight and cameras.
- **Cameras** (3106): first-person look from actions (a stick, or the
  pointer turned into an axis, both quantized in `TickInput`, 1121),
  a third-person orbit that pulls in when a wall is in the way, and an
  overhead follow camera with a dead zone. Each takes its marker. A
  look never reads a key or a mouse (`AGENTS.md` rule 8).
- **Rigid bodies in floats** are a separate module, `rapier3d`
  (Apache-2.0) as 1128 does with rapier2d, ticketed when an example
  needs it (ADR-0009 §7). Not ticketed yet.

### 7. HD-2D is a skin over a 2D stage

- **`shuyi-diorama` paints a 2D stage in 3D.** It reads the stage's
  tile map and scene, and builds a 3D view from them:
  - **terrain** from the map's tiles and a **height** per tile. Heights
    come from a map layer (Tiled or LDtk, 1124) or the map's own data.
    Each tile kind maps to a block mesh, or to a cliff, slope or flat
    piece, in a RON file of the skin;
  - **props** from the map's objects, each a mesh or a billboard;
  - **characters** as billboards of their 2D clips (1123), upright, at
    their 2D position lifted to the terrain's height, with a shadow
    blob or a shadow-casting quad;
  - **the camera** from the stage's 2D camera (1120): the same follow
    and bounds, seen from a tilt, a distance and a field of view set in
    the skin's file.
- **Picking stays 2D.** A pointer ray goes through the 3D camera, hits
  the terrain, and gives a tile, so cursors and grid targeting work.
- **Effects, by name**, each with settings in the skin's file:
  `depth_of_field` (tilt-shift by default), `bloom`, `fog`,
  `light_shafts`, `vignette`, `colour_grade`. Each target's profile says
  which it can run (ADR-0012 §1). An effect a target can't run is a
  `shuyi check --target` error unless the skin marks it `optional`, and
  an optional effect is left out on that target and listed in the check's
  output.
- **What a game gets:** a grid or free 2D game reskinned in HD-2D keeps
  its rules, its tests and its replays. Only its skin changes (`AGENTS.md`
  rule 9).

### 8. What every target runs

Each profile declares a `three_d` capability (ADR-0012 §1):
`none`, `software { max_triangles }`, `hardware { max_triangles,
max_texture_bytes, effects }`, or `full`. A stage with a 3D view, or a
diorama skin, fails `shuyi check --target` on a target that can't draw
it. Planned:

| Target | 3D | HD-2D |
| ------ | -- | ----- |
| desktop, Steam Deck | full | all effects |
| web | full, on WebGL 2 | the effects 3104 shows WebGL 2 can run |
| DS | hardware: 2 048 triangles a frame, fog, no shaders | reduced: fog and edge marking only (ADR-0012) |
| Playdate | software, a few hundred triangles (prov.) | none |
| GBA | none | none |

## Consequences

- A developer can start a first-person, third-person, top-down 3D or
  HD-2D game from a template, and still get replays, headless tests,
  snapshots as text and `frame-png` pictures.
- HD-2D costs a 2D game only a skin. That is the cheapest path to the
  look, and it means a grid tactics game can try it without touching its
  rules.
- 3D in fixed-point is less convenient than floats. The engine gives
  `Fx12` operators, tables, a tested mover and cameras, and the examples
  show the idioms.
- A software rasterizer is real work, and it is what keeps "any frame
  is a PNG with no window" true in 3D. It also gives 3D to targets with
  no GPU.
- `Fx` becoming `Fixed<8>` touches the core. The change is mechanical,
  and the existing fixtures prove nothing moved.
- The GPU painter is a second renderer to keep working on the web.
  WebGL 2 limits may cost the web some HD-2D effects.
- The engine's README stops saying "No 3D" and says 3D is planned, as
  modules, parked.

## Alternatives considered

- **Floats in 3D simulations** — what most 3D engines do, and easier
  to write. It loses replays, rollback and server checks across native
  and web (ADR-0003 §3), and it would make 3D games second-class in the
  engine's own promises.
- **`Fx` (24.8) for 3D too** — one type, but 1/256 m steps make
  rotations and slow movement visibly jittery, and products of
  rotations lose most of their bits. 16.16 was the other choice: more
  precision, but only ±32 km of range, and the DS would need a
  conversion on every vertex.
- **wgpu for the GPU painter** — modern and portable, but on the web it
  needs `wasm-bindgen`, which our web build doesn't use (game ADR-0034),
  and it adds a large dependency tree. miniquad is already in the build.
  A wgpu backend can be added later as a platform backend, without
  changing scenes.
- **Embed Bevy or Fyrox for 3D** — both are capable, but each brings
  its own ECS, loop, assets and floats, so a 3D game would be a
  different engine with Shuyi bolted on.
- **HD-2D as a perspective of its own, with its own rules** — it would
  duplicate the 2D kits and split the games that want it from the 2D
  tools they already use. The look is a skin; the rules stay 2D.
- **No software rasterizer; `frame-png` needs a GPU for 3D** — CI and
  agents without a GPU would lose pictures of 3D frames, and the
  pictures would differ by driver.
- **Gameplay reads animated bones** (hitboxes on the skeleton) —
  precise, but it ties the rules to float sampling. A game that needs
  it can sample bones in `Fx12` later; it starts with shapes of its own.
