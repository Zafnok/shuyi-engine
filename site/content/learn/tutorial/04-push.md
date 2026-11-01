+++
title = "Pushing"
description = "Push a box, win when every box is on a target, and test it through the Harness."
weight = 40

[extra]
step = 4
+++

This is the step where it becomes a game. You'll push a box, and win:

{{ try_it(name="solved") }}

Here's the pay-off from step 3. **All of the game's new behaviour goes in the rules**. The screen
barely changes, because it was already sending commands and showing whatever the state says.

| File | What changes |
| ---- | ------------ |
| `crates/game/src/rules.rs` | Pushing, and winning |
| `crates/game/src/board.rs` | One line: the help line after a win |
| `crates/game/files/en.ron` | The words for that |
| `crates/game/tests/board.rs` | Tests of pushing and winning |

## New words for what can happen

A box can now move, or be stuck, and a level can be over. So the rules need new words. In
`rules.rs`, `Event` gains `Pushed`:

{{ snippet(step="04-push", file="crates/game/src/rules.rs", anchor="event") }}

> **Rust:** an enum's possibility can have named fields, like a struct. `Pushed { from, to }`
> says plainly which is which, where `Pushed(Pos, Pos)` would leave you to remember.

and `Refusal` swaps `Box` for two new reasons:

{{ snippet(step="04-push", file="crates/game/src/rules.rs", anchor="refusal") }}

The `Display` code below it needs the same two reasons. Here the compiler helps you: as soon as
you remove `Box`, the `match` in `fmt` stops compiling, and tells you what to fix. This happens all
the time in Rust. Change a type, and follow the errors. They lead you to every place that needs
to change.

## Pushing

A box can move if the tile behind it is free. Picture the player, the box and the tile past the
box, in a line:

{{ snippet(step="04-push", file="crates/game/src/rules.rs", anchor="check") }}

The new part is the middle: `to` is where the player would step, and `past` is one further. If
there's a box at `to`, and `past` holds a wall or another box, the push is refused. `&&` is "and",
`||` is "or", and the brackets group them as in arithmetic.

And `apply` moves the box first, then the player:

{{ snippet(step="04-push", file="crates/game/src/rules.rs", anchor="apply") }}

`self.boxes.remove(&to)` removes a box from the set at `to` if there is one, and answers whether
it did. So that `if` reads: "if there was a box here, it's now one further along". Notice that
`apply` doesn't re-check anything. `check` has already said this is allowed, which is why it's the
first thing `apply` calls.

Now run it, and push the box around. Here it is after one step right:

{{ try_it(name="pushed") }}

## Winning

> **Game idea:** the game's outcome is a question you ask of the state, not something you have
> to remember to record. No command has to remember to say "you won". There can't be a bug where
> one forgets.

Add a type for how a level ends. `Won` holds no data: it's a struct with no fields, as the title was:

{{ snippet(step="04-push", file="crates/game/src/rules.rs", anchor="won") }}

In `TurnRules`, change `type Outcome = ();` to `type Outcome = Won;`, and answer the question:

{{ snippet(step="04-push", file="crates/game/src/rules.rs", anchor="outcome") }}

That's the entire win condition: the set of boxes is the same as the set of targets. A set was
a good choice here, since two sets are equal however they were filled. `.then_some(Won)` turns true
into `Some(Won)`, and false into `None`.

The start of `check` also refuses every move once the level is won, which is the `Refusal::Over`
you added.

The box turning green is already done. In step 2, `scene` worked out each box's `home`, and
`Pieces` picks the `box_home` color for it. Nobody has to tell the screen about winning, because it
only ever draws the state.

## Saying so

The board picks its help line according to the outcome. In `view`, replace the `let help` line:

{{ snippet(step="04-push", file="crates/game/src/board.rs", anchor="won_view") }}

and add the words to `files/en.ron`:

{{ snippet(step="04-push", file="crates/game/files/en.ron", anchor="board", language="ron") }}

Once again, words and colors are in their own files, so they're quick to change:

{{ try_it(name="won-words") }}

{{ try_it(name="box-home-blue") }}

## The tests

Add these to `tests/board.rs`, with `use shuyi::core::TurnRules;` at the top so that `outcome()`
can be called:

{{ snippet(step="04-push", file="crates/game/tests/board.rs", anchor="test_push") }}

{{ snippet(step="04-push", file="crates/game/tests/board.rs", anchor="test_stuck") }}

Each of these plays the actual game, by pressing keys. Reading them, you can follow what's
happening on the board in your head: that's a good sign in a test. The second walks round the box,
pushes it up against the wall, and tries once more.

Level 1 is won in two moves. Try it.

## What you learned

- **Rust:** enum possibilities with named fields, `||`, and letting the compiler's errors walk you
  through a change.
- **Game idea:** the outcome is a question about the state.
- **Shuyi:** how little the screen has to do, once the rules own everything.

Next: a puzzle needs an undo.
