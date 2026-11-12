---
id: "2068"
title: The sound gate's timing cells fail 3 runs in 20 after 2050
type: bug
milestone: "Engine: in-house platform"
model: opus-5.5 / gpt-6.1-sol
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2068 — The sound gate's timing cells fail 3 runs in 20 after 2050

## Context

Ticket 2050 ran the `sound-parity` pair twenty times on one revision (CI run
42507044770, scratch branch `t2050-calibration`, never merged). No
`browser-in-house` cell failed. Six jobs failed on other platforms. Three of them
were `windows-in-house` gaps, which 3089 owns (jobs 6, 10 and 17). The other
three were timing cells, and no open ticket owns them:

- job 2: `browser-chromium perf:cpu_ms` failed, median ratio 1.051, spread 0.071,
  over its `max_ratio` of 1.05;
- job 13: `windows-in-house old-new:cpu_ms` failed, new against old: median ratio
  4.093, spread 0.357 (new 115.46, old 28.20), over its `max_ratio` of 4.0;
- job 15: `browser-chromium perf:file_io_ms` failed, median ratio 1.333, spread
  0.474, over its `max_ratio` of 1.25.

Their artifacts are `sound-parity-run-2`, `-13` and `-15` of that run, while they
last. `tools/sound-parity/budget.ron` says how each limit was set: 2042 and 2046
for `browser-chromium`, 3031 and 2044 for `old_new`. Its rule is to find the lean
and fix its cause, never to widen a limit to pass, and to name the runs beside a
limit that moves.

The flaky-gates ledger (2064) lists a sound cell under an open ticket. These
three cells are this ticket's.

**From 3089 (done):** its three twenty-job runs showed more of these timing cells,
each of two runs of one build:

- `linux-native perf:peak_kib`: jobs 3 and 14 of CI run 43131815627, medians 1.056
  and 1.057.
- `browser-in-house perf:file_io_ms`: job 8 of CI run 43143877256, median 1.316.
- `windows-in-house perf:cpu_ms`: job 10 of CI run 43143877256, median 1.195
  against 1.15, its spread 0.265, the process's cycles rising in both sides partway
  through. The same job's `perf:decode_ogg_ms` was inconclusive (spread 0.102).

3089 raised the in-house output's audio thread on Windows. Its runs'
`windows-in-house perf:cpu_ms` medians were 0.951 to 1.128 before that and 0.947
to 1.195 after, centred on 1. They are this ticket's cells too.

## Nick input

None.

## Scope

**In:**
- For each of the three cells, the pairs in its job's `perf.jsonl` or
  `old-new.jsonl` against a passing job's: whether one side leans (the side
  measured second, a pair, a first run), or the machine was noisy.
- Fix a lean in the capture (`tools/sound-parity/capture.py`) or the probe
  (`crates/app/examples/sound_probe.rs`) if one is found.
- If it is noise, the rule `budget.ron` already applies: more runs, judged as
  2042 and 2046 did, and each limit that moves names its runs and raises
  `version`.
- Twenty `sound-parity` runs on one revision, with the result in the notes.

**Out (do not do):**
- A limit raised without runs named beside it.
- The `windows-in-house` gaps (3089).
- The in-house web backend's start (2050, done).

## Implementation steps

1. Download the three artifacts and a passing job's from CI run 42507044770.
2. Compare each failing cell's pairs with the passing job's, pair by pair, and
   with the side measured second against the first.
3. Fix a lean where it is, or measure again by `budget.ron`'s rule.
4. Run the twenty jobs and judge every cell.

## Acceptance criteria

- [ ] The notes say what each of the three failures was.
- [ ] Twenty `sound-parity` jobs on one revision pass these three cells, or each
      failure is named in the notes with its cause.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: a test in `tools/sound-parity/capture_test.py` for any change to the
  capture, or in `crates/xtask/src/sound_parity/tests.rs` for one to the gate.
- Integration: the twenty runs above, in CI.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
