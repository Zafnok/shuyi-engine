+++
title = "Actions, never keys"
description = "Screens react to actions; the keymap file gives their keys and buttons; text names a key through the player's keymap."
weight = 40

[extra]
part = "chapters"
chapter = 4
+++

{% newcomer(to="shuyi", topic="why a game never names a key") %}
The first thing you would write in a game is "if Enter was pressed, plant a seed". It
works, and it is a bug waiting to happen. Players rebind every key and button, a
controller has no Enter, and two keyboard layouts already use different keys. A key
written into a game's code, or into text a player reads, stops working after a rebind,
or it lies: the help line says "press Enter", and Enter does nothing.
{% end %}

The code never asks about keys. A screen reacts to the game's **actions**, the things
a player can ask for (Up, Confirm, Plant). The **keymap** file says which keys and
buttons give them, and text names a key through the player's keymap. Then every player
can change their keys, a controller just works, and the help line is always right:

{{ try_it(name="book-rebind") }}

Only one file changed. The help line is text that asks the keymap for the key, so it
followed.

## Reacting to an action

A screen's `update` gets a `FrameInput` of the game's actions: `input.actions` holds
those pressed this frame, and `input.is_held(action)` says whether one is held down. A
held action that repeats comes again at the keymap's repeat rate, so a screen never times
key presses itself. When the game comes back from a suspend (the device slept, or the
player switched apps), or its window loses focus, nothing is held any more: a key let go
while it was away doesn't stay held, and one still down counts once it is pressed again.
While the window has no focus the controllers do nothing. The garden's `update`, in
[chapter 6](@/learn/book/games-and-screens.md#asking-the-screen-below), turns each action
into a command of its rules.

Each screen declares the actions it reads, in `Screen::actions`. `cargo shuyi check
--target all` checks that the required ones bind to distinct inputs of each target the
game ships on, and in tests the Harness hands a screen only the actions it
declares.

{% newcomer(to="shuyi", topic="a target") %}
A **target** is a kind of machine a game runs on, and "distinct" means no
two need the same button.
{% end %}

A screen that declares none (`None`) counts as reading every action, and the
check warns about it.

## The keymap file

`files/keymap.ron` gives each action, by its name, up to three key **chords** in each
**layout**. A chord is a key with any keys held with it, such as `Shift+a`, or just `a`,
and a layout is one complete set of keys for a player to pick:

{{ snippet(file="files/keymap.ron", anchor="layouts", language="ron") }}

It also gives each action's controller buttons, named by their place on the pad (`South`,
`East`, `DpadUp`), never by a label such as "A", since the same place has a different
label on another make of pad. Then come the timings of repeated keys, and how far a stick
goes before it counts as pushed:

{{ snippet(file="files/keymap.ron", anchor="pad", language="ron") }}

Plain Escape and Delete may not appear: the rebinding screen keeps them, Escape to back
out of a capture and Delete to empty a slot. `cargo shuyi check` reads the file and names
a wrong line. On a Nintendo-style pad the engine swaps Confirm and Cancel by itself.

## Naming a key in text

In the language file, an action's name in braces shows the player's own key for it, and
`{Cursor}` the four direction keys:

{{ snippet(file="files/en.ron", anchor="garden", language="ron") }}

`ctx.text_with("garden.help", &[])` gives "arrows move · Enter plant · Backspace leave"
with the default keys, and other words once the player rebinds. Text built in code names
a key with `key_name`, and passes it in as an argument:

{{ snippet(file="src/actions.rs", anchor="key_name") }}

{% newcomer(to="rust", topic="lending with `&`, and a tuple") %}
`&Ctx<Data>` lends the function the context, to look at. `&[("key", &key)]`
lends a list of pairs, each a **tuple**, values side by side: here the placeholder's
name, and what to put there.
{% end %}

A key's name reads like a word when it is printed like one ("Enter plant"). To make it
stand out, a screen asks for the text with `ctx.text_styled` instead: the same words, but
each key's name marked as a key. Its skin prints it with `print_styled`, which takes the
colours of each part from the palette:

{{ snippet(file="src/actions.rs", anchor="styled") }}

The palette decides how a key looks: a colour named `"key"` in `files/palette.ron` is the
keys' text, and `"key_bg"` the cells behind them. A palette with neither prints the keys
like the words around them. A game made with `shuyi new` has a `"key"` colour already,
and its title and stage print their help with `print_styled`, so their keys stand out
from the first run; change the colour, or add a `"key_bg"`, in `files/palette.ron`. A key's name passed in as an argument, as `plant_hint` does,
is marked too when it goes to `ctx.text_styled_with_keys` as one of its key arguments.

A game can draw each key as a key cap instead: its name in a small rounded box, in bold, a
single letter as its capital (`F`, not `f`). It turns them on in its `GameData`, with
`fn key_caps(&self) -> Option<KeyCapStyle> { Some(KeyCapStyle::new()) }`
(`shuyi::ui::frame::key_cap`). The box is the palette's `"key_bg"`, its edge
`"key_edge"`, the name `"key"`; the style sets the bold, the capitals and the margin. Every
hint printed with `print_styled` gets caps, the engine's own screens' too. A cap takes a
cell more than its name, so a hint with caps is wider: a skin that fits one by measuring it
with the frame's widths, `help.width(buf.widths())`, fits it with caps.

The cursor's keys can be a picture: when the four cursor actions are on the arrow keys,
`{Cursor}` is one span in the role `TextRole::Arrows`, and a style with
`.with_arrows(image, part)` draws it as that part of an image (the arrow cluster your game
draws), one line tall, in the cells its width needs. Any other four keys are four caps,
`w a s d`. Without a picture, the arrows are a cap of the language's word for them.

Bold is the font's own bold face when the atlas has one, made with `font-atlas --bold`
from a bold font of the same cell size (Terminus Bold, `ter-u16b.bdf`, beside Terminus):

```text
cargo shuyi font-atlas --bold files-src/ter-u16b.bdf files-src/ter-u16n.bdf crates/game/files/font
```

Without one, each letter is drawn twice, a pixel apart.

So a placeholder's first letter says who fills it. A capital, `{Confirm}`, names a key: an
action of the game, or `{Cursor}`. Lowercase, `{seeds}` or `{key}`, is a value the screen
passes. `cargo shuyi check` fails on a capitalized name that is no action.

Both follow the device the player pressed last: keys on the keyboard, the pad's own
button names on a controller. An optional action may have no key; the text then shows the
language's `keys.not_mapped` words.

`cargo shuyi lint` fails on a key named in the game's code or text, with its file and line
(problem code [`SH2101`](@/learn/errors/sh2101.md)).

## Rebinding

The engine's Key bindings screen (`shuyi::ui::screens::key_bindings`) lets the player
rebind three key slots and three button slots for each action, and saves them in the
player's profile. It refuses to close while a required action has no key or no button. A
keymap file with several layouts lets a player pick one on the layout picker the first
time the game starts.

You don't write any of this. It works since your code only ever asked for actions.

## The mouse and taps

A mouse or a finger points at the engine's menus: the row under it takes the focus as it
moves, and a click or a tap on a row does what Confirm does there. It is no key, so
nothing binds it, and it is never the only way: a menu takes its keys as before, and a
target with no pointer (the Playdate) loses nothing. A screen of your own hands its menu
the frame's pointer beside its actions, with `menu.point_with_sound(input.pointer(), &mut
ctx.audio)`, and on a grid map `skin.tile_at(&scene, area, pixel)` gives the tile under
it. A pointer moves the cursor and gives the click; only the command a key would send
reaches the rules, so a game played with the mouse keeps the same record.

## Actions of a game's own

The short path plays with `UiAction`'s six moves, which the engine's menus and screens
react to too. A game that needs more (a watering can, a debug key) writes its own action
enum, and implements `ActionSet` for it. The engine may add moves to `UiAction`, so a
`match` on it ends with a `_` arm.

{% newcomer(to="rust", topic="an enum and a trait") %}
An **enum** is a type that is one of a fixed list. A
**trait** like `ActionSet` is a list of functions that a type promises to have.
{% end %}

{{ snippet(file="src/actions.rs", anchor="action_set") }}

- `ALL` lists every action in order, and `name` gives each one's name in the keymap file
  and the player's saved keys;
- `is_repeatable`, `is_required` (the player must keep a key and a button on it) and
  `is_rebindable` say how each behaves;
- `group` says which actions may share a key (`()` when none may);
- `ui` says what the engine's widgets take each for, so menus keep working;
- `fixed_keys` gives keys no player can move. `UiAction` has one, plain Escape for Cancel;
  this game has none.

{% newcomer(to="rust", topic="`Self`, `&'static` and `Copy`") %}
`Self` is the type this `impl` is for, so `Self::Left` is `GardenAction::Left`.
`const ALL: &'static [Self]` is a fixed list, kept for as long as the program runs.
`self` with no `&` takes the value itself, which is cheap here, as `Copy` in the
`derive` line says a value this small is simply copied.
{% end %}

{% newcomer(to="rust", topic="`matches!` and `type Group = ()`") %}
`match` handles every possibility. `matches!(self, Self::Left | Self::Right)` is a short
way to ask "is it one of these?". `type Group = ();` fills in one of the trait's blanks,
with `()`, a value holding nothing. So `fn group(self) {}` has nothing to give back, and
an empty body does it.
{% end %}

The game's data becomes `BasicData<GardenAction, Plan>` and its game `BasicGame<GardenAction,
Plan>`, and its keymap file names every action of `ALL`:

{{ snippet(file="actions/keymap.ron", anchor="keymap", language="ron") }}

An engine screen declares the game's actions for the `UiAction`s it reacts to with
`shuyi::ui::input::ui_actions`.

## Tests

Harness scripts press the game's default keys by name (`h.keys("Enter Right")`),
`h.pad("South")` presses a button by its place, and `h.act(action)` presses whatever key
the player bound to an action. A test of rebinding rebinds in the test, then presses the
new key. `h.click_text("Play")` clicks a menu row by its words, and `h.point(x, y)` and
`h.click(x, y)` move and click the pointer in console pixels.

## When it goes wrong

`cargo shuyi check` reads the keymap and the language file, and these are its real
answers.

A misspelled action. It says what it knows, and that the real one is now missing:

```text
check: 2 error(s)
  files/keymap.ron:17: error[SH2008]: layout "Arrows": unknown action "Confirn"; known actions: Up, Down, Left, Right, Confirm, Cancel
  files/keymap.ron: layout "Arrows": missing action "Confirm" (list it with [] to leave it unbound)
help[SH2008]: use one of the names the message lists, or fix the spelling
```

A misspelled key:

```text
  files/keymap.ron:18: error[SH2008]: layout "Arrows": action "Cancel": unknown key chord "Backspce"; expected a key such as "h", "0", ";", "Left", "Enter", "Escape", "Space", "Tab", "Backspace" or "F1", optionally prefixed by "Shift+"
```

A key the rebinding screen keeps:

```text
  files/keymap.ron:18: layout "Arrows": action "Cancel": "Escape" can't be bound here: Esc and Delete are fixed, see controls.md
```

An action misspelled in the language file. `check` lists the actions and
`{Cursor}`, as the names a capitalized placeholder may use:

```text
check: 1 error(s)
  files/en.ron:15: error[SH2008]: "title.help": unknown action "{Confirn}"; known actions: Up, Down, Left, Right, Confirm, Cancel, and {Cursor} for the cursor keys
help[SH2008]: use one of the names the message lists, or fix the spelling
```

`cargo shuyi lint` reads your code and your text, and finds a key named in either:

```text
lint: 1 finding(s)
  crates/game/files/en.ron:11: error[SH2101]: text tells the player to press "Enter": "Press Enter" — react to an action (the keymap names the keys), or name a key through the player's keymap
help[SH2101]: react to an action instead, and name a key in text through the player's keymap
```

One more reaches neither:

**A screen reads an action it didn't declare.** Take `UiAction::Left` off the garden's
`actions`, keep reading it in `update`, and in a test the Harness holds it back, so Left
does nothing. A test that walks left then fails, with the plot it stayed on:

```text
assertion `left == right` failed
  left: 2
 right: 1
```

If a key does nothing in a test, look at the screen's `actions` first.

## Check yourself

{% check() %}
The language file has a help line with `{Confirm}` in it. What does the player read in
its place?

- [x] The key or button the player has for Confirm, from the device they pressed last. — A capitalized name in braces names an action, and the text asks the player's keymap for its key ([naming a key in text](#naming-a-key-in-text)).
- [ ] The word "Confirm". — The braces are filled in: a capitalized name shows the player's own key for that action ([naming a key in text](#naming-a-key-in-text)).
- [ ] Always the default key, Enter. — It shows the key the player bound, so the line stays right after a rebind ([naming a key in text](#naming-a-key-in-text)).
- [ ] A value the screen passes in as an argument. — That is a lowercase placeholder, such as `{seeds}`; a capital names a key ([naming a key in text](#naming-a-key-in-text)).
{% end %}

{% check() %}
The player holds Right down. How does a screen move the cursor once per repeat?

- [ ] It reads the clock and moves again once enough time has passed. — The repeats come with the actions at the keymap's rate, and a screen never times key presses itself ([reacting to an action](#reacting-to-an-action)).
- [x] It does nothing extra: the held action comes again in `input.actions` at the keymap's repeat rate. — The keymap file gives the repeat timings, so a screen never times key presses itself ([reacting to an action](#reacting-to-an-action)).
- [ ] It checks `input.is_held(action)` and moves on every frame it is held. — That would move once a frame, far faster than a repeat; repeats already arrive in `input.actions` ([reacting to an action](#reacting-to-an-action)).
{% end %}

{% check() %}
A screen's `update` reads `UiAction::Left`, but its `actions` leaves Left out. In a
Harness test, what happens when Left is pressed?

- [ ] Left works as usual, since `actions` is only read by `cargo shuyi check`. — In tests the Harness holds back every action a screen doesn't declare ([reacting to an action](#reacting-to-an-action)).
- [ ] The test stops at once with an error naming the undeclared action. — Nothing names it: Left is held back, and the test fails later on what didn't change ([when it goes wrong](#when-it-goes-wrong)).
- [x] Left does nothing, since the Harness hands a screen only the actions it declares. — So a test that walks left fails on the plot it stayed on ([when it goes wrong](#when-it-goes-wrong)).
{% end %}
