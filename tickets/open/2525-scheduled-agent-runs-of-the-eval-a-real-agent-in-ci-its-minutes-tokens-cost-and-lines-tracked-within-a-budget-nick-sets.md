---
id: "2525"
title: "Scheduled agent runs of the eval: a real coding agent makes each task in CI on a schedule, its minutes, tokens, cost, lines and checks tracked, within a budget Nick sets"
type: infra
milestone: After MVP
model: opus-5.5
effort: high
status: todo
blocked_by: ["2524", "1133", "2513", "2016"]
nick_input: decision
completed:
---

# 2525 — Scheduled agent runs of the eval

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8, and the 2D engines GameMaker and Defold. They all ship
tooling for coding agents, and none publishes how well an agent makes a
game with it. Engines track performance over time, like Bevy's compile
times, sizes and benchmarks (https://metrics.bevy.org); nobody tracks
agent results.

2524 gives the tasks, the grader and the reference solutions. It shows
that the checks can pass. It doesn't show that an agent passes them, in
how long, at what cost or in how many lines, which is what the end goal
says ([`docs/decisions.md`](../../docs/decisions.md), 2026-10-19: under
an hour, in few lines; "Every new API is judged against it"). This
ticket runs a real coding agent on each task, in CI, on a schedule, and
keeps the numbers, so a slide shows up within weeks instead of at
launch.

Each run costs API money: up to an hour of an agent's work per task per
run. Money is Nick's call (the `ask-nick` skill), and the 2026-10-20
entry in `docs/decisions.md` lists "how much the scheduled agent check
may spend" among the decisions left for later. So this ticket starts
with his answer.

Agent runs are noisy: the same prompt passes one week and fails the
next. So each task runs k times and the pass rate is kept; a single run
is labelled indicative; each result names the agent's version and the
model, so a jump in the numbers can be traced to the model rather than
the engine. A run's minutes are split into cargo's time (builds, tests
and the engine's tools) and the agent's own time, so the compile work
of 2009 shows in the number.

What exists (checked 2026-10-21):

- `.github/workflows/sonar-triage.yml` is a scheduled workflow
  (`schedule` and `workflow_dispatch`, a `concurrency` group,
  `permissions: {}` at the top), and every workflow pins its actions by
  commit SHA.
- `shuyi new` takes `--engine-rev <commit>`
  (`crates/cli/src/contract/new.rs`), so a project can depend on the
  engine as a user's game does, at the commit under test. From the
  engine's checkout it runs as `cargo shuyi new` (the alias in
  `.cargo/config.toml`).
- 2513 runs CI's `templates` job in a bare `container: ubuntu:24.04`.
  That image has no `git`, so `actions/checkout` downloads the commit
  without `.git`. 2524's grader needs `git` (`git rev-parse HEAD`,
  `git diff --no-index`).
- `crates/xtask/tests/cli.rs` runs the `xtask` binary
  (`CARGO_BIN_EXE_xtask`) and reads its exit code. The workspace
  forbids `unsafe` (edition 2024), so a test can't set an environment
  variable in its own process; it sets one on a child process. xtask
  already depends on `serde_norway`, which reads YAML.
- Claude Code's non-interactive mode (`claude -p`, read 2026-10-21 at
  https://code.claude.com/docs/en/headless and
  https://code.claude.com/docs/en/permission-modes):
  - `--output-format stream-json --verbose` prints one JSON event a
    line: a `system/init` event with the model and each MCP server's
    `status`, and a last `result` event with `total_cost_usd`, the
    usage and the session's metadata;
  - it takes `--model`, `--max-budget-usd`, `--mcp-config`,
    `--permission-mode`, `--allowedTools`, and `--permission-prompts
    none` (v2.1.259 or later), which denies whatever would ask;
  - on SIGINT it ends its turn; on SIGTERM it exits with 143 and keeps
    no result for the turn;
  - without `--bare` it loads the project's `CLAUDE.md`, skills, hooks
    and `.mcp.json`, and whatever is in `~/.claude`;
  - on Linux it refuses `bypassPermissions` (and
    `--dangerously-skip-permissions`) as root, and a container job runs
    as root.

  Read the pages again on the day: flags change.

## Nick input

**Decision** first, in one `ask-nick` round, in plain words with the
numbers:

1. **How much may it spend a month?** The session estimates one run's
   cost on the day (the model's published price times a typical hour's
   tokens) and offers three choices, for example: the two turn-based
   tasks once a month; once a week; once a week three times each, for a
   pass rate.
2. **Which agents?** Claude Code only, or others too (OpenAI's Codex
   CLI, Google's Gemini CLI, Cursor's CLI). Each other agent needs its
   own account, key and money, and gets a ticket of its own.
3. **May the results go on the website?** The numbers only (tasks, pass
   rate, minutes, cost, lines), or nothing public. The agents'
   transcripts are never published unless he says so.

His answers go in `docs/decisions.md` and in `evals/budget.ron`. If he
says no money, record it in `docs/decisions.md`, move this ticket to
`tickets/done/` with that in its completion notes, and the eval stays
the graded references of 2524.

**Setup**, after the decision. These are Nick's alone:

1. In the Claude Console (`https://platform.claude.com`), make a
   workspace for the eval and create an API key in it. If the Console
   offers a spend limit per workspace on the day, set it to the monthly
   cap he chose; if not, the plan's own cap below is the only one, and
   the session tells him so.
2. In the repository on GitHub, Settings, Environments, New environment:
   name it `agent-eval`. Under "Deployment branches and tags" pick
   "Selected branches and tags" and add `main`. Add an environment
   secret `ANTHROPIC_API_KEY` with the key.

## Scope

**In:**
- **`evals/budget.ron`**, Nick's answers as data, read by
  `crates/xtask/src/eval/budget.rs` with `serde` and `ron`, as 2524's
  `task.rs` reads tasks (`deny_unknown_fields`): `monthly_usd`,
  `per_run_usd` (the agent's own cap for one run), `runs_per_task` (k),
  `schedule` (the workflow's cron line, kept equal by a test), `agents`
  (each: an id, the CLI's package and exact version, the model id), and
  `publish`.
- **`cargo xtask eval-plan --results <results.json> [--task <id>]
  [--runs <k>]`** (`crates/xtask/src/eval/plan.rs`), what one workflow
  run may spend:
  - lists the ready tasks in `evals/tasks/` (those with no
    `waits_for`), or the one named, each with k runs;
  - reserves `per_run_usd` for each run, in order, while this month's
    recorded cost (the UTC calendar month; a run with no `cost_usd`, or
    a lost one, counts `per_run_usd`) plus what is reserved stays within
    `monthly_usd`. The rest are left out;
  - refuses a k whose job would pass GitHub's 360-minute job limit (k
    times the box, plus 30);
  - prints the matrix for `$GITHUB_OUTPUT` (`{"include": [...]}`, each
    entry with `task`, `runs`, `timeout`, `agent`, `package`, `version`
    and `model`) and, for the step summary, the runs left out and why.
- **`cargo xtask eval-run --task <id> --agent <id> --runs <k> --out
  <dir>`** (`crates/xtask/src/eval/run.rs`):
  - refuses with exit code 2 unless `GITHUB_ACTIONS` is `true` ("agent
    runs spend money and run in CI only"), with no flag around it;
    refuses a task that waits (2524's `waits_for`);
  - its process code is `#[cfg(unix)]`. Elsewhere `eval-run` says it
    runs on Linux only, so the workspace still builds and lints on
    Nick's Windows machine;
  - the three outside programs (the project maker, the agent and the
    grader) are commands in one `Tools` value, which the CLI fills with
    the real ones; the time box and the grace before a kill are fields
    of the run's plan. So a test passes fakes and one-second limits;
  - for each run:
    1. makes a folder under `$RUNNER_TEMP`, outside the checkout, with
       an empty `HOME`. `CARGO_HOME` and `RUSTUP_HOME` stay where the
       setup script put them, so the toolchain is still found, and
       `CARGO_TARGET_DIR` is unset, so the game builds as a user's first
       build does. From the checkout, `cargo shuyi new <template>
       <run>/game --engine-rev <GITHUB_SHA>`, so the game depends on the
       engine as a user's does; then a copy of `game` as the base for
       the line count;
    2. writes the MCP server's entry (1133) for Claude Code to a file,
       the one `shuyi mcp install --dry-run` prints, and passes it with
       `--mcp-config`, so nothing asks; the skills are the ones
       `shuyi new` copied into the project (1134);
    3. writes a `cargo` wrapper into the run's `bin/` (from a string in
       `run.rs`, mode 755) and puts that folder first on `PATH`: a `sh`
       script that runs the real `cargo` (its path found before `PATH`
       changes) and appends a line `start end args` (`date +%s%N`) to
       `cargo.log`;
    4. runs the agent in `game` with the task's `prompt` and nothing
       else, in its own process group (`CommandExt::process_group(0)`).
       For Claude Code: `claude -p <prompt> --model <model>
       --max-budget-usd <per_run_usd> --output-format stream-json
       --verbose --permission-mode acceptEdits --allowedTools
       "Bash,Edit,Write,WebFetch,WebSearch,mcp__<server>"
       --permission-prompts none --mcp-config <file>`, its stream into
       `agent.jsonl`, outside the folder that is uploaded. Not
       `bypassPermissions`: it is refused as root. Not `--bare`: a
       user's run reads the project's `CLAUDE.md` and skills, and the
       eval measures that. Only the agent's process gets the key's
       variable (`ANTHROPIC_API_KEY`); `shuyi new` and the grader run
       without it;
    5. at `time_box_minutes`, sends SIGINT to the agent, and two minutes
       later kills its process group if it hasn't exited, so no build it
       started outlives it; the run is marked `timed_out`. Signals go
       through `sh -c` and the shell's own `kill`: the bare image may
       have no `/bin/kill`, and `libc::kill` needs `unsafe`;
    6. grades the project with the plain `PATH`: `cargo xtask
       eval-grade evals/tasks/<id>.ron <run>/game --base <base> --out
       <run>/eval-result.json` (2524);
    7. writes `eval-run.json`: the grade, `task`, `run`, `date`,
       `engine` (the commit), `agent`, `agent_version` (`claude
       --version`), `model`, `mcp_connected` (the server's `status` in
       `system/init`), `timed_out`, `wall_seconds` (the agent's start to
       its exit), `cargo_seconds` (the union of the log's intervals, so
       a cargo call inside another, as when `cargo shuyi web` runs
       `cargo build`, counts once), `agent_seconds` (wall minus cargo,
       at least 0), `turns`, `tokens` (input, output, cache read and
       cache write, as the `result` event gives them) and `cost_usd`
       (none when there is no `result` event);
    8. deletes `agent.jsonl` (the transcript), then searches every file
       left (the project, `cargo.log`, `eval-run.json`) for the key's
       value. If a file holds it, the run's folder is deleted except
       `eval-run.json`, which then says only that the run was withheld
       and in which file the key was found: its path, never the value;
  - each agent is one small adapter type: its command line and how its
    stream reads. This ticket writes Claude Code's.
- **`cargo xtask eval-record <results.json> --planned <matrix>
  <eval-run.json>...`** (`crates/xtask/src/eval/record.rs`): appends the
  runs to the array, and a `lost` entry for each planned run that has no
  `eval-run.json` (a job that hit its timeout), which the plan counts at
  `per_run_usd`. It keeps a summary per task and ISO week: runs, passes,
  the pass rate (`indicative` when there was one run), and the medians
  of minutes, cargo minutes, tokens, cost and lines added.
- **`.github/workflows/agent-eval.yml`**, named "Agent eval":
  - `on: schedule` (the cron line in `budget.ron`) and
    `workflow_dispatch` (inputs: task, runs). Never `push`,
    `pull_request`, `pull_request_target`, or any event a fork can
    start;
  - `permissions: {}` at the top, and `concurrency: { group: agent-eval,
    cancel-in-progress: false }`, so two runs never plan against the
    same `results.json`;
  - the inputs reach scripts through `env:`, never `${{ }}` inside
    `run:` (zizmor's template-injection audit, which 2011 runs on every
    PR);
  - a `plan` job (`contents: read`, no secrets, and `if: github.ref ==
    'refs/heads/main'`, as the environment allows only `main`) reads
    `results.json` from the `eval-results` branch (`git fetch origin
    eval-results`; an empty array when the branch doesn't exist yet)
    and runs
    `eval-plan`;
  - a `run` job: `needs: plan`, the plan's matrix with `max-parallel: 1`
    (one agent at a time, so runs don't share the workspace's rate
    limit), `environment: agent-eval`, `permissions: contents: read`,
    `timeout-minutes: ${{ matrix.timeout }}`. It runs in `container:
    ubuntu:24.04` as 2513's `templates` job does, with the
    `$GITHUB_WORKSPACE` and `$RUNNER_TEMP` variables, not the contexts,
    as 2513 says. Its steps:
    1. `apt-get update && apt-get install -y --no-install-recommends
       git ca-certificates`, so the checkout is a repository and the
       grader's `git` works;
    2. checkout with `persist-credentials: false`;
    3. `bash scripts/setup-linux.sh` (2513);
    4. Node from `actions/setup-node`, pinned by commit SHA, then
       `npm install -g "$PACKAGE@$VERSION"` from the matrix (Claude
       Code: `@anthropic-ai/claude-code` at the exact version);
    5. `cargo xtask eval-run`, the only step with `ANTHROPIC_API_KEY:
       ${{ secrets.ANTHROPIC_API_KEY }}` in its `env:`;
    6. with `if: always()`, uploads two artifacts kept 30 days:
       `eval-run-<task>` (the `eval-run.json` files only) and
       `eval-project-<task>` (each run's project without `target/`, and
       its `cargo.log`);
  - a `record` job: `needs: [plan, run]`, `if: always() &&
    needs.plan.result == 'success'`, `permissions: contents: write`, no
    environment and no secrets, and no code from a run. It checks out
    `main` (for `xtask`), downloads only the `eval-run-*` artifacts,
    checks out the data branch `eval-results` (an orphan branch made on
    the first run), runs `eval-record` with the plan's matrix, and
    pushes with the job's token passed through `env:` (both checkouts
    keep `persist-credentials: false`). The branch holds data only and
    is never merged.
- **`AGENTS.md`**, *Environment*: "Never run `cargo xtask eval-run` on
  Nick's machine: it spends money, and CI runs it (2525)."
- **`evals/README.md`:** how the runs work, how to start one by hand
  (Actions, "Agent eval", Run workflow), how to read `results.json`, and
  that a single run is indicative.
- **`docs/continuity.md`** (2016's list of accounts and secrets): a row
  each for the Claude Console workspace, the `agent-eval` environment
  and its `ANTHROPIC_API_KEY` secret: where each is kept (never a
  value), who holds it, how to rotate it, and this ticket.

If the PR grows well past 600 changed lines, not counting fixtures, move
`eval-record`'s weekly summary into the page's ticket (step 1) and say
so in the PR.

**Out (do not do):**
- the tasks, the grader and the references (2524); new tasks (each a
  task file of 2524's kind, in a ticket of its own);
- the results page on the website: if Nick said publish, step 1 writes
  its ticket, blocked by this one and by the website (2801). A chart or
  a card on the home page goes there too;
- agents other than Claude Code: a ticket each, if Nick picks them;
- running agents on Nick's machine, or from pushes, pull requests or
  forks;
- grading by a model, or judging fun: the verdicts are 2524's checks;
- a gate on new APIs from these numbers: "every new API is judged
  against it" stays a convention; once a baseline has held for some
  months, a ticket may propose a gate;
- real-time tasks: they wait in 2524's task files (snake for 2739, the
  platformer for 1127);
- hiding the references: the agent may read the public repository like
  anyone, and the eval doesn't pretend otherwise;
- publishing transcripts;
- the MCP server (1133), the skills (1134), the setup script (2513),
  compile times (2009), and the lines a template hands its author
  (2512);
- acting on what the runs show, such as 2808's follow-up on docs agents
  can't find: a ticket each.

## Implementation steps

1. Ask Nick (`ask-nick`) with the estimate, and record his answers in
   `docs/decisions.md`. With the `write-ticket` skill, write a ticket
   for the results page if he said publish (After MVP, blocked by 2525
   and 2801: `pages.yml` copies `results.json` from `eval-results`
   before `zola build`, and a page reads it), and one for each other
   agent he picked. Give him the setup steps, and wait until he says
   they are done.
2. Write `budget.rs`, `evals/budget.ron` and `plan.rs`, with tests on a
   made-up `results.json`.
3. Write the Claude Code adapter, with tests on a fixture stream
   written from the documented `system/init` and `result` events.
4. Write `run.rs`. Its tests write `sh` fakes into a temporary folder:
   a project maker that copies a small folder, an agent that edits a
   file, calls the `cargo` wrapper (whose real `cargo` is a fake that
   sleeps) and prints the fixture stream, an agent that ignores SIGINT
   and starts a child that sleeps, and a grader that writes a made-up
   `eval-result.json`. No test builds a game or spends money.
5. Write `record.rs`.
6. Write the workflow, and a test in `crates/xtask/tests/cli.rs` that
   reads it as YAML. Run it once by hand, `runs: 1` on tic-tac-toe, and
   put the result in the PR description.
7. Update `AGENTS.md`, `evals/README.md` and `docs/continuity.md`, and
   run the gates.

## Acceptance criteria

- [ ] Nick's answers are in `docs/decisions.md` and `evals/budget.ron`,
      and the tickets step 1 calls for exist.
- [ ] `cargo xtask eval-run` without `GITHUB_ACTIONS` exits with code 2
      and starts nothing (a test runs the binary).
- [ ] With the fakes, a run writes an `eval-run.json` with every field
      above; a run past its box is marked `timed_out`, and nothing of
      its process group is left (tests, Linux only).
- [ ] `cargo_seconds` counts overlapping cargo calls once (a test on a
      made-up log).
- [ ] `eval-plan` leaves out the runs that would pass `monthly_usd`,
      counts a run with no cost and a lost run at `per_run_usd`, and
      refuses a k past 360 minutes (tests on a made-up `results.json`).
- [ ] A run whose files hold the key's value keeps only an
      `eval-run.json` that names the file and not the value (a test
      with a fake key).
- [ ] A test reads the workflow as YAML: its only triggers are
      `schedule` and `workflow_dispatch`; `permissions: {}` is at the
      top; only the `run` job names the `agent-eval` environment, and
      only its `eval-run` step names `secrets.ANTHROPIC_API_KEY`; only
      `record` has `contents: write`; the cron equals `budget.ron`'s
      `schedule`.
- [ ] One run by hand on tic-tac-toe finished with `mcp_connected` true,
      a `cost_usd` and its tokens, and its result is on the
      `eval-results` branch and in the PR description.
- [ ] `docs/continuity.md` has the rows, and `AGENTS.md` the line.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `budget.ron` parsing, with unknown fields refused; the plan (the
  month's cost, the runs left out, the 360-minute cap); reading Claude
  Code's stream from the fixture; the union of the cargo intervals;
  `eval-record`'s entries, lost runs, pass rates and medians; the key
  search.
- Unit, Linux only: `run.rs` with the fakes in a temporary folder, one
  run that finishes and one past a one-second box.
- Integration (`crates/xtask/tests/cli.rs`): `eval-run` with
  `GITHUB_ACTIONS` removed from the child's environment exits 2; the
  workflow read as YAML.
- The one run by hand.

## Completion notes

*(Filled in by the session that completes the ticket.)*
