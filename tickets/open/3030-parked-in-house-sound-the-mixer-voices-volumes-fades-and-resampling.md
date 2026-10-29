---
id: "3030"
title: "Parked: In-house sound: the mixer in shuyi-sound: voices, volumes, fades, looping, streamed music and resampling, in integers"
type: feature
milestone: "Engine: in-house platform"
model: opus-5.5
effort: high
status: blocked
blocked_by: ["2210", "2214", "2116"]
nick_input: none
completed:
---

# 3030 — In-house sound: the mixer

> **Parked** (Nick, 2026-10-19, chose "Park until MVP" for the in-house platform layer, `docs/decisions.md`): after the engine reaches parity with the game (the MVP in `docs/ROADMAP.md`). Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title. It also waits for the sound research (2902), which may keep, change or drop it.

## Context

Nick, 2026-10-19 (`docs/decisions.md`): sound goes in-house, replacing
quad-snd. quad-snd does two jobs: it mixes the playing sounds, and it
hands the mix to the system. This ticket is the mixing, in `shuyi-sound`
(2210), so it has no system code and runs everywhere. Sound output is 3031
and 3032.

[ADR-0021](../../docs/adr/0021-the-platform-layer-in-house-clean-room-pieces-in-their-own-crates.md)
§4 makes `shuyi-sound` `no_std`, so this mixer can also serve the Playdate
(3029) and the GBA (3027). 3027 plans "an integer software mixer in
shuyi-gba". The order between them:

- **If 3027 is done first**, move its mixer into `shuyi-sound` and grow it
  here.
- **If this is done first**, 3027 uses this mixer and keeps only the GBA's
  timing and its 8-bit output.

2344 gives games sound handles (`SoundId`), and 2394 `pause_sounds`
and `Audio::suspend`, over quad-snd, which can only mute a sound it
holds.
This mixer's `VoiceId`, `looped`, `fade_to`, `stop`, `pause` and
`resume` back them once it replaces quad-snd.

Written from first principles and textbook signal processing, never from
the source of quad-snd, rodio, cpal, kira or any other mixer (ADR-0021 §2).

## Nick input

None.

## Scope

**In:**
- **`shuyi_sound::mix::Mixer`**, fixed output rate and channels (stereo, or
  mono for small devices), with:
  - `play(sound, Params) -> Result<VoiceId, Full>`, with `Params {
    volume, looped, rate, balance, region }`, where `sound` is a shared
    `Pcm` (sound effects). `rate` is a 16.16 fixed-point playback rate
    (1.0 by default) that the resampler applies: it scales the
    resampler's step, so pitch is cheap here (2345 left pitch to this
    mixer). `balance` is the per-voice balance below, and `region` the
    loop region of `stream`. A pitch range in the audio manifest's cue
    rules (2345) is 2379's (parked), once the desktop and the web play
    through this mixer (3031, 3032);
  - this list holds every need that 2902's needs list marks required, so
    the mixer is no smaller than the trait 3029 shares: `rate` (pitch,
    2345), `pause` and `resume` (2394), a stream's loop region (intro
    then loop), two streams at once for a crossfade (2341), and a voice
    limit that never steals a voice (2345's rule, 2378's limit). It also
    holds the balance 2902 notes for positioned sound (2363). If 2902's
    report marks another need required, it joins this list; a need it
    marks optional (effects, seeking a stream to any time) stays out;
  - `stream(source, Params) -> Result<VoiceId, Full>`, where `source` is
    anything that implements `Source` (`fn fill(&mut self, out: &mut
    [i16]) -> usize`, `fn rewind(&mut self)`), so a `vorbis::Decoder`
    (2214) plays music without decoding it whole. `Params::region` is an
    optional loop region (start and end in frames), so a stream plays its
    intro once and then loops the region with no gap. However the source
    gets back to the start frame (a decoder state saved when the stream
    first passes it, or a seek; 2214 has only `rewind`), say which in the
    completion notes;
  - `set_volume`, `fade_to(volume, millis)`, `stop`, `is_playing`;
  - `pause(VoiceId)` and `resume(VoiceId)`, so 2394's `PauseSounds` and
    `Audio::suspend` hold a sound where it is instead of letting it run
    on muted;
  - `render(&mut self, out: &mut [i16])`: mixes every voice into `out`,
    then removes finished ones;
  - a maximum number of voices, set at creation, so it never allocates in
    `render`. `play` or `stream` with every voice busy returns `Full` and
    never stops a voice on its own. Which sound loses is 2345's rule,
    applied in `App` before the mixer (2378), and the in-house backend
    (3031) gives the mixer at least as many voices as the cue rules let
    sound.
- **Integers only.** Volumes are `u16` in units of 1/4096, the sum is in
  `i32`, and the result is clamped to `i16`. No floats, so the mixer gives
  the same samples on every target.
- **Resampling** of a sound whose rate is not the mixer's: linear
  interpolation in fixed point, with a test that the length comes out as
  expected. Better filters are a ticket if a game hears the difference.
- **Mono to stereo** and stereo to mono.
- **Per-voice balance** on a stereo output: `set_balance(voice, left,
  right)`, `u16` in 1/4096 like the volumes, and `Params::balance` for
  the start (`(4096, 4096)`, centred, by default), so a positioned
  sound's pan (2363, whose `shuyi_ui::audio::spatial::pan_gains` gives
  the two gains in 1/65536; the backend divides them by 16) reaches the
  speakers. A mono output plays their mean. A test: a voice at `(4096,
  0)` sounds only on the left.
- **Real-time safety.** `render` takes no lock and does not allocate. The
  output threads of 3031 and 3032 call it from the system's audio
  callback. Commands reach it through a fixed-size queue (a single
  producer and a single consumer, `no_std`), written here without
  `unsafe`: `shuyi-sound` forbids it (ADR-0021 §6). Sounds are handed
  over as shared `Pcm`s ahead of time and named in commands by a
  `SoundId` (the mixer's own, not 2344's handle to a playing sound), so
  a command is plain `Copy` data. If a safe queue can't be made
  real-time safe, leave the queue out and say why in the completion
  notes: `shuyi-sound-out` doesn't exist yet, and 3031, which makes it,
  writes the queue in its `sys` module.

**Out (do not do):**
- System output (3031, 3032).
- Effects (reverb, filters, a limiter) and buses for groups: write a
  ticket when a game asks. Panning by position is 2363's (parked); this
  mixer only gives each voice the left and right volumes above.
- The audio player's rules (cue volumes, variants, the music clock) and
  the cue rules (2345: variants and minimum gaps; 2378: copy caps,
  priority, the voice limit): they stay where 3029, 2345 and 2378 put
  them, on top of this. So do the sound groups (2342) and ducking
  (2343): a voice's `volume` is its cue's volume times its group's gain
  (2342's `Gain`, already in this mixer's 1/4096), worked out before
  the mixer. The mixer never sees the group tree, and a change of a
  group's gain reaches it as `set_volume`.
- Sounds and music that follow a tick game's speed (2123's
  `GameSpeed`): a ticket when a game asks, whichever of 2123 and this
  lands first. `rate` and `pause` above are what it would use.

## Implementation steps

1. `src/mix.rs`, `src/mix/voice.rs`, `src/mix/resample.rs`,
   `src/mix/queue.rs`, `src/source.rs`.
2. The queue first, with tests from two threads under `std` (a test-only
   feature), and `loom` only if a plain stress test can't show it right.
3. Voices, volumes, fades, pause and resume, and the balance: a fade is
   a fixed number of frames with a step per frame, so its end is exact.
4. Streaming voices over `Source`, with `rewind` for whole-file loops,
   and the loop region.
5. Resampling and the playback rate.
6. A golden test: a fixed script of plays, fades and stops renders to a
   buffer whose hash is a row of a golden table (2116's
   `shuyi_assets::golden`, as a dev-dependency with `default-features =
   false` and `features = ["golden"]`), checked natively, in the no-std
   test run, and on wasm32 by CI's `determinism` job (add the crate's
   `-p` there). The test has `golden` in its name, as 2116 asks.

## Acceptance criteria

- [ ] Two sounds at full volume that overflow `i16` clamp, never wrap.
- [ ] A fade from 1.0 to 0 over 100 ms at 48 kHz ends at exactly 0 after
      4800 frames.
- [ ] `render` doesn't allocate (a test with a counting global allocator).
- [ ] The golden hash is the same natively, in the no-std build's test
      run (a host test of the same code without `std`), and on wasm32
      (CI's `determinism` job).
- [ ] Each need that 2902 marked required has a unit test. Examples: a
      voice paused for 4800 frames resumes at the frame where it
      stopped; a sound at rate 2.0 ends in half its frames.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: clamping; fades; looping a sound and a stream; a stream with a
  loop region renders its intro once and the region twice, sample-exact
  at the joins; pause and resume; rates; the balance; voice limits (a
  full mixer refuses a play); resampling lengths; mono and stereo
  conversions.
- Property: for random scripts of commands, `render` never panics and no
  voice outlives its sound unless looped.
- Golden: the golden render's row (step 6).

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
