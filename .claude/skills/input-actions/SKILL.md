---
name: input-actions
description: Rules for anything that reads input or names a key or button to a player — engine and game code react to actions, never keys; every binding is the player's. Use when adding or changing input handling, an ActionSet, UiActions, keymap files, rebinding, controllers, pointer or analog input, help text or any text that names a key.
---

# Input: actions, never keys

Players rebind every key and button. A key written into engine or game
code, or into text a player reads, is a bug: it stops working or lies
after a rebind, and different layouts already use different keys. These
rules come from the game this engine came from (game ADR-0015,
ADR-0031, ADR-0034, ADR-0053). The game's ticket 1106 made them generic:

- **`ActionSet`** is the trait a game implements for its own actions:
  `ALL`, `name`, `from_name`, `is_repeatable`, `is_required`,
  `is_rebindable`, `fixed_keys`, and `ui()`.
- **`UiAction`** is the engine's own small set that widgets react to: Up,
  Down, Left, Right, Confirm, Cancel, Debug. A game maps its actions onto
  them through `ActionSet::ui`.
- **Generic types**: `Keymap<A>`, `InputState<A>`, `FrameInput<A>`,
  `PlayerKeys<A>`, `HelpKeys<A>`.
- **Layouts are data.** Named layouts live in the keymap file, never in an
  enum.

## Only these places know about keys and buttons

- `shuyi-app`'s key and pad readers turn platform codes into the engine's
  `Key`, `Chord` and `Button`.
- The keymap file *format* (`shuyi-assets`) and the input runtime
  (`shuyi-ui::input`).
- Test code.
- The rebinding screen, which captures keys through the helpers it is
  given.

Everywhere else, code reacts to `FrameInput` actions (`is_pressed`,
`is_held`) and nothing else. `check-keys` enforces this once the code
arrives (game ticket 1117 moves it).

## Rules

1. **React to actions, not keys.** Need a new kind of input? The *game*
   adds an action to its `ActionSet`; the engine adds a `UiAction` only
   if every game's widgets need it, and with an ADR.
2. **Name keys in text through the keymap.** Help bars use `HelpKeys`
   (`key_name`, `help_line`), which name the key or button for the device
   pressed last. Data text uses `{ActionName}` placeholders. Never write
   "F", "[Space]", "Esc", "arrows" or "WASD" into player text.
3. **Expect any key, or none.** An optional action may have no key; text
   shows the language's "not mapped" word. Two actions may share a key in
   one layout.
4. **Defaults belong to the game.** An engine example's defaults live in
   its own keymap file. The engine never picks a game's keys.
5. **Fixed keys are data.** A key that can't be rebound (the game's
   Escape-is-Cancel) is the `ActionSet`'s `fixed_keys`, never a match in
   engine code.
6. **Held and repeated input** comes from `InputState` (repeat timings in
   the keymap file). Don't time presses in a screen.
7. **Controllers are keys too.** Every rule applies to buttons. Code and
   data name buttons by position (`South`, not "A"). Only the pad layer
   holds button names per controller kind.
8. **Pointer and analog input** (parked ticket 1121) become rebindable
   actions and axes, quantized to integers for deterministic records.

## Tests

- Unit tests of screens feed `FrameInput`s of actions.
- Harness scripts may press default keys of the example or toy game.
- A test about rebinding rebinds in the test and presses the new key.

## Checklist before pushing input work

- [ ] No key or button types outside the allowed places.
- [ ] No key names in player text or data; helpers and placeholders are
      used.
- [ ] Works with a rebound key and with no key for optional actions.
- [ ] Text that names a key reads right on a controller.
- [ ] `check-keys` passes, once it exists here.
