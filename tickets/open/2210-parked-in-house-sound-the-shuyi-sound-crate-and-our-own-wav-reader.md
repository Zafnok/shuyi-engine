---
id: "2210"
title: "Parked: Sound PCM/source core and permissive WAV/Vorbis decoder adapters"
type: feature
milestone: "Engine: in-house platform"
model: sonnet-5 / gpt-6.1-terra
effort: high
status: blocked
blocked_by: ["2201", "2902", "2573"]
nick_input: none
completed:
---

# 2210 — Sound PCM/source core and decoder adapters

> **Parked**: Nick chose the gated hybrid in 2902 on 2026-10-31; its build chain remains parked until he starts it. Whoever unparks it sets `status: todo` and removes `Parked:`. The working automated baseline of 2573 must pass before implementation.

## Context

2902 selected library codecs and output with an owned small mixer, conditional
on automated speed/correctness parity and staged migration. See
[ADR-0066](../../docs/adr/0066-sound-uses-library-codecs-and-output-with-an-owned-mixer-and-parity-gates.md)
and its [research](../../docs/research/2902-sound-after-quad-snd.md).
This replaces the original clean-room WAV/Ogg/Vorbis plan; 2211–2214
were dropped, not implemented. The mixer is 3030, output 3031/3032,
and removal of quad-snd belongs only to 3079.

## Nick input

None.

## Scope

**In:**
- `crates/sound` (`shuyi-sound`), MIT OR Apache-2.0, independent of other
  engine crates. Its PCM/source core is `no_std` with `alloc`; `std` and
  codec features are optional. New growable public types use
  `#[non_exhaustive]` and constructors (ADR-0039).
- `Pcm`: rate, one/two channels and interleaved i16 samples, checked
  frame/duration arithmetic, conversion and resource limits; typed errors.
- Optional `std` hound/lewton adapters, defaults off, taking supplied
  bytes and producing PCM or bounded chunks. Recheck manifest and repository
  licenses (hound Apache-2.0; lewton MIT OR Apache-2.0). No I/O or device
  workers in this crate. Preserve Ogg comments and seek position facts
  needed by 2397; exact boundaries require pre-roll/trim, not page seek alone.
- Keep `Sound`/`MusicTrack` format registration and header validation pure
  in `shuyi-assets`; no new decoded asset kind is required. Preserve existing
  Playdate IMA ADPCM baking; do not claim hound decodes that variant.
- Add each crate/edge/codec dependency to `tools/layers.ron`, explicitly
  classifying hound/lewton as non-device decoders so tools such as 2725
  may use optional std adapters; the PCM/mixer core stays no_std.
- Extend 2573's corpus with decoder errors, limits and expected PCM; compare
  permitted formats against the old black-box path where supported and
  independent fixtures otherwise. Correctness/speed budgets must pass
  before any consumer uses this adapter. Keep the old backend available.

**Out (do not do):** mixing (3030), workers/output (3031/3032), loop cue
fields (2397), optional FLAC/MP3 formats/playback (2203/2234), removal
(3079), an original WAV/Vorbis decoder or reading implementation source
to copy it. Existing `.sfx.ron` work (2225) continues to produce WAV bytes.

## Implementation steps

1. Require 2573's working baseline; add crate/core and optional codecs.
2. Adapt public decoder APIs, preserving metadata and bounded chunk reads.
3. Wire formats/layers/docs and GNU/wasm/no_std builds without changing
   the default playback path.
4. Run the comparative corpus/performance gate and record its artifacts.

## Acceptance criteria

- [ ] The core builds for `thumbv7em-none-eabihf` without std or engine dependencies.
- [ ] Supported WAV/Vorbis fixtures and malformed input checks pass; limits never panic.
- [ ] Optional codecs build with GNU and current-loader wasm; no denied license is selected.
- [ ] 2573's decoder correctness/speed checks pass; quad-snd remains available and default.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

Unit/property: PCM conversions, checked arithmetic, random/truncated input,
channel/rate/size limits, bounded chunk reconstruction and metadata access.
Integration: format/header fixtures, no_std fence, 2573 decoder comparisons
and performance artifacts. Use owned/public permissive fixtures, no game assets.

## Completion notes

*(Record integration, measurements and remaining consumers when completed.)*
