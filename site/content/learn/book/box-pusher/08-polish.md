+++
title = "Polish"
description = "A moves counter from the history, and the engine's options and key-rebinding screens."
weight = 80
aliases = ["learn/tutorial/08-polish/", "learn/tutorial/08-polish/play/"]

[extra]
step = 8
follows = "saves-and-settings"
+++

The game is whole. This step adds what makes it pleasant: a moves counter, and a way for players to change
their keys. Both are short, because the work's already been done, by you or by the engine.

{{ try_it(name="moves") }}

| File | What changes |
| ---- | ------------ |
| `crates/game/src/board.rs` | Shows the moves |
| `crates/game/src/options.rs` | New: which options to offer |
| `crates/game/src/levels.rs` | An Options item on the menu |
| `crates/game/src/stages.rs` | Names the two new screens |
| `crates/game/src/lib.rs` | `pub mod options;` |
| `crates/game/files/en.ron` | Words for all of it |
| `crates/game/tests/board.rs` | Tests of both |

## A moves counter

How many moves has the player made? You don't need a new variable, because you already keep that: it's
how many commands are in the history. And undo already takes one away, and a walk into a wall already
doesn't count, as it's never recorded.

{% newcomer(to="games", topic="one copy of each fact") %}
A second copy of a fact is a chance for the two to disagree. If you can work a thing out from
what you already hold, do that.
{% end %}

Add a `moves` field to `BoardView`, as `pub moves: String,`, and fill it in `view`:

{{ snippet(step="08-polish", file="crates/game/src/board.rs", anchor="moves") }}

{% newcomer(to="rust", topic="a tuple and `&[...]`") %}
`("n", &self.history.len())` is a **tuple**, two values side by side, and `&[...]` is a borrowed
list of those. This fills in `{n}` in the text.
{% end %}

Then draw it at the top right, in `draw`, by working out where it must start from how long it is:

{{ snippet(step="08-polish", file="crates/game/src/board.rs", anchor="draw_moves") }}

Its words go in `files/en.ron`, as `"board.moves": "Moves: {n}",`.

And here's a nice thing about measuring. Use longer words, and it stays against the edge:

{{ try_it(name="moves-words") }}

Or centre it, by changing the sum:

{{ try_it(name="moves-left") }}

The test presses keys, and reads the screen's text. It also shows what the counter doesn't count:

{{ snippet(step="08-polish", file="crates/game/tests/board.rs", anchor="test_moves") }}

## Options and rebinding

Letting players change their keys is a lot of work to get right: reading the new key, refusing keys
already taken, saving them, restoring the defaults. It's also just the same in every game, which makes it
perfect for an engine. You only say what you want.

{{ try_it(name="options") }}

Make `crates/game/src/options.rs`, which lists the rows you want, and every action a player can rebind:

{{ snippet(step="08-polish", file="crates/game/src/options.rs", anchor="uses") }}

{{ snippet(step="08-polish", file="crates/game/src/options.rs", anchor="rows") }}

`KEY_ROWS` lists each action once, with the text key of its name. `RowGroup::Required` says it must keep a key.

`option_rows` builds the rows of the options screen. There's a frame rate, then a row that opens the Key
bindings screen, then Restore defaults. The engine makes the common ones for you (`OptionRow::frame_cap()`).
The engine has more that you could add, like text speed and volume, when your game has those things.

The two `|ctx| ...` and `|_| ...` say what the row shows and what it does when chosen. And
`.starting_group()` adds a gap above that row.

{% newcomer(to="rust", topic="closures") %}
Both are closures again.
{% end %}

The menu gets an item for it. In `levels.rs`, `Choice` gains `Options`, and the items go on as one list:

{{ snippet(step="08-polish", file="crates/game/src/levels.rs", anchor="menu") }}

{% newcomer(to="rust", topic="`.chain`") %}
`items.chain([options])` joins two sequences, so the Options item comes after the levels.
{% end %}

In `update`, add an arm for it:

{{ snippet(step="08-polish", file="crates/game/src/levels.rs", anchor="chosen") }}

{% newcomer(to="rust", topic="passing a function by name") %}
`OptionsScreen::new(option_rows)` passes the function itself, by name, without calling it: note there
are no `()` after `option_rows`. The screen calls it when it needs its rows.
{% end %}

{% newcomer(to="rust", topic="the compiler finding each `match`") %}
Notice you didn't have to change `Choice`'s users in a hundred places. The compiler told you which `match` wanted a
new arm, and there was only this one.
{% end %}

Next, `screens` in `stages.rs` names both new screens, so `cargo shuyi check --target` can check that their controls
fit each machine:

{{ snippet(step="08-polish", file="crates/game/src/stages.rs", anchor="screens") }}

Their text goes in `files/en.ron`. The engine's screens ask for a lot of words, one key each, so a game can be
translated in full. You'll want all of them, and there's no need to type them: take them from
[this step's `en.ron`](https://github.com/Zafnok/shuyi-engine/blob/main/site/tutorial/08-polish/crates/game/files/en.ron),
as they are. If you miss one, a debug build stops and names the key it couldn't find.

{{ snippet(step="08-polish", file="crates/game/tests/board.rs", anchor="test_options") }}

The `Up` in that test wraps round from the top of the menu to its last item, Options.

Run it, and open **Options**, then **Key bindings**:

{{ try_it(name="key-bindings") }}

Now change your Undo key. Then go back to a level, and read the help line. It names your new key, and you didn't
do anything to make that happen. Every line of text has said `{Undo}` all along, since step 5. **This is why text
never names a key.**

## What you learned

{% newcomer(to="rust", topic="the Rust in this step") %}
In Rust: tuples, passing a function by name, and `.chain`.
{% end %}

{% newcomer(to="games", topic="the ideas of making games in this step") %}
In making games: don't keep two copies of one fact.
{% end %}

In Shuyi: the engine's options and key-binding screens, and how much a game gets free by using actions.

Next: the game can test itself.

## Check yourself

{% check() %}
The counter shows `("n", &self.history.len())`. The player presses Up into a wall, then
Undo after a move. What does it count?

- [ ] One more for each key pressed, the wall and the Undo too. — It counts commands in the history, and a key isn't a command ([a moves counter](#a-moves-counter)).
- [x] Nothing for the wall, and one fewer for the Undo. — A refused step is never recorded, and undo takes a command out of the history ([a moves counter](#a-moves-counter)).
- [ ] Nothing for the wall, but the Undo counts as a move. — Undo already takes one command away, so the count goes down ([a moves counter](#a-moves-counter)).
{% end %}

{% check() %}
A player changes their Undo key on the Key bindings screen. Why does the board's help
line name the new key, with no change to the game?

- [x] The line says `{Undo}`, which is filled with the player's own key for the action each time it's shown. — Text never names a key, so a rebound key shows everywhere ([options and rebinding](#options-and-rebinding)).
- [ ] The Key bindings screen writes the new key into `en.ron`. — The game's words don't change. The key bindings are saved apart ([options and rebinding](#options-and-rebinding)).
- [ ] `option_rows` tells the board which key is bound to Undo. — `option_rows` only lists the rows of the options screen ([options and rebinding](#options-and-rebinding)).
{% end %}

{% check(to="rust") %}
In `OptionsScreen::new(option_rows)`, there are no `()` after `option_rows`. What does the
screen get?

- [ ] The rows, made once when the line runs. — Without `()` the function isn't called here ([options and rebinding](#options-and-rebinding)).
- [x] The function itself, which the screen calls when it needs its rows. — A function can be passed by name, without calling it ([options and rebinding](#options-and-rebinding)).
- [ ] Nothing, as the line doesn't compile without `()`. — Passing a function by name is plain Rust ([options and rebinding](#options-and-rebinding)).
{% end %}
