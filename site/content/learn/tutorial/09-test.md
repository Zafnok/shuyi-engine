+++
title = "Bots, replays and a frame"
description = "The game's own tools: a scene a program can read, a bot that plays, a replay that catches a change, and a frame kept as a test."
weight = 90

[extra]
step = 9
+++

The game plays and looks just as it did in the last step. So why is this a step of its own? Because of how
you'll keep it working. As a game grows, a change here breaks something over there, and no one can replay
every level by hand after every change. So you teach the game to test itself.

Everything in this step is possible because of step 3. The rules are deterministic, so **a game can be tested
by playing it**, and the same play always gives the same result.

| File | What changes |
| ---- | ------------ |
| `crates/game/src/tools.rs` | New: the game's own tools |
| `crates/game/src/stages.rs` | `tools()` returns them |
| `crates/game/src/lib.rs` | `pub mod tools;`, only for the tools build |
| `crates/game/src/rules.rs` | Counts its turns, for bots |
| `crates/game/files/lines/1.ron` to `5.ron` | New: a winning line for each level |
| `crates/game/tests/tools.rs` | New: tests through the tools |
| `crates/game/tests/frames/level1.txt` | New: a frame kept as a file |

## The game's own tools

`cargo shuyi` has run your game since step 1, using `BasicTools`, the engine's ready-made tools. Those can't know
about your rules: they have no idea what a move is. So the game now writes its own, `Tools`, which adds what's
special to this game, and calls `BasicTools` for the rest.

> **Shuyi:** `GameTools` is the trait for that. It's the bridge between your game and the engine's tools,
> whether a person runs them from a terminal or a coding assistant calls them. Your `Tools` is
> [in this step's `tools.rs`](https://github.com/Zafnok/shuyi-engine/blob/main/site/tutorial/09-test/crates/game/src/tools.rs).
> Most of its functions just pass the call on to `self.basic()`. This page shows the parts that aren't that.

In `lib.rs`, the module is only part of a build with the tools in it, so it never ends up in the game you ship:

{{ snippet(step="09-test", file="crates/game/src/lib.rs", anchor="tools") }}

> **Rust:** `#[cfg(feature = "tools")]` means "only include this if the `tools` feature is on". A **feature**
> is an optional part of a crate, listed in its `Cargo.toml`. That's what `--features tools` was doing in the
> `shuyi` alias, in your `.cargo/config.toml`, all along.

And `stages::tools` returns it:

{{ snippet(step="09-test", file="crates/game/src/stages.rs", anchor="tools") }}

## The scene: what a program can read

A person looks at the screen. A program needs the same facts as data:

{{ snippet(step="09-test", file="crates/game/src/tools.rs", anchor="scene") }}

Try asking:

```bash
cargo shuyi scene --json --script "Enter Enter Right Right"
```

That plays those keys with no window, and answers with the map as text, plus your `moves` and `won`. `json!` writes
**JSON**, the most common format for programs to exchange data. Here is how a test uses it:

{{ snippet(step="09-test", file="crates/game/tests/tools.rs", anchor="test_scene") }}

## A bot

> **Game idea:** a **bot** is a player made of code. It looks at the state, and picks the next command. Bots test a
> game from outside it, playing whole games in the blink of an eye. In a game with opponents, they can also be the
> opponents.

A bot needs to know who moves, for games with several players. The puzzle has one, so its side is `()`, a value with
nothing in it. It also needs to know the turn, so the puzzle now counts them. In `rules.rs`, add `turn: u32` to the
state, start it at `1`, and add 1 in `apply`. (`u32` is a whole number that can't go below zero.) Then:

{{ snippet(step="09-test", file="crates/game/src/rules.rs", anchor="sides") }}

This bot isn't clever. It plays a line kept in a file: one in `files/lines/` for each level, such as `files/lines/2.ron`:

{{ snippet(step="09-test", file="crates/game/files/lines/2.ron", language="ron") }}

Take the five from
[this step's copy](https://github.com/Zafnok/shuyi-engine/tree/main/site/tutorial/09-test/crates/game/files/lines). Those
are the winning moves for each level, written by hand. Here's the bot that plays one:

{{ snippet(step="09-test", file="crates/game/src/tools.rs", anchor="bot") }}

> **Rust:** `Vec<Dir>` read straight from the file by `ron::from_str`, thanks to serde. `steps.reverse()` puts the last step
> first, because `.pop()` takes from the end of a list, and that's quick. `.or_else(legal)` says "or, if that gave `None`, try
> this". So when the line is used up, the bot makes any legal step, and the turn limit will end the play.

Why does this matter? Here's the test:

{{ snippet(step="09-test", file="crates/game/tests/tools.rs", anchor="test_bot") }}

It plays each level's line, and checks it wins. **So every level is proven winnable, every time you run your tests.** Edit a
level so it can't be won, and this test fails, and says which one. For a puzzle game, that's the most valuable test there is.

To run the bots by hand:

```bash
cargo shuyi bots 20
```

The tools know how to do that because of `bots`, in `tools.rs`, which plays them through the engine's `play`. Look it up
in this step's copy.

## A replay

A **record** is a level and its commands, and nothing more. Since the rules are deterministic, that's the whole play:

{{ snippet(step="09-test", file="crates/game/src/tools.rs", anchor="record") }}

> **Shuyi:** the `replay` tool plays a record **twice** and compares a hash of the state at every step. A **hash** is a short
> number worked out from all of a value, so if any of it changes, the number will too. So anything that isn't deterministic
> shows up at once, at the very step where it went wrong. If someone reports a bug, a record of what they did will reproduce
> it exactly.

{{ snippet(step="09-test", file="crates/game/tests/tools.rs", anchor="test_replay") }}

`made` says which build made a record, since a record from another build may not play alike, and the tool refuses to compare those.

## A frame

Most tests read the state. So how do you test a look? Here's the first level:

{{ try_it(name="test-frame") }}

And here's the same frame as text. `h.snapshot()` gives you this, its characters first, and then a map of each cell's colors:

{{ snippet(step="09-test", file="crates/game/tests/frames/level1.txt", language="text") }}

Notice that the colors are named by their role (`fg:box`), not by a number. So this test doesn't fail if you make the box blue, as
that's a change of look on purpose, but it does if the box stops being drawn with its `box` color. The test compares this with the
file you keep:

{{ snippet(step="09-test", file="crates/game/tests/tools.rs", anchor="test_frame") }}

To make the file the first time, or after you've changed a look on purpose, set `UPDATE_FRAMES`, which is what the test checks for:

```bash
UPDATE_FRAMES=1 cargo test --workspace the_first_level_looks_as_it_did
```

On Windows, in PowerShell, the same thing takes two lines:

```bash
$env:UPDATE_FRAMES = "1"
cargo test --workspace the_first_level_looks_as_it_did
```

(In PowerShell, close that terminal afterwards, or that variable stays set.) **Read the new file before you keep it.** It'll
be what the test holds the game to from now on.

Keep these tests few. A frame test fails on every change of look, while a test of the state only fails if something's really wrong.
So test what happened through the state, as you've been doing, and keep a frame or two for the screens you care about most.

## See them catch something

Change `'@'` to `'P'` in `Pieces`, as you tried back in step 2, and run `cargo test --workspace`. `the_first_level_looks_as_it_did`
fails, and shows you the difference. Put it back. Now change one line of `files/lines/1.ron`, from `[Right, Right]` to `[Right]`, and
run the tests: `every_levels_line_wins_it` fails, naming level 1. Put that back too.

That is the feeling you want. You can change anything, because if it breaks, you'll know within a few seconds.

## What you learned

- **Rust:** features and `#[cfg]`.
- **Game ideas:** bots, records and replays, hashes, and when a test should read the picture.
- **Shuyi:** `GameTools`, scenes for programs, `Bot` and `play`, `replay`, and `h.snapshot()`.

[Chapter 8](@/learn/book/testing.md) has more on testing, and [chapter 10](@/learn/book/tools-for-agents.md) on the tools. Next: hand it out.
