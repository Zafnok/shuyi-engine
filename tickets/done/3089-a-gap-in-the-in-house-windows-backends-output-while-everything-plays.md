---
id: "3089"
title: "A gap in the in-house Windows backend's output while everything plays"
type: bug
milestone: "Engine: in-house platform"
model: opus-5.5 / gpt-6.1-sol
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-11
---

# 3089 — A gap in the in-house Windows backend's output while everything plays

## Context

Ticket 3031 added the in-house Windows backend, CPAL on a virtual render
endpoint
([ADR-0075](../../docs/adr/0075-in-house-sound-output-cpal-beside-quad-snd-a-worklet-fed-by-messages-and-a-virtual-endpoint-for-windows-evidence.md)).
The `sound-parity` job's `perf:underruns` cell counts runs of quiet in the
recording while 32 voices and two streams play (`sound_facts` in
`tools/sound-parity/capture.py`, and its Windows twin `wasapi.py`). A gap is a
count, not a ratio: one fails the cell, and the job doesn't measure it again.

Ticket 2042 ran the job 20 times on one revision with no sound change (CI run
41595469804). In job 10, `windows-in-house perf:underruns` failed with `1 gaps
while everything played`. The other 19 had none, and quad-snd on the same
runners (`windows`) had none in 20. One in 20 is rare, but a player hears it,
and it fails a pull request.

**From 2048 (done):** two of its twenty `sound-parity` jobs (CI run 42389680314,
branch `t2048-calibration`) failed on `windows-in-house` scenarios. In job 6,
`windows-in-house/run-0/wav_48k.pcm` has 794 frames of digital zero ending 1,245.2
ms after the onset, in the middle of the tone (`repeat:wav_48k`, 1.939 dB). In job 1,
`scenario:stream_gapless` failed with `loop.wav differs by 11756 at frame 45844 of
66150, over 2 (pass 3)`. Their artifacts (`sound-parity-run-6`, `sound-parity-run-1`
and the matching `sound-parity-windows-N`) are more of this ticket's evidence, while
they last.

**From 2049 (done):** quad-snd's "hole" in `windows repeat:loop` was its own gap at
the native loop's join, which `loop` now marks as a `Boundary`. These two in-house
holes are not that: the in-house backend's loops are held to every frame, so it may
leave no gap at a join. The loopback
never flagged a packet silent in 2049's twenty jobs (CI run 42455240426), where
`capture.py` now logs every flagged packet, and the silent keeper stream writes
zeros: a run of zeros in a recording is what the engine mixed, so these are the
in-house backend's output, not the capture's. No in-house hole came in those twenty.

**From PR 368 (ticket 2054):** the gap came again, on a change that touches no
sound code. `windows-in-house perf:underruns` failed with `1 gaps while everything
played` in CI run 43068380206, recorded by its `sound-parity-windows` job at
17:28 UTC on 2026-11-11 (artifact `sound-parity-windows`, id 13878925661, kept
until 2027-02-09). The same commit's other jobs all passed, and the same day
`main`'s code (a5354c89) passed both sound jobs at the first try (CI run
43102931198, a probe PR closed unmerged).

That run also shows a trap. `sound-parity` only judges the recordings that
`sound-parity-windows` makes. "Re-run failed jobs" re-runs `sound-parity` alone:
the passed `sound-parity-windows` and its artifact are carried over, so the same
recording, with its gap, fails again on every attempt. Attempts 1 to 4 of that
run judged one recording four times. A re-run that can pass re-runs
`sound-parity-windows` (GitHub's "Re-run job" on it, or `gh run rerun --job
<its id>`), which runs `sound-parity` after it.

## Nick input

None.

## Scope

**In:**
- Find whether the gap is the backend's (an underrun of CPAL's buffer, a block
  the mixer was late with) or the capture's (the loopback recording dropping
  frames), from job 10's artifact (`sound-parity-run-10` and
  `sound-parity-windows-10`, while they last) and new runs.
- If it is the backend's, fix it. If it is the capture's, make the capture
  report it apart from the backend's gaps, so it doesn't pass as one either way.
- When a cell of a Windows platform fails, the `sound-parity` job's error says
  that re-running it alone judges the same recording again, and how to record
  anew (re-run `sound-parity-windows`), and `tools/sound-parity/README.md`
  says the same.

**Out (do not do):**
- allowing gaps in `perf:underruns`;
- the in-house web backend's missing marker (3088);
- timing limits (2044).

## Implementation steps

1. Find the gap in job 10's `perf-*.pcm` recording of `windows-in-house`, and
   what the backend and the capture logged then.
2. Decide which side it is, and fix that side.
3. Run the `sound-parity` job 20 times on one revision (as 2042's completion
   notes say), and keep the numbers in the completion notes.
4. In the `sound-parity` job's step that holds the matrix (ci.yml), print after a
   failed Windows cell that a re-run must re-run `sound-parity-windows`, with the
   job's id; write the same in `tools/sound-parity/README.md`.
5. Run the `document` skill, then the gates.

## Acceptance criteria

- [x] Whose gap it was, and why, is in the completion notes.
- [x] No `windows-in-house perf:underruns` failure in 20 runs on one revision.
- [x] A failed Windows cell's log in `sound-parity` says to re-run
      `sound-parity-windows`, not the failed jobs alone, and the README says so.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- A test of the fixed side, where one can be written below the gate.
- CI: the 20 runs.

## Completion notes

**Whose gap: the backend's.** Its render stream ran dry; the capture lost nothing.
In 2048's job 6, `windows-in-house/run-0/wav_48k.pcm` holds 794 frames of digital
zero 1,227 ms after the onset. Before them the tone is equal, sample for sample, to
run-1's clean recording at the same place. After them it is equal to run-1's 794
frames earlier. So 794 frames of zeros were put in and none of the sound was lost:
the device played nothing while the stream had nothing in its buffer, then went on
where it was. A loopback dropping frames loses sound instead (2047). PR 368's gap,
in the `perf-a.pcm` its artifact kept, is the same kind: 353 frames of zeros. No
recording had a rebuilt stream or a callback silent for the lock (136 `output`
lines, all 0 and 0), and the loopback flagged no packet silent.

**Why.** quad-snd asks WASAPI for a buffer of 4,096 frames (93 ms), and its
latency is about 85 ms. The in-house output asks CPAL for its default, the
engine's least, about 20 ms (its latency is 19 ms), and CPAL's thread ran at normal
priority: CPAL raises it only with its `realtime` feature, whose crate is MPL-2.0.
On a busy runner, a normal thread can wait longer than 20 ms for a core. A larger
buffer would cost its latency: `old_new` holds it to 0.35 of quad-snd's (about 30
ms), a limit this ticket leaves (2044).

**The fix.**

- `shuyi-sound-out` raises CPAL's thread at the stream's first callback: Windows'
  multimedia scheduler's "Pro Audio" task, or the time-critical priority where that
  is refused. It reverts when the callback is dropped, on that thread.
  `crates/sound-out/src/sys.rs`, `raised`, through the `windows` crate (0.62, which
  CPAL already builds on; MIT OR Apache-2.0).
- `shuyi-sound-out` counts each callback that finds the device's buffer empty
  (`Output::dry_callbacks`), and says whether the thread was raised
  (`Output::raised_priority`).
- The probe says both in its `output` line, now after a timing too, and
  `capture.py` puts `dry_callbacks` in each timing's metrics.
- The gate's `perf:underruns` and `old-new:underruns` say how often the buffer ran
  dry, or that it never did and the gap isn't the output's. A capture's gap now
  says so either way.

**Deviation: a frame's commands reach the mix together.** The raise brought out a
race that was always there. The game's thread sends a frame's commands one by
one, and the audio thread applied whatever had come at each block. Raised, it ran
between them more often. `voices_full`, whose 32 copies must start on one frame,
measured 0.170 and 0.192 instead of 0.230 in jobs 3 and 4 of the first run with the
fix (CI run 43131692726): a split of about 20 and 12 copies, and 26 and 6. It had
never failed before. Fixed here, at Nick's word in the session, rather than as a
ticket of its own:

- `shuyi_sound::mix::Remote::hold` and `publish`: held commands take room in the
  queue but the audio thread sees none until they are published, all at once.
- `shuyi_platform::audio::player::Backend::frame(open)`, which does nothing by
  default. `Player::play` calls it around each frame.
- The in-house backend holds while a frame is open and publishes when it closes.

**Twenty runs, three times.** Each on a scratch branch, never merged, with a
workflow of the `sound-parity-windows` and `sound-parity` pair twenty times. In the
first two, every `sound-parity` job ended red on this ticket's own re-run advice
step, a quote that broke its shell. The table judges each job's `matrix.json`.

| Run | Revision | Jobs with every mandatory cell passed | `windows-in-house` gaps | Dry callbacks |
| --- | -------- | ------------------------------------- | ----------------------- | ------------- |
| 43131815627, `t3089-baseline`: counted, not raised | 1a245b0e | 17 of 20 | 1 (job 5, `perf`) | 3, the gap's own and 2 too short to show |
| 43131692726, `t3089-calibration`: raised | dbe37828 | 17 of 19 | 0 | 0, every stream raised |
| 43143877256, `t3089-calibration-2`: raised, frames held | 86a84729 | 18 of 20 | 0 | 0, every stream raised |

The baseline's gap, in job 5's `perf` pair 2, came with one dry callback in that
same recording: the gate said "1 gaps while everything played; the output's buffer
ran dry 1 times". Its other two failures were `linux-native perf:peak_kib` (jobs 3
and 14, medians 1.056 and 1.057), which 2068 owns. The first fix run's job 2 hung
for over 90 minutes recording the web backend (`Record the web backend on its real
loader`) and was cancelled, so it has no matrix.

The last run is this change before `main` was merged in (69023aec, `main` at
b6dc5c22): no `windows-in-house` gap and no dry callback in its 20 Windows jobs, and
`voices_full` passed in all 20. Its two failures are timings of two runs of one
build (sides a and b), which no backend change can lean. In job 8,
`browser-in-house perf:file_io_ms` measured a median ratio of 1.316. In job 10,
`windows-in-house perf:cpu_ms` measured 1.195 against a limit of 1.15, the process's
cycles rising in both sides partway through (a 53 to 96, b 55 to 107), and
`perf:decode_ogg_ms` was inconclusive (spread 0.102). Its other 19 jobs' `cpu_ms`
medians are 0.968 to 1.033; the baseline's are 0.951 to 1.128. Both cells go to
2068, which owns the timing cells.

**Re-running a failed Windows cell.** The matrix step of `sound-parity`
(`ci.yml`) now prints an error after a failed Windows cell. It says that
re-running the job, or the failed jobs, judges the same recording again, and that
`sound-parity-windows` is what to re-run, with its job's id from the API
(`actions: read`, the job's only new permission) and the `gh run rerun --job`
line. `tools/sound-parity/README.md` says the same, and how a gap says whose it is.

**Docs.** The new items' rustdoc, and `tools/sound-parity/README.md`.

Book: none, the items added are for a backend and the sound probe: no game names
`Backend::frame`, `Remote::hold`, `Remote::publish` or the output's counts, and the
book's chapters teach none of them.

Behaviour: shuyi-app: the in-house backend starts the sounds one frame asks for on
the same sample. Before, they could start a block apart, a few milliseconds, when
the audio thread ran between two of them.

Behaviour: shuyi-sound-out: on Windows, the output's audio thread runs as a "Pro
Audio" task of the multimedia scheduler, or at the time-critical priority, above
the game's threads.

Handoffs: 2064 gets a **From 3089** note (no flaky entry for 3089; the three
scratch branches to delete). 2068 already names the timing cells the baseline
showed, and a **From 3089** note with the last run's two.

No ticket: the hang in job 2's web recording happened once in 60 jobs, and on
none of the platforms this ticket changed. 2064's ledger is where a second
sighting goes.
