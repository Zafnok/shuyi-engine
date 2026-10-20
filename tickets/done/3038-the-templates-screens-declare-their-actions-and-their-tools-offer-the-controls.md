---
id: "3038"
title: "The templates' screens declare their actions, and their tools offer the controls to check --target"
type: feature
milestone: After MVP
model: sonnet-5
effort: medium
status: done
blocked_by: ["3002"]
nick_input: none
completed: 2026-10-20
---

# 3038 — The templates declare their controls

## Context

Ticket 3002 ([ADR-0025](../../docs/adr/0025-controls-fit-screens-declare-actions-targets-have-button-tables-and-the-harness-holds-back-the-rest.md))
added the controls' check to `shuyi check --target`:

- each screen declares the actions it reacts to (`Screen::actions`);
- a game's tools hand its keymap and its screens to the check
  (`GameTools::controls`, most simply a
  `shuyi_ui::input::fit::GameControls`);
- the keymap file has a table of buttons for each target that has no
  keyboard (`targets: { "steam-deck": { … } }`).

`examples/hello-grid` does all three: see its `screens::all`, its
`tools::controls` and its `files/keymap.ron`.

The games `shuyi new` makes (`templates/grid`, `templates/blank`) do none
of it yet. Their `check --target all` passes, but its limits say "the
controls aren't checked". A new game should start out checked.

## Nick input

None.

## Scope

**In:**
- In both templates (`templates/grid/crates/game`,
  `templates/blank/crates/game`):
  - every screen of the game's declares `Screen::actions`;
  - a `screens::all`-style function lists every screen the game can
    show, the engine's it opens included;
  - the tools implement `GameTools::controls` with `GameControls`.
- Each template's keymap file gets a table in `targets` for each target
  its `shuyi.ron` lists that has no keyboard. Today they list only
  `desktop` and `web`, so likely none. Check, and add a comment to the
  keymap file saying where a target's table goes.
- A test in each template: every listed screen declares its actions, and
  `GameControls::fit` finds nothing on each target of its `shuyi.ron`.

**Out (do not do):**
- adding targets to the templates' `shuyi.ron` (the wizard's targets
  are 2504);
- the template's Playdate part (2504 adds `check --target playdate` to
  its CI);
- engine changes.

## Implementation steps

1. Read `examples/hello-grid/src/screens.rs` (`all`, the `actions`
   overrides), `examples/hello-grid/src/tools.rs` (`controls`) and
   `examples/hello-grid/tests/it/controls.rs`.
2. Do the same in `templates/grid`, then in `templates/blank`.
   Templates aren't formatted by `cargo fmt --all`: format them as CI's
   template job does.
3. Run `cargo shuyi new grid <dir>` and `cargo shuyi new blank <dir>`,
   then `cargo shuyi check --target all` in each. Its limits no longer
   say the controls aren't checked.

## Acceptance criteria

- [x] In both templates, every screen declares its actions.
- [x] `cargo shuyi check --target all` in a new game of either template
      passes, and its limits don't say the controls aren't checked.
- [x] The templates' own tests pass in CI's template job.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit or integration, in each template: the controls fit every target
  of its `shuyi.ron`, and every listed screen declares its actions.

## Completion notes

- Both templates' screens declare their actions: grid's stage all six,
  blank's title only Cancel.
- Each template's `lib.rs` has `screens(ctx)`, every screen the game can
  show (the `screens::all` of hello-grid; neither template has a
  `screens` module, and neither opens an engine screen yet).
- Each template's `tools.rs` has `controls()` (a `GameControls` of the
  keymap and those screens) and `GameTools::controls` returns it.
- Both keymap files get an empty `targets: {}` with a comment saying
  where a target without a keyboard puts its buttons. The templates list
  only `desktop` and `web`, which have keyboards, so no table is needed.
- A test in each template: the screens listed, each declaring its
  actions; `GameControls::fit` empty on every target of its `shuyi.ron`;
  and `check --target all` passing without the "controls aren't checked"
  limit.
- No engine change, no follow-up ticket. Nick input: none.
