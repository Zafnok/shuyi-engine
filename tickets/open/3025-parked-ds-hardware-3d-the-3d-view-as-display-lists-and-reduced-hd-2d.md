---
id: "3025"
title: "Parked: DS hardware 3D: the 3D view on the DS's 3D engine, meshes baked to display lists, a triangle budget, and reduced HD-2D"
type: feature
milestone: "Parked: export targets"
model: opus-5.5
effort: xhigh
status: blocked
blocked_by: ["3024", "3103", "3107"]
nick_input: none
completed:
---

# 3025 — DS hardware 3D and reduced HD-2D

> **Parked** (Nick, 2026-10-17: "I want GBA, and DS ports"): until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

[ADR-0012](../../docs/adr/0012-gba-and-ds-targets-capabilities-indexed-colour-two-screens-and-homebrew.md)
§3, its 3D part, and [ADR-0011](../../docs/adr/0011-3d-fixed-point-space-a-3d-view-in-the-frame-and-hd-2d-as-a-skin.md)
§8. The DS's 3D engine (ADR-0012, *The hardware*; as 3018 re-checked it
in `docs/targets/nds.md`):

- draws on one screen, or on both at 30 fps through a capture trick;
- takes up to 6 144 vertices a frame, so 2 048 triangles or 1 536 quads;
- uses fixed-point geometry: 20.12 matrices, and 16-bit vertex
  coordinates (4.12, so about ±8 units; re-check in GBATEK);
- has textures, fog, toon shading, edge marking and alpha blending, a
  few directional lights, and no programmable shaders;
- holds textures in video RAM banks, 512 KB of them (prov.).

What ADR-0011 and ADR-0012 decide:

- the 3D space is `Fx12`, 20.12, which is the DS geometry engine's own
  format, so transforms are handed over as they are (ADR-0011 §3);
- the 3D view in the frame (3103) is plain data: a camera, mesh draws,
  billboards, lights, fog and effects by name (ADR-0011 §5). On the DS
  it maps to the 3D engine, a hardware painter;
- meshes are baked to the hardware's display lists and textures at bake
  time, in the pack (2204);
- billboards are textured quads;
- over the triangle budget is a check error for baked scenes, and a
  counted drop at run time;
- HD-2D (3107's diorama skin) runs reduced: terrain, billboards and fog,
  with edge marking for outlines. Depth of field, bloom and light shafts
  are effects the DS doesn't have, so a diorama skin must mark them
  `optional` to export there (ADR-0012 §1, 3017).

The `nds` profile says `three_d: Hardware { max_triangles: 2048,
max_texture_bytes: 524288, effects: [fog, edge_marking, toon, alpha] }`
(3017). The 2D backend is 3024.

## Nick input

None.

## Scope

**In:**
- **Baking meshes for the DS**, `shuyi bake --target nds`:
  - each `Mesh` (ADR-0011 §4) becomes a `DsMesh`: a display list of
    packed geometry commands, with vertices in 4.12 and the mesh's scale
    moved into its matrix, normals and texture coordinates in the
    hardware's formats, and its triangle count;
  - each `Material`'s texture becomes a DS texture in a paletted format
    (16 or 256 colours, through 3019's quantizer) or direct colour,
    chosen per material in its sidecar. Pixel-art textures stay
    unfiltered, which the DS is anyway;
  - the bake reports each mesh's triangles and the texture bytes per
    stage against the profile, and too many texture bytes for a stage is
    an error naming the stage;
  - equality (ADR-0007 §4): decoding a baked `DsMesh` gives the same
    triangles as the source mesh, after the documented rounding to 4.12,
    on every fixture.
- **The 3D painter in `shuyi-nds`**, `shuyi_nds::three_d`:
  - the camera becomes the projection and view matrices, from `Fx12`
    without conversion;
  - each mesh draw loads its `FxTransform` and calls its display list;
  - skinned meshes are posed on the ARM9 at draw time, within the
    budget, or are an error at bake time if the mesh is too big (say
    which in the PR);
  - billboards become textured quads facing the camera or upright, as
    the 3D view says;
  - lights: ambient and up to the hardware's number of directional
    lights. Point lights and shadow maps are not in the profile's
    `effects`, so a view that needs them fails the check unless they
    are optional (3017);
  - fog, toon shading, edge marking and alpha come from the view's
    effects by name;
  - the 3D picture goes to the surface the 3D view is on (3023), through
    the main engine. A 3D view on both surfaces runs each at 30 fps by
    display capture into a free video RAM bank, alternating screens.
- **The triangle budget.** A baked scene (a stage's static meshes and
  its declared needs, 3017) over 2 048 triangles is a check error, naming
  the stage and both numbers. At run time, draws past the budget are
  dropped farthest first, counted in `PresentStats`, and logged once a
  frame.
- **Reduced HD-2D.** The diorama skin (3107) builds its 3D view as on
  any target. On the DS:
  - terrain, props, billboards, shadow blobs and fog draw;
  - outlines use edge marking;
  - `depth_of_field`, `bloom` and `light_shafts` are not in the profile,
    so a skin that doesn't mark them `optional` fails `shuyi check
    --target nds`, and one that does gets them listed in `left_out`
    (3017);
  - picking stays 2D (ADR-0011 §7), so cursors and grid targeting work
    with touch on the lower surface.
- **A 3D example on the DS.** If 3108's example exists, it gets an `nds`
  layout and exports. If not, a test scene in `crates/nds/tests/`: a
  few meshes, billboards, a light and fog. In the emulator 3018 chose:
  - a replay fixture ends with the same state hash as on desktop;
  - the 3D view's snapshot text (ADR-0011 §5) equals the desktop's;
  - the emulator's screenshot is compared by eye in the PR with the
    software rasterizer's `frame-png` (3103), as ADR-0011 §5 says for
    pictures of a look.
- **A diorama on the DS.** The engine's HD-2D example (3108 or 3109,
  whichever holds it), if it exists, or a small diorama fixture
  otherwise, exports with the effects the DS lacks marked optional, and
  runs in the emulator.
- **Bench scenarios** (3008): the 3D example's heaviest view, and the
  diorama, run in the emulator runner (3020) against the `nds` budgets.

**Out (do not do):**
- the 2D backend (3024);
- point lights, shadow maps and post effects in software on the DS;
- the GPU painter or the software rasterizer (3103, 3104);
- making a non-optional effect optional by itself (ADR-0012 §1: the
  engine never drops a required feature silently);
- measuring on a device (3026).

## Implementation steps

1. Re-read the 3D parts of `docs/targets/nds.md` (3018). Fix the profile
   if a number differs.
2. Write `DsMesh` and the DS texture forms, their `Format`s, the bake
   and the equality tests.
3. Write the bindings for the 3D calls the painter uses, in 3024's
   `sys.rs`, and extend the fake hardware with a recorder of 3D commands.
4. Write the painter: camera, meshes, billboards, lights, effects.
   Test it against the recorder.
5. Add the triangle budget at bake time and at run time.
6. Add the two-screen capture.
7. Export the 3D example and the diorama, run them in the emulator, and
   add the bench scenarios.

## Acceptance criteria

- [ ] A baked fixture mesh's display list matches its checked-in bytes,
      and decodes to the source's triangles after rounding.
- [ ] A fixture stage with 2 100 static triangles fails `shuyi check
      --target nds`, naming the stage, 2 100 and 2 048.
- [ ] At run time, a view over the budget draws 2 048 triangles, and
      `PresentStats` counts the rest (host test against the recorder).
- [ ] A diorama skin with `bloom` not optional fails on `nds`; marked
      optional, it passes, and `bloom` is in `left_out`.
- [ ] The 3D example's replay hash and snapshot text in the emulator
      equal the desktop run's (CI).
- [ ] The 3D bench scenarios run in CI against the `nds` budgets.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the vertex and normal conversion with its rounding, display
  list packing, the camera matrices, the triangle budget and the drop
  order.
- Property: random meshes bake without a panic, and the triangle count
  is kept.
- Integration: the 3D example and the diorama in the emulator, in CI.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
