---
id: "2046"
title: "The sound gate's native file read limit calibrated against more runs"
type: bug
milestone: "Engine: in-house platform"
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2046 — The sound gate's native file read limit, calibrated

## Context

Ticket 2042 set four of `tools/sound-parity/budget.ron`'s `linux-native` and
`browser-chromium` performance limits again from 58 CI runs, and left the others,
as no attempt had exceeded them. `linux-native perf:file_io_ms` stayed at
`max_ratio: 1.05`.

Ticket 2044 then ran the `sound-parity` job 20 times on one revision with no
sound change (CI run 41785090975, branch `t2044-calibration`, never merged). In
job 13, `linux-native perf:file_io_ms` failed: median ratio 1.057, spread 0.133,
against 1.05. So that limit fails a pull request that changes no sound now and
then. 2044's completion notes say how its runs were gathered and judged, and its
scratch scripts are described there.

## Nick input

None.

## Scope

**In:**
- `linux-native`'s `file_io_ms` limits, over every `sound-parity` artifact since
  2042 merged (59785356), at least twenty runs, the 20 of run 41785090975 among
  them: by the budget's rule, `max_ratio` 1 plus twice the worst distance from 1 of
  a median, `max_spread` twice the worst range.
- Whether the ratio leans one way (the side measured second against the first),
  which would be a bias in `capture.py` to fix rather than to allow for.
- The other `linux-native` and `browser-chromium` limits 2042 left, checked over
  the same runs: one that an attempt now exceeds is set again by the same rule.
- The runs named beside each changed limit in `budget.ron`, and its `version`
  raised.

**Out (do not do):**
- loosening a limit with no new runs, or one the runs never exceed;
- re-measuring a `failed` cell (the README, *Performance*);
- `windows`, `windows-in-house` and `decoders`, which 2044 set;
- the in-house bugs (3088, 3089).

## Implementation steps

1. Read `tools/sound-parity/README.md`, `budget.ron`, and the completion notes of
   2042 and 2044.
2. Gather the runs' `perf.jsonl` and `perf-attempt-N.jsonl` of `linux-native` and
   `browser-chromium` (a scratch workflow that downloads the artifacts on a runner
   and keeps only the `.jsonl` files and `matrix.json`, as 2044's did).
3. Compute each attempt's median and interquartile range as `ratios` in
   `crates/xtask/src/sound_parity.rs` does, and set the limits the numbers support.
4. Check that the injected slowdown still fails on every metric where it `slows`.
5. Run the job ten times on one revision, and keep the result in the notes.
6. Run the `document` skill, then the gates.

## Acceptance criteria

- [ ] The calibration's runs and numbers are in the completion notes, with
      whether a ratio leans one way.
- [ ] `budget.ron` says why each changed limit is what it is, naming the runs.
- [ ] The injected slowdown still fails, on every metric where it `slows`.
- [ ] No `linux-native` or `browser-chromium` performance cell fails in ten runs on
      one revision.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Existing: `sound_parity`'s tests.
- CI: the ten runs.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
