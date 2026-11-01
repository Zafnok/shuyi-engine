+++
title = "Undo"
description = "A history of commands, and undo as an action of the game's own."
weight = 50

[extra]
step = 5
+++

A puzzle needs undo. The engine has the history, and the game needs one
more action.

## The game's own actions

Undo is an action `UiAction` lacks, so the game's own enum replaces it. It
keeps the same six names, so the keymap and the title's code still read
the same. Make `crates/game/src/action.rs`:

{{ snippet(step="05-undo", file="crates/game/src/action.rs") }}

Replace `UiAction` with `Action` in `lib.rs`, `title.rs`, `stages.rs` and
the tests. Name this file in `shuyi.ron`'s `check_keys` as `key_files`, as
it names the one fixed key. Then give Undo its default key in
`files/keymap.ron`, and its button in the same file's `pad`:

{{ snippet(step="05-undo", file="crates/game/files/keymap.ron", anchor="undo", language="ron") }}

## History

A `History` is the first state and every command since. The board keeps one
beside the puzzle:

{{ snippet(step="05-undo", file="crates/game/src/board.rs", anchor="board") }}

It records only the commands the rules accepted, so a walk into a wall is
no move:

{{ snippet(step="05-undo", file="crates/game/src/board.rs", anchor="send") }}

Undo rewinds it, which replays every command but the last. The rules are
deterministic, so that always gives the same state:

{{ snippet(step="05-undo", file="crates/game/src/board.rs", anchor="undo") }}

{{ snippet(step="05-undo", file="crates/game/src/board.rs", anchor="update") }}

## The test

{{ snippet(step="05-undo", file="crates/game/tests/board.rs", anchor="test_undo") }}
