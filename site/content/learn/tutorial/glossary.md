+++
title = "Glossary"
description = "Every word this tutorial uses, in one place, with a link to where it was explained."
weight = 110
+++

Every word the tutorial teaches, with the page that explains it. If a word in the
engine's [API docs](@/learn/_index.md) or the [book](@/learn/book/_index.md) stops you, look here.

## Making games

| Word | What it means | Where |
| ---- | ------------- | ----- |
| Frame | One trip round the game's loop: read the input, update, draw. About 60 a second | [Before you start](@/learn/tutorial/00-start.md#what-a-game-is-to-a-computer) |
| Update, draw | The game loop's two jobs: what changes, and what it looks like now | [Step 1](@/learn/tutorial/01-new.md#the-screen-s-jobs) |
| Tile | One square of a level | [Step 2](@/learn/tutorial/02-board.md#the-level-s-file) |
| State | Everything that's true right now and could change | [Step 2](@/learn/tutorial/02-board.md#what-can-change-rules-rs) |
| Command | Plain data saying what a player asks for. The one way the state changes | [Step 3](@/learn/tutorial/03-walk.md#the-big-idea-commands) |
| Determinism | The same state and command always give the same result, on every computer | [Step 5](@/learn/tutorial/05-undo.md#history) |
| Outcome | How a game ends: a question asked of the state | [Step 4](@/learn/tutorial/04-push.md#winning) |
| Bot | A player made of code | [Step 9](@/learn/tutorial/09-test.md#a-bot) |
| Record, replay | A level and its commands, and playing it again | [Step 9](@/learn/tutorial/09-test.md#a-replay) |
| Hash | A short number worked out from a whole value, which changes if any of it does | [Step 9](@/learn/tutorial/09-test.md#a-replay) |
| Target | A kind of machine or place a game runs: desktop, web, a handheld | [Step 10](@/learn/tutorial/10-ship.md) |
| Gate | A check that must pass before a change goes in | [Step 10](@/learn/tutorial/10-ship.md#check-it) |

## Shuyi

| Word | What it means | Where |
| ---- | ------------- | ----- |
| Screen | One thing the game shows: a title, a level, a menu. Implements `Screen` | [Step 1](@/learn/tutorial/01-new.md#a-screen-title-rs) |
| Stack | The screens, one on another. Only the top hears input | [Step 1](@/learn/tutorial/01-new.md#a-screen-title-rs) |
| `Transition` | What `update` answers: stay, push, pop, quit | [Step 1](@/learn/tutorial/01-new.md#the-screen-s-jobs) |
| Result, `uncovered` | The value a screen hands the one below when it pops | [Step 6](@/learn/tutorial/06-levels.md#a-result) |
| `Ctx`, context | What a screen is given to reach everything: text, colors, saves, data | [Step 1](@/learn/tutorial/01-new.md#what-it-shows) |
| Stage | What the game is really about, which the title starts | [Step 2](@/learn/tutorial/02-board.md#joining-it-up) |
| `BasicData`, `BasicGame` | The short path: what the engine reads from your files, and your game | [Step 1](@/learn/tutorial/01-new.md#what-the-game-is-lib-rs) |
| `Own` | The game's own part of its data | [Step 2](@/learn/tutorial/02-board.md#joining-it-up) |
| View | What's on a screen, as plain data | [Step 1](@/learn/tutorial/01-new.md#what-it-shows) |
| Skin | What decides how a view or scene looks | [Step 2](@/learn/tutorial/02-board.md#the-skin) |
| Glyph, cell, `GlyphBuffer` | A character; one place in the grid; the whole frame of 48 by 24 | [Step 1](@/learn/tutorial/01-new.md#the-screen-s-jobs) |
| Text key | A name for a line of words in `en.ron` | [Step 1](@/learn/tutorial/01-new.md#the-words-en-ron) |
| Palette | The game's colors, by name | [Step 1](@/learn/tutorial/01-new.md#the-colors-palette-ron) |
| Action | Something a player can ask for, like Confirm | [Step 1](@/learn/tutorial/01-new.md#the-keys-keymap-ron) |
| Keymap | The file saying which keys give which actions | [Step 1](@/learn/tutorial/01-new.md#the-keys-keymap-ron) |
| `UiAction`, `ActionSet` | The engine's six moves; the trait for a game's own actions | [Step 5](@/learn/tutorial/05-undo.md#the-game-s-own-actions) |
| Bundle | The game's files, embedded in it | [Step 2](@/learn/tutorial/02-board.md#reading-it-level-rs) |
| Kind, format, registry | What you want from a file; what reads it; what finds the format | [Step 2](@/learn/tutorial/02-board.md#reading-it-level-rs) |
| Grid kit, scene, camera | The engine's map painter; the plain data it paints; which part is in view | [Step 2](@/learn/tutorial/02-board.md#the-skin) |
| `TurnRules` | The trait of a turn game's rules: `check`, `apply`, `legal_commands`, `outcome` | [Step 3](@/learn/tutorial/03-walk.md#the-rules) |
| `History` | The first state and every command since | [Step 5](@/learn/tutorial/05-undo.md#history) |
| `Menu` | The engine's menu widget | [Step 6](@/learn/tutorial/06-levels.md#the-menu) |
| Storage | Where saves go, whatever the machine | [Step 7](@/learn/tutorial/07-save.md#where-saves-go) |
| `SaveFormat`, `SaveHeader` | A save's name and version; reading only that version | [Step 7](@/learn/tutorial/07-save.md#what-is-kept) |
| Harness | Plays your game with no window, for tests | [Step 1](@/learn/tutorial/01-new.md#a-test) |
| `GameTools` | What connects a game to the engine's tools | [Step 9](@/learn/tutorial/09-test.md#the-game-s-own-tools) |
| Snapshot | A frame as text | [Step 9](@/learn/tutorial/09-test.md#a-frame) |
| Id | What says which game this is; its saves are kept under it | [Step 10](@/learn/tutorial/10-ship.md#who-it-is) |

## Rust

| Word | What it means | Where |
| ---- | ------------- | ----- |
| Compiler, Cargo | What turns code into a program; the tool that runs it for you | [Before you start](@/learn/tutorial/00-start.md#what-you-are-about-to-do) |
| Crate, workspace, dependency | A package of code; crates built together; code yours is built on | [Step 1](@/learn/tutorial/01-new.md#the-folder-from-the-outside-in) |
| `fn`, calling, returning | A named list of instructions; running it; what it gives back. The last value with no `;` | [Step 1](@/learn/tutorial/01-new.md#your-first-rust-main-rs) |
| `use`, `::` | A shortcut to a name; walking down modules | [Step 1](@/learn/tutorial/01-new.md#your-first-rust-main-rs) |
| `mod`, `pub` | Making a file part of the program; letting others use it | [Step 1](@/learn/tutorial/01-new.md#what-the-game-is-lib-rs) |
| Type | What kind of thing a value is | [Step 1](@/learn/tutorial/01-new.md#what-the-game-is-lib-rs) |
| `let`, `let mut` | A name for a value; one that may change | [Step 1](@/learn/tutorial/01-new.md#what-the-game-is-lib-rs) |
| Macro | Code ending in `!`, that writes code while compiling | [Step 1](@/learn/tutorial/01-new.md#what-the-game-is-lib-rs) |
| Generics | The `<...>` blanks in a type, to fill in | [Step 1](@/learn/tutorial/01-new.md#what-the-game-is-lib-rs) |
| Closure | A small nameless function, `\|x\| { ... }` | [Step 1](@/learn/tutorial/01-new.md#what-the-game-is-lib-rs) |
| `Box`, `dyn` | A value in a box; "some type with this trait" | [Step 2](@/learn/tutorial/02-board.md#joining-it-up) |
| `struct`, `derive` | Named values grouped in one; standard abilities written for you | [Step 1](@/learn/tutorial/01-new.md#what-it-shows) |
| `Option`, `Some`, `None` | Maybe a value | [Step 1](@/learn/tutorial/01-new.md#what-it-shows) |
| `&`, `&mut` | Borrowing: to look at, or to change | [Step 1](@/learn/tutorial/01-new.md#what-it-shows) |
| `impl` | Adding functions to a type | [Step 1](@/learn/tutorial/01-new.md#what-it-shows) |
| Trait | A list of functions a type promises to have | [Step 1](@/learn/tutorial/01-new.md#the-screen-s-jobs) |
| `enum` | One of a fixed list of possibilities | [Step 1](@/learn/tutorial/01-new.md#the-screen-s-jobs) |
| `if let` | Open an `Option` or an enum, if it matches | [Step 1](@/learn/tutorial/01-new.md#the-screen-s-jobs) |
| `const`, tuple struct, array | A fixed value; a struct with unnamed fields; a list of fixed length | [Step 2](@/learn/tutorial/02-board.md#reading-it-level-rs) |
| `?` | If this is a `None` or an `Err`, return it now | [Step 2](@/learn/tutorial/02-board.md#reading-it-level-rs) |
| `Result`, `Ok`, `Err` | A value, or what went wrong | [Step 2](@/learn/tutorial/02-board.md#reading-it-level-rs) |
| `match`, `for`, `Self` | Handle every possibility; a loop; the type this `impl` is for | [Step 2](@/learn/tutorial/02-board.md#what-can-change-rules-rs) |
| `BTreeSet` | A set that always keeps the same order | [Step 2](@/learn/tutorial/02-board.md#what-can-change-rules-rs) |
| Float | A number with a fractional part. Never in rules | [Step 2](@/learn/tutorial/02-board.md#the-skin) |
| Lifetime | The `'a`: how long a borrow lasts | [Step 2](@/learn/tutorial/02-board.md#the-pieces) |
| Panic | Stop the program: for what can't happen | [Step 2](@/learn/tutorial/02-board.md#the-screen) |
| Iterator chain | `.map(...).filter(...).collect()` | [Step 3](@/learn/tutorial/03-walk.md#the-rules) |
| `*`, `_` | The value a reference points to; "anything else" | [Step 3](@/learn/tutorial/03-walk.md#the-rules) |
| Ownership, `clone` | Each value has one owner; a copy | [Step 5](@/learn/tutorial/05-undo.md#history) |
| Range, `Vec`, indexing | `0..n`; a list that can grow; `list[2]` | [Step 6](@/learn/tutorial/06-levels.md#the-menu) |
| Match guard | An arm with `if` | [Step 6](@/learn/tutorial/06-levels.md#a-result) |
| `let ... else` | This must match, or else do this | [Step 7](@/learn/tutorial/07-save.md#what-is-kept) |
| Tuple | Values side by side, `("n", 1)` | [Step 8](@/learn/tutorial/08-polish.md#a-moves-counter) |
| Feature, `#[cfg]` | An optional part of a crate; including code only if it's on | [Step 9](@/learn/tutorial/09-test.md#the-game-s-own-tools) |
