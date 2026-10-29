+++
title = "Quick start"
description = "Install the tools, make a game with shuyi new, run it in a window and without one."
weight = 10

[extra]
chapter = 1
+++

This chapter installs the engine's tools, makes a game, and runs it, in
a window and without one. The game it makes is the one the rest of the
book grows: Moon Garden, a row of plots to plant.

## Try it with nothing installed

[Open the engine in GitHub
Codespaces](https://codespaces.new/Zafnok/shuyi-engine?quickstart=1): a
container in your browser with Rust, the C linker and the system
libraries set up, and the engine's tools registered with the coding
agents it finds. Codespaces bills whoever opens it, from their own
GitHub account's free hours or plan. In it, `cargo shuyi new
../moon-garden --template blank` makes a game beside the engine, and the
rest of this chapter works as written, with no window: the tools play
the game headless.

## Install Rust

Install Rust with [rustup](https://rustup.rs). A game pins the version
it builds with in its `rust-toolchain.toml`, and rustup fetches that
version on the first build.

On Windows, either of Rust's toolchains works. With the MSVC one
(rustup's default), install Visual Studio's C++ build tools as rustup
says. With the GNU one, rustup's own linker lacks libraries the game's
window needs, so install MSYS2's:

```bash
winget install -e --id MSYS2.MSYS2
C:\msys64\usr\bin\pacman.exe -S mingw-w64-x86_64-gcc
```

and put `C:\msys64\mingw64\bin` on your `PATH`. A new game's
`.cargo/config.toml` already links with it.

## Install the tools

The engine isn't on crates.io yet: install its tools, the `shuyi`
program, from its repository.

```bash
cargo install --locked --git https://github.com/Zafnok/shuyi-engine shuyi-cli
```

This needs only Rust and a C linker. `shuyi doctor` then checks the rest
of what a game needs on your machine, and says how to install each
piece it finds missing.

## Make a game

```bash
shuyi new moon-garden --template blank --name "Moon Garden"
cd moon-garden
```

`blank` is one screen, a title. `grid` adds a turn-based grid stage
with its rules in a crate of their own, a bot and records; chapter 6
explains the rules. The game depends on the engine's repository at its
latest commit, and `shuyi new` says so: pass `--engine-rev <commit>` to
pin it to one commit instead.

On Linux, run the setup script the game came with before its first
build. It installs the C linker, `pkg-config`, the system libraries the
window and sound need, and the game's Rust, with `apt`, `dnf` or
`pacman`:

```bash
bash scripts/setup-linux.sh
```

## What it is made of

A new game is on the engine's **short path**: it names only what the
engine can't read from its files. Its code is a handful of short files.

`crates/game/src/lib.rs` says what the game is. Its files are embedded
in the binary:

{{ snippet(file="src/quick_start.rs", anchor="files") }}

Its data, which every screen shares, is the engine's `BasicData`: the
English text, the keys, the palette and the font, read from those
files, with the six moves of `UiAction` (`Up`, `Down`, `Left`, `Right`,
`Confirm` and `Cancel`) as its actions. A game's own part rides along;
here, the garden's plan (a new `blank` game has none, `()`):

{{ snippet(file="src/quick_start.rs", anchor="data") }}

And `game()` gives the rest, a `BasicGame`: its name, where the
player's settings are kept, its files, its first screen, and every
screen it can show:

{{ snippet(file="src/quick_start.rs", anchor="game") }}

`crates/game/src/main.rs` runs it in a window, with `run_game`:

{{ snippet(file="src/quick_start.rs", anchor="main") }}

`crates/game/examples/tools.rs` is the game's **tool program**: the
engine's tools run the game in it, with `BasicTools`, the tools of a
game on the short path:

{{ snippet(file="src/quick_start.rs", anchor="tools") }}

The rest of the folder:

| Path | What |
| ---- | ---- |
| `crates/game/src/title.rs` | The title, the first screen ([chapter 2](@/learn/book/games-and-screens.md)) |
| `crates/game/files/` | The English text (`en.ron`), the keys (`keymap.ron`), the colors (`palette.ron`) and the font |
| `crates/game/tests/` | The game played without a window, and through its tools ([chapter 8](@/learn/book/testing.md)) |
| `shuyi.ron` | The project file the tools read ([chapter 9](@/learn/book/targets-and-export.md)) |
| `AGENTS.md`, `.claude/skills/` | The rules a game on the engine keeps, and the engine's skills, for you and your coding agent ([chapter 10](@/learn/book/tools-for-agents.md)) |
| `.github/workflows/ci.yml` | The game's gates, run on every pull request |

## Run it

In a window:

```bash
cargo run
```

Without one: in the game's folder, `cargo shuyi` runs the engine's tools
on the game, through its tool program. The first one builds it.

```bash
cargo shuyi run --frames 30
```

`run` plays the game headless and answers with the screen on top, the
stack of screens and the frame as text, the title in a box with its
help line under it. Give it keys to press, by the names of the game's
default keys, and a picture of the frame with no window at all:

```bash
cargo shuyi run --script "Backspace"
cargo shuyi frame-png title.png
```

Every command answers for people, or with `--json` for a program. Then
the gates, as the game's CI runs them:

```bash
cargo test --workspace
cargo shuyi check
```

## In the browser

[Hello, Grid](@/examples/hello-grid.md), the engine's example, plays in
your browser on this site. A new game has no web build yet:
[chapter 9](@/learn/book/targets-and-export.md#the-web-build) gives it
one, in two short files, and then `cargo shuyi web` builds the page.

## Next

A rebuild after a one-line change takes about a second: [Fast
compiles](@/learn/book/fast-compiles.md) has the numbers and the setup
behind it. [Chapter 2](@/learn/book/games-and-screens.md) writes the
garden's screens.
