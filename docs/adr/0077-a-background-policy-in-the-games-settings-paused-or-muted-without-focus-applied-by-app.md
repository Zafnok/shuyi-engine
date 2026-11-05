# ADR-0077: A background policy in the game's settings, paused or muted without focus, applied by App

- **Status:** Accepted
- **Date:** 2026-11-04
- **Related tickets:** 3093 (this), 3042, 3043, 2342, 2394, 2739
- **Extends:** ADR-0076

## Context

ADR-0076 made the desktop and the web send `FocusLost` and `FocusGained`,
and `App` let go of held input on `FocusLost`. What the game does while
its window has no focus was left to its `SystemHook`: today it keeps
running with its sound. A game that wants to stop or go quiet wrote that
itself, and an agent's game had no one place to say it (Nick's end goal of
2026-10-19 in `docs/decisions.md`: the pause and the mute from one
constant). Other engines have one switch each: Unity's
`Application.runInBackground`, Bevy's `WinitSettings::unfocused_mode`,
Godot's focus notifications.

Two rules bind it: a pause must change no replay or hash (ADR-0002: it is
"no update", never a different update), and `shuyi-ui`'s settings stay
`no_std`.

## Decision

### 1. The policy is a constant of the game's settings

In `shuyi-ui` (`crates/ui/src/settings.rs`):

- `Background { pub pause: bool, pub audio: BackgroundAudio }` and
  `BackgroundAudio::{Keep, Mute}`, both `#[non_exhaustive]` (ADR-0039),
  so the player's own setting and fewer frames in the background (3043)
  and a duck can join;
- `Background::KEEP_RUNNING` (no pause, `Keep`) and the const builders
  `paused(self)` and `muted(self)`: a game writes
  `Background::KEEP_RUNNING.paused().muted()`;
- `GameSettings::BACKGROUND: Background`, an associated const beside
  `DEFAULT_FRAME_CAP`, defaulting to `KEEP_RUNNING`.

It is a constant, not a field of `Settings`: it is the game's choice, not
the player's, and is never saved. The player's setting (3043) builds on
it.

### 2. `App` applies it while unfocused

`App` copies `<G::Settings as GameSettings>::BACKGROUND` into a private
field when it is made (its tests set another). While the window has no
focus and the game runs (not while loading, suspended or quit):

- **pause:** the input is drained and dropped, no update, no fixed steps,
  and the game's last frame is presented again each frame. No perf
  record is made. On `FocusGained` while paused, the clock restarts as on
  `Resume`, so the time away makes no catch-up steps.
  `App::is_paused()` says so. `QuitRequested` still quits;
- **mute:** every frame's `AudioFrame` carries its requests and music as
  usual with the music, sound and voice volumes at 0, so the music keeps
  its place and a sound started meanwhile is silent rather than lost. A
  sound already playing plays out, since its volume is set when it
  starts. Paused and muted, `App` still calls `audio().play` each frame,
  with nothing in it and the volumes at 0, so the music goes quiet. The
  first frame with the focus back sends the player's volumes again.

The game's hook still hears `FocusLost` and `FocusGained` after `App`'s
part. Suspend is unchanged: it already stops everything, and the sound on
`Suspend` is 2394's.

## Consequences

- The default changes nothing: every game, test, snapshot and replay
  behaves as before until it opts in. The change is additive.
- A pause is a frame with no update, like a suspended frame, so a session
  record replays it: the focus events are recorded with the system's
  events, and the frame hash covers the same facts.
- With 2342's `Gains`, the mute becomes `Gains::muted()` in place of three
  zeros (2342 carries this).
- Looped sounds started before the focus loss keep their volume until
  2394 lets the mute follow them.

## Alternatives considered

- **A field of `Settings`, saved with the player's** — the policy is the
  game's; the player's choice is 3043's, layered on this one.
- **Leave it to each game's `SystemHook`** — what was there; every game
  writes the same pause and mute, and a hook can't skip the update or set
  the device's volumes without `App`.
- **Pause by sending `Suspend`** — `Suspend` holds the device's audio and
  shows nothing; a window behind another is still seen, so it should show
  its last frame (ADR-0076 §1).
- **Mute by not calling `play`** — the music commands and sounds of a
  muted, running game would be lost, and the music would keep its volume.
