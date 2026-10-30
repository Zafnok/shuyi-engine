# ADR-0019: The format registry: kinds are types, a format reads one file, conflicts are per kind

- **Status:** Accepted; extended by ADR-0057 (import settings: per file
  and per folder, beside §5's game-wide choices)
- **Date:** 2026-10-19
- **Related tickets:** 2201 (the registry); 2202, 2203, 2204, 1123 and 1124
  (the formats that come after it)
- **Extends:** ADR-0007 (formats are plugins)

## Context

ADR-0007 decided that asset kinds are separate from file formats, that a
format is a `Format` registered in `Formats`, how lookup goes (sniff, then
extension) and that two formats claiming one extension is an error.
Ticket 2201 built it on the parsers the carve left in `shuyi-assets`.
Building it raised questions ADR-0007 leaves open:

- **What a kind is in code.** `Formats` must keep formats of different
  asset types side by side, and many of the engine's kinds are generic
  over a game's own types (`GridMap<T, F>`, `KeymapDef<A>`, `Scene<D>`,
  `CinematicFile<S, P>`).
- **Where a conflict is.** Most of the engine's files are RON, so every
  RON format claims `.ron`. A registry-wide extension rule would refuse
  the second of them.
- **What a format may read.** Many parsers check one file against other
  assets: a scene against the game's characters, a cinematic against the
  music's lengths, a voice manifest against the dialogue, a pack against
  English. Those assets borrow from the game, while a registered format
  must be `'static`.
- **One file in two kinds.** The image table needs a PNG's size from its
  header and never its pixels. Ticket 2203's decoders, and the pack of
  2204, need pixels.
- **Asking by id.** "Code that uses an asset asks for the kind by id and
  never sees which file it came from" (ADR-0007 §1) is not true of a path
  that names the file's extension.
- **Limits.** ADR-0007 §5 asks for size limits but not where they live.

## Decision

In `shuyi_assets::format` (`crates/assets/src/format.rs`), documented
for users in `docs/formats.md`:

### 1. A kind is a type

- `trait AssetKind: Sized + 'static { const NAME: &'static str; }`. A
  kind is the type itself; `NAME` is for messages (`"image"`).
- `Format::Asset: AssetKind`. Every other part of `Format` is as
  ADR-0007 §2 shows, plus `max_bytes` (§4 below).
- A generic type is a kind per instance: `KeymapDef<GameAction>` is the
  game's keymap kind.
- `Formats` keeps one list per kind behind `dyn Any`, found by
  downcasting, so it builds without `std`.

### 2. Conflicts are per kind

- Within one kind, no two formats share an id or an extension.
  `Formats::register` refuses the second with `RegisterError`, whose
  message names both. `Formats::replace` puts the new format in the place
  of every one it clashes with (the first of them) and returns their ids.
- Different kinds may claim one extension. The kind asked for decides
  which format reads a file.
- An extension is lowercase, has no dot and isn't empty. A format may
  claim none and be found only by its magic bytes (`OneBitInfo`, a
  one-bit picture standing in a PNG's place).

### 3. Lookup

- `Formats::decode::<K>(file, bytes, cx)`: kind `K`'s formats whose
  `sniff` says yes, in registration order, then the one that claims the
  file's extension (in any case), each tried once. The first that decodes
  the file gives the asset. When none does, the errors are the first
  one's, the most specific.
- `Formats::find::<K>(bundle, id)`: the file at `id` if the bundle has
  one, else the one file at `id.<extension>` for an extension kind `K`'s
  formats claim. Two such files is an error naming both.
  `Formats::load::<K>(bundle, id)` finds and decodes; `load_under` reads
  every claimed file under a directory.
- `AssetPath` is a file's bundle path and the name errors show it by.
  `Bundle::has` checks for a file without telling the watcher of a read.

### 4. Limits

- `Format::max_bytes`, `DEFAULT_MAX_BYTES` (16 MiB) unless the format
  says. The engine's text formats read at most `TEXT_MAX_BYTES` (4 MiB),
  PNG 64 MiB, one-bit pictures 16 MiB, sound files 64 MiB.
- `Limits { max_bytes }`, in `Formats::limits` and `DecodeCx`: a lower
  cap for the whole registry, for mods and downloads.
- `format::decode_with` refuses a file over either before the format's
  `decode` runs. Every lookup goes through it.

### 5. A format reads one file, configured by the game

- A format reads one file and the files it names, through
  `DecodeCx { limits, bundle }` (`cx.bytes`, `cx.text`, `cx.shown`,
  `cx.left_out`, `cx.files_under`).
- What the game decides goes in the format's value when it is
  registered:
  - its types, as type parameters (`Dlg<D>`, `KeymapRon<A>`);
  - its rules (`PaletteRon::requiring`, `FontAtlasRon::requiring`,
    `LangPackRon::new(limits, tokens)`);
  - and owned closures (`MapText::new(format, terrain)`).
- A check against another loaded asset runs after loading, with the
  functions the carve left: `dialogue::scripts_from_parsed` (new, the
  checking half of `scripts_from_sources`), `cinematic::check`,
  `voice::from_source`, `tip::from_source`, `credits::merge`,
  `lang::from_files`.

### 6. An image is two kinds

- `ImageInfo`, its size, is what the image table reads. `PngInfo` reads
  the header only; `OneBitInfo` reads a one-bit picture's source size.
- `Image`, its RGBA pixels, is what a decoder of pixels gives. `Png`
  reads it with the `png` feature, after checking the size.
- `ImageTable::load_with(bundle, formats, skip)` builds the table with
  every image format a game registers. With `Formats::standard()` it
  reads exactly what `ImageTable::load` reads.

### 7. What is standard

`Formats::standard()` (or `register_standard` on a registry) holds the
formats that need nothing of the game:

- PNG for a size, one-bit, and PNG for pixels with `png`;
- WAV and OGG Vorbis for a sound, OGG Vorbis for a music track;
- the audio manifest, credits, font atlas, palette, lights and names
  sidecars;
- `.dlg` without directives.

The `.map`, keymap, cinematic and language-pack formats read a game's
own types or follow its rules, so the game registers them. A game's own
data table is `ron::Ron<T>`.

### 8. The existing API stays

Every parser keeps its function (`from_source`, `parse`, `load`), and
each format calls it. A game moves to the registry one loader at a time.

## Consequences

- A new format is one type and one `register` call. The code that asks
  for its kind doesn't change: the toy `*.txtimg` plugin
  (`crates/assets/tests/it/txtimg.rs`) proves it for an image.
- Every built-in format has a property test that random bytes, and its
  fixtures with a few characters changed, never panic. Writing them found
  that a one-bit picture's header could overflow `usize` on a 32-bit
  target; the sizes are now checked arithmetic.
- A format of a generic kind is registered by the game, so the registry
  can't hold it until the game's types exist. `Formats::standard()` is
  smaller than "every format the engine has".
- Checks across assets stay outside the formats. A game loads its
  assets, then checks them, as it did before, but through the registry.
- The `ImageInfo` and `Image` split means a format that reads pixels
  also needs a size-only twin for the image table. The toy shows the
  pattern: a second small format that reads through the first.

## Alternatives considered

- **Extension conflicts across the whole registry** — every RON format
  would conflict with the first. Giving each RON file its own extension
  would rename files games already have.
- **A format that can read other assets through `DecodeCx`** (a typed map
  of loaded assets) — the assets a check needs borrow from the game, and
  a check that needs them is not reading a file. It would also hide the
  order assets load in.
- **One image kind that holds an optional decoded picture** — every
  decoder would face a flag, and the image table would pay for pixels or
  pretend it has them.
- **Asking only by full path** — the code asking would change with the
  file's format, which is what ADR-0007 §1 rules out.
- **`Format` generic over the kind (`Format<K>`) instead of an associated
  type** — one type could then read several kinds, but lookup could not
  name a format's kind, and ADR-0007 §2 shows the associated type.
