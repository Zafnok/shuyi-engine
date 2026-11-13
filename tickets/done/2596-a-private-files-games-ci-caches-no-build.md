---
id: "2596"
title: "A game with private files caches no build in CI, which a fork's pull request could restore"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: low
status: done
blocked_by: []
nick_input: none
completed: 2026-11-12
---

# 2596 — A game with private files caches no build in CI, which a fork's pull request could restore

## Context

ADR-0111 §5 (ticket 2590): the template's `ci.yml` checks a game's
private files out before its gates, and no workflow may upload an
artifact that embeds them (game ADR-0117 §4). But the `gates` job then
runs `Swatinem/rust-cache`, which saves `target/` to the Actions cache
on a push to `main`, and `target/` holds builds that embed
`private/files/` (`include_dir!`). A pull request's run may restore the
caches of its base branch, a fork's pull request too, and a fork's
changed workflow can print or send what it restored: the private files
leave as surely as through an artifact.

2595 closed this in the engine's reusable playtest workflow: its
`playtest` job sets `cache-targets` to false when the game has the
action `.github/actions/private-files`. The game's own `ci.yml` still
caches `target/`.

## Nick input

None.

## Scope

**In:**
- `templates/shuyi-default/base/.github/workflows/ci.yml`: the
  `rust-cache` step caches no `target/` when the game has the private
  part (`cache-targets: ${{ hashFiles('.github/actions/private-files/action.yml') == '' }}`,
  as `.github/workflows/playtest.yml` does), with a comment saying why.
- Check every other job of the template's workflows for a cache that
  holds a build.
- ADR-0111's §5 rule, restated where `docs/templates.md` says it: no
  artifact **and no cache** embeds the private files.

**Out (do not do):**
- The game repository's own workflows (its own ticket there, if needed).

## Implementation steps

1. Read ADR-0111 §5 and `.github/workflows/playtest.yml`'s
   `rust-cache` step.
2. Change the template's `ci.yml`; bless `plans.golden`
   (`SHUYI_BLESS=1 cargo test -p shuyi-cli --lib template`).
3. Extend `the_private_part_writes_the_repository_its_ignore_line_and_its_ci_step`
   (`crates/cli/src/contract/new/tests.rs`): the `ci.yml` caches no
   target when the action is there.
4. `docs/templates.md`, *The private files*.

## Acceptance criteria

- [x] A game with private files saves no `target/` to the Actions cache.
- [x] A game without them caches as before.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- `crates/cli/src/contract/new/tests.rs`: the `cache-targets` line.

## Completion notes

- The template's `ci.yml`: its `rust-cache` step sets `cache-targets`
  to `hashFiles('.github/actions/private-files/action.yml') == ''`, as
  the engine's reusable playtest workflow does (2595), with a comment
  saying why. A game with private files still caches the cargo
  registry, which holds none of them. `plans.golden` blessed.
- Every other workflow of the template checked: the grid stage's
  `playtest.yml` only calls the engine's reusable workflow, whose
  `playtest` job is its one cache, guarded by 2595. No other job caches
  a build.
- `the_private_part_writes_the_repository_its_ignore_line_and_its_ci_step`
  checks the guarded `cache-targets` line and that `ci.yml` has one
  `rust-cache` step.
- `docs/templates.md`, *The private files*: no artifact **and no
  cache** embeds them. ADR-0111 is accepted and left as is; this
  restates its §5 rule.
- No deviations; the ticket is a day old.
- Book: none, the book's quick start names the private part but not
  what CI caches; nothing a reader does changes.
- Game: nothing to adopt, the template's workflow only reaches new
  games; the game's own workflows are out of this ticket's scope.
