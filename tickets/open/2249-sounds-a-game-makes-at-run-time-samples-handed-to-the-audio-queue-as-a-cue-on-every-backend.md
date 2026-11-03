---
id: "2249"
title: "Sounds a game makes at run time: samples handed to the audio queue as a cue, played on every backend"
type: feature
milestone: "Engine: demos"
model: sonnet-5 / gpt-6.1-terra
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2249 — Sounds a game makes at run time

## Context

Nick, 2026-11-03 ([`docs/decisions.md`](../../docs/decisions.md),
*Bad Apple and DOOM*). DOOM (2780) keeps its sound effects inside the
player's WAD file: about a hundred lumps of 8-bit unsigned samples,
mostly at 11,025 Hz. They are read when the game starts, from a file
the engine has never seen, so they can't be cues in the audio manifest
that `shuyi bake` checks.

What there is:

- a game plays a sound by naming a cue of its audio manifest
  (`crates/ui/src/audio.rs`, `AudioRequest::PlaySound { cue, volume }`
  and `StartSound`, through `ctx.audio`, `AudioQueue`);
- each backend loads the manifest's files: `shuyi-app` through quad-snd
  or the in-house sound (`crates/sound`, whose `Mixer::add_sound`
  takes a `SharedPcm` at run time already, `crates/sound/src/mix.rs`;
  3084 makes it the default), the Playdate (3029), and the headless
  speaker the harness listens with;
- `shuyi_sound::pcm::Pcm::new(format, samples: Vec<i16>, limits)`
  holds 16-bit samples at any rate; the mixer resamples (3030).

Other games that need this: a game that loads its mods' sounds, a
synth toy or a music maker, a game that renders a sound from a seed
(2225 renders `.sfx.ron` at load, as an asset; this is the same
samples without a file), a voice recorder toy, an emulator.

## Nick input

None.

## Scope

**In:**
- **`AudioRequest::AddSound { cue, pcm }`** and **`RemoveSound { cue
  }`** (the enum is `#[non_exhaustive]`, so they are additive), with
  `AudioQueue::add_sound(cue, Pcm)` and `remove_sound(cue)`. After
  `add_sound`, `PlaySound` and `StartSound` of that cue play the
  samples as a cue of the manifest would, with the cue's volume 1 and
  no loop. A cue the manifest already has is refused (a problem in the
  log, never a panic): a game can't replace a checked cue by accident.
- **Limits**: at most 32 MB of samples added in all, and a sound no
  longer than 60 seconds; past them the request is refused and logged.
- **Every backend plays them:**
  - in-house sound: `Mixer::add_sound`;
  - quad-snd: the samples written as an in-memory WAV and loaded with
    `load_sound_from_bytes`;
  - the Playdate (`crates/playdate/src/audio.rs`): a sample made from
    the data through the SDK;
  - the headless speaker: it records the add and every play, so a
    harness test hears them; the session's scene lists the sounds
    added.
- **8-bit helper**: `Pcm::from_u8(rate, channels, &[u8], limits)`,
  the samples `(b − 128) × 256`, because 8-bit unsigned sound is common
  in old formats.
- **The docs** (the `document` skill): the sound chapter of the book
  and `docs/contract.md`'s scene section if the scene changes.

**Out (do not do):**
- Streaming generated sound (a synth fed every frame): a sound here is
  whole when it's added.
- Reading DOOM's sound lumps: the port does that (2780).
- Positional sound (2363, parked) and pitch per play (2379, parked).

## Implementation steps

1. Read `crates/ui/src/audio.rs`, `crates/platform/src/audio.rs` and
   `crates/platform/src/audio/player.rs`, `crates/sound/src/{mix,pcm}.rs`,
   `crates/app/src/audio.rs` and `crates/playdate/src/audio.rs`, and
   the headless speaker (`crates/platform/src/headless`).
2. Add the requests and the queue's methods; refuse a manifest's cue.
3. Each backend, with its test; then the headless speaker and the
   scene.
4. `Pcm::from_u8`, the limits, the docs, then the gates.

## Acceptance criteria

- [ ] A game adds a sound from samples and plays it, on the in-house
      sound, quad-snd, the Playdate simulator and the headless speaker.
- [ ] Adding a cue the manifest has is refused and logged.
- [ ] The limits refuse a sound too long and samples too many.
- [ ] A removed cue plays nothing and is logged once.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `Pcm::from_u8` on hand-worked bytes; the queue's requests in
  order; the refusals.
- Property: any samples within the limits, added then played on the
  headless speaker, are heard as the same samples.
- Snapshot / integration: a harness test adds a 100 ms tone, plays it
  twice and reads both plays from the speaker; the in-house mixer's
  output for an added sound equals that of the same samples as a
  manifest cue.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
