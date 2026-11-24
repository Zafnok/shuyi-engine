---
id: "3421"
title: "The eval's references graded on fresh games: cargo xtask eval-reference and an eval job in CI"
type: infra
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: done
blocked_by: ["2524"]
nick_input: none
completed: 2026-11-24
---

# 3421 — The eval's references graded on fresh games, and in CI

## Context

2524 (done) gave the agent eval its tasks (`evals/tasks/*.ron`), read by
`crates/xtask/src/eval/task.rs`, and its grader, `cargo xtask eval-grade
<task.ron> <project> [--base <dir>] [--out <file>]`
(`crates/xtask/src/eval/grade.rs`), which runs each check through the
worked-example runner (`crates/xtask/src/script.rs`) and counts a game's
lines over a fresh `shuyi new` with `git diff --no-index`
(`evals/README.md`). It split this off to stay under the size limit:
nothing yet makes the two games a reference is graded on, and CI grades
none, so a change that breaks a reference goes unseen. That was 2524's
reason to exist ("reference solutions that CI grades on every pull
request").

**A draft is written.** 2524's session wrote this ticket's code before
the split, and it worked: it is on the branch
`draft-2524-eval-references` (commit `97e04ff4`, pushed, no PR), in
`crates/xtask/src/eval/reference.rs`, its tests in
`crates/xtask/src/eval/tests.rs` and its wiring in
`crates/xtask/src/main.rs`. Graded locally on Windows on 2026-11-24,
box-pushing passed every check but `Export linux` (`SH1003`: it builds
only on Linux). Take it from there (`git checkout
draft-2524-eval-references -- crates/xtask/src/eval/reference.rs`),
rebased on what 2524 merged: 2524 moved `Graded` to the runner's
`Verdict` and dropped `Task::listed` and `TASKS`, which the draft uses.
The draft also holds 3422's reference (`evals/references/`): leave it
to 3422.

## Nick input

None.

## Scope

**In:**
- **`cargo xtask eval-reference (<id> | --all) [--out <dir>]
  [--template-only]`** (`crates/xtask/src/eval/reference.rs`), as
  2524's scope wrote it:
  - makes two games with `cargo run -q -p shuyi-cli --locked -- new
    <dir> --template <template> --name <name> --engine-path <this
    checkout> --json` (`skills::make_game`, as the `skills` job makes
    its games): `target/eval/grade/<id>` to grade and
    `target/eval/base/<id>` as the base; they build in
    `target/eval-build` with no incremental cache unless
    `CARGO_TARGET_DIR` is set (`skills::runners`'s pattern);
  - lays an `Overlay` over the first (each file copied at its path), or
    takes a `Project` where it is, then grades it with `--base` through
    `grade::to_file` and writes `<out>/<id>.json` (`--out` defaults to
    `eval-results`);
  - refuses, with exit code 2, an overlay whose `Cargo.toml` says where
    the engine's crates are (a `path`, a `git` or a version, or a table
    of its own): `--engine-path` does. One naming them as the
    workspace's (`shuyi-core.workspace = true`) may be laid over (the
    draft's `places_engine_crate`);
  - with `--template-only`, grades the plain template, to show that a
    task's checks can fail;
  - with `--all`, grades every task with a reference and no
    `waits_for`, lists the ones that wait, and warns when an id in a
    `waits_for` is in `tickets/done/` ("1127 is done: give `platformer`
    a reference and empty its `waits_for`");
  - exits 0 when every graded task passed, 1 when one failed, 2 when one
    couldn't be graded.
- **CI** (`.github/workflows/ci.yml`): a job `eval`, after `changes` and
  only on code changes:
  - it runs in the sniper SDK container pinned in the `export` job's
    `linux` leg (`shuyi_cli::export::SNIPER`), as `export.yml` does,
    so the `Export("linux")` checks build (their glibc floor is the
    container's, 2.31); `bash scripts/setup-linux.sh` sets it up, and a
    tool the image lacks (`git` for the line count and `rev-parse`,
    `jq`) is installed in a step of its own. If the image can't run the
    other checks (the `Web` check's `cargo shuyi export web`, say), keep
    the job on `ubuntu-latest`, grade `Export("linux")` in a second job
    in the container, and say why in the completion notes. Decide by
    running it;
  - `CARGO_TARGET_DIR` shared as in the `templates` job;
  - `cargo xtask eval-reference --all --out eval-results` must pass;
  - `cargo xtask eval-reference box-pushing --template-only --out
    eval-template` must fail, and the step checks with `jq` in
    `eval-template/box-pushing.json` that `Check` passed and that every
    `Scene` check failed;
  - it uploads `eval-results/` as an artifact and writes a table (task,
    checks passed, lines added, seconds) into the step summary;
  - `ci-result` needs it.
- `evals/README.md`'s *Grading* says how to grade a reference with
  `eval-reference`, and that CI does on every PR.
- **A `Project` reference's lines.** Graded by hand on 2026-11-24,
  box-pushing counted +1,753 −38,467 lines, 101 files, over a fresh
  `blank`: the tutorial's step was made before the template gained the
  skill pack's copy (`.claude/skills/`), its workflows and its
  `AGENTS.md`, so they count as removed. Decide whether a `Project`
  reference counts only the files it has (the removals of files it
  never had left out), and say in `evals/README.md` which it does.

**Out (do not do):**
- the tic-tac-toe reference (3422, which also adds its `--template-only`
  step to this job);
- the tasks and references other tickets add, which `--all` grades once
  they are there: snake (2739), the platformer (1127), the maze chase
  (2572, which waits for this ticket), the roguelike (2735), the
  match-3 (2736), the card battle (2737) and the brick-breaker (2738);
- running an agent (2525).

**Estimated size:** about 700 lines (code 330, tests 200, CI and docs
170), much of it in the draft.

## Implementation steps

1. Read 2524's completion notes, `crates/xtask/src/eval/` and the
   draft on `draft-2524-eval-references`.
2. Bring `reference.rs` over, fit it to 2524's `grade.rs` (`Verdict`,
   no `Task::listed`), wire `eval-reference` into `main.rs`, its usage
   text and the package's `description`, and bring its tests: the
   waiting warning against a fake `tickets/done/`, the overlay refusal,
   the options.
3. Run `cargo xtask eval-reference --all` and `--template-only` locally.
4. Add the `eval` job and run it on the PR.

## Acceptance criteria

- [x] `cargo xtask eval-reference --all` passes on box-pushing in CI, and
      lists tic-tac-toe, snake and the platformer as waiting.
- [x] `--template-only` on box-pushing fails every `Scene` check and
      passes `Check`, checked by the job.
- [x] An overlay with a `Cargo.toml` that places an engine crate is
      refused with exit code 2.
- [x] The PR description has box-pushing's lines added and seconds.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`xtask`): the waiting warning, the overlay refusal, the options.
- Integration: the `eval` CI job, both ways.

## Completion notes

- `cargo xtask eval-reference (<id> | --all) [--out <dir>]
  [--template-only]` (`crates/xtask/src/eval/reference.rs`), from the
  draft on `draft-2524-eval-references`, fitted to what 2524 merged:
  the exit codes are `grade::PASSED`, `FAILED` and `UNGRADED`, and the
  draft's `TASKS` and `Task::listed` live in `reference.rs` (`TASKS`,
  `waiting`), beside a new `pick`. `skills::runners` takes its build
  folder, so `eval-reference` builds in `target/eval-build` with the
  same pattern. A `Project` reference makes only the base; the draft
  made an unused graded game too. The draft's refusal message lost a
  line continuation (a run of spaces inside it): fixed, and the overlay
  check now matches `Cargo.toml` exactly, not any name starting so.
- **A `Project` reference counts only the files it has.** `grade::Base`
  names the base and `grade::Over` which of its files count:
  `Everything` for an agent's game and `eval-grade --base`,
  `GamesFiles` for a `Project` reference, whose result's `limits` says
  so. Box-pushing went from +1,753 −38,467 lines, 101 files, to +1,753
  −404, 35 files. `evals/README.md` says why.
- Graded locally on Windows: box-pushing passed 11 of 12 checks, all
  but `Export linux` (`SH1003`), +1,753 −404 lines, 35 files, 163.4 s
  of checks (4 min with the two games made). Its plain template passed
  8 of 12 (`Check` among them) and failed `Responds`, `Replay`, `Scene 1`
  and `Export linux`, exiting 1. `--all` listed platformer, snake and
  tic-tac-toe as waiting.
- **CI**: the `eval` job runs in the sniper SDK image
  (`shuyi_cli::export::SNIPER`), git, jq and ca-certificates installed
  before the checkout so it clones. It grades `--all`, then
  `--template-only` must exit 1 with `Check` passed and every `Scene`
  failed (jq), writes the table to the step summary and uploads
  `eval-results/` and `eval-template/` as `eval-results`. `ci-result`
  needs it. The grader's outputs are in `.gitignore`.
- `cargo xtask gates` lists `eval` as CI's alone: `Export("linux")`
  builds only in the Steam runtime's image, so a local run would always
  fail it; `cargo xtask eval-reference <id>` grades one locally.
- Settled what named 3421: the "either order with 3421" clauses of
  1127, 2735, 2736, 2737, 2738 and 2739 now say what holds.
- The draft's tic-tac-toe reference (`evals/references/`) is left to
  3422, which names it, and adds its own `--template-only` step to this
  job.
- No break: xtask isn't published. No `Behaviour:` line: no crate a
  game uses changed.
- Docs: `evals/README.md`'s *Grading*, and `cargo xtask`'s usage text.
