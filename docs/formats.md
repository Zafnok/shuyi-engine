# Formats

How Shuyi Engine reads assets: **asset kinds** are plain types, **file
formats** are plugins that read one kind from a file's bytes, and a
**registry** holds them
([ADR-0007](adr/0007-formats-are-plugins-standard-files-always-optimized-ones-optional.md),
[ADR-0019](adr/0019-the-format-registry-kinds-are-types-a-format-reads-one-file.md)).
The code is `shuyi_assets::format` (`crates/assets/src/format.rs`).

## The parts

- **A kind** is a type that implements `AssetKind`, with a name for
  messages: `ImageInfo` is `"image"`, `PaletteDef` is `"palette"`. A game
  makes one of each of its data tables.
- **A format** implements `Format`: a stable `id`, the `extensions` it
  claims, an optional `sniff` of its magic bytes, a `max_bytes` limit,
  and `decode`, which reads the bytes of one file into its kind or
  returns every problem with file and place. A format that can also
  write its kind implements `Encode`.
- **The registry**, `Formats`, holds the formats of every kind.
  `Formats::standard()` has the engine's formats that need nothing of the
  game. A game registers the rest, and its own, at start-up.
- **An asset is asked for by id**: `formats.load::<PaletteDef>(&bundle,
  "data/palette")`. The id is the file's path in the bundle, with or
  without its extension. Without one, the registry looks for the one
  file at the id and an extension the kind's formats claim
  (`images/card` is `images/card.png`, or `images/card.txtimg` once a
  format claims that). Two such files is an error. The code that asks
  never sees which format read the file.
- **The bundle** is layers searched in order, so a file in an upper layer
  replaces the one at its path below and a file only there is added
  (`shuyi_assets::bundle`). A layer is embedded in the binary, or handed
  in at run time (`Layer::owned`; `shuyi_app::read_folder` reads a folder
  on disk into one, on desktop): a player's mod laid over the game's
  files reaches the same loaders.
- **A folder's assets** come all or nothing over a tree with
  `formats.load_under::<K>(&bundle, dir)`, or file by file in one folder
  with `formats.load_each::<K>(&bundle, dir)`, each with its asset or its
  errors, for a loader that checks each file after reading it.
- **`shuyi import <file>`** reads a file through the registry as every
  kind whose formats know it, and says what it read or every error
  ([`contract.md`](contract.md)).

## The rules

- **Lookup.** For each file, the formats of the kind whose `sniff` says
  yes are tried first, in the order they were registered, then the one
  that claims the file's extension (in any case). The first that decodes
  the file gives the asset. When none does, the errors are the first
  one's. So a one-bit picture at a PNG's path is read as one, and a PNG
  named `.png` but broken says it isn't a valid PNG.
- **Conflicts.** Within one kind, two formats may not have the same id
  or claim the same extension. `register` refuses the second with a
  message naming both; `replace` puts it in place of the first and
  returns the ids it replaced. Different kinds may claim one extension:
  every RON file is `.ron`, and the kind asked for says which format reads
  it. An extension is lowercase, without the dot, and not empty.
- **Limits.** A file bigger than its format's `max_bytes`, or than the
  registry's `limits.max_bytes`, is refused before it is decoded. A game
  that loads mods or downloads lowers the registry's limit.
- **One file.** A format reads one file, and the files that one names
  through its `DecodeCx` (a sidecar's picture, a manifest's sounds, a
  pack's English). A check against another kind of asset runs after,
  on the assets loaded (below).
- **`meta` is reserved.** No format may claim the extension `meta`:
  `register` and `replace` refuse it (`RegisterError::Reserved`). It is
  the extension of import settings files (below), so no walker over a
  folder reads one as an asset.
- **Never a panic.** Bytes may come from mods or downloads. Every
  built-in format has a property test that random bytes, and its own
  fixtures with a few characters changed, never make it panic.
- **Watched.** A front end may set one watch of every decode for the
  process, `format::watch_decodes`: told the kind before each decode,
  and its file, size and result after. A debug build of `shuyi-app` tags
  what each decode allocates with its kind this way, in the heap
  counter.

## The built-in formats

The standard ones, in `Formats::standard()`:

| Kind | Type of the kind | Format | Id | Extensions | Magic bytes | Limit |
| ---- | ---------------- | ------ | -- | ---------- | ----------- | ----- |
| image | `image::ImageInfo` (its size) | `image::PngInfo` | `png` | `png` | `\x89PNG\r\n\x1a\n` | 64 MiB |
| image | `image::ImageInfo` | `bitmap::OneBitInfo` | `one-bit` | none | `1BIT` | 16 MiB |
| image pixels | `image::Image` (RGBA) | `image::Png`, with the `png` feature | `png` | `png` | `\x89PNG\r\n\x1a\n` | 64 MiB |
| sound | `audio::Sound` (its header) | `audio::Wav` | `wav` | `wav` | `RIFF`, then `WAVE` | 64 MiB |
| sound | `audio::Sound` | `audio::OggVorbis` | `ogg-vorbis` | `ogg` | `OggS` | 64 MiB |
| music track | `audio::MusicTrack` (its header and length) | `audio::OggVorbisTrack` | `ogg-vorbis` | `ogg` | `OggS` | 64 MiB |
| audio manifest | `audio::AudioManifest` | `audio::AudioManifestRon` | `audio-manifest` | `ron` | none | 4 MiB |
| credits | `credits::CreditsFile` | `credits::CreditsRon` | `credits` | `ron` | none | 4 MiB |
| font atlas | `font::FontAtlasDef` | `font::FontAtlasRon` | `font-atlas` | `ron` | none | 4 MiB |
| palette | `palette::PaletteDef` | `palette::PaletteRon` | `palette` | `ron` | none | 4 MiB |
| pattern map | `onebit::PatternMapDef` | `onebit::PatternMapRon` | `pattern-map` | `ron` | none | 4 MiB |
| lights | `lighting::Lighting` | `lighting::LightingRon` | `lights` | `ron` | none | 4 MiB |
| names table | `names::Names` | `names::NamesRon` | `names` | `ron` | none | 4 MiB |
| target profile | `target::TargetProfile` | `target::TargetRon` | `target-ron` | `ron` | none | 4 MiB |
| dialogue | `dialogue::DialogueFile` (a file's scenes) | `dialogue::Dlg` | `dlg` | `dlg` | none | 4 MiB |

The ones a game registers with its own types or rules:

| Kind | Type of the kind | Format | Id | Extensions | What the game gives |
| ---- | ---------------- | ------ | -- | ---------- | ------------------- |
| map | `grid_map::GridMap<T, F>` | `grid_map::MapText<T, F>` | `map` | `map` | its `MapFormat` and what a terrain id resolves to |
| keymap | `keymap::KeymapDef<A>` | `keymap::KeymapRon<A>` | `keymap` | `ron` | its actions |
| cinematic | `cinematic::CinematicFile<S, P>` | `cinematic::CinematicRon<S, P>` | `cinematic` | `ron` | its shots and shared lists |
| dialogue | `dialogue::DialogueFile<D>` | `dialogue::Dlg<D>` | `dlg` | `dlg` | its directives |
| language pack | `lang::LangPack` | `lang::LangPackRon` | `lang-pack` | `ron` | its limits, tokens, variants, data-text rules and data text |
| english | `lang::English`: the screen text by key, as one file (`en.ron`) holds it | `ron::Ron<English>` | `ron` | `ron` | nothing |
| its own | any type serde reads | `ron::Ron<T>` | `ron`, or its own | `ron`, or its own | the type |

Some formats are configured with the game's rules. They replace the
standard one of their kind:

- `PaletteRon::requiring(names)`: the colours the game needs;
- `FontAtlasRon::requiring(glyphs)`: the glyphs it draws;
- `LightingRon::new(light)`: the light of a map that names none (`noon`
  by default);
- `CreditsRon::new(audio_manifest, private_root)`: how its errors name
  the game's other files.

What each one reads:

- **PNG** for an image's size reads only the header (the platform decodes
  the pixels when it uploads the picture). For the pixels, with the `png`
  feature, it checks the size first, so a picture too big is refused
  before it is decoded. Either way an image is 1 to 4096 pixels a side
  (`ImageInfo::checked`).
- **A one-bit picture** in a PNG's place gives the size of the PNG it was
  made from. It claims no extension, so a `.1bit` file beside a PNG (a
  touched-up picture a build reads) is no image of its own.
- **WAV and OGG Vorbis** read the header: one or two channels at
  44.1 kHz. A music track must be a whole file, whose last Ogg page gives
  its length. `audio::format_problem`, which the manifest check uses,
  still requires a sound's content to match its extension.
- **The audio manifest** reads its sound files beside it (`audio/audio.ron`
  names `sfx/a.wav`, the bundle's `audio/sfx/a.wav`). The music folder
  isn't in the bundle, so its tracks are checked by a tool that can see
  it, with `audio::from_source`. A sound may say `looped: true`: it loops
  until the game stops it, and is started with a handle
  (`AudioQueue::start_sound`, ADR-0055). Left out, it is `false`, and
  the sound plays once.
- **The font atlas** reads the PNG beside its glyph map with the same
  name (`fonts/atlas.ron` and `fonts/atlas.png`).
- **A language pack** is read from its `lang.ron`, in a directory named
  by its code (`lang/fr/lang.ron`). With it come its `ui.ron`, checked
  against English's beside it (`lang/en/ui.ron`), and its `data.ron` and
  `dialogue/*.ron`, checked against the game's data text when the format
  has it (`LangPackRon::with_data`) and left out otherwise. Any other file
  in its directory is an error, except import settings files (below),
  which are skipped.
- **`.dlg`, `.map`, keymaps, cinematics and the RON sidecars** read what
  their parsers read (`dialogue::parse_dlg`, `grid_map::parse`,
  `KeymapDef::from_source`, `cinematic::parse`, each module's
  `from_source`), with the file named as the bundle shows it.

### The kinds of ADR-0007 §1

| Kind | Type |
| ---- | ---- |
| `Image` | `image::ImageInfo` (its size, for the image table) and `image::Image` (its pixels) |
| `Atlas`, `AnimationSet` | ticket 1123, with Aseprite and TexturePacker as formats |
| `TileMap` | `grid_map::GridMap<T, F>` for the grid kit's `.map`; Tiled's and LDtk's layered map is ticket 1124's, with both as formats |
| `Tileset` | `tileset::TerrainTiles`; how a game writes its tiles is its own format |
| `Font` | `font::FontAtlasDef` |
| `Sound`, `MusicTrack` | `audio::Sound`, `audio::MusicTrack` |
| `DialogueScene` | `dialogue::DialogueFile<D>`: the scenes of one file |
| `LanguagePack` | `lang::LangPack` |
| `Cinematic` | `cinematic::CinematicFile<S, P>`: a file as written |
| `Keymap` | `keymap::KeymapDef<A>` |

The engine's other kinds: `palette::PaletteDef`, `onebit::PatternMapDef`,
`lighting::Lighting`,
`names::Names`, `audio::AudioManifest` and `credits::CreditsFile`.

A game on the short path ([ADR-0036](adr/0036-the-short-path-ready-made-parts-beside-the-long-form.md))
reads its files with `shuyi_ui::basic::BasicData::formats()`: the standard
formats, the palette with the engine's colours, the font atlas with
`BASIC_GLYPHS`, its keymap and its English. `BasicData::load` reads
`en.ron`, `keymap.ron`, `palette.ron`, `font/atlas.ron` (with
`font/atlas.png` beside it) and every other picture.

### Checks after loading

A format reads one file. What needs other assets runs after, on the
assets loaded, with the functions that were there before the registry:

- a dialogue file's scenes against the game's characters, names and
  music, and scene ids across files: `dialogue::scripts_from_parsed`
  with the game's `SceneRules`;
- a cinematic against the music and the game's shots:
  `cinematic::check`, with the file's stem (`AssetPath::stem`);
- a voice manifest against the dialogue (`voice::from_source`), a cast
  against who speaks (`voice::cast_from_source`), tips against the
  game's placeholders and faces (`tip::from_source`), a pattern map
  against its palette (`onebit::check`, [ADR-0026](adr/0026-one-bit-by-role-a-pattern-map-beside-the-palette-colours-find-their-roles-and-the-renderer-takes-its-shades.md)), a one-bit palette
  against the base one (`PaletteDef::from_ink_source`), a theme against
  the default (`palette::theme_problems`), and the credits merged with the
  audio manifest (`credits::merge`);
- every language at once, with its directory's other rules:
  `lang::from_files`.

## A game at start-up

```rust
use shuyi_assets::{Formats, cinematic, dialogue, grid_map, keymap, lang, palette, ron};

fn formats(terrains: Terrains) -> Result<Formats, shuyi_assets::format::RegisterError> {
    let mut formats = Formats::standard();
    formats.replace(palette::PaletteRon::requiring(REQUIRED_COLOURS))?;
    formats.register(dialogue::Dlg::<Directive>::new())?;
    formats.register(keymap::KeymapRon::<Action>::new())?;
    formats.register(grid_map::MapText::<TerrainId, Feature>::new(
        MAP_FORMAT,
        move |id| terrains.id_of(id),
    ))?;
    formats.register(cinematic::CinematicRon::<Shot, Actions>::new())?;
    formats.register(lang::LangPackRon::new(LIMITS, &GameTokens).variants(VARIANTS))?;
    formats.register(ron::Ron::<ClassTable>::new())?;
    Ok(formats)
}
```

Then each loader asks the registry:

```rust
let palette = formats.load::<PaletteDef>(bundle, "data/palette")?;
let files = formats.load_under::<DialogueFile<Directive>>(bundle, "dialogue")?;
let scripts = dialogue::scripts_from_parsed(files.into_values(), &rules)?;
let images = ImageTable::load_with(bundle, &formats, &[ATLAS_PNG])?;
```

## Writing a format plugin

A format outside the engine is a crate that depends on `shuyi-assets`
and implements `Format`; the game depends on it (behind a feature if it
is optional) and registers it. There is no loading of plugin libraries
at run time.

The toy below is `*.txtimg`, a picture written as text, one character a
pixel. It is used only in tests: the whole of it, with its tests, is
[`crates/assets/tests/it/txtimg.rs`](../crates/assets/tests/it/txtimg.rs).

```text
txtimg 3 2
#.r
kgb
```

### 1. The kind

Use the engine's kind if there is one (here `image::Image`, an image's
pixels, and `image::ImageInfo`, its size). A new kind is a type with an
`AssetKind` impl:

```rust
impl AssetKind for ClassTable {
    const NAME: &'static str = "class table";
}
```

### 2. The format

```rust
struct TxtImg;

impl Format for TxtImg {
    type Asset = Image;

    fn id(&self) -> &'static str {
        "txtimg"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["txtimg"]
    }

    fn sniff(&self, bytes: &[u8]) -> bool {
        bytes.starts_with(b"txtimg ")
    }

    fn max_bytes(&self) -> usize {
        TEXT_MAX_BYTES
    }

    fn decode(
        &self,
        file: &AssetPath,
        bytes: &[u8],
        _cx: &DecodeCx<'_>,
    ) -> Result<Image, Vec<ContentError>> {
        let source = text(file, bytes)?; // "file is not valid UTF-8" otherwise
        // Read the header and the rows. Every problem is a
        // `ContentError::new(file.shown(), message).at(line, None)`, and
        // all of them are returned together.
    }
}
```

- **Errors** name `file.shown()`, the name the bundle shows the file by,
  with a line and column when there is one, and say what is wrong in
  plain words. Report every problem, not the first.
- **Never panic**: no indexing that can go out of range, no `unwrap`, no
  arithmetic that can overflow (the web build is 32-bit).
- **Limits**: set `max_bytes` to what a real file could need. Check sizes
  inside the file before allocating for them (`ImageInfo::checked` for a
  picture).
- **Files it names** come from `cx` (`cx.bytes`, `cx.text`, `cx.shown`,
  `cx.left_out`, `cx.files_under`), found from the file's own path
  (`file.sibling`, `file.with_extension`). Without a bundle they are
  missing, and the format says so.
- **Settings that differ by file or folder** come from
  `cx.import_settings::<S>(file)` (*Import settings*, below). The toy
  flips a picture left to right when its section says so:

  ```rust
  #[derive(Debug, Default, PartialEq, Eq, Deserialize)]
  #[serde(default, deny_unknown_fields)]
  struct TxtImgSettings {
      mirror: bool,
  }

  impl ImportSettings for TxtImgSettings {
      const SECTION: &'static str = "txtimg";
  }

  // In `decode`, once the picture is read:
  if cx.import_settings::<TxtImgSettings>(file)?.mirror {
      // flip each row
  }
  ```

  A choice the same for the whole game stays in the format's value
  instead (ADR-0019 §5).
- **Determinism**: the same bytes give the same asset on every platform
  (ADR-0007 §5). Its fixture is a row of its crate's golden table
  (`shuyi_assets::golden`, behind the `golden` feature): the `state_hash`
  of what it decodes from a fixed file, named `<kind>: <format id>`. The
  engine's CI checks its own tables on x64, ARM64 and 32-bit wasm32; a
  format that decodes with floats needs one most.
- **A second kind** from the same file (here the size alone, for the
  image table) is a second small format, `TxtImgInfo`, that reads through
  the first.

### 3. Writing back (optional)

```rust
impl Encode for TxtImg {
    fn encode(&self, image: &Image) -> Vec<u8> {
        // `txtimg <width> <height>`, then each row's characters.
    }
}
```

Decoding what `encode` writes gives an asset equal to the one encoded. A
property test checks it on random assets the format can hold.

### 4. Registering it

```rust
let mut formats = Formats::standard();
formats.register(TxtImg)?;
formats.register(TxtImgInfo)?;
formats.register_import_settings::<TxtImgSettings>()?;
```

From then on `formats.load::<Image>(&bundle, "images/card")` reads
`images/card.txtimg` as it read `images/card.png`, and the image table
built with `ImageTable::load_with` lists it. Registering a second format
for `.txtimg` is refused with a message naming both, unless it is
registered with `replace`.

### 5. Testing it

The toy's tests are the checklist for any format:

- a fixture of each feature decodes to the asset expected;
- each kind of broken file gives its error, with the line;
- the code that asks for the asset reads it through the registry, with
  no change to that code;
- `encode` then `decode` gives the asset back (if it encodes);
- a property test: random bytes, and text shaped like the format, never
  panic.

## Import settings

How one file, or every file of a folder, is read or baked: a picture's
dither on a one-bit target (3007), its palette group (3019). The files
are RON, beside the files they set
([ADR-0057](adr/0057-import-settings-a-meta-file-beside-a-file-and-folder-defaults-typed-sections-the-nearest-wins.md));
the code is `shuyi_assets::import_settings`.

- **The two names.** `images/hero.png.meta` holds `images/hero.png`'s
  own settings: the file's full name, then `.meta`, so `hero.png` and
  `hero.ogg` never share one. `_folder.meta` in a folder holds the
  defaults of every file in it and below it.
- **What is in one.** A struct of sections, each named after the step
  that reads it, never after a file format:

  ```ron
  // images/_folder.meta: every picture here
  (
      onebit: (dither: bayer),
      indexed: (group: "ui"),
  )
  ```

  `()` holds no section. A section given twice is an error.
- **Which wins.** For a file, the files looked at are its own `.meta`,
  then `_folder.meta` in its folder, then in each folder above it up to
  the root (`import_settings::settings_chain`). A section is taken whole
  from the first that has it: a folder's `dither` doesn't fill in a
  file's own `onebit` section that leaves `dither` out, the section's
  default does. With none, the section's `Default`.
- **Layers.** A settings file is a file: a higher layer's copy replaces
  a lower one's (`assets@playdate/images/hero.png.meta` replaces
  `assets/images/hero.png.meta`). Nearness wins over layers: a higher
  layer's `_folder.meta` doesn't override a file's own `.meta` in a
  lower layer.
- **A section is a struct** with `#[serde(default, deny_unknown_fields)]`
  that implements `ImportSettings`, named by its `SECTION`. The engine's
  sections hold integers, bools, enums and strings, never floats.
- **Registering it.** `formats.register_import_settings::<S>()`, at
  start-up, whether or not the step that reads it runs this build, so a
  typo in a Playdate setting is found on a desktop build too. It refuses
  a second section of one name, a type with a field that has no default,
  and a type that doesn't deny unknown fields, each with the attribute
  to add. The engine registers its own in `Formats::register_standard`;
  `Formats::standard()` has none yet.
- **Reading it in a format.** `cx.import_settings::<S>(file)` walks the
  chain through the bundle, so a front end that watches reads hears of
  every settings file looked for, and gives the section or its default.
  A settings file on the way that doesn't read is an error, even when a
  farther one has the section.
- **The errors** name the settings file, a line and a column: bad RON, a
  section given twice, an unknown section (with the registered ones), an
  unknown field or a bad value inside a section, and a `<name>.meta`
  whose file the bundle lacks (unless a layer leaves that file out on
  purpose) or is itself a settings file.
  `formats.check_import_settings(&bundle)` finds every one in a bundle;
  `Bake::bake` runs it and refuses a bundle with any.
- **Packs.** A settings file goes into a pack as its own bytes, like any
  file the bake doesn't name, so a file kept raw and decoded at run time
  reads the same settings from the pack as from its sources.
- **Settings are not data.** The engine's RON sidecars (font atlas,
  palette, lights, names, the audio manifest) are assets of their own,
  and a game's data stays its own kinds.

## Optimized formats

An optimized format is derived from a text source by a tool and tested
equal to it (ADR-0007 §4). The first is the pack below. JPEG, WebP, QOI,
JPEG XL, FLAC and MP3 come as plugins in ticket 2203; Yarn Spinner and
Ink in ticket 2202.

## The pack

A game may ship its files as one pack (`.shuyipak`,
[ADR-0023](adr/0023-the-pack-one-indexed-file-of-baked-assets-proven-equal-to-its-sources.md)),
made by `shuyi bake` from its sources, in place of its loose files or
under them. The code is `shuyi_assets::pack`; its reader builds without
`std`, for the Playdate.

- **What is in it.** Every file of the game's bundle, at its own path. A
  file the game bakes holds the assets read from it in their binary form
  (postcard), so loading it parses no text. Every other file (a picture
  the platform draws, a sound it plays) holds its own bytes. Each file is
  compressed with LZ4 when that makes it smaller.
- **Loading from it.** The pack is a bundle layer, and the registry finds
  a baked file by its magic bytes, so the code that asks for an asset
  doesn't change. A game registers the pack's formats once:

  ```rust
  static PACK: &[u8] = include_bytes!("../target/shuyi/game.shuyipak");

  let bake = game_bake(); // the same `Bake` its tools give `shuyi bake`
  let mut formats = formats();
  bake.register(&mut formats)?;
  let pack = Pack::open(PACK, PACK_STAMP)?;
  let bundle = Bundle::new(vec![pack.into_layer("pack")]);
  let palette = formats.load::<PaletteDef>(&bundle, "data/palette")?;
  ```

  A file the pack keeps as it is borrows the pack's bytes; only a
  compressed one is unpacked into memory of its own.
- **What a game bakes.** Its tools' `GameTools::bake(target)` gives a
  `Bake`: each asset as its loaders ask for it, `.asset::<K>(id)` or
  `.under::<K>(dir)`, and the game's **stamp** (its version, say). It also
  gives the bundle of its sources and its formats. A baked kind is
  `Serialize + DeserializeOwned + PartialEq + Debug`: every engine kind
  is, and a game's own kinds derive the same. Bake only kinds whose files
  nothing else reads raw: not a PNG the platform draws, nor a sound it
  plays, nor a picture a font atlas reads beside it.
- **Proven equal.** `shuyi check --baked` bakes the game in memory and
  reads every baked asset from the pack and from its source: they must be
  equal (`Bake::check`). Every other file must be the same bytes, and no
  file may be missing or extra. The pack's own tests do the same for one
  file of every kind of the built-in formats.
- **Refused, never migrated.** A pack of another `pack::VERSION`, or
  baked with another stamp, is refused with a message to bake it again.
  So is a pack cut short or changed: a checksum covers every byte, and
  every size is checked before anything is allocated. `pack::VERSION`
  goes up whenever the binary form of an engine kind changes; a pinned
  snapshot of every engine kind's binary form fails until it does. A
  game changes its stamp when one of its own baked kinds changes.
- **It stands for its sources.** The index holds the fingerprint of the
  bundle it was baked from (`PackInfo::content`, `Bundle::fingerprint`),
  and its layer stands for it (`Layer::standing_for`): a bundle of the
  pack alone has the loose files' fingerprint, and a loose layer laid
  over it counts as it would over them. So a record or a save stamped
  with what made it (`shuyi_core::Made`, [ADR-0051](adr/0051-stamped-records-the-engine-version-the-games-word-and-a-fingerprint-of-the-content-and-the-tools-refuse-a-mismatch.md))
  on the loose files reads as made with the same content on the pack.
- **Targets.** `shuyi bake --target <id>` takes a target with a profile
  (the engine's in `targets/`, or the game's), passes it to the game,
  which picks the layers and kinds it bakes for it, and the pack records
  it (`PackInfo::target`). Ticket 3007 bakes one-bit pictures for the
  Playdate through it.

`shuyi bake` writes `target/shuyi/game.shuyipak` (or `<target>.shuyipak`,
or `--out <file>`) and says what went in, the sizes and the times:

```text
bake: target/shuyi/game.shuyipak: 10 files, 6 baked (english, keymap, palette, dialogue, map, font atlas)
  size: sources 41,223 bytes, pack 20,631 bytes (50%)
  baked in 2.4 ms
  load: sources 0.6 ms, pack 0.3 ms
```

### Measurements

Measured on 2026-10-20 on Nick's machine (Windows 11, x86-64), in a
release build. "Web" is the same program built for
`wasm32-unknown-unknown` and run in Chromium (the desktop app's
browser). Each load is the mean of many runs. It reads every baked asset
through the registry: from the embedded loose files, or by opening the
pack (embedded, as `include_bytes!` embeds it) and reading from it.

| Files | Loose | Pack | Load, loose | Load, pack (open + read) | Pack open alone |
| ----- | ----- | ---- | ----------- | ------------------------ | --------------- |
| Hello, Grid, native | 41,223 bytes | 20,631 bytes (50%) | 0.19 ms | 0.075 ms | 0.013 ms |
| Hello, Grid, web | 41,223 bytes | 20,631 bytes (50%) | 0.22 ms | 0.055 ms | 0.010 ms |
| the game's public assets, native | 813,487 bytes | 603,462 bytes (74%) | 1.15 ms | 0.85 ms | 0.25 ms |
| the game's public assets, web | 813,487 bytes | 595,573 bytes (73%) | 1.17 ms | 0.91 ms | 0.27 ms |

- **Hello, Grid's size** is today's. Its times were measured when its
  keymap's buttons were written as numbers, 61 bytes smaller; they are
  written by their names now, as target profiles read them.
- **Hello, Grid** bakes every text file it loads: its English, keymap,
  palette, dialogue, map and font atlas. Its two PNG files stay as they are.
  It loads from its pack 2.5× faster natively and 4× faster on the web.
- **The game's public assets** are Visions of Shuyi's `assets/` folder
  (122 files) at its commit of 2026-10-20. The game hasn't adopted the
  pack, so only the kinds the engine reads without the game's own types
  were baked:
  - the palette and three themes, the names, the lights and the credits;
  - the audio manifest, the font atlas and the English screen text;
  - the dialogue, its directives read as any directive.

  That is 11 files and 109 KB of sources, all read 25% faster from the
  pack. The game's data tables (classes, items, maps, battles, …) went in
  as their own bytes; baking them needs the game's types to derive
  serde, which is the game's change.
- **Size.** Text shrinks to about a third. PNG and OGG files are compressed
  already and are kept as they are, so they set the size of a game's
  pack.
- **Opening** is the checksum, the index, and unpacking the compressed
  files. Before stored files borrowed the pack's bytes and the checksum
  read 64-bit words, opening the game's pack took 0.61 ms and ate the
  whole gain.
- **Bytes per platform.** LZ4's output differs between a 64-bit and a
  32-bit baker (603,462 against 595,573 bytes), so the pack's bytes do
  too. The assets read from it don't.
- **Not measured:** a pack's effect on a whole game's start-up, with
  every data table baked. That waits for the game to give its kinds a
  binary form and its tools a `bake`.
