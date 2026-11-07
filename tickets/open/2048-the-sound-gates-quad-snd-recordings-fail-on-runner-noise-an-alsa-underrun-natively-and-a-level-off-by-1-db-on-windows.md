---
id: "2048"
title: "The sound gate's quad-snd recordings fail on runner noise: an ALSA underrun natively, and a level off by 1 dB on Windows"
type: bug
milestone: "Engine: in-house platform"
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2048 — The sound gate's quad-snd recordings fail on runner noise

## Context

The `sound-parity` job (ADR-0067, `tools/sound-parity/README.md`) runs on a pull
request that changes a sound path, `tools/sound-parity/` included. On ticket
3069's PR (CI run 42341397532, head `22399e38`), whose only sound-path change was
`tools/sound-parity/footprint.ron`, the gate failed twice, on different cells, with
nothing that plays sound changed:

- **Attempt 1** (job 128315452054): `windows repeat:loop: failed: 1.060 dB apart at
  worst`. The limit is `repeat_db: 1.0` in `budget.ron`, whose comment says the worst
  measured was 0.29 dB. The `sound-parity-windows` job, which records it, had passed.
- **Attempt 2** (job 128327086456, after `sound-parity-windows` alone ran again and
  passed): `linux-native scenario:wav_48k: failed: the sound doesn't end within
  100 ms of 1500 ms (a wrong duration)` and `linux-native repeat:wav_48k: failed:
  12.224 dB apart at worst`. The step "Record the native backend" logged quad-snd's
  two ALSA underrun warnings, a short write and then `-EPIPE`, each ending
  `attempting recover` (quad-snd's lines start with `Underrun`, spelt its own way;
  search the log for them). Attempt 1's native recording logged none.

Both are quad-snd, the old backend, so this is not 3089 (the in-house Windows
backend's gap). It is 2047's kind of failure: a recording with a hole in it judged
as evidence. 2047 made the Windows loopback record a scenario again when it lost
frames; the native recording has no such check, and an underrun of the backend's own
output, rather than of the recorder, is not seen at all.

## Nick input

None.

## Scope

**In:**
- **The native underrun.** Find which scenario's recording the underrun fell in
  (attempt 2's artifact keeps the recordings; `wav_48k` is the likely one), and
  whether the output the gate judged has the hole. Then make `capture.py`'s native
  recording notice one: the probe's stderr carries quad-snd's `Underrun` lines,
  or the recording's own gaps show it. A scenario recorded while the
  backend underran is never judged: it is recorded again, up to three times, as 2047
  does on Windows, and the step fails if each underruns.
- **The Windows level.** Gather `repeat:loop`'s worst level difference over many
  Windows jobs on one revision (2044's notes say how to run them and fetch the
  artifacts), and say whether 1.06 dB is noise the limit should hold, by the
  budget's rule (twice the worst measured), or a recording that should have been
  refused. Change `repeat_db` only when those runs say so, with the runs named in
  `budget.ron`'s comment.
- A test in `capture_test.py`'s manner for whatever decides to record again.

**Out (do not do):**
- judging part of a recording, or accepting one with an underrun in it;
- raising a limit without the runs that show it;
- the in-house backends' gaps (3089) and the timing limits (2044, 2046);
- retrying the whole job in CI on a failed cell: a `failed` cell stays failed.

## Implementation steps

1. Read `tools/sound-parity/README.md`, `capture.py` (the native recorder and
   2047's `LostFrames` path), `capture_test.py`, `budget.ron`, and the logs and
   artifacts of CI run 42341397532, attempts 1 and 2.
2. Find where the underrun fell and what it did to the recording.
3. Make the native recording refuse and record again a scenario that underran; say
   so in the README's *Run it*, beside 2047's sentence.
4. Run the `sound-parity-windows` job twenty times on one revision and keep each
   `repeat:loop` level in the notes; set `repeat_db` by the rule, or leave it and
   say why.
5. Run the `sound-parity` job twenty times on one revision, and keep the result in
   the notes.

## Acceptance criteria

- [ ] A native recording during which quad-snd reported an underrun is recorded
      again, never judged (test in `capture_test.py`).
- [ ] The notes say where attempt 2's underrun fell and whether the judged sound had
      the hole.
- [ ] `repeat_db` stays or moves by the budget's rule, from runs named in its
      comment.
- [ ] Twenty `sound-parity` jobs on one revision pass, or each failure is named in
      the notes with its cause.
- [ ] The book's chapter is updated (2802), or the completion notes have a
      `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `capture_test.py`, a recording with an underrun reported is made again, and
  three in a row fail the step.
- Integration: the twenty runs above, in CI.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
