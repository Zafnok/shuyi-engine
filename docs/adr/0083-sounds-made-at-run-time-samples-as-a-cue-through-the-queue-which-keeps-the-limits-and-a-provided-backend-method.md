# ADR-0083: Sounds made at run time: samples as a cue through the queue, which keeps the limits, and a provided backend method

- **Status:** Accepted; extends ADR-0063 (the shared player's `Backend`)
- **Date:** 2026-11-05
- **Related tickets:** 2249 (this), 2780 (the DOOM port, its first user),
  2225 (sound effects rendered at load, as an asset), 3079 (removing
  quad-snd, parked), 2363 and 2379 (positional sound and pitch per play,
  parked)

## Context

A game played a sound only by naming a cue of its audio manifest, which
`shuyi bake` checks and every backend loads at the start. Some sounds
exist only once the game runs: DOOM's, inside the player's own WAD file;
a mod's; a synth toy's; a sound rendered from a seed. Their samples are
already in memory.

The engine has a type for decoded samples, `shuyi_sound::Pcm`
(ADR-0066), and the ticket asked for it. But `shuyi-sound` is part of the
sound replacement, which ADR-0068 keeps reversible until quad-snd is
removed (3079, parked): `cargo xtask sound-parity give-up --apply` takes
the crate out, and "a ticket outside the chain that would use
`shuyi-sound` waits for 3079, so that giving up never takes another
feature with it". This feature is outside the chain and can't wait for a
parked ticket.

Three things had to be settled: what type the samples are and where it
lives, who checks what may be added, and how a backend that can't take
samples breaks nothing.

## Decision

1. **The samples are the engine's own type, in `shuyi-ui`:**
   `shuyi_ui::audio::Samples` (16-bit, interleaved, mono or stereo, at
   any rate from 1 to 192,000), with `Samples::new`, `Samples::from_u8`
   (8-bit unsigned samples as `(b − 128) × 256`, as old formats keep
   them) and `Samples::to_wav`, a 16-bit WAV in memory for a device that
   loads a sound from a file's bytes. `SharedSamples` shares them (`Arc`,
   or `Rc` without atomics). Nothing outside the chain names
   `shuyi-sound`, so giving the replacement up leaves this feature whole.
   The in-house backend, inside the chain, copies them into its mixer's
   `Pcm` when they are added.
2. **Samples travel as a request, in order.** `shuyi_ui::audio::AudioRequest`
   gains `AddSound { cue, samples }` (non-exhaustive) and
   `RemoveSound { cue }`; `shuyi_platform::audio::SoundRequest` gains `Add`
   and `Remove` to match. They go down the same list as the plays, so a
   play after an add finds its sound on every backend, in the same frame.
   The samples are shared, never copied on their way: the Harness's log
   of every frame's requests holds a pointer each. A record of a
   `SoundRequest::Add` writes the samples' rate, channels and frames, not
   the samples.
3. **The queue keeps the rules.** `AudioQueue::add_sound(cue, Samples)`
   refuses a cue the manifest has (`set_manifest`, which the context
   calls), a sound longer than `ADDED_SECONDS_MAX` (60 s), and samples
   that would make the sounds added hold more than `ADDED_BYTES_MAX`
   (32 MiB, two bytes a sample). Adding a cue added before replaces it,
   counted once. After `remove_sound`, the queue drops a play of the cue
   and logs the first one; a refusal is an `AddSoundError` and a line in
   the game's log. `Game` takes the queue's problems into the log each
   frame. The queue is pure and deterministic, so all of this is tested
   without a device, and every backend sees only what was allowed.
4. **The player treats an added sound as a cue.** `Player` keeps added
   sounds beside the manifest's; a play of one takes the cue's volume as
   1 and plays once (or as long as a handle lives). A manifest cue wins a
   name both have, which the queue already prevents.
5. **The backend method is provided** (ADR-0063's rule):
   `Backend::add_sound(&SharedSamples) -> Result<Sound, String>`, whose
   default says the backend can't, and `Backend::remove_sound(Sound)`,
   whose default stops and drops. Each of the engine's backends has it:
   - quad-snd: the samples as a WAV (`Samples::to_wav`), loaded as a file
     is; quad-snd resamples to its rate. On the web the browser decodes it
     in the background, and a play before it has is silence;
   - the in-house mixer: a copy in its bank, and a `Job::Unsound` that
     takes it out on the audio thread and sends it back to be freed on the
     game's, the place released only then;
   - the Playdate: `sample->newSampleFromData` on a copy the shim makes in
     memory of the SDK's own, which `freeSample` frees with it, so the
     debug build's heap counter never sees memory leave Rust's hands.
     `Sdk::sample_from_data` and `sample_free` have defaults, and an added
     sound is a `Clip::Sample` the speaker remembers it made;
   - the headless `Speaker`: it keeps the samples by cue and records each
     play's samples (`added_plays`), for tests.

## Consequences

- A game makes a sound with two calls, and a test reads back the very
  samples each play used.
- Giving up the sound replacement keeps the feature: quad-snd, the
  Playdate and the headless speaker read `Samples` directly.
- The in-house backend holds two copies of an added sound while the
  queue's request lives (the frame), then one. When 3079 removes quad-snd,
  `Samples` could become `shuyi_sound::Pcm` again (a break, with a
  `Breaking:` line), or stay: a later ticket's choice.
- A backend written outside the engine keeps building; its added cues play
  nothing, with a warning, until it writes `add_sound`.
- Memory is bounded per game by the queue, not by each device: a target
  with less memory than 32 MiB (the Playdate has 16) can still run out,
  and then its backend's `add_sound` fails with a warning rather than a
  crash.
- The Harness's audio log holds every added sound for the run, by
  pointer: a test that adds many large sounds keeps them alive until it
  clears the log.

## Alternatives considered

- **`shuyi_sound::Pcm` in the requests, as the ticket said:** it puts
  `shuyi-sound` under `shuyi-ui`, so giving up the replacement
  (ADR-0068) would take this feature with it, and its footprint test
  fails on the new mentions.
- **A new asset kind loaded at run time** (a format plugin that reads the
  bytes): the samples here never were a file, and a format plugin is for
  files the engine checks (ADR-0007).
- **The checks in the player, per backend:** each backend would log
  through its own channel (macroquad's log, the SDK's console), not the
  game's log the contract and the tests read, and the headless backend
  has no manifest to check against.
- **A required backend method:** it would break every backend outside the
  engine, against ADR-0063.
- **Handing the Playdate the Rust buffer** (`shouldFreeData = 1`): the SDK
  would free memory Rust's counting allocator handed out, and the debug
  build's heap counts would drift.
- **Streaming generated sound** (a synth fed every frame): out of the
  ticket's scope; a sound here is whole when it is added.
