---
id: "2584"
title: The ticket skills check a ticket against the code before trusting it
type: infra
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-11
---

# 2584 — The ticket skills check a ticket against the code before trusting it

## Context

The retro of engine PRs #254–#313 (2026-11-08) rated 46 closed tickets.
11 held, 29 had minor deviations and 6 had major ones. Most of the
deviations came from a ticket that was wrong about the code when it was
worked. These need judgement, not a lint, so they belong in the skills:

- **Stale tickets.** Four of the six major deviations (2044, 2249, 2301,
  2544, 3042, 3046) were in tickets written 14–16 days before they were
  worked:
  - 3042 specified a method that 2411 had already built (`release_all`
    and `forget_held`), and neither ticket named the other;
  - 2350 and 2252 named template files and loaders that no longer
    existed.
- **Claims about the game nobody checked:**
  - 2388 said "the game builds no `RolePatterns`", which one code search
    disproves, and #307 had to add `RolePatterns::of`;
  - 2350 said "nothing in the game must change", but the game keeps
    `.dlg` text ASCII (game ADR-0045);
  - 3090 said game#1170 waited only on it, and 1170 turned out to need
    2388 and 3096 too.
- **A ticket written after its code encodes the code's bug.** 3210 was
  written and closed inside #294, and its acceptance case "23 lines at
  row 2" copied an off-by-one. It also contradicted its own "panels that
  already fit are painted as before". Fixing it took ticket-only PR #305
  (which needed its own test-table fix) and #313: three PRs, about 212
  lines, for a fix of about 10.
- **Evidence from a stale revision.** #308 (2817) tested a revision a
  day old and drafted a corrective ticket for a bug 2576 (#304) had
  already fixed.
- **PR bodies that don't match the final diff:**
  - #302 ticks "All gates" but lists fmt, clippy and tests;
  - #297's body says contract version 38 where the merge took it to 39;
  - #283's body names the wrong ADR.
- **ADR status.** ADR-0081 extends and narrows ADR-0008 §6, but
  ADR-0008's status line wasn't updated. `write-adr` covers only
  superseding.

## Nick input

None.

## Scope

**In:**
- `work-ticket` §0 or §1: for a ticket older than 7 days, re-check every
  type, function, file and ticket its steps name against `main` and
  against the tickets done since it was written. Fix the ticket in the
  same PR before coding, and record the fix as a deviation.
- `write-ticket`:
  - each claim about the game ("the game builds no X", "nothing in the
    game must change") is backed by a `gh search code --repo
    Zafnok/visions-of-shuyi` at the game's pinned engine, cited in the
    ticket;
  - each type or function the steps name is grepped in `tickets/open`,
    to find a ticket that plans the same thing;
  - a ticket that loosens or tightens a validator or format searches the
    game's ADRs for the rule and names the consequence.
- `write-ticket` and `work-ticket`: a change with anything to decide
  gets its ticket **before** the code. Acceptance values come from the
  requirement, never from pasting the code's output. Each changed bound
  or clamp gets a case that reaches it.
- `work-ticket` §5: manual evidence (a browser walk-through, a device
  run) is recorded at the tip of `main`, merged into the branch, and
  names that commit.
- `work-ticket` §6: before the last push, re-read the PR body against
  the final diff: gates run, numbers (ADR, ticket, contract version),
  and "no break" claims.
- `work-ticket`: a bug that the game's pin move finds is one PR, its
  ticket plus the fix, when the fix is small. Not a ticket-only PR
  followed by a fix PR.
- `write-adr`: a new ADR that extends, narrows or amends an older one
  updates the older one's status line to name it, as superseding does.
- Pictures in a PR body link at a commit, not at the branch, which is
  deleted on merge (game #516, #531, #534 and #555 have dead links).

**Out (do not do):**
- Lints for any of these (2051, 2052, 2053, 2059 do the deterministic
  ones).
- The game's skills (game#0148 does the game's half).

## Implementation steps

1. Edit `.claude/skills/work-ticket/SKILL.md`,
   `.claude/skills/write-ticket/SKILL.md` and
   `.claude/skills/write-adr/SKILL.md` with the steps above, each in
   the section it belongs to, each with its one-line reason.
2. Run `pwsh -File scripts/sync-agent-skills.ps1` and `-Check`.
3. Update ADR-0008's status line to name ADR-0081, as the first use of
   the new rule.

## Acceptance criteria

- [x] Each bullet of *In* is a step or rule in its skill, with its
      reason.
- [x] ADR-0008's status names ADR-0081.
- [x] `pwsh -File scripts/sync-agent-skills.ps1 -Check` passes.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- None beyond the gates (skill text).

## Completion notes

What was done:

- `work-ticket`:
  - §0 step 3 re-checks a ticket older than 7 days against `main` and
    against the tickets done since, and fixes it first;
  - §3 puts a decision's ticket before its code (acceptance values
    from the requirement, a case for each bound or clamp), and makes a
    small bug found by the game's pin move one PR;
  - §5 records manual evidence at the tip of `main` and names the
    commit;
  - §6 re-reads the PR body against the final diff before the last
    push, and links pictures at a commit.
- `write-ticket` has a new section, *Check the claims against the
  code* (step 7):
  - a claim about the game cites a `gh search code` at the game's pin;
  - each named type or function is grepped in `tickets/open`;
  - a validator or format change searches the game's ADRs;
  - the ticket comes before the code.
- `write-adr` step 5 has the old ADR's status line name a new ADR that
  extends, narrows or amends it, as superseding does.
- ADR-0008's status line, and its row in the ADR index, name ADR-0081
  (§6 extended and narrowed).
- `docs/ROADMAP.md` marks 2584 done.

Deviations:

- The ticket was 3 days old, so the new 7-day re-check didn't apply
  to it. Its named files (`scripts/sync-agent-skills.ps1`, the three
  skills, ADR-0008 and ADR-0081) all exist on `main`.
- The "pictures link at a commit" bullet names no skill. It went in
  `work-ticket` §6, beside the PR body's other rules.
- The sync script rewrote no loader. The loaders point at the shared
  skills, and no skill was added or renamed.

No break.

Book: none, the book is for people making games, and these are the
engine repository's own working skills.

Follow-up tickets: none. game#0148 does the game's half.
