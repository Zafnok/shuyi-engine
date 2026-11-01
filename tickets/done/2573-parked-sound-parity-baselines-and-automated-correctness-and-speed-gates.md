---
id: "2573"
title: "Establish sound parity baselines and automated correctness and speed gates before replacement"
type: infra
milestone: "Engine: in-house platform"
model: opus-5.5 / gpt-6.1-sol
effort: xhigh
status: done
blocked_by: ["2902", "3003", "3029", "2116"]
nick_input: none
completed: 2026-11-01
---

# 2573 — Sound parity gates before replacement

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

- [x] Actual old native Linux and current-loader browser PCM baselines pass in CI; their absence blocks 2210.
- [x] Known-good captures pass; silent/channel/rate/voice/loop faults and injected slowdown fail.
- [x] Repeated equal-work performance and explicit calibrated tolerances/budgets are saved; noise/stale results fail closed.
- [x] Platform evidence contracts block new port implementation until its reference capture/performance works; no unverified Mac/Windows cell passes.
- [x] The old shipping backend/dependencies remain; no replacement implementation ships here.
- [x] All gates in the `run-gates` skill pass.

## Tests required

Unit: alignment/comparator/result schema and malformed/stale artifacts.
Integration: real native/browser capture, good-vs-good, all fault fixtures,
paired timing and slowdown detection. Owned reference PCM remains available
for future regression tests. See [WASAPI loopback](https://learn.microsoft.com/en-us/windows/win32/coreaudio/loopback-recording)
and [OfflineAudioContext](https://developer.mozilla.org/en-US/docs/Web/API/OfflineAudioContext)
for the distinction between endpoint capture and an independent graph oracle.

## Completion notes

**What was done.** The first gate on leaving quad-snd works and is required in CI:
real output recorded from a PulseAudio null sink natively (ALSA routed to it) and
from Chromium on the real loader (Playwright's mute flag removed, a real click),
held to independent expected signals, to itself, and to paired equal-work speed
(`tools/sound-parity/README.md`,
[ADR-0067](../../docs/adr/0067-sound-parity-is-judged-from-a-real-sink-against-expected-signals.md)).
Nick unparked the whole chain, ahead of the MVP, and asked that giving up stay one
session at any stage until quad-snd is removed
([decisions](../../docs/decisions.md),
[ADR-0068](../../docs/adr/0068-the-sound-replacement-stays-reversible-at-every-stage.md)).

**Baseline runs.** Limits were calibrated from three independent runs of the job:
attempts 1 to 3 of CI run 41271510575, on `ubuntu-24.04`, Chromium 156.0.8078.4,
PulseAudio 16.1. The rule and each number are in `budget.ron`. A fresh run on data
they weren't made from (CI run 41275055017) passed every correctness cell, and no
median crossed its limit. Three of its speed cells were `inconclusive` on spread, a
noisy machine. Rather than widen a limit, an inconclusive result is measured again,
up to twice, while a failed one stands and every attempt's samples are kept. The
next fresh run (CI run 41276204176, the head before these notes) passed all of it on
its first measure: 90 cells passed on Linux and in Chromium, none inconclusive, and
`sound-give-up` passed.

The run after that (CI run 41277487852) failed one cell, native peak memory at 1.114
against itself, which is why peak memory is now read before the decodes: quad-snd
frees each on its mixer thread, so the decode loop made the peak follow that thread's
timing, 33 to 82 MB on identical code. No limit moved for it.

**Not yet exercised.** Because that run was quiet, the measure-again step in the job
has never actually run. If it is wrong it fails the job, the safe direction, and
the first noisy run will prove it.

**Evidence precision, as measured.**

- The backend against `oggdec` and against a WAV's own numbers: within 1 sample step
  on both platforms. Silence: exactly 0. Levels: worst 0.63 dB.
- Output latency on the null sink: about 49 ms natively (31 ms in `crossfade`), about
  30 ms in the browser. This is the sound server's, not a speaker's.
- Equal-work ratios: native decode within 1 % across runs, though raw times moved by
  half between machines (Ogg 168 to 255 ms), which is why only ratios are held. The
  noisiest are the browser's file reads and latency, with wider limits to match.

**What it found in today's sound.** quad-snd's native loops aren't gapless. Inside
the 4,096-frame block (93 ms) holding a pass's end, the next pass lands on top of it
at the wrong offset, doubling it to near clipping at the first boundary measured, or
the block falls silent, as at the second. It is the same on every run, and passes
otherwise land on their exact frame. Web output is gapless. Measured on test signals,
so it hasn't been listened to in the game. `loop` pins it, not as correct, and
`loop_gapless` holds a replacement to better; 3030 to 3032's output work inherits that.
No separate fix: quad-snd is what's being replaced.

**Port cells.** Windows and macOS are `unsupported`, and `port-ready` refuses them, a
CI step proving it. 3031 and 3032 establish their captures first, and hosted Mac
capture stays unproved. Playdate's SDK checks are unchanged, and GBA isn't required.

**Size, and the budget it refines.** About 4,300 authored lines beside the corpus,
the lock, tickets and docs, against the ticket's 1,000 to 1,400 and the usual 1,500 a
PR, so about three times over: 2,350 of `cargo xtask` and its tests (give-up among
them, 188 and its share of the tests, which Nick asked for during the ticket), 490 of
capture, 470 of probe, 540 of scenarios, footprint and budget. It wasn't split, since
no part proves anything alone. 2902's estimate had this as unmeasured; its report now
has the figure, and later stages should plan for it.

**Deviations.** An inconclusive speed cell is measured again in the job, up to
twice, where the ticket says only that noise fails: it still fails if the noise
persists. Performance runs order pairs a, b, b, a with the slowed runs after,
not interleaved three ways, which lined each side up with a run's parity. The
expected signals of a true pause, rate and region loop are data with no steps yet
(`future_*`): their stages give them steps. Peak memory is the process's: quad-snd
offers no count of live PCM, so bounded decoded memory starts with 3030.

**For Nick.** It adds a CI job of about 15 minutes on PRs that touch a sound path, and
a second that really gives up and builds what is left. Nothing about the game's sound
changed.

Book: none, `sound-probe` is a tools-only feature that no game turns on, and nothing a
game uses changed.

No break: the old backend, its dependencies and its defaults are unchanged. `cargo
xtask sound-parity give-up --apply` leaves code, CI and `Cargo.lock` as they were.
