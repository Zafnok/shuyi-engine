---
name: shuyi-import-art
description: Bring art and maps made in other tools into a game on Shuyi Engine and check what they read as. Tiled and LDtk maps and Aseprite sheets aren't read by the engine yet. This skill says what is read today (PNG pictures, palettes, font atlases, the `.map` grid format) and how `cargo shuyi import` checks a file. Use when adding a picture, a tileset, a map or a font to a game, or when a file's errors need reading. Not for drawing a screen (shuyi-screen) or animating sprites (shuyi-animate).
license: MIT OR Apache-2.0
metadata:
  version: "1.0"
  waits_for: "1123 1124"
---

# Import art and maps

A game's files live in `crates/game/files/`, embedded in its binary,
and every file is read through the engine's format registry. A kind of
asset (a picture, a palette, a map) is never tied to one file format.
The engine's own text formats are always read, and other tools' formats
are added to the registry beside them.

**Not built yet:** maps from Tiled (`.tmx`) and LDtk (`.ldtk`), and
sheets and animation tags from Aseprite. When the engine reads them,
this skill gets their steps and a worked example. Until then, use the
formats below, and don't write an importer of the game's own for them
unless the person who asked for the game wants one.

## When to use it

- Adding a picture, a tileset, a map, a palette or a font to a game.
- A file of the game doesn't read, and you need to know why.
- Someone asks for Tiled, LDtk or Aseprite files.

## When not to

- Drawing a screen with what was imported: `shuyi-screen`.
- A sprite's frames over time: `shuyi-animate`.
- A grid stage's rules and its map: `shuyi-stage-grid`.

## What the engine reads today

| Kind | Files |
| ---- | ----- |
| a picture | `.png` (its size for the image table; the platform draws its pixels) |
| a palette | the palette file, `files/palette.ron`: colours by name |
| a font atlas | `files/font/atlas.ron` and its `.png`, made from BDF fonts by `cargo shuyi font-atlas` |
| a grid map | the `.map` text format: a RON header, a `---` line, one character a tile (a stage's own format, registered by its tools) |
| sounds and music | `.wav` and `.ogg`, and the audio manifest |
| text and talk | the language files, `.dlg` scenes (`shuyi-dialogue`) |
| credits | the credits file (`shuyi-ship`) |

A tileset for a grid map's sprite skin is a PNG of tiles that the game's
skin cuts by rectangle; `cargo shuyi glyph-tiles` draws one from the
font and the palette, when `shuyi.ron` has a `glyph_tiles` entry.

## Steps

1. **Put the file under `crates/game/files/`.** Export pictures from any
   editor as PNG, and draw grid maps in the `.map` format.
2. **Read it back** through the registry, as every kind that knows it:

   ```bash
   cargo shuyi import crates/game/files/stage.map
   cargo shuyi import crates/game/files/palette.ron --kind palette --print
   ```

   The answer lists each kind it read as, with a summary (a map's size
   and pieces, a palette's colours), or every error with its file, line,
   column and code. `--kind` tries one kind; `--print` prints the asset
   as the engine read it, in the engine's text format when its kind has
   a writer. With `--json`, the kinds are in `result.read_as` and the
   errors in `result.errors`.
3. **Load it in the game**: a short-path game reads its text, keys,
   palette and font itself; anything else is a stage's own part, read
   by its `own` function from the bundle (`shuyi-new-game`).
4. **Check the whole game** with `cargo shuyi check`, which loads every
   file through every validator.
5. **Credit it.** A picture, a font or a map you didn't make goes in the
   credits file with its license (`shuyi-ship`).

## When it fails

Branch on each error's `code`, never its message, and read its page with
`cargo shuyi explain <code>`:

- `SH2001`: the file's syntax is wrong, at the line and column given.
- `SH2002`: a text file that isn't UTF-8.
- `SH2003`: a file it names isn't there.
- `SH2005`: no format reads the file: its extension, or the kind given
  with `--kind`, is no format of the registry.
- `SH2006`: the file is bigger than its format reads.
- `SH2008`: a name in the file isn't one it may use (a terrain the
  legend doesn't know).

## Worked example

None yet: it comes with the engine's Tiled, LDtk and Aseprite formats.
