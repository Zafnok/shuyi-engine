# ADR-0091: Sound groups: the player's volumes as a tree of groups on a loudness curve, worked out before any backend

- **Status:** Accepted
- **Date:** 2026-11-08
- **Related tickets:** 2342 (this), 2343 (ducking), 3030 (a mixer of our
  own, parked), and the follow-up that adds a game's own groups to the
  audio manifest

## Context

[ADR-0004](0004-assets-standard-files-in-the-engines-own-data-out-and-a-deterministic-movement-kit.md)
§6, "Audio stays as it is", foresaw positional sound and nothing else. The
player had three volumes: `Settings::music_volume`, `sound_volume` and
`voice_volume` (`u8`, 0–100, default 80), made into gains by a linear
`v / 100`, and handed to the device as three `f32`s in `FrameOutput` and
`AudioFrame`. There was no master volume, and no way for a game to give
its ambience or its menus' sounds a volume of their own without changing
`shuyi-ui` and `shuyi-platform`.

Every engine we were compared with (2026-10-20, `docs/decisions.md`) lets
a game group its sounds under one master: Godot's buses feed Master,
Unity's mixer groups sit under a master group, Unreal has Sound Classes in
a tree, GameMaker has audio groups, and Bevy has one `GlobalVolume`.

A linear slider also puts what a player hears in its top steps: from 50 to
100 the sound changes by only 6 dB, and everything quieter is crowded into
the bottom half.

The backends multiply: `shuyi-app` today, the Playdate's player (3029),
and later the GBA's and our own mixer (3027, 3030; parked). They share one
player (ADR-0063).

## Decision

1. **Groups.** Every sound, track and voice clip plays in a sound group,
   and every group but `master` has a parent. Five are built in, their ids
   `pub const`s in `shuyi_ui::audio::groups`: `master`; `music`, `sfx` and
   `voice` under it; and `ui` under `sfx`, so the sound volume keeps
   lowering the menus' sounds as it did. A sound cue plays in `ui` if it is
   one of the menus' (`AudioConfig`'s five cues), else in `sfx`; a music
   cue in `music`; every voice clip in `voice`. `SoundGroups` is the tree
   and that mapping; `Ctx::new` builds it, and `Game::with_stack` again.
   A game's own groups are data in the audio manifest (`groups`, and a
   `group` on a cue), added by the follow-up ticket: the tree is built
   from data, never from a game's code.
2. **The curve.** A volume the player sets is a position, 0 to 100, on a
   loudness curve (`shuyi_ui::audio::loudness`): 0 is silent, and each step
   from 1 to 100 is 0.6 dB (*tunable*), so 1 is −59.4 dB. A table of 101
   entries holds it, worked out again from the formula by a test. The
   built-ins start at 100 (`master`, `ui`) and 97 (`music`, `sfx`,
   `voice`): 97's gain is the nearest to the linear 0.8 a new player heard
   before, so nobody hears a change. `position_of_linear` puts an old
   linear volume where it sounds the same, for a saved form that reads its
   old fields (80 gives 97, 35 gives 85).
3. **Gains in integers.** A gain is a `Gain(u16)` in units of 1/4096 (4096
   is full), the unit of 3030's mixer. A group's gain is its position's
   loudness times its parent's gain, multiplied in `u32` and rounded, down
   from `master` (`GroupGains`, worked out by `Game` every frame). It stays
   out of `shuyi-core`: a volume isn't simulation.
4. **The player's volumes in the settings.** `Settings::volumes`
   (`Volumes`, a map from group id to position) holds only what the player
   set: a group not in it is at its default, and setting one back to its
   default takes it out, so a slider moved away and back is no change.
   `Ctx::volume` and `Ctx::preview_volume` read and set one through
   `preview_settings`; `Ctx::load_settings` drops a loaded volume at its
   default. A group the game no longer has is kept and ignored.
5. **A backend gets one gain per cue, never the tree.** `FrameOutput` and
   `AudioFrame` carry `gains: Gains<'a>`, a `Copy` view over the groups and
   their gains that answers `sound(cue)`, `music(cue)`, `voice()` and
   `group(id)`, each a `Gain`; `each()` for a device that records them; and
   `muted()`, every gain 0, for the background mute (ADR-0077). The shared
   player (ADR-0063) keeps the last frame's gains and plays each cue at its
   own volume times its gain, and moves a playing track, voice or sound
   started with a handle when its gain changes. A `Gain` becomes an `f32`
   only where the player calls a `Backend`, which takes 0–1.
6. **What comes later builds on the same gains.** Ducking (2343)
   multiplies into a group's gain. Effects on a group (filters, reverb,
   sends) wait for a mixer of our own (3030): the initial mixer promises
   none.

ADR-0004 §6 is extended by this: audio is no longer "as it is", but its
files and formats are.

## Consequences

- A game shows a master volume, or a volume for its menus' sounds, with
  one row each (`OptionRow::master_volume`, `ui_volume`,
  `OptionRow::volumes()`); a game's own group gets a row written the same
  way with its id.
- The voice volume is a group like the others, saved with the settings.
- Every step of a slider sounds like the same change.
- Every backend gets the same gains, worked out once, in integers; a
  device that mixes in integers (the GBA, our own mixer) takes them as
  they are.
- The API breaks: the three volume fields go from `Settings`,
  `FrameOutput` and `AudioFrame`, and every saved form reads its old
  fields with `Volumes::set_from_linear`. An unchanged volume shows as 97
  where it showed 80, at the same loudness.
- A frame now copies the groups' gains (five `u16`s) where it copied three
  `f32`s, and the player compares them each frame.

## Alternatives considered

- **Keep three volumes and add a master** as a fourth field: no group a
  game names, and the next request (ambience, menus) would be a fifth
  field in `shuyi-ui` and `shuyi-platform`.
- **Hand the backend the tree** and let it multiply: every backend would
  write the same rules, in floats or not, and they would drift. One gain
  per cue keeps the rules in one place (ADR-0063's reason too).
- **Floats for the gains**: what the backends take today, but not what an
  integer mixer takes, and a sum of floats isn't the same everywhere.
- **A linear curve, or a perceptual one in float maths** (`x²`, `x³`):
  the linear one crowds the quiet half; a power curve needs floats or a
  table anyway, and decibel steps are what audio tools show.
- **Default positions of 80 on the curve**: an unchanged game would sound
  quieter than before (80 on the curve is −12 dB, linear 0.8 is −1.9 dB).
