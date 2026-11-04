+++
title = "Rules as commands"
description = "A game's rules are a state that changes only through commands, deterministic, with a history and a hash."
weight = 60

[extra]
chapter = 6
+++

Most of what's hard about a game comes later: undo, saving a game in progress, a replay
of how someone got that bug, a bot that plays it, a test that doesn't need a screen,
online play. Each of those needs to rebuild or repeat the game, exactly.

You get all of them from one rule, if you keep it from the start. A game's rules are a
**state**, everything that's true right now and could change, and that state changes
**only through commands**. A **command** is plain data saying what a player asks for:
"plant a seed here". The same state and the same command always give the same events
and the same new state, on every machine, which is called being **deterministic**.
That is what replays, bug reports, bots, undo and online play all rest on.

The chain is short, and worth learning by heart. A key press becomes an **action** (the
keymap, [chapter 3](@/learn/book/actions.md)), an action becomes a **command** (the
screen, [chapter 2](@/learn/book/games-and-screens.md)), and a command changes the
**state** (the rules, this chapter), which the screen then shows.

A turn game sends a command for each thing a player does. A real-time game sends its
inputs once per fixed step instead, 60 steps a second, counted from whole nanoseconds
so every machine steps alike. This chapter is about turn games, until
[Rules in fixed steps](#rules-in-fixed-steps) near its end.

## The state and its commands

The garden's rules know nothing of screens, keys or the clock:

{{ snippet(file="src/rules.rs", anchor="state") }}

> **Rust:** each `enum` is one of a fixed list. `Event::Moved(usize)` is a possibility
> that carries data: moved, and to which plot. `Vec<bool>` is a list of true-or-false,
> one for each plot.
>
> `Serialize` and `Deserialize`, derived here, write the code that turns a value into
> text and back. It's what lets a state be saved and hashed, as you'll see below.
> `Garden`'s fields have no `pub`, so nothing outside this file can change them: code
> elsewhere has to send a command, and the compiler enforces it.

## TurnRules

The rules implement `shuyi_core::TurnRules` for their state, the engine's **trait**,
its list of functions a turn game's rules promise to have:

{{ snippet(file="src/rules.rs", anchor="rules") }}

> **Rust:** the `type` lines fill in the trait's blanks: this game's command, event,
> error and outcome. `Result<(), Refusal>` is either "fine, nothing more to say",
> `Ok(())`, or a refusal, `Err(...)`.
>
> `self.check(command)?` is the line that makes `apply` all or nothing. The `?` says: if
> `check` refused, hand that refusal back now, before anything has changed. The compiler
> helps too: `check` has `&self`, so it can't change the state, and `apply` has `&mut
> self`, so it may.
>
> `Command::Left if self.at == 0 => ...` is a match arm with a guard: it only fits if
> the `if` is true too. `self.plots[self.at]` reads item number `at` from the list,
> counting from 0.
>
> `.into_iter().filter(|command| ...).collect()` goes through the three commands, keeps
> those that pass, and gathers them into a list. `.all(|&grows| grows)` asks whether
> every plot grows.

Its contract:

- **Deterministic.** Luck comes from a seeded random number generator kept in the
  state, `SimRng`. A **seed** is the number it starts from: the same seed, the same luck.
  It never comes from outside, so a game is rebuilt by replaying its commands. No clock,
  no files, no **floats** (numbers with a fractional part, whose results can differ
  between machines): integers count the same everywhere.
- **All or nothing.** `apply` either changes the state and says what happened, or
  refuses the command with a reason and changes nothing.
- `check` gives `apply`'s answer without changing anything, luck included.
- `legal_commands` lists commands `check` accepts, always in the same order.
- Once `outcome` is `Some`, the game is over and `legal_commands` is empty.

A screen sends the commands (the garden's `update`, in
[chapter 2](@/learn/book/games-and-screens.md#asking-the-screen-below)) and shows the
state; it never changes the state itself. To see that the rules decide everything,
change one of them. Here, planting stops costing a seed:

{{ try_it(name="book-free-seeds") }}

That's one line of the rules. No screen and no drawing changed, and the whole game
follows. Numbers in the game's files work the same way. With a chance of 0 percent, no
seed ever grows, and the rules' `outcome` ends the game:

{{ try_it(name="book-out-of-seeds") }}

A rule's test needs no screen at all:

{{ snippet(file="src/rules.rs", anchor="test") }}

> **Rust:** `matches!` asks whether a value fits a pattern, and `|` inside it means "or".
> `events[..]` is the whole list, seen as a slice so it can be matched.

## History and undo

A `History` is the state a game started in and every command since. Since the rules are
deterministic, that's enough: any earlier point is rebuilt by replaying, which is what
undo, rewind and saving a game in progress need. There's no need to keep a copy of the
state for each move.

{{ snippet(file="src/rules.rs", anchor="history") }}

A history is written as its first state and its commands, which is also what a record
of a play is.

## Determinism and the hash

How do you know two runs really played the same? Comparing whole states by eye doesn't
scale. A **hash** is a short number worked out from a whole value, and it changes if any
of it does.

`shuyi_core::state_hash` hashes any state that serializes, the same on every platform
and every run. Two runs that should agree compare their hashes step by step, and the
first step that differs is where to look:

{{ snippet(file="src/rules.rs", anchor="hash") }}

> **Rust:** `let play = |seed| { ... };` keeps a small nameless function in a name, so
> the test can call it twice. `.unwrap()` opens a `Result`, and stops the test if it's
> an error.

A state hashes the same each run only if it serializes the same: its maps must be
`BTreeMap` and its sets `BTreeSet`, which keep their items in order, never a hash map,
whose order changes from run to run.

## Time in rules

Rules have no clock, so they count what they have: turns, or in a real-time game, steps.
`shuyi_core::timer` has the two timers a game sets and forgets. A `Countdown` fires once
and is the cooldown too; an `Every` repeats. They live in the state, so they save, hash
and replay with it:

{{ snippet(file="src/rules.rs", anchor="time") }}

A turn game advances them by one where its turn ends, in `apply`. A real-time game
advances them by one in each step, stepping its rules once for each of `input.ticks()`,
so a timer fires on the same step at 30 and at 60 frames a second. `input.dt()` is the
seconds since the last frame, for what only looks, such as a glide or a blink: a rule
timed by it would change with the frame rate, and replays would stop agreeing.

## Rules in fixed steps

A real-time game's rules implement `shuyi_core::tick::TickSim` instead of `TurnRules`.
There are no commands: each step gets a `TickInput`, what each player pressed, held
and let go of since the last step, and answers with what happened. Here a bee flies
over the garden while Left or Right is held, a plot every 10 steps:

{{ snippet(file="src/rules.rs", anchor="tick") }}

> **Rust:** `ActionId(2)` is an action by its place in the game's list of actions,
> counting from 0: the rules name actions that way, so they need nothing but the
> engine's core. `left == right` is true when neither is held, and when both are.

The contract is `TurnRules`' first one: the same state and input always give the same
events and the same new state. Events are also the only way out of a step: a sound or
a picture comes from what `step` returns, never from inside it.

A screen doesn't step the rules itself. It keeps a `TickDriver` (`shuyi_ui::tick`), and
its `update` calls `driver.update(&mut self.bee, input)` with the frame's input. The
driver steps the rules as many times as the frame has steps due (`input.ticks()`, 0 to
4) and hands back each step's events with its number. Frames and steps don't line up:
on a 120 Hz screen every other frame has no step, and a late frame has several. So the
driver keeps what each frame read until a step takes it:

- each press is pressed in one step, the first one after it, and each release in a step
  after its press's, so a quick tap is never lost;
- a key's repeats never reach a step: the step reads what is held instead;
- an action pressed twice before a step waits, so the next steps have each press and
  release in turn.

A screen whose rules step calls `driver.clear()` from its `covered`, so a press made
just before a pause menu opens doesn't fire after it closes.

A **record** of a real-time run is its seed and each step's input, a `TickRecord`, with
the stamp of the build that made it. `TickDriver::recording` keeps one as the game
plays, and `tick::replay` plays it again, giving the state's hash after each step:

{{ snippet(file="src/rules.rs", anchor="tick-test") }}

> **Rust:** `use` lines inside a function name things for that function alone.
> `vec![UiAction::Right]` is the list of actions held in each of those frames.

The record holds what each step saw, after the driver handed out the presses, so it
replays the same whatever frame rate made it. A record from another build is refused,
naming what differs: a new version of the rules may step the same inputs differently.

## A rules crate of their own

A game made with `--template grid` keeps its rules in a crate of their own,
`crates/rules`, which may depend on the engine's core and plain data only. `cargo shuyi
lint` checks that fence: no floats, hash maps or clock in the rules' code
([`SH2103`](@/learn/errors/sh2103.md)), and no screen, device or tool crate, nor a crate
of outside luck, under them ([`SH2104`](@/learn/errors/sh2104.md)).

## When it goes wrong

**The rules ask for something a type doesn't have.** Take the `Serialize` off `Garden`,
and everything that saves or hashes it stops compiling. The message's last lines tell
you what to add:

```text
error[E0277]: the trait bound `Garden: serde::Serialize` is not satisfied
   = note: for local types consider adding `#[derive(serde::Serialize)]` to your `Garden` type
```

**A float in the rules.** Make `grow_percent` an `f32`, and the compiler stops you first,
because a type that derives `Eq`, exact equality, can't hold one, and floats don't have
that:

```text
error[E0277]: the trait bound `f32: Eq` is not satisfied
```

Not every float meets that. In a `grid` game, `cargo shuyi lint` finds the rest. In a `blank` game, with
no rules crate for it to fence, it says so under `limits`, and keeping floats out is up to you.

**A replay that disagrees.** If two runs with the same seed and commands give different
hashes, something from outside the state got in: the clock, a hash map's order, a random
number from outside `SimRng`. The first step that differs names the command to look at
([chapter 8](@/learn/book/testing.md#records-and-replays)).

**Nothing happens when you press a key.** A refused command changes nothing, and the
screen above ignores the refusal with `let _ =`. So a walk into the edge is silent, as
it should be. If you expected a change, ask the rules directly in a test, and read the
refusal: `assert_eq!(garden.apply(&Command::Left), Err(Refusal::Edge))`.

## Who moves

The bots of [chapter 8](@/learn/book/testing.md) play any rules that also say whose move
it is and which turn it is:

{{ snippet(file="src/rules.rs", anchor="sides") }}

> **Rust:** a garden has one player, so its side is `()`, a value holding nothing, and
> `side_to_move` has nothing to give back.
