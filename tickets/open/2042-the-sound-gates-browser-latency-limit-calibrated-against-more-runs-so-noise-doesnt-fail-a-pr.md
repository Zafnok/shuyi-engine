---
id: "2042"
title: "The sound gate's browser latency limit calibrated against more runs, so a noisy machine doesn't fail a pull request"
type: bug
milestone: "Engine: public"
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
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

- [ ] The calibration's runs and numbers are in the completion notes, with
      whether the ratio leans one way.
- [ ] `budget.ron` says why its limit is what it is, naming those runs.
- [ ] The injected slowdown still fails, on every metric where it `slows`.
- [ ] The `sound-parity` job passes on ten runs in a row with no sound change.
- [ ] The book's chapter is updated (2802), or the completion notes have a
      `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Existing: `sound_parity`'s tests, unchanged unless the rule changes.
- CI: the ten runs above.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
