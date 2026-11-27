+++
title = "Assets and formats"
description = "Kinds of asset are plain types, formats are plugins that read them, the registry finds them by id in a bundle of layers."
weight = 30

[extra]
part = "chapters"
chapter = 3
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

More picture and sound formats are a switch each, so a game pays only for those it
uses: JPEG, WebP, QOI and JPEG XL pictures, FLAC and MP3 sounds and music. A game
turns one on as a feature of `shuyi-assets` in its own `Cargo.toml` (`features =
["jpeg", "flac"]`), and the window and `frame-png` then draw and play its files as
they do a PNG, a WAV or an Ogg file. The audio manifest names a `.flac` or `.mp3`
file as it does an `.ogg` one.

A sound can be a few numbers, too. A **sound effect** is a `.sfx.ron` file: a wave, a
pitch that may slide or jump, and how loud it is as it rises, holds and fades. The
game turns it into a sound as it loads it, the same on every machine. A game made
with `shuyi new` already has four, the sounds its menus make, in `files/sfx/`, and
turns on their switch, `sfx`:

```ron
// select: a note that jumps up a fifth, for a choice made.
(
    wave: Square(duty: 50),
    hz: 660,
    hold_ms: 70,
    fade_ms: 60,
    volume: 35,
    step: (at_ms: 40, percent: 150),
)
```

Change `hz` and the note is higher or lower; change `hold_ms` and it lasts longer. A
number out of its range is a problem `cargo shuyi check` gives with its line, like
any other file's.

You don't write one from nothing. `cargo shuyi sfx coin --out
crates/game/files/sfx/coin.sfx.ron` starts one from the engine's coin (`cargo shuyi
sfx nope` lists the others), and `--wav coin.wav` turns it into a sound file you can
play, to hear it before the game does. A cue of `files/audio.ron` then names it.

## A kind of the game's own

The garden's plan is a kind of the game's own, plain data with a name for messages:

{{ snippet(file="src/assets.rs", anchor="kind") }}

{% newcomer(to="rust", topic="`u8`, `serde` and `deny_unknown_fields`") %}
`u8` and `u32` are whole numbers that are never negative, 8 and 32 bits
long, so a `u8` goes up to 255. `Deserialize` comes from a crate called `serde`, and
deriving it writes the code that reads this type from a file.
`#[serde(deny_unknown_fields)]` says a field this type doesn't have is an error
rather than being ignored, which catches a typo in the file.
{% end %}

`impl AssetKind for Plan` keeps the one promise of that trait, a `const`, a fixed
value: its name in messages.

in a RON file of the game's:

{{ snippet(file="files/garden.ron", anchor="plan", language="ron") }}

## Reading it through the registry

The game registers a RON format for its kind beside the engine's, and asks the registry
for the plan by its id, the file's path with or without its extension:

{{ snippet(file="src/assets.rs", anchor="formats") }}

`Ron::<Plan>::new()` makes the format that reads a `Plan` from RON.

{% newcomer(to="rust", topic="`let mut`") %}
`let mut` gives a name to a value that may change, and `register` changes it.
{% end %}

{% newcomer(to="rust", topic="`Result` and `?`") %}
`Result<Plan, Vec<ContentError>>` is either a `Plan`, or the list of what's wrong.
The `?` after `.load::<Plan>(bundle, "garden")` says: if this is an error, hand it
back now, and otherwise give me the value inside. `Err(vec![...])` is the same kind of
error, made by hand, and `Ok(plan)` is the good answer.
{% end %}

{% newcomer(to="rust", topic="a borrow, `&`") %}
`&Bundle` means this function borrows the bundle, only to look.
{% end %}

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

A letter the font atlas can't draw, `Łódź` in `en.ron`. The text is checked against the
atlas once every file loads, and a language keeps its text by key, so the message names
the key, not a line:

```text
  en: 'Ł' (U+0141) isn't in the font atlas (title.subtitle); remake the atlas with font-atlas from a font that has it
```

`cargo shuyi font-atlas files-src/ter-u16n.bdf crates/game/files/font` remakes it: it
runs the game, and keeps every character its text uses that the font has.

Japanese and Chinese characters are twice as wide as a letter: each takes *two cells*
of the screen. They come from a second font, twice as wide (16 × 16 pixels beside
8 × 16), given with `--wide`:

```text
cargo shuyi font-atlas --wide files-src/my-16x16.bdf files-src/ter-u16n.bdf crates/game/files/font
```

The atlas then keeps the wide font's glyphs of every character the text uses that the
narrow font lacks, and of the characters `shuyi.ron`'s `font_atlas.wide_glyphs` names
(a name entry's letters, say).

Japanese and Chinese write many of the same characters with one code point but each in
its own shape (直, 骨, 角). A game in both gives a wide font for each, as a named
*set*:

```text
cargo shuyi font-atlas --wide ja=files-src/ja-16.bdf --wide zh-Hans=files-src/zh-16.bdf files-src/ter-u16n.bdf crates/game/files/font
```

Each language pack names the set it draws from in its `lang.ron`, and a Chinese one says
its quotes, ellipsis and dash (`“ ” ‘ ’ … —`) are wide, as Chinese writes them:

```ron
(name: "简体中文", made_by: Machine, wide_font: Some("zh-Hans"), wide_punctuation: true)
```

The source language has no `lang.ron`: a game written in Chinese says the same in its
code, where it declares the source:

{{ snippet(file="src/assets.rs", anchor="source_lang") }}

A set keeps the characters of the languages that name it, and those `font_atlas.wide_set_glyphs`
names for it (`{"ja": "..."}`). A wide character is drawn from the language's own set
first, then from any other, so a pack's own name shows in its shapes whatever language is
on; `cargo shuyi check` fails on a pack character its set lacks. A font in JIS X 0208, as many Japanese bitmap fonts are,
needs `shuyi-cli`'s `jis` feature in the game's `tools`. Text measured for a layout counts a wide character as
two: `shuyi::assets::lang::text_width` does, with the widths of the frame
(`buf.widths()`), and `buf.text_cells(text)` gives the same as a column count.
The checks that keep text to a length count so too, once they have the atlas's widths
(`CellWidths::of(&atlas)`): a language pack's rules (`PackRules::widths`, or
`LangPackRon::widths_of`), a tip's (`TipRules::widths`) and a dialogue's
(`SceneRules::widths`). So load the font atlas before the text. The source's own text is
measured in its language: `source.widths(&CellWidths::of(&atlas))`, or
`lang.widths(code, &base)` for any language.

A text can choose its words by a number or a value. It is a **message**, written as
translators and their tools know it, ICU MessageFormat's plural and select:

```ron
"seeds.left": "{n, plural, =0 {No seeds left} one {# seed left} other {# seeds left}}",
"sign.water": "{device, select, playdate {Turn the crank} other {Hold to water}}",
```

Each language has its own plural forms: Russian picks among `one`, `few` and `many`,
French counts 0 as `one`. A message follows the rules of the language it is written in,
and `lang.text_in(code, key)` gives a text with that language's tag. Every choice has an
`other` branch, and an apostrophe is a letter (`l'arrosoir`); `'{'` writes a brace.
`shuyi::assets::message::Message` reads one (`Message::parse`) and fills it
(`Message::format`). The languages read each text once, as they load, and keep what it is:
`lang.form_in(code, key)` gives it as a `Form` (plain, a message with its parse, or not a
message), which every `ctx.text_with` fills from, so no frame parses a text.

The languages check their messages as they load. A text that doesn't parse (an unclosed
`{`, or `{}`) fails, naming its file and key, and so does a translation that loses one of
its source's `{name}`s, drops a branch of its source's select (the Playdate's wording,
say), or uses a plural form its language hasn't (French has no `few`). A translation may
wrap its source's `{n}` in a plural of its own, or drop a plural its language doesn't
need. A plural that leaves out a form its language has still loads, showing `other` for
it, and `cargo shuyi lang-status ru` lists it under `missing forms:`. A tip is measured
form by form: each wording of a `{device, select, …}` must fit the box.

## A language's own pictures

A picture with words in it, a shop sign or a title logo, can differ by language. A
language's folder may hold `files/`, its copy of any of the game's files at the same
path: `lang/ja/files/title/end.png` is the Japanese `title/end.png`. Nothing reads them
as text, and they bake into the pack like any other file. A screen asks for the path
each time it draws, `ctx.localized("lang", "title/end.png", |p| images.id(p).is_some())`,
so a change of language shows the right picture at once. It looks along the language's chain (`pt-BR`, then `pt`) for the first copy
the game has, and gives the plain path when there is none. A target that needs its own
copy of a language's picture puts it at the same path in its folder
(`assets@playdate/lang/ja/files/title/end.png`); without one, the language's copy wins
over the target's plain one, since the words matter more.

## Voices

A language's voice clips sit in its voice folder, one per dialogue line, listed in its
manifest (`voice::from_source`), and its cast says which voice says each speaker's lines
(`voice::cast_from_source`). Whatever makes the clips, a speech program or an actor in a
booth, is the game's own tool, and the engine tells that tool what to do:
`voice::plan::plan` takes the dialogue, the scenes to voice, the cast and the manifest's
clips, with three answers only the game knows: which clips a line needs (one, one per
variant, or none for a reply nobody says aloud), the words a clip says, and whether a
generated clip's voice changed (its sample was redone, say). It gives back every clip
wanted, with the line before it, and which to make and why (`Why::Missing`, `Words`,
`Direction`, `Recast`, or the tool's own reason), which a person must record again, which
to delete, and which lines no voice is cast to say. A clip of a scene that isn't voiced is
kept while its line is there. `voice::plan::recording_script(&plan, "guide")` lists one
voice's lines in script order, for a recording session. It reads and writes no file, so
the tool loads the files, plans, and does the work.

## Layers

A bundle is **layers** read in order: a file comes from the first layer that has it. So
a layer laid over the game's files replaces a file at its path, and adds the ones only
it has. A player's **mod**, a change a player makes to a game, is such a layer; on
desktop, `shuyi::app::read_folder` reads a folder on disk into one. On the web, where
there are no folders, `shuyi::app::page::page_layer` reads the text files the game's
page hands it (a page that edits the game's data beside it, and runs it again, as the
[playground](@/learn/playground.md) does with Hello, Grid), and `page::report` tells the
page what was wrong with them: load your data with the layer laid first, and lay it only
if that works. A game started with `run_game`, as the project's is, does all that with no
code of its own, which is how each step of the project plays in its own page's
playground.

{{ snippet(file="src/assets.rs", anchor="layers") }}

{% newcomer(to="rust", topic="`BTreeMap` and bytes") %}
`BTreeMap` is a list of values looked up by a key. `b"..."` is text as raw
bytes, which is what a file holds, and `.to_vec()` makes it a list you own.
`.to_owned()` does the same for a `&str`.
{% end %}

A layer can also be private (files that may not be handed out, such as bought art: the
tools say which files come from one), or hold only the tests' files, which a build that
ships leaves out.

A mod that adds crops to the game's `data/crops.ron`, instead of replacing the whole
file, needs every layer's copy of it: `formats.load_versions::<Crops>(&bundle,
"data/crops")` decodes each one (`bundle.versions(path)` gives their bytes). They come in
the order layers are read, the mod's first and the game's last, so start from the
game's and walk the list backwards (`.iter().rev()`), adding each mod's rows. How rows
are matched, and what two mods changing one row do, is your game's to decide.

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

{% newcomer(to="rust", topic="a generic `impl`, `Arc` and `'_`") %}
`impl<G: GameData> Module<G> for HotPlan` says `HotPlan` is a module of any
game: `G` is a blank, and `G: GameData` says it must be a type with that trait. `Arc`
lets several owners share one value. `'_` lets the compiler work out how long a borrow
lasts.
{% end %}

A broken file never stops the game: the old value stays, and `note_reload` shows its
error, with its file and line, at the top of the frame until the next good save. A
reload is no input and is in no record, and a build that ships never looks at the disk,
but at a folder the game reads with `shuyi::app::read_watched_folder`: a program you hand
to a writer or an artist, built optimized, which plays the files they save in that one
folder. It isn't on the web, which rebuilds the page instead, or on a console.

## The pack

A game may ship its files **baked** into one pack, a `.shuyipak` file indexed for fast
loading. `cargo shuyi bake` makes it, and `cargo shuyi check --baked` proves every asset
read from the pack equal to the one read from its source, so the text files stay what
the game is made of. The formats page says [how a pack is
built](https://github.com/Zafnok/shuyi-engine/blob/main/docs/formats.md#the-pack).

A group of assets is a **folder**. A game whose pack is big, as a web game's can be,
names the folders its first frame reads in its bake, `.first("title/")`, and the bake
lays them at the front of the pack. On the web the page streams the pack in that order:
the game starts once the pack's index has arrived, and its load waits until the bundle's
`arrived("title/")` says yes (`Progress::Wait`, counting what arrived on the card with
`Progress::Of`). A file you keep beside the others, `en.ron`, can be named on its own. A
screen that opens a later part asks `arrived("part2/")` first, and shows its own wait
until it says yes. On desktop and the Playdate every folder has always arrived.

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

## Check yourself

{% check() %}
`Plan` has `#[serde(deny_unknown_fields)]`, and someone adds `rain: 3` to `garden.ron`.
What does `cargo shuyi check` say?

- [x] An `SH2001` error at the file, line and column of `rain`, naming the fields `Plan` does have. — `deny_unknown_fields` makes a field the type doesn't have an error, which catches a typo ([when a file is wrong](#when-a-file-is-wrong)).
- [ ] Nothing: the field is ignored and the plan loads. — That would happen without `deny_unknown_fields`, which asks for the error ([a kind of the game's own](#a-kind-of-the-game-s-own)).
- [ ] Nothing, but the game crashes when it starts. — A broken file never crashes a game: the format finds the problem and says where it is ([when a file is wrong](#when-a-file-is-wrong)).
{% end %}

{% check() %}
`garden.ron` says `grow_percent: 150`. `grow_percent` is a `u8`. What catches it?

- [ ] The RON format, with "Integer is out of bounds". — A `u8` goes up to 255, so 150 reads fine ([a kind of the game's own](#a-kind-of-the-game-s-own)).
- [x] `read_plan`'s own check after loading, whose message names the file but not the line. — The format can't know a percent stops at 100, so `read_plan` checks it by hand ([reading it through the registry](#reading-it-through-the-registry)).
- [ ] Nothing: the game reads it, and every seed grows. — `read_plan` gives back an error when `plan.grow_percent > 100` ([reading it through the registry](#reading-it-through-the-registry)).
{% end %}

{% check() %}
A test makes `Bundle::new(vec![layer, game().bundle().layers()[0]])`, where `layer` holds
only a `garden.ron`. Where do `garden.ron` and `en.ron` come from?

- [ ] Both from the game's layer, since the game's files come first. — The order of the list decides, and `layer` is first ([layers](#layers)).
- [ ] `garden.ron` from the mod's layer, and no `en.ron` at all. — A layer laid over the game's files only replaces the ones it has, and the test checks `bundle.has("en.ron")` ([layers](#layers)).
- [x] `garden.ron` from the mod's layer, and `en.ron` from the game's. — A file comes from the first layer that has it, so a layer replaces the files it has and leaves the rest ([layers](#layers)).
{% end %}
