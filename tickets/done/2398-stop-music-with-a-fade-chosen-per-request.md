---
id: "2398"
title: "Stop music with a fade chosen per request"
type: bug
milestone: MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: ["2341"]
nick_input: none
completed: 2026-11-04
---

# 2398 — Stop music with a fade chosen per request

## Context

1. **The game ticket:** `game#1087`,
   [the music fade-out adoption ticket](https://github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1087-camp-and-world-map-music-fade-out-over-three-seconds.md)
   (`game#1080` shipped the fade-ins and quiet intervals on 2341 alone).
2. **The job:** fade the current music out over a requested duration,
   leave a quiet interval, then fade another track in, without changing
   every other screen's default fade.
3. **No friction review: a missing capability.** The 2026-11-01 review
   found that [PR 174](https://github.com/Zafnok/shuyi-engine/pull/174)
   (2341) shipped per-request switches and fade-ins, but
   `AudioQueue::stop_music` still sends the unit `StopMusic` request.
   `MusicState::stop` always uses `self.default.out_ms`. A caller cannot
   request a three-second fade to silence when its default is half a
   second; a switch to a real track cannot represent the quiet interval.
4. **The generic shape:** a stop request carrying its own fade-out
   length, interpreted by `MusicState` into the existing gain and stop
   commands. Scheduling the quiet interval belongs to the caller.
5. **Another game:** an exploration game's scene pauses its background
   music slowly before silence, then starts a different theme after a
   timed reveal. It needs the same stop operation.

[ADR-0054](../../docs/adr/0054-music-switches-are-data-todays-fade-as-the-default-a-crossfade-as-two-tracks-with-gains-and-the-clock-on-the-newest-that-sounds.md)
keeps the old stop fade as the default. Preserve that behavior while
allowing a request to override it.

## Nick input

None. The engine operation takes a duration; the game already owns its
timings and quiet interval.

## Scope

**In:**
- `AudioQueue::stop_music_with(fade_ms: u32)` and a matching
  `AudioRequest::stop_music_with` constructor for direct request users.
  Keep `stop_music()` and the existing unit `AudioRequest::StopMusic`
  unchanged. Add a `StopMusicWith { fade_ms }` request, a
  `#[non_exhaustive]` struct variant built through the constructor.
  The outer `AudioRequest` is already `#[non_exhaustive]`, so adding a
  variant preserves downstream exhaustive-match compatibility.
- `MusicState` uses the requested length for this stop only, cancels a
  track waiting to start, and fades both sounding tracks during an
  overlap. Keep ADR-0054's existing rule: duration is the time from full
  gain to silence; a track already at half gain takes half that time.
  Zero stops immediately on the next music update, as a zero default
  does. Repeating the same request does not restart its fade.
- Update the internal request matches (`AudioRequest::cue`,
  `MusicState::request`, and `game.rs`'s `is_known`) and test through
  `Game`/Harness as well as the state machine. Use existing
  `MusicCommand::Gain` and `Stop`; backends need no new command.
- Rustdoc and a small compiled example in the book's audio discussion.
  No manifest, pack, save or contract-schema change is intended.
- Engine first, then the game adopts this stop request in `game#1087`.
  Name that game PR in completion notes.
  Run the API gate; if the implementation introduces any actual break,
  add its `Breaking:` and migration notes rather than suppress the gate.
- About 350–650 changed lines, with tests. No new dependency.

**Out (do not do):**
- The game's schedule, its quiet interval and adoption (`game#1080`
  shipped the schedule and fade-ins; `game#1087` adopts this stop). The
  engine ticket does not depend on the game ticket.
- A music sequencer, per-track manifest defaults, different fade curves,
  sound effects, or changing any backend's mixer.

## Implementation steps

1. Read `crates/ui/src/audio.rs`, ADR-0054, and the existing stop,
   overlap, interrupted-fade and default-behavior tests.
2. Add the request and constructors while preserving the old unit
   request. Thread the selected duration into `MusicState::stop` and
   update its internal request matches.
3. Add focused regressions in the audio module and a toy screen test
   through `crates/platform/tests/it/`, reading music commands.
4. Update rustdoc and the book snippet, then run the gates.

## Acceptance criteria

- [x] With a 500 ms manifest default, a full-gain track stopped with
      3,000 ms has gain about 2/3 after one second and 1/3 after two,
      and emits one stop at three seconds (stepped-frame tests).
- [x] Legacy `stop_music()` and direct unit `StopMusic` still fade over
      500 ms in that fixture; the override leaves the default unchanged
      for the next track and stop.
- [x] Zero stops without a later track starting; a waiting next track
      is canceled and never gets a start command.
- [x] Both tracks in a crossfade stop, with at most the existing two
      tracks loaded; a half-gain track follows the existing fade pace.
- [x] Repeated identical stops do not postpone silence, and a stop of
      silence emits no load or start.
- [x] A toy screen's new queue request reaches the same gain/stop
      commands through the Harness; no backend change is required.
- [x] The old public request shape remains usable; the API verdict and
      any required migration are recorded without weakening the gate.
- [x] The book's chapter is updated (2802), or the completion notes have
      a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: explicit/default/zero duration, repeated stops, silence, a
  waiting track, and a stop during overlap or an interrupted fade.
- Integration: queue to `Game` to Harness music commands, with no sound
  device or wall clock; the quiet interval remains the test caller's.
- Compatibility: existing music tests pass with legacy requests unchanged.

## Completion notes

- `AudioRequest::StopMusicWith { fade_ms }` (a `#[non_exhaustive]`
  variant), built by `AudioRequest::stop_music_with`, and
  `AudioQueue::stop_music_with(fade_ms)`. `stop_music()` and the unit
  `StopMusic` are unchanged and still fade over the manifest's default.
- `MusicState::stop` takes the fade length: the default for `StopMusic`,
  the request's own for `StopMusicWith`. It aims every track at silence
  with `Track::aim`, so a track part-way down takes its share of the
  time (ADR-0054's rule), a repeated identical stop keeps the fade's
  progress, a waiting track is dropped, and 0 stops on the next update.
  No new `MusicCommand`; no backend changed.
- `AudioRequest::cue`, `MusicState::request` and `game.rs`'s `is_known`
  name the new variant.
- Tests: unit tests in `crates/ui/src/audio.rs` (3 s from full gain with
  a 0.5 s default, repeated stops, the default kept for the next track
  and stop, zero with and without a waiting track, silence, a stop during
  a switch's fade and during a crossfade), the property test's steps now
  include stops with their own fade, and a Harness test through a toy
  screen.
- Deviation: the Harness test is in `crates/ui/tests/it/audio.rs`, beside
  the other music Harness tests and their helpers, not in
  `crates/platform/tests/it/`; it drives the same `Game` → Harness music
  commands with no sound device or wall clock.
- Book: `games-and-screens.md`'s context section says how a screen asks
  for music and shows a compiled snippet (`site/snippets/src/screens.rs`,
  anchor `music`) with a test.
- API: an added variant of a `#[non_exhaustive]` enum and two added
  functions; no break. No ADR: this is ADR-0054's per-request switch
  applied to the stop.
- The game adopts it in `game#1087` (no game PR yet); that ticket's PR
  moves the pin to this change.
