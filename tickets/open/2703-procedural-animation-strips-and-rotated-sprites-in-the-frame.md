---
id: "2703"
title: "Parked: Procedural animation: body views in the scene, strips in the frame, and a default skin"
type: feature
milestone: "Engine: procedural animation"
model: opus-5.5
effort: high
status: blocked
blocked_by: ["2702", "1120", "2335", "2336", "2018"]
nick_input: none
completed:
---

# 2703 — Procedural animation: drawing bodies

> **Parked** (Nick, 2026-10-16: "this will be very late addition since it won't tie into visions of shuyi game"): after the engine reaches parity with the game (the MVP in `docs/ROADMAP.md`) and after 1119 and 1122. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

[ADR-0010](../../docs/adr/0010-procedural-animation-bodies-limbs-and-strips.md)
§5. Graphics are a skin (ADR-0002 §4): logic says what is there, and a
skin says how it looks. Bodies and limbs (2701, 2702) become a
`BodyView` in the scene, and a skin paints it on a world layer (1120).
World layers carry 2335's rotated sprites and 2336's shapes (through
1120). Soft shapes need one more item: strips.

## Nick input

None.

## Scope

**In:**
- **`BodyView`** (`shuyi_procanim::view`, behind the `view` feature):
  - chains: points, a half-width per point, and a palette role;
  - pieces: an anchor point, an angle, and a sprite id;
  - `BodyView::from(&Body, &Rig, &DrawDef)`. `DrawDef` is the drawing
    part of the creature's RON: which points make which chain, the
    widths, the roles, the pieces.
- **The frame** (`shuyi-ui`, on 1120's world layers):
  - a `Strip` item: points with half-widths and a sort key, filled with
    a palette color or with an image stretched along its length;
  - a strip's pixels follow 2336's polygon rule (pixel centres,
    half-open edges; 2335's ADR), through a `for_each_run` like
    `Shape`'s;
  - a snapshot line for a strip, with points rounded to pixels.
- **Renderers**: `shuyi-app` and the raster (`shuyi_ui::frame::raster`,
  2332) draw strips, with the same pixels on a test scene that goes on
  2018's render card.
- **1-bit**: strips fill with their role's pattern (3006).
- **A default skin** in the module: chains as strips, pieces as rotated
  sprites (2335), sorted by the body's lowest point.

**Out (do not do):**
- the example game (2704);
- rotated sprites (2335) and drawn shapes (2336);
- lighting, shadows or shaders;
- anti-aliased edges.

## Implementation steps

1. Add `Strip` to the frame, on 2336's pixel rule, with its snapshot
   line.
2. Draw strips in the raster and in `shuyi-app`, and add a test scene
   of five strips and five pieces drawn as 2335's rotated sprites to
   2018's render card.
3. Write `BodyView`, `DrawDef` and the default skin.
4. Draw 2702's test creature at three steps.
5. Do the 1-bit fill: strips in their role's pattern through 3006's
   `PatternMap` shades, in the fast one-bit renderer and its reference
   (`shuyi_ui::frame::onebit`), with strips added to their
   fast-equals-reference proptests.

## Acceptance criteria

- [ ] `shuyi-app` and the raster give the same pixels for the test
      scene (2018's `render` CI job passes with the scene on the card).
- [ ] A strip of constant width along a straight line covers exactly
      the pixels of the matching rectangle.
- [ ] Snapshots and raster pictures of the test creature at three steps
      pass.
- [ ] Tests of what happened read `BodyView`, not frame items
      (ADR-0002 §4).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: a strip's runs on hand-made strips.
- Snapshot: the frame items of the test scene and the test creature.
- Integration: 2018's `render` job with the scene on the card.

## Completion notes
