---
id: "2044"
title: "The sound gate's Windows and decoder limits calibrated against more runs"
type: bug
milestone: "Engine: in-house platform"
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-04
---

# 2044 — The sound gate's Windows and decoder limits, calibrated

## Context

Ticket 2042 set four of `tools/sound-parity/budget.ron`'s performance limits
again from 58 CI runs, by the budget's own rule, after limits from three runs
failed pull requests that changed no sound. Its completion notes say how it was
done. The limits that 3031 added for `windows` and `windows-in-house`, and the
`decoders` limits of 2210, still come from three runs.

2042 then ran the `sound-parity` job 20 times on one revision with no sound
change (CI run 41595469804). Beside the in-house bugs (3088, 3089):

- job 12: `windows perf:cpu_ms` failed at a median ratio of 1.086 (limit 1.05),
  and `windows perf:decode_wav_ms` and `decode_ogg_ms` were inconclusive
  (spreads 0.089 and 0.056);
- job 4: `windows perf:latency_ms` inconclusive, spread 0.177 against 0.15;
- job 7: `linux-native decoder-speed:ogg` inconclusive on all three attempts,
  spreads 0.052 to 0.059 against 0.05 (`decoders`, ratios 0.81 to 0.84).

So one more limit in about seven fails a pull request on its own.

## Nick input

None.

## Scope

**In:**
- The `performance` limits of `windows` and `windows-in-house`, and every
  `decoders` ratio limit, over at least twenty runs of the job: the samples are
  in each run's `sound-parity` and `sound-parity-windows` artifacts (2042 gathered
  58 runs' with a scratch workflow that downloads them on a runner).
- Whether a ratio leans one way (the side measured second against the first),
  which would be a bias in the capture to fix rather than to allow for.
- Each new limit by the budget's rule, with the runs named beside it, and the
  budget's `version` raised.

**Out (do not do):**
- loosening a limit with no new runs, or one the runs never exceed;
- re-measuring a `failed` cell (the README, *Performance*);
- `linux-native` and `browser-chromium`'s `performance` limits, which 2042 set;
- the in-house bugs (3088, 3089).

## Implementation steps

1. Read `tools/sound-parity/README.md`, `budget.ron`, 2042's completion notes and
   `crates/xtask/src/sound_parity.rs` (`judge`, `old_new_cells`) and
   `sound_parity/decoders.rs`.
2. Gather the runs' samples since 3031 merged (12e1256e), including the 20 of
   run 41595469804, and run more on a scratch branch if they are fewer than
   twenty.
3. Set the limits the numbers support, and say why in `budget.ron`.
4. Check that the injected slowdown still fails on every metric where it
   `slows`.
5. Run the job ten times on one revision, and keep the result in the notes.
6. Run the `document` skill, then the gates.

## Acceptance criteria

- [x] The calibration's runs and numbers are in the completion notes, with
      whether a ratio leans one way.
- [x] `budget.ron` says why each changed limit is what it is, naming the runs.
- [x] The injected slowdown still fails, on every metric where it `slows`.
- [x] No cell this ticket owns fails in ten runs on one revision.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Existing: `sound_parity`'s tests.
- CI: the ten runs.

## Completion notes

**The runs.** Every `sound-parity-windows` and `sound-parity` artifact from 3031's
merge (12e1256e) to 2026-11-04: 12 CI runs, 41595169987 to 41785090975, three of
them 20 numbered copies of the jobs each (41595469804 and 41689870848, made for
2042 and 3088, and 41785090975 on this ticket's scratch branch
`t2044-calibration`, never merged). That is 69 Windows jobs, 101 attempts of
`windows`' timings and 86 of `windows-in-house`'s, and 72 native jobs. Run
41783186508 gathered the first 49 Windows and 52 native jobs' `.jsonl` files and
matrices on a runner; the last 20 were read locally. The cancelled run 41688154921
is left out. Each attempt is judged as `ratios` and `judge` do.

**Windows, old against old** (101 and 86 attempts; the limits were three runs'):

| Cell | Medians | Ranges, worst | Over the old limits (first 49 jobs) | New limits |
| ---- | ------- | ------------- | ----------------------------------- | ---------- |
| `windows decode_wav_ms` | 0.971–1.037 | 0.153 | 19 of 81 inconclusive | spread 0.05 → 0.31 |
| `windows decode_ogg_ms` | 0.965–1.053 | 0.194 | 11 inconclusive; 1.053 failed in 41785090975 | 1.05 → 1.11, 0.05 → 0.39 |
| `windows cpu_ms` | 0.850–1.094 | 0.341 | 2 failed, 13 inconclusive | 1.05 → 1.30, 0.10 → 0.69 |
| `windows peak_kib` | 0.970–1.025 | 0.106 | 5 inconclusive | spread 0.05 → 0.22 |
| `windows latency_ms` | 0.917–1.062 | 0.177 | 1 inconclusive | spread 0.15 → 0.36 |
| `windows-in-house decode_wav_ms` | 0.950–1.069 | 0.150 | 1 failed, 6 inconclusive | 1.05 → 1.14, 0.10 → 0.30 |
| `windows-in-house cpu_ms` | 0.927–1.065 | 0.187 | 8 inconclusive | 1.05 → 1.15, 0.10 → 0.38 |
| `windows-in-house latency_ms` | 0.758–1.216 | 1.139 | 5 inconclusive | spread 0.50 → 2.28 |

Each by the budget's rule: `max_ratio` 1 plus twice the worst distance of a median
from 1, `max_spread` twice the worst range, rounded up. The rest were never
exceeded and stay. Under the old limits, with the job's own retry, 5 of the first
49 jobs failed on `windows` and 2 on `windows-in-house`; under the new, none of the
69. In-house latency's samples fall anywhere from 6 to 24 ms, a period of the
endpoint's buffer, so its range is wide. Its median still catches a buffer of 10 ms
more (about 1.75 against 1.25).

**No lean in the timings.** The median ratio of the side measured second to the
first is within 0.005 of 1 on every metric of both platforms (`windows` file reads
453 pairs slower to 452 faster, CPU 465 to 444, memory 473 to 433).

**The decoders leaned, and that was fixed.** Grouped by which side went first,
`decoder-speed` was biased:
- Windows WAV: a process's first decode was slow. New was 4.8 % slower when it went
  first, old 1.2 %. The even pairs' median ratio was under the odd pairs' in 48 of
  49 jobs, by 0.059, and the ranges reached 0.109 (23 of 49 over 0.05).
- Native Ogg: new was 2.7 % slower just after old's decode. That was 49 of 52 jobs,
  by 0.025, with ranges up to 0.059. This is job 7's inconclusive cell in
  41595469804.

So the probe's `decoders` (`crates/app/examples/sound_probe.rs`) now decodes each
side once untimed, then times them in turns old, new, new, old (new, old, old, new
on odd pairs). It takes twice the decodes. Over the
20 jobs of 41785090975 with that, Windows WAV leans in 5 of 20 (−0.002) and native
Ogg in 13 of 20 (+0.003). The worst ranges were 0.029 and 0.040 on Windows (WAV,
Ogg) and 0.022 and 0.037 natively. Native WAV now leans a little (new 3 % slower
when first, 17 of 20, by 0.018), inside a range of 0.022 against a limit of 0.05.
It is left. Windows WAV's spread goes from 0.20, which held the lean, to 0.06,
twice its worst. The other three were never exceeded and stay at 0.05.

**The slowdown still fails.** Its least median, over every attempt: Windows 1.77
(Ogg decode), 1.79 (file reads), 1.86 (WAV), 57.7 (CPU); `windows-in-house` 1.84,
1.86, 1.93, 26.2; decoders 1.93 and 1.95 on Windows, 1.96 and 1.99 natively. Each
is over its `max_ratio`, and every `slowdown:` and `decoder-slowdown:` cell passed
in the ten runs.

**Ten runs.** CI run 41796857018, the jobs ten times on this change's revision
(08c16673 on the scratch branch, budget version 7). Nine judged jobs passed every
cell, none measured again by the gate. The worst of this ticket's cells: `windows
file_io_ms` median 1.057 (limit 1.25), `windows cpu_ms` spread 0.205, in-house
latency median 1.097 and spread 0.469, decoders spread at most 0.041. Job 9's
Windows recorder stopped with "the loopback lost frames 2 times" before any timing
was taken, so its gate was skipped: **2047**. Run again on the same revision,
both of job 9's jobs passed: ten of ten judged runs pass every cell.

**Other cells that failed in 41785090975**, none this ticket's:
- job 1: `windows-in-house perf:underruns`, one gap (3089);
- job 4: `browser-in-house scenario:wav_44k` (the in-house web bugs, 3088);
- job 13: `linux-native perf:file_io_ms`, median 1.057 against 2042's 1.05: **2046**.

3084 now waits for 2046 and 2047 too.

**Deviation.** The ticket asked for the limits over the runs as they were. Its own
"is there a lean" question found one in the decoders. The fix changed the probe,
so the decoder limits come from the 20 runs after it, not the 52 before.

**Docs.** `tools/sound-parity/README.md` (*The decoders*) says how the measure takes
turns, and `budget.ron` says why each limit is what it is. The scratch scripts
generated the workflow from `ci.yml`'s two jobs, numbered, with the workflow file
deleting itself before the gate's tests, as 2042's did.

**Gates.** The skill sync, `cargo fmt`, the workspace's clippy, the probe's clippy
natively and for wasm32, `cargo test -p xtask`, the engine lint, the ticket lint,
links, the docs map, the changelog and typos pass locally. The rest is CI's.

Book: none, the sound gate's budget and probe are the engine's own CI, not something a game uses.

No break: no public API, format or golden value changed. Nick input: none.
