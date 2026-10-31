---
id: "2573"
title: "Parked: Establish sound parity baselines and automated correctness and speed gates before replacement"
type: infra
milestone: "Engine: in-house platform"
model: opus-5.5 / gpt-6.1-sol
effort: xhigh
status: blocked
blocked_by: ["2902", "3003", "3029", "2116"]
nick_input: none
completed:
---

# 2573 — Sound parity gates before replacement

> **Parked**: Nick chose the conditional hybrid in 2902 on 2026-10-31; this and its build chain stay parked until he starts them. Whoever unparks it sets `status: todo` and removes `Parked:`.

## Context

Nick requires automated correctness and speed parity before progressively
leaving quad-snd, then a separate removal ticket. This is the **working
first gate**, not another research plan. No replacement implementation in
2210/3030/3031/3032/2397 begins before its common baseline passes in CI.
See [ADR-0066](../../docs/adr/0066-sound-uses-library-codecs-and-output-with-an-owned-mixer-and-parity-gates.md)
and [2902](../../docs/research/2902-sound-after-quad-snd.md).

## Nick input

None. A paid runner or new legal duty is a separate product decision;
do not assume one to make capture work.

## Scope

**In:**
- Keep this first PR about 1,000–1,400 changed lines excluding generated
  fixtures; it ships common Linux/browser baselines. Windows/macOS capture
  adapters are established first in 3031/3032, not all built here.
- A small tools-only probe over the existing backend/player and quad-snd's
  public APIs. Keep shipping dependencies/defaults and the old runtime.
  Never inspect/copy another library's implementation source.
- Owned/generated or public permissively licensed WAV/Ogg signals: silence,
  distinguishable stereo marker/ramp, two voices, intro/loop sections,
  mono/stereo and rates. Record provenance, encoder settings and hashes;
  never use game assets. Check licenses before adding tools or fixtures.
- Independent WAV sample/count oracles; Ogg decoded expectations from a
  pinned public black-box decoder, with lossy/source comparison tolerances
  distinguished from decoded PCM equality. Save format/version provenance.
- **Actual old emitted PCM in CI**, native Linux and original-loader
  Chromium. Implement an isolated virtual sink/monitor and ALSA routing,
  e.g. PulseAudio null sink, and prove it works. Chromium must emit audio:
  remove Playwright's default mute flag and unlock with a test gesture.
  OfflineAudioContext/fake callbacks alone do not prove output.
- Capture supported scenarios: onset/duration, two whole-loop boundaries,
  gain, one copy stopped/attenuated while another survives, two music
  voices/crossfade and current mute fallback. Match state/clock/events too.
  True pause/rate/region loops use independent future expected signals,
  not the old mute/full-decode behavior as a success oracle.
- Repeated old-vs-old calibration: one nonperiodic marker alignment,
  documented sample-format conversion, fixed timing/amplitude tolerances.
  No per-segment time stretching or gain normalization hiding faults.
  Silence, wrong/missing channel/rate/voice/loop, clipping and missing
  marker fail. Fault fixtures must prove these failures are detected.
- Release-mode performance: separate file I/O/context startup/full decode,
  then process CPU including worker/mixer threads, latency and underruns
  over the same captured playback duration. A timed sleep is not mixer
  throughput; quad-snd exposes no offline mixer PCM comparator. New offline
  rendering gets its own budget. Compare complete decode with complete
  decode, not old full load against a new first buffer.
- Warm up, then at least nine interleaved measured pairs per workload on
  pinned software/runner configuration. Save medians, spread, ratios,
  environment, sample counts, CPU, live PCM/memory, build time and stripped
  size separately. Calibrate old-vs-old variance and an injected slowdown;
  freeze a justified comparison/noise envelope. Inconclusive noise fails
  progress; no arbitrary percentage from 2902's standalone probes, no
  threshold widening just to pass. Include short/long stream memory tests
  as later stages arrive, with decoded PCM bounded by queue capacity.
- Machine-readable passed/failed/inconclusive/unsupported matrix and
  artifacts pinned to revision, fixture hashes, backend, platform/scenario
  and budget version. Every mandatory cell except passed exits nonzero;
  missing/stale/silent capture is failure, never a skip or silent fallback.
- Common native Linux/browser baseline enables portable 2210/3030 work.
  Define each port's mandatory evidence contract: actual Windows/macOS
  reference capture and performance must work **before that port's new
  output implementation/switch**, established first in 3031/3032 if needed.
  WASAPI loopback needs a real render endpoint; hosted Mac capture remains
  unproved. Unavailable cells block that port and cannot support a parity
  claim. Preserve unchanged Playdate SDK regressions; future GBA work is
  not required to finish this desktop/web harness.

**Out (do not do):** replacement codecs/mixer/output (2210,3030–3032),
loop cue fields (2397), removal (3079), handheld backend implementation,
manual-only or build-only parity, paid/denied capture drivers.

## Implementation steps

1. Add probe/corpus/provenance and independent expected-data self-tests.
2. Make native and actual-loader browser capture run in isolated CI audio.
3. Calibrate correctness/performance, implement fault/slowdown rejection,
   persist versioned results and required CI triggers for sound changes.
4. Document exact commands, capture limits/platform matrix and stage gates;
   refine the implementation budget with measured harness effort.

## Acceptance criteria

- [ ] Actual old native Linux and current-loader browser PCM baselines pass in CI; their absence blocks 2210.
- [ ] Known-good captures pass; silent/channel/rate/voice/loop faults and injected slowdown fail.
- [ ] Repeated equal-work performance and explicit calibrated tolerances/budgets are saved; noise/stale results fail closed.
- [ ] Platform evidence contracts block new port implementation until its reference capture/performance works; no unverified Mac/Windows cell passes.
- [ ] The old shipping backend/dependencies remain; no replacement implementation ships here.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

Unit: alignment/comparator/result schema and malformed/stale artifacts.
Integration: real native/browser capture, good-vs-good, all fault fixtures,
paired timing and slowdown detection. Owned reference PCM remains available
for future regression tests. See [WASAPI loopback](https://learn.microsoft.com/en-us/windows/win32/coreaudio/loopback-recording)
and [OfflineAudioContext](https://developer.mozilla.org/en-US/docs/Web/API/OfflineAudioContext)
for the distinction between endpoint capture and an independent graph oracle.

## Completion notes

*(Record actual baseline CI runs, evidence precision and blocked port cells.)*
