+++
title = "A game and its screens"
description = "Screens on a stack, the transitions between them, the context they share, and text by key."
weight = 20

[extra]
chapter = 2
+++

A game on Shuyi Engine is a stack of **screens**. Only the screen on top
reads input; the screens below it wait. A title, a stage, a menu over
the stage and a question over the menu are each a screen. They share
one **context**, `Ctx`, which holds the game's data and the engine's
services.

## A screen

A screen implements `Screen`. Here is Moon Garden's title, the whole of
it:

{{ snippet(file="src/screens.rs", anchor="title") }}

- `name` is a stable id in `snake_case`, for tests and the tools (the
  comment tells the text check that it isn't player text).
- `actions` declares every action the screen reads. The tools check
  that they fit each target's buttons, and in tests a screen gets only
  the actions it declares ([chapter 3](@/learn/book/actions.md)).
- `update` runs once a frame while the screen is on top. It reads the
  frame's actions and answers with a `Transition`.
- `draw` paints the screen into the frame. Here it paints the view,
  plain data built by `view`; [chapter 4](@/learn/book/frame-and-skins.md)
  says why the two are apart.

## The stack

`update` answers with what the stack does next:

| `Transition` | Does |
| ------------ | ---- |
| `None` | stays |
| `Push(screen)` | puts a screen on top of this one |
| `Pop` | closes this one: the one below is on top again; the last one quits |
| `Transition::pop_with(value)` | closes this one and hands `value` to the one below |
| `Replace(screen)` | swaps this one for another |
| `Quit` | leaves the game |

A screen whose `is_overlay` is `true` lets the one below show through:
the stack draws the one below first, then it.

## Asking the screen below

A screen can ask a question on a screen of its own and get the answer
back. The garden pushes "Leave the garden?" over itself when the player
presses Cancel:

{{ snippet(file="src/screens.rs", anchor="leave") }}

The question pops with the answer, and the stack hands it to the
screen below in `uncovered`. The garden reads it as the type the two
agree on, here a `bool`:

{{ snippet(file="src/screens.rs", anchor="garden") }}

The transition `uncovered` returns applies at once, so a screen acts on
the answer in the same frame. A screen also hears when one is pushed
over it, in `covered`.

The garden's state isn't the screen's to change: it belongs to the
rules, and the screen changes it only by sending commands.
[Chapter 6](@/learn/book/rules-as-commands.md) writes them.

## The context

Every `update` gets the context `&mut`, and every `draw` gets it `&`.
It is a plain struct, with no globals behind it:

- `ctx.game` is the game's data, the type the game names (`Data`); on
  the short path a `BasicData`, whose `own` is the game's own part;
- `ctx.text(key)` and `ctx.text_with(key, args)` give text by key in the
  player's language;
- `ctx.palette` gives colors by name, `ctx.keymap` the player's keys,
  `ctx.storage` the player's saves ([chapter 7](@/learn/book/saves-and-settings.md)),
  `ctx.settings()` the player's settings;
- `ctx.audio` takes the sounds a screen asks for, and `ctx.log(…)` writes
  a line to the engine's log.

## Text by key

No screen writes its text in its code. The text is in the game's
language file, `files/en.ron`, by key:

{{ snippet(file="files/en.ron", anchor="title", language="ron") }}

`ctx.text("title.name")` is a key's text. `ctx.text_with(key, args)`
fills its placeholders: each `{name}` that `args` names, as in
`&[("seeds", &seeds)]`, and each action's name, as `{Confirm}`, with
the player's own key for it. A key the file lacks panics in a debug
build, naming the key. `cargo shuyi lint` finds text written in code.
The keys `keys.not_mapped`, `keys.arrows`, `keys.chord_join` and
`layout.arrows` are words the engine's own screens and help lines ask
the game for.

## Every screen, on a list

`game()` names every screen the game can show (its `.screens(…)`, in
[chapter 1](@/learn/book/quick-start.md#what-it-is-made-of)).
`cargo shuyi check --target` checks the controls of each screen on that
list, so a new screen goes on it.

## The engine's own screens and widgets

Before writing a screen, look for it in the engine: the options screen,
key rebinding, the layout picker and player profiles are in
`shuyi::ui::screens`, and a menu, a list's focus, a question box, name
entry and word wrap are in `shuyi::ui::widgets`. Each takes the game's
rows or text, and says what it shows as a view.

## Modules

The default engine stays small. Everything else is a **module**: a
crate of the engine that a game turns on as a feature of `shuyi`, such
as physics or online play. `cargo shuyi modules` lists them by concern
(perspective, bodies and motion, online play, kind of game), each with
its status, and `cargo shuyi add <module>` turns one on in the game's
`Cargo.toml` and copies its skill into `.claude/skills/`. A game
installs the modules it turned on once, with
`.modules(shuyi::enabled_modules)` on its `BasicGame`, so turning one on
needs no change to its code. A module joins the running game with hooks
of its own, and keeps its state on the context, where
`ctx.module_state::<T>()` finds it.

## A test of it

A test plays the game with no window through the engine's **Harness**,
pressing the game's default keys by name, and reads the stack:

{{ snippet(file="src/screens.rs", anchor="harness") }}

[Chapter 8](@/learn/book/testing.md) has the rest of the Harness.

## When a game outgrows the short path

The short path's parts grow one at a time, and the game's start stays
as it is:

- **actions of its own**: an action set of the game's in place of
  `UiAction` ([chapter 3](@/learn/book/actions.md#actions-of-a-game-s-own));
- **a part of its own**: a map, a table of rules, read by the function
  `BasicGame::with_own` takes ([chapter 5](@/learn/book/assets-and-formats.md));
- **tools of its own**: a scene, records and bots, through a `GameTools`
  of its own ([chapter 10](@/learn/book/tools-for-agents.md)).

A game whose data needs more than `BasicData` has (sounds, voices,
color themes, settings of its own) writes its own `GameData`. Each
method hands the engine a part it reads:

{{ snippet(file="src/screens.rs", anchor="game_data") }}

Such a game makes its context with `Ctx::new(data)`, its game with
`Game::new(ctx, first_screen, GameHooks::default())`, and its binary
calls `shuyi::app::run` with its own window settings. The API docs of
`shuyi_ui` and `shuyi_app` have the details.
