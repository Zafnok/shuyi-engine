---
id: "2042"
title: "The sound gate's browser latency limit calibrated against more runs, so a noisy machine doesn't fail a pull request"
type: bug
milestone: "Engine: public"
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-03
---

# 2042 — The sound gate's browser latency limit, calibrated

## Context

[PR 226](https://github.com/Zafnok/shuyi-engine/pull/226) (ticket 2818) changes
no sound code. It edits `ci.yml`, which counts as a sound path (2039), so the
`sound-parity` job ran, correctly. At commit `9194feb`, CI run 41352854235,
attempt 1, the job failed with:

- `browser-chromium perf:latency_ms`: `failed`, median ratio 1.146, spread
  0.227, 9 pairs, median 28.44;
- `browser-chromium perf:decode_ogg_ms`: `inconclusive`, median ratio 1.000,
  spread 0.192 (its limit is 0.19);
- `linux-native perf:decode_ogg_ms`: `inconclusive`, median ratio 0.999, spread
  0.091 (its limit is 0.05).

The two sides of those pairs ran the same audio code, so the ratio of 1.146 is
noise, or a real bias in how latency is measured. The limit is 1.14 and the
spread limit 0.53 (`tools/sound-parity/budget.ron`), so it missed by 0.006.

A second sample: PR 240 (ticket 3031), CI run 41503572435, gave
`browser-chromium perf:latency_ms` a median ratio of 1.180 and spread 0.216, again
quad-snd against itself, with `linux-native perf:latency_ms` inconclusive (spread
0.132) in the same run.

`budget.ron` says each `max_ratio` is 1 plus twice the worst distance from 1 of
an old-against-old median, over three runs. Three runs may not show how far this
metric moves. `tools/sound-parity/README.md`, *Performance*, is deliberate that a
`failed` cell is never measured again, as a slowdown shows in the median however
often it is, and that a limit is raised only with new calibration runs, never to
pass. So one such run blocks a PR, and `ci-result` with it, and only a person
re-running the job clears it.

Native latency shows the same kind of noise. On PR 232 (ticket 2210), CI run
41366247884, `linux-native perf:latency_ms` stayed `inconclusive` on all three
attempts: medians 1.011, 0.990 and 0.999, spreads 0.103, 0.122 and 0.120 against a
limit of 0.09. Its samples sit near 26.5 or 30.5 ms, about one 4 ms tick of the
probe's loop apart, so the spread may be that tick rather than the machine. The
measurement in this ticket covers `linux-native` too.

ADR-0066 and ticket 2573 hold what the gate is for.

**From 3031 (done):** `old_new`'s `browser-in-house` `latency_ms` limit in `budget.ron`
comes from one run of the worklet's lead as it is now (CI run 41419687839, ratio 1.80,
spread 0.41). Calibrate it with the same runs as `browser-chromium`'s.

## Nick input

None.

## Scope

**In:**
- Measure `latency_ms` old against old on CI's runners, in the browser, over
  enough runs to show its real distribution (at least twenty), keeping the
  samples. Include whether the ratio leans one way: if the second side of a pair
  always measures slower, that is a bias in `capture.py`, to fix rather than to
  allow for.
- Choose the fix the numbers support, and say why in `budget.ron` beside the
  limit, with the runs named:
  - a new `max_ratio` by the same rule (`budget.ron`'s comment), over more runs;
  - or more pairs for this metric, if that narrows its median enough;
  - or the bias removed.
- Look at the two `decode_ogg_ms` spreads the same way, as they were retried by
  the job and may share the cause.

**Out (do not do):**
- re-measuring a `failed` cell, unless the numbers show a limit can't separate
  noise from a slowdown; then it needs an ADR superseding what the README says;
- loosening a limit with no new runs;
- changing what counts as a sound path (2039);
- skipping the job on a PR.

## Implementation steps

1. Read `tools/sound-parity/README.md`, `budget.ron`, `capture.py`, `judge` in
   `crates/xtask/src/sound_parity.rs` and the `sound-parity` job in `ci.yml`.
2. Run the calibration on CI's runners (a `workflow_dispatch` on a branch, or the
   job repeated), and keep its numbers in the PR.
3. Make the change the numbers support, and update `budget.ron`'s comment.
4. Run the `document` skill, then the gates.

## Acceptance criteria

- [x] The calibration's runs and numbers are in the completion notes, with
      whether the ratio leans one way.
- [x] `budget.ron` says why its limit is what it is, naming those runs.
- [x] The injected slowdown still fails, on every metric where it `slows`.
- [x] The `sound-parity` job passes on ten runs in a row with no sound change.
- [x] The book's chapter is updated (2802), or the completion notes have a
      `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Existing: `sound_parity`'s tests, unchanged unless the rule changes.
- CI: the ten runs above.

## Completion notes

**The calibration.** Every `perf.jsonl` (and `perf-attempt-N.jsonl`) that CI's
`sound-parity` job kept, from 2573's merge to 2026-11-02: 58 CI runs, from
41280120626 to 41503572435, 77 native attempts and 51 in the browser, each nine
pairs. The perf workload hasn't changed since 2573, so they are the job repeated
on CI's pinned image. Run 41588549868, on the scratch branch
`t2042-calibration` (never merged), gathered their timings into its `timings`
artifact. Earlier artifacts, from 2573's own development while its probe and
budget still changed, are left out (two of them failed native `peak_kib`).

Old against old, the gate's own median and interquartile range of nine ratios:

| Cell | Medians | Worst \|median − 1\| | Ranges, median / worst | Over the old limits |
| ---- | ------- | -------------------- | ---------------------- | ------------------- |
| browser `latency_ms` | 0.866–1.180, sd 0.070 | 0.180 | 0.162 / 0.400 | 2 of 51 failed (1.146, 1.180) |
| browser `decode_ogg_ms` | 0.873–1.089 | 0.127 | 0.004 / 0.192 | 1 failed (1.089), 1 inconclusive |
| native `latency_ms` | 0.925–1.099 | 0.099 | 0.034 / 0.227 | 18 of 77 inconclusive |
| native `decode_ogg_ms` | 0.956–1.031 | 0.044 | 0.030 / 0.116 | 15 of 77 inconclusive |

**No lean.** Per pair, the side measured second against the first: browser
latency slower in 220 pairs and faster in 239 (median 0.990); native latency 342
and 351; native Ogg 370 and 322; browser Ogg 115 and 116, the rest equal. So
there is no bias in `capture.py` to remove. Native latency's samples sit near
26.5, 30.5 and 37.5 ms (283, 248 and 166 of 1,386 at those whole ms), a tick of
the probe's loop and the mixer's period apart, as the ticket guessed: a run's
ratios straddle them, which is its range. Browser latency is one hump, 21 to 37
ms around 29, so a few ms of jitter is a large ratio.

**The fix: the same rule over these runs**, for the four cells: `max_ratio` 1 +
twice the worst distance, `max_spread` twice the worst range, rounded up. Browser
latency 1.36 (its spread stays 0.53, never reached), browser Ogg 1.26 and 0.39,
native latency 1.20 and 0.46, native Ogg 1.09 and 0.24. Budget version 5.
Every other limit was never exceeded in these runs, and stays. Native
`decode_wav_ms` (2 of 77 over 0.05) and `cpu_ms` (1 of 77 over 0.10) were
inconclusive now and then, which the job's retry is for.

More pairs were the other choice, and don't do it: an interquartile range is the
samples' own and doesn't narrow with more of them, and a median narrows only with
the square root of their number, so holding browser latency at 1.14 with the
same margin needs about six times the pairs, minutes on every run of the job.

**The slowdown still fails.** Its least median over the 77 and 48 attempts that
ran it: natively 1.95 (Ogg and WAV decode, file reads) and 10.9 (CPU); in the
browser 1.60 (Ogg decode), 1.73 (CPU), 1.80 (file reads), 2.00 (WAV decode).
Each is over its new `max_ratio`. Latency doesn't `slows`: its limits still
catch a replacement adding a buffer of 10 ms.

Under the new limits the same attempts give no failed cell, and 3 of 77 native
attempts inconclusive, which the job measures again.

**Ten runs in a row.** CI run 41589579520 on `t2042-calibration`: `ci.yml`'s
`sound-parity` job ten times at once on this change, with no sound change
beside it. All ten passed. Nine passed on their first measure; run 7's native
`decode_wav_ms` was inconclusive (range 0.064 against 0.05, a limit this ticket
leaves) and passed when the job measured again. A first try of the ten failed in
the gate's own tests, as the scratch workflow named the sound chain without a
`footprint.ron` line; it now deletes itself from its checkout first.

**Gates.** This changes `budget.ron` and the ticket: the skill sync, `cargo fmt`,
`cargo test -p xtask`, the ticket lint, links, the docs map and typos pass
locally. No crate's code changed, so the rest of the code gates are CI's.

Book: none, the sound gate's budget is the engine's own CI, not something a game
uses.

No break: no public API, format or golden value changed. Nick input: none.
