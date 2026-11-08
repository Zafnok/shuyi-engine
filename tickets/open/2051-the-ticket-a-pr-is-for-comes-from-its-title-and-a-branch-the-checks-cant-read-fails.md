---
id: "2051"
title: The ticket a PR is for comes from its title, and a branch the checks can't read fails
type: infra
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2051 — The ticket a PR is for comes from its title, and a branch the checks can't read fails

## Context

A retro of engine PRs #254–#313 and game PRs #513–#580 (2026-11-08)
found that the commonest way work was lost was the branch name. CI reads
the ticket only from a `t<NNNN>-<slug>` branch
(`ticket_id_from_branch`, `crates/xtask/src/tickets.rs`). Two things
follow from that:

- **A desktop worktree's `claude/<slug>-<hex>` branch is read as "no
  ticket".** The `api` job then fails any break ("branch … names no
  ticket, and a change with no ticket never breaks the public API"), and
  `ticket-lint --pr-branch` skips its "the ticket is in `done/`" check
  without saying so. #263 (2301, about 2,660 lines) and #292 (2391) were
  closed and opened again as #264 and #295 after a branch rename. A
  rename closes the PR on GitHub. #266 changed its code to avoid a break
  rather than rename. On #257, #266, #299, #301, #309, #310 and #313 the
  done/ check and the `api` job's `Book:` rule never ran. 12 of the
  game's last 60 PRs used `claude/` branches too.
- **A PR that only writes or edits a ticket on a `t<NNNN>-` branch
  fails**, because the lint expects that ticket in `done/`. #252 was
  closed and redone as #254 for this, and game #552 → #553 and
  #567 → #578 likewise. Ticket-only PRs have no written convention. The
  engine's #254, #255, #259, #281, #290 and #305 each named their branch
  and title differently.

Titles are already consistent: 47 of 47 merged ticket PRs in the window
use `[NNNN] …`. So the title is the dependable signal. The game's twin of
this ticket is game#0143.

## Nick input

None.

## Scope

**In:**
- CI passes the PR title to `cargo xtask ticket-lint` and `cargo xtask
  api` (a `--pr-title` option beside `--pr-branch`).
- The ticket a PR is for comes from the title first:
  - `[NNNN] …` is a ticket's work PR, and `NNNN` must be in `done/`;
  - `Ticket NNNN: …` or `Tickets NNNN, MMMM…: …` is a ticket-only PR.
    Its diff touches only `tickets/`, `docs/ROADMAP.md` and
    `docs/decisions.md`, and the named tickets are in `open/` or new;
  - `Game NNNN: …` is a game ticket's small change (the `write-ticket`
    skill, "A small change needs no ticket here"). It adds no public
    item, since the `api` job already fails a break there;
  - a `sonar(…)`, Dependabot or `engine-pin` title keeps the treatment
    it has.
- The branch is checked against the title. A `t<NNNN>-` branch must name
  the title's ticket. Any other branch shape passes when the title names
  the ticket.
- A PR whose title names no ticket and whose branch names none is an
  error, except for the bot and sonar kinds above.
- The convention is written down: `tickets/README.md` gets a section on
  PR titles and branches, `write-ticket` gets "a ticket-only PR", and
  `work-ticket` §2 says to run `git branch -m t<NNNN>-<slug>` in a
  pre-made worktree **before the first push**, because a rename after
  the PR opens closes it.
- `run-gates`' tickets section runs `ticket-lint --pr-branch "$(git
  branch --show-current)"` with the title the PR will have.

**Out (do not do):**
- The game's lint (game#0143 does it).
- Renaming old branches or editing merged PRs.
- Number collisions between open PRs (2052).

## Implementation steps

1. In `crates/xtask/src/tickets.rs`, add `PrKind` (`Work(id)`,
   `TicketsOnly(ids)`, `GameSmall(game_id)`, `Bot`, `Sonar`, `None`)
   parsed by `pr_kind(title, branch)`. Keep `ticket_id_from_branch` as
   the fallback when no title is given (local runs).
2. Replace `check_pr_branch` with `check_pr(tickets, kind, changed)`.
   `changed` is the diff's file list against the base, which the lint
   already reads for the mirror check. Errors say what to do, for example
   "a ticket-only PR is titled `Ticket NNNN: …`".
3. In `crates/xtask/src/api.rs`, read the ticket the same way where the
   `Book:` rule and the "names no ticket" rule use
   `ticket_id_from_branch`.
4. In `.github/workflows/ci.yml` (the `tickets` and `api` jobs), pass
   `--pr-title "$PR_TITLE"` with `PR_TITLE: ${{ github.event.pull_request.title }}`
   in `env:`, never inline. That keeps zizmor's template-injection rule
   happy. Add `edited` to the `pull_request` types so a retitle re-runs
   the check.
5. Write the convention into `tickets/README.md`, `write-ticket`,
   `work-ticket` §2 and §6, and `run-gates`' tickets section. Then run
   `pwsh -File scripts/sync-agent-skills.ps1`.
6. Write an ADR if this changes ADR-0038's reading of the branch, which
   is likely (`write-adr`).

## Acceptance criteria

- [ ] A `claude/x-1a2b3c` branch titled `[2051] …` with 2051 still in
      `open/` fails, and passes once 2051 is in `done/`.
- [ ] A `t2051-x` branch titled `Ticket 2051: …` that only adds
      `tickets/open/2051-….md` passes.
- [ ] A ticket-only title whose diff touches `crates/` fails, naming the
      first such file.
- [ ] A `[2051]` title on a `t2052-…` branch fails.
- [ ] The `api` job reads the ticket from a `claude/…` branch's title,
      checking both `Breaking:` and `Book:` lines.
- [ ] Renaming a PR's title re-runs the `tickets` job.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `pr_kind` over each title shape and branch shape, including
  `Tickets 2005 and 2823: …` (#281's form) and `Game 0859: …`.
- Unit: `check_pr` with each kind and a changed-file list.
- Integration: the existing `--pr-branch` tests in `crates/xtask` keep
  passing with no title.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
