---
id: "3265"
title: "A folding tree list widget: rows with children, opened and closed by actions and the pointer, on the widget contract"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: ["3230"]
nick_input: none
completed:
---

# 3265 — A folding tree list widget

## Context

Shuyi's lists (`crates/ui/src/widgets/list.rs`, `ListCursor`) are flat.
The Bevy coverage audit of 2026-11-28 (`docs/ROADMAP.md`, *Bevy's game
coverage*) found games that need rows with children: a quest log by
chapter, a codex or bestiary by kind, crafting categories, a save
browser by slot. Bevy has a headless tree and a disclosure toggle
(`crates/bevy_ui_widgets/src/tree.rs`,
`crates/bevy_feathers/src/controls/tree_view.rs`,
`disclosure_toggle.rs`; `examples/ui/widgets/headless_tree.rs`).
3230 gives every widget one contract for measure, input, focus, paint
and hits. 3453's state browser uses this widget too.

## Nick input

None.

## Scope

**In:**
- `shuyi_ui::widgets::tree`: a `TreeCursor` over a plain-data tree
  (row ids, labels as text keys, an open set), extending `ListCursor`'s
  scroll window to the visible rows.
- **Actions:** up and down move over visible rows; right opens a closed
  row or moves into it; left closes an open row or moves to its parent;
  confirm picks. All through `UiAction`, never keys (rule 8).
- **Pointer:** a click on a row's marker opens or closes it, a click on
  the row picks it (2409's hits).
- **A look:** the glyph skin paints indents and open and closed markers;
  the one-bit look (2371) gets the same layout.
- The widget implements 3230's contract, and a `.ui.ron` node
  (`tree:`). **Either order with 2330:** if 2330 is done, add the node;
  if not, nothing to do (2330 lists the widgets that exist when it
  lands).
- The open set is the screen's state, saved by the screen if it wants.

**Out (do not do):**
- dragging rows to reorder them (3233's drag and drop may do it later);
- many selected rows at once;
- lazy loading of children.

## Implementation steps

1. Read `widgets/list.rs`, 3230's contract and 2409's hit code.
2. Write `TreeCursor` and its moves, with the visible-row window.
3. Write the glyph and one-bit paint.
4. Implement the contract and the pointer.
5. Add the `.ui.ron` node as the clause says.
6. Write the docs page: a quest log by chapter.

## Acceptance criteria

- [ ] Left on a child moves to its parent; left on an open parent
  closes it; right on a closed parent opens it.
- [ ] Closing a parent whose child had the cursor moves the cursor to
  the parent.
- [ ] The scroll window keeps the cursor visible in a tree of 200 rows.
- [ ] A snapshot of the glyph look of a three-level tree equals its
  golden text.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] Guides in `docs/guides/` updated, if this adds a direction, a module or a move (ADR-0009 §5).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: every move, the window.
- Property: any sequence of moves keeps the cursor on a visible row.
- Snapshot / integration: glyph and one-bit goldens; a Harness run.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
