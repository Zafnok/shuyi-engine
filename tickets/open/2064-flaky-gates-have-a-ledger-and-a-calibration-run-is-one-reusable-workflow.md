---
id: "2064"
title: Flaky gates have a ledger, and a calibration run is one reusable workflow
type: infra
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
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

- [ ] `tools/flaky.ron` lists the sound-parity cells of 2068 that are still open
      (2049, 2050 and 3089 are done).
- [ ] An entry whose ticket is in `done/` fails `ticket-lint`.
- [ ] `calibrate.yml`, dispatched with a count of 5 on one job, produces
      one summary of 5 runs (link in the notes).
- [ ] No `t*-calibration` branch is left on the remote.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `flaky.ron` parsing and the open-ticket check.
- Workflow: one dispatch run.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
