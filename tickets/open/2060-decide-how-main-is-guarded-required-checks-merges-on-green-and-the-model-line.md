---
id: "2060"
title: "Decide how `main` is guarded: required checks, merges on green, and the `model` line"
type: decision
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: low
status: todo
blocked_by: []
nick_input: decision
completed:
---

# 2060 — Decide how `main` is guarded: required checks, merges on green, and the `model` line

## Context

The retro of the last 60 PRs in each repository (2026-11-08) found three
things only Nick can decide, because each one changes a setting or a
preference of his.

1. **`main` requires no status checks, and a push straight to `main`
   works.** The engine's ruleset has deletion, non-fast-forward and a
   pull-request rule with 0 approvals. The game's requires 8 checks.
   Ticket 3096 was pushed straight to the engine's `main` (`b67870fb`)
   from a game session, with no PR and no CI run. Nothing red has merged
   yet: every merged PR was green on its final commit. That was habit,
   not a guard.
2. **Green PRs wait for a batch merge.** The median wait from the last
   green check to merge was 7.2 hours in the engine (294 hours in all,
   against a median of 0.8 hours from open to green), and 2.5 hours in
   the game. Waiting caused 28 merge-from-main commits in 14 engine PRs,
   and the conflicts that come with them. Game #570 had been green and
   free of conflicts since 2026-11-07 and was skipped in the next batch
   with nothing on the PR to say why.
3. **The `model` line isn't followed.** Tickets 2041, 2046, 2047, 2411,
   2582, 3209, 2254, 2255, 3211, 2576, 2388, 3210 and 3212 recommend
   `sonnet-5 / gpt-6.1-terra`. All were worked by Opus 5.5 (their
   `Co-Authored-By` trailers). That may be fine; the line is Nick's
   routing preference (`tickets/README.md`).

## Nick input

**Decision:** three questions, asked with the `ask-nick` skill. Each
option's cost is in plain words.

1. Should `main` in the engine (and the game) refuse a push that isn't
   a PR, and refuse a merge until CI's `ci-result` is green?
   - Yes: nothing lands without CI, but a fix in a hurry still needs a
     PR.
   - No: as today.
2. Should a ticket PR merge itself when every check is green?
   - Yes, for ticket PRs and Dependabot PRs: no waiting, fewer
     conflicts, but Nick no longer sees each merge before it lands.
   - Yes, for Dependabot only.
   - No: Nick merges in batches, as today.
3. The `model` line:
   - Follow it: a session on the wrong model stops and says so.
   - Record it: a session says in the PR which model it was, nothing
     else.
   - Drop it: tickets stop naming a model.

## Scope

**In:**
- Ask the three questions and record the answers in
  `docs/decisions.md`.
- For each "yes", the steps for Nick: exact clicks in GitHub's
  *Settings → Rules → Rulesets* (the required check `ci-result`,
  "Require a pull request", and "Allow auto-merge" in *General*).
- Follow-up tickets for what the answers need in the code or skills
  (for example, `work-ticket` runs `gh pr merge --auto --squash`, or a
  `Model:` line in the PR body).

**Out (do not do):**
- Changing any setting. Nick does that himself.

## Implementation steps

1. Ask with `ask-nick`, one question at a time, in the words above.
2. Record each answer verbatim in `docs/decisions.md`.
3. Write the follow-up tickets (`write-ticket`), and the game's
   matching ones if Nick wants the game guarded the same way.

## Acceptance criteria

- [ ] `docs/decisions.md` has the three answers in Nick's words.
- [ ] Each "yes" has its follow-up ticket or setup steps.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- None (a decision).

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
