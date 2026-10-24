---
name: shuyi-input
description: Input on Shuyi Engine — a game's code reacts to actions, never keys, and every key and button is the player's to rebind. Covers the keymap file, actions of the game's own, help text that names the player's keys, controllers, targets without a keyboard, and the checks that catch a hard-coded key. Use when reading input in a screen, adding or changing an action, editing `files/keymap.ron`, writing text that names a key or button, or when `lint` reports `SH2101`. Not for building a screen itself (shuyi-screen) or for the rest of the checks before a commit (shuyi-gates).
license: MIT OR Apache-2.0
metadata:
  version: "1.0"
---

# Input: actions, never keys

Players rebind every key and button, and different layouts already use
different keys. So a key written into a game's code, or into text a
player reads, is a bug: it stops working, or lies, after a rebind. A
screen reacts to the game's **actions** (Up, Confirm, Jump), the keymap
file says which keys and buttons give them, and text names a key through
the player's keymap.

## When to use it

- A screen reads input, or a new kind of input is needed (a jump, a
  menu key).
- `files/keymap.ron` changes: default keys, controller buttons, a
  target's buttons, repeat timings.
- Text names a key or a button ("{Confirm} play").
- `cargo shuyi lint` or `check-keys` reports `SH2101`, or `check
  --target` says a screen's controls don't fit.

## When not to

- Building the screen around the input: `shuyi-screen`.
- Running every check before a commit: `shuyi-gates`.

## Steps

1. **React to an action.** A screen's `update` gets a `FrameInput` of
   the game's actions: `input.actions` holds those pressed this frame
   (a held repeatable action comes again at the keymap's repeat rate),
   and `input.is_held(action)` says whether one is held down.

   ```rust
   for &action in &input.actions {
       match action {
           UiAction::Right => self.step(Dir::Right),
           UiAction::Cancel => return Transition::Pop,
           _ => {}
       }
   }
   ```

2. **Declare the actions the screen reacts to** in `Screen::actions`,
   every one it reads: `Some(vec![UiAction::Confirm, UiAction::Cancel])`.
   `cargo shuyi check --target all` checks that the required ones bind to
   distinct inputs of each target. In tests, the Harness hands a screen
   only the actions it declares. `None` counts as every action, and
   `check --target` warns about it.

3. **Bind the defaults in `files/keymap.ron`.** Each action, by its
   name, gets up to three key chords in each layout, and buttons in
   `pad`. Name each button by its place on the pad (`South`, `East`,
   `DpadUp`, `LeftStickUp`), never its label ("A"):

   ```ron
   layouts: [(id: "Arrows", name: "layout.arrows", keys: {
       "Right": ["Right", "d"],
       "Confirm": ["Enter", "Space", "z"],
       "Cancel": ["Backspace", "x"],
   })],
   pad: { "Right": ["DpadRight", "LeftStickRight"], "Confirm": ["South"], "Cancel": ["East"] },
   repeat: (delay_ms: 300, interval_ms: 70),
   ```

   Plain `Escape` and `Delete` may not appear: the rebinding screen keeps
   them (Escape backs out of a capture, Delete empties a slot). `cargo
   shuyi check` reads the file, and names a wrong one by its line.

4. **Name keys in text through the keymap.** In `files/en.ron`, an
   action's name in braces shows the player's own key for it, and
   `{Cursor}` the four direction keys:

   ```ron
   "title.help": "{Confirm} play · {Cancel} leave",
   ```

   A screen reads it with `ctx.text_with("title.help", &[])`. Text built
   in code names a key with `key_name(ctx.help_keys(), action)`
   (`shuyi::ui::input::help_keys`). Both follow the device the player
   pressed last: keys on the keyboard, the pad's own button names on a
   controller.

5. **Check it:** `cargo shuyi lint` (or `cargo shuyi check-keys`) fails
   on a key named in code or text, with its file and line, and `cargo
   shuyi check --target all` on controls that don't fit a target.

## The rules

1. **React to actions, not keys.** A new kind of input is a new action
   of the game's own, never a key read in a screen.
2. **Name keys only through the keymap**: `{Action}` placeholders, or
   `key_name` and `HelpKeys`. Never write "F", "[Space]", "Esc",
   "arrows" or "WASD" into player text.
3. **Expect any key, or none.** An optional action may have no key: the
   text shows the language's "not mapped" words (`keys.not_mapped`). Two
   actions may share a key in one layout.
4. **The defaults are the game's**, in its keymap file. The player
   changes them; the game's code never picks a key.
5. **A key no player can rebind** (Escape cancels, in the short path's
   actions) is the action set's `fixed_keys`, data like the rest, never a
   match on a key in a screen.
6. **Held and repeated input comes from the engine's input state**, with
   the timings in the keymap file (`repeat`). A screen never times key
   presses itself.
7. **Controllers are keys too.** Every rule holds for buttons. Data names
   buttons by their place; the engine swaps Confirm and Cancel on a
   Nintendo-style pad, and turns a stick pushed far enough (`stick`'s
   thresholds) into one held direction button.

## The short path's actions, and leaving them

A game made with `shuyi new` plays with `UiAction`'s six moves: `Up`,
`Down`, `Left`, `Right`, `Confirm` and `Cancel`, every one required and
rebindable, the four moves repeating while held. The engine's own
screens and widgets (menus, the options and key bindings screens) react
to them too.

A game that needs more (a jump, a debug key, a map's own keys) writes
its own action enum and implements `ActionSet` for it:

- `ALL`, every action in order, and `name`, each one's name in the
  keymap file and the saved bindings;
- `is_repeatable`, `is_required` (the player must keep a key and a
  button on it), `is_rebindable`;
- `group`: which actions may share a key (`()` when none may);
- `ui`: the `UiAction` an engine widget takes it for, if any, so menus
  keep working;
- `fixed_keys`: the keys every keymap has on top of the player's.

Then the game's data is `BasicData<MyAction, stages::Own>`, its
`BasicGame<MyAction, stages::Own>`, and `files/keymap.ron` names every
action of `ALL`. An engine screen declares the game's actions for the
`UiAction`s it reacts to with `ui_actions` (`shuyi::ui::input`).

## The player's keys

The player rebinds on the engine's Key bindings screen
(`KeyBindingsScreen::new(ctx, rows)`, `shuyi::ui::screens::key_bindings`):
three key slots per action in each layout, three button slots, saved for
the player. It refuses to close while a required action has no key or
no button. A keymap file with several `layouts` lets the player pick one
on the layout picker; `layout_picker` names the keys that work before
one is picked.

## Targets without a keyboard

A target `shuyi.ron` lists that has no keyboard (a handheld) plays with
the buttons of the keymap file's table named after it, `targets: {
"steam-deck": { "Up": ["DpadUp"], … } }`. `cargo shuyi check --target
<id>` then checks every screen's required actions against that target's
buttons. When they don't fit, redesign the screen: the engine never
folds actions onto fewer buttons to make a game fit.

## Tests

- A screen's unit tests feed it a `FrameInput` of actions
  (`FrameInput::new(actions, dt, held)`).
- Harness scripts (`h.keys("Enter Right")`, `--script "Enter Right"`)
  press the game's default keys by the Harness's names, and
  `h.pad("South")` presses buttons by place. `h.act(action)` presses an
  action's own key.
- A test of rebinding rebinds in the test, then presses the new key.

## Before you commit input work

- [ ] No key or button is named in the game's code outside its tests.
- [ ] No key name in player text; `{Action}` placeholders name them.
- [ ] It works with a rebound key, and with no key for an optional action.
- [ ] Every new or changed screen declares its actions, and `cargo shuyi
      check --target all` passes.
- [ ] Text that names a key reads right on a controller.
- [ ] `cargo shuyi lint` passes.

## When it fails

Branch on the `code`, never the message (`cargo shuyi explain <code>`):

- `SH2101`: code or text names a key. React to an action, or put an
  `{Action}` placeholder in the text.
- `SH2001`, `SH2008` (from `check`): the keymap file doesn't read, or
  names an action, key or button that isn't one; the error gives its
  line.
- `SH3003`: no such target, or the target profiles don't load.
- A `check --target` problem names the screen, the action and the
  target's inputs: bind the action to a button the target has, or
  change the screen.

## Worked example

[`example.ron`](example.ron) runs on a new `grid` game: `Right` steps
the piece; the keymap file then binds Right to `l` and Cancel to `q`,
and with no change to the code `l` steps it, `Right` doesn't, the help
line names `q`, and `q` goes back to the title; a key named in the
stage's text fails `lint` with `SH2101`, an `{Cancel}` placeholder
fixes it, and `check --target all` passes.
