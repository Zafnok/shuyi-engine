---
id: "2070"
title: The compile-times job's Linux rebuild budget fails on the runner's speed, not the code
type: bug
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-15
---

# 2070 — The compile-times job's Linux rebuild budget fails on the runner's speed, not the code

## Context

CI's `compile-times` job (ticket 2009, [ADR-0049](../../docs/adr/0049-fast-compiles-a-new-games-rebuild-timed-in-ci-against-a-budget-no-debug-info-in-dev-and-no-dynamic-engine.md))
fails when a new `grid` game's median rebuild after a one-line change is
over `tools/compile-budget.ron`'s seconds for the runner's OS. Linux's is
1.6 s: the slowest of 2009's measures (1.04 s) plus 50 % for noise.
ADR-0049 already says the runners' speed varies by up to half between runs.

On 2026-11-10, PR #347 (2831, which changed only the website's stylesheet,
its README and a ticket, and no Rust) failed it twice on `ubuntu-latest`. The
code it timed was `main` at `9d8239eb` plus that CSS. The Linux rebuild
medians of that day's runs were:

| Run | Code | Median rebuild |
| --- | ---- | -------------- |
| `main` 42898812273 | `9d8239eb` | 1.19 s |
| `main` 42881494909 | `0312a3f8` | 1.57 s |
| PR #346, 42899778478 | `main` + 3082 | 1.06 s |
| PR #347, 42901429925, attempt 1 | `9d8239eb` + CSS | 1.65 s (1.67, 1.63, 1.65) |
| PR #347, 42901429925, attempt 2 | the same | 1.70 s (1.74, 1.70, 1.68) |

The same code measures from 1.06 s to 1.70 s, so the gate fails on the
runner's speed, not on what the change did. Raising the number would hide a
real slowdown as well as this one (ADR-0049: "a slower rebuild is found and
fixed, not budgeted").

2064 (a ledger of flaky gates) is about recording and re-running flakes in
general. This ticket fixes this gate's cause.

**From 2064 (done):** a calibration is one dispatch of `calibrate.yml`, never a
scratch branch (the `run-gates` skill, *When CI fails but local passes*). It runs
a job whose jobs live in a reusable workflow `ci.yml` calls, as
`sound-parity.yml`: this ticket moves the `compile-times` job into one, adds its
block to `calibrate.yml` and its name to the `runs` job's list, before measuring.

## Nick input

None.

## Scope

**In:**
- Find what makes the same code's rebuild vary by 60 % on `ubuntu-latest`:
  the runner's CPU model (`/proc/cpuinfo`), its core count, the cache's state,
  the order of the timed builds. Write the job's CPU model and core count
  into its summary beside the times.
- Change the gate so it follows the code and not the runner. One way is to
  time a fixed reference build in the same job and budget the rebuild as a
  ratio of it. Others are fine if the measures show they work. Record the
  choice in ADR-0049's successor (the `write-adr` skill), not by editing
  ADR-0049.
- Measure the new gate over at least 20 runs of the job on unchanged `main`
  with `calibrate.yml`, and name the runs beside the new numbers in
  `tools/compile-budget.ron`.

**Out (do not do):**
- Raising `rebuild.linux` alone.
- The Windows budget, unless the same measures show the same problem there.
  (They did: PR #443 measured 2.94 s against Windows' 2.9 s, and the
  first calibration spread Windows from 1.54 to 2.72 s. Windows is in.)
- The `dependencies` count.
- 2064's ledger and re-run rule.

## Implementation steps

1. Add the runner's CPU model and core count to `cargo xtask compile-times`'
   table and JSON (`crates/xtask/src/compile_times.rs`).
2. Run the job 20 times on unchanged `main` and tabulate the medians against
   the CPU model and the clean build's time.
3. From what step 2 shows, make the gate compare something that stays the same
   when only the runner changes, in
   `compile_times::check` and `tools/compile-budget.ron`'s format.
4. Run it 20 times again. The new gate passes all 20 on unchanged `main`. It
   still fails a branch that adds a real slowdown (for example
   `debug = true` in the template's `dev` profile).
5. Write the ADR, and update `docs/` where ADR-0049's budget is described.

## Acceptance criteria

- [x] The job's summary names the runner's CPU model and core count.
- [x] The gate passes 20 runs of 20 on unchanged `main`, and those runs are
      named in `tools/compile-budget.ron`.
- [x] The gate fails a branch with a real slowdown (step 4).
- [x] No budget number was raised without step 2's measures behind it.
- [x] An ADR supersedes ADR-0049's budget rule.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `compile_times::check` on the new budget format, over and under, and
  `parse_budget` on the new file.
- Integration: the 20-run calibrations above, with their run ids.

## Completion notes

What was done:

- `cargo xtask compile-times` names the runner's CPU model
  (`/proc/cpuinfo`, or the registry's `ProcessorNameString` on Windows)
  and its cores in the table and the JSON, and writes the JSON to a file
  with `--out`. It also builds a fixed reference crate
  (`compile_times::reference_source`, no dependencies) clean, just before
  each of the three rebuilds.
- The job moved to `.github/workflows/compile-times.yml`, which `ci.yml`
  calls and `calibrate.yml` offers (`-f job=compile-times`). Each run
  uploads its JSON as `compile-times<suffix>-<os>`.
- First calibration, on unchanged code: run 43775373036 at 9ac070d6, 20
  runs, all passing the old budget. Every runner had 4 cores. The CPU
  model explains the spread. On Linux the rebuild took 0.87 to 0.98 s on
  an EPYC 9V45, 1.00 to 1.16 s on an EPYC 9V74 and 1.30 to 1.33 s on an
  EPYC 7763; on Windows, from 1.54 s (9V45) to 2.72 s (Xeon 8573C). The
  rebuild over the reference build stayed within ×1.10 on both OSes
  (median of each pair: Linux 1.069 to 1.171, Windows 1.136 to 1.232).
  The rebuild over the clean build spread ×1.13 and ×1.31, and over the
  `check` rebuild ×1.27 and ×1.28. The order of the builds didn't matter:
  within a run the three rebuilds agree within a few hundredths.
- The gate now holds the median of each rebuild over its reference to
  `tools/compile-budget.ron`'s `rebuild_over_reference`: the highest
  measured plus 20 %, 1.40 on Linux and 1.48 on Windows. The seconds are
  gone. ADR-0120 records it and supersedes ADR-0049 §2.
- Second calibration of the new gate: run 43776381949 at 0f8c48fa, 20 of
  20 passed. Ratios were 1.066 to 1.174 on Linux and 1.153 to 1.235 on
  Windows, while the seconds spread 0.76 to 1.35 and 1.39 to 2.84, on
  two CPU models the first run hadn't drawn.
- A real slowdown, `debug = true` in the template's `dev` profile, failed
  every run on both OSes: run 43776383981, 3 runs, ratios 2.395 to 2.986.
  The calibrations ran from the plain branches `calibrate-2070` and
  `calibrate-2070-slowdown`, because the pre-push hook's ticket lint
  refuses a `t2070-` branch until the ticket is in `done/`.
- Docs: `README.md`'s *Fast compiles*, the docs map's row, the book's
  *Fast compiles* chapter, ADR-0049's status and the ADR index.

Deviations:

- Windows is in scope: Nick asked for it after PR #443 failed Windows'
  2.9 s budget at 2.94 s, and the first calibration showed the same
  problem there. The title still names Linux, as it was filed.
- The first calibration already timed the reference build, so that one
  set of 20 runs could compare every candidate ratio (step 2 with step
  3's measure in it). "Unchanged `main`" means the engine and templates
  of `main` at d32945ca, with only the xtask and the workflows changed.

Book: the *Fast compiles* chapter says the budget is a ratio to a reference
build on the same runner, and why.

No break: `xtask` is not published, and no engine crate changed.
`tools/compile-budget.ron`'s format changed (`rebuild` became
`rebuild_over_reference`): only this repository's CI reads it.
