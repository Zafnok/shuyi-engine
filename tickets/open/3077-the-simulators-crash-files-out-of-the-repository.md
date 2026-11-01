---
id: "3077"
title: "The Simulator's crash files out of the repository: .sentry-native untracked and ignored"
type: infra
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: low
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3077 — The Simulator's crash files out of the repository

## Context

- [PR 197](https://github.com/Zafnok/shuyi-engine/pull/197) (ticket
  3010) committed both `.sentry-native/` at the repository root and
  `examples/hello-grid/.sentry-native/`: crash reporter run files written
  by the Playdate Simulator in the folder it starts in. They aren't the
  engine's. The example's files were found while working 3029; the
  2026-11-01 PR review found that the root's files need removing too.
- The tracked state includes simulator session/installation identifiers
  and OS metadata. No credential was identified in the review; these
  generated files still do not belong in source control.
- Any `cargo shuyi export playdate --run` from the example deletes those files
  and writes others, so a clean checkout goes dirty, and `git add -A` would commit
  it.

## Nick input

None.

## Scope

**In:**
- Remove every tracked file under both actual `.sentry-native/`
  directories above, including the lock files and identifiers. Add
  `.sentry-native/` to the root `.gitignore`, matching it at any depth,
  for the root, a game's folder and the example's alike.
- `templates/shuyi-default`'s `.gitignore`, if a game gets one, the same.

**Out (do not do):**
- Starting the Simulator in another folder.
- Rewriting git history or changing the Simulator's telemetry settings.

## Implementation steps

1. Inventory the tracked files with `git ls-files '*sentry*'`; verify
   both resolved directory paths are inside this checkout, then remove
   the tracked root and example crash directories.
2. Add `.sentry-native/` to `.gitignore`, and to the template's if it has one.
3. Run the Simulator once from the root and once from the example; check
   both their generated folders are ignored and no tracked file changes.

## Acceptance criteria

- [ ] `git ls-files '*sentry*'` prints nothing, including root and example
      crash-reporter paths.
- [ ] `git check-ignore` recognizes a generated file under either
      `.sentry-native/` location and under a new game's copied ignore rules.
- [ ] After Simulator runs from both the root and `examples/hello-grid`,
      `git status` has no new or modified tracked crash files; the
      completion notes record both checks.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

None: a repository's files.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
