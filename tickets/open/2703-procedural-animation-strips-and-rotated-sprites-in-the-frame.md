---
id: "2703"
title: "Parked: Procedural animation: body views in the scene, strips and rotated sprites in the frame, and a default skin"
type: feature
milestone: "Engine: procedural animation"
model: opus-5.5
effort: high
status: blocked
blocked_by: ["2702", "1120"]
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
World layers have only sprite items, with no rotation. Soft shapes need
two additions to the frame: strips and rotation.

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
  - an angle on sprite items, 0–255, drawn with nearest-neighbour
    sampling (1120 left rotation out);
  - one coverage rule, written once: a pixel is covered when its center
    is inside a strip's triangles or a rotated sprite's rectangle;
  - snapshot lines for both, with points rounded to pixels.
- **Renderers**: `shuyi-app` and `frame-png`'s software renderer draw
  both, with identical pixels on a test scene.
- **1-bit**: if 3006 is done, strips fill with their role's pattern.
  Otherwise, write a follow-up ticket for it and link it from 3006.
- **A default skin** in the module: chains as strips, pieces as rotated
  sprites, sorted by the body's lowest point.

**Out (do not do):**
- the example game (2704);
- lighting, shadows or shaders;
- anti-aliased edges.

## Implementation steps

1. Add `Strip` and the sprite angle to the frame, with the coverage
   rule and snapshot lines.
2. Draw both in `frame-png` and in `shuyi-app`. Compare the two on a
   test scene of five strips and five rotated sprites.
3. Write `BodyView`, `DrawDef` and the default skin.
4. Draw 2702's test creature at three steps.
5. Do the 1-bit fill, or write its follow-up ticket.

## Acceptance criteria

- [ ] `shuyi-app` and `frame-png` give the same pixels for the test
      scene (compared as in 1120).
- [ ] A strip of constant width along a straight line covers exactly
      the pixels of the matching rectangle.
- [ ] A sprite at angle 0 draws exactly as an unrotated sprite does
      today.
- [ ] Snapshots and `frame-png` pictures of the test creature at three
      steps pass.
- [ ] Tests of what happened read `BodyView`, not frame items
      (ADR-0002 §4).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the coverage rule on hand-made shapes.
- Snapshot: the frame items of the test scene and the test creature.
- Integration: the two renderers compared.

## Completion notes
