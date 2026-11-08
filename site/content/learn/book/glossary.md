+++
title = "Glossary"
description = "Every word the book uses, in one place, with a link to where it was explained."
weight = 220
aliases = ["learn/tutorial/glossary/"]

[extra]
part = "appendices"
appendix = "C"
+++

Every word the book teaches, with the pages that explain it: the project's step that
first uses it, and the chapter that says more. If a word in the engine's
[API docs](@/learn/_index.md) stops you, look here.

## Making games

| Word | What it means | Where |
| ---- | ------------- | ----- |
| Frame | One trip round the game's loop: read the input, update, draw. About 60 a second | [Before you start](@/learn/book/before-you-start.md#what-a-game-is-to-a-computer), [chapter 2](@/learn/book/frame-and-skins.md#the-frame) |
| Update, draw | The game loop's two jobs: what changes, and what it looks like now | [Step 1](@/learn/book/box-pusher/01-new.md#the-screen-s-jobs), [chapter 6](@/learn/book/games-and-screens.md#a-screen) |
| Tile | One square of a level | [Step 2](@/learn/book/box-pusher/02-board.md#the-level-s-file) |
| State | Everything that's true right now and could change | [Step 2](@/learn/book/box-pusher/02-board.md#what-can-change-rules-rs), [chapter 5](@/learn/book/rules-as-commands.md#the-state-and-its-commands) |
| Command | Plain data saying what a player asks for. The one way the state changes | [Step 3](@/learn/book/box-pusher/03-walk.md#the-big-idea-commands), [chapter 5](@/learn/book/rules-as-commands.md#the-state-and-its-commands) |
| Determinism | The same state and command always give the same result, on every computer | [Step 5](@/learn/book/box-pusher/05-undo.md#history), [chapter 5](@/learn/book/rules-as-commands.md#determinism-and-the-hash) |
| Seed | The number a game's luck starts from: the same seed gives the same luck | [Chapter 5](@/learn/book/rules-as-commands.md#turnrules) |
| Outcome | How a game ends: a question asked of the state | [Step 4](@/learn/book/box-pusher/04-push.md#winning), [chapter 5](@/learn/book/rules-as-commands.md#turnrules) |
| Tween, ease curve | A value moving from one place to another over time; how fast it goes on the way | [Chapter 2](@/learn/book/frame-and-skins.md#things-that-move) |
| Bot | A player made of code | [Step 9](@/learn/book/box-pusher/09-test.md#a-bot), [chapter 8](@/learn/book/testing.md#bots) |
| Record, replay | A level and its commands, and playing it again | [Step 9](@/learn/book/box-pusher/09-test.md#a-replay), [chapter 8](@/learn/book/testing.md#records-and-replays) |
| Hash | A short number worked out from a whole value, which changes if any of it does | [Step 9](@/learn/book/box-pusher/09-test.md#a-replay), [chapter 5](@/learn/book/rules-as-commands.md#determinism-and-the-hash) |
| Property test | Something that should hold for any input, tried on hundreds of random ones | [Chapter 8](@/learn/book/testing.md#the-rules) |
| Golden table | A record's hash at every step, kept beside the tests: the answers known to be right | [Chapter 8](@/learn/book/testing.md#golden-values) |
| Headless | Run with no window, as a test or a server does | [Chapter 1](@/learn/book/quick-start.md#try-it-with-nothing-installed) |
| Target | A kind of machine or place a game runs: desktop, web, a handheld | [Step 10](@/learn/book/box-pusher/10-ship.md), [chapter 9](@/learn/book/targets-and-export.md#the-targets-a-game-ships-on) |
| Export | Making the package a player downloads, for each of the game's targets | [Chapter 9](@/learn/book/targets-and-export.md#export) |
| Gate | A check that must pass before a change goes in | [Step 10](@/learn/book/box-pusher/10-ship.md#check-it), [chapter 1](@/learn/book/quick-start.md#run-it) |

## Shuyi

| Word | What it means | Where |
| ---- | ------------- | ----- |
| Screen | One thing the game shows: a title, a level, a menu. Implements `Screen` | [Step 1](@/learn/book/box-pusher/01-new.md#a-screen-title-rs), [chapter 6](@/learn/book/games-and-screens.md#a-screen) |
| Stack | The screens, one on another. Only the top hears input | [Step 1](@/learn/book/box-pusher/01-new.md#a-screen-title-rs), [chapter 6](@/learn/book/games-and-screens.md#the-stack) |
| `Transition` | What `update` answers: stay, push, pop, quit | [Step 1](@/learn/book/box-pusher/01-new.md#the-screen-s-jobs), [chapter 6](@/learn/book/games-and-screens.md#the-stack) |
| Result, `uncovered` | The value a screen hands the one below when it pops | [Step 6](@/learn/book/box-pusher/06-levels.md#a-result), [chapter 6](@/learn/book/games-and-screens.md#asking-the-screen-below) |
| `Ctx`, context | What a screen is given to reach everything: text, colors, saves, data | [Step 1](@/learn/book/box-pusher/01-new.md#what-it-shows), [chapter 6](@/learn/book/games-and-screens.md#the-context) |
| Stage | What the game is really about, which the title starts | [Step 2](@/learn/book/box-pusher/02-board.md#joining-it-up) |
| `BasicData`, `BasicGame`, short path | The short path: what the engine reads from your files, and your game | [Step 1](@/learn/book/box-pusher/01-new.md#what-the-game-is-lib-rs), [chapter 1](@/learn/book/quick-start.md#what-it-is-made-of) |
| `Own` | The game's own part of its data | [Step 2](@/learn/book/box-pusher/02-board.md#joining-it-up), [chapter 6](@/learn/book/games-and-screens.md#when-a-game-outgrows-the-short-path) |
| View | What's on a screen, as plain data | [Step 1](@/learn/book/box-pusher/01-new.md#what-it-shows), [chapter 2](@/learn/book/frame-and-skins.md#the-view) |
| Skin | What decides how a view or scene looks | [Step 2](@/learn/book/box-pusher/02-board.md#the-skin), [chapter 2](@/learn/book/frame-and-skins.md#the-skin) |
| Glyph, cell, `GlyphBuffer` | A character; one place in the grid; the whole frame of 48 by 24 | [Step 1](@/learn/book/box-pusher/01-new.md#the-screen-s-jobs), [chapter 2](@/learn/book/frame-and-skins.md#the-frame) |
| Text key | A name for a line of words in `en.ron` | [Step 1](@/learn/book/box-pusher/01-new.md#the-words-en-ron), [chapter 6](@/learn/book/games-and-screens.md#text-by-key) |
| Palette | The game's colors, by name | [Step 1](@/learn/book/box-pusher/01-new.md#the-colors-palette-ron) |
| Debug view, channel, gizmo, role | What your logic works with, drawn over the running game; one thing it can show; a shape it draws; what a gizmo shows, which picks its color | [Chapter 2](@/learn/book/frame-and-skins.md#seeing-what-the-game-sees) |
| Widget | A piece of a screen the engine gives you, such as a menu | [Step 6](@/learn/book/box-pusher/06-levels.md#the-menu), [chapter 6](@/learn/book/games-and-screens.md#the-engine-s-own-screens-and-widgets) |
| Action | Something a player can ask for, like Confirm | [Step 1](@/learn/book/box-pusher/01-new.md#the-keys-keymap-ron), [chapter 4](@/learn/book/actions.md#reacting-to-an-action) |
| Keymap | The file saying which keys give which actions | [Step 1](@/learn/book/box-pusher/01-new.md#the-keys-keymap-ron), [chapter 4](@/learn/book/actions.md#the-keymap-file) |
| Chord, layout | A key with any keys held with it, such as `Shift+a`; one complete set of keys for a player to pick | [Chapter 4](@/learn/book/actions.md#the-keymap-file) |
| `UiAction`, `ActionSet` | The engine's six moves; the trait for a game's own actions | [Step 5](@/learn/book/box-pusher/05-undo.md#the-game-s-own-actions), [chapter 4](@/learn/book/actions.md#actions-of-a-game-s-own) |
| Asset, bundle | A file the game reads; the game's files, embedded in it | [Step 2](@/learn/book/box-pusher/02-board.md#reading-it-level-rs), [chapter 3](@/learn/book/assets-and-formats.md) |
| Kind, format, registry | What you want from a file; what reads it; what finds the format | [Step 2](@/learn/book/box-pusher/02-board.md#reading-it-level-rs), [chapter 3](@/learn/book/assets-and-formats.md#reading-it-through-the-registry) |
| Plugin | A format added to the registry, the engine's or your own | [Chapter 3](@/learn/book/assets-and-formats.md#a-format-of-your-own) |
| Layers, mod | A bundle read in order, each file from the first layer that has it; a change a player makes to a game, laid over it as a layer | [Chapter 3](@/learn/book/assets-and-formats.md#layers) |
| Pack, baked | The game's files in one indexed file, for fast loading; put there by `cargo shuyi bake` | [Chapter 3](@/learn/book/assets-and-formats.md#the-pack) |
| Grid kit, scene, camera | The engine's map painter; the plain data it paints; which part is in view | [Step 2](@/learn/book/box-pusher/02-board.md#the-skin), [chapter 2](@/learn/book/frame-and-skins.md#the-skin) |
| `TurnRules` | The trait of a turn game's rules: `check`, `apply`, `legal_commands`, `outcome` | [Step 3](@/learn/book/box-pusher/03-walk.md#the-rules), [chapter 5](@/learn/book/rules-as-commands.md#turnrules) |
| `History` | The first state and every command since | [Step 5](@/learn/book/box-pusher/05-undo.md#history), [chapter 5](@/learn/book/rules-as-commands.md#history-and-undo) |
| `Menu` | The engine's menu widget | [Step 6](@/learn/book/box-pusher/06-levels.md#the-menu), [chapter 6](@/learn/book/games-and-screens.md#the-engine-s-own-screens-and-widgets) |
| Storage | Where saves go, whatever the machine | [Step 7](@/learn/book/box-pusher/07-save.md#where-saves-go), [chapter 7](@/learn/book/saves-and-settings.md#storage) |
| Profile | One player's saves and settings, kept apart from another's | [Chapter 7](@/learn/book/saves-and-settings.md#profiles) |
| `SaveFormat`, `SaveHeader` | A save's name and version; reading only that version | [Step 7](@/learn/book/box-pusher/07-save.md#what-is-kept), [chapter 7](@/learn/book/saves-and-settings.md#a-save) |
| Harness | Plays your game with no window, for tests | [Step 1](@/learn/book/box-pusher/01-new.md#a-test), [chapter 8](@/learn/book/testing.md#the-harness) |
| `GameTools` | What connects a game to the engine's tools | [Step 9](@/learn/book/box-pusher/09-test.md#the-game-s-own-tools), [chapter 10](@/learn/book/tools-for-agents.md#one-value-of-the-game) |
| Snapshot | A frame as text | [Step 9](@/learn/book/box-pusher/09-test.md#a-frame), [chapter 8](@/learn/book/testing.md#a-look) |
| Session | A game the tools start and play, a step at a time | [Chapter 10](@/learn/book/tools-for-agents.md#the-mcp-server) |
| Session record | Everything the platform gave the whole game, frame by frame, to play it again | [Chapter 8](@/learn/book/testing.md#recording-a-session) |
| Id | What says which game this is; its saves are kept under it | [Step 10](@/learn/book/box-pusher/10-ship.md#who-it-is), [chapter 9](@/learn/book/targets-and-export.md#who-the-game-is) |
| Platform backend | The part of the engine that talks to a machine's window, files and sound | [Chapter 9](@/learn/book/targets-and-export.md) |
| Tool contract | The operations the engine's tools offer, the same from the command line and to an agent | [Chapter 10](@/learn/book/tools-for-agents.md#the-tool-contract) |
| MCP | The way an agent calls the tools of the contract | [Chapter 10](@/learn/book/tools-for-agents.md#the-mcp-server) |
| Skill pack | The engine's instructions for a coding agent, shipped with each game | [Chapter 10](@/learn/book/tools-for-agents.md#the-skills) |
| Problem code | A stable code on every error the tools give, such as `SH0001`, with a page saying how to fix it | [Chapter 3](@/learn/book/assets-and-formats.md#when-a-file-is-wrong), [chapter 10](@/learn/book/tools-for-agents.md#problem-codes) |

## Rust

| Word | What it means | Where |
| ---- | ------------- | ----- |
| Compiler, Cargo | What turns code into a program; the tool that runs it for you | [Before you start](@/learn/book/before-you-start.md#what-you-are-about-to-do), [chapter 1](@/learn/book/quick-start.md#install-rust) |
| Crate, workspace, dependency | A package of code; crates built together; code yours is built on | [Step 1](@/learn/book/box-pusher/01-new.md#the-folder-from-the-outside-in), [chapter 1](@/learn/book/quick-start.md#what-it-is-made-of) |
| RON | A simple text format for data, which the game's files are written in | [Chapter 1](@/learn/book/quick-start.md#what-it-is-made-of) |
| `fn`, calling, returning | A named list of instructions; running it; what it gives back. The last value with no `;` | [Step 1](@/learn/book/box-pusher/01-new.md#your-first-rust-main-rs) |
| `use`, `::` | A shortcut to a name; walking down modules | [Step 1](@/learn/book/box-pusher/01-new.md#your-first-rust-main-rs) |
| `mod`, `pub` | Making a file part of the program; letting others use it | [Step 1](@/learn/book/box-pusher/01-new.md#what-the-game-is-lib-rs), [chapter 6](@/learn/book/games-and-screens.md#modules) |
| Type | What kind of thing a value is | [Step 1](@/learn/book/box-pusher/01-new.md#what-the-game-is-lib-rs) |
| `let`, `let mut` | A name for a value; one that may change | [Step 1](@/learn/book/box-pusher/01-new.md#what-the-game-is-lib-rs) |
| Macro | Code ending in `!`, that writes code while compiling | [Step 1](@/learn/book/box-pusher/01-new.md#what-the-game-is-lib-rs) |
| Generics | The `<...>` blanks in a type, to fill in | [Step 1](@/learn/book/box-pusher/01-new.md#what-the-game-is-lib-rs) |
| Closure | A small nameless function, `\|x\| { ... }` | [Step 1](@/learn/book/box-pusher/01-new.md#what-the-game-is-lib-rs) |
| `Box`, `dyn` | A value in a box; "some type with this trait" | [Step 2](@/learn/book/box-pusher/02-board.md#joining-it-up) |
| `struct`, `derive` | Named values grouped in one; standard abilities written for you | [Step 1](@/learn/book/box-pusher/01-new.md#what-it-shows) |
| `Option`, `Some`, `None` | Maybe a value | [Step 1](@/learn/book/box-pusher/01-new.md#what-it-shows) |
| `&`, `&mut` | Borrowing: to look at, or to change | [Step 1](@/learn/book/box-pusher/01-new.md#what-it-shows) |
| `impl` | Adding functions to a type | [Step 1](@/learn/book/box-pusher/01-new.md#what-it-shows) |
| Trait | A list of functions a type promises to have | [Step 1](@/learn/book/box-pusher/01-new.md#the-screen-s-jobs), [chapter 6](@/learn/book/games-and-screens.md#a-screen) |
| `enum` | One of a fixed list of possibilities | [Step 1](@/learn/book/box-pusher/01-new.md#the-screen-s-jobs), [chapter 6](@/learn/book/games-and-screens.md#the-stack) |
| `if let` | Open an `Option` or an enum, if it matches | [Step 1](@/learn/book/box-pusher/01-new.md#the-screen-s-jobs) |
| `const`, tuple struct, array | A fixed value; a struct with unnamed fields; a list of fixed length | [Step 2](@/learn/book/box-pusher/02-board.md#reading-it-level-rs) |
| `?` | If this is a `None` or an `Err`, return it now | [Step 2](@/learn/book/box-pusher/02-board.md#reading-it-level-rs) |
| `Result`, `Ok`, `Err` | A value, or what went wrong | [Step 2](@/learn/book/box-pusher/02-board.md#reading-it-level-rs) |
| `match`, `for`, `Self` | Handle every possibility; a loop; the type this `impl` is for | [Step 2](@/learn/book/box-pusher/02-board.md#what-can-change-rules-rs) |
| `BTreeSet` | A set that always keeps the same order | [Step 2](@/learn/book/box-pusher/02-board.md#what-can-change-rules-rs) |
| Float | A number with a fractional part. Never in rules | [Step 2](@/learn/book/box-pusher/02-board.md#the-skin), [chapter 5](@/learn/book/rules-as-commands.md#turnrules) |
| Lifetime | The `'a`: how long a borrow lasts | [Step 2](@/learn/book/box-pusher/02-board.md#the-pieces) |
| Panic | Stop the program: for what can't happen | [Step 2](@/learn/book/box-pusher/02-board.md#the-screen), [chapter 6](@/learn/book/games-and-screens.md#when-it-goes-wrong) |
| Iterator chain | `.map(...).filter(...).collect()` | [Step 3](@/learn/book/box-pusher/03-walk.md#the-rules) |
| `*`, `_` | The value a reference points to; "anything else" | [Step 3](@/learn/book/box-pusher/03-walk.md#the-rules) |
| Ownership, `clone` | Each value has one owner; a copy | [Step 5](@/learn/book/box-pusher/05-undo.md#history) |
| Range, `Vec`, indexing | `0..n`; a list that can grow; `list[2]` | [Step 6](@/learn/book/box-pusher/06-levels.md#the-menu), [chapter 2](@/learn/book/frame-and-skins.md#the-skin) |
| Match guard | An arm with `if` | [Step 6](@/learn/book/box-pusher/06-levels.md#a-result), [chapter 6](@/learn/book/games-and-screens.md#asking-the-screen-below) |
| `let ... else` | This must match, or else do this | [Step 7](@/learn/book/box-pusher/07-save.md#what-is-kept) |
| Tuple | Values side by side, `("n", 1)` | [Step 8](@/learn/book/box-pusher/08-polish.md#a-moves-counter), [chapter 2](@/learn/book/frame-and-skins.md#the-skin) |
| Feature, `#[cfg]` | An optional part of a crate; including code only if it's on | [Step 9](@/learn/book/box-pusher/09-test.md#the-game-s-own-tools), [chapter 6](@/learn/book/games-and-screens.md#modules) |
| Build script | A program Cargo runs before it builds a crate | [Chapter 9](@/learn/book/targets-and-export.md#the-web-build) |
| Semver | A version's three numbers, like `1.4.2`, that say how big a change was | [Chapter 9](@/learn/book/targets-and-export.md#who-the-game-is) |
| Linker, clean build, rebuild | What joins compiled code into a program; building from nothing; building again after a change | [Chapter 1](@/learn/book/quick-start.md#when-it-goes-wrong), [appendix B](@/learn/book/fast-compiles.md) |
