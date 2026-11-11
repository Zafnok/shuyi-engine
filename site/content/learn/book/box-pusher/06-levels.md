+++
title = "Levels and a menu"
description = "A level list as a file, a menu screen, and a result handed back to it when a level is won."
weight = 60
aliases = ["learn/tutorial/06-levels/", "learn/tutorial/06-levels/play/"]

[extra]
step = 6
follows = "games-and-screens"
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

{% newcomer(to="rust", topic="a struct around a `Vec`") %}
`Vec<String>` is a list of texts, and `pub struct LevelList(pub Vec<String>)` wraps it in
a type of its own. `#[serde(transparent)]` tells serde to read the file as just the list inside.
You get to it as `.0`, the first field.
{% end %}

`read_all` shows a good habit: it doesn't stop at the first bad level. It tries them all, gathering
every problem into `errors`, so when you run `cargo shuyi check` you see everything that's wrong at
once.

{% newcomer(to="rust", topic="`extend`") %}
`errors.extend(...)` adds a whole list to a list.
{% end %}

Now `Own` holds them all. In `stages.rs`, the `level` field becomes `levels`:

{{ snippet(step="06-levels", file="crates/game/src/stages.rs", anchor="own") }}

and the board takes which one:

{{ snippet(step="06-levels", file="crates/game/src/board.rs", anchor="new") }}

{% newcomer(to="rust", topic="indexing with `[]` and `usize`") %}
`ctx.game.own.levels[level]` picks one item from a list by its position, counted from 0.
A position past the end panics, and the `# Panics` part of the comment warns of that. `usize` is the
kind of number used for positions.
{% end %}

Also add `pub level: usize,` to the top of `struct Board`.

## The menu

Make `crates/game/src/levels.rs` (with an `s`: `level.rs` is one level, and this is the screen that
lists them).

{% newcomer(to="shuyi", topic="the engine's widgets") %}
Before you write a screen, look for it in the engine. `shuyi::ui::widgets` has menus, lists
and question boxes.
{% end %}

Your screen just wraps the engine's `Menu`, which handles moving the focus up and down,
wrapping round, and sounds.

{{ snippet(step="06-levels", file="crates/game/src/levels.rs", anchor="uses") }}

Each item of a menu carries the value it stands for, so the screen doesn't need a list beside it to
know what was chosen. Say what those values can be:

{{ snippet(step="06-levels", file="crates/game/src/levels.rs", anchor="choice") }}

The screen remembers only which item has the focus.

{% newcomer(to="rust", topic="`Default`") %}
`Default` in the derive means a `LevelMenu` can be made with sensible starting values, and `focus`
starts at 0.
{% end %}

{{ snippet(step="06-levels", file="crates/game/src/levels.rs", anchor="struct") }}

Then an `impl LevelMenu { ... }` block holds two functions. `new` makes one, using that default:

{{ snippet(step="06-levels", file="crates/game/src/levels.rs", anchor="new") }}

and `menu` makes the menu's items:

{{ snippet(step="06-levels", file="crates/game/src/levels.rs", anchor="menu") }}

{% newcomer(to="rust", topic="a range and `.map`") %}
`0..levels` is a **range**: the numbers from 0 up to, but not including, `levels`. And
`.map(|n| ...)` makes something from each one.
{% end %}

Here, a chain of builders makes a menu item: its text key, then what to fill `{n}` with, then its
value.

{% newcomer(to="programming", topic="counting from 0") %}
`n + 1` is there since people count from 1 and programs count from 0.
{% end %}

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

{% newcomer(to="rust", topic="`let _ = ctx;`") %}
`let _ = ctx;` only says that this function doesn't need its `ctx` yet. (It will in step 7.)
{% end %}

## A result

Now the stack has three cards: title, menu, board. What should happen when a level is won? The menu
should move on to the next level. But the board doesn't know about the menu, and shouldn't: it's only
a board. So how does the news get across?

A screen that finishes can hand a value to the one below it. It pops with a **result**,
and the one below hears it in `uncovered`. Neither has to know what the other is, beyond the type
of the value that they agree on.

So agree on a type, in `levels.rs`:

{{ snippet(step="06-levels", file="crates/game/src/levels.rs", anchor="result") }}

When the level is won, Confirm pops the board with what happened. In the board's `update`, above
`Action::Cancel`:

{{ snippet(step="06-levels", file="crates/game/src/board.rs", anchor="won") }}

The board's help lines in `en.ron` should now say so, and that Cancel goes back to the levels:

{{ snippet(step="06-levels", file="crates/game/files/en.ron", anchor="board", language="ron") }}

{% newcomer(to="rust", topic="a match guard") %}
`Action::Confirm if ... =>` is a **match guard**, an arm that counts only if its condition is
true too. If it's false, `match` goes on to try the next arm, which here is the plain `Action::Confirm
=> {}`. So a Confirm on a level not yet won does nothing.
{% end %}

The menu hears it when it's uncovered, and moves on:

{{ snippet(step="06-levels", file="crates/game/src/levels.rs", anchor="uncovered") }}

`result.and_then(ScreenResult::take::<LevelWon>)` asks "is there a result, and is it a
`LevelWon`?".

{% newcomer(to="rust", topic="`if let` and a pattern inside a pattern") %}
The `if let Some(LevelWon(level)) = ...` then opens it up, two layers at once, and calls
the number inside `level`. Patterns like this can go as deep as you need.
{% end %}

{% newcomer(to="rust", topic="`saturating_sub` and `min`") %}
`saturating_sub(1)` subtracts, but stops at 0 rather than going below. And `.min(last)` takes the
smaller of the two, so that winning the last level doesn't move the focus off the end.
{% end %}

## The tests

`on_board` in `tests/board.rs` needs another Enter now, and the stack has grown:

{{ snippet(step="06-levels", file="crates/game/tests/board.rs", anchor="on_board") }}

Every test that uses it keeps working. That's why it was a helper. Now a test of the whole flow:

{{ snippet(step="06-levels", file="crates/game/tests/board.rs", anchor="test_result") }}

It wins level 1, and checks that the board has gone, the menu's focus moved on, and that Enter opens
level 2. `h.screen::<LevelMenu>()` is why screens need `as_any`.

## What you learned

{% newcomer(to="rust", topic="the Rust in this step") %}
In Rust: `Vec`, ranges, match guards, patterns inside patterns, and indexing with `[]`.
{% end %}

In Shuyi: the engine's `Menu`, items carrying values, `AssetKind` and `Ron`, and results between
screens.

[Chapter 6](@/learn/book/games-and-screens.md) explains screens and their results. Next: the game should
remember how far you got.

## Check yourself

{% check() %}
You swap the first and last names in `levels.ron`. What has to change in the code for
level 5 to come first?

- [x] Nothing: the order of the list is data, read from the file. — The list names the levels in the order they're played ([the list](#the-list)).
- [ ] `Board::new` needs the new order of the levels. — The board takes a position on the list, and the list comes from the file ([the list](#the-list)).
- [ ] The menu's items must be written again in the new order. — `menu` makes one item for each level on the list, whatever its order ([the menu](#the-menu)).
{% end %}

{% check() %}
Two of the maps on the list are broken. What does `read_all` do?

- [ ] It stops at the first broken level and reports that one. — It tries them all, so you don't fix one problem only to find the next ([the list](#the-list)).
- [x] It tries every level and gathers each problem into `errors`, so `cargo shuyi check` shows them all at once. — `errors.extend(...)` adds each level's problems to the list ([the list](#the-list)).
- [ ] It leaves the broken levels out and loads the rest. — The problems are gathered and reported, not hidden ([the list](#the-list)).
{% end %}

{% check() %}
The board pops with `LevelWon(1)`, and the menu's `uncovered` runs
`self.focus = (level + 1).min(last);`. What does the player see?

- [x] The menu, with the focus on the third item, ready for Enter to start it. — The focus moves on to the next level, and `.min(last)` keeps it on the list ([a result](#a-result)).
- [ ] The board of the next level, pushed by the menu at once. — `uncovered` only moves the focus. Enter, chosen in `update`, pushes a board ([a result](#a-result)).
- [ ] The menu with its focus where it was, as the menu can't know which board was won. — The two screens agree on the type `LevelWon`, which carries the level ([a result](#a-result)).
{% end %}
