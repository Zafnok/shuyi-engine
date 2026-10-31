---
id: "2021"
title: "A reusable playtest workflow: every stage played when what decides it changed, the summary in the job, a failure only on a floor; the grid template calls it"
type: infra
milestone: After MVP
model: implementation
effort: medium
status: done
blocked_by: ["2532", "2533"]
nick_input: none
completed: 2026-10-24
---

# 2021 — A reusable playtest workflow

## Context

A request from the game.

1. **The game ticket:** game#0506, the persona playtest bots
   (`github.com/Zafnok/visions-of-shuyi/blob/main/tickets/done/0506-persona-bots-casual-normal-hardcore.md`).
   The game's follow-up that moves onto this ticket is game#1180.
   Neither is a blocker of this one.
2. **The job in one sentence:** on every pull request or push that
   changes a game's stages, rules or bots, play every stage's playtest, put the
   report in the job's summary, and fail the check only when a bot falls
   under its floor (a missed band is reported, never a failure).
3. **No friction review triggered:** game#0506 measured 1.8×. Nick
   (2026-10-22) asked that the generic parts of the game's playtesting
   move into the engine, so another game gets them out of the box; this
   is one ticket of that sweep (game `docs/engine/friction.md`, row
   "0506 (sweep)"). The game's `.github/workflows/playtest.yml` is 94
   lines (YAML; `code-lines` doesn't count it): a detect job that skips
   by `if:` (never a workflow `paths` filter, so the check always
   reports: the reasoning of this repository's `changes.yml`), a
   release build, a loop over its battles, the report minus its per-try
   rows into `$GITHUB_STEP_SUMMARY`, and an exit code. Only the paths and
   the system packages are the game's. With this ticket the game's file
   is about 15 lines.
4. **The generic shape:** `.github/workflows/playtest.yml` in this
   repository with `on: workflow_call`, inputs for the paths that make a
   play, the working directory, extra system packages, the command
   (default `cargo shuyi playtest --every-stage --brief`), and the
   timeout; it plays when a changed file is a lever (by the paths, or by
   a script the game owns), writes each stage's text into the job
   summary, and fails exactly when the command exits non-zero, saying
   whether a stage was blocked or the playtest itself broke.
   **From 2532 (done):** the exit status can't tell those apart. A
   blocked stage is `ok: false` with no `code` and exits 1. An error
   the playtest hits (SH3001 no plan, SH3002 the game's error, SH3004 a
   file not written, SH3005 the tool program stopped) has a `code` and
   exits 1 too. Usage exits 2 and setup 3 (ADR-0028). `--report <file>`
   writes the JSON answer, error or not; there is no file when the
   arguments were wrong, the file couldn't be written or the tool
   program stopped.

   **Changed files on a push too** (added 2026-10-22, from the game's
   retro of its last 50 PRs): the game's workflow already compares a
   push to `main` with the commit before it (game#0514), because playing
   on every push cost about 25 minutes on every dialogue and art change,
   which Nick asked to stop. And a push's run must never be cancelled by
   the next push, or the first push's lever goes unplayed: on
   2026-10-22 two balance changes merged together were not played on the
   game's `main` for 1.5 hours (game#0515 fixes the game's file). The
   game's lever rule is a script with exclusions (tests and `*.md` never
   play), `.github/playtest-levers.sh`, which shell patterns alone can't
   say.
5. **Examples from games that aren't tactics:** any game with
   `shuyi playtest`: a puzzle game whose levels are its stages, a
   platformer with input bots per room.

## Nick input

None.

## Scope

**In:**
- `.github/workflows/playtest.yml` (reusable), pinned actions as the
  repository's other workflows pin them (full SHAs with a version
  comment), `permissions: contents: read`, `persist-credentials: false`
  (zizmor clean).
- This repository's CI calls it on `examples/hello-grid`, so it is
  tested on every engine PR that touches the bots, the CLI or the
  example.
- `templates/grid/.github/workflows/playtest.yml`: a caller of a few
  lines, pinned to the engine commit the template's game builds with,
  with a comment on moving the pin; `templates/blank` too if its game
  has a playtest plan.
- A section in the docs page about CI for games (or `docs/contract.md`'s
  `playtest` section): what the inputs mean, why skipping is by `if:`.
- **From 1134 (done)** (the skill pack): the `shuyi-balance-bots` skill
  (`skills/shuyi-balance-bots/SKILL.md`) gets a few lines on calling
  this workflow: its inputs, `levers`, and the concurrency that never
  cancels a push, with `check-skills` passing.
- **Either order with 2755 (parked):** if 2755 is done, also call the
  workflow on `examples/crawler`, with paths covering the example and
  the crates it is made of (`shuyi-crawl`, `shuyi-rpg`); if not,
  nothing to do here (2755 adds that caller when it lands).

**Out (do not do):**
- Playtest on a schedule, or on more runners than one.
- Caching the history between runs (it is per checkout).
- Changing `changes.yml`.
- The game's use of it (game#1180).

## Implementation steps

1. **Read** the game's `.github/workflows/playtest.yml` (game
   repository, `main`), this repository's `changes.yml` and `ci.yml`,
   and the templates' `ci.yml`.
2. **Inputs**: `paths` (newline-separated shell patterns, default
   everything but `*.md`, `docs/*`, `tickets/*`), `levers` (optional: a
   command in the caller's repository that reads changed paths on stdin,
   one a line, and prints `play=true` or `play=false`; when given,
   `paths` is not used: the game passes `bash .github/playtest-levers.sh`),
   `working-directory` (default `.`), `system-packages` (default the
   macroquad libraries the templates install), `command` (default
   above), `timeout-minutes` (default 120).
3. **Concurrency**: the caller's runs of one pull request cancel each
   other; a push's run is never cancelled (its group is its commit:
   `${{ github.event_name == 'push' && github.sha || github.ref }}`,
   `cancel-in-progress` only when the event isn't a push). Document it for
   callers, since `concurrency` is set in the calling workflow.
4. **Jobs**: `detect` (as the game's: `HEAD^1..HEAD` on a pull request,
   `github.event.before..HEAD` on a push, playing when `before` is
   missing or no longer in the history (a new branch, a force push);
   `workflow_dispatch` always plays; each path through `levers` or
   matched with `case`) and
   `playtest` (`if: needs.detect.outputs.play == 'true'`): checkout,
   `rustup show`, the rust cache, the packages, `cargo build --release
   --locked` of the game's tool program, the command with `tee`, the
   text into `$GITHUB_STEP_SUMMARY` in a fenced block per stage, and the
   command's exit status as the step's. The step adds `--report
   "$RUNNER_TEMP/playtest.json"` to the command, and on a non-zero
   status reads that file with `jq`: no `code` gives an `::error::`
   naming each stage whose `report.blocked` is true; a `code`, or no
   file, gives an `::error::` saying the playtest broke, with the code
   and the error.
5. **Caller in this repository's CI**: a job calling it with
   `working-directory: examples/hello-grid` and paths covering
   `crates/core/src/bots/`, `crates/cli/`, `examples/hello-grid/`.
6. **Template callers** and the docs section.

## Acceptance criteria

- [x] An engine PR that changes `crates/core/src/bots/` runs the
      hello-grid playtest through the reusable workflow and shows its
      report in the job summary.
- [x] A PR changing only `*.md` files reports the playtest job as
      skipped (not missing).
- [x] With `levers` set to a script that answers `play=false`, a PR
      changing `crates/core/` skips; answering `play=true`, it plays.
- [x] On a push, `detect` judges `before..HEAD` (a test branch push that
      changes only `*.md` skips), and the docs show callers the
      concurrency that never cancels a push.
- [x] With a deliberately raised floor in a test branch (not merged),
      the job fails with an `::error::` naming the stage; with the floor
      restored it passes.
- [x] In a test branch (not merged) whose playtest fails with a code
      (a plan that errors: SH3002), the job fails with an `::error::`
      saying the playtest broke, and names no blocked stage.
- [x] `zizmor` and `actionlint` (whichever this repository's CI runs)
      pass on the new files.
- [x] `shuyi new grid <dir>` makes a game whose `playtest.yml`
      calls the reusable workflow at a pinned commit.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- CI: the hello-grid caller is the test.
- Template: the template test (`shuyi new` output) includes the new
  file.

## Completion notes

- `.github/workflows/playtest.yml`, reusable (`on: workflow_call`):
  `detect` judges a pull request's merge commit (`HEAD^1..HEAD`) or a
  push's `before..HEAD` (a new branch or a force push plays; any other
  event plays), by `paths` (shell patterns one a line, the first match
  decides, `!pattern` doesn't play) or by `levers` (the caller's
  command, `play=true`/`play=false`). `playtest` is skipped by `if:`.
  It runs the command with `--report "$RUNNER_TEMP/playtest.json"`,
  writes a fenced block per stage into the job summary (split at the
  report's titles), and on a non-zero exit reads the report with `jq`:
  no `code` gives an `::error::` per blocked stage naming the entrants
  under their floor; a `code`, or no report, gives an `::error::` that
  the playtest itself broke, with the code and the error. zizmor clean.
- `ci.yml` calls it on `examples/hello-grid` (job `playtest`, in
  `ci-result`'s needs), with paths `crates/core/src/bots/*`,
  `crates/cli/*`, `examples/hello-grid/*` and the workflow itself.
- The grid template's stage part writes `.github/workflows/playtest.yml`,
  a caller of a few lines with the concurrency that never cancels a push.
  Its pin is a new placeholder, `{{engine_ref}}`: a 40-digit
  `--engine-rev`, else `main` (GitHub takes no short SHA in a `uses:`;
  a short `--engine-rev` gets a limit saying so). `templates/blank` gets
  none: its `BasicTools` has no playtest plan. `check-pins` skips the
  engine's own call at `@{{engine_ref}}` in a template, and a workflow
  folder with no SHA pin needs no Dependabot entry; zizmor's
  `unpinned-uses` on that line is ignored in the new `.github/zizmor.yml`,
  with why.
- Docs: `docs/contract.md` *The playtest in CI* (inputs, why skipping is
  by `if:`, the caller's concurrency); `docs/templates.md` (the
  placeholder); the docs map's pins row; the `shuyi-balance-bots` skill.

**Deviations:**

- **`allow-no-plan`**, an input the ticket doesn't list: the grid
  template's game has no playtest plan (`playtest` answers `SH3001`
  until the game writes one, as the `shuyi-balance-bots` worked example
  starts from), so its caller would have failed every pull request of a
  new game. With `allow-no-plan: true` (the template's caller) an
  `SH3001` is a notice and passes; the caller's comment says to drop it
  once the plan exists.
- **`build`** replaces step 4's fixed `cargo build --release --locked`:
  the default command builds the game's tool program itself, through the
  `cargo shuyi` alias (dev profile, dependencies optimized), so a release
  build would be a second build nobody runs. A game whose command runs a
  release binary (the game's `target/release/xtask`) passes `build`.
- **2755 (parked)** isn't done, so no crawler caller; its ticket now says
  how to add one (**From 2021 (done)**).
- This repository's `ci.yml` keeps its own concurrency (cancelling a
  push's run by the next), as every other job there; the playtest's
  never-cancel concurrency is for games' callers, documented.

**Tested** on test branches, draft pull requests against this ticket's
branch, closed unmerged (Zafnok/shuyi-engine#107 to #110 and a pushed
branch), each with a CI of only the hello-grid caller:

- floor raised to 50%: failed, `::error::` "the stage (files/stage.map):
  random under the floor";
- `playtest.ron` that doesn't parse: failed, `::error::` "the playtest
  itself broke (exit 1), SH3002: …", no stage named;
- only `*.md` changed: `playtest` skipped;
- `levers: echo play=false` with `crates/core/` changed: skipped; then
  `play=true`: played and passed;
- pushes: a new branch played; then a push of only a `README.md`
  change, judged by `before..HEAD`, skipped.

The first run of the floor test found the step stopping at the failed
pipeline (Actions runs `bash -e`), before its `::error::` lines; the step
now runs `set +e`. The job summaries were written without error, but
GitHub shows them only to a signed-in viewer, so the text in them was
read in the logs, not on the summary page.

The game moves onto this in game#1180 (no PR yet).

Breaking: shuyi-cli: template::Fill: a new field `engine_ref`, and `Fill` is `#[non_exhaustive]`: build it with `Fill::default()` and assign the fields. The game builds none.
