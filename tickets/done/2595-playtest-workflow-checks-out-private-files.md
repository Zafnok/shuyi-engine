---
id: "2595"
title: "The reusable playtest workflow checks out a game's private files"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-12
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

- [x] A grid game made with the private part passes the key to the
      engine's playtest workflow, which checks the private files out
      before it builds.
- [x] Hello, Grid's run of the workflow (CI's `playtest` job), which has
      no private part, is unchanged.
- [x] No job of the workflow uploads an artifact that embeds the
      private files.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- `crates/cli/src/contract/new/tests.rs`: the grid game with the
  private part passes the secret.
- CI's `playtest` job, unchanged.

## Completion notes

- `.github/workflows/playtest.yml` takes `PRIVATE_ASSETS_KEY` as an
  optional `workflow_call` secret. The `playtest` job (the only one that
  builds; `detect` only diffs) runs the game's own
  `./.github/actions/private-files` after its checkout and before
  `rustup`, the cache and the build, when
  `hashFiles('.github/actions/private-files/action.yml')` finds it. The
  action keeps its rules: it fails a run on the game's own repository
  without the key and warns on a fork's pull request.
- The grid stage's `playtest.yml` passes the secret by name
  (`secrets: PRIVATE_ASSETS_KEY: ...`). A grid game without the part
  passes an empty secret, which nothing reads. Test:
  `the_grid_game_with_private_files_passes_their_key_to_the_playtest`.
- Uploads: the workflow uploads no artifact. A cache is like one,
  though: a pull request can restore its base branch's caches, a fork's
  too. So a game with the action caches no `target/` in this job
  (`rust-cache`'s `cache-targets`), and the template's `ci.yml`, which
  does, is ticket 2596.
- Docs: `docs/contract.md` (*The playtest in CI*: the secret),
  `docs/templates.md` (*The private files*), the part's `AGENTS.md`.
- ADR-0111's consequence "the engine's reusable playtest workflow checks
  out no private files ... until ticket 2595" holds no more: the
  workflow checks them out for a game that passes the key. Recorded
  here, as the ticket allows, not in a new ADR.
- Deviation, `working-directory`: a step's `uses:` can't hold an
  expression, and the game's action reads `private.rev` and checks out
  `private/` at the repository's root. So the step runs the root's
  action, and a game in a subfolder that has its own action is refused
  with an error rather than played on its public files. A game with its
  own CI is at its repository's root anyway (GitHub reads workflows
  only there). Hello, Grid (`examples/hello-grid`) has no action, so
  CI's `playtest` job is unchanged: both new steps are skipped and the
  cache keeps its targets.
- zizmor at CI's `min-severity: medium`: clean. Locally it shows one
  low `self-repository` help on `./.github/actions/private-files`, as on
  the template's `ci.yml`; its `$/` form would name the engine's
  repository from a reusable workflow, which is wrong here.

Book: none, the book says what the private part does on the developer's
machine (quick start); CI's checkout is in the game's `AGENTS.md`.

Game: nothing to adopt, the game has its own playtest workflow, which
checks its content out already.
