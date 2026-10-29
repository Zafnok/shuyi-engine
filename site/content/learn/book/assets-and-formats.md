+++
title = "Assets and formats"
description = "Kinds of asset are plain types, formats are plugins that read them, the registry finds them by id in a bundle of layers."
weight = 50

[extra]
chapter = 5
+++

Everything a game reads from files (its text, keys, colors, maps,
pictures, dialogue) comes through three parts:

- a **kind** of asset is a plain type: a palette, an image, the garden's
  plan;
- a **format** reads one kind from a file's bytes, and says every
  problem with its file and line;
- the **registry**, `Formats`, holds the formats, and finds an asset by
  its id in the game's **bundle** of files.

The code that asks for an asset never sees which format read it, so a
faster or smaller format can come later without touching that code.
Standard formats (PNG, WAV, Ogg Vorbis) always load, and the text
formats (RON files, `.map` grids, `.dlg` dialogue) stay the source of
what is in a game.

## A kind of the game's own

The garden's plan is a kind of the game's own, plain data with a name
for messages:

{{ snippet(file="src/assets.rs", anchor="kind") }}

in a RON file of the game's:

{{ snippet(file="files/garden.ron", anchor="plan", language="ron") }}

## Reading it through the registry

The game registers a RON format for its kind beside the engine's, and
asks the registry for the plan by its id, the file's path with or
without its extension:

{{ snippet(file="src/assets.rs", anchor="formats") }}

`read_plan` is the function `BasicGame::with_own` takes in
[chapter 1](@/learn/book/quick-start.md#what-it-is-made-of): it reads
the game's own part when the game starts. A check the format can't make
(a percent over 100) comes after loading, and names the file as the
bundle shows it.

`cargo shuyi check` loads every file the game reads and gives every
error at once, each with its file, line and problem code. `cargo shuyi
import <file>` reads one file as every kind whose formats know it, and
says what it read.

## Layers

A bundle is layers read in order: a file comes from the first layer
that has it. So a layer laid over the game's files replaces a file at
its path, and adds the ones only it has. A player's mod is such a
layer; on desktop, `shuyi::app::read_folder` reads a folder on disk
into one.

{{ snippet(file="src/assets.rs", anchor="layers") }}

A layer can also be private (files that may not be handed out, such as
bought art: the tools say which files come from one), or hold only the
tests' files, which a build that ships leaves out.

## A format of your own

A file format the engine lacks is a plugin: a type that implements
`Format` (its id, the extensions it claims, an optional sniff of its
first bytes, a size limit, and `decode`), registered beside the
others. It never panics on a broken file, says every problem rather
than the first, and reads the same bytes into the same asset on every
platform. The engine's [formats
page](https://github.com/Zafnok/shuyi-engine/blob/main/docs/formats.md)
walks through one, with its tests, and lists the built-in formats.

## The pack

A game may ship its files baked into one pack, a `.shuyipak` file
indexed for fast loading. `cargo shuyi bake` makes it, and `cargo shuyi
check --baked` proves every asset read from the pack equal to the one
read from its source, so the text files stay what the game is made of.
The formats page says [how a pack is
built](https://github.com/Zafnok/shuyi-engine/blob/main/docs/formats.md#the-pack).
