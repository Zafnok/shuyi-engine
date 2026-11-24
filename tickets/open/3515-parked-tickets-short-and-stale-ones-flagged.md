---
id: "3515"
title: Parked tickets keep a short shape, and a monthly sweep flags tickets the code has moved under
type: infra
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3515 — Parked tickets keep a short shape, and a monthly sweep flags tickets the code has moved under

## Context

The backlog is as heavy as the product, and most of the weight is plans
for work that may not start for months. Counted on 2026-11-23 over the
tracked files of `tickets/open/`:

- 392 open tickets come to 4,077,338 bytes (about 4.08 MB);
- the 214 `Parked:` tickets alone come to 2,118,384 bytes, and every one
  of them has a full `## Implementation steps` section;
- 120 of the 214 were last changed on 2026-10-31 (`git log -1
  --format=%cs` per file);
- they name 237 distinct `crates/*/src/*.rs` paths, and 74 of those
  don't exist today. Some are files still to be made, but the count
  shows how fast the code moves under a plan that waits.

Nick parked real-time work until the game launches (`docs/decisions.md`,
2026-10-20), so most of that text will be months old when it is
unparked. Detailed steps that are out of date are worse than none: they
read as settled, and the session that unparks a ticket trusts them.

Ticket 2584 (done) made the `work-ticket` and `write-ticket` skills
check a ticket against the code when it is picked up. That is judgement
at pick-up time. Nothing checks a ticket while it waits, and the lint
knows nothing of how old a ticket's claims are.

This ticket does three things:

1. a parked ticket keeps its Context, Scope and Acceptance criteria, and
   its steps are written when it is unparked;
2. a ticket can say which commit its claims were checked against
   (`verified_against`);
3. a monthly workflow lists the open tickets whose named files changed
   or vanished since then, in one issue, as `dependabot-waiting.yml`
   lists waiting Dependabot PRs.

Source: the engine blind-spot audit of 2026-11-23 (proposal 87).

## Nick input

None.

## Scope

**In:**
- The short shape of a parked ticket: no `## Implementation steps` and
  no `## Tests required`. The ticket lint fails either heading in an open
  `Parked:` ticket. The `Parked:` note, Context, Nick input, Scope,
  Acceptance criteria and Completion notes stay.
- Compressing every parked ticket open when this lands (214 today, plus
  any added since) to that shape, with a pointer to the old steps in git.
- An optional `verified_against` frontmatter key: a commit id, 7 to 40
  lowercase hex characters. The lint checks the form only. It is optional
  because 392 tickets don't have it; none is required to add it.
- `cargo xtask stale-tickets`: a read-only report over `tickets/open/`.
  It never fails a build.
- `.github/workflows/stale-tickets.yml`: monthly, one issue labelled
  `stale-tickets`, rewritten each run and closed when the list is empty.
- The skills and `tickets/README.md` say when to set `verified_against`
  and that steps are written at unpark.

**Out (do not do):**
- Closing or merging groups of parked tickets (GBA and DS, 3D, consoles)
  into one tracking note. Which parked work to keep is Nick's call, and
  this ticket asks nothing of him.
- Making the lint fail a stale ticket. Code moving under a ticket is not
  a fault of the PR that moves it, so a gate on it would fail unrelated
  PRs. The sweep reports; a person or agent re-checks.
- Rewriting the steps of a ticket that is not parked.
- The roadmap's status cells, the `rank` frontmatter field and
  `cargo xtask tickets ready`: 3514. Both tickets add frontmatter fields
  to `TicketFields`; neither needs the other.
- The lint of the release's blockers: 2099.
- Editing `docs/ROADMAP.md` or `docs/decisions.md`.

**Estimated size:** about 1,300 lines (code 600, tests 450, docs and
skills 250). The rewrite of the parked tickets is left out of the count:
`cargo xtask pr-size` skips tickets.

**Split:** past 900 lines, split off steps 5–7 (`stale-tickets` and its
workflow) into a new ticket, and land the short shape, the compression
and the `verified_against` key here.

## Implementation steps

1. **The lint of the short shape.** In `crates/xtask/src/tickets.rs`,
   extend `check_parked` (it now checks `status: blocked` and the
   `> **Parked**` note) to push an error for an open `Parked:` ticket
   with a line `## Implementation steps` or `## Tests required`. The
   error says: "a `Parked:` ticket keeps Context, Scope and Acceptance
   criteria only; its steps are written when it is unparked
   (`tickets/README.md`, *Parked tickets*)". Done tickets are not
   checked.
2. **The `verified_against` key.** In `parse_ticket`, read it with
   `scalar_string`. When present and not empty, it must match
   `^[0-9a-f]{7,40}$`; otherwise an error names the value. Don't look
   the commit up: CI checks out shallow clones.
3. **Compress the parked tickets.** In the same PR, rewrite every open
   `Parked:` ticket:
   - remove its `## Implementation steps` and `## Tests required`
     sections, each up to the next `## ` heading;
   - put one line right before `## Acceptance criteria`:
     `**Steps as first written:** \`git show <commit>:<path>\`` (the
     merge commit on `main` before this PR, and the ticket's path then);
   - fix each acceptance line that names a step ("as in step 4"), found
     with `grep -n -i 'step [0-9]'`, so it stands on its own.

   Do it with a script in a scratch file that writes `\n` line endings,
   not with an xtask command: it runs once. Count the bytes before and
   after for the completion notes. The tickets of open PRs that add
   parked tickets (for example #532, 3325–3333) are compressed by the
   PR that rebases onto this one: the lint will fail them until then.
4. **The docs and skills.**
   - `tickets/README.md`, *Parked tickets*: the short shape, why, the
     pointer line, and that the session that unparks a ticket writes its
     steps and tests after checking the code, and sets
     `verified_against`. Add `verified_against` to the *Frontmatter*
     block with a comment.
   - `.claude/skills/write-ticket/SKILL.md`: a parked ticket, or a
     follow-up of one, is written in the short shape. Set
     `verified_against` to `git rev-parse --short origin/main` when the
     claims are checked (*Check the claims against the code*).
   - `.claude/skills/work-ticket/SKILL.md`: unparking writes the steps
     and the tests, and a ticket re-checked against the code gets its
     `verified_against` moved, in the PR that starts the work or a small
     ticket-only PR.
   - `tickets/TEMPLATE.md`: `verified_against:` in the frontmatter,
     empty.
   - Run `pwsh -File scripts/sync-agent-skills.ps1`, then `-Check`.
5. **`cargo xtask stale-tickets`.** A new module
   `crates/xtask/src/tickets/stale.rs`, dispatched from `main.rs` as
   `Some("stale-tickets")`. For each open ticket:
   - **Baseline:** its `verified_against`, or, without one, the last
     commit that changed the ticket's file (`git log -1 --format=%H --
     <file>`). The report says which.
   - **Named paths:** every token in the body that looks like a repo
     path: it starts with `crates/`, `docs/`, `tools/`, `scripts/`,
     `templates/`, `examples/`, `skills/`, `site/`, `.github/` or
     `.claude/`, and ends in a file extension. Strip backticks and
     trailing punctuation. Leave out globs (`*`) and paths in a
     `## Completion notes` section.
   - **Vanished:** a path that existed at the baseline (`git cat-file -e
     <baseline>:<path>`) and doesn't exist now. A path that didn't exist
     at the baseline either is a file the ticket plans to make, not a
     sign of age, and is not counted.
   - **Changed:** the number of commits on the first-parent history of
     `HEAD` since the baseline that touch each named path that still
     exists (`git log --first-parent --format=%h <baseline>..HEAD --
     <path>`).
   - **Behind:** the number of first-parent commits since the baseline.

   A ticket is **stale** when a path vanished, or a named path changed
   in 3 or more commits, or it is 150 or more first-parent commits
   behind. Name the three thresholds as constants beside their reason.
   A baseline that can't be found (a commit not in the clone) is a line
   of its own: "verified_against `<id>` is not in this clone".

   The output is Markdown on stdout: parked tickets and the others in
   two lists, each sorted by most vanished, then most changed. One line
   per ticket: its id and title in backticks, its baseline, and the
   vanished and changed paths (at most five, then "and N more").
   Nothing stale prints nothing and exits 0. `--json` prints the same
   as JSON, for the workflow and later tools.
6. **The workflow.** `.github/workflows/stale-tickets.yml`, on
   `schedule` (`0 7 1 * *`, the 1st of each month) and
   `workflow_dispatch`:
   - `permissions: contents: read` at the top; the job adds `issues:
     write`;
   - check out with `fetch-depth: 0` and `persist-credentials: false`,
     pinned as the other workflows pin `actions/checkout`;
   - install the pinned Rust the way `ci.yml` does, then run `cargo
     xtask stale-tickets > list.md`;
   - write, edit or close the one issue labelled `stale-tickets`, with
     the steps of `dependabot-waiting.yml` (`gh label create --force`,
     `gh issue edit` or `gh issue create`, `gh issue close` with the
     run's URL when the list is empty). The body ends: "Re-check each
     against the code with the `write-ticket` skill's *Check the claims
     against the code*, and move its `verified_against`, in a
     ticket-only PR. This issue is rewritten monthly by
     `.github/workflows/stale-tickets.yml`."
   - Run zizmor on it as CI does; a finding that is wrong here goes in
     `.github/zizmor.yml` with why.
7. **Where it is written down.** The workflow's own comment says what it
   does and names this ticket. Add the workflow and the command where
   the docs map (`docs/README.md`, the `document` skill) puts CI
   workflows and xtask commands.

## Acceptance criteria

- [ ] `cargo xtask ticket-lint` fails an open `Parked:` ticket with
      `## Implementation steps`, and one with `## Tests required`, with
      the error of step 1. A done ticket with either passes. An open
      ticket not parked with either passes.
- [ ] No open `Parked:` ticket has either heading, and each has the
      **Steps as first written** line; `git show` of the line's
      commit and path prints the old ticket.
- [ ] The completion notes give the parked tickets' bytes before and
      after the compression.
- [ ] `verified_against: 74736aa` passes the lint; `verified_against:
      main`, `verified_against: 74736A` and a 41-character id fail it,
      naming the value; no key, or an empty one, passes.
- [ ] `cargo xtask stale-tickets` in a test repository: a ticket naming
      a file that existed at its baseline and was deleted after is listed
      with that path as vanished. A ticket naming a file that never
      existed is not listed. A file changed in 2 commits since the
      baseline doesn't make it stale, and in 3 does. 149 commits behind
      is not stale, 150 is.
- [ ] A ticket whose `verified_against` is newer than its file's last
      change uses `verified_against` as its baseline, and the report
      says so.
- [ ] A `verified_against` not in the clone is reported on its own line,
      and the command still exits 0.
- [ ] Nothing stale prints nothing. Running the command twice prints the
      same thing.
- [ ] The workflow passes zizmor and `actionlint` as CI runs them, and a
      `workflow_dispatch` run on the branch writes, or closes, the
      `stale-tickets` issue (its URL in the completion notes).
- [ ] `tickets/README.md`, `tickets/TEMPLATE.md`, the `write-ticket` and
      `work-ticket` skills say what step 4 says, and
      `pwsh -File scripts/sync-agent-skills.ps1 -Check` passes.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`tickets.rs`): the two headings in a parked open ticket, the
  same in a done ticket and in an unparked one; each `verified_against`
  case in the acceptance criteria.
- Unit (`stale.rs`): the path finder over a body with backticked paths,
  trailing punctuation, globs, a path in `## Completion notes` and a
  word like `docs/` alone; the stale rule at each threshold, both sides.
- Integration: a temporary git repository built in the test (`git init`,
  commits that add, change and delete files, a ticket with and without
  `verified_against`), with `stale-tickets` run over it, as the other
  xtask commands' git tests do. Snapshot the Markdown and the JSON.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
