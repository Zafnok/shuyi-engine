# ADR-0092: The ticket a PR is for is read from its title, and a PR that names none fails

- **Status:** Accepted
- **Date:** 2026-11-08
- **Related tickets:** 2051 (this), game#0143 (the game's twin), 2052
  (number collisions between open PRs)

## Context

CI read a PR's ticket only from a `t<NNNN>-<slug>` branch
(`ticket_id_from_branch`). A retro of engine PRs #254–#313 found that
the commonest way work was lost:

- A desktop worktree's `claude/<slug>-<hex>` branch read as "no ticket".
  The `api` job failed any break there, and `ticket-lint --pr-branch`
  skipped its "the ticket is in `done/`" check without saying so: on
  seven PRs neither that check nor the `Book:` rule ran. Renaming the
  branch closes the PR on GitHub, so two large PRs were closed and opened
  again.
- A PR that only wrote a ticket on a `t<NNNN>-` branch failed, as the
  lint wanted that ticket in `done/`. Ticket-only PRs had no convention.

Titles were already consistent: every merged ticket PR in the window used
`[NNNN] …`.

## Decision

1. **The title names the ticket first.** CI passes `--pr-title` beside
   `--pr-branch` to `cargo xtask ticket-lint` and `cargo xtask api`.
   `tickets::pr_kind(title, branch)` reads a `PrKind`:

   | Title | Kind |
   | ----- | ---- |
   | `[NNNN] …` | `Work(NNNN)` |
   | `Ticket NNNN: …`, `Tickets NNNN, MMMM and KKKK: …`, ranges `NNNN-MMMM` | `TicketsOnly(ids)` |
   | `Game NNNN: …` | `GameSmall(NNNN)` |
   | `sonar(…)`; a `sonar/` branch | `Sonar` |
   | `[advisory #N] …`, `engine-pin…`; a `dependabot/` branch | `Bot` |

2. **The branch is checked against the title.** A `t<NNNN>-` branch must
   name the title's ticket (a `TicketsOnly` title may name several). Any
   other branch passes when the title names a ticket. A title naming none
   falls back to a `t<NNNN>-` branch's ticket; with neither, the PR fails.
   With no title (a local run) the branch alone is read, as before, and a
   branch naming nothing is not checked.

3. **What each kind is checked for** (`check_pr`):
   - `Work(id)`: the ticket is in `done/`.
   - `TicketsOnly(ids)`: the diff against the base touches only
     `tickets/`, `docs/ROADMAP.md` and `docs/decisions.md` (the first
     other file is named), and each id is a ticket in `open/`.
   - The rest: nothing more of the tickets.

4. **ADR-0038 §5's rule reads the kind**, not the branch. A break needs a
   `Work` ticket in `done/` with a `Breaking:` line; any other kind with a
   break fails, as a branch with no ticket did. The `Book:` rule asks only
   a `Work` ticket. A `GameSmall` PR also fails when it adds to the public
   API: a small change of the game's needs no ticket here only because it
   changes no API.

5. **The title is read as it is when a job runs.** The `tickets` and `api`
   jobs ask GitHub for it (`scripts/pr-title.sh`), falling back to the
   event's title, so a re-run reads a retitle. `ci.yml` does **not** add
   `edited` to its triggers. A new `pr-title.yml` runs on `edited` when
   the title changed: it waits for the PR's latest CI run, then re-runs
   its failed jobs, or its `tickets` job if none failed, and with it
   `ci-result`. It checks nothing out, so its `actions: write` token never
   meets the PR's code.

## Consequences

- Work on a `claude/…` branch is checked like work on a `t<NNNN>-` one,
  and nobody renames a branch after the PR opens.
- A ticket-only PR has one form, and the lint keeps code out of it.
- A PR titled with no ticket fails at once, rather than skipping checks.
- A retitle costs a wait and one re-run, not a run of every job; a fork's
  PR (read-only token) has its CI re-run by hand.
- The game's lint is its own (game#0143).

## Alternatives considered

- **`edited` in `ci.yml`'s triggers**, as ticket 2051 first said: an edit
  of the body would re-run every job, and `cancel-in-progress` would
  cancel the run under way. Skipping the jobs on an edit that leaves the
  title would post a skipped, so passing, `ci-result` over a red one.
- **Keep the branch as the only signal and rename branches**: a rename
  closes the PR.
- **Read the PR body**: bodies vary far more than titles, and agents edit
  them while CI runs.
