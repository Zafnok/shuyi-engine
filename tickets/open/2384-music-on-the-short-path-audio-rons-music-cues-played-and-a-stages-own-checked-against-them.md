---
id: "2384"
title: "Music on the short path: audio.ron's music cues played, the music clock running, and a stage's own part checked against them"
type: feature
milestone: After MVP
model: implementation
effort: high
status: todo
blocked_by: ["2225"]
nick_input: none
completed:
---

# 2384 — Music on the short path

## Context

Found by the skill pack's authors (1134), writing the worked examples of
`shuyi-dialogue` and `shuyi-cinematic`. A game on the short path
(ADR-0036, `BasicData`) has no music:

- `BasicData::load` (`crates/ui/src/basic.rs`) gives the game
  `AudioManifest::default()`, and `run_game` (`crates/app/src/lib.rs`)
  plays with `no_sound` files. So a `.dlg` scene's `@music` line, a
  cinematic whose length is a music cue (`Length::Music(cue)`), and a
  screen that keeps time with the music (`Ctx::music_clock`) work only
  in a game that writes its own `GameData` with its manifest (Hello,
  Grid: `sound::manifest()`, `GameData::audio_manifest`).
- A stage's `own` function gets the game's `Bundle` alone. So even with
  a manifest in its files, it can't check a cinematic against its music
  cues (`cinematic::check` takes `&AudioManifest`) or a scene's `@music`
  lines (`SceneRules::music_problem`). The cinematic skill's example
  checks against an empty manifest.

2225 makes `BasicData::load` read `audio.ron` when the game's files have
it, and `run_game`'s sound files read the installed bundle, for its
sound effects. Its *Out* leaves music aside. This ticket builds on it
for music.

## Nick input

None.

## Scope

**In:**
- **Music cues play on the short path.** With 2225's `audio.ron` read,
  its `music` cues (OGG or WAV files under the manifest's music folder,
  `files/music/`) play through `run_game` as a long-form game's do. If
  2225 already makes them play, this ticket's part is the tests and the
  docs. If it doesn't, change what is missing in `run_game`'s `Files` or
  `BasicData`.
- **The music clock runs** in the Harness for a short-path game whose
  screen asks for a cue (`Harness::music_clock`, `Ctx::music_clock`). A
  test proves it with a cue of a short WAV in a test bundle.
- **A stage's own part can read the manifest**: `shuyi_ui::basic::audio_of(bundle:
  &Bundle) -> Result<AudioManifest, Vec<ContentError>>`, the same read
  `BasicData::load` does (2225's), so `own` checks a cinematic's
  `Music(cue)` and a scene's `@music` against the game's cues. One
  function, used by both, so they never differ.
- **The skills:**
  - `skills/shuyi-cinematic/SKILL.md`, *Music*: a short-path game names
    its music in `audio.ron`, a cinematic's length may be `Music(cue)`,
    `own` checks it with `audio_of`, and a screen keeps time with
    `ctx.music_clock`. Its worked example adds a short music file and a
    music-timed cinematic only if the file can be made in the example
    (a few seconds of WAV written by the example from `example/`), and
    otherwise keeps `Seconds`;
  - `skills/shuyi-dialogue/SKILL.md`: `@music` lines play on the short
    path, and `own` checks them against `audio.ron`;
  - `skills/shuyi-animate/SKILL.md` names no music today. Check that it
    still says nothing wrong about it;
  - `check-skills` passes, and the examples pass in CI's `skills` job.
- **Either order with 2383:** if 2383 is done, its talk screen hands a
  scene's `@music` lines to the game's music as the scene reaches them
  (`DialoguePlayer::take_music`). If not, nothing to do (2383 does it
  once this is done, by its own clause).

**Out (do not do):**
- MIDI, sequences and tracks (2205, 2206, parked) and a beat clock
  (2365, parked);
- music in the minimal template's files: a new game stays silent until
  it adds music (2225 gives it its menu sounds);
- crossfades (2341) and ducking (2343), which work on any game's music
  once it plays.

## Implementation steps

1. Read 2225's completion notes, `crates/ui/src/basic.rs`,
   `crates/app/src/lib.rs`'s `run_game` and its `Files`, and how Hello,
   Grid gives its manifest.
2. Add `audio_of`, have `BasicData::load` use it, and write the tests
   below.
3. Make music play on the short path if 2225 didn't.
4. Update the skills and their worked examples, and run them.
5. Run the gates.

## Acceptance criteria

- [ ] A short-path game with a music cue in `audio.ron` plays it, and
      its `music_clock` runs in the Harness (a test).
- [ ] A short-path stage's `own` checks a cinematic's `Music(cue)`
      against `audio.ron` with `audio_of` (a test with a cue the
      manifest lacks: its error names the cue).
- [ ] The skills say how a short-path game uses music, and
      `check-skills` passes.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`crates/ui/src/basic/tests.rs`): `audio_of` on a bundle with and
  without `audio.ron`; `BasicData::load` and `audio_of` give the same
  manifest.
- Integration (`crates/platform`'s Harness tests): a short-path test game
  asks for a cue, and `music_clock` advances with `wait`.
- Integration: the skills' worked examples in CI's `skills` job.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
