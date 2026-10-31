---
id: "2394"
title: "Sounds paused under a pause menu, and the audio held while the system suspends the game"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-10-29
---

# 2394 — Sounds paused under a pause menu, and the audio held on suspend

## Context

2344 split in two by its own size rule: it gave sounds handles and
owners, and left the pauses here. Read its completion notes and
[ADR-0055](../../docs/adr/0055-sounds-by-a-handle-the-queue-gives-owned-by-the-screen-that-started-them-and-one-playback-each-in-shuyi-app.md)
first.

**From 2344 (done):**

- `shuyi_ui::audio::SoundId`, and `AudioQueue::start_sound`,
  `start_sound_at`, `stop_sound(id, fade_ms)`, `set_sound_volume` and
  `advance(dt)` (`crates/ui/src/audio/handles.rs`). Each handle keeps
  its owner: the depth on the stack of the screen that started it,
  set by the crate-private `AudioQueue::set_owner`, which `ScreenStack`
  calls before it runs a screen (`ScreenStack::own_top`). When a screen
  leaves through `ScreenStack::apply_in` (a pop, or a replace before it
  goes to `apply`), the stack calls the crate-private
  `AudioQueue::release(depth)`, which stops that screen's handles.
- `AudioRequest` and `SoundRequest` are `#[non_exhaustive]` (ADR-0039),
  so the variants below are additions. They have `StartSound`,
  `StopSound`, `SoundVolume` and `Start`, `Stop`, `Volume`.
- The headless `Speaker` keeps the handled sounds as
  `shuyi_platform::headless::PlayingSound` (`#[non_exhaustive]`, `id`
  and `cue`), read by `Speaker::sounds_playing` and
  `Harness::sounds_playing`.
- `shuyi-app` plays sound effects on a quad-snd context of its own.
  `Backend::play` returns `Self::Playing`, and `Backend` has
  `set_playing_volume` and `stop_playing`. The player keeps each
  handled sound as `Handled { playing, cue_volume, volume }` by its
  `SoundId` (`crates/app/src/audio.rs`), and sets its level again when
  the player's sound volume changes.
- The property test `no_handle_outlives_its_screen`
  (`crates/ui/src/screen/tests/sounds.rs`) and the Harness tests in
  `crates/platform/tests/it/sounds.rs` cover the handles.

What is still missing, as 2344's Context found on 2026-10-20: a pause
menu can't hold the sounds of the screen under it, and on
`SystemEvent::Suspend` `App::on_system_event`
(`crates/platform/src/app.rs`) stops stepping, but whatever sounds goes
on sounding (the music, the voice, the loops). The game gets this: the
web build's hidden tab (3043) and the Playdate's lock and menu (3010)
send `Suspend`, and today the music plays on. ADR-0024 §5 says nothing
about sound.

Rules it keeps: requests stay plain data (ADR-0024 §2); only backends
touch the device (`AGENTS.md` rule 5); volumes are `f32` in the UI
layer; `shuyi-ui` and `shuyi-platform` build without `std` (3005), so
any map is a `BTreeMap`.

## Nick input

None.

## Scope

**In:**

- **Pausing under a menu (`crates/ui/src/audio/handles.rs`):**
  - `AudioQueue::pause_sounds()` pauses every handled sound playing
    now. Anything started after it plays, so the menu's own sounds and
    loops are heard. One-shot cues already playing finish. The music and
    the voice go on.
  - `resume_sounds()` ends the calling screen's pause. A screen pauses
    at most once (calling it every frame is fine), and leaving the stack
    ends its pause too (in `release`), so a menu that pauses on its
    first update needs no resume call.
  - Several pauses (a pause menu and the debug pause) don't stack
    further: the queue sends `PauseSounds` when the first begins and
    `ResumeSounds` when the last ends.
  - A crate-private owner that no pop releases, for code outside the
    stack: 2316's debug pause.
  - New `AudioRequest` variants `PauseSounds` and `ResumeSounds`.
    `MusicState::request` ignores them, and `is_known` knows them.
  - **Either order with 2316:** if 2316 is done, its debug pause calls
    `pause_sounds` under that owner when it pauses and `resume_sounds`
    when it resumes, and a stepped frame's requests still play. If 2316
    isn't done, nothing to do (2316 does that wiring).
  - **Either order with 2408** (parked, rumble): if 2408 is done, `App`
    stops the motors (2408's `RumblePlayer::stop` and `Motors::stop`)
    in a frame whose audio holds `PauseSounds`, and rumble cues asked
    for after it play, as the menu's own sounds do. If 2408 isn't done,
    nothing to do (2408 does it when it lands).
- **The device side (`crates/platform/src/audio.rs`):**
  - `SoundRequest` gains `PauseSounds` and `ResumeSounds`, from the
    matching `AudioRequest`s in `SoundRequest::from_request`.
  - `Audio` gains `fn suspend(&mut self)` and `fn resume(&mut self)`,
    with no default body, so every backend decides (ADR-0039 §7 allows
    a required method on a trait backends implement; say why here).
    `suspend` holds everything sounding (handled sounds, the voice, the
    music); one-shot cues already playing finish. `resume` brings back
    what `suspend` held and leaves a `PauseSounds` pause in place. Both
    are idempotent.
  - The rule for every backend, written on the trait: a backend that
    can pause pauses; one that can't mutes (volume 0) and the sound runs
    on silently. A held sound keeps the volume it is given and sounds at
    it when it is released. `music_playing` reports where the track
    really is: a paused track's clock holds, a muted one's runs.
  - `Silent` does nothing.
- **`App`:** `App::system_events` hands the platform to
  `on_system_event`, which calls `platform.audio().suspend()` on
  `Suspend` and `resume()` on `Resume`. Nothing changes after the game
  has quit. `docs/platform-backends.md` says so in its frame order, its
  table of system events and its `audio` section.
- **The headless backend and the Harness:** `PlayingSound` gains
  `paused: bool` (an addition: it is `#[non_exhaustive]`), set by
  `PauseSounds` and `ResumeSounds`; the `Speaker` keeps whether it is
  suspended (`Speaker::suspended()`). Like `shuyi-app`, it mutes rather
  than pauses, so its music clock runs on while suspended.
- **`shuyi-app`:** quad-snd can't pause. `PauseSounds` and `suspend`
  set volume 0 and remember (the handled sounds, the voice and the
  music tracks); `ResumeSounds` and `resume` restore, through the
  `Handled` levels and the tracks' gains. The backend's docs and the
  ADR say this is its limit. `Sound` in `crates/app/src/backend.rs`
  implements `suspend` and `resume` through the player, and does nothing
  before the game has loaded. The tests' `Fake` already logs a
  playback's volume.
- **Either order with 3029:** if 3029 is done, the player's rules live
  where it moved them: add the pauses and suspend there. Its provided
  `pause` and `resume` then take the handle (`Self::Playing`), and
  `PauseSounds`, `ResumeSounds`, `suspend` and `resume` call them
  through its warning helper, with the mute above as their fallback
  (the macroquad backend keeps the defaults). Give the Playdate backend
  a sample player per handle, paused with the SDK rather than muted. If
  3029 isn't done, nothing to do (3029 carries them across).
- **The ADR** (the `write-adr` skill, the next free number):
  `pause_sounds`, `Audio::suspend` and `resume`, and the mute rule for a
  backend that can't pause. It extends ADR-0055 and amends ADR-0024 §2
  (the trait) and §5 (`Suspend` holds the audio); ADR-0024's header
  names it.
- **What changes for the game:** `Audio` gains two required methods, a
  break for a backend (the game has none of its own). The new variants
  and the field are additions. The PR says so, and the game's PR that
  moves the pin follows it.

**Out (do not do):**
- holding the music or the voice under a pause menu: they go on. A
  follow-up adds a flag to `pause_sounds` if a game asks;
- sending `Suspend` from a hidden browser tab (3043) and focus events
  (3042): this ticket makes any `Suspend` hold the audio, and who sends
  it is theirs;
- a real pause on quad-snd, or the browser's `AudioContext.suspend()`:
  the in-house output (3031, parked) gives that;
- overlay screens that keep the screen below running (2367, parked).

## Implementation steps

1. Read 2344's completion notes, ADR-0055, `crates/ui/src/audio/handles.rs`,
   `crates/ui/src/screen.rs`, `crates/platform/src/audio.rs`, `app.rs`,
   `headless.rs` and `harness.rs`, and `crates/app/src/audio.rs` and
   `backend.rs`. Check whether 2316, 2408 and 3029 are done.
2. The queue's pauses by owner, the crate-private debug owner, and the
   release ending a pause. Unit tests.
3. `SoundRequest`, `Audio::suspend` and `resume`, `Silent`, `App`, the
   `Speaker` and the Harness. Tests.
4. `shuyi-app`: the mute for pause and suspend. Tests with the `Fake`.
5. Check by ear, natively and in the web build (the `run` skill), with
   a temporary change to `examples/hello-grid` that starts a looped cue
   and pauses it under a pushed screen. Don't commit the change; say
   what was heard in the Completion notes.
6. Write the ADR, update `docs/platform-backends.md`, and run the gates.

## Acceptance criteria

- [x] Harness: a menu pushed over a screen with a loop calls
      `pause_sounds`; the loop is paused, a loop the menu starts after
      that plays unpaused, and popping the menu stops the menu's loop
      and resumes the first with no `resume_sounds` call.
- [x] Harness: two pauses send one `PauseSounds`, and one `ResumeSounds`
      when the last ends.
- [x] Harness: `system(SystemEvent::Suspend)` leaves the `Speaker`
      suspended and `system(SystemEvent::Resume)` brings it back; a
      second `Suspend` changes nothing.
- [x] `shuyi-app` with the `Fake`: `PauseSounds` and `suspend` set
      volume 0, and `ResumeSounds` and `resume` restore it; `suspend`
      mutes the music and the voice too.
- [x] The property test `no_handle_outlives_its_screen` also pauses and
      resumes, and shows `PauseSounds` and `ResumeSounds` alternate.
- [x] The Completion notes say what was heard natively and on the web.
- [x] The ADR is written, and ADR-0024's header names it.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: pauses by owner, a screen pausing twice, the release ending a
  pause; `SoundRequest::from_request` for each new variant; the
  `Speaker`'s paused and suspended state.
- Property: the stack's property test with pauses.
- Snapshot / integration: the Harness cases above; `shuyi-app`'s player
  against the `Fake`.

## Completion notes

Done on 2026-10-29.

- **The queue** (`crates/ui/src/audio/handles.rs`): `pause_sounds`,
  `resume_sounds` and `sounds_paused`. The queue keeps the set of owners
  pausing; `PauseSounds` goes when the set stops being empty and
  `ResumeSounds` when it empties. `release` ends the leaving screen's
  pause after it stops its sounds. The crate-private owner is
  `OUTSIDE_THE_STACK` (`usize::MAX`); `release` asserts in debug builds
  that no pop reaches it. `AudioRequest` gains `PauseSounds` and
  `ResumeSounds`; `MusicState::request` ignores them and `is_known` knows
  them.
- **The device side:** `SoundRequest::PauseSounds` and `ResumeSounds`;
  `Audio::suspend` and `resume`, required (ADR-0039 §7: an empty default
  would let a new backend's music play on through a suspend without a
  word), with the pause-or-mute rule on the trait. `Silent` does nothing.
  `App::on_system_event` takes the platform and calls them; nothing after
  the game has quit (its early return).
- **Headless:** `PlayingSound::paused`, `Speaker::suspended()`; the
  `Speaker` mutes, so its music clock runs on while suspended.
- **`shuyi-app`:** `Handled` has a `paused` flag and the player a
  `suspended` flag and the voice volume asked for. A handled sound's
  level is 0 while paused or suspended, a track's music level is 0 while
  suspended, and the voice is set to 0. Volumes asked for meanwhile are
  kept and applied on the way back. A sound started or a track started
  while suspended starts muted (no frame plays while suspended today, so
  this is only a guard). `Sound` in `backend.rs` does nothing before the
  game has loaded.
- **Either order:** 2316, 2408 and 3029 are not done, so nothing was
  wired for them. Their tickets now say what this one gives them
  (**From 2394 (done)** in 2316 and 3029; the clause in parked 2408).
- **Checked by ear** (Nick, natively and in the web build): a temporary,
  uncommitted change to `examples/hello-grid` gave it two looped cues.
  The stage starts a low hum; the Leave menu pauses the sounds on its
  first frame and starts a higher buzz. Heard as expected on both: the
  hum plays through the intro, goes silent under the menu while the
  buzz plays, comes back when the menu is answered No (no resume call),
  and both end when the stage is left. The suspend path couldn't be
  heard: no backend sends `Suspend` yet (3042, 3043); the Harness and
  Fake tests cover it.
- **Docs:** ADR-0056; ADR-0024's status line names it;
  `docs/platform-backends.md`'s frame order, system events table and
  `audio` section.

Breaking: shuyi-platform: Audio: gains the required methods `suspend` and `resume`: a backend outside the engine implements both (pause, or mute where it can't, and bring back).

```migration
since: 0.1.0
what: The sound device holds its audio while the system suspends the game, so every backend implements Audio::suspend and Audio::resume.
old: "impl Audio for MyDevice { fn music_playing(..); fn play(..); fn stop_all(..); }"
new: "impl Audio for MyDevice { fn music_playing(..); fn play(..); fn stop_all(..); fn suspend(&mut self); fn resume(&mut self); }"
before: |
  impl Audio for MyDevice {
      fn music_playing(&self, now: Millis) -> Option<(&str, f64)> { self.music(now) }
      fn play(&mut self, frame: &AudioFrame<'_>, now: Millis) { self.mix(frame, now) }
      fn stop_all(&mut self) { self.silence() }
  }
after: |
  impl Audio for MyDevice {
      fn music_playing(&self, now: Millis) -> Option<(&str, f64)> { self.music(now) }
      fn play(&mut self, frame: &AudioFrame<'_>, now: Millis) { self.mix(frame, now) }
      fn stop_all(&mut self) { self.silence() }
      fn suspend(&mut self) { self.mute_all() }
      fn resume(&mut self) { self.unmute_all() }
  }
```

The new `AudioRequest` and `SoundRequest` variants are additions (both
enums are `#[non_exhaustive]`), and so is `PlayingSound::paused`. The
game has no backend of its own, so its pin bump needs no change.

Book: none, the book has no chapter on sound yet; `AudioQueue::pause_sounds`'s rustdoc, the `Audio` trait's docs, `docs/platform-backends.md` and ADR-0056 say how to use them.
