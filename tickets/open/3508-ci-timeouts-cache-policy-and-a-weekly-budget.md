---
id: "3508"
title: CI has a timeout on every job, caches saved from main only and pruned, and a weekly budget check
type: infra
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3508 — CI has a timeout on every job, caches saved from main only and pruned, and a weekly budget check

## Context

Nothing measures or caps CI's own cost. A blind-spot audit of the engine
(2026-11-22) found it, and this ticket's writer checked it again on
2026-11-23:

- **No timeouts.** `grep -c timeout-minutes .github/workflows/*.yml`
  gives 0 for `ci.yml` (32 jobs: 27 on a runner, 5 calling a
  workflow), `changes.yml`, `sound-parity.yml`, `export.yml`,
  `compile-times.yml`, `pages.yml` and the rest. Only
  `mutants.yml` (4 jobs: 15, 30, 90 and 90 minutes), `playtest.yml` (an
  input, `timeout-minutes`) and `pr-title.yml` set one. GitHub's default
  is 360 minutes, so a hung Xvfb capture or a hung browser holds a runner
  for six hours.
- **No cache policy.** `Swatinem/rust-cache` is used 35 times in the
  root's workflows (21 in `ci.yml`, 6 in `mutants.yml`, 2 in
  `sound-parity.yml`, one each in six others), and in the template's
  `ci.yml` and `export.yml`. None sets `save-if`, so a pull request's run
  saves caches of its own beside `main`'s.
- **The cache is at GitHub's limit.** `gh api
  repos/Zafnok/shuyi-engine/actions/cache/usage` gave 11,853,596,791
  bytes in 64 entries on 2026-11-22 and 10,528,934,439 bytes on
  2026-11-23, against the repository's 10 GB. GitHub then evicts the
  least recently used entries, and jobs build cold. Of the 60 entries
  listed on 2026-11-23 (`gh api …/actions/caches?sort=size_in_bytes`),
  56 were `main`'s (9.81 GB):
  - most keys are there twice, one per dependency hash (the key's last
    part, `…-786d4027` and `…-4cd8ffdc`): the superseded one stays until
    it is evicted;
  - the `tutorial` matrix job (`ci.yml`, `tutorial (${{ matrix.step }})`,
    one workspace per step) wrote 13 entries of 149–192 MB under one key
    prefix, `v0-rust-tutorial-Linux-x64-eb21f145-`, because its key names
    no step.
  - the two pull-request refs held 4 entries (1.03 GB), the
    `sound-parity` and `web-render` builds.
- **The runs.** One push to `main` (CI run 45078957922) ran 52 jobs (49
  that weren't skipped) for 240 runner-minutes and 56 minutes of wall
  time; `sound-parity / sound-parity` took 33 of them. Run 44474708379
  took 303 runner-minutes and 67 minutes (`templates (grid)`, 67). Run
  45108481844 took 165 minutes and 18. Measured with `gh api
  repos/Zafnok/shuyi-engine/actions/runs/<id>/jobs`, summing
  `completed_at - started_at`.

The repository is public, so standard runners' minutes cost nothing. What
does bite is the 10 GB cache, the account's concurrent jobs (20, 5 of
them macOS, on GitHub's Free plan), and the wall time an agent waits for
before it can merge. Every new target, template, example and tutorial
step adds a full build, so all three grow with the backlog.

The patterns to follow: `cargo xtask check-pins`
(`crates/xtask/src/pins.rs`, ADR-0037) walks every `.github/workflows`
folder and checks the `uses:` lines; `tools/flaky.ron` with `cargo xtask
flaky` (ticket 2064) is a ledger CI reads; `tools/compile-budget.ron`
(ADR-0049) is a budget CI holds a measurement to. ADR-0130 rules which
caching steps a job with the private files may have
(`cache-targets: ${{ hashFiles('.github/actions/private-files/action.yml') == '' }}`);
this ticket keeps that guard and adds to it.

## Nick input

None.

## Scope

**In:**
- `cargo xtask check-workflows`, a new gate: every job has a timeout,
  every cache is saved from the default branch only, and a matrix leg that
  builds its own workspace has a cache key of its own. The lint covers
  `launch.yml`'s jobs (3336), which set their own `timeout-minutes`.
- A `timeout-minutes` on every job of the root's workflows and of the
  template's (`templates/shuyi-default/**/.github/workflows/`), set from
  measured times.
- `save-if` on every `Swatinem/rust-cache` step there, and a `key:` per
  leg for the `tutorial` job, whose legs build different workspaces.
- `cargo xtask ci-budget`: measures a week of `main`'s CI runs and the
  cache, and holds them to `tools/ci-budget.ron`; `cargo xtask ci-budget
  prune` deletes `main`'s superseded cache entries.
- `.github/workflows/ci-budget.yml`: the prune every day, the budget
  every week.
- An ADR for the policy; the docs map's row for the ledger; the
  `run-gates` skill's line for the new gate.
- What changes for a game: a new game's `ci.yml` and `export.yml` get
  timeouts and `save-if` (the template). No public API changes; the
  game's own workflows are its own.

**Out (do not do):**
- Skipping a job when the pull request's crates are outside its
  dependency closure. The budget shows first whether it is needed; when
  the budget fails, write that ticket then.
- Merging two jobs' caches under one shared key. Measure first; the
  ledger's numbers say whether it is worth it.
- Moving the tutorial's web builds out of `web-render`: 3502 does it.
- Changing what any job checks, or its runner.
- Opening a GitHub issue from the budget run: a failing budget is a red
  scheduled run, and the session that sees it writes a ticket
  (`write-ticket`).

**Either order with 3502:** if 3502 is done, its `web-tutorial` job gets
its timeout and `save-if` in step 3 like every other job; if not, nothing
to do here (`check-workflows` makes 3502's new job carry both).

**Estimated size:** about 1,400 lines (code 650, tests 500, docs 250).

**Split:** past 1,100 lines, split off steps 6–8 (the `ci-budget`
command, its ledger and the scheduled workflow) into a new ticket.

**One Rust setup step first** (the redundancy sweep of 2026-11-28): `rustup show`,
`Swatinem/rust-cache` and `scripts/setup-linux.sh` begin about 38
jobs. Add `.github/actions/rust-setup/` (a composite action with the
cache's `save-if` written once, and inputs for the cache key and whether
to run the setup script), and call it from every engine job before
adding the timeouts. Step 1's check 2 reads the composite as well as
each workflow. `export.yml` keeps its own steps: it is byte-identical to
the template's and runs in games, which have no such local action.

## Implementation steps

1. **The gate.** Add `crates/xtask/src/workflows.rs`, run as `cargo
   xtask check-workflows` (`main.rs`, its usage text and module list) and
   listed in `gates.rs` beside `check-pins` (fast, no network). It reads
   each `*.yml` and `*.yaml` in every `.github/workflows` folder that
   `check-pins` walks (reuse its walk), parses it with `serde_norway`,
   and fails, naming the file, job and step:
   1. a job with `runs-on` and no `timeout-minutes`. The value is an
      integer from 1 to 120, or `${{ inputs.<name> }}` where the input
      has a number default from 1 to 120 (`playtest.yml`). A job that
      calls a workflow (`uses:`) is skipped: GitHub allows no timeout
      there, and the called workflow's jobs carry it.
   2. a `Swatinem/rust-cache` step whose `with:` lacks exactly
      `save-if: ${{ github.ref == format('refs/heads/{0}', github.event.repository.default_branch) }}`.
      The default branch, not `main`: a game's may be named otherwise.
      The text is one constant in the module.
   3. an `actions/cache` step (it saves on every ref), or an
      `actions/cache/save` step whose `if:` lacks the same condition as
      one of its `&&` terms. ADR-0130's guard stays beside it where the
      job has the private files.
   4. a `Swatinem/rust-cache` step whose `workspaces:` names a
      `matrix.` value that its `key:` or `shared-key:` doesn't name. Why:
      such legs build different workspaces, `ci-budget prune` groups
      entries by key, and legs under one key look like generations of
      one cache. Legs that build the same thing (`mutants.yml`'s shards,
      the `skills` shards keyed by `strategy.job-index`, a matrix over
      `os`, which the key already holds) are not refused.
   A file that doesn't parse is a failure too.
2. **Measure the timeouts.** For each job, take its longest duration
   over the last 20 successful push runs of its workflow on `main` (`gh
   api repos/Zafnok/shuyi-engine/actions/runs/<id>/jobs`), or over the
   runs there are. Its timeout is twice that, rounded up to 5 minutes,
   at least 10. Put the table (job, longest, timeout) in the completion
   notes. A template job, which only runs in a game, gets the time of the
   `templates` job's matching step in the engine's CI, by the same rule.
3. **Set them.** Add `timeout-minutes` to every job step 2 measured: the
   27 of `ci.yml` that run on a runner, `changes.yml`'s `detect`,
   `sound-parity.yml`'s two, and those of `export.yml`,
   `compile-times.yml`, `pages.yml`, `calibrate.yml`, `flaky-rerun.yml`,
   `codeql.yml`, `scorecard.yml`, `security.yml`, `sonar-triage.yml`,
   `dependabot-waiting.yml` and `notify-dependents.yml`, and the
   template's. `mutants.yml`'s stay. `compile-times.yml`'s job has
   none today (3415's Out leaves it here): if 3415 is done, its
   completion notes give the job's time with the new lib and rules
   measures, and that time goes in step 2's table; if not, measure it
   as the others. Run `check-workflows` until it
   passes rule 1.
4. **The cache.** Add the `save-if` of step 1 to every
   `Swatinem/rust-cache` step (35 in the root's workflows, and the
   template's). Give the `tutorial` job's step `key: ${{ matrix.step }}`
   beside its `workspaces:`. Keep every `cache-targets` guard of
   ADR-0130 as it is; the test
   `every_job_with_the_private_files_caches_no_build_and_uploads_none`
   (`crates/cli/src/contract/new/tests.rs`) must still pass. Update the
   template's snapshots if they hold the workflows.
5. **Check it on a pull request.** On this ticket's own PR, the jobs
   restore `main`'s caches and save none: the rust-cache step's log says
   it skips the save, and `gh api …/actions/caches?ref=refs/pull/<n>/merge`
   lists no entry the PR's run made. Put both in the completion notes.
6. **The ledger.** `tools/ci-budget.ron`, read by
   `crates/xtask/src/ci_budget.rs` with `serde` and `ron` (as
   `flaky.rs` reads `flaky.ron`), with a header comment saying what reads
   it and how to change a ceiling (a ticket that raises one says why).
   Its ceilings:
   - `cache_bytes: 8_000_000_000`: 80% of GitHub's 10 GB, so eviction
     never starts;
   - `main_wall_minutes: 60`, the slowest successful push run of
     `ci.yml` on `main` in the week, `run_started_at` to `updated_at`;
   - `main_runner_minutes: 300`, the median of those runs' summed job
     durations;
   - `main_jobs: 60`, the most jobs that weren't skipped in one of them.
7. **The command.** `cargo xtask ci-budget [--repo <owner/name>]`
   lists the week's successful push runs of `ci.yml` on the default
   branch and their jobs, and the cache usage, through `gh api` (as
   `tickets.rs` lists the game's tickets). It writes a Markdown table to
   standard output and to `$GITHUB_STEP_SUMMARY` when that is set: each
   measure, its ceiling, the five longest jobs, and the cache's five
   biggest key groups. It fails past any ceiling, naming it. With no run
   in the week it passes and says so. `cargo xtask ci-budget prune` lists
   the default branch's caches, groups them by key less its last
   `-<hex>` part, and deletes every entry of a group but the most
   recently created (`gh api -X DELETE
   repos/<repo>/actions/caches/<id>`); `--dry-run` only prints them. The
   API calls sit behind one small function, so the logic takes JSON
   fixtures.
8. **The workflow.** `.github/workflows/ci-budget.yml`, `permissions:
   {}` at the top:
   - job `prune`, daily (`cron: "17 4 * * *"`) and on
     `workflow_dispatch`: `actions: write`, runs `cargo xtask ci-budget
     prune`;
   - job `budget`, weekly (`cron: "41 4 * * 1"`, the `if:` reading
     `github.event.schedule`) and on `workflow_dispatch`: `actions:
     read`, runs `cargo xtask ci-budget`;
   - both with `GH_TOKEN: ${{ github.token }}`, a timeout, pinned actions
     and `persist-credentials: false`, so `check-pins`,
     `check-workflows` and zizmor pass. Run it once by
     `workflow_dispatch` on the PR's branch with `--dry-run`, and put
     what it printed in the completion notes.
9. **The ADR** (`write-adr`): CI's own budget. A timeout on every job,
   twice its longest; caches saved from the default branch only and the
   superseded ones pruned daily; a leg that builds its own workspace keyed by its leg;
   the ceilings and why (the 10 GB limit, concurrent jobs, the wall time
   agents wait for, minutes free on a public repository). It names
   ADR-0130 as the rule it sits beside, and amends nothing.
10. **The docs** (`document`): a row in `docs/README.md` for
    `tools/ci-budget.ron`, beside `tools/flaky.ron`'s; a line for
    `cargo xtask check-workflows` in the `run-gates` skill's list beside
    `check-pins`; run `pwsh -File scripts/sync-agent-skills.ps1 -Check`.

## Acceptance criteria

- [ ] `cargo xtask check-workflows` passes on the repository and is a
      gate of `cargo xtask gates`.
- [ ] Each of its four rules has a fixture it refuses: a job with no
      timeout, one with `timeout-minutes: 200`, a rust-cache step with no
      `save-if`, a `save-if` naming `refs/heads/main`, an `actions/cache`
      step, a rust-cache step with `workspaces: ${{ matrix.step }}` and no
      key naming it; and it passes a job that calls a workflow with no
      timeout, a `${{ inputs.<name> }}` timeout with a default of 30, a
      matrix over `os` only, and shards under one key.
- [ ] Every job on a runner in the root's and the template's workflows
      has a timeout, at least twice its longest measured duration; the
      table is in the completion notes.
- [ ] This ticket's PR's run saved no cache under its own ref (step 5,
      in the notes).
- [ ] ADR-0130's test passes unchanged.
- [ ] `cargo xtask ci-budget` fails a fixture of a week whose slowest run
      took 61 minutes and passes one whose slowest took 60; the same at
      the edge of each other ceiling (cache 8,000,000,001 bytes against
      8,000,000,000; median 301 runner-minutes against 300; 61 jobs
      against 60); and passes a week with no run, saying so.
- [ ] `ci-budget prune` on a fixture keeps the newest entry of each
      group, deletes the older generations, and never deletes an entry of
      another ref; two tutorial legs keyed by step are two groups.
- [ ] `ci-budget.yml` ran once by `workflow_dispatch`, its output in the
      notes; zizmor and `check-pins` pass it.
- [ ] The ADR, the docs map's row and the `run-gates` line are written.
- [ ] The book's chapter is updated (2802), or the completion notes have
      a `Book:` line saying why not (CI's own checks; no game reads them).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `workflows.rs`, each rule's refusal and pass over small YAML
  fixtures in the test module, and a test that runs it over the
  repository's own workflows.
- Unit: `ci_budget.rs`, the ledger parses; the measures from a jobs-API
  JSON fixture (skipped jobs not counted, the median of an even count);
  each ceiling's edge; the prune's grouping, from a caches-API fixture.
- Integration: the PR's own CI run (step 5) and one `workflow_dispatch`
  run of `ci-budget.yml` (step 8).

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
