---
id: "3263"
title: "More drawn shapes: arcs, sectors, rings, ellipses and rounded rectangles with opacity, and gradient fills, borders and drop shadows for panels without art"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["2336", "2335", "3227"]
nick_input: none
completed:
---

# 3263 — Arcs, ellipses, gradients and panel looks

## Context

2336 draws lines, circles and polygons. Its **Out** leaves "curves,
arcs, ellipses and rounded rectangles", a thick ring, and opacity on
shapes. The Bevy coverage audit of 2026-11-28 (`docs/ROADMAP.md`,
*Bevy's game coverage*) found the games these leave out:

- stealth games with vision cones, radar sweeps, cooldown pies, and
  tactics games with translucent range rings (Bevy's `CircularSector`,
  `Annulus`, `Ellipse`, `Capsule2d`: `crates/bevy_shape/src/dim2.rs`;
  `examples/2d/2d_shapes.rs`, `mesh2d_arcs.rs`);
- art-free games with gradient skies, heat maps and bars that shade
  from green to red (`crates/bevy_color/src/oklaba.rs`,
  `color_gradient.rs`);
- polished UIs built from generated panels with rounded corners,
  borders, drop shadows and gradients instead of nine-slice art
  (`crates/bevy_ui_render/src/box_shadow.rs`, `gradient.rs`;
  `examples/ui/styling/borders.rs`, `box_shadow.rs`, `gradients.rs`).

Every renderer must draw the same pixels, as 2336's ADR requires, so
each rule is an integer test at pixel centres emitted as runs.

## Nick input

None.

## Scope

**In:**
- **Shapes**, beside 2336's in `crates/ui/src/frame/shape.rs`:
  `Arc { center, radius, start: Angle, sweep: Angle, width }`,
  `Sector`, `Ring { center, outer, inner }`, `Ellipse { center, rx, ry
  }` and `RoundedRect { rect, radius }`, filled or outlined. Angles use
  2335's integer `Angle` and sine table; sectors test the two edges as
  half-planes, ellipses use the midpoint rule.
- **Opacity** as a `u8` on every shape, blended as 3227's ADR says.
- **Gradient fills:** a `ColorRamp` of up to 8 stops from palette roles,
  sampled at a `u16` position through an integer table in the mix space
  3227 decides (Oklab, unless its ADR says otherwise), with an integer
  twin as `Rgb::lerp` has today. `Fill::Linear { from, to, ramp }` and
  `Fill::Radial { center, radius, ramp }` on shapes and overlay
  rectangles. The one-bit renderer dithers each stop's role.
- **Panel looks:** per-side border widths and colours on a rounded
  rectangle, and a hard-edged offset drop shadow, which 3235's look file
  may name.
- Snapshot text lists each as plain data. New types follow ADR-0039
  (2336 makes its shape types `#[non_exhaustive]` where they may grow,
  so new kinds are additive; if one isn't, the completion notes get a
  `Breaking:` line).

**Out (do not do):**
- anti-aliased edges and blurred shadows: 3236's research (question 8)
  and its smooth tier;
- blend modes on shapes (2337 adds them to sprites and rectangles);
- curves (a skin draws them as 2336's lines);
- gradients along a line (stroke gradients).

**Estimated size:** about 1,500 lines (code 700, tests 650, docs 150).

**Split:** past 1,000 lines, split off gradient fills and panel looks
(the third and fourth bullets) into a new ticket.

## Implementation steps

1. Read 2336's and 2335's ADRs and code and 3227's ADR.
2. Add the five shapes with a per-pixel reference for each in the
   tests, then their runs.
3. Add opacity through every renderer.
4. Add `ColorRamp` and the fills; then borders and the shadow.
5. Add snapshot text and goldens.
6. Write the docs page: a vision cone and a gradient health bar built
   from plain view data.

## Acceptance criteria

- [ ] Each new shape's runs equal its per-pixel reference (property
  test over random sizes and angles).
- [ ] A 90-degree sector covers exactly the pixels of a quarter of the
  filled circle of the same radius.
- [ ] A ring with `inner == outer` draws nothing; a sector with a sweep
  of a full turn equals the filled circle.
- [ ] A ramp sampled at 0 and at the end gives its first and last stop
  exactly, and the integer twin equals the table.
- [ ] Frame-png, the one-bit renderer and `shuyi-app`'s capture (2018)
  agree on a fixture of every shape and fill, within ADR-0052's
  tolerance.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] Guides in `docs/guides/` updated, if this adds a direction, a module or a move (ADR-0009 §5).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each shape's edge cases, ramps, borders, the shadow.
- Property: runs against references.
- Snapshot / integration: goldens in each renderer; the capture.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
