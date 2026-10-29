+++
title = "Actions, never keys"
description = "Screens react to actions; the keymap file gives their keys and buttons; text names a key through the player's keymap."
weight = 30

[extra]
chapter = 3
+++

Players rebind every key and button, and two layouts already use
different keys. So a key written into a game's code, or into text a
player reads, is a bug: after a rebind it stops working, or it lies. A
screen reacts to the game's **actions** (Up, Confirm, Plant), the
**keymap** file says which keys and buttons give them, and text names a
key through the player's keymap.

## Reacting to an action

A screen's `update` gets a `FrameInput` of the game's actions:
`input.actions` holds those pressed this frame, and
`input.is_held(action)` says whether one is held down. A held action
that repeats comes again at the keymap's repeat rate, so a screen never
times key presses itself. The garden's `update`, in
[chapter 2](@/learn/book/games-and-screens.md#asking-the-screen-below),
turns each action into a command of its rules.

Each screen declares the actions it reads, in `Screen::actions`.
`cargo shuyi check --target all` checks that the required ones bind to
distinct inputs of each target the game ships on, and in tests the
Harness hands a screen only the actions it declares. A screen that
declares none (`None`) counts as reading every action, and the check
warns about it.

## The keymap file

`files/keymap.ron` gives each action, by its name, up to three key
chords in each layout:

{{ snippet(file="files/keymap.ron", anchor="layouts", language="ron") }}

and its controller buttons, named by their place on the pad (`South`,
`East`, `DpadUp`), never by a label such as "A", with the timings of
repeated keys and how far a stick goes before it counts as pushed:

{{ snippet(file="files/keymap.ron", anchor="pad", language="ron") }}

Plain Escape and Delete may not appear: the rebinding screen keeps them,
Escape to back out of a capture and Delete to empty a slot. `cargo
shuyi check` reads the file and names a wrong line. On a Nintendo-style
pad the engine swaps Confirm and Cancel by itself.

## Naming a key in text

In the language file, an action's name in braces shows the player's own
key for it, and `{Cursor}` the four direction keys:

{{ snippet(file="files/en.ron", anchor="garden", language="ron") }}

`ctx.text_with("garden.help", &[])` gives "arrows move · Enter plant ·
Backspace leave" with the default keys, and other words once the
player rebinds. Text built in code names a key with `key_name`, and
passes it in as an argument:

{{ snippet(file="src/actions.rs", anchor="key_name") }}

Both follow the device the player pressed last: keys on the keyboard,
the pad's own button names on a controller. An optional action may have
no key; the text then shows the language's `keys.not_mapped` words.

`cargo shuyi lint` fails on a key named in the game's code or text, with
its file and line (problem code
[`SH2101`](@/learn/errors/sh2101.md)).

## Rebinding

The engine's Key bindings screen (`shuyi::ui::screens::key_bindings`)
lets the player rebind three key slots and three button slots for each
action, and saves them in the player's profile. It refuses to close
while a required action has no key or no button. A keymap file with
several layouts lets a player pick one on the layout picker the first
time the game starts.

## Actions of a game's own

The short path plays with `UiAction`'s six moves, which the engine's
menus and screens react to too. A game that needs more (a watering
can, a debug key) writes its own action enum and implements `ActionSet`
for it:

{{ snippet(file="src/actions.rs", anchor="action_set") }}

- `ALL` lists every action in order, and `name` gives each one's name in
  the keymap file and the player's saved keys;
- `is_repeatable`, `is_required` (the player must keep a key and a button
  on it) and `is_rebindable` say how each behaves;
- `group` says which actions may share a key (`()` when none may);
- `ui` says what the engine's widgets take each for, so menus keep
  working;
- `fixed_keys` gives keys no player can move. `UiAction` has one, plain
  Escape for Cancel; this game has none.

The game's data becomes `BasicData<GardenAction, Plan>` and its game
`BasicGame<GardenAction, Plan>`, and its keymap file names every action
of `ALL`:

{{ snippet(file="actions/keymap.ron", anchor="keymap", language="ron") }}

An engine screen declares the game's actions for the `UiAction`s it
reacts to with `shuyi::ui::input::ui_actions`.

## Tests

Harness scripts press the game's default keys by name (`h.keys("Enter
Right")`), `h.pad("South")` presses a button by its place, and
`h.act(action)` presses whatever key the player bound to an action. A
test of rebinding rebinds in the test, then presses the new key.
