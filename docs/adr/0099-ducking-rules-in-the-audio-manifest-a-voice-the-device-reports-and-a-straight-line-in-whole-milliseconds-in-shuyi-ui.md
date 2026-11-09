# ADR-0099: Ducking: rules in the audio manifest, a voice the device reports, and a straight line in whole milliseconds in shuyi-ui

- **Status:** Accepted; extends ADR-0091 (ducking multiplies into its
  gains)
- **Date:** 2026-11-09
- **Related tickets:** 2343 (this), 2342 and 2258 (the groups), 2255 (the
  device says which voice clip plays), 2725 (a group per voiced
  character, parked), 3030 (a mixer of our own, parked), game#0865 (the
  music dips under a voiced line)

## Context

When a character speaks over music, games lower the music so the words
are heard, and bring it back after. Unity does it with a Duck Volume
effect on a mixer group, Godot with a Compressor on a bus with another
bus as its sidechain, Unreal with Audio Modulation; Bevy has nothing.

Shuyi has sound groups and their gains, worked out in integers in
`shuyi-ui` every frame and handed to every backend as one gain per cue
(ADR-0091). The device already says which voice clip plays
(`Audio::voice_playing`, ticket 2255), and `App` hands that to
`Game::set_voice_playing` before every frame, so a screen reads it as
`Ctx::voice_playing`. No device says when any other sound ends.

A real sidechain follows the voice's loudness, which needs the samples:
a DSP stage over a mixer of our own (3030, parked), not what the
backends have today.

## Decision

1. **Rules are data in the audio manifest.** An optional `ducks` list of
   `DuckRule`s, `(when: "voice", lower: "music", to: 80, attack_ms: 100,
   release_ms: 500)`. A game turns ducking on, and tunes it, without code,
   and a manifest without the list sounds as before.
2. **Only a voice lowers.** `when` must be `voice`: it is the only group a
   device says is playing. `lower` is a group the manifest knows (built
   in or declared), never `when` or one of its parents, since lowering
   `master` would lower the voice too. Two rules lowering the same group
   are refused: with one `when`, they would always run together, and the
   ticket that allows another `when` decides how two combine. A rule has
   no id, so an error names it by its place and its `lower`
   (`ducks[1] (lower: "music")`) at the first line with that `lower`.
3. **How far down is a position on the loudness curve.** While ducked,
   the lowered group's gain is multiplied by `loudness(to)`: 80 is 12 dB
   down, 100 nothing. The same curve as the player's volumes, so a
   writer reads both the same way.
4. **A straight line in whole milliseconds, in `shuyi-ui`.**
   `shuyi_ui::audio::DuckState` keeps, per rule, the multiplier now,
   where it set off from, where it is going and the whole milliseconds
   since. The multiplier is a pure function of those: the full distance
   (full minus `loudness(to)`) over `attack_ms` going down and over
   `release_ms` coming back, rounded, never past its end. So the frames'
   sizes add no rounding, a turn halfway down is back in half the
   release, and a length of 0 is at once. Integers only, `no_std`: the
   same on every backend and in every test. `Game` builds it from the
   manifest in `with_stack` and steps it in `collect_audio` by the
   frame's `dt` in whole milliseconds.
5. **A voice sounds when the device says so and the player hears it.**
   The step's `sounding` is `Ctx::voice_playing` (the device's answer,
   ticket 2255) and the `voice` group's gain above 0: a voice the player
   turned down to 0 lowers nothing. No rule lowers `voice` or its
   parents, so that gain never depends on the duck.
6. **Into ADR-0091's gains.** `GroupGains::update_ducked` multiplies each
   lowered group's own gain by its multiplier (in `u32`, rounded, as
   ADR-0091 §3 multiplies), before its children multiply by it, so they
   go down with it. The frame's `Gains` carry it: no backend changes, and
   a track already playing follows it as it follows the player's volume.

## Consequences

- One line of a game's manifest dips the music under its voiced lines,
  on every backend that says when a voice plays.
- A backend that can't tell when a voice plays never ducks.
- A test hears the same ducking every run: the rules are data and the
  step a function of whole milliseconds.
- The manifest's binary form changes: the pack format's version is the
  next number.
- `AudioManifest` gains a public field; it is `#[non_exhaustive]`, so a
  game builds it with `default()` already and nothing breaks.

## Alternatives considered

- **A sidechain compressor following the voice's loudness** (Godot's
  way): it needs the samples, a DSP stage over a mixer of our own (3030).
  Ducking by the voice's start and end is what the backends can do now.
- **A `duck` call screens make**: every game would write when to call it,
  and forget a path. The rule is the same for every line, so it is data.
- **Ducking in the backend**: each backend would write the same rules in
  its own numbers, and they would drift (ADR-0091's reason for one gain
  per cue).
- **A step per frame** (move by speed × `dt` from where it is): each
  frame rounds, so 60 frames and 30 land in different places; the pure
  function of the time doesn't.
- **Decibels for `to`**: a second scale beside the player's volumes,
  where the curve already is one.
- **Ease curves for attack and release**: a straight line is what the
  engines we compared default to; 2109's curves can follow if a game
  asks.
