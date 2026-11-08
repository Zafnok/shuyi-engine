---
id: "2048"
title: "The sound gate's quad-snd recordings fail on runner noise: an ALSA underrun natively, and a level off by 1 dB on Windows"
type: bug
milestone: "Engine: in-house platform"
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-07
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

- [x] A native recording during which quad-snd reported an underrun is recorded
      again, never judged (test in `capture_test.py`).
- [x] The notes say where attempt 2's underrun fell and whether the judged sound had
      the hole.
- [x] `repeat_db` stays or moves by the budget's rule, from runs named in its
      comment.
- [x] Twenty `sound-parity` jobs on one revision pass, or each failure is named in
      the notes with its cause.
- [x] The book's chapter is updated (2802), or the completion notes have a
      `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `capture_test.py`, a recording with an underrun reported is made again, and
  three in a row fail the step.
- Integration: the twenty runs above, in CI.

## Completion notes

**Where the native underrun fell.** Attempt 2's two ALSA lines (06:33:25.14) fell
in `linux-native/run-0/wav_48k`: the artifact's recordings end at 06:33:24
(`ogg_22k`) and 06:33:26 (`wav_48k`). The judged sound had the hole. Measured in
5 ms windows from the onset, run-0's tone is quiet from 820 to 835 ms and ends at
1,645 ms, where run-1's and both `ogg_48k`'s end at 1,755 ms. About 110 ms is
missing, a quad-snd block (4,096 frames, 93 ms) and a little more. That is the
`Fall` 100 ms off and the 12.2 dB repeat difference. quad-snd prints these lines with
`println!`, on its **stdout** (`alsa_snd.rs`), not its stderr as the ticket guessed,
and `capture.py` never read the probe's stdout. That is why the lines reached the log
unseen.

**What was done.**
- `capture.py`: `Native.play` reads the probe's stdout too. A scenario whose
  play printed quad-snd's `Underrun occured` raises `LostFrames`, and `whole` (2047's)
  records it again, up to three times, failing the step if each underruns. A refused
  run's timings are cleared, as on Windows. A timing (`perf`, `decoders`, `mixer`) is
  never played again: the gaps it records are its own `underruns` count, and quad-snd's
  stdout is still echoed to the log there.
- `capture_test.py`: a scenario that underran is made again and only the whole one
  answers, three in a row fail the step, and a timing isn't played again. CI's
  `sound-parity` job now runs it before recording, as the Windows job does.
- `README.md`'s *Run it* and file table say so. `_typos.toml` allows quad-snd's
  spelling, `occured`, which the recorder matches.

**The Windows level.** The 1.060 dB wasn't noise. Attempt 1's
`windows/run-1/loop.pcm` has exactly 441 frames (one 10 ms packet) of digital zero on
both channels, ending 1,010.0 ms after the onset, mid-ramp. Every other packet
matches run-0's sample for sample, and the device position never skipped, so 2047's
loss check couldn't see it. The 50 ms window at 1,000 ms read 1.06 dB. That recording
should have been refused, so 2049 is written for the Windows recorder. Over the 20
`sound-parity-windows` jobs of CI run 42389680314 (all passed, none recorded
anything again), `windows repeat:loop` read 0.000 in 17 and 0.268 in 3 (jobs 4, 5
and 9), with no hole in any. Windows' worst repeat over all scenarios was 0.268.
Twice that is under 1.0, so `repeat_db` stays, with the runs named in `budget.ron`.

**Twenty runs.** CI run 42389680314 on scratch branch `t2048-calibration` (936f9ff9:
this change plus a workflow of both jobs twenty times, never merged). All 20
`sound-parity-windows` jobs passed. 16 of 20 `sound-parity` jobs passed. No native log
has an underrun line, so no scenario was recorded again, and `linux-native
repeat:wav_48k` read 0.000 in all twenty. None of the four failures is quad-snd's:
- job 1: `windows-in-house scenario:stream_gapless` (`loop.wav differs ... at frame
  45844 ... (pass 3)`) and `repeat:stream_gapless` 2.244 dB, the in-house Windows
  backend: 3089, which now names it.
- job 6: `windows-in-house repeat:wav_48k` 1.939 dB: a 794-frame hole of zeros ending
  1,245.2 ms after the onset, in-house again: 3089.
- job 9: `browser-in-house scenario:loop` (`differs ... at frame 0 ... (pass 1)`).
- job 20: `browser-in-house scenario:ogg_44k` and `repeat:ogg_44k`, missing marker
  (correlation 0.943). Jobs 9 and 20 are 3088's kind, the start of a recording lost,
  after its fix: 2050.

**Follow-ups.** 2049 (the Windows recording takes a packet of silence mid-sound as
evidence), 2050 (the in-house web backend still loses a recording's start). 3089 got
the evidence above.

**Gates.** `capture_test.py`, `cargo test -p xtask` (the give-up fence reads
`ci.yml`), the skill sync check, `check-links`, `check-pins`, `check-docs`, `typos`
and the ticket lint pass locally. No Rust changed.

No break: nothing of the engine's API, formats or golden values changed.

Book: none, a CI tool's recorder changed; no public item or chapter.

Nick input: none.
