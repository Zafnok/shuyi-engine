# ADR-0063: The audio player shared by every backend: a small required trait, provided methods for what some lack, and audio baked for the Playdate

- **Status:** Accepted; extends ADR-0004 §6, ADR-0024 §2 and ADR-0056
- **Date:** 2026-10-31
- **Related tickets:** 3029 (this), 3010 (the Playdate backend), 2394 (pause
  and suspend), 2344 (handles), 2341 (crossfades), 3027 and 3024 (GBA and DS
  sound, parked), 3017 (target capabilities, parked), 2363 (positional sound,
  parked), 2379 (cue pitch, parked), 3030 and 3031 (in-house sound, parked),
  game#1170

## Context

- **The rules were private.** A cue's volume, picking a variant, handled
  sounds, fades and the music clock were `pub(crate)` in `shuyi-app`, so the
  game wrote them again for its Playdate build (245 lines, later 864 with
  tests), over the SDK's players.
- **`shuyi-platform` is the boundary every backend already depends on**
  (ADR-0024) and touches no device. It has `no_std` and floats.
- **What devices can do differs.** quad-snd 0.2.8 can't pause, change a rate
  or pan. The SDK's sample and file players can pause and set a rate
  (`pd_api_sound.h`, SDK 3.1). Godot's web export drops whole features by
  platform, and says so. The trait is about to have many implementers
  (desktop, the web, the Playdate, later the GBA and the DS), so adding a
  feature must not break them.
- **The SDK loads a sound from a path.** `sample->load` and
  `fileplayer->loadIntoPlayer` take the path of a `.pda` that `pdc` made.
  `newSampleFromData` has no parameter for an ADPCM file's block size.
- **What the Playdate plays well:** the SDK's docs recommend IMA ADPCM, and
  game ADR-0084 measured it: mono at 44.1 kHz in 1,024-byte blocks.
- **`tools/layers.ron`** lists `lewton` and `hound` under `devices`, so that a
  game's rules never reach them, and `shuyi-cli` has `devices: false`.

## Decision

1. **The player is `shuyi_platform::audio::player::Player<B: Backend>`**, with
   its voice clips and all its rules. A backend writes only `Backend`.
   `PlayerDevice<B>` is the rest of a backend's `Audio`: the player once the
   game has loaded, its warnings said through `Backend::warn`. `shuyi-app`
   keeps quad-snd under the trait and where its folders are. `Player` is
   `no_std`.
2. **The required methods stay what they were:** `load_sound`, `load_music`,
   `poll_music`, `length`, `play`, `set_volume`, `stop`, `set_playing_volume`
   and `stop_playing`.
3. **A feature some backends lack is a provided method.** Its default does
   nothing and says `Err(Unsupported)`, so adding one breaks no backend. Today:
   - `pause(playing)` and `resume(playing)`: the fallback is ADR-0056's mute;
   - `set_rate(sound, rate)`, 16.16 fixed point (`RATE_ONE` is 1.0), from the
     sound's next `play`: the fallback is playing at 1.0. Nothing but
     `Player::set_rate` calls it, and cue pitch (2379) will.

   Two provided methods have defaults that do the work, for a backend that
   wants otherwise: `load_sound_file(file, files)`, which reads the bytes and
   calls `load_sound`, and `warn(line)`, which says nothing.
4. **One warning per missing feature.** The player calls these through one
   helper (`Provided`): the first `Unsupported` of each feature becomes a line
   of `take_warnings`, naming it ("this backend can't pause sounds: they are
   muted instead"), and the fallback follows. A pan or a loop region is added
   the same way, with its own line.
5. **Holding.** A playback is held if a `PauseSounds` or a suspend holds it, and
   paused once whichever came first. On a backend that pauses, volumes asked for
   meanwhile are set on the paused player. A resume resumes only what nothing
   else holds. One-shot sound cues aren't held (ADR-0056).
6. **The music clock stays the player's.** It counts from when the player told
   the backend to play. A track really paused holds its clock at the last
   frame's time, and at the first `play` after the resume `started_at` moves
   by the time away; the same goes for a voice's end. So `Audio`'s `suspend`
   and `resume` need no clock, and a muted track's clock runs on, as before.
   Reading the file player's offset instead would give the trait a required
   call for one backend.
7. **The Playdate.** `shuyi_playdate::audio::Speaker` implements every provided
   method. It keeps eight sample players, each playback one of its own with a
   serial, so an old handle does nothing; a new sound takes a free one, else the
   next in turn that isn't paused. A file player opens per track or clip, and is
   freed when its `Sound` is dropped. A track that isn't in the `.pdx` fails its
   load, which is the player's existing warning and silence.
8. **The bake.** `shuyi export playdate` asks `GameTools::audio(target)` what
   the game ships there, decodes each file to mono and writes an IMA ADPCM WAV
   under `audio/` or `music/` (`shuyi_assets::audio::BAKED_SOUND_DIR`,
   `BAKED_MUSIC_DIR`, `without_extension`, which both sides use), which `pdc`
   makes a `.pda`. A music cue left out of what the tools answer isn't baked.
   Each result is cached by its source's CRC-32 and length. A file at another
   sample rate than `SAMPLE_RATE` is an error, as in `check`.
9. **`readers` in `tools/layers.ron`.** A crate's entry may name crates of the
   `devices` list it reaches all the same, each a file reader used at build time
   and no device. `shuyi-cli` names `lewton` and `hound`; the list itself, and
   the fence around a game's rules, are unchanged. Both are in the lock file
   already through quad-snd, under MIT OR Apache-2.0 and Apache-2.0.

## Consequences

- A new backend is that one trait. The game's copies go in game#1170.
- The trait is public, so a required method added later breaks every backend:
  that is what the provided-method rule is for.
- `lewton` was last released in 2021. In-house Vorbis (2212–2214, parked)
  would replace it here too.
- The Playdate's file players stream from the `.pdx`; at most two tracks open at
  once is the music state's rule, not a limit in the backend. Voice clips aren't
  baked yet, so they are the same warning and silence (3078).
- A sound played once on the device finishes through a suspend, as everywhere.

## Alternatives considered

- **The player in `shuyi-ui`** beside `AudioQueue`. It would put a device trait
  under the screens, and `SoundRequest` is `shuyi-platform`'s.
- **Required `pause` for all**, each backend writing its own fallback: every
  backend repeats the mute, and every new feature breaks them all.
- **`load_sound` taking a path**: it would change what the desktop and web
  backends do with the game's embedded bytes. The provided `load_sound_file`
  leaves them as they were, and leaves the Playdate's `load_sound` an error
  that says why.
- **`newSampleFromData`** with bytes read from the `.pda`: no parameter for the
  ADPCM block size, and the `.pda`'s layout isn't documented.
- **Our own Vorbis decoder for the export**: parked in 2212–2214, and not this
  ticket's to start.
- **Taking `lewton` and `hound` off the `devices` list**: it would also let a
  game's rules reach them.
- **Lifting the game's voice stealing into the player**: copy caps and the voice
  limit are 2378's, in `App`. Here it is only which device player is free.
