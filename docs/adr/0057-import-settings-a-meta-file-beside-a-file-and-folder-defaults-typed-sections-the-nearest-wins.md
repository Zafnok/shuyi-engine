# ADR-0057: Import settings: a `.meta` file beside a file and folder defaults, typed sections, the nearest wins

- **Status:** Accepted; extends ADR-0007 and ADR-0019 §5
- **Date:** 2026-10-29
- **Related tickets:** 2229 (this); 3007 (`onebit`), 3019 (`indexed`,
  parked), 3025 (`ds`, parked), 2328 (`lit`, parked), which register
  sections; 2220 (hot reload), 2516 (the files no load reads), 2349 and
  2238 (the language readers), which meet the settings files; 3008
  (budgets), which may leave them out of packs

## Context

A format reads one file and the files it names through
`DecodeCx { limits, bundle }`, and ADR-0019 §5 puts what a game decides
in the format's value when it is registered. Every choice is game-wide.
Several tickets want a choice per file or per folder:

- 3007: an image's one-bit dither and alpha cut (ADR-0008 §4: "An
  image's sidecar may choose `dither: ...`");
- 3019 (parked): an image's indexed group, depth and dither;
- 3025 (parked): a DS texture format per texture;
- 2328 (parked): an atlas's normal map.

Each says "in the sidecar", and nothing says what that file is called,
where it sits, what a folder may set or how a typo is reported. The
engine's RON sidecars (font atlas, palette, lights, names, the audio
manifest) are assets of their own, not settings for how another file is
read.

Bevy keeps a loader's settings in a `.meta` file beside an asset; Godot
in a committed `.import` file beside each source, with defaults per
importer; Unity in a `.meta` file per asset, with Presets by folder or
name. Unreal keeps its Interchange pipelines with the imported asset.

The name is constrained by the registry. `Formats::claims::<K>(path)`
takes the extension after the last dot, so a name like
`hero.png.import.ron` would be claimed by every RON format, and
`Formats::load_under`, `Formats::load_each` and `Bake::under` would
decode it as a palette or a game's data table. The language readers call
any unknown file in a language's directory an error. No format claims
`meta`.

## Decision

In `shuyi_assets::import_settings` (`crates/assets/src/import_settings.rs`)
and `shuyi_assets::format`, documented for users in `docs/formats.md`,
*Import settings*:

### 1. The files

- `<file>.meta` beside its file, the file's full name kept:
  `images/hero.png.meta`. `hero.png` and `hero.ogg` never share one.
- `_folder.meta` in a folder holds the defaults for every file in it and
  below it. Not a dotfile, so every tool and listing shows it.
- The contents are RON, a struct of sections:
  `(onebit: (dither: bayer), indexed: (group: "ui"))`. `()` has no
  section. A section given twice in one file is an error.
- `import_settings::SETTINGS_EXTENSION` (`"meta"`) and `FOLDER_SETTINGS`
  (`"_folder.meta"`) name them; `is_settings_file`, `settings_of` and
  `settings_chain` work them out from a path.

### 2. `meta` is reserved

- `Formats::register` and `Formats::replace` refuse a format that claims
  `meta`, with `RegisterError::Reserved`.
- Every walker over a folder skips settings files. `load_under`,
  `load_each`, `Bake::under` and `ImageTable::load_with` keep only files
  a format claims, so they skip them already; `ImageTable::load` keeps
  only `.png` files; `LangPackRon` and `lang::from_files` skip them by name.

### 3. Sections are typed and registered

- `pub trait ImportSettings: DeserializeOwned + Default + 'static
  { const SECTION: &'static str; }`. A section is a struct with
  `#[serde(default, deny_unknown_fields)]`.
- `Formats::register_import_settings::<S>()` registers one. It refuses,
  in this order, a second section of the same name, a type that can't
  read `()` (a field without a default), and a type that reads
  `(shuyi_probe: ())` (it doesn't deny unknown fields). Each refusal is
  its own `RegisterError` variant, naming the section and, for the last
  two, the attribute to add.
- The engine registers its sections in `Formats::register_standard`, a
  game its own at start-up, whether or not the step that reads them runs:
  `shuyi check` then sees a typo in a Playdate setting on a desktop
  build. `Formats::standard()` registers none yet; each step's ticket
  registers its own (`onebit` 3007, `indexed` 3019, `ds` 3025, `lit`
  2328).
- A section is named after the step that reads it (`onebit`,
  `indexed`), never after a file format (`AGENTS.md` rule 10).

### 4. Which wins

- For a file, the files looked at are its own `.meta`, then
  `_folder.meta` in its folder, then in each folder above it up to the
  root (`settings_chain`).
- A section is taken whole from the first of them that has it. Fields
  are not merged across files: a folder's `dither` doesn't fill in a
  file's own `onebit` section that leaves `dither` out; the section's
  own `Default` does.
- With no file that has it, the section's `Default`.

### 5. Layers

- A settings file is a file: a higher layer's copy replaces a lower
  one's (`assets@playdate/images/hero.png.meta`, 3007's layers, replaces
  `assets/images/hero.png.meta`).
- Nearness wins over layers. The chain is walked by nearness, and each
  step reads through every layer, so a target layer's `_folder.meta`
  doesn't override a file's own `.meta` in the base layer; a
  `<file>.meta` in the target layer does.

### 6. Read through the bundle

`DecodeCx::import_settings::<S>(file)` reads each file of the chain
with `DecodeCx::bytes`, so `bundle::watch` hears of every settings file
looked for, found or not, and 2220's `Hot` tracks it with the asset's
other reads. Without a bundle it gives `S::default()`. A settings file
with bad RON on the way is an error even if a farther one has the
section: the nearest file is broken, and guessing past it would read
settings the author didn't mean.

### 7. Errors

- Every error is a `ContentError` with the settings file's shown name,
  a line and a column:
  - bad RON (`codes::PARSE`);
  - a section given twice (`codes::DUPLICATE`);
  - an unknown section, naming the registered ones
    (`codes::UNKNOWN_NAME`);
  - an unknown field or a bad value inside a section (`codes::PARSE`);
  - a `<name>.meta` whose file the bundle lacks
    (`codes::MISSING_FILE`), unless a layer leaves that file out on
    purpose (`Bundle::left_out`);
  - a settings file of a settings file (`hero.png.meta.meta`,
    `_folder.meta.meta`).
- The splitter reads a settings file as a struct whose values are
  `&ron::value::RawValue`, which borrow each section's text from the
  source. A section's start is its offset in the source, counted into a
  line and column. An error inside a section, which RON counts from the
  section's own line 1, is moved to the file's line (and, on that first
  line, the file's column).
- A settings file is a text file read at most `TEXT_MAX_BYTES` and the
  registry's `Limits` (`codes::TOO_BIG`), and never a panic (ADR-0019).
- `Formats::check_import_settings(bundle)` checks every settings file of
  a bundle. `Bake::bake` returns its errors with its own, so a typo
  refuses the bake. `shuyi check` will run it on the bundle and formats
  of the game's `GameTools::bake(None)` (ticket 2571).

### 8. Packs

A settings file goes into a pack as its own bytes, like any other file:
`Bake::bake` keeps every file, `Bake::check` wants every file, and a
file kept raw in the pack and decoded at run time reads the same
settings it read from the sources. Leaving settings out when every file
they apply to is baked is a follow-up, if a target's budget (3008)
needs the bytes.

### 9. Settings are not data

Settings say how a file is read or baked. The engine's RON sidecars
stay assets, and a game's data stays its own kinds. The engine's
sections hold integers, bools, enums and strings, never floats.

### 10. ADR-0019 §5 still holds

Game-wide choices stay in the format's value at registration. Settings
are for choices that differ by file or folder.

## Consequences

- One convention for every per-file choice: 3007, 3019, 3025 and 2328
  each register a section and read it with one call, and an agent making
  a Playdate version learns one file.
- A typo in a settings file is found by `check_import_settings` even
  when the step that reads it doesn't run, because every section is
  registered at start-up.
- `RegisterError` grows four variants and becomes `#[non_exhaustive]`
  (ADR-0039's *When*): a game's `match` on it without a wildcard arm
  stops compiling once.
- The language readers must know of settings files. When 2238 shares
  their check of a pack's files, the skip moves there.
- A pack carries settings files it may not need; that costs bytes until
  a target's budget asks for the follow-up.
- Whole sections, not merged fields, mean a file's own section repeats
  every field it wants different from the default, not from the
  folder's. That is the price of a rule an artist reads off one file.

## Alternatives considered

- **`<file>.import.ron` or `<file>.settings.ron`** — every RON format
  would claim it, so the walkers would decode it as a palette or a
  game's table, and the language readers would refuse it.
- **One settings file per folder only** (Unity's Presets alone) — no
  way to set one picture apart without a folder for it.
- **A dotfile (`.folder.meta`)** — hidden in listings and file pickers,
  and some tools skip dotfiles; an artist wouldn't find it.
- **Merging fields across files and layers** — a field's value would
  depend on several files at once, and a reader would have to look at
  all of them to know one setting. 2228's `Bundle::versions` would allow
  it if a game asks.
- **Settings in the format's value only** (ADR-0019 §5 as it is) — every
  choice game-wide, so a whole game dithers one way.
- **Untyped sections** (a `ron::Value` each step digs into) — no typo
  found until the step runs, and each step writes its own checks.
- **Settings inside the asset's own file** — a PNG has no place for
  them, and a format would have to learn every step's options.
