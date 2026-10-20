---
id: "3102"
title: "Parked: Meshes, materials, skeletons and clips as asset kinds, loaded from glTF 2.0, with OBJ as a second format"
type: feature
milestone: "Parked: 3D"
model: opus-5.5
effort: high
status: blocked
blocked_by: ["2201", "3101"]
nick_input: none
completed:
---

# 3102 — Meshes, materials, skeletons and clips from glTF

> **Parked** (Nick, 2026-10-17: "I want to expand on this and add true 3D support (all kinds, first person, third, top down with 3D models), and HD-2D"): after the engine reaches parity with the game (the MVP in `docs/ROADMAP.md`) and after the 2D work already planned (`docs/decisions.md`, 2026-10-17). Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

[ADR-0011](../../docs/adr/0011-3d-fixed-point-space-a-3d-view-in-the-frame-and-hd-2d-as-a-skin.md)
§4. glTF 2.0 is the standard 3D format, so it always loads when the `3d`
module is on
([ADR-0007](../../docs/adr/0007-formats-are-plugins-standard-files-always-optimized-ones-optional.md)
§3). Wavefront `.obj` is a second format, behind a feature, for simple
static props. Both give the same asset kinds:

- `Mesh`: positions in `Fx12` (3101), normals, UVs, skin weights;
- `Material`: base colour, a texture, emission, alpha, and a `pixel`
  flag for pixel-art textures;
- `Skeleton` and `Clip3`: bones and their keys over time.

Gameplay never reads a bone (ADR-0011 §4). Skeletons and clips are used
only in drawing, so they keep their values as the file gives them.
Positions feed collision (3105), so they are integers.

It builds on 2201 (`Format`, the registry, `AssetPath`, `DecodeCx`) and
3101 (`Fx12`, `FxVec3`, the crate).

3105 (collision) builds static meshes from a `Mesh`. Either order works:
if 3105 has landed, add `TriMesh::from_mesh(&Mesh, &FxTransform)` to it
here, with a test; if not, 3105 adds it.

**The `gltf` crate:** version 1.4.1, MIT OR Apache-2.0, checked on
crates.io on 2026-10-17. Check the current version and license again
before adding it, and check its dependency tree with `cargo deny`.

## Nick input

None.

## Scope

**In:**
- **Asset kinds** in `shuyi_3d::mesh`:
  - `Mesh`: `positions: Vec<FxVec3>`, `normals: Option<Vec<FxVec3>>`,
    `uvs: Option<Vec<[Fx12; 2]>>`, `skin: Option<Vec<SkinWeights>>`,
    and `primitives: Vec<Primitive { material: Option<AssetPath>,
    indices: Vec<u32> }>`, triangles only;
  - `SkinWeights { joints: [u16; 4], weights: [u8; 4] }`, the weights
    summing to 255 (rounded by largest remainder, ties to the lower
    index);
  - `Material { base_colour: [u8; 4], texture: Option<AssetPath>,
    emission: [u8; 3], alpha: Opaque | Cutout(u8) | Blend,
    double_sided: bool, pixel: bool }`;
  - `Skeleton { joints: Vec<Joint { name, parent: Option<u16>,
    inverse_bind: [f32; 16] }> }`;
  - `Clip3 { name, length_ms: u32, channels: Vec<Channel { joint: u16,
    property: Translation | Rotation | Scale, times_ms: Vec<u32>,
    values: Vec<f32>, step: bool }> }`;
  - a text dump for each kind (counts, the first vertices, and a hash),
    and a `hash()` over every value's bits.
- **Sub-asset paths**: one glTF file holds many assets, named
  `house.glb#mesh/Roof`, `#material/Tiles`, `#skeleton/<skin name>`
  and `#clip/<animation name>`. If 2201's `AssetPath` has no fragment,
  add one.
- **Format `gltf`** (extensions `gltf` and `glb`, sniffing the `glTF`
  magic of `.glb`) through `gltf` with `default-features = false` and
  only the features we need (no `import`, so it never reads files or
  decodes images itself):
  - `.gltf`'s `.bin` and image URIs, and `.glb`'s embedded buffers and
    images, are resolved through `DecodeCx`, never the file system;
  - textures decode through the registry's image formats (PNG from
    2201, others from 2203);
  - **floats become `Fx12` by their bits.** `fx12_from_f32_bits(u32)`
    reads sign, exponent and mantissa in integers and rounds toward
    negative infinity. NaN, infinity and values out of range are errors
    naming the accessor;
  - times become milliseconds the same way;
  - triangle strips and fans become triangles; points and lines are an
    error naming the primitive;
  - `pixel` is true when the base colour texture's sampler uses
    `NEAREST` for magnification (Blender's *Closest*);
  - `CUBICSPLINE` channels, morph targets (3110 adds them) and required
    extensions are errors naming the file and the place; optional
    extensions are ignored and listed in the decode's `limits`;
  - size limits from `DecodeCx`: bytes, vertices, joints (256) and
    channels.
- **Format `obj`** behind the `obj` feature of `shuyi-3d`, our own
  small parser (no crate):
  - `v`, `vt`, `vn`, `f` (polygons split as fans), `o`, `g`,
    `usemtl` and `mtllib`;
  - `.mtl` with `Kd`, `Ke`, `d` and `map_Kd`;
  - decimals parsed straight into `Fx12` in integers.
- **Features of `shuyi-3d`:** `gltf` (on by default, needs `std`) and
  `obj` (off). The `3d` feature of `shuyi` turns on `gltf`, so glTF
  always loads where 3D is on.
- **ADR-0007 duties** for each format:
  - a fuzz-style property test (random bytes, and truncated fixtures,
    never panic);
  - a determinism fixture: each fixture's text dump and hash are
    checked in and compared on native and `wasm32`.
- **Fixtures made for the tests** (ours, CC0): a textured cube with a
  nearest-filtered texture, a two-bone arm with one clip, both as
  `.gltf` + `.bin` and as `.glb`, and the cube as `.obj` + `.mtl`.

**Out (do not do):**
- **an optimized mesh format.** It is derived at bake time in 2204's
  pack and tested equal to its glTF (ADR-0011 §4, ADR-0007 §4). Write
  that as a parked follow-up ticket when 2204 is done;
- glTF's node tree and scenes (a level with many placed meshes). Write
  a ticket if 3108 needs one;
- glTF cameras and lights (they come from the scene, 3103);
- more than one UV set;
- drawing (3103, 3104).

## Implementation steps

1. Write the kinds, their dumps and hashes, and `fx12_from_f32_bits`
   with a test over every `f32` exponent and random mantissas.
2. Add sub-asset paths, if 2201 lacks them.
3. Write the `gltf` format, with the fixtures and error fixtures.
4. Write the `obj` format behind its feature, with the cube fixture.
5. Add the fuzz-style tests and the determinism fixtures.
6. Add `gltf` to `cargo deny`'s run, and write its license check in the
   PR.

## Acceptance criteria

- [ ] The cube decodes from `.gltf`, `.glb` and `.obj` to equal `Mesh`
      positions and indices (a test).
- [ ] The cube's material has `pixel: true`.
- [ ] The arm decodes to a skeleton of two joints and a clip whose
      dump matches its checked-in fixture.
- [ ] Every weight set sums to 255 (property test over random inputs).
- [ ] `fx12_from_f32_bits` agrees with exact rounding for every `f32`
      exponent and 10 000 random mantissas each, and rejects NaN,
      infinity and values out of range.
- [ ] Each bad fixture gives an error naming the file and the place
      (accessor, primitive, channel or line).
- [ ] Random and truncated bytes never panic either format.
- [ ] The fixtures' hashes match on native and `wasm32`.
- [ ] `cargo deny check` passes with `gltf`, and the web build passes.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each kind, the conversions, the sub-asset paths, each error.
- Exhaustive over exponents: `fx12_from_f32_bits`.
- Property: skin weights; random bytes for both formats.
- Fixture: the dumps and hashes, native and `wasm32`.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
