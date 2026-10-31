---
id: "2022"
title: "The Markdown a test reads counts as code in CI, and run-gates runs the templates job"
type: infra
milestone: MVP
model: sonnet-5 / gpt-6.1-terra
effort: low
status: done
blocked_by: []
nick_input: none
completed: 2026-10-22
---

# 2022 — The Markdown a test reads counts as code, and run-gates runs the templates job

## Context

A retro of PRs #38–#68 (2026-10-22) found two checks that don't run when
they should.

**A docs-only PR skips the test of the docs.** 2017 added
`crates/cli/tests/it/docs.rs`. It checks `docs/contract.md` (its
operations table and its example answer) and the `shuyi-cli` row of the
root `README.md` against `Op::ALL` and `contract::VERSION`. It runs in
`ci.yml`'s `test` job, which runs only when `changes.yml` says the PR
touches code. `changes.yml` counts every `*.md` and everything under
`docs/` as docs-only. So a PR that edits only `docs/contract.md` or the
`README.md` skips the test, `ci-result` passes on the skip, and the break
first shows on `main`. The same holds for the problem-code pages
`crates/cli/errors/*.md`, which `contract/codes.rs` builds into the binary
with `include_str!`. 2810 avoided this for `check-docs` by putting it in
the `tickets` job, which always runs; ADR-0032 avoided it for
`tools/layers.ron` by keeping it out of `docs/`.

**`run-gates` says "These mirror CI", but leaves out the `templates`
job.** That job (`ci.yml`, `templates:`) makes a game with `shuyi new` for
each template, runs the made game's gates, and, for the grid template,
proves the rules crate's fence (2015) and `shuyi lint` (2534) on code that
breaks them. 2015 and 2534 each added about 40 lines to it. Nothing in
the skill says how to run it locally. Ticket 3003 failed this job twice
on rustfmt's import order in a template file, because `templates/` is not
in the workspace and `cargo fmt --all` never touches it.

The docs map (`docs/README.md`, *Pairs*) also carries a stale row: "The
contract's operations … nothing yet (ticket 2017 adds `docs.rs`)". 2017
landed, so its *Checked by* cell is `crates/cli/tests/it/docs.rs` now.

## Nick input

None.

## Scope

**In:**
- **`.github/workflows/changes.yml`:** a pull request that changes any of
  these counts as code, whatever its extension:
  - anything under `crates/`, `templates/` or `examples/` (the binary
    builds in `crates/cli/errors/*.md`, `shuyi new` copies `templates/`,
    and the examples' tests read their own files);
  - `docs/contract.md` and the root `README.md` (`tests/it/docs.rs`).

  The rest stays docs-only: other `*.md`, `docs/**`, `tickets/**` and
  `.claude/**`. The case list puts the code paths first. The comment at
  the top of the file names each code path and the test or build step
  that reads it.
- **`.claude/skills/run-gates/SKILL.md`:** a section *A new game from
  each template (CI's `templates` job)* after the web build. It gives the
  commands from the job's steps: `cargo run -q -p shuyi-cli --locked --
  new <template> <dir> --engine-path <repo>` for `grid` and `blank`, then
  the made game's gates in `<dir>`, as the job lists them. It says that
  `templates/` is not in the workspace, so `cargo fmt --all` doesn't format
  its files and the made game's `cargo fmt --all --check` is what catches
  them. It says that the two proof steps (the fence holds, `shuyi lint`
  finds what breaks the game's code) run only in CI. The line *Later
  tickets add their checks here* stays.
- **`docs/README.md`, *Pairs*:** the contract's row's *Checked by* is
  `crates/cli/tests/it/docs.rs`. The local gates' row keeps "nothing
  yet".
- **`.github/workflows/ci.yml`:** the comments at the top and above the
  `tickets` job name `check-docs` beside the ticket lint, the links and
  typos.

**Out (do not do):**
- moving `tests/it/docs.rs` into the `tickets` job: it needs `shuyi-cli`
  built, and the `test` job builds it already;
- a check that `run-gates` and `ci.yml` list the same jobs;
- changing what the `templates` job runs: 2543.
  **Either order with 2543:** if 2543 is done, copy the made game's gates
  as `ci.yml` lists them then (with no `check-keys` line); if not, copy
  them as they are, and 2543 removes `check-keys` from the skill too.

## Implementation steps

1. Read `.github/workflows/changes.yml`, `ci.yml` (the `changes`,
   `test`, `templates` and `tickets` jobs), `crates/cli/tests/it/docs.rs`,
   `crates/cli/src/contract/codes.rs` (the `include_str!`) and the
   `run-gates` skill.
2. Change the case list in `changes.yml` and its comment.
3. Test the classifier by hand: run its shell loop over a list of paths
   (`docs/contract.md`, `README.md`, `crates/cli/errors/SH0001.md`,
   `templates/grid/AGENTS.md`, `docs/ROADMAP.md`, `tickets/open/x.md`,
   `CONTRIBUTING.md`) and check each says what Scope says. Put the
   output in the PR body.
4. Write the `run-gates` section, then run it on this machine for both
   templates and fix any step that doesn't work as written.
5. Fix the *Pairs* row and the `ci.yml` comments.

## Acceptance criteria

- [x] `changes.yml` classes `docs/contract.md`, `README.md`,
      `crates/cli/errors/*.md` and every path under `templates/` as code,
      and `docs/ROADMAP.md`, `tickets/**`, `.claude/**` and
      `CONTRIBUTING.md` as docs-only (the PR body shows the loop's
      output).
- [x] The `run-gates` skill has the templates section, and its commands
      pass on Nick's machine for `grid` and `blank`.
- [x] `docs/README.md`'s contract row names `crates/cli/tests/it/docs.rs`.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- None in Rust: the classifier is shell, tested by hand as step 3 says.

## Completion notes

Done as scoped. `changes.yml` now counts `crates/**`, `templates/**`,
`examples/**`, `docs/contract.md` and the root `README.md` as code; the
classifier loop gave the expected answer for all nine sample paths. The
`run-gates` skill has the templates section (both templates' commands
pass locally, MSYS2 on `PATH`). The docs map's contract row names
`docs.rs`; `ci.yml`'s comments name `check-docs`. No docs beyond these;
no follow-ups. Nick: nothing.
