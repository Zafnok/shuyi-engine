# ADR-0139: Cue rules decided once, in `App`, before every backend: variants, volumes, gaps, copy caps and the voice rule

- **Status:** Accepted
- **Date:** 2026-11-23
- **Related tickets:** 2345 (this one: the place, variants, volumes,
  gaps, drops), 3419 (`run` answers the drops), 2378 (part two: file lengths, copy caps, priority, the
  voice limit, one copy stopped alone), 2379 (parked: pitch), 3017
  (parked: a voice count per target), 3027 and 3030 (parked: the GBA's and
  the in-house mixer's fixed voices), 3029 (the shared player)
- **Extends:** ADR-0004 §6 (audio: cues by id), ADR-0024 §2 (the
  platform boundary's `Audio` trait), ADR-0063 (the audio player shared by
  every backend)

## Context

Godot, Unity, Unreal and GameMaker each say, per sound, how its
variants are picked, how its volume varies, how soon it may retrigger,
how many copies may sound and which loses when too many do; Bevy doesn't
(the comparison of 2026-10-20 in ticket 2345). Shuyi's shared player
(ADR-0063) picked `files[rng.below(n)]` with a picker seeded from the
backend's clock: a file could play twice in a row, and a cursor that
moved several cells in one frame ticked several times at once.

Ports are the end goal (2026-10-19). The GBA (3027) and the in-house
mixer (3030) have a fixed number of voices: if each backend picked the
variant or the sound that loses, a game would sound different on each
port, and no test would see it. `App` already has, each frame, the
game's manifest, the frame's time in whole `Millis` (ADR-0024 §3) and
the requests `Game::collect_audio` hands out, before any backend.

## Decision

### 1. The rules are data on the cue

`SoundCue::rules: CueRules`, read flat from the sound's entry in the
manifest, each field optional and defaulting to today's behaviour, so
every manifest written before loads as it was:

- `pick: Pick`: `Random` (default), `NoRepeat` (never the last file
  again), `Shuffle` (each file once in a random order, then a new order
  that never starts with the file that ended the last), `InOrder` (the
  files in turn, then from the first). A cue of one file plays it every
  time.
- `volume_min: Option<u8>`: each play picks a whole percent from
  `volume_min` to the cue's `volume`. Over `volume` is an error at the
  cue's line; a manifest built in code that has it plays at `volume`.
- `min_gap_ms: u32`: a play less than this after the cue's last play
  that sounded is dropped.

`CueRules` is config (ADR-0039 kind 2): `#[non_exhaustive]`, a `const
DEFAULT` equal to its `Default`, and `const` `with_` methods.
`SoundCue::new` stays `const` with the default rules. `Pick` and
`DropWhy` are `#[non_exhaustive]`.

### 2. They run in `App`, after `Game::collect_audio` and before the device

`shuyi_platform::audio::cues::Cues` (`no_std`, integers only) turns a
frame's `AudioRequest`s into `SoundRequest`s, in order. `App` keeps one,
made from `ctx.music_seed` when the game starts running, and
`App::dropped_sounds` gives the frame's drops; the Harness records them
(`Harness::dropped_sounds`), and `run` is to answer them under
`dropped` (ticket 3419).

- Each play goes through, in order: the gap, then (2378) the cue's copy
  cap, then the voices. The first rule that drops it names the reason
  (`DropWhy::Gap`; 2378 adds `Copies`, `Voices` and `Cut`).
- A play dropped draws nothing from the picker and starts no gap, so the
  plays that sound are the same with it or without it. A play that
  sounds draws its file, then its volume.
- The picker is a `SplitMix64` seeded with `splitmix64(seed ^ SALT)`,
  apart from the simulation's RNG and from the music pool's picks. What
  plays never changes the game; the same seed and requests give the same
  plays and drops on every port.
- Time is the frame's `Millis`. A clock that goes backwards counts as no
  time passing.
- A sound started with a handle (`SoundRequest::Start`) is picked the
  same way, is never dropped and starts no gap: the game stops it
  itself. Music and voice clips have their own players and are not cue
  rules' business.
- A cue the manifest lacks, or one with no files, goes through as file 0
  at volume 0, and the device warns of it (or plays a sound the game
  added, at its own volume).

### 3. A backend plays the file and the volume it is told

`SoundRequest::Sound` and `Start` carry `variant`, an index into the
cue's `files`, and `cue_volume`, the percent picked. The shared player
keeps a slot per file, so `variant` names the same file when another
failed to load, and plays `percent(cue_volume) × volume × gain`. No
backend picks a file, a volume or a sound to drop.

### 4. The rules of part two (2378), decided here

- **A copy's end** is its start plus its file's length, read from the
  file's header when the manifest is checked (a WAV's `data` chunk over
  its block align and rate, an OGG's last granule). A copy of unknown
  length ends at once, so it never holds a copy or a voice.
- **The copy cap:** `max_copies` per cue, with `when_full`: `Skip` (the
  new play is dropped, `Copies`) or `StopOldest` (the cue's oldest copy is
  cut, and the new one plays).
- **The voice rule:** when `max_voices` copies sound in all, a new play
  takes the voice of the copy with the lowest priority, the oldest of
  those (ties to the lower id), if that priority is at most its own;
  otherwise it is dropped (`Voices`). A cut copy is stopped alone
  (`StopCopy`) and reported as `Cut`, naming its cue.
- **A backend's own voice maximum** (3030, 3027) is a hard limit under
  these rules: the rules admit no more than it has, and it never drops or
  steals a sound on its own.

### 5. Pitch waits

No backend can play a pitch today (quad-snd's `PlaySoundParams` is
`looped` and `volume`). 2379 (parked) adds a pitch range to these rules
once every desktop and web build plays through the in-house mixer, which
can resample, played through the player's `set_rate`.

## Consequences

- A game sounds the same on every backend: the decisions are made once,
  in integers, and tests and the Harness see them (and agents, in
  `run`'s `dropped`, with 3419).
- Footsteps vary without repeating, hits vary in loudness, and a cursor
  that moves several cells in one frame ticks once (hello-grid gives its
  cursor and move cues `min_gap_ms: 40`).
- What reaches the device changes: a session record's frame hashes cover
  each sound's file and volume, so a record made before this plays
  differently from its first sound. The pack's version goes up, as the
  manifest's binary form changes.
- `Player::load` and the Playdate's `audio::player` lose their seed; a
  backend of a game's own plays `variant` at `cue_volume` instead of
  picking.
- `Cues` keeps a little state per cue played, by id: the last file, a
  shuffle order, the time of the last play that sounded.

## Alternatives considered

- **Each backend applies the rules** (as the shared player picked
  variants) — a backend of a game's own would pick on its own, the
  Harness would never see the drops, and time would be float seconds.
- **The rules in `shuyi-ui`'s `AudioQueue`** — the queue has no manifest
  and no frame time, and a screen's requests are the game's record of
  what it asked for: they stay as asked.
- **A copy's end from the backend** — not every device says when a
  sound ends, and the answer would differ by port.
- **Pitch now** — a field that does nothing on the desktop and the web
  would mislead.
- **A "stop quietest" rule** — no game has shown it needs one; `WhenFull`
  is `#[non_exhaustive]` so one can be added.
