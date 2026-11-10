---
id: "3306"
title: "A Playdate edition's sounds and music come from its content root"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: done
blocked_by: ["3304"]
nick_input: none
completed: 2026-11-10
---

# 3306 — A Playdate edition's sounds and music come from its content root

## Context

3304 ([ADR-0101](../../docs/adr/0101-an-editions-pack-is-baked-by-the-game-s-tool-program-built-with-its-content-root.md))
bakes a Playdate edition's pack by the game's tool program built with the
edition's content root as `SHUYI_CONTENT`, so the pack holds only the
edition's files. The Playdate's sounds and music are not in the pack: they are
baked beside it from `GameTools::audio` (`crates/cli/src/export/playdate.rs`,
`bake_audio`), in the tool program running the export, built from `files/`. So
a public demo's `.pdx` can still hold the full game's sounds and music, as
loose `audio/` and `music/` files. Its disclosure part's canaries and
`allowed_files` find them; nothing keeps them out.

## Nick input

None.

## Scope

**In:**
- The Playdate export of an edition with a `content` root bakes its sounds
  and music from the tool program built with that folder, as its pack is: the
  audio manifest and the files read are the edition's.
- The way the audio is asked of that program (a command of the tool program,
  or the bake's answer carrying it) is one a later export of another target
  that bakes audio can use too.
- `docs/targets/playdate.md`'s paragraph on `--edition` says so.

**Out (do not do):**
- Changing the audio bake's formats or `GameAudio`'s shape beyond what the
  way chosen needs (a new field follows ADR-0039).
- Music read from the project's root (`GameAudio::files` with no music
  folder): if the folder is the project's, not the content root's, say so in
  the ADR and leave it.

## Implementation steps

1. Read `bake_audio`, `audio::jobs`, `GameTools::audio` and
   `contract::bake::bake_content` (3304's subprocess bake).
2. Choose how the tool program built with `SHUYI_CONTENT` hands over the
   audio: a tool-program command that bakes it into a folder (as `bake`
   writes the pack), in `CONTENT_PROFILE`, with `--json`. Write an ADR
   amending ADR-0101.
3. In `PlaydateExporter::prepare`, take the edition's content root through it;
   without one, bake in process as now.
4. Update `docs/targets/playdate.md` and the `shuyi-ship` skill's Playdate
   note.

## Acceptance criteria

- [x] A Playdate export of an edition whose content folder lacks a sound of
      `files/` has no `audio/` file of it.
- [x] Without an edition the audio is baked as before.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the audio bake's input for an edition with and without a content
  root.
- Integration: a stand-in game's tool program (as 3304's test in
  `crates/cli/src/export/tests/editions.rs`) baking two content roots'
  audio, the files read back (no SDK).

## Completion notes

- `bake --audio <folder>` (`BakeInput::audio`) empties the folder and writes
  the sounds and music the game ships on the target there, baked as the
  Playdate plays them; `PackMade::audio_files` counts them. With `--content`
  it is passed through to the tool program built with the folder, so one run
  of one program bakes the pack, the sounds and the music.
  [ADR-0102](../../docs/adr/0102-bake-writes-a-targets-sounds-and-music-too-so-an-editions-come-from-its-content-root.md)
  amends ADR-0101. Contract 45.
- The Playdate exporter's own audio bake is gone: `playdate::bake_input` asks
  for the audio in `baked-audio/`, where the export took it from before, with
  or without an edition. Without a content root the bake runs in process, as
  it did; the files it writes are the same.
- Deviation: the way chosen is a field of `bake` (the ticket's "the bake's
  answer carrying it"), not a command of its own. One run already has the
  target, the content root and the program built; the ADR says why.
- The cache of baked sounds moved from the exporter's build folder to the
  project's `target/shuyi/audio-v1`, since `bake` doesn't know that folder. The
  first export after this decodes its sounds once more.
- Music a game's tools read from the project's own `music/` folder (no music
  reader in `GameAudio::files`) is that folder's in every edition, as the
  ticket's Out says: the ADR and `docs/targets/playdate.md` say so.
- Tests: the in-process audio bake (folder emptied first, files written,
  cache, no audio, a file that isn't a sound, no target), `bake_input` and the
  `--audio` passed to the content program (unit), and the stand-in game's tool
  program baking the demo's and the full game's sounds, read back (integration).
- Docs: `docs/contract.md`, `docs/targets/playdate.md`, the `shuyi-ship` skill
  and the book's *Targets and export* chapter.
- No follow-up tickets.

No break: `BakeInput` and `PackMade` are `#[non_exhaustive]`; the new fields
are additions.
