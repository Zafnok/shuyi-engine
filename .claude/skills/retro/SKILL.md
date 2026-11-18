---
name: retro
description: Run a retrospective over the last batch of merged PRs with about 20 read-only reviewers, report what went wrong and what went well, propose tickets, and file the ones the user picks. Use for /retro, "do a retro", "look back over the last N PRs", or "what went well and what went wrong".
---

# Retro

Looks back over a batch of merged PRs and turns what went wrong into
tickets. It changes no code: what it ships is a report, then a
ticket-only PR here and, when the game needs one, a ticket there.

## What Nick asks for

His words, 2026-11-17:

> use ~20 agents and conduct a retro on last 100 merged PRs and note any
> problems. This is pretty open ended, it can be design decisions that
> are bad, it can be redundancies, it can be prioritization issues, it
> can be performance issues. Come up with action items in the form of
> proposed tickets.

So a retro:
- is **open-ended**: design, redundancy, priority, performance, process,
  security, tests;
- is **broad**: about 20 reviewers, so that no one reviewer's view
  anchors it;
- ends in **tickets**, not prose: each finding is fixed by a proposed
  ticket, or by an edit to an open one.

What went well is reported too, briefly, so that it is kept.

Earlier retros:

| Date | PRs | Tickets |
| --- | --- | --- |
| 2026-10-22 | engine PRs 38–68, and the game's last 50 | #70, and #69 for the game's |
| 2026-11-08 | 60 PRs | #314 |
| 2026-11-17 | #379–#484 | engine #488, game #740 |

## Arguments

`/retro [N] [reviewers]`:
- **N:** without one, the PRs merged since the last retro. Find that
  with `git log --oneline -i --grep=retro origin/main`. If there was
  none, take 100.
- **Reviewers:** about 20. Claude Code runs at most 20 subagents at
  once, and a 21st fails to start. Start the extra ones as reports
  come back.

## 1. List the window

Work in a worktree. Write the list to the scratchpad, never the repo:

```bash
gh pr list --state merged --limit N \
  --json number,title,mergedAt,additions,deletions,changedFiles,author \
  --jq '.[] | "\(.number)\t\(.mergedAt[0:10])\t+\(.additions)/-\(.deletions)\t\(.changedFiles)f\t\(.author.login)\t\(.title)"'
```

Read every title, then count three things:
- the open and done tickets;
- the open ids in 2005's `blocked_by`, which is what is left before the
  release;
- the tickets opened and closed in the window, with
  `git log --diff-filter=A` on `tickets/open`.

## 2. Plan the reviewers

Split them about half and half.

**Batches.** Each batch reviewer reads 10 PRs closely, in merge order:
the body, the diff, and the ticket the PR closed.

**Themes.** Each theme reviewer follows one subject across the whole
window. Always include these six:
- prioritization against `docs/ROADMAP.md` and `docs/decisions.md`:
  sort the PRs into buckets with line totals, and trace the critical
  path to 2005;
- the public API before the release (ADR-0039, ADR-0038,
  `cargo xtask api`);
- CI and the local gates: time per PR from the gates tables in PR
  bodies, `gh run list` and `gh run view --json jobs`, flakes and
  re-runs, and the weekly jobs;
- the process: the lines of tickets, ADRs, docs and skills against
  Rust, ticket-only PRs, fix-up PRs, and skills that grew;
- redundancy in the tools: `crates/xtask`, `crates/cli`, `scripts/`
  and the templates;
- tests and runtime performance: allocation or parsing every frame,
  scans in hot paths, tests that can't fail or that flake.

Add 4–6 themes from the clusters you saw in the titles. Past ones:
text and i18n, assets and packs, the book and its playground, new-game
repositories and releases, sound and the platform backend, and the
worktree tooling.

## 3. Brief each reviewer

Send them all in one message (the `Agent` tool, `general-purpose`, in
the background). Each brief holds:

1. **Read-only.** Never edit, commit, push, comment on GitHub, re-run a
   workflow, or run a cargo build or test. Give the worktree's absolute
   path.
2. **Context to read first:** `AGENTS.md`, `docs/ROADMAP.md`, and the
   path of the PR list.
3. **Its slice:** the PR numbers, or the theme with its PRs, files and
   questions. Be concrete: "is X duplicated in Y?" or "does Z run every
   frame?".
4. **What to look for:**
   - bad design;
   - redundancy;
   - over-building;
   - prioritization;
   - performance (runtime, build, CI, disk);
   - fragility and flakes;
   - weak tests;
   - churn that breaks the game;
   - process friction.

   Be skeptical. Cite PR numbers and `file:line`. A few well-evidenced
   problems beat many nitpicks.
5. **Overlap:** grep `tickets/open` before proposing a ticket. If one
   already covers it, cite it, and say whether it looks mis-prioritized.
6. **The report, in at most 900 words:**
   - `## Findings`: each with a title, severity, evidence and why it
     matters;
   - `## Proposed tickets`: each with a title that states the outcome,
     why, what done looks like, a size, the finding it fixes and any
     overlap;
   - `## Went well`: at most 3 bullets.

When a reviewer starts after others have reported, tell it what is
already covered, so that it looks elsewhere.

## 4. While the reviewers run

- **Don't predict a report.** Say in one line which report came in,
  and what stood out.
- **Check every high-severity claim yourself before it goes in the
  report.** That means a leak, a red `main`, a game over its budget, or
  a missing check. Run a command to check it (`gh api`, a grep, reading
  the file), and say so in the report.

  Why: on 2026-11-17, "the game caches its private content" was half
  true. The steps ran in that order, but the pinned `rust-cache` deletes
  the workspace's own crates before it saves. A ticket's writer found
  this only after the report was out.

## 5. Write the report

Write it as `retro-prs-<first>-<last>.md` in the session's scratchpad,
outside the repository, and send it to the user. Two reasons it stays
out:
- A ticket-only PR can carry only `tickets/`, `docs/ROADMAP.md` and
  `docs/decisions.md`.
- The gates read untracked files: an untracked report fails `typos` in
  the pre-push hook, and fails `check-docs` ("not in the docs map").

It has six parts:

1. **The window at a glance:**
   - PRs, lines, and the share that is Rust;
   - tickets opened and closed;
   - what is left before the release;
   - a table of buckets.
2. **The top problems, ranked by harm.** About 10. Mark each one two or
   more reviewers reached on their own as **(×N)**, and each one you
   checked yourself as *confirmed*.
3. **Proposed tickets, in tiers:**
   - Tier 1: safety, the game and the release;
   - Tier 2: the dev loop, which saves the most time;
   - Tier 3: correctness and performance found in passing;
   - Tier 4: the tooling, the book and the process.

   Each ticket has an outcome title, a size, and 1–3 "done" bullets. An
   edit to an existing ticket is marked as an edit.
4. **Questions for Nick:** product calls only, asked later through the
   `ask-nick` skill.
5. **What went well.**
6. **Not filed yet:** a line saying so.

Then ask the user which tiers to file. They may add some ("the perf
ones too").

## 6. File the tickets

**Reserve the ids first.** After `git fetch origin`, run
`cargo xtask next-id ticket <NN>` for each block. Then:
- Take consecutive ids from the one it gives: no open PR has taken any
  id above it.
- **A block ends at NN99.** 2100 is not a `20xx` id. When a block is
  full, open a continuation block with a row in `tickets/README.md` and
  one in `docs/README.md`'s *What each task reads*. That is how `35xx`
  was made.

**Write them in parallel.** Give one agent 1–3 related tickets. Its
brief holds:
- the `write-ticket` skill, which it follows exactly;
- the report's path;
- the evidence for each ticket;
- the exact ids, the milestone, and whether 2005 waits for the ticket;
- an owner for every file two agents might touch:
  - 2005;
  - `docs/ROADMAP.md`, which you edit yourself;
  - an open ticket that two new tickets both answer.

  Other agents send that file's owner the lines they need, through you.

An agent that finds its ticket too big gets a new id from you to split
into, and does the split. An agent that finds an id is taken asks you.

**Settle each ticket's place:**
- **An MVP ticket goes in 2005's `blocked_by`** (the rule of
  2026-11-05). Growing the release's list is partly Nick's call, so the
  PR body says how much it grew and why.
- **The roadmap:** an MVP ticket gets a row before 2005 in Phase 2's
  table. Every other ticket goes in a Phase 3 section, *From the retro
  of PRs #A–#B*.

**The game's half** goes in a worktree of the game:

```bash
git -C <game clone> worktree add -b <branch> --no-track <path> origin/main
```

- Never touch the working tree of Nick's own clone.
- Follow the game's `write-ticket` skill. Its PR is titled
  `Ticket NNNN: …`.
- **The game's PR merges first:** the engine's `tickets` job looks each
  `game#NNNN` up on the game's `main`. Say so in both PR bodies.
- Lint the engine against the game's branch:
  `cargo xtask ticket-lint --game-repo <game worktree>`.

**The checks before the push:**
- `cargo xtask ticket-lint`;
- `typos`;
- `cargo xtask check-paths`.

**The PR:**
- **Title:** `Tickets NNNN, MMMM and KKKK: what a retro of the last N
  PRs found`, with ranges allowed. It names only tickets still in
  `open/`.
- **Body:** a table for each tier, the line for Nick, the merge order,
  and the open tickets edited for handoffs.

**When `main` closes one of your tickets while the PR is open:**
1. Merge `main` without taking either side of a file whole.
   `check-merges` fails any line changed outside a conflict.
2. Commit the follow-up edits after the merge. For example, move what
   the closed ticket left into the tickets beside it.
3. Run `cargo xtask check-merges` before you push.

If a bad merge is already pushed, never force-push. Rebuild the change
as one commit on a fresh branch from `main`, open it as a new PR, and
close the old one with a link to the new one.

## 7. Close

Wrap up with a short message:
- the links to both PRs;
- what joined the release;
- Nick's open questions;
- any claim the filing corrected.
