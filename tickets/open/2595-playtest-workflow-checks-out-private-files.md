---
id: "2595"
title: "The reusable playtest workflow checks out a game's private files"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2595 — The reusable playtest workflow checks out a game's private files

## Context

ADR-0111 (ticket 2590): a game made with `--set private-files=yes`
lays its private repository's `files/` over its own files whenever the
checkout is there, and its `ci.yml` checks them out before the gates
with its action `.github/actions/private-files` and the secret
`PRIVATE_ASSETS_KEY`, so the gates read the real game (game ADR-0117
§3).

The grid stage's `playtest.yml` calls the engine's reusable workflow
(`.github/workflows/playtest.yml`, `workflow_call`), which checks out
only the game's repository: a game's bots play its public files, not
the real game (ADR-0111, *Consequences*). Game ADR-0117 §3 lists the
game's playtest jobs among those that check the content out.

## Nick input

None.

## Scope

**In:**
- The reusable workflow takes the key as a `workflow_call` secret
  (`PRIVATE_ASSETS_KEY`, optional) and, in every job that builds the
  game, runs the game's own `.github/actions/private-files` action after
  its checkout when the game has it (`hashFiles`), with the same rules:
  fail on the game's own repository without the key, warn on a fork's
  pull request.
- The template's grid `playtest.yml` passes the secret
  (`secrets: PRIVATE_ASSETS_KEY: ${{ secrets.PRIVATE_ASSETS_KEY }}`,
  never `secrets: inherit`).
- No job uploads an artifact that embeds the private files: check the
  workflow's uploads (a built tool program would).
- zizmor stays clean.

**Out (do not do):**
- The private part's action itself (2590).
- Anything in the game repository: it has its own playtest workflow.

## Implementation steps

1. Read ADR-0111 §5 and the game's `.github/workflows/playtest.yml`.
2. `.github/workflows/playtest.yml`: the secret, the step in each job
   that builds the game (`working-directory` relative), and the upload
   check.
3. `templates/shuyi-default/stages/top-down-grid/.github/workflows/playtest.yml`.
4. `docs/contract.md` (`playtest`'s workflow inputs) and ADR-0111's
   consequence: a new ADR line, or this ticket's notes, saying it holds
   now.

## Acceptance criteria

- [ ] A grid game made with the private part passes the key to the
      engine's playtest workflow, which checks the private files out
      before it builds.
- [ ] Hello, Grid's run of the workflow (CI's `playtest` job), which has
      no private part, is unchanged.
- [ ] No job of the workflow uploads an artifact that embeds the
      private files.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- `crates/cli/src/contract/new/tests.rs`: the grid game with the
  private part passes the secret.
- CI's `playtest` job, unchanged.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
