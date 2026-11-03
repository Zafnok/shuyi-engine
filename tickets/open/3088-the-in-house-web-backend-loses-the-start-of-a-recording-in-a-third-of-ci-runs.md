---
id: "3088"
title: "The in-house web backend loses the start of a recording in a third of CI runs"
type: bug
milestone: "Engine: in-house platform"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3088 — The in-house web backend loses the start of a recording in a third of CI runs

## Context

Ticket 3031 added the in-house web backend: a worklet fed by messages
([ADR-0075](../../docs/adr/0075-in-house-sound-output-cpal-beside-quad-snd-a-worklet-fed-by-messages-and-a-virtual-endpoint-for-windows-evidence.md)).
The `sound-parity` job records it as platform `browser-in-house`
(`tools/sound-parity/README.md`).

Ticket 2042 ran the job 20 times on one revision with no sound change (CI run
41595469804, on the merge of `main` at 12e1256e into 2042's branch). In 7 of the
20 (jobs 2, 4, 10, 11, 18, 19 and 20), one or two `browser-in-house` scenarios
failed with `missing marker`, and their `repeat:` cell with them. Which scenarios
varied (`wav_44k`, `wav_22k`, `gain`, `onset`, `loop`, `loop_gapless`,
`ogg_48k`, `future_rate`, `voices_full`). The best match's correlation was
nearly always one of two values: 0.804 left and 0.809 right, or 0.837 and
0.843, against `marker_correlation` 0.95. In job 2, the recording `fault:` cells
break was one of those, so they failed too. PR 245's own run (CI run
41595404465) failed the same way on `wav_44k`. In the 47 matrices kept from
2573's merge to 2026-11-02 that 2042 gathered, only `browser-in-house` ever
failed `missing marker`, once, and no other platform did in the 20.

Two exact values in place of noise say the marker's start is cut or delayed by
the same amount each time: likely the worklet's first blocks, before the ring
has the samples, or a gap between the context resuming and the first message.

## Nick input

None.

## Scope

**In:**
- Find why the marker's start goes missing, from the 20 runs' recordings (the
  `sound-parity-run-N` artifacts of run 41595469804, while they last) and new
  ones.
- Fix it in the backend (`crates/app/src/audio/in_house.rs`, the worklet and its
  ring), so that a sound asked for is heard whole from its start.
- A test that would have caught it, below the gate if it can be (the ring's
  test, `tools/sound-parity/ring_test.js`, or the backend's own).

**Out (do not do):**
- lowering `marker_correlation` or any correctness tolerance;
- retrying a scenario that failed;
- the in-house Windows backend's gap (3089);
- timing limits (2044).

## Implementation steps

1. Download a failing job's artifact and compare its recording of a failed
   scenario with the same scenario from a passing job: where the marker starts,
   and what is missing.
2. Find what in the worklet's start leaves those samples out.
3. Fix it, and add the test.
4. Run the `sound-parity` job 20 times on one revision (2042's completion notes
   say how: a scratch branch with numbered copies of the two jobs), and keep the
   numbers in the completion notes.
5. Run the `document` skill, then the gates.

## Acceptance criteria

- [ ] The cause is in the completion notes.
- [ ] No `browser-in-house` cell fails `missing marker` in 20 runs of the job on
      one revision.
- [ ] A test fails on the old code and passes on the fix.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit or integration: the test named above.
- CI: the 20 runs.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
