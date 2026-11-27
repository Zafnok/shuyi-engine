---
id: "3248"
title: A game may play its volumes on a straight line, each number that percent of full
type: feature
milestone: MVP
model: opus-5.5 / gpt-6.1-sol
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-26
---

# 3248 — A game may play its volumes on a straight line, each number that percent of full

## Context

- **The game ticket:** game#0882
  (`github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/0882-volumes-are-a-straight-percentage-of-full.md`).
  Its owner found the music silent at a saved music volume of 20: on the
  loudness curve of 2342 (ADR-0091, `shuyi_ui::audio::loudness`, 0.6 dB a
  step) 20 is 48 dB under full, about 0.4% of full. He decided that a
  volume's number is that percentage of full: 1 is 1%, 20 is 20%.
- **The job in one sentence:** a game says its players' volume numbers
  are straight percentages of full volume, where today every game hears
  them on the dB curve.
- **The friction numbers:** no friction review: a missing choice.
- **The generic shape:** a volume curve, chosen per game in its audio
  manifest: `Decibels` (today's, the default) or `Linear` (gain
  `volume / 100` of full). The rest of the sound groups (the tree, the
  defaults, ducking) is unchanged.
- **An example from a game that isn't tactics:** a farming game whose
  options show the volume as "Music 40%" and whose players expect the
  number to mean a share of full, as the volume sliders of many PC games
  and operating systems do.

The dB curve stays the default: another game may want every step to
sound like the same change, as ADR-0091 argued. So nothing changes for a
game that doesn't name the curve.

## Nick input

None.

## Scope

**In:**
- `shuyi_assets::audio::VolumeCurve`, a new public enum,
  `#[non_exhaustive]`, with `Decibels` (`#[default]`) and `Linear`; the
  manifest's new field `AudioManifest::volume_curve` (additive: the
  struct is `#[non_exhaustive]`), written `volume_curve: Linear` in the
  file, optional.
- `shuyi_ui::audio::loudness::gain_on(curve, volume) -> Gain`: on
  `Decibels` it is `loudness(volume)`; on `Linear` it is
  `round(volume × 4096 / 100)`, 0 silent, 100 [`Gain::FULL`], above 100
  as 100.
- `SoundGroups` keeps the manifest's curve; `GroupGains` works each
  group's own gain on it.
- A ducking rule's `to` stays a position on the **dB** curve, whatever
  the game's curve: it is the dip's own amount, not a player's volume, so
  a game that picks `Linear` keeps the same dip in dB.
- The docs that say every volume is on the dB curve: the module docs,
  `GroupDef`, the book's *Settings* (`saves-and-settings.md`), and the
  ADR (a new one beside ADR-0091, which is accepted and not edited).
- `position_of_linear` and `Volumes::set_from_linear` stay as they are,
  for the dB curve: their docs say so. A game on `Linear` reads an old
  linear volume as the same number with `Volumes::set`.

**Out (do not do):**
- A third curve, or a curve per group.
- Changing the default curve, or any built-in default volume.
- The ducking's own units (ADR-0099).

## Implementation steps

1. `crates/assets/src/audio/groups.rs`: `VolumeCurve`; re-export it from
   `shuyi_assets::audio`. `crates/assets/src/audio.rs`: the field, and
   `RawFile`'s `#[serde(default)] volume_curve`.
2. `crates/ui/src/audio/loudness.rs`: `gain_on` and the straight line's
   gains; module docs for both curves.
3. `crates/ui/src/audio/groups.rs`: `SoundGroups` keeps `curve`;
   `GroupGains::update_ducked` uses `gain_on`.
4. Docs: `DuckRule::to` says the dB curve on every game; the book's
   *Settings*; a new ADR.

## Acceptance criteria

- [x] On `Linear`, every volume 0–100 plays at `volume / 100` of full,
  within one unit of 4096: 20 at 819, 1 at 41, 100 at 4096, 0 silent
  (unit test over all 101).
- [x] On `Decibels`, and with no `volume_curve` in the file, every gain
  is today's (`loudness`).
- [x] A manifest with `volume_curve: Linear` reads as `Linear`; one with
  an unknown curve is an error.
- [x] A group's gain on `Linear` is its volume's straight-line gain times
  its parent's; a ducking rule's floor is `loudness(to)` on either curve.
- [x] The book's chapter is updated (2802).
- [x] The skill pack (`skills/`) is updated for the changed format, or
  the completion notes have a `Skills:` line saying why not.
- [x] Guides in `docs/guides/` updated, if this adds a direction, a
  module or a move (ADR-0009 §5).
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `gain_on` on both curves for all 101 volumes; the manifest's
  field read and defaulted; `GroupGains` on `Linear` with a parent and a
  duck.
- Property: on `Linear` the gain never goes down as the volume goes up,
  and is `volume / 100` of full within rounding.

## Completion notes

- `VolumeCurve` (`Decibels`, the default, and `Linear`) in
  `shuyi_assets::audio`, read from the manifest's optional
  `volume_curve`; `loudness::linear` and `loudness::gain_on`;
  `SoundGroups::curve`, and `GroupGains` works each group's own gain on
  it. A ducking rule's floor stays `loudness(to)` on both curves.
- ADR-0157 records the choice; the book's *Settings* says how a game
  picks the straight line.
- Deviation: the pack's version moves to 25 (the Scope said nothing
  of the binary form): a new manifest field is in the baked form. The duck rule's error text still says "on the
  loudness curve", unchanged, so a game's tests of it see no change.
- Searched the game for what this touches (`gh search code --repo
  Zafnok/visions-of-shuyi 'volume_curve'`: no hit; the game pins
  `96eb097`): nothing in the game changes until it names the curve, which
  game#0882 does.

Game: game#0882 moves the pin and sets `volume_curve: Linear`.

Book: the *Settings* section of `saves-and-settings.md` names the new
field.

Skills: none, the skill pack's `shuyi-new-game` shows an `audio.ron` with
no groups or curve, and a new game keeps the default curve.

Breaking: shuyi-assets: `pack::VERSION` and `formats.ron` "audio manifest: audio-manifest": an audio manifest's binary form holds its volume curve, so a baked pack is of version 25 and a decoded audio manifest's state hash moves; a pack baked before is refused, so a game bakes again with `shuyi bake`, and blesses again its own golden row of a decoded audio manifest.

```migration
since: 0.1.0
what: The pack format is version 25, its audio manifest with the volume curve; a pack of version 24 is refused, and a golden row of a decoded audio manifest moves.
old: "pack::VERSION == 24"
new: "pack::VERSION == 25"
before: |
  cargo shuyi bake   # with the engine before 3248
after: |
  cargo shuyi bake   # again, with this engine
  cargo xtask golden --bless   # or the game's own way, for a row of a decoded audio manifest
```

Behaviour: none beyond the pack; with no `volume_curve` every gain is as before.
