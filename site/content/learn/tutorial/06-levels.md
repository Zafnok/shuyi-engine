+++
title = "Levels and a menu"
description = "A level list as a file, a menu screen, and a result handed back to it when a level is won."
weight = 60

[extra]
step = 6
+++

One level isn't a game. This step adds four more, a list of them, and a menu to choose from:

{{ try_it(name="level-menu") }}

The stack of screens gets a third card too, so you'll see how screens talk to one another.

| File | What changes |
| ---- | ------------ |
| `crates/game/files/levels/2.map` to `5.map` | New: four more levels |
| `crates/game/files/levels.ron` | New: the list of levels |
| `crates/game/src/level.rs` | Reads the list, and every level on it |
| `crates/game/src/levels.rs` | New: the menu screen |
| `crates/game/src/stages.rs` | `Own` holds every level, and the title starts the menu |
| `crates/game/src/board.rs` | Starts on a chosen level, and hands back a result |
| `crates/game/src/lib.rs` | `pub mod levels;` |
| `crates/game/files/en.ron` | The menu's words |
| `crates/game/tests/board.rs` | `on_board` goes through the menu, and new tests |

## The levels

Add these to `crates/game/files/levels/`. Designing levels is half of making a puzzle game, and
you're welcome to change them once the step works. These four were each checked to be winnable.
Step 9 has a test that proves it.

`2.map`:

{{ snippet(step="06-levels", file="crates/game/files/levels/2.map", language="ron") }}

`3.map`:

{{ snippet(step="06-levels", file="crates/game/files/levels/3.map", language="ron") }}

`4.map`:

{{ snippet(step="06-levels", file="crates/game/files/levels/4.map", language="ron") }}

`5.map`:

{{ snippet(step="06-levels", file="crates/game/files/levels/5.map", language="ron") }}

Here's how the second looks, when you reach it. The camera has more to frame, but no code had to
change for that:

{{ try_it(name="level-two") }}

## The list

Name them in `crates/game/files/levels.ron`, in the order they're played:

{{ snippet(step="06-levels", file="crates/game/files/levels.ron", language="ron") }}

A list in a file means the order is data too. Swap the first and last, and level 5 comes first,
with no code changed:

{{ try_it(name="levels-order") }}

The list is a kind of asset of the game's own, read as RON. In `level.rs`:

{{ snippet(step="06-levels", file="crates/game/src/level.rs", anchor="list") }}

It's the three parts from step 2 again. `LevelList` is a new **kind**, a plain type with a name for
error messages (that's all `AssetKind` asks). `Ron::<LevelList>::new()` is its **format**, which the
engine provides for any type. Add it to the **registry**, in `formats()`, below the other:

{{ snippet(step="06-levels", file="crates/game/src/level.rs", anchor="register") }}

> **Rust:** `Vec<String>` is a list of texts, and `pub struct LevelList(pub Vec<String>)` wraps it in
> a type of its own. `#[serde(transparent)]` tells serde to read the file as just the list inside.
> You get to it as `.0`, the first field.

`read_all` shows a good habit: it doesn't stop at the first bad level. It tries them all, gathering
every problem into `errors`, so when you run `cargo shuyi check` you see everything that's wrong at
once. `errors.extend(...)` adds a whole list to a list.

Now `Own` holds them all. In `stages.rs`, the `level` field becomes `levels`:

{{ snippet(step="06-levels", file="crates/game/src/stages.rs", anchor="own") }}

and the board takes which one:

{{ snippet(step="06-levels", file="crates/game/src/board.rs", anchor="new") }}

> **Rust:** `ctx.game.own.levels[level]` picks one item from a list by its position, counted from 0.
> A position past the end panics, and the `# Panics` part of the comment warns of that. `usize` is the
> kind of number used for positions.

Also add `pub level: usize,` to the top of `struct Board`.

## The menu

Make `crates/game/src/levels.rs` (with an `s`: `level.rs` is one level, and this is the screen that
lists them).

> **Shuyi:** before you write a screen, look for it in the engine. `shuyi::ui::widgets` has menus, lists
> and question boxes. Your screen just wraps one. The `Menu` here handles moving the focus up and down,
> wrapping round, and sounds.

{{ snippet(step="06-levels", file="crates/game/src/levels.rs", anchor="uses") }}

Each item of a menu carries the value it stands for, so the screen doesn't need a list beside it to
know what was chosen. Say what those values can be:

{{ snippet(step="06-levels", file="crates/game/src/levels.rs", anchor="choice") }}

The screen remembers only which item has the focus. `Default` in the derive means a `LevelMenu` can be
made with sensible starting values, and `focus` starts at 0:

{{ snippet(step="06-levels", file="crates/game/src/levels.rs", anchor="struct") }}

Then an `impl LevelMenu { ... }` block holds two functions. `new` makes one, using that default:

{{ snippet(step="06-levels", file="crates/game/src/levels.rs", anchor="new") }}

and `menu` makes the menu's items:

{{ snippet(step="06-levels", file="crates/game/src/levels.rs", anchor="menu") }}

> **Rust:** `0..levels` is a **range**: the numbers from 0 up to, but not including, `levels`. And
> `.map(|n| ...)` makes something from each one. Here, a chain of builders makes a menu item: its text
> key, then what to fill `{n}` with, then its value. `n + 1` is there since people count from 1 and
> programs count from 0.

The text comes from the key `levels.level`, so it's in `en.ron` as ever:

{{ snippet(step="06-levels", file="crates/game/files/en.ron", anchor="levels", language="ron") }}

{{ try_it(name="menu-words") }}

Then the jobs of a screen, inside `impl Screen<Data> for LevelMenu`. Give it a `name` of `"levels"`,
as you did for the others. Its actions are whichever its menu reads:

{{ snippet(step="06-levels", file="crates/game/src/levels.rs", anchor="actions") }}

`update` hands each action to the menu, which answers with what happened, if anything:

{{ snippet(step="06-levels", file="crates/game/src/levels.rs", anchor="update") }}

If a level is chosen, it pushes a board. If the player cancels, it pops back to the title. So only
the title ever leaves the game, and only where the platform allows it, which means nothing here can
leave a browser tab frozen.

`draw` is the same shape you've written before, and `as_any` is the same as the board's:

{{ snippet(step="06-levels", file="crates/game/src/levels.rs", anchor="draw") }}

Then, in `stages.rs`, the title starts the menu rather than the board, and the menu goes on the list of
screens:

{{ snippet(step="06-levels", file="crates/game/src/stages.rs", anchor="first") }}

{{ snippet(step="06-levels", file="crates/game/src/stages.rs", anchor="screens") }}

`let _ = ctx;` only says that this function doesn't need its `ctx` yet. (It will in step 7.)

## A result

Now the stack has three cards: title, menu, board. What should happen when a level is won? The menu
should move on to the next level. But the board doesn't know about the menu, and shouldn't: it's only
a board. So how does the news get across?

> **Shuyi:** a screen that finishes can hand a value to the one below it. It pops with a **result**,
> and the one below hears it in `uncovered`. Neither has to know what the other is, beyond the type
> of the value that they agree on.

So agree on a type, in `levels.rs`:

{{ snippet(step="06-levels", file="crates/game/src/levels.rs", anchor="result") }}

When the level is won, Confirm pops the board with what happened. In the board's `update`, above
`Action::Cancel`:

{{ snippet(step="06-levels", file="crates/game/src/board.rs", anchor="won") }}

The board's help lines in `en.ron` should now say so, and that Cancel goes back to the levels:

{{ snippet(step="06-levels", file="crates/game/files/en.ron", anchor="board", language="ron") }}

> **Rust:** `Action::Confirm if ... =>` is a **match guard**, an arm that counts only if its condition is
> true too. If it's false, `match` goes on to try the next arm, which here is the plain `Action::Confirm
> => {}`. So a Confirm on a level not yet won does nothing.

The menu hears it when it's uncovered, and moves on:

{{ snippet(step="06-levels", file="crates/game/src/levels.rs", anchor="uncovered") }}

> **Rust:** `result.and_then(ScreenResult::take::<LevelWon>)` asks "is there a result, and is it a
> `LevelWon`?". The `if let Some(LevelWon(level)) = ...` then opens it up, two layers at once, and calls
> the number inside `level`. Patterns like this can go as deep as you need.
>
> `saturating_sub(1)` subtracts, but stops at 0 rather than going below. And `.min(last)` takes the
> smaller of the two, so that winning the last level doesn't move the focus off the end.

## The tests

`on_board` in `tests/board.rs` needs another Enter now, and the stack has grown:

{{ snippet(step="06-levels", file="crates/game/tests/board.rs", anchor="on_board") }}

Every test that uses it keeps working. That's why it was a helper. Now a test of the whole flow:

{{ snippet(step="06-levels", file="crates/game/tests/board.rs", anchor="test_result") }}

It wins level 1, and checks that the board has gone, the menu's focus moved on, and that Enter opens
level 2. `h.screen::<LevelMenu>()` is why screens need `as_any`.

## What you learned

- **Rust:** `Vec`, ranges, match guards, patterns inside patterns, and indexing with `[]`.
- **Shuyi:** the engine's `Menu`, items carrying values, `AssetKind` and `Ron`, and results between
  screens.

[Chapter 2](@/learn/book/games-and-screens.md) explains screens and their results. Next: the game should
remember how far you got.
