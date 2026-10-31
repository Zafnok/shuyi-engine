---
id: "2210"
title: "Parked: In-house sound: the shuyi-sound crate, plain PCM samples, and our own WAV reader"
type: feature
milestone: "Engine: in-house platform"
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: blocked
blocked_by: ["2201", "2902"]
nick_input: none
completed:
---

# 2210 — In-house sound: the shuyi-sound crate and our own WAV reader

> **Parked** (Nick, 2026-10-19, chose "Park until MVP" for the in-house platform layer, `docs/decisions.md`): after the engine reaches parity with the game (the MVP in `docs/ROADMAP.md`). Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title. It also waits for the sound research (2902), which may keep, change or drop it.

## Context

Nick, 2026-10-19 (`docs/decisions.md`): the platform layer goes in-house,
sound first, "with our own OGG/WAV readers". Today `quad-snd` 0.2.8 (last
release May 2024) decodes WAV and OGG through `audrey` and `lewton` (last
released January 2021), inside `Sound::load`
(`crates/app/src/audio/native_music.rs`, `crates/app/src/audio.rs`
`mod device`).

[ADR-0021](../../docs/adr/0021-the-platform-layer-in-house-clean-room-pieces-in-their-own-crates.md)
sets the rules: written from specifications, never from another
implementation's source (§2); a crate of its own that depends on no
`shuyi-*` crate (§4); the engine registers it as formats (ADR-0007,
[ADR-0019](../../docs/adr/0019-the-format-registry-kinds-are-types-a-format-reads-one-file.md)).

This is the first sound ticket: the crate, the sample type every later
ticket uses, and WAV. Ogg and Vorbis are 2211–2214, the mixer 3030, and
sound output 3031 and 3032.

## Nick input

None.

## Scope

**In:**
- **The crate** `crates/sound` (`shuyi-sound`), MIT OR Apache-2.0, `no_std`
  with `alloc`, and no dependency on any `shuyi-*` crate. A `std` feature,
  off by default, adds `std::error::Error` impls. Dependencies: `libm`
  only, if needed.
- **`Pcm`**: decoded sound as plain data: `rate: u32`, `channels: u8`
  (1 or 2; more is an error for now), and interleaved `i16` samples.
  Also `frames()` and `duration_millis()`.
- **The WAV reader**, `shuyi_sound::wav::decode(&[u8]) -> Result<Pcm,
  WavError>`, written from the RIFF/WAVE documents and
  `WAVE_FORMAT_EXTENSIBLE` (Microsoft Learn) and the IMA ADPCM
  description:
  - PCM, 8-bit unsigned, 16-, 24- and 32-bit signed, and 32-bit float,
    all converted to `i16` (rounded, clamped);
  - `WAVE_FORMAT_EXTENSIBLE` carrying any of those;
  - IMA ADPCM (format tag 0x0011), which the Playdate build already uses
    (3029);
  - chunks it doesn't know are skipped, including odd-length chunks with
    their pad byte;
  - `WavError` names what is wrong and where (offset in bytes).
- **`shuyi_sound::wav::header(&[u8])`**: rate, channels, frames and
  format without decoding, for a format's "size from the header" path
  (ADR-0019).
- **The format in `shuyi-assets`**: the `Sound` kind exists, a file's
  header read by the `Wav` and `OggVorbis` formats
  (`crates/assets/src/audio/formats.rs`). Make `Wav` read its header
  through `shuyi_sound::wav::header`, sniffed by `RIFF....WAVE`. Whether
  a decoded `Pcm` kind is added beside it is this ticket's call: today
  formats read headers, and the platform decodes as it plays.
- **Either order with 2225** (`.sfx.ron` sound effects): if 2225 is
  done and this ticket adds a `Pcm` kind, add a `.sfx.ron` format of
  it too, from `sfx::render`'s samples; if not, and this ticket adds
  one, write that step into 2225's scope. With no `Pcm` kind, nothing
  to do: 2225 plays a `.sfx.ron` as a WAV.

**Out (do not do):**
- Ogg and Vorbis (2211–2214), mixing (3030), playing (3031, 3032).
- Changing `shuyi-app`'s audio: the old path keeps working until 3032.
- More than two channels, or WAV writing.
- Opening the source of `hound`, `audrey`, quad-snd or any other WAV
  reader (ADR-0021 §2).

## Implementation steps

1. Add `crates/sound` with `Cargo.toml` (workspace lints, license, `no_std`
   via `#![no_std]` and `extern crate alloc`), a README in the module
   order the other crates use, and `src/lib.rs`, `src/pcm.rs`,
   `src/wav.rs`, `src/wav/adpcm.rs`.
2. Write `Pcm` and its conversions (`u8` to `i16`, `i24` to `i16`, `f32` to
   `i16`) with tests at the edges (minimum, maximum, zero, and NaN for
   float, which becomes 0).
3. Write the RIFF chunk walker, then `fmt ` and `data`, then the sample
   conversions, then IMA ADPCM blocks.
4. Make the test files (step 1 of *Tests required*), commit them under
   `crates/sound/tests/fixtures/wav/` with a `README.md` saying how each
   was made. Files we make are ours, so they need no entry in
   `THIRD_PARTY_ASSETS.md`.
5. Register the format in `shuyi-assets` (scope above).
6. Add `shuyi-sound` to the no-std job in CI (`.github/workflows/ci.yml`,
   the `run-gates` skill's no-std list).
7. In the completion notes, list the documents used, by name and link
   (ADR-0021 §2).

## Acceptance criteria

- [ ] `shuyi-sound` builds for `thumbv7em-none-eabihf` without `std`, and
      depends on no `shuyi-*` crate (`cargo tree -p shuyi-sound`).
- [ ] Each supported WAV variant decodes to the expected samples.
- [ ] A malformed WAV gives an error naming the problem, never a panic.
- [ ] The PR body says the code was written from the documents listed
      in the completion notes.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: one small WAV per variant (8, 16, 24, 32-bit, float, extensible,
  IMA ADPCM, mono and stereo), made by a test helper that writes the
  bytes by hand. Expected samples are written in the test.
- Property: any `Pcm` written by the test helper as 16-bit PCM decodes back
  to itself. Truncating a valid file at any byte gives an error, never a
  panic.
- Fuzz-style: random bytes after a valid `RIFF` header never panic
  (ADR-0007).
- Integration: the format in `shuyi-assets` loads a fixture through the
  registry.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
