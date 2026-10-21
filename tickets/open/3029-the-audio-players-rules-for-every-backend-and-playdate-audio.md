---
id: "3029"
title: "The audio player's rules shared by every backend, and audio on the Playdate: sounds baked to ADPCM, music streamed through the SDK"
type: feature
milestone: After MVP
model: opus-5.5
effort: high
status: todo
blocked_by: ["3003", "3010"]
nick_input: none
completed:
---

# 3029 — The audio player for every backend, and Playdate audio

## Context

- **The game ticket:** game#1170
  ([`1170-the-playdate-front-end-on-the-engines-backend.md`](https://github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1170-the-playdate-front-end-on-the-engines-backend.md))
  waits for this one. Found by the game's first engine-friction sweep,
  game#1153. 3010 lists audio under **Out** and asks for this follow-up
  "when this lands"; it is written now because there is working code to
  lift.
- **The job:** "play a game's sounds and music on the Playdate the way
  they play on the PC".
- **Friction:** the game's Playdate front end, 1735 production lines
  against an estimate of 100 (17×), outcome 1, with a signal: **engine
  code copied into the game**. The engine's audio player, which keeps
  the rules (cue volumes, picking a variant, how many voices, fades,
  the music clock), is `pub(crate) struct Audio<B: Backend>` in a
  private module of `shuyi-app` (`crates/app/src/audio.rs`). So the
  game re-wrote those rules for the Playdate (`crates/playdate/src/audio.rs`,
  245 lines, with a fake speaker in its tests) over the SDK's sample
  and file players (`src/pd/sound.rs`, 89 lines), and converts its OGG
  and WAV files to IMA ADPCM at build time (`crates/xtask/src/playdate_audio.rs`,
  about 400 lines).
- **The generic shape:**
  - the player's rules move out of `shuyi-app` to a place every backend
    shares (`shuyi-ui`'s audio, or `shuyi-platform` from 3003), behind
    the small backend trait they already have (`load_sound`;
    `load_music` and `poll_music` for streamed music; `length`;
    `play(sound, volume, looped)`; `set_volume`; `stop`), so a backend
    writes only that trait. The music clock is the player's own: it
    comes from when the player told the backend to play
    (`Track::started_at`);
    - once 2345 (After MVP) is done, the cue rules (which file, the
      volume picked and minimum gaps) run in `shuyi-platform`'s `App`
      before the device, and `SoundRequest::Sound` carries `variant`
      and `cue_volume`. Its part two (a ticket 2345 writes) adds copy
      caps, priority and the voice limit there, with `copy` on
      `SoundRequest::Sound` and `SoundRequest::StopCopy { copy }`,
      which stops one copy. What moves here is then the rest: playing a
      request at its volumes, the fades and the music clock;
  - `shuyi-playdate` implements it over the SDK's sample players (sounds
    loaded at start) and file players (music streamed, at most two at
    once for a fade);
  - `shuyi export playdate` bakes sounds and music to the formats the
    SDK plays.
- **Outside tactics:** any game exported to the Playdate with sound.
- **Found by the 2026-10-20 comparison** with Bevy 0.20, Godot 4.7,
  Unity 6 and Unreal 5.8. Bevy's built-in sink pauses, changes speed,
  seeks and mutes
  ([`AudioSink`](https://docs.rs/bevy/latest/bevy/audio/struct.AudioSink.html)).
  Godot's web export, in its default Sample mode, drops bus effects,
  reverb and Doppler, so engines say what each platform's sound can do.
  Shuyi's backend trait (`crates/app/src/audio.rs`) has only what
  quad-snd 0.2.8 plays. quad-snd's `PlaySoundParams` is
  `{ looped, volume }`, and its `Sound` and `Playback` have only
  `play`, `stop`, `set_volume` and `delete`. This ticket is the first
  to share that trait across desktop, the web and the Playdate, and
  later the GBA (3027) and the DS (3024). That makes it the cheap moment
  to decide how the trait grows without breaking a backend: a real
  pause for 2344 now, and later a pan (2363, parked), a loop region for
  intro-then-loop music (the ticket 2902 writes, parked) and a rate for
  pitch (3030, parked).

## Nick input

None.

## Scope

**In:**
- Move the rules; `shuyi-app` keeps its behavior on desktop and the web
  (its tests unchanged).
- **Either order with 2342 and 2343** (sound groups, ducking): if 2342
  is done, the groups' gains move with the rules. The shared player
  plays a sound at its cue's volume times the request's volume times
  `AudioFrame::gains.sound(cue)`, a track at `gains.music(cue)`, and the
  voice at `gains.voice()`, and sets them again on what is already
  playing when they change. The player never sees the group tree.
  Ducking (2343) reaches it through the same gains, and
  `Audio::voice_playing` (2343) reads the shared voice player. If 2342
  isn't done, carry the three volumes as they are.
- The Playdate backend's audio, from the game's two files (read them at
  the game's `main`).
- **Either order with 2345** (cue rules): if 2345 is done, the Playdate
  backend plays the file and the volume each `SoundRequest::Sound` names
  (`variant` and `cue_volume`). If not, move the player as it is, its
  variant pick included, and 2345 changes the moved player. The same
  goes for its part two (the ticket 2345 writes): if it is done, the
  Playdate backend keeps each copy's sample player and stops one copy
  on `SoundRequest::StopCopy`; if not, that ticket adds it here.
- The conversion in the exporter, from the game's `playdate_audio.rs`;
  check the license of anything it uses (`cargo deny`).
- **`.sfx.ron` sounds (2225):** the exporter renders one with
  `shuyi_assets::sfx::render` before the ADPCM conversion
  (`audio::playable`, with the `sfx` feature, gives its WAV), so it
  plays on the Playdate like any WAV. Either order: if 2225 is not
  done, there is nothing to do; if this lands first, 2225 adds the
  step.
- A track the game doesn't ship on the Playdate plays as silence, with
  a warning (the game's rule: some music is left out for the `.pdx`
  size).
- **Room to grow, as provided methods.** Today's methods stay required.
  A feature that some backends lack is a provided method. Its default
  does nothing and returns `Err(Unsupported)`. The first is a pause:
  `pause(sound)` and `resume(sound)`, where a playing sound stops, then
  later goes on from where it was.
  - The player calls such methods through one helper. The first
    `Unsupported` of each feature becomes one warning through
    `take_warnings`, for example "this backend can't pause sounds: they
    are muted instead". A second call gives no warning.
  - After the warning, the player carries on with the feature's
    fallback. For a pause it is 2344's rule: the sound is muted
    (volume 0) and runs on silently, and `resume` gives it its volume
    back.
  - The macroquad backend keeps the defaults, because quad-snd 0.2.8
    has no pause. The Playdate backend implements them over the SDK's
    sample and file players if the SDK has them (check its sound docs
    on the day). 2344 connects them to requests.
- **Either order with 2344** (looped sounds with handles,
  `pause_sounds`, `Audio::suspend` and `resume`): if 2344 is done, its
  handled sounds (`SoundRequest::Start`, `Stop`, `Volume`,
  `PauseSounds`, `ResumeSounds`), the per-playback handle
  (`Backend::play` returning `Self::Playing`, `set_playing_volume`,
  `stop_playing`) and its pausing rule move with the rules. `pause` and
  `resume` above then take the handle, and 2344's mute becomes their
  fallback. The Playdate gives each handle a sample player of its own
  and pauses it with the SDK rather than muting it, and its
  `Audio::suspend` pauses every player, so the `Suspend` that 3010
  sends on the lock and the system menu holds the sound. If 2344 isn't
  done, 2344 adds them where this ticket put the rules, and calls
  `pause` and `resume` for them.
- **Either order with 2341** (music crossfades): if 2341 is done,
  `MusicCommand::Start`'s `gain` and two tracks sounding at once during
  an overlap move with the rest, and the Playdate backend opens a second
  file player for the incoming track during an overlap. If not, 2341
  changes the rules in their shared place once this is done.
- **Either order with 2363 (parked, positional sound):** if 2363 is
  done, `shuyi-app`'s `Backend::play` already takes a pan and the trait
  has `can_pan`. Move them with the rest. The Playdate backend can pan:
  it sets the SDK sample player's left and right volumes from
  `shuyi_ui::audio::spatial::pan_gains`, with a test against the fake
  SDK. If 2363 isn't done, 2363 adds the pan to the shared trait by the
  rule the ADR below sets.
- **An ADR for the shared trait** (the `write-adr` skill), because the
  trait becomes public here. It records the required methods, the
  provided ones with their defaults and fallbacks, the one warning per
  missing feature, and the rule that a feature some backends lack is
  added as a provided method so that no backend breaks. It extends
  ADR-0004 §6 and ADR-0024 §2.

**Out (do not do):**
- MIDI through the SDK's synth (2205, 2206, parked).
- Any game's sound list.
- A stereo pan of this ticket's own. It is added as another provided
  method when a tick example needs it (ADR-0004 §6; 2363, parked).
- Pitch: the SDK's sample player can change its rate, but cue pitch
  waits until the desktop and the web can play it too (3030, parked);
  don't add it for the Playdate alone.
- Groups and ducking (2342, 2343: the player's rules over
  `set_volume`); this ticket only carries their gains across, as above.
  Effects, buses and seeking: write a ticket when a game asks.

**For the game:** the game's audio on the Playdate moves onto this in
game#1170. If the rules move to a new place, the game's desktop build
may need an import path changed in that PR.

## Implementation steps

1. Read `shuyi-app`'s audio module and its tests; read the game's
   Playdate audio and conversion. Check whether 2225, 2341, 2342, 2343,
   2344, 2345 (and its part two) and 2363 are done (the either-order
   clauses).
2. Move the rules behind the backend trait; desktop tests unchanged.
3. Add the provided methods and the warning helper, and write the ADR.
4. The Playdate backend's audio with a fake SDK.
5. The bake step in the exporter.

## Acceptance criteria

- [ ] `shuyi-app`'s audio tests pass unchanged.
- [ ] Host tests of the Playdate audio with a fake SDK: a cue plays at
      its volume; a fade between two tracks opens the second before
      closing the first; the music clock follows the file player; if
      2341 is done, a crossfade has both file players sounding, their
      volumes moving opposite ways.
- [ ] The example exported for the Playdate plays a sound in the
      Simulator, or the reason it can't is recorded.
- [ ] A test backend that implements only the required methods
      compiles. Pausing on it twice gives exactly one warning, the
      warning names the feature, and the sound is muted until it
      resumes.
- [ ] With the fake SDK, a paused track resumes from where it stopped.
      If the SDK lacks a pause, the Playdate backend keeps the default
      and the completion notes say why.
- [ ] If 2344 is done: host tests with the fake SDK show `suspend`
      pausing the sample and file players and `resume` starting them
      again where they were, and a handle's sample player stopping
      alone.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (host) against a fake SDK; the existing desktop audio tests.
- Unit: the provided methods' defaults, and one warning per missing
  feature.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
