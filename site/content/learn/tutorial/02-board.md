+++
title = "A level on the board"
description = "A level as a text file, read through the format registry, held as the rules' state and drawn by the grid skin."
weight = 20

[extra]
step = 2
+++

By the end of this step, pressing Confirm on the title opens a real level:

{{ try_it(name="board") }}

You can't move yet. This step is about three questions every game has to answer:
where does a level come from, how does the game hold it in memory, and how does it
get onto the screen? In Shuyi, each of those has its own file, and keeping them apart
is most of what makes a game easy to change.

| File | Is it new? | What it does |
| ---- | ---------- | ------------ |
| `crates/game/files/levels/1.map` | New | The level, as text |
| `crates/game/src/level.rs` | New | Reads that text |
| `crates/game/src/rules.rs` | New | What can change in the puzzle |
| `crates/game/src/board.rs` | New | The screen that shows it |
| `crates/game/src/stages.rs` | Changed | Tells the title to start the board |
| `crates/game/src/lib.rs` | Changed | Names the three new modules |
| `crates/game/files/palette.ron`, `en.ron` | Changed | The new colors and words |
| `Cargo.toml`, `crates/game/Cargo.toml` | Changed | Adds `serde` |
| `crates/game/tests/board.rs` | New | A test of the board |

Your project should end up like
[this step's copy](https://github.com/Zafnok/shuyi-engine/tree/main/site/tutorial/02-board).

## The level's file

> **Game idea:** keep a game's **data** apart from its **code**. If a level is a text
> file, then changing one never means changing code, and a level designer needs no
> programmer. It also means a player could make their own.

Make the folder `crates/game/files/levels/`, and in it the file `1.map`:

{{ snippet(step="02-board", file="crates/game/files/levels/1.map", language="ron") }}

This is the engine's `.map` format. Below the `---` line, each character is a
**tile**, one square of the level. Above it, the header says what those characters
mean. `legend` says that `.` is floor, `#` is wall, and `o` is a target, a square a
box should end up on.

`features` says what else stands where. Tiles are counted like cells: `(x, y)`, from
the top left, starting at 0. So `(2, 2): Start` is the third column of the third row,
where the player begins, and `(3, 2): Box` is the square right beside them.

This is so readable that you can already play with it. Here is the room made twice as
wide and twice as tall:

{{ try_it(name="level-double") }}

Once you've finished this step, try that, then put it back. Doing it right now would
give you an error, since nothing reads this file yet.

## Getting ready: `serde`

The rules' state has to be able to turn into text and back, for saves and replays.
Rust has a well-known library for that called **serde**. Add it to your game's
dependencies. In the top `Cargo.toml`, under `[workspace.dependencies]`, add the
line `serde = { version = "1", features = ["derive"] }`, and in
`crates/game/Cargo.toml`, under `[dependencies]`, add `serde.workspace = true`.

Then, in `crates/game/src/lib.rs`, name the three files you're about to write:

{{ snippet(step="02-board", file="crates/game/src/lib.rs", anchor="mods") }}

(Your game won't compile again until all three exist. That's fine: you'll finish them
all before you run it.)

## Reading it: `level.rs`

Make `crates/game/src/level.rs`. It starts by saying what it uses:

{{ snippet(step="02-board", file="crates/game/src/level.rs", anchor="uses") }}

Those `{ }` in a `use` take several names from one place at once. Then, give each
kind of ground a name, and each kind of thing that stands on it:

{{ snippet(step="02-board", file="crates/game/src/level.rs", anchor="terrains") }}

> **Rust:** `const` makes a fixed value. `TerrainId(0)` is a **tuple struct**: a struct
> whose fields have no names, here just wrapping a number, so that a terrain can never
> be mixed up with some other number. `[&str; 3]` is an **array** of exactly three
> pieces of text, and `&str` is borrowed text, as opposed to the owned `String`.
>
> `pub fn terrain_named(name: &str) -> Option<TerrainId>` turns a word from the legend
> into a terrain, or `None` if there's no such word. The `?` at the end of the
> `position` line says: "if this is `None`, stop here and return `None`; if it's
> `Some`, carry on with what's inside". `?` is how Rust passes a problem up to whoever
> called, in a single character, and you'll use it constantly.
>
> `enum Feature` lists all the things a tile may hold, `Start` or `Box`. Enums are how
> you say "one of these, and nothing else", and you'll write a lot of them.

`impl MapFeature for Feature {}` promises the engine's `.map` format that `Feature` is
a thing a tile can hold. The trait asks for no functions of its own, so the `{}` is
empty. And the `derive` line now has `Serialize, Deserialize`: those come from serde,
and let this enum be written out and read back.

Next comes the reading itself:

{{ snippet(step="02-board", file="crates/game/src/level.rs", anchor="read") }}

> **Shuyi:** the **format registry**. Everything a game reads from a file goes through
> three parts. A **kind** is what you want (here `Level`). A **format** knows how to read
> one kind from one sort of file (`MapText` reads `.map`). And the registry, `Formats`,
> holds all the formats, and finds the right one for a file. Your code just says "load
> me a `Level` from this file", and never cares what's in the file. So a game could
> switch to a different level editor later, and only the format changes.
>
> The **bundle** is your game's files, the ones `include_dir!` put inside it.

> **Rust:** `Result<Level, Vec<ContentError>>` is the other type you'll see everywhere.
> `Option` says "maybe a value". `Result` says "a value, or what went wrong": it's
> either `Ok(value)` or `Err(problem)`. The `?` works on these too, passing an `Err` up
> to the caller.
>
> `load::<Level>` fills in a generic by hand, to say which kind you want, a form of
> writing people call the "turbofish". `let _ = ...` means "I know this gives something
> back, and I don't need it".

If a level makes no puzzle, say there's no start, `read` reports it as a problem with
that file, along with every other problem, as the game loads. So you hear about a bad
level straight away, with its file name, and not when a player reaches it. Try it after
this step: remove `(2, 2): Start,` from the level, and run `cargo shuyi check`.

## What can change: `rules.rs`

Here's the most important file of your game. Make `crates/game/src/rules.rs`:

{{ snippet(step="02-board", file="crates/game/src/rules.rs", anchor="uses") }}

{{ snippet(step="02-board", file="crates/game/src/rules.rs", anchor="state") }}

> **Game idea:** the **state** is everything that's true right now, and could change.
> In this puzzle, that's only where the player and the boxes are. The walls never move,
> so the state just keeps a copy of the tiles. Whatever isn't in the state doesn't exist
> as far as the rules are concerned.

Look at the top of the file: it uses no screens, no keys, no colors. That's on purpose.
In step 3, you'll see why this matters so much.

> **Rust:** `Pos` is a position, an `x` and a `y`, and `Grid<TerrainId>` is the engine's
> two-dimensional grid of terrains. `BTreeSet<Pos>` is a **set**: a collection with no
> duplicates, which is just what boxes are, since no two share a tile.
>
> Why `BTreeSet`, and not the better-known `HashSet`? A `BTreeSet` always keeps its
> items in the same order, while a `HashSet`'s order differs from one run to the next.
> Shuyi's rules must do exactly the same thing every time, on every computer. You'll see
> what that buys you in step 5 and step 9. So in rules, always use `BTree`.

Now to build a `Puzzle` from a level:

{{ snippet(step="02-board", file="crates/game/src/rules.rs", anchor="new") }}

This function goes through the level's features, and checks that they make a puzzle.
Reading it will show you a lot of Rust at once:

> **Rust:** `Self` means "the type this `impl` is for". `Puzzle::new` is the usual way of
> writing a function that makes a new one: `new` isn't special, it's just a habit.
>
> `for (&pos, feature) in &level.features` is a **loop**, running once for each feature,
> and calling its parts `pos` and `feature` each time.
>
> **`match`** is like an `if` for all the possibilities of a value. It tries each arm
> from the top, and the compiler refuses to compile a `match` that misses a possibility.
> This is one of the best things about Rust: add a new kind of `Feature` later, and the
> compiler will take you to every `match` that needs to think about it.
> `Feature::Start if player.is_some()` is an arm with an extra condition.
>
> `let player = player.ok_or("no start")?;` turns the `Option` into a `Result`: a `None`
> becomes the error "no start", which `?` returns.

And then a few small functions that let others read the state:

{{ snippet(step="02-board", file="crates/game/src/rules.rs", anchor="getters") }}

{{ snippet(step="02-board", file="crates/game/src/rules.rs", anchor="targets") }}

Notice that `tiles`, `player` and `boxes` in the struct aren't `pub`, so no other file
can change them. These functions let others look, and only look. That's how it stays
true that only the rules can change the rules' state.

## Drawing it: `board.rs`

In step 1, you met the idea that graphics are a skin. Now you'll use it. Make
`crates/game/src/board.rs`:

{{ snippet(step="02-board", file="crates/game/src/board.rs", anchor="uses") }}

First, how much of the frame the level may use. A `Rect` is a rectangle: left, top, width,
height. This one leaves the top and bottom rows free for text:

{{ snippet(step="02-board", file="crates/game/src/board.rs", anchor="area") }}

### What a terrain looks like

{{ snippet(step="02-board", file="crates/game/src/board.rs", anchor="looks") }}

Each row here is one terrain, in the order `level.rs` numbered them. Each tile takes up
**two cells**, side by side, because a cell is twice as tall as it is wide, so two make a
square. Floor is two spaces, a wall is two blocks, and a target is `<>`. Then come two
names from the palette, for the text color and the background.

Try changing a wall's look. Only the first part of the row changes:

{{ try_it(name="wall-look") }}

To paint the terrains, the engine asks about them through a trait:

{{ snippet(step="02-board", file="crates/game/src/board.rs", anchor="terrains") }}

`TERRAIN_LOOKS.get(...)` gives an `Option`, since there might be no such row, and `?`
handles that. `usize::from(id.0)` takes the number from inside the `TerrainId` (`.0`
means the first field of a tuple struct), and turns it into the kind of number used for
counting positions in a list, `usize`.

### The skin

> **Shuyi:** the **grid kit** (`shuyi::ui::grid`) paints maps for you. You build a
> **scene**, plain data saying which terrain and which pieces are where, and a **skin**
> paints it. A `GlyphSkin` paints with characters, and a sprite skin paints with pictures.
> They take the same scene. So your rules and your scene are already ready for art.

{{ snippet(step="02-board", file="crates/game/src/board.rs", anchor="glyphs") }}

This names the palette's colors the skin uses for things this game won't draw (a path, a
cursor, a threat), and how fast a piece glides. `8.0` is a **float**, a number with a
fractional part. Floats are fine here, as this is only how something looks. They are never
allowed in the rules: floats can round differently on different computers, and rules must
be the same everywhere.

### The pieces

Now the things that stand on the tiles. Each is described as data:

{{ snippet(step="02-board", file="crates/game/src/board.rs", anchor="piece") }}

An enum's possibilities can hold data: `Who::Box(usize)` is a box with a number. And
`MapUnit` is the trait that tells the kit what it needs to know about a piece. (`type Id =
Who;` fills in a blank the trait left, like a generic.)

Then the code that paints the pieces:

{{ snippet(step="02-board", file="crates/game/src/board.rs", anchor="pieces") }}

It takes each piece, picks its characters and its color, finds its cell with
`glyph::cell_of`, and writes the characters there. `Cell { glyph, fg, ..cell }` makes a
new cell with this character and text color, and everything else, like its background,
from the old one.

> **Rust:** the `'a` in `Pieces<'a>` is a **lifetime**. A struct that holds a borrow (the `&`)
> has to say so, so the compiler can check the borrowed thing lives long enough. You'll seldom
> need to write these yourself: when you do, the compiler tells you what to put.

So the player's character is one small piece of data:

{{ try_it(name="player-look") }}

### The screen

The screen holds the state, and the name to show:

{{ snippet(step="02-board", file="crates/game/src/board.rs", anchor="boardview") }}

{{ snippet(step="02-board", file="crates/game/src/board.rs", anchor="board") }}

{{ snippet(step="02-board", file="crates/game/src/board.rs", anchor="new") }}

Starting it reads the level from the game's data, and builds the puzzle.
`unwrap_or_else(|e| panic!("{e}"))` says: if this is an error, **panic**, stop the
program with that message. A panic is for things that can't happen if the game is right.
It can't happen here, as `read` already checked this level when the game loaded. The comment
says so.

Then it builds its scene:

{{ snippet(step="02-board", file="crates/game/src/board.rs", anchor="scene") }}

The scene is built new every frame, from the state. That's a good habit: there's only one
copy of what's true, so what's on screen can never disagree with it. The **camera** picks
which part of the level is in view, here centred on the player.

{{ snippet(step="02-board", file="crates/game/src/board.rs", anchor="view") }}

{{ snippet(step="02-board", file="crates/game/src/board.rs", anchor="view_tiles") }}

The screen's jobs are the ones you saw on the title. For now, its only action is Cancel,
which goes back to the title with `Transition::Pop`:

{{ snippet(step="02-board", file="crates/game/src/board.rs", anchor="screen_top") }}

And `draw` puts it all together:

{{ snippet(step="02-board", file="crates/game/src/board.rs", anchor="draw") }}

`print(1, 0, ...)` prints the level's name at the top left, and the help goes on the last
row. `GLYPHS.paint(...)` then paints the scene in `AREA`.

The very last function of the file is what lets a test find this exact screen on the
stack, by asking "are you a `Board`?". Every screen you want a test to look inside
needs it:

{{ snippet(step="02-board", file="crates/game/src/board.rs", anchor="as_any") }}

Close the `impl` with a final `}`.

## Joining it up

`crates/game/src/stages.rs` is the file that was empty in step 1. It now names your stage,
reads your own data, and answers the title's question of what comes first. Replace the whole
file with this:

{{ snippet(step="02-board", file="crates/game/src/stages.rs") }}

> **Shuyi:** `Own` is the game's own part of its data, which `BasicData` carries for you
> alongside the words and the keys, in `ctx.game.own`. And `game()` in `lib.rs` already
> passes `stages::own` to `BasicGame::with_own`. That's why step 1 could say `Own = ()`,
> a value that holds nothing, and you could change it here with no other change.

> **Rust:** `Box<dyn Screen<Data>>` means "a box holding some type that implements
> `Screen`, and which type isn't known until the game runs". That is how a stack can hold
> a title, a board and a menu all at once. You'll keep seeing it, and you can read it as
> "any screen".

Now the words and the colors it asked for. In `files/en.ron`, add this before the last `}`:

{{ snippet(step="02-board", file="crates/game/files/en.ron", anchor="board", language="ron") }}

and in `files/palette.ron` add the game's own colors:

{{ snippet(step="02-board", file="crates/game/files/palette.ron", anchor="game", language="ron") }}

## Run it

```bash
cargo run
```

If the compiler has complaints, read them from the **top** down, as one mistake can cause
many later messages. Fix the first and run it again. Compare with
[this step's copy](https://github.com/Zafnok/shuyi-engine/tree/main/site/tutorial/02-board)
when you're stuck.

Press Enter on the title, and you'll see the picture from the top of this page. Note that
the title now shows a line saying how to play: it appeared because `stages::names()` isn't
empty any more. Backspace goes back. That's `Transition::Pop`, taking the board card off the
stack, and showing the title that was underneath, unchanged.

Colors work like they did before, for your own colors as much as the engine's:

{{ try_it(name="box-blue") }}

## A test

Make `crates/game/tests/board.rs`. Each file in `tests/` is a separate little program:

{{ snippet(step="02-board", file="crates/game/tests/board.rs") }}

`on_board` and `board` are two helpers, used by every test from now on. This test checks that
the level's name is on screen, that the puzzle has one box, and that Cancel closes the
board. Run `cargo test --workspace`.

Notice that it reads the state (`puzzle.boxes()`) and not the picture. Tests of what happened
read the state. So you could repaint the entire game, and this test would never notice.

## What you learned

- **Rust:** `const`, tuple structs and arrays, `Result`, the `?` operator, `match`, `for`,
  `Self`, `panic!`, and lifetimes when you can't avoid them, `dyn`, and why `BTreeSet`.
- **Game ideas:** tiles, state, and data apart from code.
- **Shuyi:** `.map` files, the format registry and the bundle, `Own`, the grid kit, scenes and
  skins, and cameras.

For another take on the same ideas, [chapter 5](@/learn/book/assets-and-formats.md) explains the
registry, and [chapter 4](@/learn/book/frame-and-skins.md) the skins. Next, you'll walk.
