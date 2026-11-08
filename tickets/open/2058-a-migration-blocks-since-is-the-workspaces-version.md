---
id: "2058"
title: A migration block's `since` is the workspace's version
type: infra
milestone: After MVP
model: haiku-4.5 / gpt-6-luna
effort: low
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2058 — A migration block's `since` is the workspace's version

## Context

`tickets/README.md` (*Breaking changes*) defines a `migration` block's
`since` as "the engine version that first has the change, `X.Y.Z`: the
workspace `version` after the PR". The workspace is at `0.1.0`. The
README's own example says `since: 0.2.0`, and the retro of 2026-11-08
found that copied into done tickets: #268 (3042), #295 (2391) and in
all 28 blocks in 10 done tickets carry `since: 0.2.0`, against 54 with
`0.1.0`. `cargo xtask changelog`
checks only that the value is a version. `cargo xtask changelog` (2804)
and the book's "what changed since your version" page (2829) group by
`since`, so a wrong value puts the migration under a release that
doesn't have it.

## Nick input

None.

## Scope

**In:**
- `cargo xtask changelog` fails a `migration` block in a ticket moved to
  `done/` by the PR whose `since` is not the workspace version on the
  branch. Blocks already on `main` get a one-time fix, not a check.
- The README example uses a placeholder the lint recognises (for
  instance `since: <workspace version>`) or the current version.
- Every done ticket's block with a `since` newer than the version it
  shipped in is corrected to `0.1.0`, the version every one of them
  shipped in, since nothing has been released past 0.1.0.

**Out (do not do):**
- Raising the workspace version.
- Changing the migration format.

## Implementation steps

1. Find the migration parser in `crates/xtask/src/changelog.rs`. Add
   the version compare against the workspace `version` in the root
   `Cargo.toml`, applied to tickets added to `done/` in the PR (the diff
   against the base, as `ticket-lint --pr-branch` reads it).
2. `grep -rln "since: 0.2.0" tickets/done` and fix each one.
3. Fix the README example.

## Acceptance criteria

- [ ] A ticket moved to `done/` with `since: 0.2.0` while the workspace
      is `0.1.0` fails `cargo xtask changelog`.
- [ ] `grep -rn "since: 0.2.0" tickets/done` finds nothing.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the compare, with an equal and a newer `since`.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
