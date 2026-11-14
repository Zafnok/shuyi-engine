---
id: "2058"
title: A migration block's `since` is the workspace's version
type: infra
milestone: After MVP
model: haiku-4.5 / gpt-6-luna
effort: low
status: done
blocked_by: []
nick_input: none
completed: 2026-11-14
---

# 2058 — A migration block's `since` is the workspace's version

## Context

`tickets/README.md` (*Breaking changes*) defines a `migration` block's
`since` as "the engine version that first has the change, `X.Y.Z`: the
workspace `version` after the PR". The workspace is at `0.1.0`. The
README's own example gives `since` as `0.2.0`, and the retro of 2026-11-08
found that copied into done tickets: #268 (3042), #295 (2391) and in
all 28 blocks in 10 done tickets carry a `since` of `0.2.0`, against 54 with
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
2. `grep -rlnE "^ *since: 0.2.0" tickets/done` and fix each one.
3. Fix the README example.

## Acceptance criteria

- [x] A ticket moved to `done/` with a `since` of `0.2.0` while the workspace
      is `0.1.0` fails `cargo xtask changelog`.
- [x] `grep -rnE "^ *since: 0.2.0" tickets/done` finds nothing.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the compare, with an equal and a newer `since`.

## Completion notes

- `cargo xtask changelog` reads the tickets the PR adds to
  `tickets/done/` (`git diff --name-only --no-renames --diff-filter=A`
  against `--base <rev>`, else where `HEAD` left `origin/main`, else
  `main`; with none of them, as in a shallow clone, no ticket is new) and
  fails each `migration` block among them whose `since` is not
  `[workspace.package] version` of the root `Cargo.toml`, naming the
  file and the fence's line. Blocks already on `main` are not checked.
  CI's tickets job passes `--base HEAD^1` on a pull request, as the
  ticket lint does.
- `Migration` keeps its fence's line (`line`, not read from the YAML).
- `tickets/README.md`'s example writes `since: <workspace version>`, and
  `check` answers that placeholder with what to write instead.
- Every block in `tickets/done/` whose `since` was `0.2.0` says `0.1.0`: 39 blocks in
  15 tickets (the retro counted 28 in 10; five more tickets copied it
  since).
- `docs/ROADMAP.md` marks 2058 done.
- Tests: the compare with an equal and a newer `since`, a ticket already
  on `main` left alone, the placeholder's message, the manifest's
  version, and `run` with a fake `git` (with `--base`, and with no merge
  base). By hand: a probe ticket with a `since` of `0.2.0`, added with
  `git add -N` failed the command, naming its line.

Deviations: the ticket's own text named the old value as `since: 0.2.0`, so the grep of step 2 and the acceptance criteria found this ticket once it was in `done/`; the text now says "a `since` of `0.2.0`", and the grep is anchored to a block's line (`^ *since:`). No other change to the ticket.

Book: none, the change is to `cargo xtask` and the tickets, no public API.

No break: nothing public changed; `cargo xtask` is not shipped to games.
