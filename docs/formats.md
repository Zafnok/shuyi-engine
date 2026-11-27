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
- **A read hands out its bytes as a `Cow<'static, [u8]>`** (ADR-0128
  §1): `Bundle::bytes`, and `DecodeCx::bytes` for a format reading a file
  its file names. `Bundle::file`, `DecodeCx::text` and
  `Bundle::test_part`'s text are a `Cow<'static, str>`, `None` when not
  UTF-8, and `load_each` lends each file's text (`LoadFile::text`) for its
  parse. Every layer today (embedded, a table, `Layer::owned`, a pack)
  lends its bytes, `Cow::Borrowed`, so nothing is copied; a layer that
  reads its files on demand, from a pack too big to keep
  (`pack::Index::into_layer`, below), hands them out owned, `Cow::Owned`,
  for the reader to drop. Paths stay
  `&'static`: the listings, `ImageId`, `ImageTable` and the keys of
  `load_under` and `load_each`. A value a game keeps for the whole run
  (a font's bits) keeps the borrowed bytes as they are, and leaks only
  owned ones.
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
| one-bit picture | `bitmap::OneBitPicture` (a one-bit picture file, whole) | `bitmap::OneBitFile` | `one-bit` | none | `1BIT` | 16 MiB |
| one-bit picture | `bitmap::OneBitPicture` | `bitmap::dither::Dithered`, with the `png` feature | `dithered` | `png` | `\x89PNG\r\n\x1a\n` | 64 MiB |
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
| atlas | `atlas::Atlas` (pages and a table of sprites) | `atlas::AtlasRon` (an `atlas.ron`: every image of a folder, packed) | `atlas` | `ron` | none | 4 MiB |

With a feature of `shuyi-assets` on, `Formats::standard()` has its
formats too ([below](#formats-behind-a-feature)):

| Kind | Type of the kind | Format | Id | Extensions | Magic bytes | Limit | Feature |
| ---- | ---------------- | ------ | -- | ---------- | ----------- | ----- | ------- |
| image | `image::ImageInfo` | `image::JpegInfo` | `jpeg` | `jpg`, `jpeg` | `FF D8 FF` | 64 MiB | `jpeg` |
| image pixels | `image::Image` | `image::Jpeg` | `jpeg` | `jpg`, `jpeg` | `FF D8 FF` | 64 MiB | `jpeg` |
| image | `image::ImageInfo` | `image::WebPInfo` | `webp` | `webp` | `RIFF`, then `WEBP` | 64 MiB | `webp` |
| image pixels | `image::Image` | `image::WebP` | `webp` | `webp` | `RIFF`, then `WEBP` | 64 MiB | `webp` |
| image | `image::ImageInfo` | `image::QoiInfo` | `qoi` | `qoi` | `qoif` | 64 MiB | `qoi` |
| image pixels | `image::Image` | `image::Qoi` | `qoi` | `qoi` | `qoif` | 64 MiB | `qoi` |
| image | `image::ImageInfo` | `image::JpegXlInfo` | `jpeg-xl` | `jxl` | `FF 0A`, or the container's `JXL ` box | 64 MiB | `jpeg-xl` |
| image pixels | `image::Image` | `image::JpegXl` | `jpeg-xl` | `jxl` | `FF 0A`, or the container's `JXL ` box | 64 MiB | `jpeg-xl` |
| sound | `audio::Sound` | `audio::Flac` | `flac` | `flac` | `fLaC` | 64 MiB | `flac` |
| music track | `audio::MusicTrack` | `audio::FlacTrack` | `flac` | `flac` | `fLaC` | 64 MiB | `flac` |
| sound | `audio::Sound` | `audio::Mp3` | `mp3` | `mp3` | `ID3`, or a frame's sync | 64 MiB | `mp3` |
| music track | `audio::MusicTrack` | `audio::Mp3Track` | `mp3` | `mp3` | `ID3`, or a frame's sync | 64 MiB | `mp3` |
| sound | `audio::Sound`: what a `.sfx.ron` renders to, mono at 44.1 kHz | `sfx::SfxRon` | `sfx` | `ron`, a name that ends in `.sfx.ron` | none | 4 MiB | `sfx` |
| sound effect | `sfx::SfxDef`: a `.sfx.ron`'s numbers | `sfx::SfxDefRon` (and `Encode`) | `sfx` | `ron` | none | 4 MiB | `sfx` |

The ones a game registers with its own types or rules:

| Kind | Type of the kind | Format | Id | Extensions | What the game gives |
| ---- | ---------------- | ------ | -- | ---------- | ------------------- |
| map | `grid_map::GridMap<T, F>` | `grid_map::MapText<T, F>` | `map` | `map` | its `MapFormat` and what a terrain id resolves to |
| terrain tiles | `tileset::TerrainTilesDef`: a tileset's ground as written | `tileset::TerrainTilesRon` | `terrain-tiles` | `ron` | its `TileRules`: the looks its maps may name, the sides a tile may have |
| picture set | `picture_set::PictureSetDef`: an id's pictures by name, as written | `picture_set::PictureSetRon` | `picture-set` | `ron` | its `PictureRules`: the names every set has, the largest a picture may be |
| keymap | `keymap::KeymapDef<A>` | `keymap::KeymapRon<A>` | `keymap` | `ron` | its actions |
| cinematic | `cinematic::CinematicFile<S, P>` | `cinematic::CinematicRon<S, P>` | `cinematic` | `ron` | its shots and shared lists |
| dialogue | `dialogue::DialogueFile<D>` | `dialogue::Dlg<D>` | `dlg` | `dlg` | its directives |
| language pack | `lang::LangPack` | `lang::LangPackRon` | `lang-pack` | `ron` | its limits, tokens, variants, data-text rules and data text |
| english | `lang::English`: the screen text by key, as one file (`en.ron`) holds it | `ron::Ron<English>` | `ron` | `ron` | nothing |
| its own | any type serde reads | `ron::Ron<T>` | `ron`, or its own | `ron`, or its own | the type |

A format of `shuyi-ui`, not of `Formats::standard()` (its kind needs the
storage's keys): `shuyi import` reads it with the standard ones.

| Kind | Type of the kind | Format | Id | Extensions | Limit |
| ---- | ---------------- | ------ | -- | ---------- | ----- |
| save bundle | `storage::bundle::SaveBundle`: some of a storage's entries, its version (`SAVE_BUNDLE_VERSION`) and its `Made` stamp | `storage::bundle::SaveBundleRon` (and `Encode`) | `save-bundle` | `ron` | 4 MiB |

Version 2 (ticket 3312) writes each entry that is one RON value as that
value, laid out over lines with four spaces to a level, under `entries`,
and each other entry (empty, plain words, two values) as a string under
`texts`, left out when empty; a key in both is refused. Reading it back
compacts each RON entry (whitespace and comments outside its strings
dropped), so a value written with `ron::to_string` comes back byte for
byte and one a player edited as the same value. In memory,
`SaveBundle::entries` holds every entry as text, as the storage held it.
A bundle of another version, version 1 too, is refused by its version
alone, whatever the rest holds (`SaveHeader`), and so is a key that isn't
a valid storage key or starts with `_bundle-`, the bundle's own. Its
golden row is in `crates/ui/tests/golden/bundle.ron`, over the fixture
`crates/ui/tests/fixtures/bundle_v2.ron`.

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
  touched-up picture a build reads) is no image of its own. Read whole
  (`OneBitFile`), it is what a one-bit screen draws with no copying:
  `shuyi_ui`'s one-bit renderer takes a bundle's pictures as they lie.
- **A PNG dithered** (`Dithered`) is made a one-bit picture by its
  `onebit` import settings, below, in integers, so it is the same file on
  every platform.
- **WAV and OGG Vorbis** read the header: one or two channels at
  44.1 kHz. A music track must be a whole file, whose last Ogg page gives
  its length. `audio::format_problem`, which the manifest check uses,
  still requires a sound's content to match its extension: WAV for
  `.wav`, OGG Vorbis for `.ogg`, FLAC for `.flac`, MP3 for `.mp3` and a
  sound effect's numbers for `.sfx.ron` (with their features, below). A
  `.flac`, `.mp3` or `.sfx.ron` file whose bytes are a WAV (a game's hook
  decoded or rendered them) is checked as a WAV.
- **The audio manifest** reads its sound files beside it (`audio/audio.ron`
  names `sfx/a.wav`, the bundle's `audio/sfx/a.wav`). The music folder
  isn't in the bundle, so its tracks are checked by a tool that can see
  it, with `audio::from_source`. A sound may say `looped: true`: it loops
  until the game stops it, and is started with a handle
  (`AudioQueue::start_sound`, ADR-0055). Left out, it is `false`, and
  the sound plays once.
  **The cue rules** (ticket 2345, ADR-0139), each optional and flat on the
  sound: `pick` says which file each play plays (`Random`, the default,
  any file; `NoRepeat`, never the last again; `Shuffle`, each file once in
  a random order, then a new order, never the same file twice in a row
  across the seam; `InOrder`, the files in turn), and a cue of one file
  plays it every time; `volume_min` makes each play pick a whole percent
  from it to `volume` (left out, it plays at `volume`; written bare,
  `volume_min: 45`), and one over `volume` is an error at the cue's line;
  `min_gap_ms` drops a play less than that after the cue's last play that
  sounded (0, the default: none is). `App` applies them for every
  backend, in whole milliseconds, with a picker of its own, apart from the
  simulation's RNG:

  ```ron
  (id: "step", files: ["sfx/step_1.ogg", "sfx/step_2.ogg", "sfx/step_3.ogg"],
   volume: 60, credit: Own, pick: NoRepeat, volume_min: 45, min_gap_ms: 60),
  ```

  A play dropped is still asked for (the Harness's `sounds()`, `run`'s
  `sounds`), and listed with why (`Harness::dropped_sounds`). A sound started with a handle is picked the same way, and is
  never dropped. In Rust a cue takes them with `SoundCue::with_rules`
  (`CueRules::DEFAULT.with_min_gap_ms(40)`).
  **A sound's length** (ticket 2280, ADR-0139 §4) is measured, never
  written: the check keeps each file's in the cue's `lengths_ms` (read
  with `SoundCue::length_ms(file)`), from `audio::sound_length_ms`, in
  whole milliseconds, to the nearest. A WAV's (by its bytes, whatever its
  name) is its `data` chunk's frames over its rate, an OGG's from its last
  page, a FLAC's or an MP3's as its track reads, and a sound effect's the
  samples it renders to. It is `None` for a file left out of the build or
  not found, a WAV with no `data` chunk or a block align or a rate of 0,
  and an OGG with no last page. A manifest built in code measures its
  sounds with `AudioManifest::measure_sounds(read)`. The `Sound` kind has
  the same `length_ms`. The cue rules end a copy of a sound at its start
  plus its file's length (ticket 2378).
  **A loop region** (ticket 2397): a looped track may say `loop_start_ms`
  and `loop_end_ms`, both or neither, with `0 <= start < end <=
  length_ms`. It plays from its start up to `loop_end_ms` once, then
  from `loop_start_ms` to `loop_end_ms` again and again, half-open: the
  end is where it comes round. Left out, a track goes round the region
  its file names, an Ogg file's `LOOPSTART` and `LOOPLENGTH` comments
  (in samples of one channel), and round the whole track if it names
  none; the manifest's fields win over the file's. The check refuses a
  region with one end, an end not after its start, one past `length_ms`
  and one on a track that has `looped: false`, as written, never
  clamped; tags that don't read are refused as the track loads, naming
  the tag and the file, and it loops whole. A backend plays a
  millisecond `ms` at frame `ms × rate / 1000`, rounded down
  (`LoopRegion::frames`), and a file's own region at its exact frames.
  A backend that can only loop whole files (quad-snd) plays the whole
  track, and the log says so once. The pack holds the two fields since
  version 20.
- **The font atlas** reads the PNG beside its glyph map with the same
  name (`fonts/atlas.ron` and `fonts/atlas.png`). Whether it draws the
  game's text is a check across assets, once both load
  (`font::undrawn`, which `check` runs). Beside its narrow glyphs
  (`glyphs`, a cell each) it may hold wide ones (`wide`, left out when
  there are none): a Japanese or Chinese character drawn over two cells
  side by side, in one row of the picture, `wide` giving the left one.
  `font-atlas --wide <font.bdf>` makes them from a font twice as wide
  (16 × 16 beside 8 × 16). A character is narrow or wide, never both,
  but for the six marks below.
  **Wide sets.** Japanese and Chinese share a character's code point but
  not its shape (直, 骨, 角), so the wide glyphs come in sets: `wide`, the
  unnamed one, and `wide_sets`, the named ones (`{"ja": {...}, "zh-Hans":
  {...}}`, left out when there are none), each laid out as `wide` is. A
  set's name is the atlas's own word; a character may be in several
  sets. `font-atlas --wide ja=<font.bdf> --wide zh-Hans=<font.bdf>`
  makes them, one `--wide` a set, laid out one after another after the
  narrow glyphs: a set keeps the wide characters of the packs that name
  it and `shuyi.ron`'s `font_atlas.wide_set_glyphs` for it, an unnamed
  `--wide` those of the languages that name none. A wide glyph is looked
  up in the set of the language the text is in first, then in the
  unnamed set, then in the other sets by name (`FontAtlasDef::wide_cell`,
  which every renderer uses), so a pack's name for itself shows in its
  own shapes in any language.
  **Six marks narrow or wide.** `“ ” ‘ ’ … —` are narrow in English and
  wide in Chinese, with the same code points
  (`font::WIDE_PUNCTUATION`): an atlas may have each both narrow and in
  a set, and the language's pack says which it is (below).
  **Ink** (ticket 3215, ADR-0124): `ink`, left out when the atlas has no
  capital, says where its glyphs put ink in their cell: `capitals`, the
  box most capitals `A`–`Z` share the edges of, and `columns`, each
  printable ASCII glyph whose leftmost and rightmost inked columns
  differ (`'W': (0, 6)`). `font-atlas` measures it; a key cap places its
  name by it, and an atlas without it counts the whole cell as ink.
  **Bold face** (ticket 3216): `bold`, left out when empty, gives a
  narrow glyph the cell of its bold shape, and `bold_ink` that face's
  ink. `font-atlas --bold <bold.bdf>` makes them from a bold font of the
  same cell size (Terminus Bold beside Terminus), after the wide sets.
  A key cap's name is drawn from it, once, and placed by its ink; a
  glyph without a bold shape is drawn twice, 1 px apart
  (`FontAtlasDef::bold_strikes`, which every renderer uses).
  **What the cells draw:** text left to right, one character to a cell
  and two to a wide one (`font::CellWidths` says which, in the language
  the text is in, and `lang::text_width` counts with it). A cell holds one character: no
  combining marks (write `é` as one character, U+00E9), no joining and
  no reordering. Right-to-left and shaped scripts (Arabic, Hebrew, Thai,
  Devanagari) aren't drawn.
- **A language pack** is read from its `lang.ron`, in a folder named by
  its BCP 47 tag as BCP 47 writes it (`lang/fr/lang.ron`,
  `lang/pt-BR/lang.ron`, `lang/zh-Hans/lang.ron`; `lang::LangCode`):
  the language in lower case, then a script in title case, a region in
  upper case, variants and a private-use part (`en-x-pseudo`), at most
  35 characters. A folder named another way (`pt-br`) is an error naming
  the tag, so a file system that ignores case never makes it a second
  language; extensions (`-u-`) and grandfathered tags aren't taken. With
  it come its `ui.ron`, checked against the source language's beside it
  (`lang/en/ui.ron`, or the folder of the source a game declares:
  `lang::from_files_in`, `LangPackRon::source_lang`), and its `data.ron`
  and `dialogue/*.ron`, checked against the game's data text when the
  format has it (`LangPackRon::with_data`) and left out otherwise. Its
  `files/` holds its copies of the game's files, each at the game's path
  (`lang/ja/files/title/end.png` is the Japanese `title/end.png`, as
  Unreal's `L10N/<culture>/<path>`), never read as text: a picture with
  words in it, say. `lang::localized_path` finds the first copy along a
  chain that the bundle has, else the plain path, and `Ctx::localized`
  does it along the language in use, so a screen that asks each time it
  draws shows the right one at once. A target's own copy of a language's
  file sits at the same path in its layer
  (`assets@playdate/lang/ja/files/title/end.png`); without one, the
  language's copy wins over the target's plain file. Any other file in
  its folder is an error, except import settings files (below), which
  are skipped; the source's folder holds its `ui.ron` only, and no
  copies. A key a pack lacks, or has stale, shows from the pack of the
  next shorter tag the game has (`pt-BR`, then `pt`), then from the
  source (`Lang::chain`). Its `lang.ron` holds its `name`, `made_by`, and
  two fields for wide text, both optional: `wide_font`, the font atlas's
  wide set it draws from (`Some("zh-Hans")`; with the atlas, one it
  has), and `wide_punctuation`, `true` for `“ ” ‘ ’ … —` wide and drawn
  from that set, as Chinese writes them (`false` by default:
  `LangInfo::widths`). Its wide characters must be in its set:
  `font::undrawn` fails `check` on one that isn't. The source language
  has no `lang.ron`; the game says the same of it in code, where it
  declares it: `SourceLang::with_wide_font` and
  `SourceLang::with_wide_punctuation` (`SourceLang::widths`; neither by
  default). `Lang::widths` gives any language's table, a pack's or the
  source's, and the screen, `undrawn` and `font-atlas` use it, so the
  source's text is drawn, checked and kept in its own set as a pack's is. Its text is drawn as the font atlas's cells draw
  text (above): left to right, a character a cell or two for a wide one,
  so a pack in a language the atlas has no glyphs for, or in a script
  that joins or reorders its letters, fails `check` or shows wrongly.
  Its limits (a line's length, a reply's, a name's) count cells too: a
  wide character is two once the format has the atlas
  (`LangPackRon::widths_of`), or the rules have its widths
  (`PackRules::widths`, `CellWidths::of(&atlas)`), and its punctuation as
  its `lang.ron` has it, so a game loads its
  atlas before its text. Without them every character is one cell. A
  dialogue's limits (`SceneRules::widths`) and a tip's
  (`TipRules::widths`) count the same way. Text is wrapped in cells too
  (`text::word_wrap`): Latin text at spaces, text without spaces
  (Japanese) between any two characters, but never before closing
  punctuation or a small kana (`。`, `、`, `」`, `っ`, `，`, `》`) nor after
  an opening bracket (`「`, `《`); where the language has its quotes,
  ellipsis and dash wide, also never before `” ’ … —` nor after `“ ‘`, so
  `……` and `——` are never split (narrow, they wrap as English always
  has); and never at a no-break space
  (`text::NO_BREAK_SPACE`, which joins the words of a key's name filled
  into a text).
- **The source language's screen text** (`lang/en/ui.ron`, English
  unless the game declares another) is a map of key to text.
  A key's value can instead be `(text: "Start", max_cells: 12, note:
  "The title menu's first item.")`: `max_cells`, how many cells wide the
  place its text is drawn in is (at least 1), and `note`, what a
  translator should know (where it shows, what a placeholder holds),
  never shown in the game; both are optional
  (`lang::english_and_places_from_source`, `Lang::max_cells`,
  `Lang::note`). A pack's `ui.ron` entry can't set either.
  `lang-status <tag>` lists the pack's entries wider than their key's
  `max_cells` (`Lang::overflow`, a message's widest form), measured with
  the font atlas's widths; they fail no gate.
- **A text that chooses its words** is a message, a subset of ICU
  MessageFormat 1 ([ADR-0127](adr/0127-messages-choose-their-words.md),
  `shuyi_assets::message`): `{name}` as always, `{n, plural, one {# key}
  other {# keys}}` and `{n, selectordinal, one {#st} two {#nd} few {#rd}
  other {#th}}` by CLDR's categories (and `=0 {…}`), `#` the number, and
  `{device, select, playdate {Crank it} other {Turn it}}` by a value
  (a screen's text, and a tip shown through `Ctx::fill_text`, have the
  built-in `device`, the kind the player pressed last, and `target`, the
  target's id, unless the caller passes its own); every choice has an `other` branch, and `{n, number}` writes plain
  digits. An apostrophe is a letter ("l'épée"), but `''` is one and `'{'`
  writes a brace. A plural follows the rules of the language the text is
  in (`Lang::text_in`): a key a pack lacks shows English with English's
  rules. `message::plural` holds the rules of `de`, `en`, `es`, `fr`,
  `it`, `ja`, `ko`, `pl`, `pt`, `ru`, `uk` and `zh`; another language has
  only `other`. A text that isn't a message is filled as plain
  placeholders. Each screen and data text's form (plain, a message with
  its parse, not a message) is learned once, when the `Lang` is built,
  and `Ctx`'s fills read it (`Lang::form_in`, ADR-0131): no frame parses
  a text.

  The languages check their texts as messages when they load, each
  problem an error naming the file and the key. The source's screen text
  must parse (`{}`, an unclosed `{` or a comma inside braces is an
  error), and a select on `device` names only devices (`message::DEVICES`)
  and `other`. A pack's `ui.ron` and `data.ron` entry, its text and its
  `source` both parse, and the text keeps what the source has:
  - the same arguments: it may wrap the source's `{n}` in a plural, or
    drop a plural its language doesn't need, but never lose or add a
    `{name}`;
  - every branch of the source's selects, by value, so a translation
    can't drop the Playdate's wording of a `{device, select, …}`;
  - only the plural and ordinal categories of its own language
    (`message::plural::categories`: `fr` has no `few`), when it is read
    with its tag (`lang::from_files`, `LangPackRon`; not
    `pack_from_files` or `pack_from_sources`).

  A plural that lacks a category its language has fails nothing (a
  number of it shows `other`): `lang-status <tag>` lists it under
  **missing forms** (`LangStatus::missing_forms`). The `many` of French,
  Spanish, Italian and Portuguese, which only numbers of a million or
  more pick, isn't asked for, and is still accepted. A tip's text and
  `how` must parse too, and each text they can show
  (`Message::forms`) fits the box's lines and cells; `device` and
  `target` are always known placeholders there (`tip::from_source`).
- **Terrain tiles** (`terrain-tiles`, ticket 2243) are a tileset's
  ground: its `image`, `tile_px`, a tile per terrain (`terrain`), the
  `layers` painted over them and other `looks` by the name a map's look
  gives, each picture a `(column, row)` cell of `tile_px` cells. A corner
  layer may give its pictures a `size` (left out, `tile_px`), an
  `offset` and terrains to `skip`. Each look may name the pictures a map
  places on its tiles (`props`, ticket 2281): a cell and a `size` in
  pixels (left out, `tile_px`), `props: { "lamp": (at: (4, 0), size:
  (16, 32)) }`; a look's `props` are its own. The module docs of
  `tileset` have a file. The format checks the file alone (`TerrainTilesDef::problems`): a
  side of `tile_px` or a `size` outside the rules' `side_px`; a look's
  name not one of the rules' `looks`, or `outdoor` (the own look, written
  at the top); a layer whose `of` is empty; a mix not four of `#` and
  `.`, or listed twice; a corner layer with a picture for `....`, or with
  none; a picture to place with an empty name, or a side of its `size` 0
  or outside `side_px`. Each problem names its look and layer or picture
  (`look "winter": layer 2: …`, `look "winter": prop "lamp": …`).
  `TerrainTilesDef::resolve` reports a picture outside the image, a
  picture to place too.
  A corner layer's 15 pictures may also be made, not drawn: a
  `<name>.corners.ron` recipe, a tool's input that no game loads, makes
  them into a PNG from one tile (`shuyi_assets::corners`, behind its
  `corners` feature, and `shuyi-cli`'s `corners` command; ADR-0160).
- **`.dlg`, `.map`, keymaps, cinematics and the RON sidecars** read what
  their parsers read (`dialogue::parse_dlg`, `grid_map::parse`,
  `KeymapDef::from_source`, `cinematic::parse`, each module's
  `from_source`), with the file named as the bundle shows it.

### Formats behind a feature

JPEG, WebP, QOI and JPEG XL pictures, and FLAC and MP3 sounds, are
formats of `shuyi-assets` behind a cargo feature each (ticket 2203,
ADR-0007 §3), so a game pays only for what it uses. A game turns one on
in its own `Cargo.toml`, with a line for `shuyi-assets` from the same
source as its `shuyi` line (cargo then turns the feature on for every
crate of the build):

```toml
shuyi-assets = { git = "https://github.com/Zafnok/shuyi-engine", rev = "<its pin>", features = ["jpeg", "flac"] }
```

With it on, `Formats::standard()` registers its formats, so
`ImageTable::load_with` sizes the files, `Formats::load::<Image>` gives
their pixels and `Formats::load::<MusicTrack>` a track's length.

**What draws and plays them** (ticket 2234), with no feature of
`shuyi-app` or `shuyi-cli`, since the game's feature of `shuyi-assets`
reaches them:

- **Pictures.** `shuyi-app` decodes a PNG with macroquad, as it always
  has, and any other picture with the formats of `Formats::standard()`;
  `frame-png` (`frame_png::decode_image`) does the same beside its own
  PNG decoder, so it still paints what players see. A file no format
  reads is drawn as a magenta rectangle, and its error names the formats
  the build has. The font atlas stays a PNG.
- **Sounds.** The audio manifest takes `.flac` and `.mp3` sound files
  beside `.ogg` and `.wav`, and `.flac` and `.mp3` tracks beside `.ogg`,
  checking a track's `length_ms` against `audio::track_length_ms`. Every
  backend gets a FLAC or MP3 sound as a 16-bit WAV in memory:
  `audio::decode_to_wav`, which the shared player's
  `Backend::load_sound_file` calls on every target. A file that doesn't
  decode is a warning and never reaches the backend, and a WAV is never
  over `audio::MAX_DECODED_BYTES` (256 MiB, about 25 minutes of stereo).
- **Music.** On native, a FLAC or MP3 track is decoded to that WAV on
  the music's worker thread; on the web, the browser decodes the file as
  it is (Web Audio's `decodeAudioData`), off the page's thread. The
  in-house backend plays them as sounds; its streamed tracks get them
  with ticket 2275.

An MP3 starts and ends with silence its encoder added, and the
decoders cut it only when a LAME tag says how much; the browser may
not. A looped MP3 track can then have a gap at its loop: loop an OGG or
a FLAC.

What each one reads:

- **A picture** is two formats, as PNG is: one reads its size from its
  headers (`JpegInfo`, …), the other its pixels (`Jpeg`, …) as RGBA, and
  checks that size first, so a picture too big is refused before its
  pixels are decoded. A picture is 1 to 4096 pixels a side, as a PNG is.
  - **JPEG**: baseline and progressive, gray, RGB and CMYK, made opaque.
  - **WebP**: lossy or lossless, with or without alpha; an animation
    gives its first frame.
  - **QOI**: RGB or RGBA; its colour-space tag is ignored.
  - **JPEG XL**: a bare codestream or a container, lossless or lossy;
    its first frame, in sRGB. A CMYK picture, or one whose ICC profile
    needs converting, is refused.
- **A sound** reads only its header, as WAV and OGG do: one or two
  channels at 44.1 kHz.
  - **FLAC**: its `STREAMINFO`; a track's length is its sample count, so
    a file that doesn't say it (a count of 0) is a sound but no track.
  - **MP3**: its first frame's header, past an ID3 tag; a track's
    length counts every frame, without the silence its encoder added at
    both ends when its LAME tag says how much.

**The same pixels everywhere** (ADR-0007 §5). The JPEG, WebP and QOI
decoders work in integers, with no SIMD, so a picture decodes to the
same pixels on every target. `jxl-oxide` renders a lossy JPEG XL in
floats with SIMD it picks at run time (AVX2 and FMA on x64, NEON on
ARM64): the fixture's pixels are pinned on x64, ARM64 and wasm32, but
another lossy picture could round a pixel apart on another machine. A
lossless JPEG XL is exact everywhere. Each format's row in
`tests/golden/formats.ron` pins what it decodes from its fixture
(`crates/assets/tests/fixtures/formats/`, our own files, CC0): the
pixels of the lossy JPEG, WebP and JPEG XL, and of the QOI; the header
of the sounds, and the length of the tracks. Its tests also check the
lossless fixtures against their source, and the lossy ones against what
libjpeg-turbo (within 4 a channel), libwebp (exactly) and libjxl (within
1) decode.

**The decoders**, checked on 2026-11-22: each one's licence from its
`Cargo.toml` and the files in its package, and `cargo deny check` with
every feature on (ADR-0002 §8). Each is pure Rust, builds for
`wasm32-unknown-unknown` without `wasm-bindgen`, and is used with
`default-features = false`.

| Feature | Crate | Licence | Last release | Features used | Without `std` |
| ------- | ----- | ------- | ------------ | ------------- | ------------- |
| `jpeg` | `zune-jpeg` 0.5.15 (`zune-core`) | MIT OR Apache-2.0 OR Zlib | 2026-03-26 | none: no `x86` or `neon` SIMD | yes |
| `webp` | `image-webp` 0.2.4 (`byteorder-lite`, `quick-error`) | MIT OR Apache-2.0 | 2025-08-27 | none | no |
| `qoi` | `qoi` 0.4.1 (`bytemuck`) | MIT OR Apache-2.0 | 2022-10-16 | `alloc` | yes |
| `jpeg-xl` | `jxl-oxide` 0.12.6 (eleven `jxl-*` crates, `brotli-decompressor`, `tracing`) | MIT OR Apache-2.0 (`brotli-decompressor`: BSD-3-Clause OR MIT) | 2026-05-29 | none: no `rayon` | no |
| `flac` | `claxon` 0.4.3 | Apache-2.0 | 2020-08-09 | none | no |
| `mp3` | `nanomp3` 0.2.0 (`nanomp3-core`) | MIT OR Apache-2.0 | 2026-09-30 | `alloc`: no layers I and II | yes |

`symphonia`, the common Rust audio decoder, is MPL-2.0 and denied
(ADR-0004 §7). `qoi` and `claxon` have had no release in years; each
is small, its format is frozen, and `cargo deny` reports an advisory
the week one is filed. A feature whose crate needs `std` turns on
`shuyi-assets`' own `std`; the `jpeg`, `qoi` and `mp3` ones also build
without it (`thumbv7em-none-eabihf`). Pure Rust or `no_std` alone
doesn't prove a compressed track fits a handheld's time to decode it.

**What each feature adds to a web build:** the bytes of a
`wasm32-unknown-unknown` release build (the workspace's profile: thin
LTO, one codegen unit, stripped; before `wasm-opt`) of a library that
decodes every kind through `Formats::standard()`, beside the same
library with no feature on (674,911 bytes), measured on 2026-11-22.

| Feature | Bytes added | KiB |
| ------- | ----------: | --: |
| `jpeg` | 144,767 | 141 |
| `webp` | 191,661 | 187 |
| `qoi` | 10,347 | 10 |
| `jpeg-xl` | 1,668,481 | 1,629 |
| `flac` | 17,124 | 17 |
| `mp3` | 55,639 | 54 |
| all six | 2,073,029 | 2,024 |

JPEG XL costs the most by far: its decoder renders every coding tool
the format has. A game that ships to the web and has few JPEG XL files
saves more by converting them to WebP or PNG than they save it. FLAC
and MP3 read only headers here: decoding their samples
(`audio::decode_to_wav`, ticket 2234) adds to these numbers, unmeasured.

### Sound effects (`.sfx.ron`)

A sound effect is a few numbers in a text file (ticket 2225, ADR-0144),
behind the `sfx` feature of `shuyi-assets`: no dependency, and it builds
without `std`. The game renders it once, as it loads it, to a 16-bit
mono WAV at 44.1 kHz, so a new game has sounds with no audio file. A
manifest names it like any sound file:

```ron
(id: "ui_select", files: ["sfx/select.sfx.ron"], volume: 80, credit: Own),
```

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

| Field | What it is | Range | Left out |
| ----- | ---------- | ----- | -------- |
| `wave` | `Square(duty: N)` (high `N` percent of each cycle), `Triangle`, `Saw`, `Sine` or `Noise` (a new random level every half cycle) | duty 1 to 99 | needed |
| `hz` | the pitch at the start, in hertz | 20 to 12 000 | needed |
| `to_hz` | the pitch at the end: it slides in a straight line | 20 to 12 000 | `hz` |
| `attack_ms` | the loudness rises from nothing for this long | | 0 |
| `hold_ms` | then holds | | needed |
| `fade_ms` | then falls to nothing | | needed |
| | the three together, the sound's length | 1 to 5 000 ms | |
| `volume` | how loud, in percent, as a cue's volume | 0 to 100 | needed |
| `vibrato` | `(hz: N, percent: P)`: the pitch wobbles `N` times a second by `P` percent of itself | 1 to 50 each | none |
| `step` | `(at_ms: T, percent: P)`: from `T` on, the pitch is times `P` / 100 (a coin's jump up is 133) | `T` inside the length, `P` 10 to 400, the pitch it reaches 12 000 at most | none |
| `seed` | the noise's luck | any | 0 |

- **Checked.** A field the format doesn't know, or a number out of its
  range, is an error on its field's line: `cargo shuyi check` reports
  it in the manifest's problem for the sound, as `sound "ui_select":
  files/sfx/select.sfx.ron: line 3: hz 30000 is over 12000`.
- **Two kinds**, as a picture is two: `sfx::SfxDefRon` reads the numbers
  (`SfxDef`) and writes them back (`sfx::to_text`); `sfx::SfxRon` reads
  the `Sound` it renders to. Only a name that ends in `.sfx.ron` is a
  sound, so another RON file among the sounds is an error that says so.
  `shuyi import x.sfx.ron --kind "sound effect" --print` prints its text;
  without `--kind` it reads as both kinds, and says so.
- **Rendered through the sound hook.** `audio::playable(name, bytes)`
  gives a `.sfx.ron`'s WAV, and every other file as it is: `run_game`'s
  sound files go through it, and a game's own hook should too.
  `audio::decode_to_wav` renders one as well, beside FLAC and MP3, so the
  shared player plays one a hook passed raw, and the Playdate's export
  bakes it to ADPCM like any WAV.
- **The same bytes everywhere.** The render is integers only
  (`sfx::render`), so a file gives the same samples on every platform;
  the presets' WAV hashes are golden rows (`tests/golden/sfx.ron`). A
  number out of range is clamped before rendering, so nothing overflows.
- **Presets**: `sfx::PRESETS` are `blip`, `tick`, `select`, `cancel`,
  `coin`, `jump`, `hit`, `explosion`, `power_up` and `laser`, as files
  (`sfx::preset_source`). `sfx::preset(name, seed)` reads one; a seed
  other than 0 moves its pitches by up to 10 % and its `hold_ms` by up to
  20 %, so a cue's variants can be a few seeds of one preset.
  `shuyi sfx <preset> --out <file.sfx.ron>` writes one for a game to
  change, and `shuyi sfx <file> --wav <file.wav>` renders a file to
  listen to (`docs/contract.md`).
- **The feature is the game's.** A game turns `sfx` on in its own
  `Cargo.toml`, as the templates do (`shuyi-assets = { workspace = true,
  features = ["sfx"] }`). `shuyi-cli` turns it on for itself, so `cargo
  shuyi check` passes without it; only the game's own load then fails,
  with `a .sfx.ron sound needs the sfx feature of shuyi-assets`.
- **The pack** keeps a `.sfx.ron` as its own bytes, as any sound
  ([The pack](#the-pack)): it is rendered after it is read.

### Sprite atlases (`atlas.ron`)

An `atlas.ron` packs every image directly inside a folder into square
pages ([ADR-0157](adr/0157-sprite-atlas-pages-as-painted-pictures-a-skyline-packer.md)):

```ron
(from: "sprites/", page_size: 1024, padding: 1)
```

- **`from`** is the folder, from the bundle's root; folders below it are
  not read. Each image is read through the picture formats whose features
  are on (PNG with `png`), and named by its path (`sprites/coin.png`).
- **`page_size`** is a page's side, at most 4096, and at most the
  target's `display.max_texture_side` (2048 on `web`) when the game
  registers `AtlasRon::new().with_max_side(..)` for it. **`padding`**
  (0 unless said) is kept clear between sprites and from a page's edges.
- **The same images give the same atlas**, whatever order they come in
  (tallest first, then by name), so the pack holds it baked
  (`Bake::asset::<Atlas>("sprites/atlas.ron")`) and `Bake::check` proves it
  equal to the one packed at load.
- **While the game runs**, `Atlas::insert` adds a sprite in free space
  or on a new page, moving none. In the frame,
  `shuyi_ui::frame::atlas::show` puts the pages among the painted
  pictures, `insert` writes the page that changed, and `sprite` places one.

### The kinds of ADR-0007 §1

| Kind | Type |
| ---- | ---- |
| `Image` | `image::ImageInfo` (its size, for the image table) and `image::Image` (its pixels) |
| `Atlas` | `atlas::Atlas` (2222): pages and a table of name to page and rectangle; Aseprite and TexturePacker are its formats in 1123 |
| `AnimationSet` | ticket 2231's, whose Docs item names its type |
| `TileMap` | `grid_map::GridMap<T, F>` for the grid kit's `.map`; Tiled's and LDtk's layered map is ticket 1124's, with both as formats |
| `Tileset` | `tileset::TerrainTiles`, made from the `terrain-tiles` file (`tileset::TerrainTilesDef`) after loading, or read by a game's own format |
| `Font` | `font::FontAtlasDef` |
| `Sound`, `MusicTrack` | `audio::Sound`, `audio::MusicTrack`; a sound effect's numbers are `sfx::SfxDef` |
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
  against who speaks and the game's clip variants
  (`voice::cast_from_source`, with the voice tool's own settings type), tips against the
  game's placeholders and faces (`tip::from_source`), a pattern map
  against its palette (`onebit::check`, [ADR-0026](adr/0026-one-bit-by-role-a-pattern-map-beside-the-palette-colours-find-their-roles-and-the-renderer-takes-its-shades.md)), a theme against
  the default (`palette::theme_problems`), and the credits merged with the
  audio manifest (`credits::merge`);
- terrain tiles against the image table and the game's terrains, by
  name: `TerrainTilesDef::resolve(file, &images, &terrains)` makes the
  `TerrainTiles` a sprite skin paints, or reports every problem: the
  image not in the table, a picture outside it (a corner layer's at its
  `size`), a terrain named (`terrain`, `of`, `beside`, `skip`) that the
  game lacks, a terrain of the game with no tile in a look. The terrains
  are a map of name to `TerrainId`, not a function as `MapText` takes,
  since "every terrain has a tile" needs them all;
- a picture set against the image table: `PictureSetDef::resolve(file,
  dir, &images, bundle, &rules)` makes the `PictureSet` a game draws
  from, its pictures the required names first, or reports each picture
  not in the table (`SH2003`) or bigger than the rules' `max_px`
  (`SH2010`), at its line. The format checked the file alone: its `id`
  is the file's stem, and it has every required name. Whether an id may
  show a picture of a name, as `SceneRules::expression_problem` asks, is
  `picture_set::name_problem(&sets, name, without_a_set)`;
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

An engine kind that may gain fields is `#[non_exhaustive]` (ADR-0039), so
a format outside the engine builds it with its constructors, never a
literal: `Sound::mono(rate)` or `Sound::stereo(rate)` for a sound's
header, `FontAtlasDef::new(cell_w, cell_h, glyphs, columns)` for a font
atlas, then the fields it has assigned.

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
dither on a one-bit target, its palette group on an indexed one. The files
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
  to add. The engine registers its own in `Formats::register_standard`:
  `onebit`, below.
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

### The `onebit` section

How a picture is made one-bit for a black-and-white screen
(`bitmap::dither::OneBitSettings`, ADR-0008 §4). Every field may be left
out:

```ron
// images/_folder.meta
(onebit: (dither: bayer, alpha_cut: 128, invert: false))
```

- **`dither`**: `atkinson` (the default, for pictures: error diffusion,
  three quarters of each pixel's error spread to six after it), `bayer`
  (the 8 × 8 ordered dither, for flat art and anything that moves: a
  flat grey is the pattern map's own dither of it), `threshold(N)` (white
  from brightness `N` up, for line art) or `none` (the picture must
  already be black and white, and any other drawn pixel is an error
  naming the file, the pixel and its colour).
- **`alpha_cut`** (128): a pixel whose alpha is below it is see-through,
  clear in the picture's mask and white in it, and no error spreads to or
  from it; 0 draws every pixel.
- **`invert`** (`false`): the brightness turned over first.

A pixel's brightness is `(77 r + 150 g + 29 b + 128) / 256`, from 0 to
255. The picture keeps its size, one dot a pixel. `bitmap::dither::dither`
does it for a decoded picture, and `Dithered` for a PNG in a bundle.

## Hot reload

A value read through the registry can reload when its files change on disk
(ADR-0071): `shuyi_assets::hot::Hot<T>`, for any kind, with no code in its
format.

- `Hot::load(&formats, &bundle, id)` decodes the asset as `Formats::load` does,
  and keeps the paths the decode asked the bundle for through `Bundle::bytes`:
  its file, what its format read through `DecodeCx`, and its import settings,
  found or not, so a `.meta` file that appears reloads it.
- `hot.refresh(&changed)` decodes again if one of those paths is among
  `changed`, and answers `Reload::Unchanged`, `Reloaded` or `Failed(errors)`. A
  failure keeps the old value. A `Hot<T>` derefs to its `T`.
- Only a layer handed in at run time changes (`Layer::owned`,
  `shuyi_app::read_folder`), with `Bundle::replace` or `Layer::replace`. An
  embedded file stays as it is. The old bytes are leaked so every borrowed
  read (`Cow::Borrowed`) stays valid, which is why this is for debug builds.
- `bundle::recording(f)` gives the paths `f` found on its thread, for a tool
  that wants to know what a load read.

The game's half is a module: it declares `Hook::Reloaded`, keeps its `Hot`
values as its own state, refreshes them in `Module::reloaded`, and says how each
went with `Ctx::note_reload`, whose errors show over the frame. The example
Hello, Grid's `src/hot_palette.rs` is one. `shuyi run --watch` starts a game so:
the folder `shuyi.ron`'s `assets` names (`assets` until it says) over its
embedded files. A file deleted keeps its last bytes until the game restarts. It
isn't on the web or a console.

The pictures the window draws follow with no module: `shuyi-app` makes a texture
again when the font's picture or an image of the look changes. A picture that
doesn't decode keeps its old texture, and its error shows over the frame until
its next good save. A sprite's source stays in the picture's pixels from its
top-left corner: a picture that grows shows the same pixels, and a sprite whose
source is past the edge of one that shrank draws as a missing picture, magenta.
The game's `ImageTable` keeps the size it loaded, and an atlas's glyph rectangles
stay what its font file says, so a font picture is saved at its own size.
A module that reads the atlas's description again (its `atlas.ron`) changes
which characters print wide: after files change, the App hands the frame the
widths of the look's atlas (`Game::set_widths`), and draws it again if they moved.

## Optimized formats

An optimized format is derived from a text source by a tool and tested
equal to it (ADR-0007 §4). The first is the pack below; a one-bit picture
baked from a PNG is another, kept in a pack in its own format. JPEG, WebP, QOI,
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
- **In a format of its own.** `.under_as(dir, format)` bakes every file of
  `format`'s kind under `dir`, read with the game's formats, and keeps it
  in `format`'s own bytes (`Encode`), at its path, for whatever reads the
  file raw on the target. A one-bit target's game bakes its pictures so,
  `.under_as("", OneBitFile)`: each PNG goes in as a one-bit picture file
  dithered by its settings, which the image table and the one-bit
  renderer read as they are. `Bake::register` adds `format` where the
  kind lacks it, and such a file is baked as no other kind.
- **Proven equal.** `shuyi check --baked` bakes the game in memory and
  reads every baked asset from the pack and from its source: they must be
  equal (`Bake::check`). For a file in a format of its own, the pack's
  file read by that format equals the source read by the game's formats:
  a one-bit picture equals dithering its PNG again with its settings. Every other file must be the same bytes, and no
  file may be missing or extra. The pack's own tests do the same for one
  file of every kind of the built-in formats.
- **Refused, never migrated.** A pack of another `pack::VERSION`, or
  baked with another stamp, is refused with a message to bake it again.
  So is a pack cut short or changed: from version 18 the header's checksum
  covers the header and the index, and each file has a checksum of its
  own, checked when it is read; every size is checked before anything is
  allocated. LZ4 keeps only a file it makes smaller, and no file's bytes
  reach past 4 GiB. `pack::VERSION`
  goes up whenever the binary form of an engine kind changes; a pinned
  snapshot of every engine kind's binary form fails until it does. A
  game changes its stamp when one of its own baked kinds changes.
- **Laid out in any order.** The index lists the files in path order, and
  `shuyi bake` lays their bytes in that order with nothing between them,
  but for the folders laid first (below). From version 15 a reader takes
  them in any order and with bytes between them, so long as no two
  overlap and the last ends the pack. A closed build's copy (`export
  --recipient`) lays its pack out by its recipient's code
  (`pack::Raw::write_marked`, [ADR-0118](adr/0118-a-closed-builds-recipient-overlay-three-marks-a-signed-ledger-never-proof.md)),
  and the game reads the same files from it. No checksum covers the bytes
  between files: one changed changes nothing the game reads.
- **Folders laid first** ([ADR-0128](adr/0128-asset-memory-bytes-handed-out-a-pack-read-file-by-file-folders-as-groups.md)
  §3, version 19). A group of assets is a folder. `Bake::first(dir)` lays
  the files under `dir` at the front of the pack, after those of the
  folders named before it, and every other file follows in path order:
  a web game names the folders its first frame reads
  (`.first("title/")`), and the page that streams the pack in its order
  starts the game once they have arrived. `title/` and `title` are the
  same folder. A name that is a file's path (`en.ron`) lays that file
  alone, a group of one, for a game that keeps files its first frame
  reads beside the others (Hello, Grid names its seven, ticket 3318). A
  file under two named folders goes with the first, a
  folder named twice is laid where first named, and a bake names up to
  `pack::MAX_FIRST` (256). The index holds each folder's **span**: from
  the first byte of its files to the last (`Index::spans()`, each folder
  and its byte range from the pack's start), and `Index::read` refuses a
  span that doesn't hold every file under its folder, or the file it names.
  A recipient's code
  shuffles each named folder's files among their own places, and the
  rest among theirs, so each span still holds its folder; with no folder
  named, the order is the one it laid before spans. `pack::write_first`
  is the same without a `Bake`. `shuyi bake` names each span it laid
  (`PackMade::spans`).
- **Whether a folder has arrived.** `Bundle::arrived(dir)` says whether
  every file under `dir`, and the file at `dir` if there is one, can be
  read now: always for a layer read whole,
  and for a layer over a `Source`, once the folder's span has arrived, or
  for a folder not laid first, once each file under it has. It asks the
  source's `arrived(at, len)`, which a source whose bytes arrive in time
  (the web's stream) implements; one that has every byte at once keeps
  its default, `true`. A screen that opens a later part asks it first
  and shows its own wait. Like `Bundle::has`, it reads nothing.
- **Read file by file** ([ADR-0128](adr/0128-asset-memory-bytes-handed-out-a-pack-read-file-by-file-folders-as-groups.md)
  §2), for a pack too big to hold whole (the Playdate's heap, a web page
  that starts before the pack has arrived). `pack::Index::read(prefix,
  stamp)` reads the header and the index alone; given too few bytes it
  says how many it needs (`IndexError::Short { needs }`), so a backend
  reads the header's 24, then that many. `Index::check_file(path,
  stored)` checks one file's kept bytes (`Index::file_range`) and unpacks
  them. `Index::into_layer(name, source)` makes a layer over a
  `pack::Source`, which a backend implements (`read(at, into)`); it
  answers `has` and the listings from the index, and reads, checks and
  unpacks a file each time the bundle asks for it, handing it out owned
  and keeping nothing but the index (about 52 bytes a file with its
  path). A file whose bytes haven't arrived (`SourceError::NotYet`) reads
  as missing for now; one whose read fails, or whose bytes are refused,
  reads as missing too, and `Layer::problems` lists it once, for the
  backend to log. `pack::InMemory` is a source over bytes the program
  holds (an embedded pack, a test's). The one-bit frame reads a picture
  only where the bundle lends it, so a game over this layer keeps its
  one-bit pictures as values of its own (ADR-0128 §3). Desktop reads its
  pack whole, with `Pack::open`, which checks every file.
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
  it (`PackInfo::target`). For a one-bit target (the Playdate) a game
  bakes its pictures one-bit, from its target's own files where it has
  them (below). The answer names every picture dithered, for an artist to
  check, and those the target's files replaced; `--sheet <file>` draws
  each dithered picture beside its source in one PNG.

### The pack a release ships

A release export of the desktop or the web ships the game's files as a pack
beside the program, `<package>.pack`, when the game's package has a `packed`
feature ([ADR-0110](adr/0110-desktop-and-web-exports-ship-a-baked-pack-beside-the-program.md));
the Playdate's ships one in its `.pdx`. A development build (`cargo run`,
`shuyi run`) embeds the files and needs no pack.

- **The export** bakes the pack for the target's profile with the game's
  tools, proves it equal to its sources (`bake --check`), and builds the
  program with `packed`: in a `.app` the pack is in `Contents/Resources`, on
  the web beside `index.html`.
- **The game** opens it with `shuyi_app::pack_beside_program(name,
  stamp)` and lays it in its bundle (`OpenedPack::into_layer`). Desktop
  reads it whole. A pack that is missing, of another version or another
  stamp, is refused with a message to bake it again.
- **On the web the pack streams** ([ADR-0128](adr/0128-asset-memory-bytes-handed-out-a-pack-read-file-by-file-folders-as-groups.md)
  §2, §3 and §6, ticket 3318). The page fetches the `.wasm` and the pack
  together (`index.html` preloads the `.wasm`), and starts the game once
  the pack's header and index have arrived (`crates/cli/web/pack-file.js`
  reads the index's length from the header). It reads the rest as a
  stream (`response.body.getReader()`, a plain GET with no `Range`, so
  any host serves it, Python's `http.server` included; a host with no
  body stream gives the whole pack at once), in the order the bake laid
  it, and keeps each chunk only until the game takes it. The game's call
  is the same: `pack_beside_program` reads the index from the page's
  first bytes, and gives a layer over a source of the pack's whole length
  in the program's memory, one copy. Between frames `shuyi-app` copies
  the chunks the page received into it, in order. A file whose bytes
  haven't arrived reads as missing for now (`SourceError::NotYet`), so a
  load in steps that needs a folder returns `Progress::Wait` until
  `Bundle::arrived(dir)` says it has, and counts it on the card with
  `Progress::Of`; the folders its first frame reads are the ones it lays
  first (`Bake::first`). When the fetch fails or the body ends short
  after the game started, the source is failed: a file past what arrived
  reads as missing, and `Layer::problems` lists it once, and every
  folder counts as arrived, so a load waiting for one reads it and fails
  with the missing file's error instead of waiting forever. A fetch that
  fails before the index arrives is the error `pack_beside_program`
  gives (`PackFileError::Missing`, with what the page said).
- **A game on the short path** (`BasicGame`) bakes its English, palette and
  font atlas (`BasicData::bake`), with its id and version as the stamp
  (`AppInfo::pack_stamp`); its keymap, pictures and its stages' files go in
  as their own bytes. `BasicData::formats` reads either form.

Measured on 2026-11-11 on Nick's machine (Windows 11, x86-64), Hello, Grid's
release builds, embedded against packed. Load is reading every baked asset
through the registry (`shuyi bake`'s own timing, the fastest of five runs, in
a release build of the game's tool program): from the embedded files, or by
opening the pack and reading from it. The pack read from disk or fetched by
the page isn't in it.

| Hello, Grid | Embedded | Packed |
| ----------- | -------- | ------ |
| Windows program (stripped) | 4,935,168 bytes | 4,912,640 bytes, and the pack, 22,342 bytes |
| Windows package (zip) | 2,183,336 bytes | 2,192,856 bytes |
| Web program (stripped, before `wasm-opt`) | 3,436,862 bytes | 3,413,053 bytes, and the pack, 22,338 bytes |
| Load, native | 0.180 ms | 0.078 ms (2.3× faster) |

- **The program** is smaller by about the pack's size: the 45,342 bytes of
  sources were in it as text, and the pack holds them in half that.
- **The zip** is 9,520 bytes bigger: deflate shrinks text more than it
  shrinks the pack's LZ4 and postcard bytes. A game whose files are mostly
  pictures and sounds, which neither compresses again, would not see this.
- **The web's load time** wasn't measured again. The pack's open and read are
  the same code as in 2204's measurement above, which found the web 4×
  faster from a pack.

### A target's own files

A folder `assets@<target>/` beside `assets/` holds files that target
uses in place of the game's (ADR-0008 §4): a picture drawn by hand where
dithering spoils it, a settings file. It is a layer laid right above its
own, used only when baking or running for that target:

```rust
static ASSETS: Embedded = shuyi::files!("$CARGO_MANIFEST_DIR/assets");
static PLAYDATE: Embedded = shuyi::files!("$CARGO_MANIFEST_DIR/assets@playdate");

let targets = [Layer::dir("assets@playdate", PLAYDATE)];
// `target` is what `GameTools::bake` gets, or `ctx.platform.target`.
let bundle = Bundle::for_target(vec![Layer::dir("assets", ASSETS)], &targets, target);
```

Any file may be replaced so, a settings file too
(`assets@playdate/images/hero.png.meta` replaces the base one), and a
language's copy of a file (`assets@playdate/lang/ja/files/title/end.png`)
is a plain path like any other. Another target's layer is left out, and
`Layer::target` and `Bundle::layer_of` tell where a file was read from.

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
