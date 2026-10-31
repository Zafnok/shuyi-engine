---
id: "2214"
title: "In-house sound: Vorbis, part 3: the inverse MDCT, windows and overlap, a streaming decoder, and the ogg format"
type: feature
milestone: "Engine: in-house platform"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: done
blocked_by: ["2213"]
nick_input: none
completed: 2026-10-31
---

# 2214 — In-house sound: Vorbis, part 3: the MDCT, the decoder and the format

> **Closed as dropped by 2902 (2026-10-31):** the conditional hybrid uses library codecs; this original clean-room implementation was not built.

## Context

The last of three Vorbis tickets (2212 lists them). 2213 gives each
channel's spectrum. This ticket turns spectra into samples, wraps the
whole decoder in an API a player can stream from, and registers `.ogg`
with the engine's formats (ADR-0007,
[ADR-0019](../../docs/adr/0019-the-format-registry-kinds-are-types-a-format-reads-one-file.md)).

Written from the Xiph *Vorbis I specification*, §1.3.2 (the window shape),
§4.3.7 to §4.3.9 (the inverse MDCT, overlap-add and returning samples),
and §A (Ogg mapping: granule positions and the last page's end trim).
Never from another decoder's source
([ADR-0021](../../docs/adr/0021-the-platform-layer-in-house-clean-room-pieces-in-their-own-crates.md)
§2).

Today, quad-snd decodes a whole track inside `Sound::load` (80–580 ms per
track, `crates/app/src/audio/native_music.rs`), so the game decodes on a
worker thread. A decoder that streams fixes that at the root: a player
decodes a few packets at a time.

## Nick input

None.

## Scope

**In:**
- **The inverse MDCT** for each block size (64 to 8192), as the spec
  defines it, computed with an FFT-based method we write from a textbook
  or paper (name it in the completion notes). Floats, with `libm` for
  `sin` and `cos` so it builds without `std`. Twiddle tables built once
  per block size, at decoder setup.
- **Windows and overlap-add** (§1.3.2, §4.3.7, §4.3.8): the Vorbis window
  for long and short blocks, and the four long/short transitions.
- **`shuyi_sound::vorbis::Decoder`**:
  - `Decoder::new(bytes: &[u8]) -> Result<Self, VorbisError>` (reads the
    headers);
  - `rate()`, `channels()`, `comments()`;
  - `frames() -> Option<u64>`: the length from the last page's granule
    position;
  - `decode_into(&mut self, out: &mut [i16]) -> usize`: fills `out` with
    interleaved samples, returns the frames written, 0 at the end;
  - `rewind()`, for looping music;
  - `decode_all(bytes) -> Result<Pcm, VorbisError>` for sound effects.
- **The first packet's discarded half and the last page's trim**, so the
  sample count equals the granule position (§A.2).
- **The `ogg` format in `shuyi-assets`** for the sound kind 2210 used,
  sniffed by `OggS` and the Vorbis identification header.
- **Speed.** Measure decoding the largest fixture, natively, with
  `cargo bench` or a timed test, and record the speed against the
  track's length in the completion notes. If it's slower than 20 times
  real time on Nick's machine, add the short-codeword lookup table that
  2212 left out.

**Out (do not do):**
- Playing it (3030–3032). `shuyi-app` keeps quad-snd until 3032.
- Seeking to a time (write a ticket when a game needs it; `rewind` is
  enough to loop).
- SIMD.

## Implementation steps

1. `src/vorbis/mdct.rs`, `src/vorbis/window.rs`,
   `src/vorbis/decoder.rs`.
2. Test the MDCT against a naive direct sum (O(n²)) for every block size,
   on random input, within 1e-4 relative error.
3. Windows and overlap-add, then the decoder loop.
4. **Expected output.** Decode each fixture once with an existing
   permissively licensed decoder **run as a black box** (ADR-0021 §2):
   for example, a throwaway program in the scratchpad that uses the
   `lewton` crate, which is still in the tree until 3032. Commit only its
   output (raw 16-bit samples, compressed if large) under
   `crates/sound/tests/fixtures/ogg/expected/`, with the fixture
   `README.md` saying how it was made. Never read that decoder's source.
5. The format in `shuyi-assets`.
6. In the completion notes, list the spec sections and the MDCT
   reference used, and the measured speed.

## Acceptance criteria

- [x] ~~Each fixture decodes to the same number of frames as its expected output, and every sample is within ±2 of it (rounding differences between float paths).~~ (not done: original codec work dropped by 2902.)
- [x] ~~`decode_into` with output buffers of random sizes gives the same samples as `decode_all`.~~ (not done: original codec work dropped by 2902.)
- [x] ~~`rewind` then decoding again gives the same samples.~~ (not done: original codec work dropped by 2902.)
- [x] ~~`shuyi-sound` still builds without `std`.~~ (not done: original codec work dropped by 2902.)
- [x] ~~The PR body says the code was written from the Vorbis I specification.~~ (not done: original codec work dropped by 2902.)
- [x] ~~All gates in the `run-gates` skill pass.~~ (not done: original codec work dropped by 2902.)

## Tests required

- Unit: the MDCT against the naive sum; the window values against the
  spec's formula; each block transition.
- Property: random output buffer sizes (above); decoding never yields more
  frames than `frames()`.
- Fuzz-style: random bytes after valid headers, and fixtures with bytes
  flipped, never panic (ADR-0007).
- Integration: an `.ogg` loads through the format registry.

## Completion notes

Dropped, not implemented, by 2902 on 2026-10-31. Nick selected library
codecs with an owned mixer, conditional on automated correctness/speed
gates (2573), staged migration and final-only removal (3079). 2210's
hound/lewton adapters supply decoding/container behavior; 2397 reads loop
comments and 3030/3031 supply bounded streaming. No original decoder code
or its implementation tests were written. The struck criteria preserve
the rejected scope without claiming it was delivered.
