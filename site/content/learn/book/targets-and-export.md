+++
title = "Targets and export"
description = "Each target says what its hardware can do, shuyi check --target holds a game to it, and shuyi web builds the browser's page."
weight = 90

[extra]
chapter = 9
+++

Each platform a game can ship on is a **target**, and each target has a
**profile**: plain data that says what its hardware can do. Its display
(the sizes it shows, its colors, how small text may be), its frame rate,
its inputs (which buttons, a pointer, a keyboard), its budgets of time
and memory, and the platform backend that runs a game there. A game
that needs more than a target has doesn't export there, and the tools
say why before anyone builds for it.

The profiles are the engine's
[`targets/`](https://github.com/Zafnok/shuyi-engine/tree/main/targets)
folder: `desktop` (Windows, Linux and macOS), `web` and `steam-deck` on
the macroquad backend, and `playdate` for its own backend, with phones
and the Switch planned.

## The targets a game ships on

A game lists its targets in its project file, `shuyi.ron`, each with the
size of its frame in pixels, how it scales (`integer`, or `fit`) and
the height of its smallest text:

{{ snippet(file="project/shuyi.ron", anchor="targets", language="ron") }}

## Checking them

```bash
cargo shuyi check --target all
```

checks, for each target the game lists, that:

- its frame at its scale fits the target's display, inside its safe
  area, and its smallest text is as tall as the target asks;
- every screen's required actions bind to distinct inputs the target
  has ([chapter 3](@/learn/book/actions.md)). A target with no keyboard
  (a Steam Deck) plays with the buttons of the keymap file's table
  named after it, `targets: { "steam-deck": { … } }`; when the actions
  don't fit, the screen changes, since the engine never folds actions
  onto fewer buttons;
- on a one-bit target, every palette role has a pattern.

Each problem names its target, the setting in `shuyi.ron` and the
numbers. `--target <id>` checks one.

## The web build

`cargo shuyi web` builds the game's binary for the browser
(`wasm32-unknown-unknown`, which `cargo shuyi doctor` checks rustup
has) and writes it with the page it runs in to a folder. A game that
has a web build says so in `shuyi.ron`:

{{ snippet(file="project/shuyi.ron", anchor="web", language="ron") }}

and its binary links with `--allow-undefined` on the web, so the page's
own scripts (the player's storage, controllers) are found when it
loads. A link argument reaches a binary only from its own build script,
`crates/game/build.rs`:

{{ snippet(file="build.rs", anchor="build") }}

Then:

```bash
cargo shuyi web --release
```

writes `dist/web/`: `index.html`, the game's `.wasm`, the engine's
scripts, and `NOTICES.txt`, the licenses of every crate and file the
build carries, with the game's own texts that `shuyi.ron`'s `notices`
names. Serve that folder over HTTP and open it in a browser; a page
opened from the disk can't load its `.wasm`. The engine's examples on
this site are such folders.

## Size and speed

A target's budgets are the most frame time, heap and start-up it
allows. `cargo shuyi run --perf` measures each frame headless, and the
engine's perf traces say where a frame's time and memory go; the
`shuyi-perf` skill
([chapter 10](@/learn/book/tools-for-agents.md#the-skills)) walks
through them.
