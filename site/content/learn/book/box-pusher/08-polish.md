+++
title = "Polish"
description = "A moves counter from the history, a sound on a push and on a win, and the engine's options and key-rebinding screens."
weight = 80
aliases = ["learn/tutorial/08-polish/", "learn/tutorial/08-polish/play/"]

[extra]
step = 8
# The files its playground edits, in their tabs' order (ticket 2820).
files = ["levels.ron", "levels/1.map", "levels/2.map", "levels/3.map", "levels/4.map", "levels/5.map", "palette.ron", "en.ron", "keymap.ron"]
# The Rust files its playground edits in the book's Codespace, in their tabs' order (ticket 2841).
rust = ["crates/game/src/board.rs", "crates/game/src/options.rs", "crates/game/src/levels.rs", "crates/game/src/stages.rs", "crates/game/src/lib.rs"]
follows = "saves-and-settings"
+++

The game is whole. This step adds what makes it pleasant: a moves counter, a sound on a push and on a win,
and a way for players to change their keys. Each is short, because the work's already been done, by you or
by the engine.

{{ try_it(name="moves") }}

| File | What changes |
| ---- | ------------ |
| `crates/game/src/board.rs` | Shows the moves, and plays a push's sound and a win's |
| `crates/game/src/options.rs` | New: which options to offer |
| `crates/game/src/levels.rs` | An Options item on the menu |
| `crates/game/src/stages.rs` | Names the two new screens |
| `crates/game/src/lib.rs` | `pub mod options;` |
| `crates/game/files/sfx/push.sfx.ron`, `win.sfx.ron` | New: the two sounds |
| `crates/game/files/audio.ron` | Their cues |
| `crates/game/files/en.ron` | Words for all of it |
| `crates/game/tests/board.rs` | Tests of all three |
| `crates/game/tests/it/main.rs` | A test that every sound is there and plays |

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

## Sounds

A push and a win should be heard as well as seen. Your game already makes sounds: its menus play the
engine's, which `shuyi new` put in `crates/game/files/sfx/`, named by `crates/game/files/audio.ron`.
The board gets two of its own.

Each is a **sound effect**: a `.sfx.ron` file of a few numbers, a wave, a pitch and how loud it is as it
holds and fades. The game turns it into a sound as it loads it, so there's no sound file to record. The
`sfx` switch of `shuyi-assets`, in `crates/game/Cargo.toml`, is what lets it, and `shuyi new` turned it
on. You don't write one from nothing. Start each from one of the engine's:

```bash
cargo shuyi sfx hit --out crates/game/files/sfx/push.sfx.ron
cargo shuyi sfx power_up --out crates/game/files/sfx/win.sfx.ron
```

`cargo shuyi sfx nope` lists the others. To hear one before the game plays it, turn it into a sound file
your computer can play:

```bash
cargo shuyi sfx crates/game/files/sfx/push.sfx.ron --wav push.wav
```

The engine's `hit` is a crash, too long and too high to hear on every push. Make it a short, low scrape
by changing its numbers:

{{ snippet(step="08-polish", file="crates/game/files/sfx/push.sfx.ron", language="ron") }}

`hz` is the pitch it starts at and `to_hz` the one it slides to; `hold_ms` and `fade_ms` are how long it
holds and fades, in thousandths of a second; `volume` is how loud. `wave` is the sound's color:
`Noise` here, a hiss, or `Square(duty: 50)`, `Triangle`, `Saw` and `Sine`, which are notes. Each field and
its range is in
[the engine's formats](https://github.com/Zafnok/shuyi-engine/blob/main/docs/formats.md#sound-effects-sfxron),
and `cargo shuyi check` gives a number out of its range with its line. Run `--wav` again after each change,
until it sounds right.

The game plays a sound by its **cue**, a name in `files/audio.ron`, which says which file it is and how
loud. Add two to its `sounds`:

{{ snippet(step="08-polish", file="crates/game/files/audio.ron", anchor="board", language="ron") }}

In `board.rs`, name them once, so a typo is the compiler's to find:

{{ snippet(step="08-polish", file="crates/game/src/board.rs", anchor="sounds") }}

`send` is where a move happens, so it's where its sound is played. It takes the `ctx` now, as that holds
`ctx.audio`, and it asks the rules what the move did. The move that wins plays `WIN`, and any other push
plays `PUSH`. A walk, a refused step and an undo play nothing:

{{ snippet(step="08-polish", file="crates/game/src/board.rs", anchor="send") }}

{% newcomer(to="rust", topic="`let ... else` and `matches!`") %}
`let Ok(events) = ... else { return; };` is a [let-else](@/learn/book/box-pusher/07-save.md#what-is-kept):
a refused command is an `Err`, and `send` stops there. `events.iter().any(|e| matches!(e, Event::Pushed { .. }))`
asks whether any of the move's events is a push. `matches!` asks whether a value fits a pattern, and
`{ .. }` says the push's tiles don't matter.
{% end %}

`update` hands its `ctx` on, where it used to be `_`:

{{ snippet(step="08-polish", file="crates/game/src/board.rs", anchor="steps") }}

The Harness keeps every cue the game asked for, so a test can check which move made which sound:

{{ snippet(step="08-polish", file="crates/game/tests/board.rs", anchor="test_sounds") }}

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

Run it, and open *Options*, then *Key bindings*:

{{ try_it(name="key-bindings") }}

Now change your Undo key. Then go back to a level, and read the help line. It names your new key, and you didn't
do anything to make that happen. Every line of text has said `{Undo}` all along, since step 5. **This is why text
never names a key.**

## What you learned

{% newcomer(to="rust", topic="the Rust in this step") %}
In Rust: tuples, passing a function by name, and `.chain`.
{% end %}

{% newcomer(to="games", topic="the ideas of making games in this step") %}
In making games: don't keep two copies of one fact, and a sound can be a few numbers.
{% end %}

In Shuyi: sound effects and their cues, the engine's options and key-binding screens, and how much a game
gets free by using actions.

Next: the game can test itself.

## Check yourself

{% check() %}
The counter shows `("n", &self.history.len())`. The player presses Up into a wall, then
Undo after a move. What does it count?

- [x] Nothing for the wall, and one fewer for the Undo. — A refused step is never recorded, and undo takes a command out of the history ([a moves counter](#a-moves-counter)).
- [ ] One more for each key pressed, the wall and the Undo too. — It counts commands in the history, and a key isn't a command ([a moves counter](#a-moves-counter)).
- [ ] Nothing for the wall, but the Undo counts as a move. — Undo already takes one command away, so the count goes down ([a moves counter](#a-moves-counter)).
{% end %}

{% check() %}
`send` plays `WIN` if `self.puzzle.outcome().is_some()`, and `PUSH` only if not. The player pushes the
last box onto its target. What do they hear?

- [ ] The push's sound, then the win's. — The two are an `if` and an `else if`: only one of them runs ([sounds](#sounds)).
- [x] The win's sound alone. — The move that wins plays `WIN` in place of `PUSH` ([sounds](#sounds)).
- [ ] Nothing, as a winning move isn't a push. — The rules still report the push, and the win's sound comes first ([sounds](#sounds)).
{% end %}

{% check() %}
A player changes their Undo key on the Key bindings screen. Why does the board's help
line name the new key, with no change to the game?

- [ ] The Key bindings screen writes the new key into `en.ron`. — The game's words don't change. The key bindings are saved apart ([options and rebinding](#options-and-rebinding)).
- [x] The line says `{Undo}`, which is filled with the player's own key for the action each time it's shown. — Text never names a key, so a rebound key shows everywhere ([options and rebinding](#options-and-rebinding)).
- [ ] `option_rows` tells the board which key is bound to Undo. — `option_rows` only lists the rows of the options screen ([options and rebinding](#options-and-rebinding)).
{% end %}

{% check(to="rust") %}
In `OptionsScreen::new(option_rows)`, there are no `()` after `option_rows`. What does the
screen get?

- [ ] The rows, made once when the line runs. — Without `()` the function isn't called here ([options and rebinding](#options-and-rebinding)).
- [ ] Nothing, as the line doesn't compile without `()`. — Passing a function by name is plain Rust ([options and rebinding](#options-and-rebinding)).
- [x] The function itself, which the screen calls when it needs its rows. — A function can be passed by name, without calling it ([options and rebinding](#options-and-rebinding)).
{% end %}

{% exercise(check="yellow-player", file="palette.ron") %}
Make the player yellow.

Hint: The player's color is `player` in the game's part of the palette. Yellow is a lot of
red and green and little blue, like `#ffe040`.
{% end %}
