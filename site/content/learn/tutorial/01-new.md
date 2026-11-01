+++
title = "A new game"
description = "Make the project with shuyi new, run it, and see what each file is."
weight = 10

[extra]
step = 1
+++

Install the tools as the [quick start](@/learn/book/quick-start.md) says,
then make the game. Use this name: the rest of the tutorial expects it.

```bash
shuyi new box-pusher --template blank --name "Box Pusher"
cd box-pusher
```

## Run it

In a window, and then with no window at all, through the engine's tools:

```bash
cargo run
cargo shuyi run --frames 30
cargo test --workspace
```

You see a title, the game's name in a box. Nothing else happens yet.

## What each file is

| Path | What |
| ---- | ---- |
| `shuyi.ron` | The project file the tools read |
| `Cargo.toml`, `crates/game/Cargo.toml` | The workspace and the game's crate |
| `crates/game/src/lib.rs` | What the game is: its files, its data and `game()` |
| `crates/game/src/main.rs` | Runs `game()` in a window |
| `crates/game/src/title.rs` | The title, the first screen |
| `crates/game/src/stages.rs` | The game's stages: none yet, and you fill this in next |
| `crates/game/src/app_info.rs` | Who the game is, written from `shuyi.ron` |
| `crates/game/examples/tools.rs` | The tool program `cargo shuyi` runs |
| `crates/game/files/` | The text, the keys, the colors and the font |
| `crates/game/tests/` | The game played without a window |

`shuyi new` also writes the rules for your coding agent and a CI workflow.
The tutorial's own copies of each step leave those out.

## Make it play in the browser

Every step of this tutorial plays on its page, so give the game its web
build now. Say so in `shuyi.ron`:

{{ snippet(step="01-new", file="shuyi.ron", anchor="web", language="ron") }}

and add `crates/game/build.rs`, as the page provides a few functions when
the game runs:

{{ snippet(step="01-new", file="crates/game/build.rs", anchor="build") }}

Then `cargo shuyi web --release` writes the page to `dist/web/`. On the
web there is no quitting: the player closes the tab. The title already
knows, by asking `ctx.platform.can_quit()`. Every step keeps that rule
([chapter 9](@/learn/book/targets-and-export.md#the-web-build) has more).
