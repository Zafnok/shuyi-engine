---
id: "3030"
title: "Parked: In-house sound: the mixer in shuyi-sound: voices, volumes, fades, looping, streamed music and resampling, in integers"
type: feature
milestone: "Engine: in-house platform"
model: opus-5.5
effort: high
status: blocked
blocked_by: ["2210", "2214"]
nick_input: none
completed:
---

# 3030 — In-house sound: the mixer

> **Parked** (Nick, 2026-10-19, chose "Park until MVP" for the in-house platform layer, `docs/decisions.md`): after the engine reaches parity with the game (the MVP in `docs/ROADMAP.md`). Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Nick, 2026-10-19 (`docs/decisions.md`): sound goes in-house, replacing
quad-snd. quad-snd does two jobs: it mixes the playing sounds, and it
hands the mix to the system. This ticket is the mixing, in `shuyi-sound`
(2210), so it has no system code and runs everywhere. Sound output is 3031
and 3032.

[ADR-0021](../../docs/adr/0021-the-platform-layer-in-house-clean-room-pieces-in-their-own-crates.md)
§3 makes `shuyi-sound` `no_std`, so this mixer can also serve the Playdate
(3029) and the GBA (3027). 3027 plans "an integer software mixer in
shuyi-gba". The order between them:

- **If 3027 is done first**, move its mixer into `shuyi-sound` and grow it
  here.
- **If this is done first**, 3027 uses this mixer and keeps only the GBA's
  timing and its 8-bit output.

Written from first principles and textbook signal processing, never from
the source of quad-snd, rodio, cpal, kira or any other mixer (ADR-0021 §1).

## Nick input

None.

## Scope

**In:**
- **`shuyi_sound::mix::Mixer`**, fixed output rate and channels (stereo, or
  mono for small devices), with:
  - `play(sound, Params) -> VoiceId`, with `Params { volume, looped }`,
    where `sound` is a shared `Pcm` (sound effects);
  - `stream(source, Params) -> VoiceId`, where `source` is anything that
    implements `Source` (`fn fill(&mut self, out: &mut [i16]) -> usize`,
    `fn rewind(&mut self)`), so a `vorbis::Decoder` (2214) plays music
    without decoding it whole;
  - `set_volume`, `fade_to(volume, millis)`, `stop`, `is_playing`;
  - `render(&mut self, out: &mut [i16])`: mixes every voice into `out`,
    then removes finished ones;
  - a maximum number of voices, set at creation, so it never allocates in
    `render`.
- **Integers only.** Volumes are `u16` in units of 1/4096, the sum is in
  `i32`, and the result is clamped to `i16`. No floats, so the mixer gives
  the same samples on every target.
- **Resampling** of a sound whose rate is not the mixer's: linear
  interpolation in fixed point, with a test that the length comes out as
  expected. Better filters are a ticket if a game hears the difference.
- **Mono to stereo** and stereo to mono.
- **Real-time safety.** `render` takes no lock and does not allocate. The
  output threads of 3031 and 3032 call it from the system's audio
  callback. Commands reach it through a fixed-size queue (a single
  producer and a single consumer, `no_std`), written here without
  `unsafe`: `shuyi-sound` forbids it (ADR-0021 §5). Sounds are handed
  over as shared `Pcm`s ahead of time and named in commands by a
  `SoundId`, so a command is plain `Copy` data. If a safe queue can't be
  made real-time safe, move the queue to `shuyi-sound-out`'s `sys`
  module and say why in the completion notes.

**Out (do not do):**
- System output (3031, 3032).
- Effects (reverb, filters, panning by position): write a ticket when a
  game asks.
- The audio player's rules (cue volumes, variants, the music clock): they
  stay where 3029 puts them, on top of this.

## Implementation steps

1. `src/mix.rs`, `src/mix/voice.rs`, `src/mix/resample.rs`,
   `src/mix/queue.rs`, `src/source.rs`.
2. The queue first, with tests from two threads under `std` (a test-only
   feature), and `loom` only if a plain stress test can't show it right.
3. Voices, volumes and fades: a fade is a fixed number of frames with a
   step per frame, so its end is exact.
4. Streaming voices over `Source`, with `rewind` for loops.
5. Resampling.
6. A golden test: a fixed script of plays, fades and stops renders to a
   buffer whose hash is snapshotted (`insta`), on every target CI builds.

## Acceptance criteria

- [ ] Two sounds at full volume that overflow `i16` clamp, never wrap.
- [ ] A fade from 1.0 to 0 over 100 ms at 48 kHz ends at exactly 0 after
      4800 frames.
- [ ] `render` doesn't allocate (a test with a counting global allocator).
- [ ] The golden hash is the same natively and in the no-std build's test
      run (a host test of the same code without `std`).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: clamping; fades; looping a sound and a stream; voice limits;
  resampling lengths; mono and stereo conversions.
- Property: for random scripts of commands, `render` never panics and no
  voice outlives its sound unless looped.
- Snapshot: the golden render (step 6).

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
