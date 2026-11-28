---
id: "2296"
title: "Seek, position and rate of a playing sound or track: jump to a time, read where it is, and change its speed with a ramp, by handle"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: ["3239"]
nick_input: none
completed:
---

# 2296 — Seek and rate of a playing sound

## Context

2344 (done) controls a looped sound by handle: start, stop, fade,
volume. 3239 lets music start partway through a track. Nothing jumps
within a track that is already playing, reads a looped sound's
position, or changes how fast something plays while it plays; 2379
(parked) only picks a pitch once per play. The Bevy coverage audit of
2026-11-28 (`docs/ROADMAP.md`, *Bevy's game coverage*) found the games
that need them: sound-test and jukebox screens, rhythm practice that
jumps to a section, a visual novel replaying a voiced line, music that
speeds up as a timer runs low, and tape-stop or slow-motion effects.
Bevy's sinks have `try_seek`, `position` and `set_speed`
(`crates/bevy_audio/src/sinks.rs`; `examples/audio/audio_control.rs`).

The in-house mixer (3030, done) has a resampler
(`crates/sound/src/mix/resample.rs`) and sources that rewind
(`Source::rewind_to`, `VorbisSource::seek_exact`).

## Nick input

None.

## Scope

**In:**
- `AudioRequest::SeekMusic { at_ms, fade_ms }` and `SeekSound { id,
  at_ms, fade_ms }`, through `rewind_to` and `seek_exact`, with an
  optional short fade so the jump doesn't click.
- **Position:** the scene's audio section reports each handle's
  position in whole milliseconds, beside the music clock.
- **Rate:** `AudioRequest::SoundRate { id, permille, over_ms }` and a
  music twin: an integer ratio (no floats reach a game's state), ramped
  in whole milliseconds as fades are, applied by the mixer's resampler.
  The music clock (2518) advances by the rate.
- Seeks and rates are presentation requests, recorded at the platform
  boundary like the music position (2518).
- A backend that can't seek or change rate refuses with a problem code;
  the docs list which can.

**Out (do not do):**
- time-stretching that keeps the pitch (rate changes pitch with speed);
- a pitch range picked per play from cue rules (2379, parked);
- seeking a sound a game generates (2295 has no rewind).

## Implementation steps

1. Read 2344's handles, 3239's start offset, the mixer's voices and
   resampler, and 2518's records.
2. Add the seek requests with the fade.
3. Report positions in the scene's audio section.
4. Add the rate requests and the ramp in the resampler; advance the
   music clock by the rate.
5. Add the problem codes and the docs.

## Acceptance criteria

- [ ] A seek to 2,000 ms then a read of the position gives 2,000 ms
  within one mixer chunk.
- [ ] A rate of 2,000 permille plays a 1-second clip in 500 ms in the
  headless `Speaker`.
- [ ] A ramp from 1,000 to 500 over 200 ms changes the rate in steps of
  whole milliseconds, ending exactly at 500.
- [ ] A replay of a session record with seeks gives the same music
  positions.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] Guides in `docs/guides/` updated, if this adds a direction, a module or a move (ADR-0009 §5).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each request, the ramp.
- Property: a seek to any time inside a track then reading gives that
  time within a chunk.
- Snapshot / integration: `Speaker` goldens; a record replay.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
