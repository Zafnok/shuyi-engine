---
id: "2393"
title: "The headless Speaker plays two tracks: a crossfade seen in the Harness with its gains and a load delay"
type: feature
milestone: After MVP
model: sonnet-5
effort: medium
status: todo
blocked_by: ["2341"]
nick_input: none
completed:
---

# 2393 — The headless Speaker plays two tracks

## Context

2341 (done) added music crossfades and fade-ins
([ADR-0054](../../docs/adr/0054-music-switches-are-data-todays-fade-as-the-default-a-crossfade-as-two-tracks-with-gains-and-the-clock-on-the-newest-that-sounds.md)):
`MusicSwitch`, `AudioQueue::play_music_with`, `MusicCommand::Start`'s
`gain`, and `MusicState::heard`, which holds a crossfade's old track
until the new one sounds. 2341's PR passed its size limit (about 1,100
changed lines, mostly `MusicState`'s tests), so its own **Size** rule
sent two pieces here: the `Speaker`'s changes and the Harness test with
a load delay.

What is there today:

- `crates/platform/src/headless.rs`: the `Speaker` (the headless
  backend's sound device, which the Harness plays on) remembers only the
  track started last (`started: Option<(String, Millis)>`). In a
  crossfade the new track's `Start` replaces the old one, so after the
  load delay `music_playing` names the new track, but during the delay
  it names nothing, where `shuyi-app`'s player keeps naming the old
  track. It keeps no gains.
- `crates/ui/tests/it/audio.rs`: the `Beeper` screen crossfades to
  `drums` over 1 s on `Action::Menu`
  (`play_music_with("drums", MusicSwitch::crossfade(1000))`), and
  `a_crossfade_plays_both_tracks_until_the_old_one_has_faded` checks the
  crossfade in the commands (`Harness::music_log`) and the clock, with
  no load delay.
- `shuyi-app`'s `Audio::music_playing` (`crates/app/src/audio.rs`)
  reports the track with the latest `started_at`: the newest that
  sounds.

## Nick input

None.

## Scope

**In:**
- **The `Speaker`** keeps every track started and not stopped, oldest
  first, with its start time and its last gain (from `Start` and
  `Gain`): `tracks: Vec<(String, Millis, f32)>`, and the time of the last
  frame (`now: Millis`) set in `play`.
  - `Start { cue, gain, .. }` drops a track of that cue and pushes it
    with `now` and `gain`; `Gain` changes that track's gain; `Stop` and
    `Load` drop it; `stop_all` clears them all.
  - `music_playing(now)` reports the newest track that sounds (its
    `now - start` past the load delay), else the one before it, so a
    slow load in a crossfade keeps the old track's clock. `set_silent`
    still makes it `None`.
  - a new `pub fn playing(&self) -> Vec<(&str, f32)>`: the tracks
    sounding at the last frame's time, oldest first, with their gains.
    Empty when silent.
- **The `Speaker`'s doc** says it plays every track started and not
  stopped, as the platforms' players do.
- **A Harness test with a load delay** in `crates/ui/tests/it/audio.rs`:
  `h.music_load_delay(0.3).keys("d").wait(1.0).act(Action::Menu)`, then
  - after `wait(0.25)`: `h.platform().speaker().playing()` is only
    `theme` at 1.0, `h.music_clock()` names `theme`, and no music command
    came since the switch;
  - after another `wait(0.25)`: the clock names `drums` at 0.2 s
    (`sounds_at(&h, "drums", 0.2)`), and `playing()` has `theme` below
    1.0 and `drums` above 0, adding up to 1 (to 0.03: frame times are
    `f32`s);
  - after `wait(1.0)`: `playing()` is only `drums` at 1.0.
- The existing crossfade test without a delay also checks `playing()`
  halfway (both at 0.5, to 0.03).
- `docs/platform-backends.md`'s Harness paragraph (it says the
  `Speaker` stands in for the music player) says it plays two tracks in
  a crossfade.

**Out (do not do):**
- any change to `MusicState`, `MusicSwitch` or `shuyi-app`'s player
  (2341 did them);
- a public API break: `playing` is an addition, and the `Speaker`'s
  fields are private.

## Implementation steps

1. Read `crates/platform/src/headless.rs` (`Speaker`, its `apply`,
   `play` and `music_playing`) and its tests
   (`crates/platform/src/headless/tests.rs`), and
   `crates/ui/tests/it/audio.rs`.
2. Change the `Speaker` as the Scope says. A private helper
   `sounding(&self, now) -> impl DoubleEndedIterator<Item = (&str, f64,
   f32)>` (cue, seconds sounding, gain) serves both `music_playing`
   (`next_back`) and `playing`.
3. The unit test and the Harness tests below.
4. The doc in `docs/platform-backends.md`, then the gates.

## Acceptance criteria

- [ ] Every existing `Speaker` and Harness audio test passes unchanged.
- [ ] A unit test: with a 0.5 s load delay, track `a` started, then `b`
      started at gain 0 a second later: `playing()` is `[("a", 1.0)]`
      and `music_playing` names `a` until `b` has loaded; after `Gain`s
      of 0.5 each it is `[("a", 0.5), ("b", 0.5)]` and `music_playing`
      names `b`; after `a`'s `Stop`, `[("b", 0.5)]`; silent, empty.
- [ ] The Harness test with a 0.3 s load delay passes as the Scope says.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the `Speaker`'s `playing()` and its clock during an overlap
  (`crates/platform/src/headless/tests.rs`).
- Integration: a crossfade through the Harness with a load delay
  (`crates/ui/tests/it/audio.rs`).

## Completion notes

*(Filled in by the session that completes the ticket.)*
