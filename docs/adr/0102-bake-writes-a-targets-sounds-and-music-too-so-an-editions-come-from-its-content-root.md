# ADR-0102: `bake` writes a target's sounds and music too, so an edition's come from its content root

- **Status:** Accepted; amends ADR-0101
- **Date:** 2026-11-10
- **Related tickets:** 3306, 3304

## Context

ADR-0101 bakes a Playdate edition's pack in the game's tool program built with
the edition's content root as `SHUYI_CONTENT`. The Playdate's sounds and music
aren't in the pack: the exporter baked them beside it, in its own process, from
`GameTools::audio` of the tool program running the export, which was built from
`files/`. So a public demo's `.pdx` could hold the full game's sounds and music,
as loose files under `audio/` and `music/`.

The program built with the content root has to answer for the audio too. Two
ways were open (ticket 3306):

- (a) a command of its own in the tool program (`bake-audio --target <id>
  --out <folder> --json`), run by the exporter beside `bake`;
- (b) a field of `bake`'s input, so the one run that bakes the pack writes the
  sounds and music too.

## Decision

1. **`bake --audio <folder>`.** `BakeInput::audio` is a folder of the project,
   from its root: `bake` empties it, then writes there each sound file and
   music track the game ships on the target (`GameTools::audio`), baked as the
   target plays them, under `audio/` and `music/`. It needs a target
   (`SH0001` without one). It makes no folder when the game's tools offer no
   audio. A sound the tools name and can't read, or can't decode, answers
   `SH3004`; the tools' own error `SH3002`. The answer's `PackMade::audio_files`
   counts the files written.
2. **With a content root, the same program.** `bake --content` passes
   `--audio` through to the tool program it builds with the folder, so the
   pack and the sounds and music come from one build of one program, with one
   rule for what the folder replaces: the game's build script (ADR-0101).
3. **The Playdate takes it.** `playdate::bake_input` asks for the audio in
   `baked-audio/` of the export's folder, where the exporter took it from
   before, with or without an edition; the exporter's own audio bake is gone.
   Without a content root the bake runs in process, as it did.
4. **The form is the target's.** Today the only target that bakes its audio is
   the Playdate, so `--audio` writes its form (mono IMA ADPCM WAV,
   `export::playdate::audio`). A later target that bakes audio another way
   picks its form by the target in the same field, and its exporter asks for it
   as the Playdate's does.
5. **The cache** of baked files moves to the project's `target/shuyi/`
   (`audio-v<FORMAT>`), beside the packs `bake` writes, since `bake` doesn't
   know an exporter's build folder.

## Consequences

- A Playdate edition's sounds and music are those its content root holds: a
  sound of `files/` the folder lacks isn't in the `.pdx`.
- A game whose tools read music from the project's root (`GameAudio::files`
  with no music folder of its own, so `jobs` reads `<root>/music/`) still ships
  that folder's tracks in every edition: the folder is the project's, not the
  content root's, and the game's tools say so. A game that keeps a demo's music
  apart reads it through its own files.
- The first export after this change decodes its sounds and music once more,
  into the new cache folder.
- `bake`'s input and answer gain a field each: contract 45.

## Alternatives considered

- **(a) A command of its own:** a second run of the program for every export,
  a second command, its usage, schema and answer to keep, and the exporter
  would have to keep the two runs on the same folder and target. One run of
  `bake` already has the target, the content root and the program built.
- **The audio in the pack:** the Playdate's player opens loose `.pda` files
  that `pdc` makes, so its sounds and music can't be in the pack.
