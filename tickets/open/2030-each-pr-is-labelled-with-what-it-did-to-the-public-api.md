---
id: "2030"
title: "Each PR is labelled with what it did to the public API (api: none, api: added, api: breaking), so a game pinned by git revision knows when to move its pin"
type: infra
milestone: MVP
model: sonnet-5
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2030 — Each PR is labelled with what it did to the public API

## Context

1. **The game ticket:** game#1190
   (`github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1190-a-watcher-on-the-engines-main-files-a-pin-move-when-its-public-api-changes.md`).
   It replaces the game's hand-written "move the engine pin" tickets
   with a watcher. Every 15 minutes it reads this repository's `main`,
   and when an engine PR past the game's pin changed the public API, it
   files an issue in the game's repository to move the pin. Nick
   (2026-10-25): a weekly move is too slow for an engine built in a
   week, so the pin should follow the public API. It is not a blocker
   here.
2. **The job in one sentence:** tell a project that depends on the
   engine by git revision which merged PRs changed the engine's public
   API, and whether the change breaks.
3. **No friction review:** a missing feature.
4. **The generic shape:** CI's `api` job already computes the answer
   for every PR (ADR-0038: the listing diff of each crate in
   `API_CRATES` and `cargo-semver-checks`). It only writes the answer to
   the job's summary, and other repositories can't read job summaries
   through GitHub's API. This ticket puts the verdict where they can: as
   one label on the PR, `api: none`, `api: added` (the listing changed
   and semver finds no break) or `api: breaking`. GitHub's API returns a
   merged PR's labels to anyone, because the repository is public.
   Without it, game#1190 guesses from `pub` lines in each PR's patch,
   which counts too much (a `pub` item in a private module) and can't
   tell a break from an addition without a `Breaking:` line.
5. **An example that isn't tactics:** any game that tracks the engine's
   `main` by git revision before the first crates.io release (2005), for
   example a farming game made with `shuyi new`, wants to know "did
   anything I can call change?" before it spends a build on moving its
   pin. After 2005, crates.io versions answer that, and the labels still
   say which PR did it.

**Either order with game#1190:** if game#1190 is done, its watcher
reads this label from the first PR that has one (it prefers the label
when present and falls back to its `pub`-line test when not): nothing
to change in the game. If not, nothing to do: game#1190 reads the label
when it is written.

## Nick input

None.

## Scope

**In:**
- `cargo xtask api` writes a machine-readable verdict: `--verdict
  <path>` writes one word, `none`, `added` or `breaking`, by ADR-0038's
  rule: a semver finding on any crate is `breaking`, a listing diff
  without one is `added`, and nothing is `none`. A run that fails
  without a verdict (a build or tool error) writes nothing and fails as
  it does today.
- The `api` job in `.github/workflows/ci.yml` sets the matching label
  on the PR and removes the other two. A PR the job skips (no change
  under `crates/`, the `changes` job's output) gets `api: none` from a
  small step or job that needs no Rust.
- The three labels exist (`gh label create`), with a description each.
- ADR-0038 gains a short section: the label is the verdict for readers
  outside CI, and which project reads it (game#1190).
- `docs/contract.md` or the README section on depending on the engine
  by git revision says that consumers can read the label.

**Out (do not do):**
- A push to other repositories (`repository_dispatch`): it needs a
  token in this repository's secrets that can write to the other
  repository, and ADR-0037 (engine 2011) keeps tokens out of CI where
  it can. Consumers poll the public labels instead.
- Changing what counts as breaking (ADR-0038 §4).
- Labelling PRs from forks: the `pull_request` event gives them a
  read-only token. The step skips with a notice and no failure. A fork
  PR is then unlabelled, and readers fall back to their own test.

## Implementation steps

1. `crates/xtask/src/api.rs`: add `--verdict <path>`. The verdict is a
   pure function of the per-crate results the command already has
   (`fn verdict(results: &[CrateResult]) -> Verdict`), so it is
   unit-tested beside the existing fixtures in
   `crates/xtask/src/api/tests.rs`.
2. `ci.yml`, the `api` job: give the job `pull-requests: write` (job
   level only, with a comment saying why), pass `--verdict
   "$RUNNER_TEMP/api-verdict"`, then add a step that runs `gh pr edit
   "$PR" --add-label "api: $VERDICT" --remove-label …` for the other
   two. It runs only when `github.event.pull_request.head.repo.full_name
   == github.repository`, and reads every value through `env:`. A failed
   `api` run sets no label.
3. The skipped case: a job `api-none` with `needs: changes`, `if:` the
   PR event and `crates != 'true'`, `pull-requests: write`, and the same
   label step with `none`. It needs no checkout.
4. Labels: `gh label create "api: none" --color ededed`, `"api: added"
   --color 0e8a16`, `"api: breaking" --color d93f0b`, each with a
   description naming ADR-0038.
5. Docs as in Scope. The PR body says the game's watcher (game#1190)
   reads the label.

## Acceptance criteria

- [ ] `verdict` unit tests: no listing diff and no finding is `none`; a
      listing diff and no finding is `added`; any semver finding is
      `breaking`; a failed run writes no verdict file.
- [ ] On this ticket's own PR, the `api` job (or `api-none`) sets
      exactly one `api:` label, and the label on the PR matches the
      job's summary.
- [ ] `gh pr view <N> --json labels` shows the label after merge without
      a token, because the repository is public. Write the command in
      the Completion notes.
- [ ] `zizmor` passes on `ci.yml` (`security.yml`).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `verdict` (step 1).
- CI: the label on this PR (acceptance criteria).

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
