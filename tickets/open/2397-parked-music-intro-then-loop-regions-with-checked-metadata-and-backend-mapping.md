---
id: "2397"
title: "Music intro-then-loop regions with checked metadata and backend mapping"
type: feature
milestone: "Engine: audio"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: blocked
blocked_by: ["2902", "2573", "3031", "2013"]
nick_input: none
completed:
---

# 2397 — Music intro-then-loop regions

## Context

Whole-file looping cannot play an intro once and then repeat a selected
region. 2902 assigns this contract to the chosen hybrid sound path;
[ADR-0066](../../docs/adr/0066-sound-uses-library-codecs-and-output-with-an-owned-mixer-and-parity-gates.md)
requires automated parity before migration. 3030 supplies sample-level
mixing, 3031 workers/output; 3079 alone removes the old backend.

## Nick input

None.

## Scope

**In:**
- Optional `MusicCue.loop_start_ms`/`loop_end_ms`; both present or neither,
  checked `0 <= start < end <= length_ms`. Update RON/serde/validation,
  constructor/defaults, pack version/pinned binary fixtures and public API
  migration notes as needed. New growable public types follow ADR-0039.
- Read Ogg `LOOPSTART`/`LOOPLENGTH` sample comments via lewton (2210),
  checked addition/unit conversion and known sample rate. Manifest values
  override file defaults; errors name file/field. Do not silently clamp
  invalid/overflowing metadata or make a codec-specific manifest.
- `MusicClock` plays the intro once then wraps end to start, including
  pause/crossfade and elapsed spans across several loops. Preserve length
  meaning and document the half-open region/sample conversion policy.
- A provided shared-backend loop-region method, `Unsupported` by default;
  whole-file-only backends warn once and use their truthful fallback.
  Wire exact worker prefetch/pre-roll/trim and mixer boundaries, not a
  main-frame seek at the audible endpoint. Both crossfade streams can loop.
- Playdate SDK file-player loop ranges map with tested units. Baked PCM
  metadata retains the range; no desktop decoder on handheld is implied.
- **Either order with 3027:** if 3027 is done, map a GBA tracker restart
  point/order-row to this region with checked conversion; reject/report
  unsupported ranges and warn once rather than rounding arbitrary ms.
  If not, nothing to do (3027 implements its mapping).
- **Either order with 2365:** if 2365 is done, wrap `beats_since` and
  `bars_since` through the region and count the loop point once; if not,
  nothing to do (2365 adds that wrapping). Keep the reciprocal clause.
- Extend 2573's actual output and independent oracle tests: expected
  `intro + region + region`, exact sample indices, decoder-page boundaries,
  no added silence/duplicates, queue wrap and producer stalls. Correctness,
  CPU/speed and bounded-memory gates pass before selecting this behavior.
  3017 is still open and reads implemented audio features when it lands.
- Update platform contract, format docs and book; test asset registry/pack
  round trips. API breaks carry Breaking/migration notes and a game pin handoff.

**Out (do not do):** arbitrary seeking UI, stems/sample scheduling/latency
calibration, tempo reading, beat-clock implementation (2365), new GBA
backend (3027), deleting quad-snd (3079).

## Implementation steps

1. Extend cue/metadata validation and serialized forms, then backend method.
2. Implement clock wrap and stream/mixer exact boundaries; map SDK ranges.
3. Settle either-order mappings, extend automated gates and document migrations.

## Acceptance criteria

- [ ] Metadata/defaults/manifest precedence and invalid/overflow/range cases are checked.
- [ ] Intro plays once; at least two streamed loop boundaries have exact expected samples and bounded PCM storage.
- [ ] Clock and, when present, beat/bar queries wrap correctly through pause/crossfade.
- [ ] Unsupported fallback warns once; Playdate/GBA mapping is checked and truthful.
- [ ] 2573's correctness/speed gates pass without removing the reference backend.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

Unit/property: ranges, metadata units/overflow, multi-wrap clocks, independent
sample oracle and malformed comments. Integration: real native/browser
emitted loops, two streams/fades, pause/rate, ring/page boundaries,
underruns, SDK/tracker mapping, provided Unsupported warning, pack round trip.

## Completion notes

*(Record parity artifacts, target mappings, serialization/API migrations.)*
