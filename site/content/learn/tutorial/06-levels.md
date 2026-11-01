+++
title = "Levels and a menu"
description = "A level list, a menu screen, and a result handed back when a level is won."
weight = 60

[extra]
step = 6
+++

One level isn't a game. This step adds four more, a list of them, and a
menu.

## The list

Add `levels/2.map` to `levels/5.map`, and name them in
`crates/game/files/levels.ron`:

{{ snippet(step="06-levels", file="crates/game/files/levels.ron", language="ron") }}

The list is a kind of asset of the game's own, read as RON:

{{ snippet(step="06-levels", file="crates/game/src/level.rs", anchor="list") }}

`Own` now holds every level, and `Board::new` takes which one.

## The menu

`crates/game/src/levels.rs` is a screen around the engine's `Menu`. Each
item carries the value it stands for, so the screen needs no list beside
it to know what was chosen:

{{ snippet(step="06-levels", file="crates/game/src/levels.rs", anchor="menu") }}

{{ snippet(step="06-levels", file="crates/game/src/levels.rs", anchor="update") }}

Cancel goes back to the title, and only the title leaves the game, where
the platform allows it. So nothing here can freeze a browser tab.

## A result

A screen that finishes can hand a value to the one below it. When the level
is won, Confirm pops the board with what happened:

{{ snippet(step="06-levels", file="crates/game/src/levels.rs", anchor="result") }}

{{ snippet(step="06-levels", file="crates/game/src/board.rs", anchor="won") }}

and the menu hears it when it is uncovered, and moves on:

{{ snippet(step="06-levels", file="crates/game/src/levels.rs", anchor="uncovered") }}

{{ snippet(step="06-levels", file="crates/game/tests/board.rs", anchor="test_result") }}

[Chapter 2](@/learn/book/games-and-screens.md) explains screens and their
results.
