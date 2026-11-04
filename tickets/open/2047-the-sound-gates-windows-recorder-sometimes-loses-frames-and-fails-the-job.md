---
id: "2047"
title: "The sound gate's Windows recorder sometimes loses frames and fails the job"
type: bug
milestone: "Engine: in-house platform"
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: in-progress
blocked_by: []
nick_input: none
completed:
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

- [ ] The notes say where the frames were lost, and why.
- [ ] A recording with lost frames is still never judged.
- [ ] No `sound-parity-windows` job fails on lost frames in twenty runs on one
      revision.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- A test of whatever decides to record again, if that is the fix.
- CI: the twenty runs.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
