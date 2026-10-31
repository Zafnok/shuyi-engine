---
id: "3078"
title: "Voice clips baked for the Playdate: the export bakes a game's voice folder, and the player finds them"
type: feature
milestone: After MVP
model: implementation
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3078 — Voice clips baked for the Playdate

## Context

- **From 3029 (done):** the shared player (`shuyi_platform::audio::player`,
  ADR-0063) plays voice clips on the Playdate as file players, from `voice/`
  in the `.pdx` (`VOICE_DIR` in `crates/playdate/src/audio.rs`). 3029 bakes
  sounds and music only, as its ticket said, so today a voice is silence, with a
  warning naming the clip.
- **No friction review:** a missing feature. No game ships voices on the
  Playdate yet, so a game asking for it is the moment to start this.
- **Outside tactics:** a visual novel exported to the Playdate.

## Nick input

None.

## Scope

**In:**
- `GameAudio` (`crates/cli/src/game.rs`) gains a way to name the voice folder
  and the language it ships there, additively (ADR-0039).
- The bake (`crates/cli/src/export/playdate/audio.rs`) writes each clip of it to
  ADPCM under `voice/<lang>/`.
- `crates/playdate/src/audio.rs` looks in the language's folder, which the
  desktop's `platform_voice_dir` does: `voice/<lang>/`, from the game's language.

**Out (do not do):**
- A voice's group (2725, parked).

## Implementation steps

1. Read `crates/app/src/audio/voice.rs` for where the desktop looks, and the bake.
2. Add the voice folder to `GameAudio`, and its jobs to `audio::jobs`.
3. Give `audio::player` the language, from `Playdate::start`'s game.
4. A test on the fake SDK, and a bake test of a clip.

## Acceptance criteria

- [ ] A clip the game ships plays on a file player, against the fake SDK.
- [ ] A clip it doesn't ship is one warning and silence, as now.
- [ ] The bake puts `voice/en/scene/line.wav` where the backend opens it.
- [ ] `docs/targets/playdate.md`'s *What doesn't work yet* loses its line.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (host) against the fake SDK; a bake test.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
