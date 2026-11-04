---
id: "3208"
title: Voices on or off and their volume are player settings
type: feature
milestone: MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-03
---

# 3208 — Voices on or off and their volume are player settings

## Context

- **The game ticket:** game#0826
  (`github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/0826-options-voices.md`).
- **The job in one sentence:** the player turns voices off, or down, in the
  options, and that choice is saved with their other settings.
- **The friction numbers:** no friction review: a missing feature. Without it
  the game would copy its saved settings into `Ctx::voices_on` and
  `Ctx::voice_volume` every frame.
- **The generic shape:** `Settings` holds whether voice clips play and their
  volume (0–100, like music and sound), saved with the rest; `Ctx` reads
  them, turning voices off stops the clip playing, `Ctx::has_voices` says
  whether any clip may be played (so a game can ask before a new game), and
  `OptionRow::voices` and `OptionRow::voice_volume` are the options rows.
- **An example from a game that isn't tactics:** a farming game whose
  villagers speak their greetings, and a player who would rather read.

## Nick input

None.

## Scope

**In:** `Settings::voices`, `Settings::voice_volume`,
`Settings::voice_factor`; `Ctx::has_voices`; voices off stops the clip
playing; `OptionRow::voices`, `OptionRow::voice_volume`; the engine-only
saved form; `Harness::voice_volume`.

**Out (do not do):** a voice language setting; the game's own question
at a new game.

The API breaks: `Ctx::voices_on`, `Ctx::voice_volume`, `Ctx::voice_gain`,
`MAX_VOICE_VOLUME` and `DEFAULT_VOICE_VOLUME` go, and `Settings` gains two
fields. The game's PR that moves the pin follows this one.

## Implementation steps

1. `shuyi-ui` `settings.rs`: the two fields, their defaults (on, 80), the
   factor, and `EngineOnly`.
2. `ctx.rs`: read the settings; `preview_settings` stops the voice when
   voices go off; `has_voices`. `game.rs`: the frame's voice volume from
   the settings.
3. `screens/options.rs`: the two rows; the toy's text.
4. `shuyi-platform` `harness.rs`: `voice_volume`.

## Acceptance criteria

- [x] The defaults, the factor and the saved form (tests).
- [x] Turning voices off stops the voice; turning them on, or another change, doesn't (tests).
- [x] The rows set the settings (test).
- [x] The book's chapter is updated (2802).
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: settings, `Ctx`, the options rows, the frame's voice volume.

## Completion notes

`Settings` gains `voices` (on) and `voice_volume` (0–100, 80, like the
music and sound), saved by the engine-only form; `Ctx` reads them,
`preview_settings` stops the voice playing when voices go off, and
`Ctx::has_voices` says whether any clip may be played. `OptionRow::voices`
and `OptionRow::voice_volume` are the rows, `Harness::voice_volume` what a
frame told the platform. The game PR that moves the pin to this change is
game#0826's.

Deviation: the volume is 0–100 rather than the old 0–10, so the row is the
engine's volume slider like the music's and the sound's.

Breaking: shuyi-ui: Ctx::voices_on: gone; read and set `Settings::voices`.

```migration
since: 0.2.0
what: Whether voices play is a player setting, saved with the others.
old: Ctx::voices_on
new: Settings::voices
before: |
  ctx.voices_on = false;
after: |
  ctx.change_settings(|s| s.voices = false)?;
find: ["voices_on"]
```

Breaking: shuyi-ui: Ctx::voice_volume, Ctx::voice_gain, MAX_VOICE_VOLUME, DEFAULT_VOICE_VOLUME: gone; the voice volume is `Settings::voice_volume`, 0 to `MAX_VOLUME`, and `Settings::voice_factor` its 0–1 factor.

```migration
since: 0.2.0
what: The voice volume is a player setting from 0 to 100, like the music's.
old: Ctx::voice_volume (0–10), Ctx::voice_gain
new: Settings::voice_volume (0–100), Settings::voice_factor
before: |
  ctx.voice_volume = DEFAULT_VOICE_VOLUME;
  let gain = ctx.voice_gain();
after: |
  ctx.change_settings(|s| s.voice_volume = DEFAULT_VOLUME)?;
  let gain = ctx.settings().voice_factor();
find: ["voice_gain", "DEFAULT_VOICE_VOLUME"]
```

Breaking: shuyi-ui: settings::Settings: two new fields, `voices` and `voice_volume`; code that builds one field by field adds them or ends with `..Settings::default()`.

```migration
since: 0.2.0
what: Settings has the voices' two fields.
old: Settings { text_speed, fullscreen, theme, music_volume, sound_volume, frame_cap, game, .. }
new: Settings { .., voices, voice_volume }
before: |
  Settings { text_speed, fullscreen, theme, music_volume, sound_volume, frame_cap, game, .. }
after: |
  Settings { text_speed, fullscreen, theme, music_volume, sound_volume, voices: true, voice_volume: 80, frame_cap, game, .. }
```

