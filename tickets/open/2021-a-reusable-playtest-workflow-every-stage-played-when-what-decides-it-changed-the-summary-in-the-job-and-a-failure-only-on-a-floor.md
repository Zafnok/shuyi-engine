---
id: "2021"
title: "A reusable playtest workflow: every stage played when what decides it changed, the summary in the job, a failure only on a floor; the grid template calls it"
type: infra
milestone: After MVP
model: sonnet-5
effort: medium
status: todo
blocked_by: ["2532", "2533"]
nick_input: none
completed:
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
   summary, and fails exactly when the command exits non-zero (1:
   blocked; anything else: the playtest itself broke, reported as such).

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
   command's exit status as the step's (with `::error::` naming a
   blocked stage when the status is 1).
5. **Caller in this repository's CI**: a job calling it with
   `working-directory: examples/hello-grid` and paths covering
   `crates/core/src/bots/`, `crates/cli/`, `examples/hello-grid/`.
6. **Template callers** and the docs section.

## Acceptance criteria

- [ ] An engine PR that changes `crates/core/src/bots/` runs the
      hello-grid playtest through the reusable workflow and shows its
      report in the job summary.
- [ ] A PR changing only `*.md` files reports the playtest job as
      skipped (not missing).
- [ ] With `levers` set to a script that answers `play=false`, a PR
      changing `crates/core/` skips; answering `play=true`, it plays.
- [ ] On a push, `detect` judges `before..HEAD` (a test branch push that
      changes only `*.md` skips), and the docs show callers the
      concurrency that never cancels a push.
- [ ] With a deliberately raised floor in a test branch (not merged),
      the job fails with an `::error::` naming the stage; with the floor
      restored it passes.
- [ ] `zizmor` and `actionlint` (whichever this repository's CI runs)
      pass on the new files.
- [ ] `shuyi new grid <dir>` makes a game whose `playtest.yml`
      calls the reusable workflow at a pinned commit.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- CI: the hello-grid caller is the test.
- Template: the template test (`shuyi new` output) includes the new
  file.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
