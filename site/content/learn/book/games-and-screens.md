+++
title = "A game and its screens"
description = "Screens on a stack, the transitions between them, the context they share, and text by key."
weight = 20

[extra]
chapter = 2
+++

Think of what a game shows you: a title, then the game itself, then perhaps a pause
menu on top of the game, then "Are you sure?" on top of the menu. When you back out,
each goes away and the one below is there again, as you left it.

That is exactly how a game on Shuyi Engine is built. Each of those is a **screen**,
and the screens sit on a **stack**, one on another, like a pile of cards. Only the
screen on top reads the player's input, and the screens below it wait. They all
share one **context**, `Ctx`, which holds the game's data and everything the engine
offers them.

Moon Garden has three screens. This is the second, the garden:

{{ try_it(name="book-garden") }}

## A screen

A screen is a Rust type with a few jobs. Here is Moon Garden's title, the whole of it.
Don't worry about reading all of it at once, as the notes below take it apart:

{{ snippet(file="src/screens.rs", anchor="title") }}

A screen has four jobs:

- `name` is a stable id in `snake_case` (lowercase, words joined by `_`), for tests
  and the tools. The comment above it tells the text check that this isn't text a
  player reads.
- `actions` declares every action the screen reads. An **action** is something a
  player can ask for, like Confirm. The tools check that they fit each target's
  buttons, and in tests a screen gets only the actions it declares
  ([chapter 3](@/learn/book/actions.md)).
- `update` runs once a frame while the screen is on top. It reads the frame's actions
  and answers with a `Transition`, what the stack should do next. Cancel quits only
  where `ctx.platform.can_quit()`: on the web, the player closes the tab instead
  ([chapter 9](@/learn/book/targets-and-export.md)).
- `draw` paints the screen into the frame. Here it paints the view, plain data built
  by `view`. [Chapter 4](@/learn/book/frame-and-skins.md) says why the two are apart.

> **Rust:** a `struct` groups named values into one type, and `TitleView` has three
> `String`s, pieces of text. `pub struct Title;` is a struct with nothing in it, as
> the title has nothing to remember. `#[derive(...)]` asks the compiler to write some
> standard abilities for you, such as `Clone` (make a copy) and `PartialEq` (compare
> with `==`).
>
> `impl Title { ... }` adds functions to a type. `impl Screen<Data> for Title` is
> different: `Screen` is a **trait**, a list of functions a type promises to have, and
> this keeps that promise. `self` is the value a function was called on. `&self` means
> it only looks, and `&mut self` means it may change it, and the compiler holds you to
> that: `draw` takes `&self` and `&Ctx`, so a screen can't change anything while it
> draws. The [tutorial](@/learn/tutorial/01-new.md#the-screen-s-jobs) goes through a
> screen like this one line by line.
>
> `Option<Vec<UiAction>>` is maybe a list of actions, and `Some(...)` means here it is.
> `&&` means "and also". `return` hands back an answer early. `&'static str` is a piece
> of text that's built into the program. `let c = |color| ...` is a small
> nameless function, kept in a name for the next few lines. `i32` is a whole number
> that may be negative, and `i32::from` converts to it.

## The stack

`update` answers with what the stack does next. A `Transition` is an **enum**, a type
that's exactly one of a fixed list of possibilities:

| `Transition` | Does |
| ------------ | ---- |
| `None` | stays |
| `Push(screen)` | puts a screen on top of this one |
| `Pop` | closes this one: the one below is on top again; the last one quits |
| `Transition::pop_with(value)` | closes this one and hands `value` to the one below |
| `Replace(screen)` | swaps this one for another |
| `Quit` | leaves the game |

`Push` holds the screen in a `Box`, written `Box::new(Title)`. Screens are different
types with different sizes, and a box lets the stack hold any of them.

A screen whose `is_overlay` is `true` lets the one below show through: the stack draws
the one below first, then it. That's one word in the code, and you can see it:

{{ try_it(name="book-overlay") }}

## Asking the screen below

Often a screen wants to ask something and act on the answer: "Leave the garden?". You
could put a yes-or-no mode in the garden itself, but then every screen that asks a
question grows a mode. Instead, a question is a screen of its own. The garden pushes
"Leave the garden?" over itself when the player presses Cancel:

{{ snippet(file="src/screens.rs", anchor="leave") }}

> **Rust:** `for &action in &input.actions` runs once for each action this frame. A
> `match` picks the arm that fits, and must handle every possibility, which is why
> there's an arm of `_ => {}`: `_` is "anything else", and `{}` is doing nothing.

The question pops with the answer, and the stack hands it to the screen below in
`uncovered`. The garden reads it as the type the two agree on, here a `bool`, which
is true or false:

{{ snippet(file="src/screens.rs", anchor="garden") }}

The transition `uncovered` returns applies at once, so a screen acts on the answer in
the same frame. A screen also hears when one is pushed over it, in `covered`.

> **Rust:** there's a lot here, so one at a time.
>
> `Self` is the type this `impl` is for, and `Self { garden: ... }` makes one.
> `pub(crate)` means only code in this crate may reach the field. `_` as a parameter's
> name, as in `_: &mut Ctx<Data>`, means a function takes the value and doesn't use it.
>
> `UiAction::Confirm if self.garden.outcome().is_some() => ...` is a match arm with a
> **guard**: it only fits if the `if` is true too, and otherwise the next arm gets its
> chance. `continue` skips to the next time round the loop.
>
> `self.garden.apply(&command)` gives back a `Result`, either a value (`Ok`) or what
> went wrong (`Err`). Rust warns if you ignore one, and `let _ =` says you mean to.
>
> `result.and_then(ScreenResult::take::<bool>)` opens the `Option`, if it's there, and
> takes a `bool` out of it, if that's what it holds. `::<bool>` names a blank in that
> function, the type to look for. If the question had popped with some other type,
> this would be `None`, and the garden would stay.
>
> `as_any` lets a test find this screen on the stack, with its real type. `dyn Any` is
> Rust's "a value of some type, to be asked which". Without it, a test's
> `h.screen::<GardenScreen>()` gets a `None`.

The garden's state isn't the screen's to change: it belongs to the rules, and the
screen changes it only by sending commands.
[Chapter 6](@/learn/book/rules-as-commands.md) writes them.

## The context

Every screen needs to reach things: the game's text, its colors, the player's saves.
Many engines keep these in **globals**, values anyone can reach from anywhere, which
makes it hard to tell what a piece of code depends on, and hard to run two games side
by side in a test. Here they all come in one value, handed to you.

Every `update` gets the context `&mut`, so it may change it, and every `draw` gets it
`&`, so it may only look. It is a plain struct, with no globals behind it:

- `ctx.game` is the game's data, the type the game names (`Data`); on the short path a
  `BasicData`, whose `own` is the game's own part;
- `ctx.text(key)` and `ctx.text_with(key, args)` give text by key in the player's
  language;
- `ctx.palette` gives colors by name, `ctx.keymap` the player's keys, `ctx.storage` the
  player's saves ([chapter 7](@/learn/book/saves-and-settings.md)), `ctx.settings()`
  the player's settings;
- `ctx.audio` takes the sounds a screen asks for, and `ctx.log(…)` writes a line to the
  engine's log.

## Text by key

No screen writes its text in its code. If it did, translating the game would mean
hunting through all of it, and fixing a typo would mean a rebuild. So the text is in
the game's language file, `files/en.ron`, each line with a name, called its **key**:

{{ snippet(file="files/en.ron", anchor="title", language="ron") }}

`ctx.text("title.name")` is a key's text. `ctx.text_with(key, args)` fills its
placeholders: each `{name}` that `args` names, as in `&[("seeds", &seeds)]`, and each
action's name, as `{Confirm}`, with the player's own key for it. In a `grid` game,
`cargo shuyi lint` finds text written in code (a `blank` game's `shuyi.ron` has no
`check_text` yet, and `lint` says so). The keys `keys.not_mapped`, `keys.arrows`, `keys.chord_join`
and `layout.arrows` are words the engine's own screens and help lines ask the game for.

> **Rust:** `&[("seeds", &seeds)]` lends a list of pairs. `("seeds", &seeds)` is a
> **tuple**, values side by side: the placeholder's name, and what to put there. `&[]`
> lends an empty list, for a line with only actions to fill in.

## Every screen, on a list

`game()` names every screen the game can show (its `.screens(…)`, in
[chapter 1](@/learn/book/quick-start.md#what-it-is-made-of)). `cargo shuyi check
--target` checks the controls of each screen on that list, so a new screen goes on it.

## The engine's own screens and widgets

Before writing a screen, look for it in the engine. The options screen, key rebinding,
the layout picker and player profiles are in `shuyi::ui::screens`, and a menu, a list's
focus, a question box, name entry and word wrap are in `shuyi::ui::widgets`. A
**widget** is a part of a screen, as a menu is. Each takes the game's rows or text, and
says what it shows as a view.

## Modules

The default engine stays small, so a game doesn't pay for what it doesn't use.
Everything else is a **module**: a crate of the engine that a game turns on as a
**feature** of `shuyi`, an optional part of a crate, such as physics or online play.
`cargo shuyi modules` lists them by concern (perspective, bodies and motion, online
play, kind of game), each with its status, and `cargo shuyi add <module>` turns one on
in the game's `Cargo.toml` and copies its skill into `.claude/skills/`.

A game installs the modules it turned on once, with `.modules(shuyi::enabled_modules)`
on its `BasicGame`, so turning one on needs no change to its code. A module joins the
running game with hooks of its own, functions the engine calls at set moments, and
keeps its state on the context, where `ctx.module_state::<T>()` finds it.

## A test of it

A **test** is a small program that runs your code and checks what happened. A test of
screens plays the game with no window through the engine's **Harness**, pressing the
game's default keys by name, and reads the stack:

{{ snippet(file="src/screens.rs", anchor="harness") }}

> **Rust:** `#[test]` marks a function for `cargo test` to run. `assert_eq!` stops the
> test, as failed, if its two sides aren't equal. `h` needs `let mut`, as pressing keys
> changes it: in Rust, a name can't change unless it says it may.

[Chapter 8](@/learn/book/testing.md) has the rest of the Harness.

## When it goes wrong

**A screen forgets one of its jobs.** Leave out `draw`, and the compiler says so, and
says what to write:

```text
error[E0046]: not all trait items implemented, missing: `draw`
   --> src\screens.rs:102:1
    |
102 | impl Screen<Data> for GardenScreen {
    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ missing `draw` in implementation
```

**`update` doesn't answer.** The last line of a function is what it gives back. Leave
out `Transition::None` at the end, and `update` ends on its loop, which gives back
nothing, `()`:

```text
error[E0308]: mismatched types
    | |_________^ expected `Transition<BasicData<UiAction, Plan>>`, found `()`
```

**A `match` misses a possibility.** Remove the `_ => continue` arm, and the compiler
lists the actions you left out. It's one of Rust's most helpful errors, and it means
that adding a new possibility to an enum shows you every place that must handle it:

```text
error[E0004]: non-exhaustive patterns: `shuyi::prelude::UiAction::Up`, `shuyi::prelude::UiAction::Down` and `shuyi::prelude::UiAction::Debug` not covered
```

**A screen tries to change something while drawing.** Rust stops it, because `draw`
only has `&self`:

```text
error[E0596]: cannot borrow `self.garden` as mutable, as it is behind a `&` reference
```

**A text key isn't in the file.** This one compiles, and `cargo shuyi check` passes.
Running the game stops it in a debug build, with a **panic**, Rust's way of stopping
when something that should never happen did. It names the key:

```text
text key not in the English text: "title.name"
```

So run the screens you change, or better, test them. And a misspelled action in the
text, like `{Confirn}`, isn't an error at all: the player sees `{Confirn}` on the
screen, so look at the line you changed.

## When a game outgrows the short path

The short path's parts grow one at a time, and the game's start stays as it is:

- **actions of its own**: an action set of the game's in place of `UiAction`
  ([chapter 3](@/learn/book/actions.md#actions-of-a-game-s-own));
- **a part of its own**: a map, a table of rules, read by the function
  `BasicGame::with_own` takes ([chapter 5](@/learn/book/assets-and-formats.md));
- **tools of its own**: a scene, records and bots, through a `GameTools` of its own
  ([chapter 10](@/learn/book/tools-for-agents.md)).

You can leave this section until you need it. A game whose data needs more than
`BasicData` has (sounds, voices, color themes, settings of its own) writes its own
`GameData`. Each method hands the engine a part it reads:

{{ snippet(file="src/screens.rs", anchor="game_data") }}

> **Rust:** the `type` lines fill in blanks of the trait. `GameData` needs to know this
> game's actions, colors, voices and settings, and `()`, a value holding nothing, says
> this game has none of its own.

Such a game makes its context with `Ctx::new(data)`, its game with `Game::new(ctx,
first_screen, GameHooks::default())`, and its binary calls `shuyi::app::run` with its
own window settings. The API docs of `shuyi_ui` and `shuyi_app` have the details.
