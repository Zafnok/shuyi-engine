+++
title = "Saving progress"
description = "The furthest level reached, kept in the player's storage, with a version so a later build can read it."
weight = 70

[extra]
step = 7
+++

The player shouldn't have to start again each time. In this step, the game remembers the furthest level
reached, and the levels past it can't be chosen yet:

{{ try_it(name="locked") }}

Levels not reached are dim. Win the first, and the next lights up:

{{ try_it(name="reached-two") }}

| File | What changes |
| ---- | ------------ |
| `crates/game/src/progress.rs` | New: what is kept, and how |
| `crates/game/src/levels.rs` | The menu reads it and writes it |
| `crates/game/src/stages.rs` | `LevelMenu::new` now needs `ctx` |
| `crates/game/src/lib.rs` | `pub mod progress;` |
| `Cargo.toml`, `crates/game/Cargo.toml` | Adds `ron` |
| `crates/game/tests/board.rs` | Tests of saving |

Add `ron` as you added `serde`. In the top `Cargo.toml`, under `[workspace.dependencies]`, add the line
`ron = "0.12"`, and in `crates/game/Cargo.toml`, under `[dependencies]`, add `ron.workspace = true`.

## Where saves go

> **Shuyi:** a game never opens a file itself. It goes through **storage**, which the platform hands it:
> a file on a computer, the browser's storage on the web, and only memory in a test. So the same code saves
> on every machine the game runs on, and a test needs no disk. It's the same idea as actions, and
> as `ctx` itself: the game asks for a thing, and doesn't care how it's done.

Storage keeps small named texts, and that's all. `ctx.storage` has `read` and `write`, and a key names
each text, just as a key names each line of your words.

## What is kept

> **Game idea:** always write the **version** of your save's format into the save. Some day you'll change
> what a save holds, and a player will still have an old one. A later build has to be able to tell what it's
> reading, or it will read it wrong.

Make `crates/game/src/progress.rs`:

{{ snippet(step="07-save", file="crates/game/src/progress.rs", anchor="uses") }}

{{ snippet(step="07-save", file="crates/game/src/progress.rs", anchor="progress") }}

Take this piece by piece.

`Progress` is what's kept. It derives `Serialize` and `Deserialize`, so serde can write it as text and read
it back. `SaveFormat` gives it a name and a version number, which you raise whenever the type changes.

`furthest` reads it. Its job is to never fail: if anything's wrong, the answer is level 0.

> **Rust:** `storage: &dyn Storage` means "a borrow of anything that implements the `Storage` trait". This
> function doesn't care whether that's a file or the browser. It's the same `dyn` as `Box<dyn Screen>`.
>
> `let Ok(Some(text)) = storage.read(KEY) else { return 0; };` is **let-else**: "this must match, or else do
> this". `read` returns a `Result` holding an `Option`: it could have failed, and if not, there might still
> be nothing saved. This handles both, in one line, and from here on `text` is the saved text.
>
> `header.is_current()` is checked in a match guard, like you saw in the last step, and `_ => 0` catches
> everything else: a version it doesn't know, or text it can't read. `map_or(0, ...)` says "use 0 if this
> failed, and otherwise get this from it".

> **Shuyi:** a `SaveHeader` reads only the version of a save, even one whose other fields can't be read any
> more. So a game can tell "this is from another version" apart from "this is damaged".

`reached` writes it, only if it's further than what's there. It returns a `Result`, as writing can fail,
for instance on a full disk. `map_err` turns one kind of error into another, so `ron`'s error can be
returned as a `StorageError`.

## Using it

The menu now has more to remember: how many levels are open. In `levels.rs`, add a field to the struct,
`pub open: usize,`, and start it from what's saved. `new` needs the context for that:

{{ snippet(step="07-save", file="crates/game/src/levels.rs", anchor="new") }}

That's why `stages.rs` changes `LevelMenu::new()` to `LevelMenu::new(ctx)`, in both `first` and `screens`.
Note how the menu is careful with what it reads: `.min(last)` means an old save can't point past the last
level, if a later version has fewer.

Levels past it can't be chosen. It's one more call in the chain:

{{ snippet(step="07-save", file="crates/game/src/levels.rs", anchor="menu") }}

The engine's `Menu` does the rest: it draws them dim, and skips them when moving the focus. Prove it to
yourself by opening every level:

{{ try_it(name="all-open") }}

Then, when a level is won, keep the next one. In `uncovered`, after `self.focus` is set:

{{ snippet(step="07-save", file="crates/game/src/levels.rs", anchor="keep") }}

`let _ =` is a decision here, not laziness, as the comment says: if saving fails, the player loses
only their progress, and that's no reason to stop their game. `&mut ctx.storage` is the changing kind of
borrow, as writing changes it.

## The test

The Harness has a storage of its own, in memory, which starts empty for every test:

{{ snippet(step="07-save", file="crates/game/tests/board.rs", anchor="test_save") }}

It makes a brand new menu to stand in for the next time the game starts, without any disk at all. And since
bad saves are the sort of thing players really do have, it's worth testing that those count as nothing:

{{ snippet(step="07-save", file="crates/game/tests/board.rs", anchor="test_damaged") }}

This needs `use shuyi::ui::storage::Storage;` at the top, as you have to name a trait to call its functions.

Now play this step in your browser from the button at the top of this page. Win a level, reload the page, and
the menu remembers. That save is in the browser's storage, and your code didn't need to know.

## What you learned

- **Rust:** `let ... else`, `&dyn`, `map_err` and `map_or`, and that a trait must be named to use it.
- **Game idea:** versions in saves, and treating a bad save as no save.
- **Shuyi:** `Storage`, `SaveFormat` and `SaveHeader`.

[Chapter 7](@/learn/book/saves-and-settings.md) has more on saves. The game is now whole. Next: the things that
make it pleasant.
