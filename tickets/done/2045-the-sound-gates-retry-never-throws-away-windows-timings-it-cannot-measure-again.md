---
id: "2045"
title: "The sound gate's retry never throws away Windows timings it can't measure again"
type: bug
milestone: "Engine: in-house platform"
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-03
---

# 2045 — The sound gate's retry never throws away Windows timings it can't measure again

## Context

CI's `sound-parity` job (`.github/workflows/ci.yml`) runs `cargo xtask
sound-parity gate` up to three times. After a pass whose only unmet cells are
`inconclusive`, it moves each such platform's `perf.jsonl` to
`perf-attempt-N.jsonl` and measures that platform again with `capture.py`
(`tools/sound-parity/README.md`, *Performance*). Its `case` measures
`linux-native`, `browser-chromium` and `browser-in-house` only. The Windows
timings come from the `sound-parity-windows` job, which measures its own
inconclusive cells again before it uploads them.

So when the gate finds a `windows` or `windows-in-house` cell `inconclusive`, the
loop moves that platform's `perf.jsonl` aside and measures nothing. The next pass
fails it as `no samples in target/sound-parity/windows/perf.jsonl`, a failed cell
from timings that were only noisy.

Ticket 3088's twenty runs hit it (CI run 41689870848, job `sound-parity (18)`).
The Windows job measured `windows` three times (inconclusive on `decode_wav_ms`,
then on `cpu_ms`) and uploaded the third. The gate found that one inconclusive
on `peak_kib`, moved it aside, and failed on `no samples`.

## Nick input

None.

## Scope

**In:**
- The gate step in `ci.yml`'s `sound-parity` job: a platform it can't measure
  again (`windows`, `windows-in-house`) keeps its `perf.jsonl`, and its
  inconclusive cell stands as the verdict, failing the job as an inconclusive
  cell does. The Windows job has already measured it again.
- The same in any other copy of that loop (search `.github/` for `perf-attempt`).
- The README's *Performance* paragraph, if its words about who measures again
  change.

**Out (do not do):**
- measuring Windows again from the Linux job, or a third attempt in the Windows
  job;
- any limit or spread in `budget.ron` (2044);
- the in-house bugs (3089).

## Implementation steps

1. In the gate step, measure again only the platforms the `case` knows. If an
   inconclusive platform is one it doesn't, stop the loop and fail with the
   matrix's cells, without moving that platform's file.
2. Say in the step's comment that Windows' timings are measured again in its own
   job.
3. Check the loop's shell by hand, with a `matrix.json` in which only a `windows`
   cell is inconclusive, and with one in which only `browser-in-house` is (which
   must still be measured again). The step's logic can sit in a small script
   under `tools/sound-parity/` if that makes it testable, with its test run in
   the job.

## Acceptance criteria

- [x] An inconclusive `windows` or `windows-in-house` cell leaves its
      `perf.jsonl` in place, and the job fails on the inconclusive cell, never on
      `no samples`.
- [x] An inconclusive `linux-native`, `browser-chromium` or `browser-in-house`
      cell is still measured again, up to twice.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- The two cases of step 3, by hand or by a script's test, written in the
  completion notes.
- CI: the job's own run on the PR.

## Completion notes

- The gate step's choice of what to measure again moved into
  `tools/sound-parity/retry.py`. Given `matrix.json`, it prints the inconclusive
  platforms of this job (`linux-native`, `browser-chromium`, `browser-in-house`),
  one per line. A mandatory inconclusive cell of any other platform (`windows`,
  `windows-in-house`) stands: it prints that cell, exits 1, and the loop stops
  without moving any `perf.jsonl` aside. A Windows cell that isn't mandatory
  doesn't stop the others being measured again.
- `retry_test.py` tests it and runs in the job, as a step before the gate. Its
  cases: an inconclusive `windows` or `windows-in-house` cell stands, with
  nothing printed; an inconclusive `browser-in-house` cell is measured again; each
  of this job's platforms is printed once; a Windows cell that isn't mandatory
  doesn't stop the rest.
- Step 3 by hand: the loop's shell, with `cargo`, `jq` and `capture.py` stubbed,
  on a `matrix.json` whose only inconclusive cell is `windows perf:peak_kib`,
  stopped after the first pass with every `perf.jsonl` in place. With only
  `browser-in-house` inconclusive, it measured it again twice and failed after
  the third pass.
- Deviation: the script instead of inline `jq`, as step 3 allows. It made the
  logic testable, and `jq` isn't on Nick's machine to check it by hand.
- The step's comment and the README's *Performance* paragraph say the Windows
  job measures its own timings again.
- No other copy of the loop: the Windows job's own retry already measures only
  its platforms.

No break.

Book: none, only CI's sound gate changed, and no public item.
