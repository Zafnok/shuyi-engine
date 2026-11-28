---
id: "2292"
title: "A text format is a TextFormat: the UTF-8 read and the limits written once, and one generic audio Track in place of three"
type: infra
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
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

- [ ] `grep -c "TEXT_MAX_BYTES" crates/assets/src` falls to the
      definition and the trait default.
- [ ] Every format's id, extensions, limits and errors are unchanged:
      the existing format tests and golden rows pass with no edits.
- [ ] An OGG, FLAC or MP3 with no length gives the same message as
      before.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: a test-only `TextFormat` through the blanket impl (bad UTF-8, an
  oversize file), and `Track<F>` refusing a sound with no length.
- The existing format tests and golden tables are the regression suite.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
