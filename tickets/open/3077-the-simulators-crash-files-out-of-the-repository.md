---
id: "3077"
title: "The Simulator's crash files out of the repository: .sentry-native untracked and ignored"
type: infra
milestone: After MVP
model: implementation
effort: low
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3077 — The Simulator's crash files out of the repository

## Context

- PR 3010 committed `examples/hello-grid/.sentry-native/`: a crash reporter's
  run files, written by the Playdate Simulator in the folder it starts in. They
  aren't the engine's. Found while working 3029.
- Any `cargo shuyi export playdate --run` from the example deletes those files
  and writes others, so a clean checkout goes dirty, and `git add -A` would commit
  it.

## Nick input

None.

## Scope

**In:**
- `git rm -r` the folder, and add `.sentry-native/` to the root `.gitignore`, for
  a game's folder and the example's alike.
- `templates/shuyi-default`'s `.gitignore`, if a game gets one, the same.

**Out (do not do):**
- Starting the Simulator in another folder.

## Implementation steps

1. `git rm -r examples/hello-grid/.sentry-native`.
2. Add `.sentry-native/` to `.gitignore`, and to the template's if it has one.

## Acceptance criteria

- [ ] `git ls-files | grep sentry` prints nothing.
- [ ] After a run of the Simulator from `examples/hello-grid`, `git status` is
      clean.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

None: a repository's files.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
