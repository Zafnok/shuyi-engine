---
id: "2052"
title: Ticket, ADR and contract numbers no open PR has taken
type: infra
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-08
---

# 2052 — Ticket, ADR and contract numbers no open PR has taken

## Context

The retro of engine PRs #254–#313 (2026-11-08) counted at least 13
renumber commits since 2026-10-22. Parallel sessions pick "the next
number" from their own checkout, so two of them take the same one:

- **Ticket ids:** 2250 had to become 2252 (#264), and 2581 became 2582
  (#285). The game's 1251 collided twice: game #567 → #578 as 1253, and
  1251 → 1252 in game #574.
- **ADR numbers:** 0083 was claimed by #280, #282 and #283 at once. #283
  went 0083 → 0084 → 0086, #284 went 0084 → 0085, and #285 went
  0083 → 0085 → 0086 → 0087. 2109's ADR went 0077 → 0078 → 0079 → 0080
  (#272). The squash messages on `main` still carry the wrong numbers:
  #283's says ADR-0083 where the file is 0086, and #282's says 0083
  where it is 0084.
- **The contract's version:** #297 and #298 both raised it to 38, and
  #306 hit 39/40. Ticket 2544's notes and #297's body still say 38.

`ticket-lint` fails duplicate ticket ids on one branch. Nothing checks
ADR numbers, and `write-adr`'s "check open PRs" step is prose that #272,
#280 and #282 did not follow. `write-ticket` step 2 doesn't look at open
PRs at all. The game's twin is game#0144.

## Nick input

None.

## Scope

**In:**
- `cargo xtask next-id ticket <block>` and `cargo xtask next-id adr`.
  Each prints the next number free on `origin/main` **and** in every
  open PR's added files (`gh pr list --state open --json files`). When
  `gh` has no access, it warns that it is reading `main` only.
- A check, run by `cargo xtask check-docs` or `ticket-lint`, that fails
  two ADR files sharing a number, or `docs/adr/README.md` listing one
  twice. CI runs on the merge ref, so the second PR to claim a number
  fails before it merges.
- A check that the PR raises `contract::VERSION` by exactly one over
  the base's when it changes it.
- `write-ticket` step 2 and `write-adr`'s numbering step use `next-id`.
- `work-ticket` §6 gets a step: after the last merge of `main`, re-read
  the PR body and completion notes for ADR, ticket and contract numbers
  that the merge moved.

**Out (do not do):**
- Rewriting history on `main` to fix the old squash messages.
- Placeholder numbers assigned at merge (considered; a merge-time
  rename can't update the links inside other files).
- The game's version (game#0144).

## Implementation steps

1. Add `crates/xtask/src/ids.rs` with `next_ticket_id(block)` and
   `next_adr_number()`. They read `git ls-tree origin/main` for
   `tickets/{open,done}` and `docs/adr`, and `gh pr list --state open
   --json number,files`.
2. Wire `next-id` into `crates/xtask/src/main.rs` and its usage text.
3. Add the duplicate-ADR check where `check-docs` reads `docs/adr/`, with
   an error naming both files.
4. The contract's version is `pub const VERSION` in
   `crates/cli/src/contract.rs` (41 today). Add the base-compare check to
   the `api` job's xtask, or to `ticket-lint --pr-branch`, comparing
   against `git show <base>:crates/cli/src/contract.rs`.
5. Update `write-ticket`, `write-adr` and `work-ticket`, then run
   `pwsh -File scripts/sync-agent-skills.ps1`.

## Acceptance criteria

- [x] `cargo xtask next-id adr` prints 0090 on today's `main`, or a
      higher number when an open PR adds 0090.
- [x] `cargo xtask next-id ticket 20` skips an id that an open PR adds.
- [x] Two files `docs/adr/0090-a.md` and `docs/adr/0090-b.md` fail the
      check, naming both.
- [x] A PR that raises `contract::VERSION` by two fails.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the next-number logic over lists of names, with gaps and with
  open-PR names; the duplicate-ADR check over a temporary `docs/adr`.
- Unit: the contract-version compare (same, +1, +2, lowered).

## Completion notes

- `crates/xtask/src/ids.rs`: `cargo xtask next-id ticket <block>` and
  `cargo xtask next-id adr` print one past the highest number found in
  `origin/main`'s tree (`git ls-tree`), this checkout's files and every
  open PR's files (`gh pr list --state open --json number,files`). A gap
  is never filled. Without `gh` (or `origin/main`) it warns and reads
  what it can. On 2026-11-08 `next-id adr` printed 0092, not 0090: 0090
  merged in #316 and open PR #318 adds 0091. `next-id ticket 20` printed
  2068, and `next-id ticket 25` printed 2585.
- `ids::duplicate_adrs`, run by `cargo xtask check-docs`: two files of
  `docs/adr/` with one number fail, naming both, and so does an index
  row listed twice in `docs/adr/README.md`.
- `crates/xtask/src/contract.rs`, run by `cargo xtask api` against its
  base (`HEAD^1` on CI's merge ref): `contract::VERSION` stays the
  base's or goes up by exactly one. +2 and a lowered version fail.
- Skills: `write-ticket` step 2 and `write-adr` step 1 use `next-id`;
  `work-ticket` §6 re-reads the PR body and notes for numbers a merge of
  `main` moved; `run-gates` names the two new checks. `docs/contract.md`
  says the raise is one over `main`'s. Loaders: `sync-agent-skills.ps1
  -Check` is clean (no description changed).
- Deviation: the branch is `t2052-next-free-numbers` (renamed from the
  worktree's `claude/…` name so `ticket-lint --pr-branch` reads it).
- No break: only `xtask`, which is not in the API catalog.

Book: none, the change is repository tooling for engine contributors; no
public item of an engine crate changed.
