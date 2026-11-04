---
id: "2046"
title: "The sound gate's native file read limit calibrated against more runs"
type: bug
milestone: "Engine: in-house platform"
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-04
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

- [x] The calibration's runs and numbers are in the completion notes, with
      whether a ratio leans one way.
- [x] `budget.ron` says why each changed limit is what it is, naming the runs.
- [x] The injected slowdown still fails, on every metric where it `slows`.
- [x] No `linux-native` or `browser-chromium` performance cell fails in ten runs on
      one revision.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Existing: `sound_parity`'s tests.
- CI: the ten runs.

## Completion notes

**The runs.** Every `sound-parity` artifact from 2042's merge (59785356,
2026-11-03 17:50 UTC) to 2026-11-04: 59 jobs in 10 CI runs, 41689870848 to
41809586244. Twenty are 41689870848 (3088's scratch branch), twenty 41785090975
and ten 41796857018 (2044's), the other nine pull requests and `main`. That is 62
native attempts and 61 in the browser, each nine pairs; the perf workload is
unchanged since 2042. Run 41879925714, on the scratch branch `t2046-calibration`
(never merged), downloaded them on a runner and kept the `.jsonl` files and
matrices. Each attempt is judged as `ratios` and `judge` do.

**Old against old** (only the cells with a number over its limit; the others
stayed inside all of theirs):

| Cell | Medians | Ranges, worst | Over the old limits | New limits |
| ---- | ------- | ------------- | ------------------- | ---------- |
| native `file_io_ms` | 0.959–1.057 | 0.134 | 1 failed (1.057, 41785090975 job 13) | ratio 1.05 → 1.12 |
| native `decode_wav_ms` | 0.981–1.026 | 0.055 | 1 inconclusive (41785090975) | spread 0.05 → 0.12 |
| browser `decode_ogg_ms` | 0.871–1.087 | 0.587 | 1 inconclusive (41796857018) | spread 0.39 → 1.18 |
| browser `cpu_ms` | 0.953–1.035 | 0.112 | 1 inconclusive (41689870848) | spread 0.09 → 0.23 |

Each new number is the budget's rule rounded up: 1 plus twice the worst distance
of a median from 1, twice the worst range. The three inconclusive attempts passed
when the job measured again; only file reads failed a job. Native file reads'
spread (0.134 against 0.17) and the other three's ratios were never exceeded, so
they stay, as the ticket's **Out** says. The browser Ogg range is a frame's
quantum: a decode is seen to end at a frame, so its times fall on steps of 17 ms
(133 against 250 ms in one pair). Its median still holds at 1.26, with the
injected slowdown's least at 1.76.

The other limits, over the same attempts (worst median, worst range): native
decode Ogg 1.045 / 0.072, CPU 1.042 / 0.081, memory 0.969 / 0.098, latency
1.127 / 0.150; browser file reads 1.104 / 0.409, WAV 1.008 / 0.015, memory
1.002 / 0.004, latency 0.877 / 0.326. None was exceeded.

**Does a ratio lean?** Per pair, the side measured second against the first:
native file reads slower in 280 pairs and faster in 269, median 1.001, so the
failure was noise, not a bias in `capture.py`. Every metric's median is within
0.002 of 1 except browser latency: 0.978, the second side faster in 299 pairs to
250. 2042 saw the same side, 239 to 220. It is 2 % and the turns a, b, b, a
cancel it in the median, so it only widens the range, which at 0.326 is far
under the 0.53 limit. Nothing to fix.

**The slowdown still fails.** Its least median over the 59 jobs: natively 1.84
(file reads), 1.94 (WAV and Ogg), 10.9 (CPU); in the browser 1.65 (file reads),
1.69 (CPU), 1.76 (Ogg), 2.00 (WAV). In the ten runs, browser file reads fell to
1.53. Each is over its `max_ratio`, the new 1.12 for native file reads
included.

**Ten runs.** CI run 41881473138 on `t2046-calibration` (08061a88): `ci.yml`'s
`sound-parity-windows` and `sound-parity` jobs ten times each on this change, as
2044's scratch workflow did. Every `linux-native` and `browser-chromium` cell
passed on its first measure: 140 `perf:` cells and 80 `slowdown:` cells, with
nothing measured again. The worst: browser file reads median 1.135 (limit 1.25),
spread 0.240; native latency spread 0.167; browser latency median 0.902. All ten
Windows jobs passed. Job 9's `sound-parity` failed only on `windows-in-house
perf:underruns`, two gaps: that is 3089, outside this ticket.

**Docs.** `budget.ron` says why each changed limit is what it is, and which runs
it came from (version 8). The README's *Performance* section still holds.

**Gates.** This changes `budget.ron` and the ticket. Locally: the skill sync,
`cargo fmt`, `cargo test -p xtask` (`sound_parity`'s 47), the ticket lint, links,
the docs map and typos all pass. No crate's code changed, so CI runs the rest of
the code gates.

Book: none, the sound gate's budget is the engine's own CI, not something a game
uses.

No break: no public API, format or golden value changed. Nick input: none.
