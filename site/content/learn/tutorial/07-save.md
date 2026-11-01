+++
title = "Saving progress"
description = "The furthest level reached, kept in the player's storage."
weight = 70

[extra]
step = 7
+++

The player shouldn't start again each time. Add `ron` to the workspace and
the game, then keep the furthest level reached.

## What is kept

Storage holds text under a key. Write the version of the format with it,
so a later build can tell what it is reading. Make
`crates/game/src/progress.rs`:

{{ snippet(step="07-save", file="crates/game/src/progress.rs", anchor="progress") }}

`ctx.storage` is a file on a computer and the browser's storage on the
web, and the game doesn't need to know which.

## Using it

The menu opens on the furthest level, and the levels past it can't be
chosen:

{{ snippet(step="07-save", file="crates/game/src/levels.rs", anchor="new") }}

{{ snippet(step="07-save", file="crates/game/src/levels.rs", anchor="menu") }}

When a level is won, it keeps the next one:

{{ snippet(step="07-save", file="crates/game/src/levels.rs", anchor="keep") }}

## The test

The Harness has a storage of its own, in memory:

{{ snippet(step="07-save", file="crates/game/tests/board.rs", anchor="test_save") }}

Play this step, win a level and reload the page: the menu remembers.
[Chapter 7](@/learn/book/saves-and-settings.md) has more on saves.
