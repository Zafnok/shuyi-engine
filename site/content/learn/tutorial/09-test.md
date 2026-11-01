+++
title = "Bots, replays and a frame"
description = "A bot that solves level 1, a replay test, a frame snapshot, and the game's own tools."
weight = 90

[extra]
step = 9
+++

The rules are deterministic, so a game can be tested by playing it.

## The game's own tools

`BasicTools` has no scene, bot or replay, so the game writes its own
`GameTools` in `crates/game/src/tools.rs`, and calls `BasicTools` for the
rest. `stages::tools` now returns it. First, the scene: what a program, or
your coding agent, can read of the game while it plays:

{{ snippet(step="09-test", file="crates/game/src/tools.rs", anchor="scene") }}

```bash
cargo shuyi scene --json --script "Enter Enter Right Right"
```

{{ snippet(step="09-test", file="crates/game/tests/tools.rs", anchor="test_scene") }}

## A bot

A bot needs to know who moves. The puzzle has one player, and it now counts
its turns:

{{ snippet(step="09-test", file="crates/game/src/rules.rs", anchor="sides") }}

This bot plays a line kept in the game's files, one in `files/lines/` for
each level. A test proves each line wins its level, so all five levels are
winnable:

{{ snippet(step="09-test", file="crates/game/src/tools.rs", anchor="bot") }}

{{ snippet(step="09-test", file="crates/game/tests/tools.rs", anchor="test_bot") }}

## A replay

A record is the level and its commands. `replay` runs it twice and compares
a hash of the state at every step, so anything that isn't deterministic
shows up:

{{ snippet(step="09-test", file="crates/game/src/tools.rs", anchor="record") }}

{{ snippet(step="09-test", file="crates/game/tests/tools.rs", anchor="test_replay") }}

## A frame

Most tests read the state. A test of a look compares the frame, as text,
with a file you keep. Read the new file before you keep it:

{{ snippet(step="09-test", file="crates/game/tests/tools.rs", anchor="test_frame") }}

[Chapter 8](@/learn/book/testing.md) has more on testing, and
[chapter 10](@/learn/book/tools-for-agents.md) on the tools.
