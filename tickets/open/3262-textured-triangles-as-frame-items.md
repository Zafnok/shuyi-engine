---
id: "3262"
title: "Textured, vertex-coloured triangles as frame items: a 2D mesh drawn by one integer fill rule in every renderer"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["2336"]
nick_input: none
completed:
---

# 3262 — Textured triangles as frame items

## Context

2336 draws lines, circles and polygons as frame items, filled or
outlined, by one integer rule that every renderer shares (its ADR: the
top-left fill rule at pixel centres). Its **Out** leaves "textured
polygons and triangles", and only the parked Spine ticket (2217) says
it would add them. The Bevy coverage audit of 2026-11-28
(`docs/ROADMAP.md`, *Bevy's game coverage*) found the games that need
them: deforming or squash-and-stretch characters, mesh-skinned 2D art
(2160), 2D water surfaces and soft bodies, ribbon trails and
gradient-shaded vector art. Bevy draws them as 2D meshes with
`ColorMaterial`, vertex colours and a texture
(`crates/bevy_sprite_render/src/mesh2d/`; `examples/2d/mesh2d.rs`,
`mesh2d_manual.rs`, `mesh2d_vertex_color_texture.rs`).

Rule 9 holds: logic builds the mesh as plain view data, and a skin
paints it.

## Nick input

None.

## Scope

**In:**
- `Item::Triangles` in `crates/ui/src/frame/`: an optional image, a
  list of vertices (position in console pixels with 1/16-pixel fixed
  point, texel position, `Rgb`), triangle indices, a layer and a clip.
  Long lists live in the frame's arena, as long item data does today.
- **One fill rule:** 2336's top-left rule at pixel centres in `i64`, so
  two triangles sharing an edge never both cover a pixel; colours and
  texels interpolated with integer barycentric weights (affine, no
  perspective); texels sampled nearest.
- **Every renderer:** the raster (frame-png), the one-bit renderer
  (each pixel's colour dithered by the existing rule) and `shuyi-app`
  paint the same pixels. `shuyi-app` may draw through macroquad's
  meshes if a test shows the same pixels; otherwise it paints through
  2399's painted pictures.
- **Snapshot text** lists the item as plain data (vertex count, image,
  bounds).
- `GlyphBuffer::add_triangles` builds it; the new types follow ADR-0039.

**Out (do not do):**
- Spine's triangles: 2217 (parked) draws through this item;
- perspective-correct texturing and 3D (3103, parked);
- anti-aliased edges: 3236's research (question 8);
- filtered sampling (3236's smooth tier);
- skeletal meshes: 2160 (parked) builds them on this item;
- a mesh asset kind: a skin builds triangles from its own view data.

**Estimated size:** about 1,200 lines (code 550, tests 500, docs 150).

**Split:** past 1,000 lines, split off the one-bit renderer's path into
a new ticket.

## Implementation steps

1. Read 2336's ADR and `frame/shape.rs`, `glyph_buffer.rs`,
   `snapshot.rs`, the raster in `crates/cli/src/frame_png.rs`, the
   one-bit renderer and `crates/app/src/render.rs`.
2. Add the item, its builder and its arena storage.
3. Write the rasteriser as runs per row (`for_each_run`), with a plain
   per-pixel reference in the tests.
4. Paint it in the raster, the one-bit renderer and `shuyi-app`.
5. Add snapshot text and a fixture.
6. Write the docs page: a ribbon trail built from a skin's view data.

## Acceptance criteria

- [ ] Two triangles sharing an edge cover every pixel of their union
  exactly once (property test over random quads).
- [ ] A triangle's runs equal the per-pixel reference.
- [ ] A vertex-coloured triangle in frame-png equals its golden picture,
  and `shuyi-app`'s capture mode (2018) matches it within ADR-0052's
  tolerance.
- [ ] The one-bit renderer draws a textured triangle without panicking
  and equals its golden picture.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] Guides in `docs/guides/` updated, if this adds a direction, a module or a move (ADR-0009 §5).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: barycentric interpolation, the fill rule's edge cases.
- Property: shared edges; runs against the reference.
- Snapshot / integration: frame-png and one-bit goldens; the capture.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
