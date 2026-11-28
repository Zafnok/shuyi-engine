---
id: "2292"
title: "A text format is a TextFormat: the UTF-8 read and the limits written once, and one generic audio Track in place of three"
type: infra
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-28
---

# 2292 — A text format is a `TextFormat`, and one generic audio `Track`

## Context

The redundancy sweep of 2026-11-28 found that the `Format` impls in
`crates/assets` repeat the same lines. There are 35 real impls
(`grep -rn "impl Format for" crates/assets/src`):

- 17 return `&["ron"]` from `extensions`;
- 20 return `TEXT_MAX_BYTES` from `max_bytes`;
- 21 begin `decode` with `let text = text(file, bytes)?;`.

`NamesRon` (`crates/assets/src/names.rs`, about 37–60) is typical: 24
lines, of which only the id and the `from_source` call vary.

The three audio track formats repeat their sound twin
(`crates/assets/src/audio/formats.rs`, about 186–445): `OggVorbisTrack`,
`FlacTrack` and `Mp3Track`, about 35–38 lines each. Each forwards
`id`, `extensions`, `sniff` and `max_bytes`, wraps the sound's `decode`
and requires a length.

The saving is about 160 lines for the text formats and about 75 for the
tracks, less about 45 for the trait and the generic.

## Nick input

None.

## Scope

**In:**
- **`pub trait TextFormat`** in `crates/assets/src/format.rs`:
  - `type Asset`, `fn id(&self)`;
  - `fn extensions(&self)`, defaulting to `&["ron"]`;
  - `fn max_bytes(&self)`, defaulting to `TEXT_MAX_BYTES`;
  - `fn read(&self, file, text: &str, cx) -> Result<Self::Asset, Vec<ContentError>>`.

  These are methods, not constants, because `Ron<T>` and `Dlg<D>` set
  their id and extensions at run time. A blanket
  `impl<T: TextFormat> Format for T` does the UTF-8 check once (the
  existing `text(file, bytes)`).
- **The text formats move onto it**: names, target, lighting, palette,
  credits, sfx, keymap, cinematic, dialogue, grid map, picture set,
  parts, the lang formats and the tileset def. `OneBitInfo` and
  `OneBitFile` claim no extensions and keep their own impls.
- **A generic `Track<F: Format<Asset = Sound>>`** that forwards to `F`
  and requires `length_ms`. The message comes from a small trait
  constant, so each format keeps its own text (`NO_LENGTH`,
  `NO_FLAC_LENGTH`, `NO_MP3_LENGTH`). `OggVorbisTrack`, `FlacTrack` and
  `Mp3Track` stay as type aliases.
- **Breaking:** the impls change shape, and a track is no longer a unit
  struct value. Write `Breaking:` lines with a `migration` block (the
  `README.md` of `tickets/`). A game that only registers formats
  changes nothing.

**Out (do not do):**
- The binary formats (Wav, image, bitmap, onebit, atlas, pack): their
  sniff and decode bodies differ.
- The `Coded` reader table in `audio/decode.rs`, which is feature-gated
  on purpose. Also out are `format_problem`, `sound_length_ms` and
  `track_length_ms`, which handle different cases.
- Error plumbing: 2247.

**Estimated size:** about 700 lines changed (about 450 removed, 250
added).

## Implementation steps

1. Add `TextFormat` and the blanket impl, with a doc example.
2. Move each text format, one module at a time. The tests stay green
   without edits.
3. Write `Track<F>` and the length-message trait. Turn the three structs
   into aliases.
4. Run the golden tables (`shuyi_assets::golden`) and the API job
   locally, if needed to reproduce CI. Bless nothing: the values must
   not move.
5. Update the book's formats chapter if it shows `impl Format` for a
   text format.

## Acceptance criteria

- [x] `grep -c "TEXT_MAX_BYTES" crates/assets/src` falls to the
      definition and the trait default.
- [x] Every format's id, extensions, limits and errors are unchanged:
      the existing format tests and golden rows pass with no edits.
- [x] An OGG, FLAC or MP3 with no length gives the same message as
      before.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: a test-only `TextFormat` through the blanket impl (bad UTF-8, an
  oversize file), and `Track<F>` refusing a sound with no length.
- The existing format tests and golden tables are the regression suite.

## Completion notes

- **`format::TextFormat`** (re-exported at the crate root): `id`,
  `extensions` (default `["ron"]`), `max_bytes` (default
  `TEXT_MAX_BYTES`) and `read(file, text, cx)`. A blanket
  `impl<T: TextFormat> Format for T` runs the UTF-8 check (`text`) once,
  with a doc example and a unit test (`Spelled` in `format/tests.rs`:
  defaults, bad UTF-8 with `SH2002`, a file one byte past the limit
  refused with `SH2006` and one at the limit read, the registry).
- **19 text formats moved onto it**: names, target, lighting, palette,
  credits, the sound-effect definition (`SfxDefRon`), keymap, cinematic,
  dialogue, grid map, picture set, parts, the language pack, the tileset
  def, and also the audio manifest, the font atlas, the pattern map, the
  atlas and `Ron<T>`.
- **`audio::Track<F>(pub F)`** reads a music track through any
  `F: audio::TrackSound` (a `Format<Asset = Sound>` with a `NO_LENGTH`
  constant): it forwards id, extensions, sniff and limit, and requires
  the sound's `length_ms`. `OggVorbis`, `Flac` and `Mp3` implement
  `TrackSound` with `NO_LENGTH`, `NO_FLAC_LENGTH` and `NO_MP3_LENGTH`;
  `OggVorbisTrack`, `FlacTrack` and `Mp3Track` are type aliases. The
  FLAC and MP3 tracks used to re-read the sample count; the sound's own
  `length_ms` is the same number from the same read (`samples_ms` of the
  same samples and rate), so the private `track` helper went. Unit test:
  `Tone` in `format/tests.rs`, a track refusing a sound with no length.
- `cargo test -p shuyi-assets --all-features` passes, the golden tables
  included, with no value blessed.

Deviations:

- **Five more text formats moved** than the list in *Scope*: the audio
  manifest, the font atlas, the pattern map, the atlas and `Ron<T>`. The
  first acceptance criterion needs it, and the *Out* line's "onebit,
  atlas" meant their binary formats (`OneBitInfo`, `OneBitFile`,
  `Dithered`, the image pages); `PatternMapRon` and `AtlasRon` read RON
  text with no sniff.
- **`SfxRon` keeps its own `Format` impl**: it checks the file's name
  before the text, so as a `TextFormat` a non-UTF-8 file with a wrong
  name would get the UTF-8 error instead of the name's. Its limit is now
  `Format::max_bytes(&SfxDefRon)`, so the constant isn't repeated.
- **The `TEXT_MAX_BYTES` count** in `crates/assets/src` falls to the
  definition, the trait default, the doc comments, the two settings-file
  reads in `DecodeCx::import_settings` and `Formats` (not formats: they
  read `.meta` files) and the tests that assert the limit. No format
  impl names it.
- **Tests edited, with no value changed**: a track is no longer a unit
  value, so `&OggVorbisTrack` became `&OggVorbisTrack::default()` (and
  the same for FLAC and MP3) in the format, codec, golden and decode
  tests; four test modules import `Format` or `TEXT_MAX_BYTES`, which
  they had through their parent module's imports; and a test that calls
  `id`, `extensions` or `max_bytes` on a text format where both traits
  are in scope writes `Format::id(&PartsRon)` (both traits have the
  method, so the call is ambiguous, E0034). Only code that imports
  `TextFormat` meets that; the game imports neither trait by glob.

Searched the game for the changed items (`gh search code --repo
Zafnok/visions-of-shuyi`): `OggVorbisTrack`, `FlacTrack` and `Mp3Track`
have no hit; `TEXT_MAX_BYTES` and `impl Format for` are in
`crates/content/src/formats.rs` (`AsciiDlg`), which still compiles: the
constant and `text` stay public, and `Format` keeps its shape.

Breaking: shuyi-assets: audio::OggVorbisTrack, audio::FlacTrack, audio::Mp3Track: each is a type alias of `audio::Track<F>`, not a unit struct; a value is `OggVorbisTrack::default()` or `Track(OggVorbis)`

```migration
since: 0.1.0
what: A music track format is the generic Track of its sound format, so its name is a type alias, no longer a value.
old: audio::OggVorbisTrack (a unit struct)
new: audio::OggVorbisTrack = audio::Track<audio::OggVorbis>
before: |
  formats.register(OggVorbisTrack)?;
  check(&FlacTrack, "a.flac");
  check(&Mp3Track, "a.mp3");
after: |
  formats.register(OggVorbisTrack::default())?;
  check(&FlacTrack::default(), "a.flac");
  check(&Mp3Track::default(), "a.mp3");
find: ["register(OggVorbisTrack)", "&FlacTrack,", "&Mp3Track,"]
```

Breaking: shuyi-assets: the engine's text formats (`names::NamesRon`, `palette::PaletteRon`, `ron::Ron`, …): they implement `format::TextFormat`, and are a `Format` through its blanket impl; a call of `id`, `extensions` or `max_bytes` on one where `TextFormat` is imported too is ambiguous and is written `Format::id(&format)`

```migration
since: 0.1.0
what: The text formats implement TextFormat and get Format from it, so with both traits in scope a method both have needs its trait named.
old: impl Format for NamesRon
new: "impl TextFormat for NamesRon (and Format through `impl<T: TextFormat> Format for T`)"
before: |
  use shuyi_assets::format::{Format, TextFormat};
  assert_eq!(NamesRon.id(), "names");
after: |
  use shuyi_assets::format::{Format, TextFormat};
  assert_eq!(Format::id(&NamesRon), "names");
```

No `Behaviour:` line: every format's id, extensions, limit, errors and their order are as before; `SfxRon` keeps its order (its name, then the text).

Book: the chapter *Assets and formats* (`site/content/learn/book/assets-and-formats.md`) says a text format implements `TextFormat`; `docs/formats.md` says the same beside its walk-through, and how `Track<F>` and `TrackSound` read a game's own sound format's tracks.

Skills: none, no schema, template, file format or problem code changed: a Rust trait's shape moved, and every format reads the same files into the same values.

Game: nothing to adopt, the game names none of the track types, imports `TextFormat` nowhere, and its own `AsciiDlg` format keeps compiling.

Nick input: none.
