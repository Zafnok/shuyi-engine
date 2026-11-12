---
id: "2064"
title: Flaky gates have a ledger, and a calibration run is one reusable workflow
type: infra
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-12
---

# 2064 — Flaky gates have a ledger, and a calibration run is one reusable workflow

## Context

The retro of 2026-11-08 found the sound gate worked as ten tickets with
one root cause. Its limits were first set from three runs, and nothing
checked the recordings' integrity:

- 2042 and 2044–2048, and 3088 (closed);
- 2049, 2050 and 3089 (done since).

Each calibration ticket built its own repeat-the-job workflow on a
scratch branch, and six `t*-calibration` branches are left on the
remote. #312 (2048) showed the right method: find where the loss fell,
fix its cause, never widen a limit, name the runs beside each limit.
Even so, `sound-parity` still fails 4 runs in 20 on the in-house
backends, and every sound-path PR meets it.

Other flakes were re-run with no record:

- `sound-parity` in #264, #270 and #293;
- `api-label` "No verdict" in #258;
- a GitHub artifact-upload outage in #263;
- one mutants shard in #264.

2049 and 2050 were also missing from 3084's `blocked_by` (the retro's PR
added them), unlike 2044, 2046 and 2047.

**From 2050 (done):** its cell (`browser-in-house`, a sound cut at its start)
passed in all twenty runs of CI run 42507044770, so it needs no entry. That run's
other failures are 3089's (`windows-in-house` gaps, jobs 6, 10 and 17) and 2068's
(three timing cells: `browser-chromium perf:cpu_ms` and `perf:file_io_ms`,
`windows-in-house old-new:cpu_ms`). They are the entries to list, under those two
tickets.

**From 3089 (done):** the `windows-in-house` gaps were the output's own, and are
fixed, so 3089 needs no entry. It left three scratch branches for step 5:
`t3089-calibration`, `t3089-calibration-2` and `t3089-baseline` (the last doesn't
match `t*-calibration*`).

## Nick input

None.

## Scope

**In:**
- `tools/flaky.ron`: each known flaky cell (job, test or sound cell),
  its ticket, the date seen, and its measured rate. `ticket-lint` checks
  that each entry's ticket is open, so fixing a flake means removing its
  entry.
- CI re-runs a failed job **once** when its failure matches a listed
  cell. The re-run shows as a warning naming the entry, never as a
  pass with no trace. A cell not listed gets no re-run.
- `.github/workflows/calibrate.yml`, a reusable `workflow_dispatch`
  workflow. It takes a job name and a count, runs that job N times on a
  ref, and uploads the artifacts together with a summary table. It
  replaces the per-ticket calibration branches.
- A rule in `run-gates` (*When CI fails but local passes*): a re-run
  goes in the PR body with its run link, and a second sighting of the
  same flake gets a `tools/flaky.ron` entry and a ticket.
- #258's calibration rule written into `run-gates`' sound parity
  section: "find the lean, fix its cause, never widen a limit; name the
  runs beside each limit".
- Delete the leftover `t*-calibration` branches on the remote, after
  listing them in the notes.

**Out (do not do):**
- Fixing 2049, 2050 or 3089 themselves.
- Retrying any cell that isn't listed.

## Implementation steps

1. Define the `flaky.ron` shape and its reader in `crates/xtask` (a new
   module), with a `ticket-lint` hook.
2. Write `calibrate.yml` with `inputs: job, count, ref`. It calls the
   existing job definitions through `workflow_call` where they allow,
   or a matrix over a count otherwise. Keep zizmor clean.
3. Add the single re-run in the `ci-result` aggregator, or a step after
   the job, matching failure names against `flaky.ron`.
4. Write the `run-gates` text, then run `pwsh -File
   scripts/sync-agent-skills.ps1`.
5. List and delete the scratch branches (`git ls-remote origin
   'refs/heads/t*-calibration*'`).

## Acceptance criteria

- [x] `tools/flaky.ron` lists the sound-parity cells of 2068 that are still open
      (2049, 2050 and 3089 are done).
- [x] An entry whose ticket is in `done/` fails `ticket-lint`.
- [x] `calibrate.yml`, dispatched with a count of 5 on one job, produces
      one summary of 5 runs (link in the notes).
- [x] No `t*-calibration` branch is left on the remote.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `flaky.ron` parsing and the open-ticket check.
- Workflow: one dispatch run.

## Completion notes

**What was done.**

- `tools/flaky.ron`, read by a new `crates/xtask/src/flaky.rs`. An entry is a
  cell (`Job(job)`, `Test(job, name)` or `Matrix(platform, check)`), its open
  ticket, the day it was seen, its rate and the runs that measured it. Its
  `matrices` name the jobs whose failures are the failed cells of a
  `matrix.json` (the sound gate's), and which of their platforms another job
  records (Windows). `ticket-lint` fails an entry whose ticket is done or
  doesn't exist (tried: 2068 changed to 2050 gave `names ticket 2050, which is
  done`).
- It lists 2068's six open cells: the three of 2050's run and the three 3089
  added to 2068.
- `.github/workflows/flaky-rerun.yml` runs when a CI run fails. It collects the
  attempt's jobs, logs and matrix (`scripts/flaky-failures.sh`), and
  `cargo xtask flaky rerun` decides. Only a first attempt whose every failed
  job failed on listed cells is re-run, once. A listed Windows cell re-runs
  `sound-parity / sound-parity-windows`, which records anew, as 3089 found a
  re-run of the judge alone judges the same recording. On a re-run attempt,
  by it or by hand, `ci-result` runs `cargo xtask flaky report` on the attempt
  before: a warning names the entries, or the failure no entry lists.
- The sound pair moved to `.github/workflows/sound-parity.yml`
  (`workflow_call`, with an artifact `suffix`); `ci.yml`'s
  `sound-parity` job calls it.
- `.github/workflows/calibrate.yml` (`workflow_dispatch`: `job` and `count`,
  1 to 40, on the dispatched branch) runs the job's reusable workflow as a
  matrix over the count, and its `summary` job writes one table
  (`cargo xtask flaky calibration`): each run's jobs and failed cells, and each
  cell's rate. It is in the step summary and the `calibration-summary`
  artifact. Five runs of `sound-parity`, one summary of 5 runs, all passed:
  https://github.com/Zafnok/shuyi-engine/actions/runs/43251721663 (artifact
  `calibration-summary`). GitHub dispatches a workflow only once it is on
  main, so this run is of `2f435bc4` plus one test-only commit (`7fd5c78d`, on
  a scratch branch `t2064-calibrate-test`, deleted after) that added a `push`
  trigger and defaulted `job` and `count` for it. A first test (run
  43246253019) failed in the gate's own tests on that commit's wording and
  showed that the `summary` job failed when no run had uploaded: fixed, the
  table now says `no matrix` for such a run.
- `run-gates`: the flake rules under *When CI fails but local passes*, and #258's
  calibration rule in the sound parity section. `docs/README.md` has a row for
  the ledger, and `tools/sound-parity/README.md` names the jobs' new names.
- Deleted from the remote, after listing them here: `t2044-calibration`,
  `t2046-calibration`, `t2047-calibration`, `t2048-calibration`,
  `t2049-calibration`, `t2050-calibration`, `t3088-calibration`,
  `t3089-calibration`, `t3089-calibration-2` and `t3089-baseline`.

**Deviations.**

- The re-run is a `workflow_run` workflow, not a step of `ci-result`: a run
  can't re-run itself while it runs. zizmor's `dangerous-triggers` finding on
  it is ignored in `.github/zizmor.yml`, with why: it checks out main, never
  the run's code, and reads the run's logs and matrix as data.
- `calibrate.yml`'s `job` is a string checked against a list in its `runs` job,
  not a `choice`, so that giving up the sound replacement can empty the list.
  It offers `sound-parity` only: a job is offered once its jobs live in a
  reusable workflow `ci.yml` calls. 2070 (compile-times) moves its job into
  one before its twenty runs; its ticket now says so.
- The ledger's code names no word of the sound chain (the cells are `Matrix`,
  the sound jobs are data in `tools/flaky.ron`), so
  `tools/sound-parity/footprint.ron` cuts only the ledger's sound part and
  `calibrate.yml`'s sound block, and owns `sound-parity.yml`. `gates.rs` lost
  its `sound parity (Windows)` gate: that job is no longer `ci.yml`'s, and the
  `sound parity` gate names both platforms.
- No `ref` input, though the ticket named one: a dispatch from main that
  checked out another ref would run unreviewed code with main's cache scope,
  which CodeQL flagged on #382 (cache poisoning; restoring only, with
  `rust-cache`'s `save-if`, didn't answer it, as any step can write a cache).
  A calibration runs on the branch it is dispatched on
  (`gh workflow run calibrate.yml --ref <branch>`), whose own scope its
  caches go to, and `github.sha` fixes its commit for every run.
- The jobs' names on GitHub are now `sound-parity / sound-parity` and
  `sound-parity / sound-parity-windows`. Branch protection requires
  `ci-result` alone, so nothing else moves.
- The other flakes of the context (`api-label` "No verdict" in #258, the
  artifact-upload outage in #263, the mutants shard in #264) were each seen
  once and get no entry: the rule gives one at a second sighting.

**Follow-ups.** No new ticket: 2068 and 2070 hold what this leaves, and each now
says so.

Book: none, the change is to CI and `cargo xtask`, which the book doesn't cover.

No break: `xtask` is not published, and no engine crate changed.

Nick input: none.
