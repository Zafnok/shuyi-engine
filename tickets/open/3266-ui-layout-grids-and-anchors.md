---
id: "3266"
title: "UI layout grids and anchors: a grid container of tracks and spans, and nodes placed against a corner of their parent with a draw order, in whole units"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["2329"]
nick_input: none
completed:
---

# 3266 — UI layout grids and anchors

## Context

2329 builds `shuyi_ui::layout`, an integer subset of flexbox of our own
(rows and columns, grow, gap, align, wrap) in whole cells or pixels,
`no_std`, tested by equality. Its **Out** leaves "CSS grid, absolute
positioning", and its Context says a follow-up could wrap `taffy`. The
Bevy coverage audit of 2026-11-28 (`docs/ROADMAP.md`, *Bevy's game
coverage*) found the screens that need them: level-select grids, shop
tables, inventory panels with a details pane, and HUD pieces pinned to
the corners of the screen. Bevy lays these out with taffy's grid and
absolute positions (`crates/bevy_ui/src/layout/`;
`examples/ui/layout/grid.rs`, `anchor_layout.rs`, `z_index.rs`).

`taffy` works in `f32`, so this extends our own integer layout rather
than wrapping it, for the reasons 2329 gives.

## Nick input

None.

## Scope

**In:**
- **Grid:** `Display::Grid` with column and row tracks, each `Fixed(n)`,
  `Fit` (its content's size) or `Fill(weight)`; a gap; a child's
  column and row and its span; auto placement in row order. Leftover
  units after integer division go to the first tracks, one each, so the
  result is exact and the same everywhere.
- **Anchors:** `Position::Anchored { corner, offset }`, taken out of the
  flow and placed against a corner (or edge centre) of its parent.
- **Draw order:** a `layer: i8` on a node; equal layers draw in tree
  order. Focus (2355) and hits follow what is drawn on top.
- New fields on 2329's `Style` are additive (it follows ADR-0039).
- The `.ui.ron` names for them. **Either order with 2330:** if 2330 is
  done, add `grid`, `tracks`, `span` and `anchor` to its node format and
  its schema; if not, nothing to do (2330 reads them from `Style`).

**Out (do not do):**
- named grid areas and line names;
- tracks sized by the largest of several children across rows in more
  than one pass (one pass, `Fit` reads the track's own children);
- scrolling inside a grid (3234's scroll panel holds one).

**Estimated size:** about 1,000 lines (code 450, tests 450, docs 100).

## Implementation steps

1. Read 2329's layout code and its tests.
2. Add tracks, spans and auto placement, with the leftover rule.
3. Add anchored nodes and the draw order.
4. Add the `.ui.ron` names as the clause says.
5. Write the docs page: a shop table and a HUD with four corners.

## Acceptance criteria

- [ ] Three `Fill(1)` columns over 10 units are 4, 3 and 3.
- [ ] A child spanning two columns gets both columns and the gap.
- [ ] Auto placement fills rows left to right, skipping cells a span
  took.
- [ ] An anchored node at the bottom right with offset (1, 1) sits one
  unit in from that corner whatever its siblings do.
- [ ] A higher layer is drawn over a lower one and gets the hit.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] Guides in `docs/guides/` updated, if this adds a direction, a module or a move (ADR-0009 §5).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each track kind, spans, anchors, layers.
- Property: tracks always sum to the container's size minus gaps; no
  two auto-placed children overlap.
- Snapshot / integration: a laid-out screen's golden text.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
