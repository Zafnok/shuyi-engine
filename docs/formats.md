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
- **Never a panic.** Bytes may come from mods or downloads. Every
  built-in format has a property test that random bytes, and its own
  fixtures with a few characters changed, never make it panic.

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
| lights | `lighting::Lighting` | `lighting::LightingRon` | `lights` | `ron` | none | 4 MiB |
| names table | `names::Names` | `names::NamesRon` | `names` | `ron` | none | 4 MiB |
| dialogue | `dialogue::DialogueFile` (a file's scenes) | `dialogue::Dlg` | `dlg` | `dlg` | none | 4 MiB |

The ones a game registers with its own types or rules:

| Kind | Type of the kind | Format | Id | Extensions | What the game gives |
| ---- | ---------------- | ------ | -- | ---------- | ------------------- |
| map | `grid_map::GridMap<T, F>` | `grid_map::MapText<T, F>` | `map` | `map` | its `MapFormat` and what a terrain id resolves to |
| keymap | `keymap::KeymapDef<A>` | `keymap::KeymapRon<A>` | `keymap` | `ron` | its actions |
| cinematic | `cinematic::CinematicFile<S, P>` | `cinematic::CinematicRon<S, P>` | `cinematic` | `ron` | its shots and shared lists |
| dialogue | `dialogue::DialogueFile<D>` | `dialogue::Dlg<D>` | `dlg` | `dlg` | its directives |
| language pack | `lang::LangPack` | `lang::LangPackRon` | `lang-pack` | `ron` | its limits, tokens, variants, data-text rules and data text |
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
  it, with `audio::from_source`.
- **The font atlas** reads the PNG beside its glyph map with the same
  name (`fonts/atlas.ron` and `fonts/atlas.png`).
- **A language pack** is read from its `lang.ron`, in a directory named
  by its code (`lang/fr/lang.ron`). With it come its `ui.ron`, checked
  against English's beside it (`lang/en/ui.ron`), and its `data.ron` and
  `dialogue/*.ron`, checked against the game's data text when the format
  has it (`LangPackRon::with_data`) and left out otherwise. Any other file
  in its directory is an error.
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

The engine's other kinds: `palette::PaletteDef`, `lighting::Lighting`,
`names::Names`, `audio::AudioManifest` and `credits::CreditsFile`.

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
  game's placeholders and faces (`tip::from_source`), a one-bit palette
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
- **Determinism**: the same bytes give the same asset on every platform
  (ADR-0007 §5). A format that decodes with floats checks in a fixture
  of its decoded output.
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

## Optimized formats

An optimized format is derived from a text source by a tool and tested
equal to it (ADR-0007 §4). The first is the pack of ticket 2204, which
uses `Encode` and the registry. JPEG, WebP, QOI, JPEG XL, FLAC and MP3
come as plugins in ticket 2203; Yarn Spinner and Ink in ticket 2202.
