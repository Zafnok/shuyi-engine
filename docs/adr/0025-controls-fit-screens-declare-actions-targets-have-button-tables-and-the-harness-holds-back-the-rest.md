# ADR-0025: Controls that fit: screens declare their actions, targets have button tables, and the Harness holds back the rest

- **Status:** Accepted
- **Date:** 2026-10-20
- **Related tickets:** 3002; game 1034 and 1035 (the Playdate's buttons,
  crank and system menu)
- **Extends:** ADR-0008 §3

## Context

ADR-0008 §3 asks for a check: on every target, the required actions of
each screen bind to distinct inputs of the target's profile. It reads
"the game's keymap layout named after the target". Three facts shaped
how that was built:

- **A keymap's layouts are keyboard layouts.** `layouts` in the keymap
  file are key chords the player picks between (`RightHanded`). The
  controller buttons are one `pad` table shared by every layout, plus
  the `playdate` table of game ticket 1034, which a Playdate uses in
  place of `pad`. Layout names allow letters, digits and `_`, and target
  ids use `-` (`steam-deck`).
- **Screens read their input as a list.** A screen loops over
  `FrameInput::actions`, a public field, and matches. Nothing can tell
  which of them it "reads".
- **`shuyi` knows no game.** Only the game's tool program knows its
  screens (`GameTools`), and a screen can need the game's context to be
  built.

## Decision

1. **A target's layout is a table of buttons.** The keymap file gets an
   optional `targets` table: target id → a complete table of default
   buttons, validated as `pad` is. `KeymapDef::targets` holds them,
   `target_buttons(id)` reads one and `for_target(id)` plays with it in
   place of `pad`. The older `playdate` table is `targets."playdate"`,
   and giving both is an error. `KeymapDef::playdate` stays, for the
   game's code that reads it. The keymap's baked form changes, so the
   pack format is version 2.
2. **The system menu's items are buttons.** `Button::SystemMenu0` to
   `SystemMenu7`, pressed and let go when the player picks the item, as
   a crank notch is. A table may bind them, never in a chord. A profile
   counts them in `system_menu_slots`, never in `buttons`. The system's
   own buttons (Menu, Lock, Steam, Quick Access) have no `Button`, so no
   keymap can name them. Saved bindings name buttons by name, so the new
   ones need no new version of the saved file.
3. **Screens declare.** `Screen::actions(&self) -> Option<Vec<A>>`.
   `None`, the default, counts as every action, and the check warns.
   Engine screens declare the game's actions for the `UiAction`s they
   react to (`input::ui_actions`, with each widget's `ACTIONS`).
4. **The Harness holds back the rest.** `ScreenStack::guard_actions`
   (on in every Harness) hands a declaring screen only its actions,
   pressed and held (`FrameInput::declared`), and logs the others
   (`Harness::undeclared`). Asking `is_held` about an undeclared action
   panics. A screen reacting to an action it didn't declare doesn't
   react in its tests, so they fail. A game's own stack never filters.
5. **The check is pure, in `shuyi-ui`.** `input::fit::controls_fit(profile,
   table, screens) -> Vec<FitProblem>` fails a binding on an input the
   profile lacks, a required action with no input it offers (system menu
   items don't count), and two required actions of one screen on one
   input. `GameControls` holds a game's keymap and its screens.
6. **A target with no table.** It plays with the keys if its profile
   has a keyboard (`desktop`, `web`), and the check passes. Without a
   keyboard it fails.
7. **The tools ask the game.** `GameTools::controls() ->
   Option<Result<Box<dyn Controls>, String>>`. `check --target` runs in
   the game's tool program when there is one. A game that offers no
   controls is checked on its display alone, and the answer says so in
   its limits. A game whose controls don't load fails. Problems come
   back as `TargetCheck::Controls`, naming the screen, the actions and
   the inputs (contract version 4).

## Consequences

- A game adding an eighth required action to a Playdate screen finds out
  from `check --target playdate`, and the message names the screen and
  the action.
- Declarations can't drift silently. A screen whose declaration misses
  an action fails its own Harness tests.
- A game must list its screens for the tools (`hello-grid`'s
  `screens::all`). A screen left out isn't checked. A tour test in the
  example checks the list against what a playthrough opens.
- A target without a keyboard needs a table even when it would copy
  `pad` (the Steam Deck). That is explicit, but a little repetitive.
- Desktop and web aren't checked key by key: a keyboard has keys for
  everything, and the keymap file's own check covers its layouts.

## Alternatives considered

- **Key layouts named after targets** (ADR-0008's wording, taken
  literally) — a Playdate has no keys. Layout names can't hold `-`
  either.
- **Count only what screens read, by wrapping the input in accessors** —
  `actions` is a public field that every screen loops over. Changing it
  breaks every game for no gain over filtering.
- **A `Declared` enum instead of `Option`** — no clearer, and every
  caller would match on it.
- **The check in `shuyi-cli`, over names** — the game's tests couldn't
  call it on their own types, and it would sit outside the layer that
  owns input.
- **A `playdate`-style top-level table per target** — the file's
  fields would be open-ended, and serde would no longer deny unknown
  ones.
