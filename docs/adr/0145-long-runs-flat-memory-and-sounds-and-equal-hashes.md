# ADR-0145: Long runs: the heap and the sounds alive stay flat over hundreds of thousands of frames, and a second run hashes the same

- **Status:** Accepted
- **Date:** 2026-11-24
- **Related tickets:** 3509 (this), 3008, 2527, 3221, 3319
- **Extends:** ADR-0081

## Context

Everything that measured a game was short. `bench` (ADR-0081) plays
scenarios of a few seconds and reads the frames from the perf ring, which
keeps the last 600 (`PERF_FRAMES`), so it refuses a longer one. The
`playtest` workflow plays each stage to a result; 2527's `shuyi soak`
plays the rules alone, with no frames, screens or sounds. A slow leak
shows only over hours: a cache that grows with each map entered, a sound
handle never released, a log that keeps every line. The real-time games
next on the roadmap run longer sessions than a turn game, and a store's
checks (Steam Deck now, consoles later) find a leak before we would.

What the engine had to build on: the program's heap counter
(`shuyi_alloc_count::global`) and its rows by memory tag (ADR-0046 §6),
read through a `Session`'s `heap_snapshot`; `App::hash_frames` and
`App::frame_hash`, which hash each frame's screens, sounds, music and the
game's own hash without recording; and `Harness::sounds_playing`. And a
trap: the Harness logs every frame's audio, music and screens, so a run
of a million frames would always grow, by the test driver's own doing.

## Decision

### 1. A long scenario is a test input, as a bench scenario is

`<name>.long.ron`, found where `bench` finds its scenarios, read with
serde and `deny_unknown_fields` (`crates/cli/src/bench/long.rs`,
`LongScenario`, format 1): `about`, `start` and `script` as a bench
scenario's; `repeat` (the times the script plays, 1 or more), `idle`
(frames with no input after each time), `warm_up` (frames not judged),
`sample_every` (frames between samples, 1 or more) and `slack`
(`heap_bytes`, 64 KiB unless given; `sounds`, 0 unless given). No build
ships it and no `Format` reads it (ADR-0081 §4).

### 2. `bench --long`, in the bench's profile, never under Callgrind

`cargo shuyi bench --long [--twice] [--scenario <name>] [--out <dir>]`
builds the game's tool program in `shuyi-bench` (ADR-0081 §2: the heap is
counted) and runs `bench-play --long <file> --out <file>` once for each
scenario. The inner half starts the session with `measured_start`, turns
perf on, and calls `Session::set_long`, which on a `Headless` turns the
Harness's logs off (`Harness::keep_logs(false)`) and `App::hash_frames`
on. It plays the script and the idle frames `repeat` times, and after a
step that passed a sample's frame it writes a `Sample` as one JSON line:
the frame, the live heap, each tag's live bytes, `Session::sounds_alive`
and `Session::frame_hash`. Written as it goes, the samples never grow
the heap they measure; the perf ring is never read, so 600 frames is no
limit. Wall time is never read (ADR-0081).

### 3. The flatness rule, by quarters

Past `warm_up`, the samples are split in quarters, each a quarter of
their count rounded down; fewer than four is an error. The run fails
when the most live heap of the last quarter is over the first quarter's
most by more than `slack.heap_bytes`, or the most sounds alive of the
last quarter over the first's by more than `slack.sounds`. The failure
names the scenario, both numbers, and the three memory tags whose most
live bytes grew most. Comparing the most of each quarter, not two single
samples, lets a heap that breathes with the game's loop stay flat.

### 4. The determinism rule, in a second process

`--twice` plays each scenario again in a process of its own and fails at
the first sample whose frame or hash differs, naming its frame. A second
process, not a second session in the first, so what one process keeps
(a static, an id, a cache) shows.

### 5. Weekly, never on a pull request

`.github/workflows/long-run.yml` runs `bench --long --twice` on Hello,
Grid every Monday and on `workflow_dispatch`, posts the table in the
job's summary, uploads `long-report.json` and `long-report.md`, and on a
failure opens one issue labelled `long-run` or comments on the open one,
as `security.yml` does. CI's `bench` job checks the toy's fixtures
(`crates/cli/toy/long/`) on each pull request that changes code: flat
passes twice, a leak fails naming its tag, loops never stopped fail
naming the counts, and a process id in the state fails `--twice` at the
first sample.

## Consequences

- A game adds a long scenario as it adds a bench scenario, and runs
  `cargo shuyi bench --long` in its own CI if it wants to; no reusable
  workflow does it for it.
- What the headless run can't see is not measured: textures on the GPU,
  the sound device's voices, open files. They exist only in `shuyi-app`
  with a window.
- `Session` gains `set_long`, `frame_hash` and `sounds_alive`, each with a
  default, so a game's own `Session` (rare) needs no change. `Harness`
  gains `keep_logs`.
- With its logs off, a Harness's `audio_log`, `music_log`,
  `dropped_sounds` and `screen_log` say the last frame only. A test that
  reads them keeps the default.
- A run's numbers are the whole program's heap, the driver's included.
  The driver holds a fixed amount once its logs are off, so it moves no
  quarter.

## Alternatives considered

- **A single sample at each end** — a heap that rises and falls with the
  game's loop would fail or pass by where the samples fell.
- **A slope fitted over all samples** — a slope in bytes per frame is
  hard to set a budget for and to say in a failure; the quarters' most
  is a number a reader checks by eye.
- **A second session in the same process for `--twice`** — a static or a
  process id that leaks into the state would hash the same both times.
- **Reading the perf ring in pages** — it keeps only what was not yet
  overwritten, at the ring's pace, and holds a frame record per frame:
  sampling every *n* frames needs neither.
- **Running on every pull request** — hundreds of thousands of frames,
  twice, take minutes on a runner, for a fault that shows over weeks.
