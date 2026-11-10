+++
title = "Playground"
description = "Hello, Grid playing in the page beside its data files: change its map, its colors, its keys or its words, run it again, and see what was wrong."
template = "playground.html"

[extra]
# The files the editors hold, in their tabs' order: Hello, Grid's, from
# examples/hello-grid/files/. `copy-playground.sh` copies them for the page
# to fetch, and the build checks fail one it doesn't.
files = ["stage.map", "palette.ron", "keymap.ron", "intro.dlg", "en.ron"]
+++

This is [Hello, Grid](@/examples/hello-grid.md), the engine's first example, playing
beside its data files. Change a file, press **Run**, and the game starts again with your
version. If something in a file is wrong, **Problems** under the editors says what and
where, as `cargo shuyi check` would, and the game plays its own files instead.

You edit the game's data, not its code. Changing the Rust code would mean compiling it,
and compiling Rust in a browser needs a build server. A game made with the engine keeps
most of what it is in files like these, so there is a lot to change without one.

<!-- more -->

## The files

- **`stage.map`** is the stage: a map drawn in characters, with a legend saying what each
  one is, and where the pieces start. It is in the engine's `.map` format, one of the
  text formats in [Assets and formats](@/learn/book/assets-and-formats.md).
- **`palette.ron`** is the skin's colors by name, the engine's and the map's.
  [The frame and skins](@/learn/book/frame-and-skins.md) says how a skin paints with
  them.
- **`keymap.ron`** binds keys to actions. The game reacts to actions, never to keys:
  [Actions, never keys](@/learn/book/actions.md).
- **`intro.dlg`** is the scene before the stage, in the engine's `.dlg` dialogue format,
  from [Assets and formats](@/learn/book/assets-and-formats.md).
- **`en.ron`** is every screen's text by key, the menus' too:
  [Text by key](@/learn/book/games-and-screens.md#text-by-key).

## How it works

**Run** keeps each file you changed in your browser's session storage, and starts the game
again. The game lays those files over its own as it loads, the
[layer](@/learn/book/assets-and-formats.md#layers) a page hands it, and tells this page
what was wrong with them. Your changes stay while this tab is open, through a reload of
the page, and go when you close it. Nothing is sent anywhere.

**Reset this file** puts the game's own text back in the file you are looking at, and
**Reset all** in every file; both run the game again.

Click the game to play it; keys you type in an editor never reach it.
