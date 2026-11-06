---
id: "3209"
title: "Hello, Grid and the grid template's stage offer the debug view's grid channel"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: ["2315"]
nick_input: none
completed:
---

# 3209 — Hello, Grid and the grid template's stage offer the `grid` channel

## Context

2315 (done) built the debug view
([ADR-0085](../../docs/adr/0085-the-debug-view-gizmos-as-data-from-screens-in-channels-turned-on-from-the-debug-menu.md)):
a screen hands in gizmos from `Screen::debug_draw`, and the debug menu's
"Debug view" turns each channel on. Its first channel is the grid kit's
`grid` (`shuyi_ui::grid::debug::layout_gizmos`, the `GRID` channel): the
cell lines, the map's bounds, the cursor's cell, each unit's cell and
index, and the edges the map goes on past.

Only the engine's toy calls it (`shuyi_ui::toy::grid::Field`). The two
grid games people actually start from don't:

- Hello, Grid's stage, `examples/hello-grid/src/screens/stage.rs`
  (`StageScreen`, `impl Screen<Hello>` near line 336, its `scene` near
  line 151);
- the grid template's stage, which every `shuyi new --template grid`
  game gets:
  `templates/shuyi-default/stages/top-down-grid/crates/game/src/stages/top_down_grid.rs`
  (`StageScreen`, `impl Screen<Data>` near line 325, its `scene` near
  line 269).

So a developer who opens the debug menu in either finds "Debug view"
with nothing to turn on, and `frame-png --debug grid` draws nothing.

## Nick input

None.

## Scope

**In:**
- `debug_draw` on both `StageScreen`s: return early unless
  `out.channel_on(GRID)`, then build the scene as `draw` does, ask the
  skin for its `Grid` (`GridSkin::grid(&scene, area)`, the same area
  `draw` paints) and call `layout_gizmos(&scene, &grid, out)`.
- A test in each: with the `grid` channel on
  (`ctx.debug_view.set("grid", true)` before the `Game` starts), the
  frame's gizmos (`GlyphBuffer::gizmos`) are all of channel `grid`, the
  cursor's cell is among them, and with the channel off there are none.
- Hello, Grid's `README.md` and the template's game `README` (its part
  under `templates/shuyi-default/stages/top-down-grid/`) say in one line
  that the debug menu's "Debug view" shows the grid's layout, and that
  `cargo shuyi frame-png out.png --debug grid` pictures it.
- The template's files stay formatted for a made game (`cargo fmt`
  doesn't reach `templates/`: make a game and run its gates, as the
  `run-gates` skill's *A new game from each template* says).

**Out (do not do):**
- new channels, or changes to `layout_gizmos` (a change it needs is its
  own ticket);
- the blank template, which has no map;
- anything a player sees: the view is drawn only with debug tools on.

## Implementation steps

1. Read ADR-0085, `crates/ui/src/grid/debug.rs` and the toy's
   `Field::debug_draw` in `crates/ui/src/toy/grid.rs`.
2. Add `debug_draw` to Hello, Grid's `StageScreen` and its test.
3. Add it to the template's `StageScreen` and its test; make a `grid`
   game with `shuyi new` and run its gates.
4. The two READMEs.
5. Run the gates; check with `frame-png --debug grid` on Hello, Grid and
   look at the picture.

## Acceptance criteria

- [ ] Hello, Grid's stage offers `grid`, and with it on draws the
      layout over the map (a test reads the gizmos).
- [ ] A new `grid` game does the same, and its own gates pass.
- [ ] `cargo shuyi frame-png out.png --debug grid` in `examples/hello-grid`
      draws the gizmos; the picture is looked at.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Integration: each stage with `grid` on and off, through the Harness.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
