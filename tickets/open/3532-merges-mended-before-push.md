---
id: "3532"
title: "A merge that carries edits of its own is split before it's pushed: cargo xtask mend-merges, a post-commit hook that runs it, and the pre-push hook checking merges"
type: infra
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: medium
status: in-progress
blocked_by: []
nick_input: none
completed:
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

- [ ] In a temporary repository, a merge that edits a file outside its
      conflict is split by `mend-merges` into a merge `check-merges`
      passes and a commit with the edit; the branch's last tree is the
      same, and commits after the merge keep their trees and messages.
- [ ] A clean history is left unchanged.
- [ ] A refused merge that a remote-tracking branch has is refused by
      `mend-merges`, changing nothing.
- [ ] A rename or delete conflict is refused with the steps to split it
      by hand, changing nothing.
- [ ] The post-commit hook splits a bad merge when it is committed, and
      does nothing after any other commit.
- [ ] The pre-push hook refuses a push with a refused merge, naming
      `cargo xtask mend-merges`.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Integration, in temporary repositories (as `merges.rs`'s tests): each
  acceptance case above but the hooks; the hooks by hand, noted in the
  completion notes.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
