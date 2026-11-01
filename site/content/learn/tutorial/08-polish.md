+++
title = "Polish"
description = "A moves counter, and the options screen with rebinding."
weight = 80

[extra]
step = 8
+++

The game is whole. These are the things that make it pleasant.

## A moves counter

The history already knows how many moves were made, and undo already takes
one back. The board only has to show it:

{{ snippet(step="08-polish", file="crates/game/src/board.rs", anchor="moves") }}

{{ snippet(step="08-polish", file="crates/game/tests/board.rs", anchor="test_moves") }}

## Options and rebinding

The engine has the screens. Make `crates/game/src/options.rs`, which lists
the rows you want and every action a player can rebind:

{{ snippet(step="08-polish", file="crates/game/src/options.rs", anchor="rows") }}

The menu gets an Options item:

{{ snippet(step="08-polish", file="crates/game/src/levels.rs", anchor="menu") }}

`screens` in `stages.rs` names both new screens, so `cargo shuyi check
--target` checks that their controls fit each target. Their text goes in
`files/en.ron`, which needs a line for each key these screens ask for:
this step's copy of the file has them all, to take as they are.

{{ snippet(step="08-polish", file="crates/game/tests/board.rs", anchor="test_options") }}

Change your keys in this step's Options, and notice that the help lines
change with them. That is why text never names a key.
