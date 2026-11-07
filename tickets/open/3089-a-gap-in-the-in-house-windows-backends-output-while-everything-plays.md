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
they last. 2049 looks at the same kind of hole in quad-snd's Windows recordings, and
whether the loopback flags it silent; what it finds says which side such a hole is.

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
4. Run the `document` skill, then the gates.

## Acceptance criteria

- [ ] Whose gap it was, and why, is in the completion notes.
- [ ] No `windows-in-house perf:underruns` failure in 20 runs on one revision.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- A test of the fixed side, where one can be written below the gate.
- CI: the 20 runs.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
