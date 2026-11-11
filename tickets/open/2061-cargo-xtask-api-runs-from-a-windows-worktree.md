---
id: "2061"
title: "`cargo xtask api` runs from a Windows worktree"
type: bug
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2061 — `cargo xtask api` runs from a Windows worktree

## Context

`cargo xtask api` (ADR-0038) is the local copy of CI's `api` job. In the
retro of 2026-11-08, sessions working from a git worktree on Nick's
Windows machine reported that it "couldn't run locally (a Windows path
error in its base copy)": #271, #291, #293 and #295. Every one fell back
on CI. #292's API failure was then found only in CI, and that PR was
closed and redone. #296 (2041) also saw `cargo-semver-checks` crash on
Windows and wrote "if it comes back, it is worth a ticket". This is that
ticket.

## Nick input

None.

## Scope

**In:**
- Reproduce from a worktree under `.claude/worktrees/` on Windows with
  the GNU toolchain.
- Fix the base copy so it works there. The likely suspects are a
  `git worktree add` or `git archive` path handed to a tool as an MSYS
  path, a path over `MAX_PATH`, or a `\\?\` prefix.
- If `cargo-semver-checks` itself crashes, record the version and the
  backtrace in the notes, and work around it in xtask. No issue
  upstream (`AGENTS.md`, rule 12).
- `run-gates`' *The public API* section says it works from a worktree.

**Out (do not do):**
- Changing what the `api` job checks.

## Implementation steps

1. Run `cargo xtask api --base origin/main` from a worktree and save the
   full error. Find the base-copy code in `crates/xtask/src/api.rs`.
2. Fix the path handling. Two known traps on this machine are where to
   look first: Git Bash (MSYS) rewrites a `/`-prefixed argument into a
   Windows path, and Windows tools want a Windows path (`pwd -W`, not
   `$PWD`).
3. Add a test for the path conversion where the code allows one.

## Acceptance criteria

- [ ] `cargo xtask api --base origin/main` completes from a worktree
      under `.claude/worktrees/` on Windows (output pasted in the notes).
- [ ] The `cargo-semver-checks` crash, if there is one, is worked
      around in xtask, its version and backtrace in the notes.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the path conversion, if one is added.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
