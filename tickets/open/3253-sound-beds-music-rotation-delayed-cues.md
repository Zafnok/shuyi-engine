---
id: "3253"
title: "Sound over time in shuyi_ui::audio: one looping bed shared by the screens, a music pool that rotates with quiet gaps, and sound cues played after a delay"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3253 — Sound beds, music rotation and delayed cues

## Context

A request from the game.

1. **The game ticket:** game#1414, which moves its ambience, its camp
   and town music and its battle's queued cues onto these once this
   lands
   (`github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1414-sound-beds-music-rotation-and-cues-on-the-engines.md`).
   It is not a blocker of this one.
2. **The job in one sentence:** keep three kinds of sound going over
   time without a screen tracking them by hand:
   - a looping sound bed under the music, shared by every screen;
   - a music pool whose tracks take turns with a quiet gap between;
   - short sounds that are due a moment after the thing that caused
     them.
3. **The friction numbers:** from the game's third engine-friction
   sweep (game#1397, `docs/engine/friction.md`):
   - **Measured:** about 228 production lines, at the game's pin
     `230080d`:
     - `crates/ui/src/ambience.rs` (76);
     - `crates/ui/src/screens/camp/music.rs` (87);
     - the pool setups in `screens/camp.rs` (about 20) and
       `screens/world_map.rs` (about 20);
     - `CueQueue` in `screens/battle/event_sounds.rs` (25).
   - **Estimate:** about 40 lines for what the game would keep:
     - which bed goes with which place and light;
     - its fade and quiet tunables;
     - which cues an event makes.
   - **Ratio:** 5.7×.
   - **Outcome:** 1, engine feature.
   - **Callers:** the bed's play-and-leave pair is called from 4
     screens (battle, camp, dialogue, world map). The rotation runs on
     2 screens.
4. **The generic shape:** three small pure schedules in
   `shuyi_ui::audio`, over what `AudioQueue` already has:
   - `start_sound_at`, `set_sound_volume`, `stop_sound` and
     `sound_is_live` for a looped sound with a handle;
   - `play_music_with` and `MusicSwitch` for music;
   - `play_sound` for one-shots.

   None of them names a place or an event.
5. **Not a tactics example:** a farming game has the same three needs:
   - a forest or rain bed that keeps playing from the farm into the
     barn's menu, and hushes under a cut scene;
   - day music that rotates its tracks with a few quiet seconds between;
   - a watering-can splash that sounds a quarter second after the swing.

**Either order with 2370** (parked: weather ambiences). If 2370 is done
first, its `AmbienceSound` (the looped sound of a weather preset) is
rebuilt on this ticket's `SoundBed`. If not, nothing to do: 2370 uses
`SoundBed` for its sound when it is unparked.

## Nick input

None.

## Scope

**In:**
- `audio::SoundBed` (`#[non_exhaustive]`, `SoundBed::off()`), one per
  game, kept in the game's state:
  - `play(&mut self, audio, owner: &'static str, cue: Option<&str>,
    hushed: bool, dt)`, called each frame by the screen that has a bed:
    - a new cue starts at volume 0 and fades in over `fade_s`;
    - another cue fades the old one out (`stop_sound` with the fade's
      ms) and starts;
    - `None` fades the bed out;
    - `hushed` fades it to 0 and back up while its loop goes on;
    - a bed whose sound the platform dropped (`!sound_is_live`) is
      forgotten.
  - `leave(&mut self, audio, owner)`: the bed this owner started fades
    out; another owner's bed plays on.
  - `cue()` and `with_fade_s(f32)`; the default fade is 0.5 s.
  - A bad `dt` counts as 0 (`frame_seconds`).
- `audio::Rotation` (`#[non_exhaustive]`), built by
  `Rotation::from_pool(manifest, pool, first, RotationTimes, now) ->
  Option<(Self, RotationAsk)>`:
  - It reads each track's `length_ms` from the manifest.
  - `RotationTimes { fade_out_s, quiet_s, fade_in_ms }` is
    `#[non_exhaustive]` with `new()`.
  - A track plays until `length - fade_out_s`, is then asked to stop
    with the fade, stays quiet for `quiet_s`, and the next in pool
    order starts. No track plays twice in a row unless the pool has one
    track.
  - `tick(now) -> Option<RotationAsk>`, `resume(now) -> Option<RotationAsk>`
    (back on a screen mid-track replays it), `track()` and
    `is_playing()`.
  - `RotationAsk` is `Play(cue)` or `Stop`. `RotationAsk::ask(&self,
    audio, manifest)` turns it into `play_music_with`, using the
    manifest's fade-out and the times' fade-in, or into
    `stop_music_with`.
  - Picking `first` stays the caller's (`Ctx::pick_music` exists).
- `audio::CueQueue` (`#[non_exhaustive]`, `Default`):
  - `extend(iter of (delay_s, cue))`;
  - `tick(dt) -> Vec<Cue>`, the cues now due, in the order queued (a bad
    `dt` counts as 0);
  - `is_empty()`.

  The cue type is generic: `CueQueue<C = &'static str>`.
- No API break, additions only. The game's PR that moves the pin follows
  this one (game#1414).

**Out (do not do):**
- weather presets, particles and overlays (engine#2370);
- positional sound (engine#2363);
- sound cues on a cinematic's timeline (engine#2364);
- choosing a bed from a place or a light: that is the game's table.

**Estimated size:** about 700 lines (code 230, tests 380, docs 90).

## Implementation steps

1. Read these, with their tests, at the game's pin:
   - `crates/ui/src/ambience.rs`;
   - `crates/ui/src/screens/camp/music.rs`;
   - `CueQueue` in `crates/ui/src/screens/battle/event_sounds.rs`.
2. Write `crates/ui/src/audio/bed.rs`, `audio/rotation.rs` and
   `audio/cues.rs` (or sections of `audio.rs` if the file layout
   prefers). Re-export them from `shuyi_ui::audio`.
3. Port the game's tests:
   - a bed crossfade, a hush and back, a leave by another owner;
   - a rotation through three tracks with the quiet gap, a resume while
     quiet, a pool of one;
   - a cue due exactly at its delay.
4. Show the bed and the rotation in the toy game if it has a screen
   with music. Otherwise use doc examples only.
5. Book: a section in the audio chapter.

## Acceptance criteria

- [ ] `SoundBed::play` with a second cue stops the first with the fade's
      ms and starts the second at volume 0 (a test reads `AudioQueue`'s
      requests).
- [ ] `SoundBed::leave` by an owner that didn't start the bed changes
      nothing.
- [ ] A `Rotation` of tracks 10 s and 20 s long, with a 3 s fade-out and
      a 5 s quiet gap, asks for `Stop` at 7 s and plays the second track
      at 15 s (10 s of track, then the gap).
- [ ] `CueQueue::tick(NaN)` returns nothing and loses nothing.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] Guides in `docs/guides/` updated, if this adds a direction, a module or a move (ADR-0009 §5).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each of the three, with the game's cases ported.
- Property: a `Rotation` never asks for the same track twice in a row
  over a pool of two or more.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
