---
id: "3089"
title: "A gap in the in-house Windows backend's output while everything plays"
type: bug
milestone: "Engine: in-house platform"
model: opus-5.5 / gpt-6.1-sol
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
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

A new recording, attempt 5 of the same run (`sound-parity-windows` re-run at
22:08 UTC), had the gap too. In both recordings the gap is in the same row of
`windows-in-house/perf.jsonl`: row 16, side `a`, pair 8, the last `a` run, whose
recording the gate keeps as `windows-in-house/perf-a.pcm` (in the `sound-parity`
artifact: ids 13882695572 and 13896844974). The gap is one run of 353 quiet
frames (8.0 ms at 44,100 Hz) both times, 2.278 s after the onset in the first
and 0.619 s in the second, counted by `sound_facts` in `capture.py`. `main`'s
probe run (artifact 13895935078) has no quiet run in its `perf-a.pcm` and no gap
in any row. The same length at two places looks like one lost block of the
in-house output, not a slow mixer, which would vary; whether the same pair both
times means anything is for this ticket to find.

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

- [ ] Whose gap it was, and why, is in the completion notes.
- [ ] No `windows-in-house perf:underruns` failure in 20 runs on one revision.
- [ ] A failed Windows cell's log in `sound-parity` says to re-run
      `sound-parity-windows`, not the failed jobs alone, and the README says so.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- A test of the fixed side, where one can be written below the gate.
- CI: the 20 runs.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
