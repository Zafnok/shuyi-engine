---
id: "3454"
title: "The debug view gains arrows, polylines, arcs and grids as gizmos, outlines the frame's hit areas, and shows a game's own named measurements beside its spans"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: high
status: todo
blocked_by: ["2336"]
nick_input: none
completed:
---

# 3454 — Gizmo shapes, hit outlines and a game's measurements

## Context

The debug view (2315, done; ADR-0085) draws `GizmoShape`s over the
frame (`crates/ui/src/frame/gizmo.rs`: rectangles, lines, crosses,
circles and labels) on channels turned on from the debug menu. Frame
records (3063, done) keep spans a game marks with `Ctx::span`
(`crates/ui/src/ctx.rs`). The Bevy coverage audit of 2026-11-28
(`docs/ROADMAP.md`, *Bevy's game coverage*) found three debug aids
Bevy has that Shuyi lacks, each useful to any game while it is made:

- **more gizmo shapes**: arrows, polylines, arcs, grids
  (`crates/bevy_gizmos/src/arrows.rs`, `arcs.rs`, `grid.rs`;
  `examples/gizmos/2d_gizmos.rs`), for steering, paths and forces;
- **the hit areas shown**: every pickable area outlined, and what is
  under the pointer named (`crates/bevy_dev_tools/src/picking_debug.rs`;
  `examples/picking/debug_picking.rs`);
- **a game's own measurements**: named counters beside the engine's
  (`crates/bevy_diagnostic/src/diagnostic.rs`;
  `examples/diagnostics/custom_diagnostic.rs`): units alive, nodes a bot
  searched (3009), particles.

## Nick input

None.

## Scope

**In:**
- **Gizmos:** `GizmoShape::Arrow { from, to, head }`, `Polyline` (up to
  16 points), `Polygon`, `Arc` and `Grid { origin, cell, count }`, drawn
  with 2336's line and circle runs, coloured by `GizmoRole`, culled to
  the view, listed in the debug snapshot. `GizmoShape` grows additively
  (ADR-0039).
- **A hits channel:** outlines each hit of the frame's list (ADR-0134)
  and labels the widget key and item under the pointer. It is painted
  as a debug overlay, never in the frame's snapshot. The contract's
  scene answer may list the hits for agents.
- **Measurements:** `Ctx::measure(name, i64)` beside `Ctx::span`, in
  debug builds: values kept in the frame record, written to 2553's
  trace as JSON lines, shown by 2316's readout, with means and p99 by
  3063's spread. Output only: no update reads them, so hashes match
  with the tools on and off.

**Out (do not do):**
- world gizmos and level layouts (2318, parked);
- 3D gizmos (2320, parked);
- the frame profiler's graph (2325, parked), which reads measurements
  when it is unparked.

**Estimated size:** about 900 lines (code 400, tests 350, docs 150).

## Implementation steps

1. Read `frame/gizmo.rs`, ADR-0085, 2336's runs, `hit.rs`, `ctx.rs`,
   `shuyi_ui::perf` and 2553's trace.
2. Add the five gizmo shapes and their runs and snapshot lines.
3. Add the hits channel.
4. Add `Ctx::measure` to frame records, the trace and the readout.
5. Write the docs page.

## Acceptance criteria

- [ ] Each new gizmo shape's snapshot line and frame-png golden match.
- [ ] With the hits channel on, every hit of a fixture screen is
  outlined, and the label names the widget under a scripted pointer.
- [ ] A measurement appears in the trace's JSON lines and the readout.
- [ ] A record replayed with the tools on and off gives the same hashes.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] Guides in `docs/guides/` updated, if this adds a direction, a module or a move (ADR-0009 §5).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each shape's bounds and culling; `measure` storage.
- Snapshot / integration: gizmo goldens; the hits overlay; a trace.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
