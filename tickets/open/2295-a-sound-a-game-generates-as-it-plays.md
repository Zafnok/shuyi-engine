---
id: "2295"
title: "A sound a game generates as it plays: a sample source the mixer pulls each buffer, started and stopped by handle like a looped sound"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2295 — A sound a game generates as it plays

## Context

2249 (done) lets a game hand the audio queue samples it made at run
time, but its **Out** says "Streaming generated sound (a synth fed
every frame): a sound here is whole when it's added." The Bevy coverage
audit of 2026-11-28 (`docs/ROADMAP.md`, *Bevy's game coverage*) found
the games that need a sound with no end known in advance: synth and
tracker toys, sequencer puzzles, generative ambience, chiptune players.
Bevy lets a game stream its own samples through the `Decodable` trait
(`crates/bevy_audio/src/audio_source.rs`; `examples/audio/decodable.rs`).

The in-house mixer (3030, done) already pulls bounded chunks from a
`shuyi_sound::Source` (`crates/sound/src/source.rs`), which may report
no total length and an underrun. So the missing piece is a way for a
game's screen to hand one in and control it, and for every backend to
play it. The parked MIDI synth (2206) and DOOM's music (2784) could
then run on top of it.

## Nick input

None.

## Scope

**In:**
- `AudioRequest::StreamSound { id, source, group }` in
  `crates/ui/src/audio.rs`, where `source` is a boxed
  `shuyi_sound::Source + Send` the game writes. The source writes
  integer samples and never reads a clock, so the same calls give the
  same samples.
- It is stopped, faded, paused and has its volume set through 2344's
  handle requests by `id`, and it belongs to its group's volume (2342).
- **Backends:** the in-house mixer pulls it like streamed music. Where
  quad-snd is the backend, the app feeds it a chunk per frame from the
  source, or refuses with a problem code if that backend can't, which
  the docs list. The headless `Speaker` records its samples, so tests
  read the scene and the samples.
- It belongs to the presentation side: the rules only emit events, and
  a screen turns them into requests (ADR-0063).

**Out (do not do):**
- the MIDI synth itself (2206, parked);
- live input from a microphone;
- effects on one sound: 3238's buses do that.

## Implementation steps

1. Read `crates/ui/src/audio.rs`, `crates/sound/src/source.rs`, the
   mixer's stream path (`crates/sound/src/mix/stream.rs`), 2344's
   handles and 2249's queue.
2. Add the request; route it to the mixer as a streamed voice.
3. Feed quad-snd per frame, or refuse with the problem code.
4. Record it in the headless `Speaker`.
5. Write the docs page and an example: a sine tone whose pitch follows
   a slider.

## Acceptance criteria

- [ ] A source that yields a 440 Hz square wave plays through the
  headless `Speaker`, and its first 4,096 samples equal the golden.
- [ ] Stopping it by id with a fade ends it after the fade, and the
  source is dropped.
- [ ] An underrun from the source gives silence for that chunk, not a
  stop.
- [ ] The group's volume scales it.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] Guides in `docs/guides/` updated, if this adds a direction, a module or a move (ADR-0009 §5).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the request's routing, the underrun.
- Snapshot / integration: the `Speaker` golden; a Harness run of the
  example.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
