---
id: "1120"
title: "Parked: World layers in the frame: cameras that follow, parallax, and sort keys for y-sorting"
type: feature
milestone: "Engine: real time"
model: opus-5.5
effort: high
status: blocked
blocked_by: ["1119"]
nick_input: none
completed:
---

# 1120 — World layers: cameras, parallax, sort keys

> **Parked** (Nick, 2026-10-14: "document the parked (i.e. after MVP/parity) tickets for networking, styles for later"): after the engine reaches parity with the game (the MVP in `docs/ROADMAP.md`). Unparked early only if the game's ticket 0060 says the army walks around freely (then 1119–1125 become game work). Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

ADR-0003 §5. Today's frame places items on integer console pixels in two
layers (Under, Over) with no sort key, cuts items where cells are
replaced, and has one backdrop with whole zoom 1–4
(the game repository's [boundary map](https://github.com/Zafnok/visions-of-shuyi/blob/main/docs/engine/boundary.md), *What other game styles would break*). A
free-moving world needs layers placed in world pixels, a camera, and
depth sorting.

## Nick input

None. Our game's frames don't change.

## Scope

**In:**
- `WorldLayer` in `shuyi-ui::frame` has:
  - a `Camera`: a position in `Fx`, a whole zoom 1–4, follow with a dead
    zone, clamp to bounds, and a shake given as amplitude and seed;
  - a parallax factor (a fraction);
  - sprite items in world pixels with `sort: (z, y, order)`.
- World layers are drawn between the backdrop and the cells. Their items
  are never cut by cells.
- Snapshots list world layers and their items, one line each.
  `frame-png` and the renderer in `shuyi-app` draw them.
- Whole-frame effects (fades, game ADR-0060's brightness) include world
  layers.
- Perspective markers (ADR-0009 §3):
  - `shuyi_core::perspective` has the trait `Perspective` and the
    markers `TopDownGrid` and `TopDown`;
  - `WorldLayer`, `Camera` and a world scene are generic over
    `P: Perspective`, and the drawing side reads `P`'s projection;
  - keep the 2D projection off `Perspective` itself (a trait of its
    own for 2D markers), because the parked 3D markers (3101) have no
    2D projection;
  - 1126 and 1127 add `Isometric` and `SideView`.

**Out (do not do):** isometric projection (1126); rotation (2703 adds
it, if procedural animation is unparked).

## Implementation steps

1. Add the types and their snapshot lines.
2. Sort items once per frame.
3. Draw them in `shuyi-app` and in `frame-png`'s software renderer, with
   the same results.
4. A test scene with two parallax layers and twenty sprites, y-sorted,
   with the camera following one of them.

## Acceptance criteria

- [ ] The test scene's snapshot and `frame-png` picture match at three
      camera positions.
- [ ] `frame-png` and the window draw the same pixels (compare a
      captured frame in a manual check, noted in the PR).
- [ ] No snapshot of the game changes.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: camera follow, clamp, dead zone, sort order.
- Snapshot: the test scene.

## Completion notes
