---
id: "2249"
title: "Sounds a game makes at run time: samples handed to the audio queue as a cue, played on every backend"
type: feature
milestone: "Engine: demos"
model: sonnet-5 / gpt-6.1-terra
effort: high
status: done
blocked_by: []
nick_input: none
completed: 2026-11-05
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

- [x] A game adds a sound from samples and plays it, on the in-house
      sound, quad-snd, the Playdate simulator and the headless speaker.
      The Playdate on the SDK's fake and its shim compiled against SDK
      3.1.2's headers, not in the Simulator (notes).
- [x] Adding a cue the manifest has is refused and logged.
- [x] The limits refuse a sound too long and samples too many.
- [x] A removed cue plays nothing and is logged once.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

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

**Done.** A game makes a sound at run time with
`ctx.audio.add_sound(cue, samples)` and frees it with `remove_sound(cue)`
([ADR-0085](../../docs/adr/0085-sounds-made-at-run-time-samples-as-a-cue-through-the-queue-which-keeps-the-limits-and-a-provided-backend-method.md)).

- `shuyi-ui`: `audio::Samples` (16-bit, mono or stereo, any rate to
  192,000) with `new`, `from_u8` (`(b − 128) × 256`), `to_wav`, `frames`,
  `millis` and `bytes`; `SharedSamples` and `SamplesError`.
  `AudioRequest::AddSound` (non-exhaustive) and `RemoveSound`;
  `AudioQueue::add_sound`, `remove_sound`, `is_added`, `added_bytes`,
  `set_manifest` and `take_problems`; `AddSoundError`, `ADDED_BYTES_MAX`
  (32 MiB) and `ADDED_SECONDS_MAX` (60). The queue refuses a manifest's
  cue and the two limits, and drops a play of a removed cue, logging the
  first; `Game` puts its problems in the game's log (category `audio`).
- `shuyi-platform`: `SoundRequest::Add` (built with `SoundRequest::add`)
  and `Remove`; the `Player` plays an added cue as a manifest's, at its
  own volume 1; `Backend::add_sound` and `Backend::remove_sound`, both
  provided. The headless `Speaker` keeps the sounds added and every play
  of them (`AddedPlay`), and the Harness reads them (`sounds_added`,
  `added_plays`).
- Backends: quad-snd loads the samples as an in-memory WAV and deletes
  the sound on removal; the in-house mixer copies them into its bank and
  takes them out with a new hand-over job; the Playdate has the SDK make a
  sample from a copy in the SDK's own memory (`pd_sample_from_data`,
  `pd_sample_free` in the shim; `Sdk::sample_from_data`, `sample_free`,
  with defaults).

**Deviations.**

- **`Samples`, not `shuyi_sound::Pcm`.** The ticket named `Pcm` and
  `Pcm::from_u8`. `shuyi-sound` belongs to the sound replacement, which
  ADR-0068 keeps removable in one command until 3079 (parked), and a
  feature outside the chain may not use it, or giving up would take the
  feature along; the footprint test (`cargo test -p xtask`) fails on the
  new mentions too. So the samples are a small type of the engine's own in
  `shuyi_ui::audio`, with `from_u8` on it, and only the in-house backend,
  inside the chain, converts them to `Pcm`. The giving up was rehearsed in
  a scratch checkout of this branch: what is left builds and checks (the
  PR says).
- The ticket said the session's scene lists the sounds added. The Harness
  does (`Harness::sounds_added`), and the contract's `sounds` lists each
  play of an added cue as any cue's, but no answer of the contract lists
  the sounds added yet: that is a contract version and its schemas, so it
  is ticket 2582. `docs/contract.md` is unchanged.
- **The Playdate Simulator wasn't run.** The Simulator gives nothing back
  on Windows, and no game here adds a sound yet. The speaker is tested on
  the SDK's fake (the samples the SDK is handed, the free on removal and
  on replacing, an empty sound), and the shim's new calls were compiled
  against SDK 3.1.2's `pd_api.h` with MinGW's `gcc -fsyntax-only`. 2783
  (DOOM on the Playdate) is the first to hear one there.
- **quad-snd is checked on a device by hand, not in CI**: a test that
  opens a device can't run on CI's runners. A throwaway test added a
  100 ms tone through `Macroquad::add_sound`, played it on this machine's
  WASAPI output and removed it, with no error; it isn't committed. In CI
  the WAV is read back by the WAV adapter (`hound`, which quad-snd's
  `audrey` reads WAV with) as the same samples, in the in-house tests.
- A removal of a cue that was never added is a problem in the log too.

**Follow-up tickets:** 2582 (the contract's answers list the sounds
added).

No break: the requests' enums are non-exhaustive, the new backend and
SDK methods have defaults, and the Playdate's `Clip` is unchanged.

Book: chapter 2, *Games and screens*, has a passage on sounds made at
run time, with a snippet its test plays (`ring_a_made_chime`).
