+++
title = "A level on the board"
description = "A level as a text file, read through the format registry and drawn by the grid skin."
weight = 20

[extra]
step = 2
+++

A level is a text file, so changing one never means changing code. This
step reads one and draws it.

## The level's file

Make `crates/game/files/levels/1.map`, in the engine's `.map` format: a
header, a `---` line, then one character a tile. The header's `legend` says
what each character is, and its `features` say what stands where:

{{ snippet(step="02-board", file="crates/game/files/levels/1.map", language="ron") }}

## Reading it

Add `serde` to the workspace's `Cargo.toml` and to the game's. Then, in
`crates/game/src/level.rs`, name the game's terrains and features:

{{ snippet(step="02-board", file="crates/game/src/level.rs", anchor="terrains") }}

and read the file through the format registry, which knows the format by
the file's extension. A level that makes no puzzle is a problem in that
file, reported with the rest when the game loads:

{{ snippet(step="02-board", file="crates/game/src/level.rs", anchor="read") }}

The level is the game's own part of its data, which `BasicData` carries
for it. `crates/game/src/stages.rs` reads it, and `game()` in `lib.rs`
already passes `stages::own` to `BasicGame::with_own`:

{{ snippet(step="02-board", file="crates/game/src/stages.rs", anchor="own") }}

Its other functions say that the title starts the board.

## The rules' state

`crates/game/src/rules.rs` holds what can change: where the player and the
boxes are. It knows nothing of screens:

{{ snippet(step="02-board", file="crates/game/src/rules.rs", anchor="state") }}

## Drawing it

Graphics are a skin. The board builds a scene of plain data, and the
engine's glyph skin paints it, with the game's own look for each terrain
and piece. `crates/game/src/board.rs`:

{{ snippet(step="02-board", file="crates/game/src/board.rs", anchor="skin") }}

{{ snippet(step="02-board", file="crates/game/src/board.rs", anchor="draw") }}

The colors it names go in `files/palette.ron`, and its text in
`files/en.ron`. Keys are never named in text: `{Cancel}` shows the
player's key for that action. Now `cargo run` shows the level after the
title, and Cancel goes back.

[Chapter 5](@/learn/book/assets-and-formats.md) explains the registry, and
[chapter 4](@/learn/book/frame-and-skins.md) the skins.
