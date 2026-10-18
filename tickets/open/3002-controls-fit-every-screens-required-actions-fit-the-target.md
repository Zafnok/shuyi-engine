---
id: "3002"
title: "Controls fit: every screen declares its actions, and each screen's required actions must bind to distinct inputs of the target"
type: feature
milestone: After MVP
model: opus-5.5
effort: high
status: todo
blocked_by: ["3001"]
nick_input: none
completed:
---

# 3002 — Controls fit the target

## Context

Nick, 2026-10-15 (`docs/decisions.md`): on the Playdate the game must
play with "dpad/a+b/lever only", and "if there are more buttons
necessary (not optional) in any context it simply cannot run on
playdate". [ADR-0008](../../docs/adr/0008-export-targets-profiles-as-data-platform-backends-controls-that-fit-one-bit-and-budgets.md)
§3 turns that into a check that runs for every target. With the
`steam-deck` profile, the same check is Deck Verified's "the default
controller configuration reaches all content".

Read the `input-actions` skill first. `ActionSet::is_required` already
exists (the game marks 7 of its 18 actions required). Named keymap
layouts are data in the keymap file.

## Nick input

None.

## Scope

**In:**
- **`Screen::actions(&self) -> &[A]`** (or a returned set): the actions
  this screen reacts to.
  - The default is every action of the game's `ActionSet`, so that old
    screens still compile. A screen that doesn't override it gets a
    `check` warning.
  - Engine screens and widgets declare theirs.
  - Overlays and popups are screens too, and are checked on their own.
- **A Harness guard.** In tests, the screen stack asserts that a screen
  never reads an action it didn't declare (`FrameInput` is filtered and
  logs the miss). This keeps the declarations honest.
- **The check**, in `shuyi check --target <id>`. For each screen and each
  target in the game's manifest:
  1. the game's keymap file has a layout named after the target id;
  2. each required action the screen declares has at least one input in
     that layout that the target's profile offers (`input.buttons`,
     `input.chords`, discrete axis inputs such as `CrankForward`);
  3. no two required actions of the same screen share an input in that
     layout;
  4. no binding in that layout uses an input the profile lacks.

  Each failure names the screen, the actions and the inputs.
- **What doesn't count** (ADR-0008 §3): long and double presses, chords
  the profile doesn't list, system menu slots for required actions, and
  inputs the system reserves (the Playdate's Menu and Lock buttons, the
  Deck's Steam and Quick Access buttons).
- **Optional actions bound to system menu slots.** A layout may bind an
  optional action to `SystemMenu(n)` when the profile has that slot. The
  check fails above `system_menu_slots`.
- **`shuyi export <id>` refuses a game that fails** (once 3004 lands;
  either order works, and whichever lands second adds the refusal).
- **The examples** get `playdate` and `steam-deck` layouts, and the
  check passes on them.

**Out (do not do):**
- folding actions onto fewer buttons automatically (ADR-0008 says no);
- on-screen touch controls (3015);
- the new-game wizard's Playdate part. Either order: if 2504 is done,
  add this check to the CI of the built-in template's `target-playdate`
  part in this PR; if not, 2504 adds it;
- the crank as an analog axis (it waits for parked 1121; `CrankForward`
  and `CrankBack` are discrete inputs here).

## Implementation steps

1. Add the profile's input names to `shuyi-ui::input`: `Button` already
   has positions; add `CrankForward`, `CrankBack` and `SystemMenu(u8)`
   as `Input`s the keymap file can name. Saved bindings get a new
   version with a migration.
2. Add `Screen::actions`, declare it on every engine screen, and add the
   Harness guard.
3. Write the check as a pure function: `controls_fit(profile, layout,
   screens) -> Vec<FitProblem>`. Then wire it into `check --target`.
4. Add the example layouts and a fixture game with a screen that needs
   eight required actions.

## Acceptance criteria

- [ ] A fixture screen with 7 required actions (4 directions, Confirm,
      Cancel, End turn) passes on `playdate` with End turn on the A+B
      chord, and passes with End turn on `CrankForward`.
- [ ] Adding an 8th required action to that screen fails on `playdate`,
      naming the screen and the action.
- [ ] Two required actions on one input fail. The same two on different
      screens pass.
- [ ] A required action bound only to `SystemMenu(0)` fails. An optional
      one passes. A fourth system menu slot fails.
- [ ] A screen reading an undeclared action fails its Harness test.
- [ ] `check-keys` passes.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each rule of `controls_fit`, passing and failing.
- Property: random layouts, where the check fails exactly when some
  screen has two required actions on one input or one with none.
- Integration: `shuyi check --target playdate` and `--target steam-deck`
  on the examples.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
