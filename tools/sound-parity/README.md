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
| `crates/app/examples/sound_probe.rs` | The probe: the shared player on the real backend |
| `crates/xtask/src/sound_parity*` | The corpus's numbers, the comparator and the gate |

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

```bash
cargo xtask sound-parity gate --dir target/sound-parity --revision "$(git rev-parse HEAD)"
```

The browser half builds the same example for `wasm32-unknown-unknown`, puts it
beside the loader as CI's job does, and runs `capture.py browser --page <its
folder>`. The gate writes `target/sound-parity/matrix.json` and exits non-zero
unless every mandatory cell passed. On a machine with one platform's capture
it fails on the other's: only CI, with both, can pass.

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
  today's fallback and holds on quad-snd alone.

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
way. A `failed` cell is never measured again, and a slowdown shows in the median
however often it is. The injected slowdown must fail. Raise a limit only with new calibration runs
that justify it, written in `budget.ron`, never to pass.

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
| Windows | None: **blocks that port** | 3031, through WASAPI loopback on a real render endpoint, before its output code |
| macOS | None: **blocks that port**. Hosted capture is unproved | 3032, before its output code; if no route works it stays blocked on the old backend |
| Playdate | Its SDK regressions, unchanged | Not part of this harness |
| GBA | Not required | Its future ticket |

Before a port's new output implementation or switch:

```bash
cargo xtask sound-parity port-ready windows --matrix target/sound-parity/matrix.json
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
| 3031 Windows, web | 3030, and Windows `port-ready` | A selectable backend beside quad-snd; old against new; `future_pause` and `future_rate` |
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
