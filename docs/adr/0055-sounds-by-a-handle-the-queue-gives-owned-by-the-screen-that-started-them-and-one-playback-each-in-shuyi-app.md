# ADR-0055: Sounds by a handle the queue gives, owned by the screen that started them, and one playback each in shuyi-app

- **Status:** Accepted; extends ADR-0004 §6 and ADR-0024 §2
- **Date:** 2026-10-29
- **Related tickets:** 2344 (this), 2394 (pauses under a menu and the
  audio held on `Suspend`, the follow-up), 2378 (one copy stopped
  alone), 2342 (sound groups), 2345 (cue rules), 3029 (the player's
  rules for every backend), 2367 (overlays that keep the screen below
  running, parked), 2363 (positional sound, parked)

## Context

- **Before this ADR.** A screen played a sound cue once
  (`AudioRequest::PlaySound`); nothing had an id, nothing looped but
  music, and nothing could be stopped. `shuyi-app` played sounds with
  macroquad's `play_sound`, which returns nothing, and its `stop_sound`
  stops every copy of a clip.
- **Other engines** (checked 2026-10-20): Bevy's `AudioSink`, Godot's
  audio players, Unity's `AudioSource`, Unreal's audio component and
  GameMaker's sound instance all loop a sound, stop or change one
  playing sound, and hold the game's sounds while it is paused.
- **quad-snd 0.2.8**, under macroquad 0.4.16, returns a `Playback` from
  `Sound::play`, with `stop` and `set_volume`, natively and on the web
  (the web bundle has `audio_playback_stop` and
  `audio_playback_set_volume`). It has no pause. On the web a second
  `AudioContext::new` shares the browser's one context: the bundle's
  `audio_init` makes it once.
- **Determinism** (ADR-0024 §2): requests are plain data, and a
  replay must give the same log.

## Decision

1. **A handle is the queue's count.** `AudioQueue::start_sound(cue)`
   and `start_sound_at(cue, volume)` give a `SoundId`, a counter in the
   queue from 1, never the device's, so the same calls give the same
   ids and the Harness logs them. The sound plays until it is stopped:
   looped if its cue says so (`SoundCue::looped`, `looped: true` in
   `audio.ron`, `false` when left out), else once.
   `stop_sound(id, fade_ms)` and `set_sound_volume(id, volume)` take it;
   an unknown or stopped id does nothing. The new requests are
   `AudioRequest::StartSound`, `StopSound` and `SoundVolume`, and the
   device's `SoundRequest::Start`, `Stop` and `Volume`. Both enums are
   `#[non_exhaustive]` from here (ADR-0039 §6), and so is
   `SoundRequest::Start`, built with `SoundRequest::start`.
2. **A fade is the queue's.** `AudioQueue::advance(dt)`, which
   `Game::collect_audio` calls each frame before it takes the requests,
   lowers a fading sound's volume in a straight line and sends a
   `SoundVolume` each frame, then the `StopSound` on the frame it ends.
   A device only ever stops at once, and every step is in the log. A
   sound fading out ignores volume changes and keeps its fade, unless
   it is stopped at once.
3. **A sound belongs to the screen that started it.** `ScreenStack`
   tells the queue the depth of the screen it is about to run (before
   `update`, `covered` and `uncovered`) and, after a transition, the top
   screen's, so a start from outside any screen belongs to the top.
   When a screen leaves in `apply_in` (a pop, or a replace before it
   goes to `apply`), its handles stop at once; one fading finishes its
   fade. `push` and `apply` build stacks and tell no one. So in a
   running `Game` no handle outlives its screen; a loop meant to outlive
   a screen is started by the screen below, or is music. 2367 (parked)
   changes the owner from the depth to the stack entry's serial number.
4. **No loop without a handle.** In a debug build, `PlaySound` of a
   looped cue panics in `collect_audio`, naming the cue. A release
   build plays it once.
5. **One playback each in `shuyi-app`.** Sound effects load into a
   quad-snd `AudioContext` of the player's own (opened with the first
   sound), natively and on the web, where `load_sound` waits for
   `Sound::is_loaded` as macroquad's loader does; quad-snd becomes a
   direct dependency on every target, at the version macroquad already
   brings. `Backend::play` returns `Self::Playing`, and `Backend` has
   `set_playing_volume` and `stop_playing`. The player keeps each
   handled sound's playback by its `SoundId`, at its cue volume times
   its requests' volume times the player's sound volume, and sets it
   again when the sound volume changes. Music and voices stay where
   they were: they have no `Playing` and are stopped by their clip.
6. **The headless `Speaker`** keeps the handled sounds playing
   (`PlayingSound`: id and cue) until they stop; one that plays once
   counts until it is stopped, as it knows no lengths.
   `Harness::sounds_playing` reads them, and `Harness::sounds` and the
   contract's `sounds` list a started cue as a played one.
   `Audio::stop_all` stops the handled sounds too.

Pausing them under a menu, and holding the audio on `Suspend`, are
2394's, with the rule for a backend that can't pause (it mutes).

## Consequences

- An ambient loop is one line, stopped or faded in one more, and a
  forgotten loop stops with its screen.
- A game's exhaustive `match` on `AudioRequest` or `SoundRequest`
  needs a wildcard arm, once; after that a new request is additive.
- Packs are rebaked: `SoundCue::looped` moved `pack::VERSION` to 5 and
  the audio manifest's golden value.
- `shuyi-app` opens one more output stream natively (quad-snd's mixer
  thread for effects), beside macroquad's own and the music's.
- A sound started with a handle that plays once stays live in the queue
  until it is stopped or its screen leaves: the queue knows no lengths.
  2378 brings file lengths.

## Alternatives considered

- **The device's playback id as the handle** — a replay could differ,
  and the Harness couldn't log it before the device ran.
- **Fades on the device** — quad-snd has none, and every backend would
  write its own; in the queue, each step is data in the log.
- **Handles that outlive their screen until stopped** — the commonest
  bug is the loop no one stopped; a loop that must outlive a screen has
  an owner below it, or is music.
- **Stopping macroquad's `Sound` for a handle** — `stop_sound` stops
  every copy of the clip, so two copies of one loop couldn't stop apart.
