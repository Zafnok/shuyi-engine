+++
title = "Walking"
description = "Actions for the four directions, a move command, and walls that stop you."
weight = 30

[extra]
step = 3
+++

The state changes only through commands. The screen hears actions, sends
commands, and shows the state.

## A command

In `crates/game/src/rules.rs`, say what the player can ask for, what can
happen, and why a command can be refused:

{{ snippet(step="03-walk", file="crates/game/src/rules.rs", anchor="command") }}

## The rules

`TurnRules` is the engine's shape for a turn game. `apply` either changes
the state or refuses and changes nothing. For now a box blocks the way:

{{ snippet(step="03-walk", file="crates/game/src/rules.rs", anchor="rules") }}

## From actions to commands

The board never reads a key. It reacts to `UiAction`'s moves, and the keys
stay in `files/keymap.ron`, where the player can change them:

{{ snippet(step="03-walk", file="crates/game/src/board.rs", anchor="update") }}

{{ snippet(step="03-walk", file="crates/game/src/board.rs", anchor="send") }}

## A test

Add `crates/game/tests/board.rs`. The Harness plays the game with no
window, and the test reads the state, not the picture:

{{ snippet(step="03-walk", file="crates/game/tests/board.rs", anchor="test_walk") }}

[Chapter 3](@/learn/book/actions.md) explains actions, and
[chapter 6](@/learn/book/rules-as-commands.md) the rules.
