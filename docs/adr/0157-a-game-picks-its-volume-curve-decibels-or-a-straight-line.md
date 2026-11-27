# ADR-0157: A game picks its volume curve: decibels by default, or a straight line

- **Status:** Accepted
- **Date:** 2026-11-26
- **Related tickets:** 3248 (this), 2342 (the curve), 2343 (ducking);
  game#0882 (the request)

## Context

[ADR-0091](0091-sound-groups-the-players-volumes-as-a-tree-of-groups-on-a-loudness-curve-worked-out-before-any-backend.md)
§2 put every player's volume on one curve: each step of 0 to 100 is
0.6 dB, so 50 is 30 dB under full and 20 is 48 dB under it, about 0.4%
of full. The argument was that a linear slider crowds what a player hears
into its top steps.

The first game on the engine found the other side of it (game#0882): a
player who had set the music to 20 heard nothing at all, and its owner
decided that a volume's number is that percentage of full ("1 should be
1% and 20 should be 20%"). Many PC games and operating systems show a
volume that way, and their players read the number as a share of full.

Both readings are reasonable, and which one fits is a game's call, not
the engine's.

## Decision

1. **The curve is data.** The audio manifest names it, `volume_curve:
   Linear` or `volume_curve: Decibels`, optional:
   `shuyi_assets::audio::VolumeCurve` (`#[non_exhaustive]`), in
   `AudioManifest::volume_curve`. One curve for every group of a game:
   a slider that meant something else in each row would read as a bug.
2. **`Decibels` stays the default.** It is ADR-0091's curve, unchanged
   (`loudness`), so a game that names no curve hears no change.
3. **`Linear` is `volume / 100` of full**, rounded to the nearest of
   4096 units (`shuyi_ui::audio::loudness::linear`): 20 is 819, 1 is 41,
   0 silent and 100 full. `gain_on(curve, volume)` is either; `SoundGroups`
   keeps the manifest's curve and `GroupGains` works every group's own
   gain on it before multiplying by its parent's.
4. **A ducking rule's `to` stays on the decibel curve** on either curve.
   It is the dip's own amount (ADR-0099), never a player's volume, and a
   game that changes its sliders keeps the same dip in decibels: `to: 90`
   is 6 dB down on both.
5. **Old linear volumes.** `position_of_linear` and
   `Volumes::set_from_linear` are for the decibel curve, where a linear
   percent moves to the position that sounds the same. On the straight
   line an old linear percent is already the same number: a game sets it
   with `Volumes::set`.

## Consequences

- A game on `Linear` hears little change over the top of its sliders
  (90 is about 1 dB under 100) and the whole fall in the last few steps:
  the trade ADR-0091 described, made by the game knowingly.
- No API breaks: a new enum, a new field of a `#[non_exhaustive]`
  struct, new functions, and a manifest field that defaults.
- A third curve, or one per group, is a new variant or field later;
  `VolumeCurve` is `#[non_exhaustive]` for it.
