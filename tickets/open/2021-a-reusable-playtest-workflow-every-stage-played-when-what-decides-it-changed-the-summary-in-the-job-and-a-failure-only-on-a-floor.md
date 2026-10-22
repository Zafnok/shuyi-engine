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
2. **The job in one sentence:** on every pull request that changes a
   game's stages, rules or bots, play every stage's playtest, put the
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
   timeout; it plays when the event isn't a pull request or a changed
   file matches, writes each stage's text into the job summary, and
   fails exactly when the command exits non-zero (1: blocked; anything
   else: the playtest itself broke, reported as such).
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
   everything but `*.md`, `docs/*`, `tickets/*`), `working-directory`
   (default `.`), `system-packages` (default the macroquad libraries the
   templates install), `command` (default above), `timeout-minutes`
   (default 120).
3. **Jobs**: `detect` (as the game's: `HEAD^1..HEAD` on a pull request,
   each path matched with `case`, `play=true` on any other event) and
   `playtest` (`if: needs.detect.outputs.play == 'true'`): checkout,
   `rustup show`, the rust cache, the packages, `cargo build --release
   --locked` of the game's tool program, the command with `tee`, the
   text into `$GITHUB_STEP_SUMMARY` in a fenced block per stage, and the
   command's exit status as the step's (with `::error::` naming a
   blocked stage when the status is 1).
4. **Caller in this repository's CI**: a job calling it with
   `working-directory: examples/hello-grid` and paths covering
   `crates/core/src/bots/`, `crates/cli/`, `examples/hello-grid/`.
5. **Template callers** and the docs section.

## Acceptance criteria

- [ ] An engine PR that changes `crates/core/src/bots/` runs the
      hello-grid playtest through the reusable workflow and shows its
      report in the job summary.
- [ ] A PR changing only `*.md` files reports the playtest job as
      skipped (not missing).
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
