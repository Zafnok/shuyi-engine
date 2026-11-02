---
id: "2210"
title: "Sound PCM/source core and permissive WAV/Vorbis decoder adapters"
type: feature
milestone: "Engine: in-house platform"
model: sonnet-5 / gpt-6.1-terra
effort: high
status: done
blocked_by: ["2201", "2902", "2573"]
nick_input: none
completed: 2026-11-01
---

# 2210 — Sound PCM/source core and decoder adapters

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
- **From 2573 (done):** its corpus and oracles are `tools/sound-parity/` and
  `crates/xtask/src/sound_parity/`; `oggdec`'s output is the expected PCM.
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

- [x] The core builds for `thumbv7em-none-eabihf` without std or engine dependencies.
- [x] Supported WAV/Vorbis fixtures and malformed input checks pass; limits never panic.
- [x] Optional codecs build with GNU and current-loader wasm; no denied license is selected.
- [x] 2573's decoder correctness/speed checks pass; quad-snd remains available and default.
- [x] It changes nothing the quad-snd path does.
- [x] All this PR adds is in `tools/sound-parity/footprint.ron`, and CI's `sound-give-up` job passes: giving up stays one command ([ADR-0068](../../docs/adr/0068-the-sound-replacement-stays-reversible-at-every-stage.md)).
- [x] All gates in the `run-gates` skill pass.

## Tests required

Unit/property: PCM conversions, checked arithmetic, random/truncated input,
channel/rate/size limits, bounded chunk reconstruction and metadata access.
Integration: format/header fixtures, no_std fence, 2573 decoder comparisons
and performance artifacts. Use owned/public permissive fixtures, no game assets.

## Completion notes

`crates/sound` (`shuyi-sound`) has `Pcm`, `PcmFormat`, `Limits`, `Error` and the
`Source` trait, `no_std` with `alloc`, plus the optional `wav` and `vorbis` adapters
over `hound` and `lewton`. No consumer uses them: only the probe and the gate do,
so nothing the quad-snd path does has changed, and quad-snd stays the default.

Licenses rechecked in the manifests: hound 3.5.1 Apache-2.0, lewton 0.9.4
MIT/Apache-2.0, ogg 0.7.1 BSD-3-Clause. `cargo deny check` passes.

What the work turned up, and how it differs from the ticket and the research:

- **lewton 0.9, not the research's 0.10.2.** 0.9.4 is what `audrey` and the Playdate
  exporter already build, so this adds no second copy.
- **lewton doesn't trim the end of a file with one page of sound.** The first decode
  of the one-second mono fixture gave 22,144 frames where the file says 22,050. The
  adapter reads the last page's granule position and cuts the excess, and then
  agrees with `oggdec` in length. A file whose last page belongs to another stream
  (a chained file) keeps what the library gives.
- **lewton's running position between pages isn't exact**: it read 448 frames behind
  in a test. `VorbisSource::position` counts its own frames and takes only a page's
  end from the library.
- **A library's panic is an error only where the target unwinds**, as in
  `native_music.rs`. `wasm32-unknown-unknown` can't, and the handoff to 3031 says so.
  The property tests over damaged files found no panic.
- **Old and new are compared through the oracle.** quad-snd exposes no decoded
  samples, so `decoder:*` holds the adapters to the same expected signals as the
  backend's recordings.
- **Browser speed isn't a ratio.** A browser decodes with its own code on its own
  threads, so old against new isn't equal work there. The ratio is recorded as a
  non-mandatory cell, and `decoder-share` is the verdict.

Measurements, CI run 41366247884 (artifact `sound-parity`, whole decodes of the
10-second `long`, new against old, 9 pairs):

| Platform | File | Old | New | Ratio | Spread | Of the sound's length |
| -------- | ---- | --- | --- | ----- | ------ | --------------------- |
| Linux | WAV | 21.1 ms | 7.8 ms | 0.37 | 0.008 | 0.08% |
| Linux | Ogg | 33.6 ms | 25.7 ms | 0.76 | 0.006 | 0.26% |
| Chromium | WAV | 25.0 ms | 12.0 ms | 0.53 | 0.125 | 0.12% |
| Chromium | Ogg | 35.3 ms | 44.7 ms | 1.26 | 0.044 | 0.45% |

All 37 cells of `decoders` passed. `budget.ron`'s limits are from this one run, and
hold in the runs that followed it on this PR. Ogg in the browser is the one place
new is slower than old: 3031 decodes off the main thread there or accepts that.
Locally, `oggdec` 1.4.2's output matched `corpus.ron`'s pinned hashes byte for byte.

The mutant that makes `next_packet` do nothing is excluded in `.cargo/mutants.toml`:
every test that reads a file hangs on it, which a shard reports as a failure.

Remaining consumers: 3030 (the mixer's `Source`s), 3031 and 3032 (the backends'
workers), 2397 (loop comments and position), 2725 (the tool, which ADR-0068 §0 says
waits for 3079, though its `blocked_by` names only this ticket).

No ADR: ADR-0066 decided the crate and its split, and `Source`'s shape is what 3030
asked for. The parked 2008 moves this crate with the rest.

Docs: rustdoc in the crate, the `README.md` crates row, `tools/sound-parity/README.md`,
the `run-gates` skill. `docs/formats.md` is unchanged, as no format or asset kind is.

Book: none, no game can name `shuyi-sound` yet: no backend plays through it, and it isn't in the `shuyi` crate.

No break: a new crate, and additions to `xtask`.

Giving up: `footprint.ron` holds all of it, and `crates/sound` goes whole.
