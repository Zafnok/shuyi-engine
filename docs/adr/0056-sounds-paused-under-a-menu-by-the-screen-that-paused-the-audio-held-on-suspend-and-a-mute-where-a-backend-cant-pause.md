# ADR-0056: Sounds paused under a menu by the screen that paused, the audio held on suspend, and a mute where a backend can't pause

- **Status:** Accepted; extends ADR-0055, amends ADR-0024 §2 (the
  `Audio` trait) and §5 (`Suspend` holds the audio)
- **Date:** 2026-10-29
- **Related tickets:** 2394 (this), 2344 (handles, ADR-0055), 2316 (the
  debug view's pause), 2408 (rumble, parked), 3029 (the player's rules
  for every backend), 3031 (in-house output, parked), 3042 (focus),
  3043 (the web's hidden tab)

## Context

- **Before this ADR.** Sounds started with a handle belonged to the
  screen that started them (ADR-0055), but a pause menu couldn't hold
  the loops of the screen under it. On `SystemEvent::Suspend`, `App`
  stopped stepping (ADR-0024 §5) while the music, the voice and the
  loops sounded on; the web's hidden tab (3043) and the Playdate's lock
  and menu (3010) send `Suspend`.
- **quad-snd 0.2.8** has `Playback::set_volume` and `stop`, and no pause,
  natively or on the web. macroquad's music has a volume per `Sound`.
- **Other engines** pause the game's sounds under a pause menu (Godot's
  pause mode, Unity's `AudioListener.pause`, Unreal's paused sound
  class) and let the menu's own sounds play.
- **The layers** (ADR-0002): requests are plain data, and only a
  backend touches the device.

## Decision

1. **A pause is a screen's.** `AudioQueue::pause_sounds()` pauses every
   handled sound playing now; one started after it plays, so the menu's
   own sounds and loops are heard. Sound cues played once, the music and
   the voice go on. The pause belongs to the calling screen (the queue's
   owner, ADR-0055 §3) and ends with its `resume_sounds()`, or when it
   leaves the stack (`release`), so a menu that pauses on its first
   update needs no resume. A screen pauses at most once.
   `sounds_paused()` says whether any pause holds.
2. **Pauses don't stack further.** The queue keeps the set of owners
   pausing, and sends `AudioRequest::PauseSounds` when the set stops
   being empty and `ResumeSounds` when it empties: two pauses are one
   pause on the device. The device's `SoundRequest` has the same two.
   The crate-private owner `OUTSIDE_THE_STACK` is for code outside the
   stack (2316's debug pause): no pop releases it.
3. **`Audio::suspend` and `resume`.** `App::on_system_event` calls
   `platform.audio().suspend()` on `Suspend` and `resume()` on `Resume`,
   and nothing after the game has quit. `suspend` holds everything
   sounding (the handled sounds, the voice, the music); a cue played once
   finishes. `resume` brings back what `suspend` held and leaves a
   `PauseSounds` pause in place. Both are idempotent. Both are required
   methods, with no default: ADR-0039 §7 allows one on a trait backends
   implement where a default would be wrong, and an empty default would
   let a new backend's music play on through a suspend unseen. `Silent`
   does nothing.
4. **Pause where you can; mute where you can't.** A backend that can
   pause a sound pauses it; one that can't sets its volume to 0 and lets
   it run on silently. A held sound keeps the volume it is given and
   sounds at it when it is released. `music_playing` reports where the
   track really is: a paused track's clock holds, a muted one's runs.
5. **`shuyi-app` mutes.** quad-snd can't pause, so `PauseSounds` mutes
   each handled sound playing (a `paused` flag on its `Handled`),
   `suspend` mutes the handled sounds, the voice and the tracks, and
   `ResumeSounds` and `resume` set them again from their levels and the
   tracks' gains. While suspended, volumes asked for are kept and
   applied on `resume`. The music's clock runs on: a track resumes where
   it would have been. This is the backend's limit until the in-house
   output (3031, parked) or the browser's `AudioContext.suspend()`.
6. **The headless backend** keeps `PlayingSound::paused` (an addition to
   a `#[non_exhaustive]` struct) and `Speaker::suspended()`, and mutes as
   `shuyi-app` does: its music clock runs on while suspended.

## Consequences

- A pause menu holds the game's loops in one line, and forgets nothing
  when it closes. The music and voice under a menu go on; a flag on
  `pause_sounds` can hold them later if a game asks.
- Hiding the tab, locking the device or opening its menu silences the
  game, once its backend sends `Suspend`.
- `Audio` gains two required methods: a break for any backend outside
  the engine (the game has none). The requests and the field are
  additions.
- On `shuyi-app`, a track muted through a long suspend has moved on when
  it comes back, and a track played once may have ended.
- A loop started under one pause plays on through a second (a menu's
  loop, then the debug pause over it), since the device hears only the
  first `PauseSounds`.

## Alternatives considered

- **Pauses that stack** (each pause holding the sounds playing at its
  moment) — needs the device to keep a stack of sets; one pause on
  the device is simpler, and the debug pause over a menu is the only
  case.
- **Pausing the music and the voice under a menu too** — most games keep
  the music under a pause menu; holding it is a follow-up's flag.
- **Defaults for `suspend` and `resume`** — a backend that forgot them
  would play on through a suspend without a word.
- **Stopping and restarting the sounds on suspend** — loses where each
  sound was, and a restart is heard.
