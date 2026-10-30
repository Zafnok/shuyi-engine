+++
title = "Rules as commands"
description = "A game's rules are a state that changes only through commands, deterministic, with a history and a hash."
weight = 60

[extra]
chapter = 6
+++

A game's rules are a **state** that changes only through **commands**.
The same state and the same command always give the same events and the
same new state, on every machine. That is what replays, bug reports,
bots, undo and online play all rest on.

A turn game sends a command for each thing a player does. A real-time
game sends its inputs once per fixed step instead, 60 steps a second,
counted from whole nanoseconds so every machine steps alike. This
chapter is about turn games.

## The state and its commands

The garden's rules know nothing of screens, keys or the clock:

{{ snippet(file="src/rules.rs", anchor="state") }}

## TurnRules

The rules implement `shuyi_core::TurnRules` for their state:

{{ snippet(file="src/rules.rs", anchor="rules") }}

Its contract:

- **Deterministic.** Luck comes from a seeded RNG kept in the state,
  `SimRng`, never from outside, so a game is rebuilt by replaying its
  commands. No clock, no files, no floats: integers count the same
  everywhere.
- **All or nothing.** `apply` either changes the state and says what
  happened, or refuses the command with a reason and changes nothing.
- `check` gives `apply`'s answer without changing anything, luck
  included.
- `legal_commands` lists commands `check` accepts, always in the same
  order.
- Once `outcome` is `Some`, the game is over and `legal_commands` is
  empty.

A screen sends the commands (the garden's `update`, in
[chapter 2](@/learn/book/games-and-screens.md#asking-the-screen-below))
and shows the state; it never changes the state itself. A rule's test
needs no screen at all:

{{ snippet(file="src/rules.rs", anchor="test") }}

## History and undo

A `History` is the state a game started in and every command since.
Any earlier point is rebuilt by replaying, which is what undo, rewind
and saving a game in progress need:

{{ snippet(file="src/rules.rs", anchor="history") }}

A history is written as its first state and its commands, which is
also what a record of a play is.

## Determinism and the hash

`shuyi_core::state_hash` hashes any state that serializes, the same on
every platform and every run. Two runs that should agree compare their
hashes step by step, and the first step that differs is where to look:

{{ snippet(file="src/rules.rs", anchor="hash") }}

A state hashes the same each run only if it serializes the same: its
maps must be `BTreeMap` and its sets `BTreeSet`, never a hash map whose
order changes from run to run.

## Time in rules

Rules have no clock, so they count what they have: turns, or in a
real-time game, steps. `shuyi_core::timer` has the two timers a game
sets and forgets. A `Countdown` fires once and is the cooldown too; an
`Every` repeats. They live in the state, so they save, hash and replay
with it:

{{ snippet(file="src/rules.rs", anchor="time") }}

A turn game advances them by one where its turn ends, in `apply`. A
real-time game advances them by one in each step, stepping its rules once
for each of `input.ticks()`, so a timer fires on the same step at 30 and
at 60 frames a second. `input.dt()` is for what only looks, such as a
glide or a blink: a rule timed by it would change with the frame rate,
and replays would stop agreeing.

## A rules crate of their own

A game made with `--template grid` keeps its rules in a crate of their
own, `crates/rules`, which may depend on the engine's core and plain
data only. `cargo shuyi lint` checks that fence: no floats, hash maps
or clock in the rules' code ([`SH2103`](@/learn/errors/sh2103.md)), and
no screen, device or tool crate, nor a crate of outside luck, under
them ([`SH2104`](@/learn/errors/sh2104.md)).

## Who moves

The bots of [chapter 8](@/learn/book/testing.md) play any rules that
also say whose move it is and which turn it is:

{{ snippet(file="src/rules.rs", anchor="sides") }}
