+++
title = "Testing"
description = "The Harness plays a game with no window, bots play its rules, records replay with a hash per step, and a frame is text or a PNG."
weight = 80

[extra]
chapter = 8
+++

A game on Shuyi Engine is tested with no window and no sound card. Its
rules are plain functions of their state. The game runs headless in the
engine's **Harness**, frame by frame. A frame is text a test can read,
and a PNG a person can look at. Tests are the review: a change is done
when a test proves it.

| What changed | Its test | Reads |
| ------------ | -------- | ----- |
| A rule | a unit test of the rules | the state and the events |
| Something true of every input | a property test | the state |
| A screen, a flow between screens | a Harness test | the screens, a screen's view |
| What the tools answer | a test calling the contract | the answer |
| A look | a text snapshot of the frame, or a picture | the frame |

## The rules

A rule's test builds a state, applies commands and reads what happened
([chapter 6](@/learn/book/rules-as-commands.md#turnrules)). The same
seed and the same commands always give the same state, so such a test
never flakes.

A property holds for any input. With `proptest` among the
dev-dependencies:

{{ snippet(file="src/testing.rs", anchor="property") }}

## The Harness

On the short path, `game().harness()` starts the game at its first
screen with no window, in frames of 1/60 of a second. A test drives it
and reads it:

- `keys("Enter Right")` presses and releases each key chord in turn, by
  the names of the game's default keys; `pad("South")` presses a
  controller button by its place, `act(action)` whatever key the player
  bound to an action, `hold(chord, seconds)` holds a key, and
  `wait(seconds)` lets time pass;
- `screens()` is the stack, `top_screen()` its top's name,
  `screen::<T>()` the screen of type `T` (one whose `as_any` gives it),
  `ctx()` the context, `sounds()` the sounds asked for,
  `quit_requested()` whether the game asked to quit, and `snapshot()` the
  frame as text.

[Chapter 2](@/learn/book/games-and-screens.md#a-test-of-it) has a test
of the flow, and [chapter 4](@/learn/book/frame-and-skins.md#why-tests-read-the-view)
one that reads a view.

## A look

A test of a look compares `h.snapshot()` with a file kept in the
repository, which it writes only when asked (an environment variable
the test reads): read the new file before keeping it. For a picture:

```bash
cargo shuyi frame-png garden.png --keys "Enter Right Enter" --scale 2
```

`--keys`, `--pad` and `--wait` are the steps, `--scale` the size, and
`--target <id>` a target's display. The picture is for people; tests
read the state.

## Bots

A bot is a player made of code: it picks the next command from the
state. `shuyi_core::bots::play` lets bots play a game to its end, or to
a turn limit:

{{ snippet(file="src/testing.rs", anchor="bot") }}

`RandomBot` picks one of the legal commands from a seed of its own, and
the core has bots that plan on copies of the game. A test then holds the
game to how often a bot should win:

{{ snippet(file="src/testing.rs", anchor="test_bot") }}

In a game whose tools give bots (a `grid` game's do), `cargo shuyi bots
20` plays them, and `cargo shuyi playtest` plays each named bot many
times on every core and says whether each wins as often as the game
wants.

## Records and replays

The seed and the commands are the whole play. A record of them replays
by the game's own rules, with the state's hash after every step:

{{ snippet(file="src/testing.rs", anchor="replay") }}

`cargo shuyi replay <record>` does the same from the game's tools.
Given two records that should play alike, it answers with the first
step whose hashes differ, and that step's command is where to look. A
record that says what made it ([chapter 7](@/learn/book/saves-and-settings.md#a-save))
is refused by another build of the game
([`SH3007`](@/learn/errors/sh3007.md)) rather than compared.

## Live, between calls

`cargo shuyi run --script "Enter Right" --frames 30` plays a script once
and answers. The MCP server keeps one game running between calls, for
an agent ([chapter 10](@/learn/book/tools-for-agents.md#the-mcp-server)).
And a debug build running in its window can be attached to: built with
the game's `link` feature, it listens, and `cargo shuyi attach` plays
steps on it and takes a picture of the real window.

## Running them

`cargo test --workspace` runs every test, and the game's CI runs them on
every pull request with the rest of its gates: formatting, clippy,
`cargo shuyi check` (with `--target all`) and `cargo shuyi lint`. The
game's `AGENTS.md` lists them.
