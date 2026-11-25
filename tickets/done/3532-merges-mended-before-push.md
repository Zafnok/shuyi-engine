---
id: "3532"
title: "A merge that carries edits of its own is split before it's pushed: cargo xtask mend-merges, a post-commit hook that runs it, and the pre-push hook checking merges"
type: infra
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-24
---

# 3532 — A merge that carries edits of its own is split before it's pushed

## Context

Ticket 2065 made `cargo xtask check-merges`: a merge commit may change
only what resolving its two parents needs. CI's `tickets` job runs it on
every PR. It catches real problems (#285 smuggled a local
`.codex/config.toml` back through a merge; #277 hid a fix in one).

But nothing runs it before a push. The pre-push hook runs `cargo xtask
gates --fast` (fmt, typos, the ticket lint, the engine lint), without
it. `cargo xtask gates` runs it among the tickets' checks, but sessions
run the gates before they commit, when the merge isn't a commit yet. So
CI is the first to see a bad merge, after it is pushed, and then the only
ways out rewrite the branch: a force-push, or a new branch and a new PR.

It happens often: of the PRs closed unmerged from 2026-11-17 to
2026-11-24, ten of eleven were replaced for this alone (#465, #472, #486,
#495, #520, #526, #538, #552, #562, #565), each by a new PR with the same
files. The edits that rode in the merges were mostly an ADR or ticket
renumbered after `main` took its number (#472, #520, #552, #565), a lint
fix (#495), or a clause updated for what `main` brought.

Nick, 2026-11-24: "weve been having this issue where we need a new pr or
force push for a while can we pls figure out what the friction point is
and rm it? I dont like force pushes."

## Nick input

None.

## Scope

**In:**
- **`cargo xtask mend-merges`** (`crates/xtask/src/merges.rs`): over the
  branch's commits no remote-tracking branch has (`git rev-list HEAD
  --not --remotes`), each merge that `check-merges` refuses is split in
  two:
  1. the same merge of the same parents, its conflicts resolved to the
     branch's side (`git merge-tree --write-tree -X ours`), which
     `check-merges` passes;
  2. a commit on top that holds the rest: the original merge's tree.

  The commits after it are made again on the new ones with their own
  trees, authors and messages. The branch's last tree is unchanged, so
  the working copy and the index stay as they are; only history
  changes, and only history no remote has, so a plain push follows. It
  refuses, changing nothing, when: HEAD isn't a branch; a merge, rebase
  or cherry-pick is in progress; a refused merge is already on a remote
  (then say: a new branch, since its history can't change without a
  force-push); or git can't remerge the parents by content alone (a
  rename or delete conflict: split it by hand, as the message says).
- **A post-commit hook** (`scripts/git-hooks/post-commit`): after a
  merge commit (HEAD with two parents), it runs `cargo xtask
  mend-merges`, so a merge with edits of its own is split the moment it
  is made. It never fails the commit, and says what it did.
- **The pre-push hook** also runs `cargo xtask check-merges` over what
  each ref pushes (from the remote's commit if it has it, otherwise from
  `origin/main`), and refuses a push that `check-merges` refuses, naming
  `cargo xtask mend-merges`.
- **`check-merges`'s errors** name `cargo xtask mend-merges` for a merge
  not yet pushed.
- **Docs:** `AGENTS.md`'s hooks bullet, the `run-gates` skill's hooks
  section, and the `work-ticket` skill's step on merging `main` (commit
  the merge with only its resolution; a renumbering is its own commit,
  and the hook splits out what slips in).

**Estimated size:** about 600 lines (code 250, tests 250, docs and hooks
100).

**Out (do not do):**
- Relaxing `check-merges` itself: what it refuses stays refused.
- Rewriting history that a remote has, or any force-push.
- Octopus merges (check-merges already refuses them).

## Implementation steps

1. `mend` in `crates/xtask/src/merges.rs`, and the `mend-merges` command
   and its usage line in `crates/xtask/src/main.rs`.
2. The hooks: `post-commit` new, `pre-push` extended.
3. The error text of `check-merges`.
4. The docs above; `pwsh -File scripts/sync-agent-skills.ps1 -Check`.
5. Tests below, and a run on this repository.

## Acceptance criteria

- [x] In a temporary repository, a merge that edits a file outside its
      conflict is split by `mend-merges` into a merge `check-merges`
      passes and a commit with the edit; the branch's last tree is the
      same, and commits after the merge keep their trees and messages.
- [x] A clean history is left unchanged.
- [x] A refused merge that a remote-tracking branch has is refused by
      `mend-merges`, changing nothing.
- [x] A rename or delete conflict is refused with the steps to split it
      by hand, changing nothing.
- [x] The post-commit hook splits a bad merge when it is committed, and
      does nothing after any other commit.
- [x] The pre-push hook refuses a push with a refused merge, naming
      `cargo xtask mend-merges`.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Integration, in temporary repositories (as `merges.rs`'s tests): each
  acceptance case above but the hooks; the hooks by hand, noted in the
  completion notes.

## Completion notes

What was done:

- **`cargo xtask mend-merges`** (`crates/xtask/src/merges/mend.rs`): over
  `git rev-list HEAD --not --remotes`, each merge `check-merges` refuses
  is made again from the same parents with `git merge-tree --write-tree
  -X ours` (which changes nothing outside a conflict), and a commit on
  top holds the original merge's tree. Later commits are made again with
  `git commit-tree`, keeping their trees, authors and messages. It checks
  that the last tree is unchanged and that every remade merge passes
  before it moves the branch (`git update-ref` with the old value), so
  the working copy and the index are untouched. It refuses, changing
  nothing, off a branch, during a merge, rebase or cherry-pick, when a
  refused merge is already on a remote, and on a rename or delete
  conflict (with the steps to split it by hand).
- **Hooks:** a new `scripts/git-hooks/post-commit` runs it after each
  merge commit and never fails the commit; `scripts/git-hooks/pre-push`
  now also runs `check-merges` from the remote's commit (or
  `origin/main`) to each pushed commit, naming `mend-merges` when it
  refuses.
- `check-merges`'s errors name `mend-merges`. `AGENTS.md`, the `run-gates`
  skill and the `work-ticket` skill (a step on merging `main`) say how it
  works; `xtask`'s usage text lists the command.
- **Tests:** four in temporary repositories (a merge with edits split
  and the last tree kept, a later commit carried; a clean history left
  alone; a pushed bad merge refused; a delete conflict left to be split
  by hand).
- **The hooks, by hand** (a scratch clone of this branch, 2026-11-24):
  a merge that resolved its conflict and also edited `README.md` was
  split by the post-commit hook as it was committed, and `check-merges`
  passed. With that hook off, the pre-push hook refused the push to a
  local bare repository, naming `mend-merges`; after it ran, the same
  push went through with no force.

Notes for Nick: the hooks run from `core.hooksPath`, which on your
machine points at the main checkout's `scripts/git-hooks`, so they start
working there once the main checkout pulls this. No game change: the
engine's own tooling only.

Book: none, this repository's own git tooling; no engine API, format or
rule a game sees changes.

Game: nothing to adopt, the engine's repository tooling only.

No break. No behaviour change a game sees.
