+++
title = "Assets and formats"
description = "Kinds of asset are plain types, formats are plugins that read them, the registry finds them by id in a bundle of layers."
weight = 50

[extra]
chapter = 5
+++

A game is made of more than code. It has words, keys, colors, maps, pictures and
dialogue, and you want to change those without touching the code, and perhaps let
players change them too. Each of these is an **asset**, and this chapter is about how
a game gets from a file to a value it can use.

Change one number in a file, and the game follows, with no code changed:

{{ try_it(name="book-plan-plots") }}

Everything a game reads from files comes through three parts:

- a **kind** of asset is a plain type, what you want from the file: a palette, an
  image, the garden's plan;
- a **format** reads one kind from a file's bytes, and says every problem with its file
  and line;
- the **registry**, `Formats`, holds the formats, and finds an asset by its **id** in
  the game's **bundle**, all its files together.

The code that asks for an asset never sees which format read it, so a faster or smaller
format can come later without touching that code. Standard formats (PNG, WAV, Ogg
Vorbis) always load, and the text formats (RON files, `.map` grids, `.dlg` dialogue)
stay the source of what is in a game.

## A kind of the game's own

The garden's plan is a kind of the game's own, plain data with a name for messages:

{{ snippet(file="src/assets.rs", anchor="kind") }}

> **Rust:** `u8` and `u32` are whole numbers that are never negative, 8 and 32 bits
> long, so a `u8` goes up to 255. `Deserialize` comes from a crate called `serde`, and
> deriving it writes the code that reads this type from a file.
> `#[serde(deny_unknown_fields)]` says a field this type doesn't have is an error
> rather than being ignored, which catches a typo in the file. `impl AssetKind for Plan`
> keeps the one promise of that trait, a `const`, a fixed value: its name in messages.

in a RON file of the game's:

{{ snippet(file="files/garden.ron", anchor="plan", language="ron") }}

## Reading it through the registry

The game registers a RON format for its kind beside the engine's, and asks the registry
for the plan by its id, the file's path with or without its extension:

{{ snippet(file="src/assets.rs", anchor="formats") }}

> **Rust:** `Ron::<Plan>::new()` makes the format that reads a `Plan` from RON. `let mut`
> gives a name to a value that may change, and `register` changes it.
>
> `Result<Plan, Vec<ContentError>>` is either a `Plan`, or the list of what's wrong.
> The `?` after `.load::<Plan>(bundle, "garden")` says: if this is an error, hand it
> back now, and otherwise give me the value inside. `Err(vec![...])` is the same kind of
> error, made by hand, and `Ok(plan)` is the good answer.
>
> `&Bundle` means this function borrows the bundle, only to look.

`read_plan` is the function `BasicGame::with_own` takes in
[chapter 1](@/learn/book/quick-start.md#what-it-is-made-of): it reads the game's own
part when the game starts. A check the format can't make (a percent over 100) comes
after loading, and names the file as the bundle shows it.

`cargo shuyi check` loads every file the game reads and gives every error at once, each
with its file, line and problem code. `cargo shuyi import <file>` reads one file as
every kind whose formats know it, and says what it read.

## When a file is wrong

A broken file should never crash a game, or be half read. A format finds each problem
and says where it is, and the tools say how to fix it. These are real messages from
Moon Garden's `garden.ron`. A **problem code**, like `SH2001`, has a page of its own
that `cargo shuyi explain SH2001` prints.

The wrong type, `grow_percent: "lots"`:

```text
check: 1 error(s)
  files/garden.ron:7:19: error[SH2001]: Expected integer
help[SH2001]: fix the file at the line and column given: the message says what was expected there
```

A field the type doesn't have, as `deny_unknown_fields` asked:

```text
  files/garden.ron:7:27: error[SH2001]: Unexpected field named `rain` in `Plan`, expected one of `plots`, `seeds`, or `grow_percent` instead
```

A number too big for its type, `plots: 300` in a `u8`:

```text
  files/garden.ron:5:15: error[SH2001]: Integer is out of bounds
```

A forgotten closing `)`:

```text
  files/garden.ron:10:1: error[SH2001]: Unexpected end of RON
```

The check `read_plan` makes by hand, which knows the file and not the line:

```text
  files/garden.ron: grow_percent is over 100
```

And no file at all:

```text
  files/garden: error[SH2003]: file not found in asset bundle, nor garden.ron
help[SH2003]: add the file at that path, or fix the name that asks for it
```

## Layers

A bundle is **layers** read in order: a file comes from the first layer that has it. So
a layer laid over the game's files replaces a file at its path, and adds the ones only
it has. A player's **mod**, a change a player makes to a game, is such a layer; on
desktop, `shuyi::app::read_folder` reads a folder on disk into one.

{{ snippet(file="src/assets.rs", anchor="layers") }}

> **Rust:** `BTreeMap` is a list of values looked up by a key. `b"..."` is text as raw
> bytes, which is what a file holds, and `.to_vec()` makes it a list you own.
> `.to_owned()` does the same for a `&str`.

A layer can also be private (files that may not be handed out, such as bought art: the
tools say which files come from one), or hold only the tests' files, which a build that
ships leaves out.

## A format of your own

A file format the engine lacks is a **plugin**, something you add without changing the
engine: a type that implements `Format` (its id, the extensions it claims, an optional
sniff of its first bytes, a size limit, and `decode`), registered beside the others. It
never panics on a broken file, says every problem rather than the first, and reads the
same bytes into the same asset on every platform. The engine's [formats
page](https://github.com/Zafnok/shuyi-engine/blob/main/docs/formats.md) walks through
one, with its tests, and lists the built-in formats.

## Settings beside a file

Some choices differ from one file to the next: how a picture is made one-bit, which
palette it uses. They go in a small RON file beside it, `images/hero.png.meta`, or for a
whole folder in its `_folder.meta`. Each holds sections named after the step that reads
them, such as `(onebit: (dither: bayer))`, and the nearest file with a section wins. A
format reads its section with `DecodeCx::import_settings`, and a typo in any settings
file is an error with its line. The formats page says [how settings are
found](https://github.com/Zafnok/shuyi-engine/blob/main/docs/formats.md#import-settings).

## Reloading while the game runs

Waiting for a rebuild to see a new color gets old. `cargo shuyi run --watch` starts the
game in its window, in a debug build, with the folder of its files on disk laid over
the embedded ones (`assets:` in `shuyi.ron` names it). Save a file there, and within a
second the bundle has its new bytes. The pictures the window draws, the font's and the
images', are redrawn from them with no code of yours; a sprite keeps its source
rectangle, so one past the edge of a smaller picture shows as missing, in magenta.

A value you want to follow it is a `Hot<T>`: any kind, read through the registry, which
remembers every file its decode asked for (its picture, its settings) and reads like
the value. A small module of the game's keeps it, and refreshes it when files change:

{{ snippet(file="src/assets.rs", anchor="hot") }}

> **Rust:** `impl<G: GameData> Module<G> for HotPlan` says `HotPlan` is a module of any
> game: `G` is a blank, and `G: GameData` says it must be a type with that trait. `Arc`
> lets several owners share one value. `'_` lets the compiler work out how long a borrow
> lasts.

A broken file never stops the game: the old value stays, and `note_reload` shows its
error, with its file and line, at the top of the frame until the next good save. A
reload is no input and is in no record, and a build that ships never looks at the disk.
It isn't on the web, which rebuilds the page instead, or on a console.

## The pack

A game may ship its files **baked** into one pack, a `.shuyipak` file indexed for fast
loading. `cargo shuyi bake` makes it, and `cargo shuyi check --baked` proves every asset
read from the pack equal to the one read from its source, so the text files stay what
the game is made of. The formats page says [how a pack is
built](https://github.com/Zafnok/shuyi-engine/blob/main/docs/formats.md#the-pack).

## A target's own files

A **target** is a kind of machine a game runs on. A folder `assets@playdate/` beside
`assets/` holds the files the Playdate uses in place of the game's: a picture drawn by
hand in black and white, a settings file. `Bundle::for_target` lays it over `assets/`
when the game bakes or runs for that target (`ctx.platform.target`), and leaves every
other target's out.

On a black-and-white screen a game bakes every picture once as a one-bit picture
(`Bake::under_as` with `OneBitFile`), so the device never **dithers**, the job of
turning colors into patterns of black and white dots. Each PNG is dithered by its
`onebit` settings: `atkinson` for pictures, `bayer` for flat art and anything that
moves, `threshold(N)` for line art, or `none` for art already black and white, unless
the target's folder has its own. `cargo shuyi bake --target playdate --sheet sheet.png`
names the pictures it dithered and draws each beside its source, so you see which ones
are worth drawing by hand. The formats page says [what each dither
does](https://github.com/Zafnok/shuyi-engine/blob/main/docs/formats.md#the-onebit-section).
