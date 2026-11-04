---
id: "2047"
title: "The sound gate's Windows recorder sometimes loses frames and fails the job"
type: bug
milestone: "Engine: in-house platform"
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-04
---

# 2047 — The sound gate's Windows recorder sometimes loses frames

## Context

`tools/sound-parity/capture.py`'s Windows recorder (`wasapi.py`, WASAPI loopback
on the virtual endpoint that `install-endpoint.ps1` installs, ticket 3031) refuses
a recording the system dropped frames from: `stop` raises "the loopback lost
frames N times" when `discontinuities` is not 0. That is right: a recording with a
hole in it is no evidence.

It fails the whole `sound-parity-windows` job, though, and with it `sound-parity`,
on a pull request that changes no sound. Ticket 2044 ran the jobs ten times on one
revision (CI run 41796857018, branch `t2044-calibration`, never merged): job 9's
"Record both backends" step stopped with "the loopback lost frames 2 times", before
any timing was taken, and `sound-parity (9)` was skipped. Over the 69 Windows jobs
2044 gathered before that, none stopped this way, so it is rare, about one job in
seventy.

## Nick input

None.

## Scope

**In:**
- Why the loopback loses frames: where in the step (a scenario's recording, the
  perf workload, the device-loss test), and what else runs then. Read the job's log
  and what `wasapi.py` reports.
- A fix where the fault is in the recorder (a buffer too small, a read too late, a
  thread that isn't given priority), or else recording that one scenario again: a
  recording with lost frames stays refused, and is never judged.
- A test in `retry_test.py`'s manner for whatever decides to record again.

**Out (do not do):**
- accepting a recording with lost frames, or judging part of one;
- the timing limits (2044, 2046) and the in-house bugs (3088, 3089).

## Implementation steps

1. Read `tools/sound-parity/README.md`, `capture.py` (`Windows`, `stop`),
   `wasapi.py`, and the logs of job `sound-parity-windows (9)` in CI run
   41796857018.
2. Find where frames are lost; run the Windows job many times on a scratch branch
   if one failure says too little (2044's notes say how).
3. Fix the recorder, or record the scenario again, and say which in the README's
   *Run it*.
4. Run the job twenty times on one revision, and keep the result in the notes.
5. Run the `document` skill, then the gates.

## Acceptance criteria

- [x] The notes say where the frames were lost, and why (the why is the evidence's
      best reading, not proven: the next loss says it, see the notes).
- [x] A recording with lost frames is still never judged.
- [x] No `sound-parity-windows` job fails on lost frames in twenty runs on one
      revision.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass (those this change can move: see the notes).

## Tests required

- A test of whatever decides to record again, if that is the fix.
- CI: the twenty runs.

## Completion notes

**Where.** Job `sound-parity-windows (9)` of CI run 41796857018 (job 126462884823,
attempt 1) stopped 32 seconds into "Record both backends": the old backend's
`wav_44k`, the eighth scenario of `windows/run-0`, before any timing and long
before the device-loss test. Nothing else runs then: one probe plays while the
loopback and the silent keeper stream run. Its artifact keeps the refused
recording: its sound is sample-for-sample the same as job 1's (93,365 frames from
the onset, no difference), so the two holes were in the silence before or after
the sound, and the recording is one 10 ms packet (441 frames) shorter than
job 1's. Lengths vary by whole packets between good jobs too, so that isn't proof
of the loss's size.

**Why.** The best reading, not a proof: the system dropped them, not the
recorder. The reader looks every 5 ms into a 500 ms buffer, and a reader too
late loses everything past the buffer (forced locally: a 1 s stall lost 24,000
frames at 48 kHz). Losing about one packet that way needs a stall of just over
half a second that ends just in time, twice in one recording, so a glitch of the
audio engine or of Scream's virtual endpoint fits better. The old recorder kept
nothing that tells the two apart, and twenty more jobs lost nothing, so the
recorder now says it: each loss gives the frames the device's position skipped,
where, and how long the reader had waited against the buffer's length, in the
job log's warning.

**What was done.** Not a recorder fix, as no fault in it was found: the
scenario is recorded again.
- `wasapi.py`: `Loopback.losses` (`Loss`: from the device positions that
  `GetBuffer` already returns) replace the bare `discontinuities` count, with
  `longest_ms` and each loss's `waited_ms`.
- `capture.py`: `LoopbackRecorder.stop` raises `LostFrames`; `whole` plays
  again, up to `TRIES` (3), and fails the step if each recording loses frames.
  `Windows.play` goes through it, clearing a refused run's timings, and the
  refused recording is overwritten: it is never judged. The device-loss test
  reads its own `Loopback` for when sound is heard, and is unchanged.
- `capture_test.py`, run in the Windows job before recording: once when whole,
  again after a loss, failing after three, nothing else retried, and `stop`'s
  refusal.
- The README's file table and *Run it* say so.

**Twenty runs.** CI run 41880881122 on scratch branch `t2047-calibration`
(c32de5db, this change plus a workflow of the Windows job twenty times, never
merged): all twenty `sound-parity-windows` jobs passed, and none of their logs
has a loss warning, so no recording was made again. At about one job in
seventy, none was expected; the warning will name the cause when one comes.

**Gates.** The skill sync check, `check-links`, `check-pins`, `check-docs`,
`typos`, the ticket lint and `cargo test -p xtask` (the give-up fence reads
`ci.yml`) pass locally. No Rust changed, so the Rust build gates were left to CI.

No break: nothing of the engine's API, formats or golden values changed.

Book: none, a CI tool's recorder changed; no public item or chapter.

Nick input: none.
