# The sound parity gates

What must hold before and during the move off quad-snd (ticket 2573,
[ADR-0066](../../docs/adr/0066-sound-uses-library-codecs-and-output-with-an-owned-mixer-and-parity-gates.md),
[ADR-0067](../../docs/adr/0067-sound-parity-is-judged-from-a-real-sink-against-expected-signals.md)).
CI's `sound-parity` job runs all of it on a pull request or a push that changes
a sound path (`sound` in `.github/workflows/changes.yml`).

| File | What it is |
| ---- | ---------- |
| `scenarios.ron` | What is played, and the expected signals a recording must match |
| `corpus.ron`, `corpus/` | The hashes that pin the test signals, how the Ogg files were made, and those files |
| `budget.ron` | The fixed tolerances, the performance limits and each platform's evidence contract, with how each number was measured |
| `capture.py` | Records the real output and the paired timings |
| `wasapi.py` | Windows' recorder: WASAPI loopback, through the documented COM interfaces |
| `install-endpoint.ps1` | CI's real render endpoint on a Windows runner, which has none: Scream, pinned by its hash |
| `alsa-sys-held/` | Holds `alsa-sys`'s place, so that CPAL and quad-snd resolve in one build ([ADR-0075](../../docs/adr/0075-in-house-sound-output-cpal-beside-quad-snd-a-worklet-fed-by-messages-and-a-virtual-endpoint-for-windows-evidence.md)) |
| `ring_test.js` | The web sound plugin under node: its ring's indices, nothing lost as the context starts, and the start's lead over a busy page |
| `retry.py`, `retry_test.py` | Which inconclusive platforms CI's gate measures again, and its test |
| `capture_test.py` | Which recordings `capture.py` makes again: one the loopback lost frames from, or a scenario quad-snd underran in |
| `crates/app/examples/sound_probe.rs` | The probe: the shared player on the real backend |
| `crates/xtask/src/sound_parity*` | The corpus's numbers, the comparator, the mixer's expected samples and the gate |

## Run it

On Linux, with `pulseaudio pulseaudio-utils libasound2-plugins libpulse0
vorbis-tools`, and the web checker's Playwright for the browser half
(`crates/cli/web/README.md`, *Checked in CI*). It needs no sound card and
touches no other sound: PulseAudio runs in a temporary home, with one null
sink.

```bash
cargo xtask sound-parity corpus --out target/sound-parity --oracle
```

```bash
cargo build -p shuyi-app --example sound_probe --features sound-probe --release --locked
```

```bash
python3 tools/sound-parity/capture.py native --dir target/sound-parity --probe target/release/examples/sound_probe --revision "$(git rev-parse HEAD)"
```

A scenario quad-snd's own output underran in is never judged either: ALSA ran dry,
so the sink got a hole, not the sound. quad-snd prints `Underrun occured` (its
spelling) on its stdout when it does, and `capture.py` plays that scenario again, up
to three times, and fails the step if each underruns (ticket 2048), as on Windows
below. A timing isn't played again: the gaps it records are its `underruns` count.

```bash
cargo xtask sound-parity gate --dir target/sound-parity --revision "$(git rev-parse HEAD)"
```

`shuyi-sound`'s adapters need neither capture: this holds them to the corpus and
the oracle the first command wrote. The gate runs the same cells.

```bash
cargo xtask sound-parity decoders --dir target/sound-parity
```

The mixer needs neither capture nor oracle: this holds it to expected samples
worked out from the corpus's numbers. The gate runs the same cells.

```bash
cargo xtask sound-parity mixer
```

The browser half builds the same example for `wasm32-unknown-unknown`, puts it
beside the loader as CI's job does, and runs `capture.py browser --page <its
folder>`. The gate writes `target/sound-parity/matrix.json` and exits non-zero
unless every mandatory cell passed. On a machine with one platform's capture
it fails on the others': only CI, with all of them, can pass.

The Windows half is CI's `sound-parity-windows` job, whose recordings go to this one
to be judged. On a person's machine it refuses to record unless the default endpoint
already runs at 44,100 Hz, and it would play the test signals out loud: nothing makes
that needed. A recording the loopback lost frames from is never judged, so `capture.py` plays that scenario or timing
again, up to three times, and fails the step if each loses frames (ticket 2047).
Each refusal is a warning in the job's log, with how many frames went, where, and
how long the reader hadn't looked: as long as the half-second buffer, the recorder
was late; much less, the system dropped them.

Each run of packets the loopback flags silent is a line in the log too (ticket
2049). A run of exact zeros in the sound of quad-snd's native `loop`, at a join, is
quad-snd's own: on Windows it is none, 127 or 441 frames, as its mixing blocks fall.
So it is never refused, and the scenario marks its joins as `Boundary`s.

To see only the timings of a recording:

```bash
cargo xtask sound-parity perf windows --dir target/sound-parity
```

How long a play takes to reach the device's buffer, measured at no volume, so that
it can be run anywhere with nothing heard. It runs in a folder set up as the probe says.

```bash
cargo build -p shuyi-app --example sound_probe --features sound-probe,in-house-sound --release
```

`capture.py encode` remakes the Ogg files and `corpus.ron`, by hand on Linux,
never in CI. A change to the corpus's numbers fails until then.

## What is compared with what

- **The expected signals are independent.** A WAV's expected samples are the
  numbers that made it. An Ogg file's come from `oggdec`, a decoder apart from
  every backend: `corpus.ron` pins its output's hash, which is *decoded PCM
  equality*. `lossy_min_snr_db` is the different question of how far the lossy
  file may be from its source.
- **One alignment.** The marker, nonperiodic noise that differs by channel,
  places the whole recording once. Nothing is stretched per segment and no gain
  is normalized, so lateness, a wrong level, a swapped channel and a gap all
  show. The only conversion is 16-bit samples to a fraction of full scale.
- **Faults prove the checks.** Every run breaks real recordings in the ways
  below and requires the checks to catch each: `fault:silent`, `channel`,
  `rate`, `voice`, `loop`, `clipping` and `marker`.
- **Runs are compared with each other** (`repeat:<scenario>`), in levels over
  short windows away from where commands land. When a replacement arrives,
  it is held to the same expected signals and to old recordings the same way.
- **What today's backend does wrong is no oracle.** `future_pause`,
  `future_rate` and `future_region` hold what a replacement must do, as their
  own expected signals; on quad-snd they report `unsupported`. `mute` pins
  today's fallback and holds on quad-snd alone. `stream_gapless` holds a stream's
  loop to every frame of its join, and `voices_full` holds that a sound with no voice
  free takes none.

## The decoders

The cells of platform `decoders` hold `shuyi-sound`'s adapters to the same
expected signals as the backend's recordings, so old and new are compared through
them: quad-snd exposes no decoded samples to compare directly.

- `decoder:<file>`: a WAV's samples are exactly its numbers. An Ogg file has the
  oracle's length and is within `decoded_peak` of it. Both come out the same in
  chunks that split every packet, and `intro_loop.ogg` keeps its comments.
- `decoder-refuses:<case>`: each malformed file (`corpus::malformed_wavs`, a
  damaged Ogg file) and each file over a limit gives an error.
- `decoder-speed:<wav|ogg>`, on each captured platform, from `decoders.jsonl`:
  whole decodes of `long`, old and new in one process: each decodes once untimed,
  then they take turns old, new, new, old, with new first in every other process.
  Without both, a process's first decode and the one just after the other side's
  leaned the ratio by as much as its limit's spread (ticket 2044).
  Natively old and new are the same work on the same libraries, so the ratio is a
  verdict and `decoder-slowdown` must catch an injected one. A browser decodes
  with its own code on its own threads, seen to end at a frame, so there the ratio
  is written down and isn't one.
- `decoder-share:<wav|ogg>`: how much of the sound's length decoding it takes, on
  each platform: a decoder over its `max_share` can't feed a stream with room to
  spare. This is what holds the browser.

Whether a corrupt file's panic becomes an error depends on unwinding, which
`wasm32-unknown-unknown` lacks. There a panic in a library stops the program, so
the web's consumer decides what it hands them (3031).

## The mixer

The cells of platform `mixer` render to memory, so they are exact where nothing
real-time can be. quad-snd exposes no offline mix, so nothing comes from the old
path: expected samples are worked out in `xtask`, by plain arithmetic in floats
that shares no code with the mixer, and differ by at most a rounding step.

- `mixer:<case>`: `pause`, rates and resampling, `loop`, a region through the mix
  and through a worker's decoder, `two-streams` at their own gains, `gains`,
  `clamp`, `fade`, `stereo-on-mono`, and `capacity`, a play refused with what plays
  untouched.
- `mixer:memory`: the most frames decoded and not yet mixed, over a short stream and
  a long one, against `mixer_memory.max_held_frames`. A longer stream may not hold
  more.
- `mixer-fault:<fault>`: each must be caught: a silent, swapped, quiet or wrapped
  mix, a frame lost or late, and a stream decoded whole.
- `mixer-share`, on each captured platform, from `mixer.jsonl`: how much of a sound's
  length it takes to mix 32 busy voices in a release build, against `max_share`.
  With nothing old to compare with, this is what holds the speed. `mixer-slowdown`:
  the injected slowdown must show.

The mixer's own promises are its crate's tests: `render` allocates and frees nothing
(`tests/mix_alloc.rs`), the rings lose nothing between real threads
(`tests/mix_queue.rs`), and its golden table
(`tests/mix_golden.rs`, `crates/sound/tests/golden/mix.ron`) is the same natively,
without `std` and on wasm32.

## Performance

`capture.py` holds a monitor stream open for the whole session and starts nothing
until frames flow: an idle sink takes blocks of about 2 seconds, and a stream
arriving then stalls. It runs warm-ups, then at least nine interleaved pairs of the
same workload, measuring file reads and the context's start apart, then the process's
CPU over the same three seconds of playback, every thread's (the browser's whole
process tree), the output latency against the recording, gaps in the recording
while everything plays, and the peak memory, and last whole decodes. A third side injects a
slowdown. Decode is whole against whole: a new first buffer is never compared
with an old full load, and a timed sleep isn't mixer throughput, so offline
rendering needs its own budget when it arrives.

The verdict on each metric is the median of the paired ratios and their
interquartile range. Over the budget's `max_spread` it is `inconclusive`,
which fails like anything else not passed; over `max_ratio` it is `failed`.
CI's job measures an `inconclusive` platform again, up to twice, keeping every
attempt's samples as `perf-attempt-N.jsonl`: a noisy machine says nothing either
way. The Windows job does this for `windows` and `windows-in-house` before it
uploads them, so the gate never measures those again: an `inconclusive` cell of
theirs stands there, and their samples stay (`retry.py`). A `failed` cell is never measured again, and a slowdown shows in the median
however often it is. The injected slowdown must fail. Raise a limit only with new calibration runs
that justify it, written in `budget.ron`, never to pass.

## A new backend

`windows-in-house` and `browser-in-house` are `NewBackend`s in `budget.ron`. Their
recordings are held to the same expected signals, and their own timings to the same
kind of limits, every cell mandatory. `old-new.jsonl` holds the same workload on the
old backend and the new in turn, and each `old-new:<metric>` is judged by the median of
the paired ratios against `old_new` in the budget. The decoders and the mixer stay
measured on the old platform's row, being the same code.

## The matrix

`matrix.json` has one cell per platform and check, each `passed`, `failed`,
`inconclusive` or `unsupported`, pinned to the revision, the corpus's and
scenarios' hashes and the budget's version. A mandatory cell not `passed` fails
the gate. Evidence that is missing, silent, from another revision or from
another corpus fails: it is never a skip.

| Platform | Evidence today | Who establishes it |
| -------- | -------------- | ------------------ |
| Linux, native | Actual output on a null sink, ALSA routed to it | This gate |
| Chromium, the real loader | Actual output on the same sink, with a real click | This gate |
| Windows | Actual output on a virtual render endpoint through WASAPI loopback, on the pinned image | This gate (3031) |
| Each of those, on the new backend | The same scenarios, as `windows-in-house` and `browser-in-house`, with `old-new` cells against quad-snd on that platform. Linux's is 3032's | This gate (3031) |
| macOS | None: **blocks that port**. Hosted capture is unproved | 3032, before its output code; if no route works it stays blocked on the old backend |
| Playdate | Its SDK regressions, unchanged | Not part of this harness |
| GBA | Not required | Its future ticket |

Before a port's new output implementation or switch:

```bash
cargo xtask sound-parity port-ready macos --matrix target/sound-parity/matrix.json
```

It fails until that platform has passed capture evidence with no unmet cell, and
an unavailable cell can't support a parity claim. A platform's capture is added
to `capture.py` and marked `Captured` in `budget.ron`, which makes every cell
of it mandatory.

## What each stage adds

| Stage | Before it may start | What it adds here |
| ----- | -------------------- | ------------------ |
| 2210 decoders | This gate passing | Decoder PCM against the oracles, errors and limits, equal-work decode |
| 3030 mixer | 2210 | Exact mixer samples from independent signals; an offline render budget; bounded decoded PCM over short and long streams |
| 3031 Windows, web | 3030, and Windows `port-ready` | A selectable backend beside quad-snd; old against new; `future_pause`, `future_rate`, `stream_gapless` and `voices_full` |
| 3032 Linux, macOS | 3031, and macOS `port-ready` | The same, on both Mac architectures |
| 2397 regions | 3031 | `future_region`, on the cue's new fields |
| 3084 new default | 3032, 2397, 2234, 2378 | The complete matrix on both backends, then one line flipped |
| 3079 removal | 3084, and the game having run on it | A fresh, complete matrix, the removal, and its revert rehearsed |

## Giving up

Nick can scrap the whole replacement in one session at any stage until quad-snd
is removed, and only he decides to
([ADR-0068](../../docs/adr/0068-the-sound-replacement-stays-reversible-at-every-stage.md)):
the `give-up-sound-replacement` skill.

```bash
cargo xtask sound-parity give-up --apply
```

`footprint.ron` lists what the chain has added, and each of its tickets adds its
own lines in its PR. `cargo test -p xtask` fails if a file names the chain and
isn't covered, or an edit no longer matches; CI's `sound-give-up` job really gives
up and builds what is left. Other fences in the same tests fail if `shuyi-app`
loses quad-snd, or the new backend becomes the default, before the ticket that
may do so.

Reaching the end isn't owed, but stopping is his call: an agent that hits a
roadbump reports what it costs, and doesn't stop on its own judgement.

## Limits of this evidence

Null-sink recording proves what reaches the sound server, not a speaker: an
output device's own processing and listening remain the device sign-offs of
3031, 3032 and 3079. Native gaps and latency are read from the recording, since
quad-snd reports neither. A command's moment is good to a tick and a mixer
block, which `timing_ms` covers, so sample-exact claims are made only inside
one sound, from its marker. Browser CPU counts the whole browser, its software
WebGL too: a comparison across backends, not an absolute cost of sound.
