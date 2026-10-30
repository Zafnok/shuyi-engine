---
id: "2036"
title: "A ticket is one PR of about 1,500 changed lines or fewer, up from 600"
type: infra
milestone: After MVP
model: sonnet-5
effort: low
status: done
blocked_by: []
nick_input: none
completed: 2026-10-30
---

# 2036 — A ticket is one PR of about 1,500 changed lines or fewer

## Context

`write-ticket` says one PR's worth is "roughly 600 changed lines or fewer
excluding fixtures and snapshots". Nick, 2026-10-29, thinks that is too
aggressive: "Engine work is more technical than game so 1500 is about right
there". The game set its own limit the same day, about 3,000 changed lines
([game#0135](https://github.com/Zafnok/visions-of-shuyi/blob/main/tickets/done/0135-a-game-pr-stays-under-about-3000-changed-lines.md),
game ADR-0101), because a PR of about 7,000 ran a session out of context.

The engine keeps PRs small by estimating and by periodic checks, and that
stays: this ticket changes the number only. It is not a game request that
needs the pin moved, and no game ticket waits on it.

## Nick input

None. Nick set the number.

## Scope

**In:**
- `.claude/skills/write-ticket/SKILL.md`, step 5: about 1,500 changed lines
  or fewer, excluding fixtures and snapshots.
- Anywhere else the engine's process states the 600 as the rule
  (`grep -rnE "600 (changed )?lines" .claude docs CLAUDE.md tickets/README.md`).

**Out (do not do):**
- A CI check, a hook or a command that measures a PR: Nick didn't ask for one
  here, and the engine's estimates and periodic checks already do the job.
- Rewriting open tickets that were split or sized by the old number, or that
  mention it in passing.
- The skill pack in `skills/`, unless the grep finds the rule there.

## Implementation steps

1. Change the number in `write-ticket`, and wherever the grep finds it stated
   as the rule.
2. Add a line to `docs/decisions.md` with Nick's words, if that file keeps
   decisions of this kind.

Expected size: under 20 changed lines.

## Acceptance criteria

- [x] `write-ticket` says about 1,500 changed lines; the grep finds 600 stated
      as the rule nowhere else.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- None: process text only. The gates still run.

## Completion notes

`write-ticket`, step 5, now says about 1,500 changed lines or fewer. The grep
found the number stated as the rule nowhere else. `docs/decisions.md` has
Nick's words. As the ticket's **Out** says, the open tickets that mention about
600 lines keep their text. The new limit comes from the skill.

No game PR moves the pin for this: it changes process text only.

Book: none, this is the repository's process and no public item changed.
