---
id: "2201"
title: "Formats are plugins: asset kinds, the Format trait, the registry, and today's loaders as built-in formats"
type: feature
milestone: MVP
model: opus-5.5 / gpt-6.1-sol
effort: xhigh
status: done
blocked_by: ["2002", "game#1131"]
nick_input: none
completed: 2026-10-19
---

# 2201 — Formats are plugins: the registry

## Context

ADR-0007 asks for three things. Asset kinds are separate from file
formats. A format is a plugin (`Format`, registered in `Formats`).
Standard formats always load, and optimized ones are derived from text
sources and tested equal.

Nick asked for this so that "if we want to introduce a highly optimized
file format, this should be easier than ever. But also maintain broad
compatibility with normal, widely used file formats"
(`docs/decisions.md`).

After the carve (game 1103–1105) and the move (game 1131), every loader
in `shuyi-assets` is a pure parser (`from_source(file, text)`) plus a
`load()` that reads the bundle. This ticket turns those parsers into
`Format`s without changing what any of them accepts.

## Nick input

None. Nothing a player sees changes.

## Scope

**In:**
- In `shuyi-assets`:
  - the asset kinds listed in ADR-0007 §1, as plain types (most already
    exist after the carve; this names them);
  - `trait Format` and `trait Encode` as ADR-0007 §2 shows;
  - `Formats` registration with the conflict rule;
  - lookup by sniffing, then by extension;
  - `AssetPath`;
  - `DecodeCx` (limits, the bundle for references).
- The built-in formats: PNG (header, size, pixels as the frame needs
  them), the RON sidecars, `.dlg`, `.map`, language packs, cinematics,
  keymaps, WAV and OGG headers.
- Each built-in format has:
  - its existing tests, moved;
  - a fuzz-style property test (random bytes never panic);
  - a size limit.
- A game registers its own kinds and formats at start-up. The game crates
  (in the game repository) change only their `load()` calls, in a game
  PR that moves the pin (game ticket to be written by this one's PR).
- `docs/formats.md` covers how to write a format plugin, with a toy
  example (`*.txtimg`: a text image format used only in tests).

**Out (do not do):**
- new formats (2202, 2203);
- the optimized pack (2204);
- importers that 1123 and 1124 add (they will be written as `Format`s
  when unparked; update those tickets' text to say so).

## Implementation steps

1. Write the traits, the registry and its tests (conflicts, order, sniff
   before extension).
2. Wrap each existing parser as a `Format`. Every existing test passes
   unchanged.
3. Add the fuzz-style tests and the limits.
4. Write the docs page and the toy plugin. Update 1123 and 1124.
5. Write the game-side ticket in the game repository (`write-ticket`
   there): register the game's kinds, move the pin.

## Acceptance criteria

- [x] A test registers the toy `txtimg` format and loads an image through
      it with no change to the code that asks for the image.
- [x] Registering two formats for one extension fails with a message
      naming both, unless the second replaces the first.
- [x] No existing test changes behavior. The game, built against this
      revision, passes its gates (checked in the game PR).
- [x] Random bytes never panic any built-in format (property tests).
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the registry.
- Property: decoders on random bytes.
- Integration: the toy plugin.

## Completion notes

**What was done**

- `shuyi_assets::format` (`crates/assets/src/format.rs`): `AssetKind`,
  `Format` and `Encode` as ADR-0007 §2 shows (plus `max_bytes`),
  `AssetPath`, `DecodeCx` and `Limits`, and `Formats`.
  - `register` refuses a second format with one id or extension in a
    kind, naming both; `replace` takes the first one's place.
  - `decode` follows the lookup rule; `find`, `load` and `load_under`
    ask for assets by id.
  - `register_standard` and `standard` hold the formats that need
    nothing of the game.
  - Beside them: `Bundle::has`, `ImageTable::load_with` and
    `dialogue::scripts_from_parsed`.
- The kinds of ADR-0007 §1 named in `docs/formats.md`. New types:
  `image::Image` (pixels), `audio::Sound`, `audio::MusicTrack`,
  `dialogue::DialogueFile`.
- The built-in formats:
  - standard: `PngInfo`, `OneBitInfo`, `Png` (`png` feature), `Wav`,
    `OggVorbis`, `OggVorbisTrack`, `AudioManifestRon`, `CreditsRon`,
    `FontAtlasRon`, `PaletteRon`, `LightingRon`, `NamesRon`, `Dlg`;
  - registered by a game with its types: `MapText`, `KeymapRon`,
    `CinematicRon`, `LangPackRon`, and `Ron<T>` for its tables.
- Each built-in has its id, extensions, magic and limit tested. A test
  checks that it reads what its parser reads, the same asset and the
  same errors. A property test checks that no bytes panic it: random
  bytes, text shaped like it, and its fixtures with up to three
  characters changed (`format/builtin_tests.rs`).
- The registry's unit tests (`format/tests.rs`), and the toy plugin
  `*.txtimg` as an integration test (`tests/it/txtimg.rs`).
- [ADR-0019](../../docs/adr/0019-the-format-registry-kinds-are-types-a-format-reads-one-file.md)
  records the registry's rules, and `docs/formats.md` says how to write
  a format plugin. 1123 and 1124 now write their importers as formats.
- The game's half: game ticket 1173, filed by
  `Zafnok/visions-of-shuyi#364`. Its own PR will move the pin to this
  change. Game ticket 1169 got a line on the order of the two.

**Deviations**

- **"Its existing tests, moved":** the parsers keep their functions, and
  their tests stay where they are, unchanged. Each format's own test
  checks it reads what the parser reads, so the old tests cover the
  formats' logic. Moving them would have changed every existing test,
  which step 2 rules out.
- **A format reads one file** (ADR-0019 §5). These checks stay functions
  that run after loading, because each needs another loaded asset:
  - a scene against the game's rules;
  - a cinematic against the music;
  - voice manifests, casts, tips and one-bit palettes, which have no
    format.
- **An image is two kinds:** its size (`ImageInfo`, what the image table
  reads) and its pixels (`Image`). PNG pixels need the `png` feature;
  the platform decodes them itself, as before.
- **The language-pack format** reads one pack from its `lang.ron`.
  Checking every language at once stays `lang::from_files`.
- **`Atlas`, `AnimationSet` and the layered `TileMap`** are 1123's and
  1124's, now written as kinds with formats there.

**Found on the way:** `Bitmap::parse` multiplied a picture's planes
unchecked, which overflows `usize` on a 32-bit target (the web) for a
crafted header. The arithmetic is now checked. Found reading the code
while writing the property tests; on 64-bit, where the tests run, it
can't overflow.

**For Nick:** nothing to do; nothing a player sees changes.
