---
id: "2206"
title: "Parked: MIDI: the synth, cues that play a sequence with a bank, playback in shuyi-app, and a wizard part with an example"
type: feature
milestone: "Engine: MIDI music"
model: opus-5.5
effort: high
status: blocked
blocked_by: ["2205", "2342", "2344"]
nick_input: sign-off
completed:
---

# 2206 — MIDI: the synth and playback

> **Parked** (Nick, 2026-10-17, asked whether MIDI should be parked because the game doesn't need it, chose "Parked"): after the engine reaches parity with the game (the MVP in `docs/ROADMAP.md`) and after 2205. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

[ADR-0014](../../docs/adr/0014-midi-music-sequences-and-a-sound-bank-in-a-module.md)
§3 and §4. 2205 reads `.mid` and `.sf2` files into a `Sequence` and a
`SoundBank`. This ticket makes them sound:

- a pure synth in `shuyi-midi` that renders samples, with no I/O and no
  clock;
- cues (ADR-0004 §6) that play a sequence with a bank, started and
  stopped by a `SoundId` handle like a looped sound cue (2344's
  `start_sound` and `stop_sound`);
- `shuyi-app` feeding the synth's samples to the audio device.

The Playdate plays MIDI with its SDK's own synth; that belongs to the
Playdate backend (3010), not here.

## Nick input

**Sign-off:** listen to the example's music in the web build, and say
whether it is what you meant by MIDI sound.

## Scope

**In:**
- **`Synth`** in `shuyi_midi::synth`, over `rustysynth` (checked in
  2205): `Synth::new(&SoundBank, sample_rate)`, `play(&Sequence)`,
  `stop()`, `set_tempo_percent(u16)`, `mute_track(u16, bool)`, and
  `render(&mut self, left: &mut [f32], right: &mut [f32])`. No I/O, no
  clock, no `unsafe`.
- **Cues:** the cue manifest (ADR-0004 §6) gains a kind, `Sequence {
  sequence, bank }`, beside recorded files. Like a recorded music cue, a
  sequence cue takes an optional `group` (2342), `music` by default, and
  plays at its group's gain, ducking (2343) included. Logic starts and
  stops it by its `SoundId` (2344), and changes tempo and muting through
  cue commands, so logic stays deterministic. The Harness reports a
  started sequence cue like any sound (1129's `run` lists sounds
  started).
- **`shuyi-app`**, behind its own `midi` feature: it owns a `Synth` per
  playing sequence cue and fills the audio device's buffer from it.
  macroquad's audio plays whole sounds, so check its API on the day; if
  it can't take a stream, render the next second ahead into a buffer and
  queue it, and record the delay this adds.
- **Budgets:** measure the synth's cost with `shuyi bench` (3008) if it
  is done, and record it in the README's cost section. If 3008 isn't
  done, measure with a plain timing test and say so.
- **A wizard part,** if 2501 is done: `music-midi`, when `music` has
  `midi` picked, adds a short CC0 `.mid` of our own, a cue for it, and
  the bank line in `shuyi.ron` the README describes. If 2501 isn't done,
  add this part to 2501's built-in template list in this PR.
- **An example**, `examples/midi-music/`: a screen that plays a sequence
  and lets the player change its tempo and mute a track through actions
  (never keys: the `input-actions` skill). It has a Harness test, a
  replay test, and a `frame-png` picture (ADR-0009 §4).
- **The guides:** if 2004 is done, each direction guide's "modules" list
  gains MIDI as an option for music.

**Either order:**
- **2343** (ducking): if it is done, a rule that lowers `music` lowers a
  sequence cue in `music` too, through the gains it already plays at;
  if not, nothing here, since 2343 multiplies into those same gains.

**Out (do not do):**
- the Playdate's MIDI (3010 decides it when it is worked);
- MIDI devices in or out;
- effects beyond what `rustysynth` gives (reverb, chorus);
- a music editor.

## Implementation steps

1. Write `Synth` and its offline render test.
2. Add the cue kind and its commands.
3. Feed `shuyi-app`'s audio from it, behind the feature.
4. Measure the cost.
5. Write the example and its tests, and the wizard part.

## Acceptance criteria

- [ ] Rendering the fixture sequence offline matches the checked-in
      sample hash on Linux, and is within the stated tolerance on
      Windows, macOS and the web (ADR-0014 §4).
- [ ] The example plays in the desktop and web builds, and its Harness
      and replay tests pass.
- [ ] A Harness run of the example lists the sequence cue as started.
- [ ] The README's cost section has the measured numbers.
- [ ] If 2502 is done: `shuyi new demo --no-input --music midi` makes a
      game that builds and plays the template's sequence.
- [ ] Nick's sign-off is recorded in the Completion notes.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `Synth` controls (stop, tempo, mute) change the rendered
  samples as expected on the fixture.
- Snapshot: the offline render hash.
- Integration: the example's Harness, replay and `frame-png` tests; the
  wizard game if 2502 is done.

## Completion notes
