---
id: "2049"
title: "The sound gate's Windows recording takes a packet of silence in the middle of a sound as evidence"
type: bug
milestone: "Engine: in-house platform"
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: in-progress
blocked_by: []
nick_input: none
completed:
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

- [ ] The notes say whether a mid-sound packet of zeros was flagged silent, from the
      runs named there.
- [ ] A Windows recording with such a hole is recorded again, never judged, or the
      notes say why it can't be told apart (test in `capture_test.py`).
- [ ] Twenty `sound-parity-windows` jobs on one revision pass.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `capture_test.py`, a recording with the hole is made again.
- Integration: the twenty runs above, in CI.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
