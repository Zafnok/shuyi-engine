---
id: "2344"
title: "Looped sounds with handles: start, stop, fade and set the volume by id, sounds a screen owns, paused under a pause menu, and audio held while the system suspends the game"
type: feature
milestone: After MVP
model: opus-5.5
effort: high
status: done
blocked_by: []
nick_input: none
completed: 2026-10-29
---

# 2344 — Looped sounds with handles, paused under a menu and held on suspend

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6,
Unreal 5.8 and GameMaker. Every one of them can loop a sound, stop
or change one playing sound, and hold the game's sounds while the game
is paused:

- Bevy: a sound plays once or loops (`PlaybackSettings::LOOP`), and its
  `AudioSink` pauses, resumes, stops and sets its volume
  (<https://docs.rs/bevy/latest/bevy/audio/struct.AudioSink.html>).
- Godot: pausing the scene tree pauses the audio players under it; a
  player set to run while paused keeps playing, which is how a pause
  menu's own sounds are heard
  (<https://docs.godotengine.org/en/stable/tutorials/scripting/pausing_games.html>).
- Unity: `AudioListener.pause` pauses every source, and a source with
  `ignoreListenerPause` keeps playing
  (<https://docs.unity3d.com/ScriptReference/AudioListener-pause.html>).
- Unreal: an audio component stops or fades out, and pausing the game
  pauses its sounds except those marked as UI sounds.
- GameMaker: `audio_play_sound` returns an instance that
  `audio_stop_sound` and `audio_pause_sound` take
  (<https://manual.gamemaker.io/monthly/en/GameMaker_Language/GML_Reference/Asset_Management/Audio/audio_play_sound.htm>).

What Shuyi has today (checked 2026-10-20):

- `AudioRequest` (`crates/ui/src/audio.rs:97`) is `PlaySound` (once),
  `PlayMusic`, `StopMusic`, `PlayVoice`, `StopVoice` and
  `PreloadVoices`. No sound loops, none has an id, and none can be
  stopped. `SoundCue` (`crates/assets/src/audio.rs:61`) has `files`,
  `volume` and `credit`; only `MusicCue` has `looped`.
- `ScreenStack` (`crates/ui/src/screen.rs:199`) updates only the top
  screen. A game screen under a pause menu makes no new requests, but
  nothing it started can be held. `ScreenStack::apply_in` handles a
  push and a pop itself and hands a `Replace` to `apply`, which tells
  no screen.
- The platform's `Audio` trait (`crates/platform/src/audio.rs:83`) has
  `music_playing`, `play` and `stop_all`. On `Suspend`,
  `App::on_system_event` (`crates/platform/src/app.rs:401`) sets
  `suspended` and stops stepping, and `play` isn't called again until
  `Resume`. Whatever was sounding goes on sounding: the music and the
  voice. [ADR-0024](../../docs/adr/0024-the-platform-boundary-app-runs-a-frame-time-in-milliseconds-storage-handed-over-and-the-harness-on-the-headless-backend.md)
  §5 says nothing about sound, and neither does
  `docs/platform-backends.md`.
- `shuyi-app`'s player (`crates/app/src/audio.rs`): `play_sound` (line
  196) calls `Backend::play(sound, volume, false)`. Its device plays
  sounds with macroquad's `play_sound` (line 474), which returns
  nothing, and `stop` calls `stop_sound`, which stops every copy of the
  clip. Underneath, quad-snd 0.2.8 (the version macroquad 0.4.16 uses)
  returns a `Playback` from `Sound::play`, with `stop` and `set_volume`.
  The web bundle `crates/cli/web/mq_js_bundle.js` has
  `audio_playback_stop` and `audio_playback_set_volume` for it.
  quad-snd has no pause, and macroquad's `AudioContext::pause` exists on
  Android only. quad-snd is a direct dependency of `shuyi-app` on native
  only (`crates/app/Cargo.toml`), for music on contexts of its own
  (`crates/app/src/audio/native_music.rs`).

Planned around it:

- 3029 moves the player's rules to a place every backend shares and
  adds the Playdate's audio. It is a move: its tests stay unchanged,
  and it adds no loops or handles. It does add provided `pause(sound)`
  and `resume(sound)` methods to the shared backend trait, whose
  fallback, for a backend that can't pause, is this ticket's mute.
- 2316 (the debug pause) holds handled sounds through this ticket's
  `pause_sounds` (its either-order clause), and leaves the music, the
  voice and one-shot cues already playing out.
- 2378 (cue rules, part two, after 2345) needs one copy of a sound
  stopped alone, and either it or this ticket moves sound effects onto
  a quad-snd context of `shuyi-app`'s own.
- 2341 (music crossfades), 2342 (sound groups and volumes), 2343
  (ducking), 3042 (focus events) and 3043 (the web build's hidden tab
  sends `Suspend`) are After MVP. 2363 (positional sound) is parked. A
  looped sound that moves with a thing needs these handles: a follow-up
  after this ticket, written when a game asks (2363's Out).
- Parked: 3030 plans `VoiceId`, `looped`, `fade_to` and `stop` in the
  in-house mixer, and 3031 `Output::suspend`. They replace quad-snd
  under this API, not the API. 2725 (visual novel voices) and 2206
  (MIDI cues) assume sounds stopped by a handle.

Why it matters:

- **The game first.** Visions of Shuyi plays one-shot cues and music
  and asks for neither looped sounds nor a pause menu that holds them
  (its open tickets were checked with `gh`). Its one-shots and music
  don't change. It gets one thing: a `Suspend` holds its audio. The web
  build's hidden tab (3043) and the Playdate's lock and menu (3010) send
  `Suspend`, and today the music plays on.
- **Other 2D games.** Rain, a fire, an engine's hum, a crowd: a sound
  that loops until the game stops it, often with a fade. A pause menu
  holds them while its own cursor sounds play. A screen that leaves
  takes its loops with it, so a forgotten loop can't play forever.
- **The end goal** (2026-10-19, `docs/decisions.md`): an agent making a
  small game starts an ambient loop in one line and pauses it under the
  menu in one more, and a loop can't outlive the screen that started
  it.

Rules it keeps:

- Requests stay plain data (ADR-0024 §2). An id is a counter in
  `AudioQueue`, never the device's, so a replay gives the same ids and
  the Harness logs them.
- Only backends touch the device (`AGENTS.md` rule 5).
- Volumes and fades are `f32` in the UI layer, as the music's are
  today. No float reaches a simulation.
- `shuyi-ui` and `shuyi-platform` build without `std` (3005): the
  queue's and the `Speaker`'s maps are `BTreeMap`s, and a fade uses
  only `+`, `-`, `*` and `/`.

## Nick input

None.

## Scope

**In:**

- **Looped cues.** `SoundCue` gains `looped: bool`. In `audio.ron` it is
  optional on a sound and `false` when left out (`RawSound` reads it
  with `#[serde(default)]`), so every manifest loads as before.
  - `docs/formats.md` (the audio manifest) says so.
  - The pack's binary form changes: `pack::VERSION`
    (`crates/assets/src/pack.rs:62`) goes up by one, and the snapshot
    in `crates/assets/src/pack/tests.rs` is accepted.
  - `Check::sound` (`crates/assets/src/audio.rs`, where a `RawSound`
    becomes a `SoundCue`) copies it. **From 2024 (done):** `SoundCue`
    is `#[non_exhaustive]` and built with its `const fn new`
    (ADR-0039), which starts `looped` at `false`, so its callers (the
    tests in `crates/app/src/audio.rs`, `crates/ui/src/toy.rs`,
    `examples/hello-grid/src/sound.rs`) don't change. The one literal
    left outside `Check::sound`, in `crates/assets/src/credits.rs`'
    tests, gets `looped: false`.
  - The toy's manifest (`crates/ui/src/toy.rs`) gains a looped cue,
    `hum` (the `click.wav` file, `looped: true`), for the tests below.
  - **Either order with 2345 and 2342**, which also add fields to
    `SoundCue` (2345 adds `rules`, 2342 a `group`): the later
    ticket rebases onto the earlier one's struct, and bumps the pack
    version again.
- **Handles in `shuyi_ui::audio`:**
  - `SoundId`: a `Copy`, `Ord` id around a `u32` counter that
    `AudioQueue` keeps, from 1.
  - `AudioQueue::start_sound(cue) -> SoundId` and
    `start_sound_at(cue, volume) -> SoundId`: plays the cue (looped if
    its cue says so) until it is stopped. The volume is clamped as in
    `play_sound_at`. A cue that doesn't loop plays once, and its handle
    lives until it is stopped or its screen leaves; `play_sound` stays
    the call for a sound nothing needs to stop.
  - `stop_sound(id, fade_ms: u32)`: at once with 0. Otherwise the queue
    fades the handle's volume to 0 in a straight line over `fade_ms`,
    then stops it. The fade is the queue's: `AudioQueue::advance(dt)`,
    which `Game::collect_audio` (`crates/ui/src/game.rs:281`) calls
    before `take`, as it calls `MusicState::update`, sends a
    `SoundVolume` each frame and the stop on the frame it ends. So a
    device only ever stops at once, and the steps are in the Harness's
    log.
  - `set_sound_volume(id, volume)`.
  - An unknown or stopped id does nothing.
  - New `AudioRequest` variants: `StartSound { id, cue, volume }`,
    `StopSound { id }`, `SoundVolume { id, volume }`, `PauseSounds` and
    `ResumeSounds`. `AudioRequest::cue` gives a started sound's cue.
    `MusicState::request` ignores them. `is_known`
    (`crates/ui/src/game.rs:483`) checks a started cue.
  - In debug builds, `PlaySound` of a looped cue panics in
    `collect_audio`, as an unknown cue does: a loop with no handle could
    never stop. A release build plays it once.
  - **Either order with 2366** (parked, sound captions): if 2366 is
    done, `SoundCaptions::feed` treats a `StartSound` of a captioned cue
    as a `PlaySound`: its line is added once when it starts, never again
    per loop, with a test. If not, nothing to do (2366 does it).
- **A sound belongs to the screen that started it:**
  - `ScreenStack` tells the queue the depth of the screen it is about to
    run (before `update`, `covered` and `uncovered`), with a
    crate-private `AudioQueue::set_owner`. After a transition it sets
    the top screen's depth, so a start from outside any screen (a
    game's hook) belongs to the top screen. A handle and a pause
    remember their owner.
  - When a screen leaves the stack in `ScreenStack::apply_in` (`Pop`,
    `PopWith`, or a `Replace`, released there before it is handed to
    `apply`), the stack stops that screen's handles at once and ends
    its pause. A handle already fading finishes its fade. `apply` and
    `push` stay as they are: they build stacks and tell no one.
  - So no handle outlives its screen in a running `Game`, which only
    changes its stack through `apply_in`; the property test below shows
    it. A loop meant to outlive its screen is started by the screen
    below, or is music. The module docs say so.
  - **Either order with 2367** (parked, overlays that let the screen
    below run): if 2367 is done, the owner is the stack entry's serial
    number, not its depth. If not, 2367 changes it (its clause says so).
- **Pausing under a menu:**
  - `AudioQueue::pause_sounds()` pauses every handled sound playing
    now. Anything started after it plays, so the menu's own sounds and
    loops are heard. One-shot cues already playing finish. The music and
    the voice go on.
  - `resume_sounds()` ends the calling screen's pause. A screen pauses
    at most once (calling it every frame is fine), and leaving the stack
    ends its pause too, so a menu that pauses on its first update needs
    no resume call.
  - Several pauses (a pause menu and the debug pause) don't stack
    further: the queue sends `PauseSounds` when the first begins and
    `ResumeSounds` when the last ends.
  - A crate-private owner that no pop releases, for code outside the
    stack: 2316's debug pause.
  - **Either order with 2316:** if 2316 is done, its debug pause calls
    `pause_sounds` under that owner when it pauses and `resume_sounds`
    when it resumes, and a stepped frame's requests still play. If 2316
    isn't done, 2316 does that wiring.
  - **Either order with 2408** (parked, rumble): if 2408 is done, `App`
    stops the motors (2408's `RumblePlayer::stop` and `Motors::stop`)
    in a frame whose audio holds `PauseSounds`, and rumble cues asked
    for after it play, as the menu's own sounds do. If 2408 isn't done,
    2408 does this when it lands.
- **The device side (`crates/platform/src/audio.rs`):**
  - `SoundRequest` gains `Start { id, cue, volume }`, `Stop { id }`,
    `Volume { id, volume }`, `PauseSounds` and `ResumeSounds`, from the
    matching `AudioRequest`s in `SoundRequest::from_request`.
  - `Audio` gains `fn suspend(&mut self)` and `fn resume(&mut self)`,
    with no default body, so every backend decides. `suspend` holds
    everything sounding (handled sounds, the voice, the music); one-shot
    cues already playing finish. `resume` brings back what `suspend`
    held and leaves a `PauseSounds` pause in place. Both are idempotent.
  - The rule for every backend, written on the trait: a backend that can
    pause pauses; one that can't mutes (volume 0) and the sound runs on
    silently. A held sound keeps the volume it is given and sounds at it
    when it is released. `music_playing` reports where the track really
    is: a paused track's clock holds, a muted one's runs.
  - `stop_all` stops handled sounds too. `Silent` does nothing.
  - **Either order with 2345:** if 2345 is done, a `Start` carries the
    variant and cue volume its `Cues` picks, as `Sound` does; its gap,
    copy and voice rules don't apply to a handled sound, which the game
    stops itself.
- **`App`:** `App::system_events` (`crates/platform/src/app.rs:391`)
  hands the platform to `on_system_event`, which calls
  `platform.audio().suspend()` on `Suspend` and `resume()` on `Resume`.
  Nothing changes after the game has quit. `docs/platform-backends.md`
  says so in its frame order, its table of system events and its
  `audio` section.
- **The headless backend and the Harness:**
  - `Speaker` (`crates/platform/src/headless.rs:97`) keeps the handled
    sounds playing (id, cue, paused) from `Start`, `Stop`,
    `PauseSounds` and `ResumeSounds`, and whether it is suspended, with
    `Speaker::sounds_playing()` and `Speaker::suspended()`. A handled
    cue that doesn't loop counts as playing until it is stopped (the
    `Speaker` knows no lengths). Like `shuyi-app`, it mutes rather than
    pauses, so its music clock runs on while suspended.
  - `Harness::sounds_playing()` reads them. `Harness::sounds()`
    (`crates/platform/src/harness.rs:367`) lists a started cue as it
    lists a played one, and so does the contract session's `sounds`
    (`crates/cli/src/contract/session.rs`), with no change to the
    answer's shape.
- **`shuyi-app`:**
  - quad-snd 0.2.8 (MIT or Apache-2.0, which `deny.toml` allows) moves
    from `shuyi-app`'s native dependencies to its dependencies on every
    target, at the version macroquad's `audio` feature already brings,
    so no new crate enters the tree.
  - Sound effects load into a quad-snd `AudioContext` of `shuyi-app`'s
    own instead of macroquad's, natively and on the web. On the web,
    `load_sound` waits for `Sound::is_loaded`, as macroquad's
    `load_sound_from_bytes` does. Music and voices stay as they are.
  - `Backend::play` returns `Self::Playing`, a handle for that one
    playback (the name 2378 uses), and `Backend` gains
    `set_playing_volume(&playing, volume)` and `stop_playing(playing)`.
    For a music clip the handle stops the clip, as `stop` does today.
  - The player keeps `SoundId` to `Playing`. The player's sound volume
    reaches handled sounds already playing, as the music volume reaches
    tracks in `set_volumes`.
  - **Either order with 2342:** if 2342 is done, a handled sound plays
    at `gains.sound(cue)` in place of the player's sound volume, set
    again on a playing handle when that gain changes, as a track's is.
    If 2342 isn't done, 2342 does that.
  - quad-snd can't pause. `PauseSounds` and `suspend` set volume 0 and
    remember; `ResumeSounds` and `resume` restore. The backend's docs
    and the ADR say this is its limit. `Sound` in
    `crates/app/src/backend.rs` (the `Audio` this backend gives `App`)
    implements `suspend` and `resume` through the player, and does
    nothing before the game has loaded.
  - **Either order with 2378** (cue rules, part two): if 2378 is done,
    sound effects are on the own context already, and `Backend::play`
    returns `Self::Playing` with `stop_playing`: use them, and add
    `set_playing_volume`. If not, add them here, and 2378 uses them.
  - The tests' `Fake` backend records a playback's start, volume and
    stop.
- **Either order with 3029:** if 3029 is done, the player's rules live
  where it moved them: add the handles, pauses and suspend there. Its
  provided `pause` and `resume` then take the handle (`Self::Playing`),
  and `PauseSounds`, `ResumeSounds`, `suspend` and `resume` call them
  through its warning helper, with the mute above as their fallback
  (the macroquad backend keeps the defaults). Give the Playdate backend
  a sample player per handle, paused with the SDK rather than muted. If
  3029 isn't done, 3029 carries them across.
- **An ADR** (the `write-adr` skill, the next free number): sounds by a
  handle the queue gives, owned by the screen that started them;
  `pause_sounds`; `Audio::suspend` and `resume`; and the mute rule for
  a backend that can't pause. It amends ADR-0024 §2 (the trait) and §5
  (`Suspend` holds the audio), and ADR-0004 §6. ADR-0024's header names
  it.
- **What changes for the game:** `AudioRequest` and `SoundRequest` gain
  variants (an exhaustive match breaks), `SoundCue` gains a field,
  `Audio` gains two methods, and packs are rebaked for the new version.
  The new public types follow ADR-0039. The PR says so, and the game's
  PR that moves the pin follows it.

**Size.** If the PR passes about 600 changed lines (snapshots not
counted), the pauses go to a follow-up ticket (`write-ticket`), After
MVP: `pause_sounds` and `resume_sounds`, `PauseSounds` and
`ResumeSounds`, `Audio::suspend` and `resume`, `App`'s wiring, the mute
rule, the ADR-0024 §5 amendment, and the 2316 and 2408 clauses. The PR
points the tickets that name 2344 for them (2316, 2408, 3029, 3031,
3042, 3043) at it.

**Out (do not do):**
- holding the music or the voice under a pause menu: they go on. A
  follow-up adds a flag to `pause_sounds` if a game asks;
- seeking, a sound's position, and saving which sounds were playing;
- sound groups, a master volume and volumes per group (2342), ducking
  (2343), cue rules such as copy caps and priority (2345, 2378), music
  crossfades (2341), and intro-then-loop (the ticket 2902 writes);
- positional sound (2363, parked; ADR-0004 §6);
- sending `Suspend` from a hidden browser tab (3043) and focus events
  (3042): this ticket makes any `Suspend` hold the audio, and who sends
  it is theirs. Muting on focus loss is not done here;
- a real pause on quad-snd, or the browser's `AudioContext.suspend()`:
  the in-house output (3031, parked) gives that;
- overlay screens that keep the screen below running (2367, parked);
- the in-house mixer (3030, parked).

## Implementation steps

1. Read `crates/ui/src/audio.rs`, `screen.rs` and `game.rs`
   (`collect_audio`, `is_known`); `crates/platform/src/audio.rs`,
   `app.rs`, `headless.rs` and `harness.rs`; `crates/app/src/audio.rs`,
   `audio/native_music.rs`, `audio/voice.rs` and `backend.rs`; ADR-0024;
   and quad-snd 0.2.8's `Sound::play`, `Playback` and `web_snd.rs`.
   Check whether 2316, 2342, 2345, 2378, 2408 and 3029 are done (the
   either-order clauses).
2. `SoundCue::looped`, the RON field, `SoundCue::new` and the two
   literals, the toy's `hum` cue, `docs/formats.md`, the pack version
   and its snapshot.
3. `SoundId`, the new requests and the queue: start, stop with a fade,
   volume, pause and resume by owner, `advance`. Unit tests.
4. The stack's owners and releases (the `Replace` too), `advance` in
   `collect_audio` and the looped `PlaySound` check. Tests with the toy
   game, and the property test.
5. `SoundRequest`, `Audio::suspend` and `resume`, `Silent`, `App`, the
   `Speaker`, the Harness and the contract's `sounds`. Tests.
6. `shuyi-app`: quad-snd on every target, sound effects on the own
   context, playbacks by id, the mute for pause and suspend. Tests with
   the `Fake`.
7. Check by ear, natively and in the web build (the `run` skill), with
   a temporary change to `examples/hello-grid` that starts a looped cue,
   fades it out and pauses it. Don't commit the change; say what was
   heard in the Completion notes.
8. Write the ADR, update `docs/platform-backends.md`, and run the gates.

## Acceptance criteria

- [x] A manifest with `looped: true` on a sound loads it; one without
      the field loads as before. The pack snapshot moved with `VERSION`.
- [x] Unit: ids count from 1 and are the same on a second run of the
      same calls; `stop_sound(id, 0)` sends one `StopSound`; a 500 ms
      fade over frames of 1/60 s sends falling `SoundVolume`s, then one
      `StopSound` on the frame it ends; an unknown id sends nothing.
- [x] Harness, with the toy's `hum`: a screen starts it and pops; that
      frame sends its `StopSound`, and `sounds_playing()` is empty
      after. A replace does the same. A loop started by the screen under
      a pushed menu keeps playing.
- [x] ~~Harness: a menu pushed over a screen with a loop calls
      `pause_sounds`; the loop is paused, a loop the menu starts after
      that plays unpaused, and popping the menu stops the menu's loop
      and resumes the first with no `resume_sounds` call.~~ (not done:
      the pauses moved to 2394 by the size rule)
- [x] ~~Harness: two pauses send one `PauseSounds`, and one `ResumeSounds`
      when the last ends.~~ (not done: moved to 2394)
- [x] ~~Harness: `system(SystemEvent::Suspend)` leaves the `Speaker`
      suspended and `system(SystemEvent::Resume)` brings it back; a
      second `Suspend` changes nothing.~~ (not done: moved to 2394)
- [x] In a debug build, `play_sound` of a looped cue panics in
      `collect_audio`, and the message names the cue.
- [x] `shuyi-app` with the `Fake`: a looped cue starts a playback with
      `looped`; its volume follows `set_sound_volume` and the player's
      sound volume; `Stop` stops that playback only, while a second copy
      of the cue plays on. ~~`PauseSounds` and `suspend` set volume 0, and
      `ResumeSounds` and `resume` restore it.~~ (not done: moved to 2394)
- [x] The web build builds with quad-snd as a direct dependency, and
      the Completion notes say what was heard natively and on the web.
- [x] The ADR is written (ADR-0055). ~~ADR-0024's header names it.~~
      (not done: its status line says it extends ADR-0024 §2, and an
      accepted ADR isn't rewritten; the §5 amendment is 2394's ADR)
- [x] All gates in the `run-gates` skill pass, but `cargo xtask api`,
      whose `cargo-semver-checks` couldn't build the base's `shuyi-app`
      on this Windows machine (a build script it never ran); CI's `api`
      job runs it.

## Tests required

- Unit: `SoundId` counting; clamped volumes; the fade's steps and end,
  with bad frame times passing no time (`frame_seconds`); pauses by
  owner; `SoundRequest::from_request` for each new variant; the
  `Speaker`'s playing list.
- Property (in `crates/ui`, with `proptest`): for any sequence of
  starts, stops with and without fades, volume changes, pauses, pushes,
  pops and replaces through `ScreenStack::apply_in`, once every fade
  has ended: each handle started got exactly one `StopSound` or is
  live, every live handle belongs to a screen on the stack, and
  `PauseSounds` and `ResumeSounds` alternate.
- Snapshot / integration: the Harness cases above; the pack snapshot;
  `shuyi-app`'s player against the `Fake`.

## Completion notes

**Done** (ADR-0055):

- `SoundCue::looped` (`looped: true` in `audio.ron`, `false` when left
  out; `SoundCue::with_looped`), the toy's looped `hum`, `docs/formats.md`,
  `pack::VERSION` 5 and its snapshot, and the golden row of the audio
  manifest blessed.
- `shuyi_ui::audio::SoundId` and `AudioQueue::start_sound`,
  `start_sound_at`, `stop_sound(id, fade_ms)`, `set_sound_volume`,
  `sound_is_live` and `advance(dt)`, in `crates/ui/src/audio/handles.rs`.
  `AudioRequest::StartSound`, `StopSound` and `SoundVolume`;
  `AudioRequest` is now `#[non_exhaustive]` (ADR-0039 §6).
- `ScreenStack` sets the owner before it runs a screen and after each
  transition (`own_top`), and stops a leaving screen's handles in
  `apply_in` on a pop or a replace (`release_top`); `Game::with_stack`
  sets the top as owner. `collect_audio` calls `advance` before `take`,
  and panics in a debug build on `play_sound` of a looped cue, naming it.
- `SoundRequest::Start` (`#[non_exhaustive]`, `SoundRequest::start`),
  `Stop` and `Volume`; `SoundRequest` is `#[non_exhaustive]`. The
  `Speaker` keeps `PlayingSound`s (`Speaker::sounds_playing`,
  `Harness::sounds_playing`); `Harness::sounds` and the contract's
  `sounds` list a started cue, with no change of shape. `stop_all`
  stops the handled sounds.
- `shuyi-app`: quad-snd is a dependency on every target (0.2.8, the
  version macroquad brings; `cargo deny` passes). Sound effects load into
  a quad-snd context of the player's own, opened with the first sound;
  on the web `load_sound` waits for `Sound::is_loaded`, and natively a
  file quad-snd can't decode is a warning, not a panic.
  `Backend::play` returns `Self::Playing`, with `set_playing_volume` and
  `stop_playing`; the player keeps each handled sound's playback by its
  `SoundId` and sets it again when the sound volume changes.
- Tests: the queue's unit tests (ids, clamping, a stop, an unknown id, a
  500 ms fade over 1/60 s frames, bad frame times, a release), the stack's
  tests and the property test `no_handle_outlives_its_screen`, the
  `Game`'s fade and debug panic, `from_request` and the `Speaker`, the
  Harness cases in `crates/platform/tests/it/sounds.rs`, and the player
  against the `Fake`.

**Heard** with a temporary change to hello-grid (not committed): a
looped 220 Hz cue started on the stage, faded out over 2 s on Cancel.
Natively and in the web build (served locally, in a browser), the hum
looped under the menu blips and faded out over about 2 s on Cancel. The
web page loaded with no console errors.

**Deviations:**

- **Split by the ticket's size rule.** Without the pauses the PR is
  already well past 600 changed lines, so the pauses went to **2394**,
  as **Size** says: `pause_sounds` and `resume_sounds`, `PauseSounds`
  and `ResumeSounds`, `Audio::suspend` and `resume`, `App`'s wiring, the
  mute rule, the ADR-0024 §5 amendment, and the 2316 and 2408 clauses.
  2316, 2408, 3029, 3031, 3042, 3043 (and 2123, 2902, 3017, 3030, 3057)
  now name 2394 for them. The acceptance criteria on pauses, two pauses
  and `Suspend`, and the Fake's mute, are 2394's.
- **A `Playing` for effects only.** Music and voice clips have no
  playback handle (`Self::Playing` is `None` for them in the macroquad
  backend): no request starts one by a handle, and they stop by their
  clip as before.
- A sound fading out ignores `set_sound_volume`, and a second
  `stop_sound` with a fade keeps the first fade; one with 0 stops it at
  once.
- `AudioQueue::sound_is_live` is new: it says whether a handle is live,
  for games and tests.

**Either-order clauses:** 2316, 2342, 2345, 2366, 2378, 2408 and 3029
aren't done. 2342's, 2345's, 2366's, 2367's and 2378's now say what
2344 left them (**From 2344 (done)**); 3029's handle part is a
**From 2344 (done)** and its pause part an either-order with 2394.

**Follow-up:** 2394 (pauses under a menu and the audio held on
`Suspend`).

**For the game:** its next pin bump rebakes packs (version 5) and blesses
its golden row of the audio manifest, if it has one. Its
`if let AudioRequest::PlaySound { .. }` patterns keep compiling; an
exhaustive `match` on `AudioRequest` or `SoundRequest` needs a wildcard
arm. No game PR moves the pin yet; the game asks for nothing here, so
the next game ticket that moves the pin carries it.

**For Nick:** a game can now play a looping sound (rain, a hum) in one
line, fade it out in one more, and the loop stops by itself when its
screen closes. Holding sounds under a pause menu and while the device
sleeps is the next ticket, 2394. Visions of Shuyi hears nothing
different.

Breaking: shuyi-ui: audio::AudioRequest: gains `StartSound`, `StopSound` and `SoundVolume` and is non-exhaustive: a `match` on it outside `shuyi-ui` needs a wildcard arm.

```migration
since: 0.1.0
what: An audio request may be a sound started, stopped or changed by its handle, and the enum may grow.
old: "match request { AudioRequest::PlaySound { .. } => .., AudioRequest::StopVoice => .., /* every variant */ }"
new: "match request { AudioRequest::PlaySound { .. } => .., _ => .. }"
before: |
  match request {
      AudioRequest::PlaySound { cue, .. } => play(cue),
      AudioRequest::PlayMusic { .. } | AudioRequest::StopMusic => {}
      AudioRequest::PlayVoice { .. } | AudioRequest::StopVoice | AudioRequest::PreloadVoices { .. } => {}
  }
after: |
  match request {
      AudioRequest::PlaySound { cue, .. } => play(cue),
      _ => {}
  }
```

Breaking: shuyi-platform: audio::SoundRequest: gains `Start`, `Stop` and `Volume` and is non-exhaustive: a backend's `match` on it needs a wildcard arm, and plays `Start { id, cue, volume, .. }` until `Stop { id }`.

```migration
since: 0.1.0
what: A sound request may start, stop or change a sound by its handle, and the enum may grow.
old: "match request { SoundRequest::Sound { .. } => .., SoundRequest::Voice { .. } => .., /* every variant */ }"
new: "match request { SoundRequest::Sound { .. } => .., SoundRequest::Start { id, cue, volume, .. } => .., _ => .. }"
before: |
  match request {
      SoundRequest::Sound { cue, volume } => play(cue, *volume),
      SoundRequest::Voice { clip } => say(clip),
      SoundRequest::StopVoice | SoundRequest::PreloadVoices { .. } => {}
  }
after: |
  match request {
      SoundRequest::Sound { cue, volume } => play(cue, *volume),
      SoundRequest::Voice { clip } => say(clip),
      SoundRequest::Start { id, cue, volume, .. } => start(*id, cue, *volume),
      SoundRequest::Stop { id } => stop(*id),
      _ => {}
  }
```

Breaking: shuyi-assets: pack::VERSION: 5 (a sound cue's binary form has `looped`): a pack baked before is refused; bake it again with `shuyi bake`.

```migration
since: 0.1.0
what: Packs are at version 5; one baked before is refused until it is baked again.
old: pack::VERSION 4
new: pack::VERSION 5
before: |
  // a pack baked by an engine before this one
after: |
  // the same pack, baked again with `shuyi bake`
```

Breaking: shuyi-assets: formats.ron "audio manifest: audio-manifest": the manifest's state hash moved with the sound cues' `looped`; a game's own golden row of its audio manifest moves the same way (bless it).

```migration
since: 0.1.0
what: An audio manifest's state hash changed with the sound cues' new field.
old: '"audio manifest: audio-manifest": 7bca5ef50c2dd0fe'
new: '"audio manifest: audio-manifest": 0bcf9f8817da21e0'
before: |
  // the game's golden table, as blessed before
after: |
  // the same table after `cargo xtask golden --bless`
```

Book: none, the book has no chapter on sound yet; `AudioQueue::start_sound`'s rustdoc, the `audio` module docs and ADR-0055 say how to use handles.
