---
id: "3038"
title: "The templates' screens declare their actions, and their tools offer the controls to check --target"
type: feature
milestone: After MVP
model: sonnet-5
effort: medium
status: todo
blocked_by: ["3002"]
nick_input: none
completed:
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

- [ ] In both templates, every screen declares its actions.
- [ ] `cargo shuyi check --target all` in a new game of either template
      passes, and its limits don't say the controls aren't checked.
- [ ] The templates' own tests pass in CI's template job.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit or integration, in each template: the controls fit every target
  of its `shuyi.ron`, and every listed screen declares its actions.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
