+++
title = "Pushing"
description = "Push a box, win when every box is on a target, and test it through the Harness."
weight = 40

[extra]
step = 4
+++

This is the step where it becomes a game. All of it is in the rules.

## Pushing

A box can move if the tile behind it is free. In `check`, replace the
refusal of a box:

{{ snippet(step="04-push", file="crates/game/src/rules.rs", anchor="push_check") }}

and in `apply`, move the box first:

{{ snippet(step="04-push", file="crates/game/src/rules.rs", anchor="push_apply") }}

`Refusal` gains `Stuck` and `Over`, and `Event` gains `Pushed`.

## Winning

`Outcome` was `()`. It is now `Won`, and it is a question about the state,
so no command has to remember to ask it:

{{ snippet(step="04-push", file="crates/game/src/rules.rs", anchor="outcome") }}

The board says so in its help line:

{{ snippet(step="04-push", file="crates/game/src/board.rs", anchor="won_view") }}

## The test

{{ snippet(step="04-push", file="crates/game/tests/board.rs", anchor="test_push") }}

Level 1 is won in two moves. Try it: the box turns green on its target.
