# ADR-0067: Sound parity is judged from a real sink against expected signals

- **Status:** Accepted
- **Date:** 2026-10-31
- **Related tickets:** 2573, 2210, 3030, 3031, 3032, 2397, 3079

## Context

[ADR-0066](0066-sound-uses-library-codecs-and-output-with-an-owned-mixer-and-parity-gates.md)
makes a working correctness and speed gate the first step off quad-snd, and
lets an uncalibrated or unavailable gate block the plan. quad-snd offers no
way to read what it mixes, so only its real output can be compared. CI's
runners have no sound card, and Playwright mutes Chromium by default.

## Decision

- **Capture is real output on a virtual device.** PulseAudio runs in a
  temporary home with one null sink at 44,100 Hz, 16-bit stereo, and `parec`
  records its monitor. Natively ALSA's default routes to the sink through
  `~/.asoundrc`. Chromium runs as the full browser with `--mute-audio` removed
  and a real click for its autoplay rule. Fake callbacks and an
  `OfflineAudioContext` never count.
- **The probe is an example, behind a feature.** `shuyi-app`'s `sound-probe`
  feature names the backend for `examples/sound_probe.rs`, which drives the
  shared `Player` from `tools/sound-parity/scenarios.ron` and never ships.
- **Expected signals are independent of what they check.** The WAV corpus is
  generated from whole numbers by `crates/xtask/src/sound_parity/corpus.rs`,
  and Ogg expectations come from `oggdec`, whose output's hash `corpus.ron` pins.
  Neither lewton nor hound is an oracle, since a backend would then grade itself.
- **One alignment, no normalization.** A nonperiodic marker that differs by
  channel places a recording once. Tolerances are fixed in `budget.ron`.
  Every run proves the checks by breaking real recordings.
- **Rust judges, Python records.** The comparator and gate are `cargo xtask
  sound-parity`, tested with the repository's other tools. `capture.py` only
  drives processes and the browser, as `tools/web-check/check.py` does, and
  decides nothing.
- **Performance is paired and fails closed.** At least nine interleaved pairs per
  metric, judged by the median ratio and its interquartile range: too noisy is
  `inconclusive`, and that fails. An injected slowdown must fail too. Limits come
  from calibration runs on the pinned image, and `budget.ron` records them.
- **The matrix is the contract.** A mandatory cell not `passed` fails, stale or
  silent evidence included. Windows and macOS are `unsupported` until their
  tickets establish a capture, and `port-ready` refuses them until then.
- **The job runs on what a sound path can feel**, by the `sound` output of
  `changes.yml`, not on every PR: it records for several minutes.

These tools (PulseAudio, `vorbis-tools`) run in CI and on developers' machines
only. Nothing of theirs is linked or shipped, and the Ogg fixtures are our own
signals, so ADR-0002 §8, on what ships, isn't touched.

## Consequences

Every replacement stage has one place to add its scenarios and a verdict it can't
talk its way past. A runner image's change can move the numbers and fail a
sound PR until calibration is redone, which is deliberate. The evidence stops at
the sound server: speakers remain the device sign-offs of 3031, 3032 and 3079.

## Alternatives considered

- **Comparing against stored old recordings alone:** it would freeze today's
  bugs as correct, and the old mute and full decode are exactly what must change.
- **A mixer callback or `OfflineAudioContext`:** neither proves anything reached
  an output, and quad-snd exposes no mixer output to read.
- **A percentage from 2902's standalone probes:** single runs on another machine,
  which ADR-0066 rules out.
- **`ffmpeg` to capture or decode:** a heavier tool with no gain over `parec` and
  `oggdec`, and its builds differ in licensing and decoders.
- **numpy for the comparator:** another pinned package for what a hundred lines of
  tested Rust do, in a language outside the repository's gates.
