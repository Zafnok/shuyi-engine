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
[API docs](@/learn/_index.md) stops you, look here. *New to* says which newcomer
needs the word: a reader new to programming, to making games, to Rust or to Shuyi,
as the notes in the pages say.

## Making games

| Word | New to | What it means | Where |
| ---- | ------ | ------------- | ----- |
| Frame | Making games | One trip round the game's loop: read the input, update, draw. About 60 a second | [Before you start](@/learn/book/before-you-start.md#what-a-game-is-to-a-computer), [chapter 2](@/learn/book/frame-and-skins.md#the-frame) |
| Game engine | Making games | The code most games need, written once, so a game holds only what is its own | [Before you start](@/learn/book/before-you-start.md#what-an-engine-is) |
| Update, draw | Making games | The game loop's two jobs: what changes, and what it looks like now | [Step 1](@/learn/book/box-pusher/01-new.md#the-screen-s-jobs), [chapter 6](@/learn/book/games-and-screens.md#a-screen) |
| Tile | Making games | One square of a level | [Step 2](@/learn/book/box-pusher/02-board.md#the-level-s-file) |
| State | Making games | Everything that's true right now and could change | [Step 2](@/learn/book/box-pusher/02-board.md#what-can-change-rules-rs), [chapter 5](@/learn/book/rules-as-commands.md#the-state-and-its-commands) |
| Command | Making games | Plain data saying what a player asks for. The one way the state changes | [Step 3](@/learn/book/box-pusher/03-walk.md#the-big-idea-commands), [chapter 5](@/learn/book/rules-as-commands.md#the-state-and-its-commands) |
| Determinism, deterministic | Making games | The same state and command always give the same result, on every computer | [Step 5](@/learn/book/box-pusher/05-undo.md#history), [chapter 5](@/learn/book/rules-as-commands.md#determinism-and-the-hash) |
| Seed | Making games | The number a game's luck starts from: the same seed gives the same luck | [Chapter 5](@/learn/book/rules-as-commands.md#turnrules) |
| Stream | Making games | One named run of the seed's luck, one per purpose, so a draw for one never changes another | [Chapter 5](@/learn/book/rules-as-commands.md#luck) |
| Outcome | Making games | How a game ends: a question asked of the state | [Step 4](@/learn/book/box-pusher/04-push.md#winning), [chapter 5](@/learn/book/rules-as-commands.md#turnrules) |
| Tween, ease curve | Making games | A value moving from one place to another over time; how fast it goes on the way | [Chapter 2](@/learn/book/frame-and-skins.md#things-that-move) |
| Pixel | Making games | One dot of a picture | [Chapter 2](@/learn/book/frame-and-skins.md#a-picture-the-skin-paints) |
| Sound effect | Making games | A sound written as a few numbers in a `.sfx.ron` file: a wave, a pitch, how loud | [Chapter 3](@/learn/book/assets-and-formats.md) |
| Dither | Making games | Turn colors into patterns of black and white dots | [Chapter 3](@/learn/book/assets-and-formats.md#a-target-s-own-files) |
| Bot | Making games | A player made of code | [Step 9](@/learn/book/box-pusher/09-test.md#a-bot), [chapter 8](@/learn/book/testing.md#bots) |
| Test | Programming | A small program that plays part of your game and checks what happened | [Step 1](@/learn/book/box-pusher/01-new.md#run-it), [chapter 8](@/learn/book/testing.md) |
| Record, replay | Making games | A level and its commands, and playing it again | [Step 9](@/learn/book/box-pusher/09-test.md#a-replay), [chapter 8](@/learn/book/testing.md#records-and-replays) |
| Hash | Programming | A short number worked out from a whole value, which changes if any of it does | [Step 9](@/learn/book/box-pusher/09-test.md#a-replay), [chapter 5](@/learn/book/rules-as-commands.md#determinism-and-the-hash) |
| Property test, property | Programming | Something that should hold for any input, tried on hundreds of random ones | [Chapter 8](@/learn/book/testing.md#the-rules) |
| Flake | Programming | A test that passes one run and fails the next | [Chapter 8](@/learn/book/testing.md#the-rules) |
| Golden table | Programming | A record's hash at every step, kept beside the tests: the answers known to be right | [Chapter 8](@/learn/book/testing.md#golden-values) |
| Headless | Programming | Run with no window, as a test or a server does | [Chapter 1](@/learn/book/quick-start.md#try-it-with-nothing-installed) |
| Target | Making games | A kind of machine or place a game runs: desktop, web, a handheld | [Step 10](@/learn/book/box-pusher/10-ship.md), [chapter 9](@/learn/book/targets-and-export.md#the-targets-a-game-ships-on) |
| Export, exporting | Making games | Making the package a player downloads, for each of the game's targets | [Chapter 9](@/learn/book/targets-and-export.md#export) |
| Debug, release | Programming | A build made quickly, which checks a lot as it runs; one made slowly to run fast, for players | [Step 10](@/learn/book/box-pusher/10-ship.md#build-it), [appendix B](@/learn/book/fast-compiles.md) |
| Static site | Programming | Files that need nothing but a place to be served from, as a web build is | [Step 10](@/learn/book/box-pusher/10-ship.md#put-it-on-pages) |
| Draft | Making games | A release no player sees until you publish it | [Chapter 9](@/learn/book/targets-and-export.md#releasing) |
| Digest | Programming | A file's fingerprint, which changes if one byte of it does | [Chapter 9](@/learn/book/targets-and-export.md#before-the-reveal) |
| Closed build | Making games | A copy of the game given to one person before its release, marked as theirs | [Chapter 9](@/learn/book/targets-and-export.md#closed-builds) |
| Gate | Programming | A check that must pass before a change goes in | [Step 10](@/learn/book/box-pusher/10-ship.md#check-it), [chapter 1](@/learn/book/quick-start.md#run-it) |
| CI | Programming | A service that runs the gates on each change you push to GitHub | [Chapter 1](@/learn/book/quick-start.md#run-it) |

## Shuyi

| Word | New to | What it means | Where |
| ---- | ------ | ------------- | ----- |
| Screen | Shuyi | One thing the game shows: a title, a level, a menu. Implements `Screen` | [Step 1](@/learn/book/box-pusher/01-new.md#a-screen-title-rs), [chapter 6](@/learn/book/games-and-screens.md#a-screen) |
| Stack, push, pop | Shuyi | The screens, one on another. Only the top hears input; putting a screen on top; taking it off | [Step 1](@/learn/book/box-pusher/01-new.md#a-screen-title-rs), [chapter 6](@/learn/book/games-and-screens.md#the-stack) |
| `Transition` | Shuyi | What `update` answers: stay, push, pop, quit | [Step 1](@/learn/book/box-pusher/01-new.md#the-screen-s-jobs), [chapter 6](@/learn/book/games-and-screens.md#the-stack) |
| Result, `uncovered` | Shuyi | The value a screen hands the one below when it pops | [Step 6](@/learn/book/box-pusher/06-levels.md#a-result), [chapter 6](@/learn/book/games-and-screens.md#asking-the-screen-below) |
| `Ctx`, context | Shuyi | What a screen is given to reach everything: text, colors, saves, data | [Step 1](@/learn/book/box-pusher/01-new.md#what-it-shows), [chapter 6](@/learn/book/games-and-screens.md#the-context) |
| Stage | Shuyi | What the game is really about, which the title starts | [Step 2](@/learn/book/box-pusher/02-board.md#joining-it-up) |
| `BasicData`, `BasicGame`, short path | Shuyi | The short path: what the engine reads from your files, and your game | [Step 1](@/learn/book/box-pusher/01-new.md#what-the-game-is-lib-rs), [chapter 1](@/learn/book/quick-start.md#what-it-is-made-of) |
| Template | Shuyi | What `shuyi new` starts a game from: `blank` or `grid` | [Chapter 1](@/learn/book/quick-start.md#make-a-game) |
| Tool program | Shuyi | The game's `examples/tools.rs`, which the engine's tools run | [Chapter 1](@/learn/book/quick-start.md#what-it-is-made-of) |
| `Own` | Shuyi | The game's own part of its data | [Step 2](@/learn/book/box-pusher/02-board.md#joining-it-up), [chapter 6](@/learn/book/games-and-screens.md#when-a-game-outgrows-the-short-path) |
| View | Shuyi | What's on a screen, as plain data | [Step 1](@/learn/book/box-pusher/01-new.md#what-it-shows), [chapter 2](@/learn/book/frame-and-skins.md#the-view) |
| Skin | Shuyi | What decides how a view or scene looks | [Step 2](@/learn/book/box-pusher/02-board.md#the-skin), [chapter 2](@/learn/book/frame-and-skins.md#the-skin) |
| Console, glyph, cell, `GlyphBuffer`, glyph buffer | Shuyi | The frame's grid of characters; a character; one place in the grid; the whole grid, 48 by 24 | [Step 1](@/learn/book/box-pusher/01-new.md#the-screen-s-jobs), [chapter 2](@/learn/book/frame-and-skins.md#the-frame) |
| Text key | Shuyi | A name for a line of words in `en.ron` | [Step 1](@/learn/book/box-pusher/01-new.md#the-words-en-ron), [chapter 6](@/learn/book/games-and-screens.md#text-by-key) |
| Palette | Shuyi | The game's colors, by name | [Step 1](@/learn/book/box-pusher/01-new.md#the-colors-palette-ron) |
| Debug view, channel, gizmo, role | Shuyi | What your logic works with, drawn over the running game; one thing it can show; a shape it draws; what a gizmo shows, which picks its color | [Chapter 2](@/learn/book/frame-and-skins.md#seeing-what-the-game-sees) |
| Widget | Shuyi | A piece of a screen the engine gives you, such as a menu | [Step 6](@/learn/book/box-pusher/06-levels.md#the-menu), [chapter 6](@/learn/book/games-and-screens.md#the-engine-s-own-screens-and-widgets) |
| Action | Shuyi | Something a player can ask for, like Confirm | [Step 1](@/learn/book/box-pusher/01-new.md#the-keys-keymap-ron), [chapter 4](@/learn/book/actions.md#reacting-to-an-action) |
| Keymap | Shuyi | The file saying which keys give which actions | [Step 1](@/learn/book/box-pusher/01-new.md#the-keys-keymap-ron), [chapter 4](@/learn/book/actions.md#the-keymap-file) |
| Chord, layout | Shuyi | A key with any keys held with it, such as `Shift+a`; one complete set of keys for a player to pick | [Chapter 4](@/learn/book/actions.md#the-keymap-file) |
| `UiAction`, `ActionSet` | Shuyi | The engine's six moves; the trait for a game's own actions | [Step 5](@/learn/book/box-pusher/05-undo.md#the-game-s-own-actions), [chapter 4](@/learn/book/actions.md#actions-of-a-game-s-own) |
| Asset, bundle, embedded | Shuyi | A file the game reads; the game's files; built into its program | [Step 2](@/learn/book/box-pusher/02-board.md#reading-it-level-rs), [chapter 3](@/learn/book/assets-and-formats.md) |
| Kind, format, registry | Shuyi | What you want from a file; what reads it; what finds the format | [Step 2](@/learn/book/box-pusher/02-board.md#reading-it-level-rs), [chapter 3](@/learn/book/assets-and-formats.md#reading-it-through-the-registry) |
| Plugin | Shuyi | A format added to the registry, the engine's or your own | [Chapter 3](@/learn/book/assets-and-formats.md#a-format-of-your-own) |
| Message | Shuyi | A text that chooses its words by a number or a value | [Chapter 3](@/learn/book/assets-and-formats.md#when-a-file-is-wrong) |
| Layers, mod | Shuyi | A bundle read in order, each file from the first layer that has it; a change a player makes to a game, laid over it as a layer | [Chapter 3](@/learn/book/assets-and-formats.md#layers) |
| Pack, baked | Shuyi | The game's files in one indexed file, for fast loading; put there by `cargo shuyi bake` | [Chapter 3](@/learn/book/assets-and-formats.md#the-pack) |
| Folder | Shuyi | A group of the pack's assets, which a game can name to load first | [Chapter 3](@/learn/book/assets-and-formats.md#the-pack) |
| Grid kit, scene, camera | Shuyi | The engine's map painter; the plain data it paints; which part is in view | [Step 2](@/learn/book/box-pusher/02-board.md#the-skin), [chapter 2](@/learn/book/frame-and-skins.md#the-skin) |
| `TurnRules` | Shuyi | The trait of a turn game's rules: `check`, `apply`, `legal_commands`, `outcome` | [Step 3](@/learn/book/box-pusher/03-walk.md#the-rules), [chapter 5](@/learn/book/rules-as-commands.md#turnrules) |
| `History` | Shuyi | The first state and every command since | [Step 5](@/learn/book/box-pusher/05-undo.md#history), [chapter 5](@/learn/book/rules-as-commands.md#history-and-undo) |
| `Menu` | Shuyi | The engine's menu widget | [Step 6](@/learn/book/box-pusher/06-levels.md#the-menu), [chapter 6](@/learn/book/games-and-screens.md#the-engine-s-own-screens-and-widgets) |
| Storage | Shuyi | Where saves go, whatever the machine | [Step 7](@/learn/book/box-pusher/07-save.md#where-saves-go), [chapter 7](@/learn/book/saves-and-settings.md#storage) |
| Profile | Shuyi | One player's saves and settings, kept apart from another's | [Chapter 7](@/learn/book/saves-and-settings.md#profiles) |
| `SaveFormat`, `SaveHeader` | Shuyi | A save's name and version; reading only that version | [Step 7](@/learn/book/box-pusher/07-save.md#what-is-kept), [chapter 7](@/learn/book/saves-and-settings.md#a-save) |
| Save bundle | Shuyi | Several entries of the storage packed into one text file | [Chapter 7](@/learn/book/saves-and-settings.md#bundles) |
| Harness | Shuyi | Plays your game with no window, for tests | [Step 1](@/learn/book/box-pusher/01-new.md#a-test), [chapter 8](@/learn/book/testing.md#the-harness) |
| `GameTools` | Shuyi | What connects a game to the engine's tools | [Step 9](@/learn/book/box-pusher/09-test.md#the-game-s-own-tools), [chapter 10](@/learn/book/tools-for-agents.md#one-value-of-the-game) |
| Snapshot | Shuyi | A frame as text | [Step 9](@/learn/book/box-pusher/09-test.md#a-frame), [chapter 8](@/learn/book/testing.md#a-look) |
| Session | Shuyi | A game the tools start and play, a step at a time | [Chapter 10](@/learn/book/tools-for-agents.md#the-mcp-server) |
| Session record | Shuyi | Everything the platform gave the whole game, frame by frame, to play it again | [Chapter 8](@/learn/book/testing.md#recording-a-session) |
| Id | Shuyi | What says which game this is; its saves are kept under it | [Step 10](@/learn/book/box-pusher/10-ship.md#who-it-is), [chapter 9](@/learn/book/targets-and-export.md#who-the-game-is) |
| Platform backend | Shuyi | The part of the engine that talks to a machine's window, files and sound | [Chapter 9](@/learn/book/targets-and-export.md) |
| Reveal kit | Shuyi | What shows which art and clips are yours and when they existed, made before a game is first shown | [Chapter 9](@/learn/book/targets-and-export.md#before-the-reveal) |
| Tool contract, contract | Shuyi | The operations the engine's tools offer, the same from the command line and to an agent | [Chapter 10](@/learn/book/tools-for-agents.md#the-tool-contract) |
| Agent | Programming | An AI that writes code by reading your files and running commands | [Chapter 10](@/learn/book/tools-for-agents.md) |
| Exit code | Programming | The number a program ends with, saying how it went | [Chapter 10](@/learn/book/tools-for-agents.md#the-tool-contract) |
| JSON Schema, JSON Pointer | Programming | A description of every key a JSON document may have; a path of keys to one value in it | [Chapter 10](@/learn/book/tools-for-agents.md#the-tool-contract) |
| MCP | Shuyi | The way an agent calls the tools of the contract | [Chapter 10](@/learn/book/tools-for-agents.md#the-mcp-server) |
| Skill pack | Shuyi | The engine's instructions for a coding agent, shipped with each game | [Chapter 10](@/learn/book/tools-for-agents.md#the-skills) |
| Problem code | Shuyi | A stable code on every error the tools give, such as `SH0001`, with a page saying how to fix it | [Chapter 3](@/learn/book/assets-and-formats.md#when-a-file-is-wrong), [chapter 10](@/learn/book/tools-for-agents.md#problem-codes) |

## Rust

| Word | New to | What it means | Where |
| ---- | ------ | ------------- | ----- |
| Program, source code | Programming | Instructions a computer follows; the text they're written in, which you edit | [Before you start](@/learn/book/before-you-start.md#what-you-are-about-to-do) |
| Rust | Programming | The language Shuyi and its games are written in | [Before you start](@/learn/book/before-you-start.md#what-you-are-about-to-do), [chapter 1](@/learn/book/quick-start.md#install-rust) |
| Terminal | Programming | A window where you type commands | [Before you start](@/learn/book/before-you-start.md#what-a-terminal-is) |
| rust-analyzer | Rust | What shows Rust's complaints in your editor as you type | [Before you start](@/learn/book/before-you-start.md#a-text-editor) |
| Compiler, compiling, Cargo | Programming, Rust | What turns code into a program; doing it; the tool that runs it for you | [Before you start](@/learn/book/before-you-start.md#what-you-are-about-to-do), [chapter 1](@/learn/book/quick-start.md#install-rust) |
| Container | Programming | A computer of its own inside another, such as the one a Codespace gives you | [Chapter 1](@/learn/book/quick-start.md#try-it-with-nothing-installed) |
| Commit, repository | Programming | One saved version of a project's code; its folder with the history of every change | [Chapter 1](@/learn/book/quick-start.md#make-a-game), [chapter 1](@/learn/book/quick-start.md#on-github) |
| Data, code | Programming | What a program reads and keeps, such as a level or the game's text; the instructions that work on it | [Step 2](@/learn/book/box-pusher/02-board.md#the-level-s-file), [chapter 1](@/learn/book/quick-start.md#what-it-is-made-of) |
| Crate, workspace, dependency | Rust | A package of code; crates built together; code yours is built on | [Step 1](@/learn/book/box-pusher/01-new.md#the-folder-from-the-outside-in), [chapter 1](@/learn/book/quick-start.md#what-it-is-made-of) |
| RON | Rust | A simple text format for data, which the game's files are written in | [Chapter 1](@/learn/book/quick-start.md#what-it-is-made-of) |
| Reverse-DNS | Programming | A web address backwards, `com.example.moon-garden`: a name no one else has | [Chapter 1](@/learn/book/quick-start.md#what-it-is-made-of), [chapter 9](@/learn/book/targets-and-export.md#who-the-game-is) |
| Comment | Programming | A note for people in the code, after `//`, which the computer ignores | [Step 1](@/learn/book/box-pusher/01-new.md#the-game-s-data-three-files-to-play-with) |
| Function, `fn`, call, return | Rust | A named list of instructions; running it; what it gives back. The last value with no `;` | [Step 1](@/learn/book/box-pusher/01-new.md#your-first-rust-main-rs) |
| `use`, `::` | Rust | A shortcut to a name; walking down modules | [Step 1](@/learn/book/box-pusher/01-new.md#your-first-rust-main-rs) |
| Module, `mod`, `pub`, public | Rust | A file that is part of the program; making it so; letting others use it | [Step 1](@/learn/book/box-pusher/01-new.md#what-the-game-is-lib-rs), [chapter 6](@/learn/book/games-and-screens.md#modules) |
| Type | Rust | What kind of thing a value is | [Step 1](@/learn/book/box-pusher/01-new.md#what-the-game-is-lib-rs) |
| Variable, `let`, `let mut` | Rust | A name for a value; one that may change | [Step 1](@/learn/book/box-pusher/01-new.md#what-the-game-is-lib-rs) |
| Macro | Rust | Code ending in `!`, that writes code while compiling | [Step 1](@/learn/book/box-pusher/01-new.md#what-the-game-is-lib-rs) |
| Generics | Rust | The `<...>` blanks in a type, to fill in | [Step 1](@/learn/book/box-pusher/01-new.md#what-the-game-is-lib-rs) |
| Closure | Rust | A small nameless function, `\|x\| { ... }` | [Step 1](@/learn/book/box-pusher/01-new.md#what-the-game-is-lib-rs) |
| `Box`, `dyn` | Rust | A value in a box; "some type with this trait" | [Step 2](@/learn/book/box-pusher/02-board.md#joining-it-up) |
| `struct`, `derive` | Rust | Named values grouped in one; standard abilities written for you | [Step 1](@/learn/book/box-pusher/01-new.md#what-it-shows) |
| `Option`, `Some`, `None` | Rust | Maybe a value | [Step 1](@/learn/book/box-pusher/01-new.md#what-it-shows) |
| Borrowed, `&`, `&mut` | Rust | Lent for a while, not owned; to look at; to change | [Step 1](@/learn/book/box-pusher/01-new.md#what-it-shows) |
| `impl` | Rust | Adding functions to a type | [Step 1](@/learn/book/box-pusher/01-new.md#what-it-shows) |
| Trait | Rust | A list of functions a type promises to have | [Step 1](@/learn/book/box-pusher/01-new.md#the-screen-s-jobs), [chapter 6](@/learn/book/games-and-screens.md#a-screen) |
| `enum` | Rust | One of a fixed list of possibilities | [Step 1](@/learn/book/box-pusher/01-new.md#the-screen-s-jobs), [chapter 6](@/learn/book/games-and-screens.md#the-stack) |
| `if let` | Rust | Open an `Option` or an enum, if it matches | [Step 1](@/learn/book/box-pusher/01-new.md#the-screen-s-jobs) |
| `const`, tuple struct, array | Rust | A fixed value; a struct with unnamed fields; a list of fixed length | [Step 2](@/learn/book/box-pusher/02-board.md#reading-it-level-rs) |
| `?` | Rust | If this is a `None` or an `Err`, return it now | [Step 2](@/learn/book/box-pusher/02-board.md#reading-it-level-rs) |
| `Result`, `Ok`, `Err` | Rust | A value, or what went wrong | [Step 2](@/learn/book/box-pusher/02-board.md#reading-it-level-rs) |
| `match`, `for`, loop, `Self` | Rust | Handle every possibility; a loop; code run once for each item; the type this `impl` is for | [Step 2](@/learn/book/box-pusher/02-board.md#what-can-change-rules-rs) |
| Set, `BTreeSet` | Rust | Values with none twice; a set that always keeps the same order | [Step 2](@/learn/book/box-pusher/02-board.md#what-can-change-rules-rs) |
| serde | Rust | The library that turns a value into text, such as RON or JSON, and back | [Step 2](@/learn/book/box-pusher/02-board.md#getting-ready-serde) |
| Float | Programming | A number with a fractional part. Never in rules | [Step 2](@/learn/book/box-pusher/02-board.md#the-skin), [chapter 5](@/learn/book/rules-as-commands.md#turnrules) |
| Lifetime | Rust | The `'a`: how long a borrow lasts | [Step 2](@/learn/book/box-pusher/02-board.md#the-pieces) |
| Panic | Rust | Stop the program: for what can't happen | [Step 2](@/learn/book/box-pusher/02-board.md#the-screen), [chapter 6](@/learn/book/games-and-screens.md#when-it-goes-wrong) |
| Global | Programming | A value anyone can reach from anywhere. A Shuyi screen is handed its context instead | [Chapter 6](@/learn/book/games-and-screens.md#the-context) |
| Iterator chain, chaining | Rust | Calling one thing after another with `.`, as in `.map(...).filter(...).collect()` | [Step 1](@/learn/book/box-pusher/01-new.md#what-the-game-is-lib-rs), [step 3](@/learn/book/box-pusher/03-walk.md#the-rules) |
| `*`, `_` | Rust | The value a reference points to; "anything else" | [Step 3](@/learn/book/box-pusher/03-walk.md#the-rules) |
| Ownership, `clone` | Rust | Each value has one owner; a copy | [Step 5](@/learn/book/box-pusher/05-undo.md#history) |
| Range, `Vec`, vector, indexing | Rust | `0..n`; a list that can grow; `list[2]` | [Step 6](@/learn/book/box-pusher/06-levels.md#the-menu), [chapter 2](@/learn/book/frame-and-skins.md#the-skin) |
| Match guard, guard | Rust | An arm with `if` | [Step 6](@/learn/book/box-pusher/06-levels.md#a-result), [chapter 6](@/learn/book/games-and-screens.md#asking-the-screen-below) |
| `let ... else`, let-else | Rust | This must match, or else do this | [Step 7](@/learn/book/box-pusher/07-save.md#what-is-kept) |
| Tuple | Rust | Values side by side, `("n", 1)` | [Step 8](@/learn/book/box-pusher/08-polish.md#a-moves-counter), [chapter 2](@/learn/book/frame-and-skins.md#the-skin) |
| JSON | Programming | The most common format for programs to exchange data | [Step 9](@/learn/book/box-pusher/09-test.md#the-scene-what-a-program-can-read), [chapter 10](@/learn/book/tools-for-agents.md#one-value-of-the-game) |
| Feature, `#[cfg]` | Rust | An optional part of a crate; including code only if it's on | [Step 9](@/learn/book/box-pusher/09-test.md#the-game-s-own-tools), [chapter 6](@/learn/book/games-and-screens.md#modules) |
| Build script | Rust | A program Cargo runs before it builds a crate | [Chapter 9](@/learn/book/targets-and-export.md#the-web-build) |
| Semver | Programming | A version's three numbers, like `1.4.2`, that say how big a change was | [Chapter 9](@/learn/book/targets-and-export.md#who-the-game-is) |
| Linker, linking, clean build, rebuild | Programming | What joins compiled code into a program; joining it; building from nothing; building again after a change | [Chapter 1](@/learn/book/quick-start.md#when-it-goes-wrong), [appendix B](@/learn/book/fast-compiles.md) |
| Debug info | Rust | What a build keeps so a debugger can name your code's lines | [Appendix B](@/learn/book/fast-compiles.md#the-setup) |
| Nightly | Rust | The Rust built each night, with features not yet finished | [Appendix B](@/learn/book/fast-compiles.md#on-a-nightly-toolchain) |
