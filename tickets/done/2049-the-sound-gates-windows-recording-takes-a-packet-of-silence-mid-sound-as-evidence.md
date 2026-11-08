---
id: "2049"
title: "The sound gate's Windows recording takes a packet of silence in the middle of a sound as evidence"
type: bug
milestone: "Engine: in-house platform"
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-08
---

# 2049 — The sound gate's Windows recording takes a packet of silence mid-sound as evidence

## Context

Ticket 2048 looked at why `windows repeat:loop` failed at 1.060 dB on ticket 3069's
PR (CI run 42341397532, attempt 1, job 128315452054). It wasn't level noise. In that
job's `sound-parity` artifact, `windows/run-1/loop.pcm` has exactly 441 frames of
digital zero on both channels, ending 1,010.0 ms after the onset, in the middle of
the loop's ramp. That is one 10 ms packet. Every other packet is sample-for-sample
the same as `run-0`'s at the same place. The device position never skipped, so
2047's loss check (`wasapi.py`'s `Loopback.losses`, from `BUFFER_DISCONTINUITY`)
didn't see it, and the gate judged a recording with a hole in it. It is the window
at 1,000 ms that read 1.06 dB.

Where the hole comes from isn't known. quad-snd's WASAPI output (`wasapi_snd.rs`)
prints nothing when it is late. The silent stream that `wasapi.py`'s `Endpoint`
keeps open writes zeros, so a late quad-snd block would mix to exact zeros. The
loopback may also have marked the packet `BUFFER_SILENT`, which `read_packet` writes
as zeros and doesn't record. Natively, 2048 refuses a scenario that quad-snd says it
underran in. Here the backend says nothing, so the evidence has to come from the
loopback or from the recording.

In 2048's twenty runs (CI run 42389680314, branch `t2048-calibration`), no
`windows` recording had such a hole: `repeat:loop` read 0.000 in 17 jobs and
0.268 in 3, and the worst `windows` repeat was 0.268 dB. Job 6's
`windows-in-house/run-0/wav_48k.pcm` had one (794 frames of zero ending 1,245.2 ms
after the onset), which failed `windows-in-house repeat:wav_48k` at 1.939 dB. That
one is the in-house backend's, which is 3089's job.

## Nick input

None.

## Scope

**In:**
- Find whether a mid-sound packet of zeros is flagged `BUFFER_SILENT`. Have
  `wasapi.py` keep each silent packet's frame (as `Loss` does for a discontinuity)
  and print them in the job's log.
- If it is the loopback's (flagged): refuse the recording, as 2047 does a loss, so
  `whole` records it again.
- If it isn't flagged, it is the backend's own output. Then decide how
  `capture.py` notices it for quad-snd without refusing a scenario's real silences
  (`Silent` expectations, the marker's gap, a native loop's join). Say how in the
  README's *Run it*.
- A test in `capture_test.py` for whatever decides.

**Out (do not do):**
- the in-house Windows backend's gaps (3089);
- judging part of a recording, or a limit raised to pass one with a hole;
- anything in the browser.

## Implementation steps

1. Read 2048's completion notes, `wasapi.py` (`read_packet`, `Loss`) and `capture.py`
   (`LoopbackRecorder.stop`, `whole`).
2. Record the silent flags' frames. Run the `sound-parity-windows` job twenty times
   on one revision, as 2047's notes say, until a hole comes, or keep the twenty
   runs' answer if none does.
3. Refuse what the evidence says is a hole, and test it.

## Acceptance criteria

- [x] The notes say whether a mid-sound packet of zeros was flagged silent, from the
      runs named there.
- [x] A Windows recording with such a hole is recorded again, never judged, or the
      notes say why it can't be told apart (test in `capture_test.py`).
- [x] Twenty `sound-parity-windows` jobs on one revision pass.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `capture_test.py`, a recording with the hole is made again.
- Integration: the twenty runs above, in CI.

## Completion notes

**What the hole was.** Not a lost packet: quad-snd's own gap at a native loop's join.
In 2048's artifact (CI run 42341397532, `sound-parity-windows`), the 441 frames of
zeros in `windows/run-1/loop.pcm` start 1,000.0 ms after the onset, exactly where
`loop.wav` (a 100 ms marker and 400 ms of ramps) comes round the second time. In
2048's twenty jobs (CI run 42389680314), jobs 4, 5 and 9, the three whose `windows
repeat:loop` read 0.268, have 127 frames of zeros at each of the three joins, 500,
1,000 and 1,500 ms after the onset; the other seventeen have none. On this ticket's
twenty, job 3 has the same 127 at each join. So quad-snd's Windows loop leaves none,
127 or 441 frames at a join, as its mixing blocks fall against the file: what
`scenarios.ron` already says of quad-snd's native loop ("isn't gapless there") and what
its `Boundary` is for. The `loop` scenario, the one that comment names, marked no
joins, so its repeat compared the 50 ms windows across them.

Every other zero run inside a quad-snd sound on Windows, over the 42 jobs looked at
(2048's two attempts' artifacts, its twenty, these twenty), is a scenario's own
silence: the marker's 500 ms gap and `mute`'s pause, 200 ms and more. Packet
alignment can't tell a hole from those: every one of them, the joins too, starts on
the same phase of the endpoint's 441-frame period.

**Flagged silent?** Not in any of this ticket's twenty jobs: no packet was flagged
`BUFFER_SILENT` in any recording, the joins' zeros included. The silent stream that
`Endpoint` keeps open writes zeros, so the loopback always has a stream rendering.
2048's run kept no flags. So the zeros are what the engine mixed, quad-snd's sound and
the keeper's zeros, and never the loopback's.

**What was done.** The first try refused a quad-snd scenario with a run of exact zeros
5–100 ms long inside its sound. It was taken back before the PR: it refused quad-snd's
real 441-frame join, and the ticket says a native loop's join must not be refused.
- `scenarios.ron`: the `loop` scenario marks its joins, `Boundary` at 500, 1,000 and
  1,500. Its repeat still compares the ramps away from them, and its `Waveform` check
  (smear 4,096) is unchanged.
- `wasapi.py`: `Loopback.silences` keeps each packet flagged silent (its first frame
  and length). `capture.py`'s `LoopbackRecorder.stop` prints them, runs of packets
  joined, in the job's log. No recording is refused for them.
- `crates/xtask/src/sound_parity/tests.rs`: on the real `loop` scenario, two runs
  that differ only by 441 zeros at the second join pass the repeat, and fail it with
  the boundaries taken out. `capture_test.py`: the log line of flagged packets.
- The README's *Run it* says so; `budget.ron`'s note on `repeat_db` names the join.

**Deviation.** The ticket's third scope item assumed a hole from the backend being
late and asked `capture.py` to notice it. The evidence says it isn't a hole: it is the
backend's real output where the scenario said it may differ, so the gate leaves it out
there, through the scenario's data. That isn't judging part of a recording to pass a
hole: `Boundary` is how every other looped scenario has always said where quad-snd's
native loop comes round. The test is in the gate's tests, as that is what decides,
and the acceptance's "recorded again, never judged" is met by "the notes say why it
can't be told apart": a recording-side rule can't tell it from the join.

**Checked on real recordings.** The gate run locally on 2048's artifact (meta's
`scenarios_sha256` set to each file's, in a scratch copy only): with `main`'s
`scenarios.ron`, `windows repeat:loop` failed at 1.060 dB, as in CI; with this one it
passed at 0.000. On this ticket's job 3 (the 127-frame joins) it passes at 0.000.

**Twenty runs.** CI run 42455240426 on scratch branch `t2049-calibration` (1ed35785:
this change plus a workflow of the Windows job twenty times, never merged). All twenty
`sound-parity-windows` jobs passed. Each ran `capture_test.py`, none logged a silent
packet, a loss or a recording made again. A first run (42454083104), on the refusing
version, was cancelled when the joins were found.

**For 3089.** The in-house holes 2048 found (job 6's 794 frames in `wav_48k`, job 1's
353 in `stream_gapless`) are not explained by this: the in-house backend's loops are
held to every frame (`loop_gapless`, `stream_gapless`), so no gap is its own, and no packet was flagged silent, so they are what the engine mixed from the in-house
backend. None came in this ticket's twenty. 3089 now says so.

**Gates.** `capture_test.py`, `cargo fmt --all --check`, `cargo clippy -p xtask
--all-targets -D warnings`, `cargo test -p xtask` (456 and 29), the skill sync check,
`check-links`, `check-pins`, `check-docs`, `typos` and the ticket lint pass locally.
Only xtask's tests changed in Rust, so the rest of the Rust gates were left to CI.

No break: nothing of the engine's API, formats or golden values changed.

Book: none, a CI tool's scenario and recorder changed; no public item or chapter.

Nick input: none.
