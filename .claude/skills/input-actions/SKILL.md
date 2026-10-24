---
name: input-actions
description: The engine's own side of input — which engine crates may know keys and buttons, adding a UiAction, the generic input types, and the engine's examples and tests. The rules every game keeps (actions, never keys; the keymap; help text from it; controllers; targets' buttons) are the shipped skill shuyi-input. Use when changing the engine's input handling, an ActionSet, UiActions, keymap files, rebinding, controllers, pointer or analog input, help text or any text that names a key.
---

# Input in the engine

Every input rule that holds in every game is the shipped skill
[`shuyi-input`](../../../skills/shuyi-input/SKILL.md): react to actions,
never keys; name keys through the player's keymap; expect any key or
none; the defaults are the game's, in its keymap file; fixed keys are
data; held and repeated input comes from the input state; controllers
are keys too, named by position; screens declare their actions, and
`cargo shuyi check --target` checks they fit; a target without a
keyboard has its buttons in the keymap file. Read it first: the engine's
own screens, widgets and examples keep those rules too. This skill holds
only what is the engine's own. The rules come from the game this engine
came from (game ADR-0015, ADR-0031, ADR-0034, ADR-0053), made generic by
its ticket 1106.

## Only these places know about keys and buttons

- `shuyi-app`'s key and pad readers turn platform codes into the engine's
  `Key`, `Chord` and `Button`.
- The keymap file *format* (`shuyi-assets`) and the input runtime
  (`shuyi-ui::input`).
- Test code.
- The rebinding screen, which captures keys through the helpers it is
  given.

Everywhere else, engine code reacts to `FrameInput` actions (`is_pressed`,
`is_held`) and nothing else. `cargo shuyi check-keys` enforces this in a
game's code; it doesn't scan the engine's own code yet (ticket 2006).

## The engine's own rules

1. **A new `UiAction` only if every game's widgets need it**, and with an
   ADR. A game that needs more input adds an action to its own
   `ActionSet`.
2. **An engine example's defaults live in its own keymap file.** The
   engine never picks a game's keys.
3. **The input types are generic over a game's actions**: `Keymap<A>`,
   `InputState<A>`, `FrameInput<A>`, `PlayerKeys<A>`, `HelpKeys<A>`.
4. **Pointer and analog input** (parked ticket 1121) become rebindable
   actions and axes, quantized to integers for deterministic records.

## Tests

`shuyi-input`'s tests section holds for the engine's tests too. Harness
scripts in the engine may press the default keys of the example or toy
game.

## Checklist before pushing input work

- [ ] No key or button types outside the places above.
- [ ] `shuyi-input`'s checklist, for each engine screen, widget and
      example the change touches.
- [ ] `check-keys` passes on the engine's own code, once it scans it.
