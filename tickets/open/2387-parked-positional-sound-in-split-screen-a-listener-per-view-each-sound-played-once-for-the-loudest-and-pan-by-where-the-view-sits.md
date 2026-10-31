---
id: "2387"
title: "Parked: Positional sound in split screen: a listener per view, each sound played once for the listener that hears it loudest, and pan by where the view sits"
type: feature
milestone: "Engine: real time"
model: implementation
effort: high
status: blocked
blocked_by: ["2363", "2386"]
nick_input: none
completed:
---

# 2387 — Positional sound in split screen: a listener per view

> **Parked** (Nick, 2026-10-24: "These will be parked since they're not needed for Visions of Shuyi"): local split screen waits until Nick unparks it. Visions of Shuyi is turn-based and plays hotseat on one shared view, so nothing here is for the game. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Nick, 2026-10-24 ([`docs/decisions.md`](../../docs/decisions.md)):
"add parked tickets for adding local split screen multiplayer, both
vertical and horizontal, and lets say even 3 or 4 player split screen".

2363 gives positional sound one listener a frame: `Listener { at:
SoundAt, half_width: u32 }`, set with `AudioQueue::set_listener`, and
`spatial_mix(listener, at, falloff) -> Option<Mix>` for a sound's gain
and pan. Its Out leaves "several listeners at once (split screen)" to a
ticket of its own. This is that ticket. 1120 gives every camera
`Camera::listener()`, and 2386 gives each seat a view with its own
camera.

With two to four views, a sound near player 3 must still be heard, and
heard once, not once per view. The engines do it like this:

- Unity's `AudioListener` allows one active listener; split-screen
  games pick the nearest one, or write their own mix;
- Unreal 5 has one listener per local player in split screen and plays
  a sound for the closest one by default;
- Godot gives each viewport its own current `AudioListener2D`.

Unreal's rule fits: each sound is heard by the listener closest to it,
played once. Here "closest" is "loudest", by each listener's falloff,
which is the same thing for one falloff and stays right when the views'
zooms differ. The pan has to know where the view sits: in a side-by-side
split, a sound at the right edge of the left view is in the middle of
the screen, so it should sound in the middle.

Kinds of game that use it: any split-screen game with sounds in its
world (racing, arena shooters, co-op platformers).

## Nick input

None. Unparking is Nick's call, as for every parked ticket.

## Scope

**In:**

- **Several listeners** (`crates/ui/src/audio/spatial.rs`, `no_std`, in
  integers only). New public types follow 2013's ADR: `ViewSpan` and
  `Listeners` are `#[non_exhaustive]` with constructors, and `SplitPan`
  is a `#[non_exhaustive]` enum.
  - `ViewSpan { left: u32, width: u32, frame_width: u32 }`: where the
    listener's view sits across the frame, in console pixels.
  - `SplitPan`: `ByScreen` (the default), `InView`, `Centre`.
  - `Listeners::new(pan: SplitPan)` and `Listeners::push(listener,
    span) -> Result<(), ListenerError>`, at most 4 (one per view; the
    spare quarter's overview is not a listener).
  - `spatial_mix_many(listeners: &Listeners, at: SoundAt, falloff:
    &Falloff) -> Option<Mix>`:
    - each listener's `spatial_mix`; the one with the highest gain
      wins, a tie going to the earlier listener (the lower seat). No
      listener hears it: `None`, the sound is never sent;
    - the gain is the winner's: the sound plays once, never once per
      listener;
    - the pan, by `SplitPan`:
      - `ByScreen`: the winner's pan inside its view, moved to where
        its view sits. `centre = (2 * left + width - frame_width) * ONE
        / frame_width` and `pan = centre + in_view * width /
        frame_width`, in `i64`, clamped to `±ONE`. With one view the
        size of the frame, this is 2363's pan exactly;
      - `InView`: the winner's pan, as if its view were the screen;
      - `Centre`: 0, for games that want no pan in split screen.
- **The requests** (`crates/ui/src/audio.rs`):
  `AudioQueue::set_listeners(Listeners)`, lasting one frame like 2363's
  `set_listener`. Setting both in one frame is a debug-build panic (one
  or the other). `Game::collect_audio` resolves `PlaySoundFrom` with
  `spatial_mix_many` when there are several, and with 2363's rule when
  there is one, so the platform, the Harness and the contract still
  see only `PlaySound` with its volume and pan.
- **From split screen** (2386): `SplitScreen::listeners(pan: SplitPan)
  -> Listeners`, one per view in view order, from each view's
  `Camera::listener()` (1120) and its rectangle, and a lost seat's view
  among them. A stage calls `ctx.audio().set_listeners(...)` once a
  frame with it, as a one-player stage calls `set_listener`.
- **Docs and skills**: the audio page in `docs/` (what the `document`
  skill says) and the split-screen section of the `shuyi-stage-field`
  skill (2386 writes it) get the rule and the three pans.
  `check-skills` passes.

**Out (do not do):**

- one listener (2363);
- the views and their cameras (2386);
- a sound played once per view, on purpose, as each view's own (a
  player's own engine hum heard only by them): it needs 2344's handles
  and per-player output, which no target here has;
- sound to each player's own headphones or pad speaker;
- a looped sound that moves with a thing (2363's Out, after 2344);
- captions saying which side a sound came from in split screen (2381,
  parked, reads the pan it is given; a follow-up if a game asks for
  captions per view).

## Implementation steps

1. Read 2363's `spatial.rs`, `AudioQueue` and `Game::collect_audio` as
   they landed, 1120's `Camera::listener` and 2386's `SplitScreen`.
2. Add `ViewSpan`, `SplitPan`, `Listeners`, `ListenerError` and
   `spatial_mix_many`.
3. `AudioQueue::set_listeners` and its resolution in `Game`.
4. `SplitScreen::listeners`.
5. Docs and the skill.
6. Run the gates.

## Acceptance criteria

- [ ] With one listener whose span is the whole frame,
      `spatial_mix_many` equals `spatial_mix` for every place and
      falloff (property test).
- [ ] With two listeners side by side (`Columns`), a sound at the right
      edge of the left view gets a `ByScreen` pan of 0, and one at the
      left edge of the left view a pan of `-ONE` (unit tests).
- [ ] Stacked (`Rows`), a sound's `ByScreen` pan is its `InView` pan.
- [ ] A sound heard by two listeners plays once, at the louder gain: a
      Harness test on 2386's test stage with two seats and one sound
      between their dots finds one `PlaySound` in `audio_log`.
- [ ] A sound past every listener's `silent` is never sent.
- [ ] `Centre` gives a pan of 0 for every sound.
- [ ] `shuyi-ui` still builds for `thumbv7em-none-eabihf`.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the winner and its tie rule; each `SplitPan` on two, three and
  four views; `ListenerError` past four; the debug panic on both
  setters.
- Property: one full-frame listener equals 2363; the pan stays in
  `±ONE`; mirroring the views and the sound left to right gives the
  opposite pan exactly.
- Snapshot / integration: the Harness test above.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
