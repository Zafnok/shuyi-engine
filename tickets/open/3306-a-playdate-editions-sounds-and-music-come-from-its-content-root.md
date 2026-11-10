---
id: "3306"
title: "A Playdate edition's sounds and music come from its content root"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["3304"]
nick_input: none
completed:
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

- [ ] A Playdate export of an edition whose content folder lacks a sound of
      `files/` has no `audio/` file of it.
- [ ] Without an edition the audio is baked as before.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the audio bake's input for an edition with and without a content
  root.
- Integration: a stand-in game's tool program (as 3304's test in
  `crates/cli/src/export/tests/editions.rs`) baking two content roots'
  audio, the files read back (no SDK).

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
