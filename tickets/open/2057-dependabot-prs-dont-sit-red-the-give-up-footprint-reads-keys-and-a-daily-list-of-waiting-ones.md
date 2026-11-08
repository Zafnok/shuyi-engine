---
id: "2057"
title: "Dependabot PRs don't sit red: the give-up footprint reads keys, and a daily list of the waiting ones"
type: infra
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2057 — Dependabot PRs don't sit red: the give-up footprint reads keys, and a daily list of the waiting ones

## Context

At the retro of 2026-11-08, Dependabot PRs #286–#289 had been open about
two days with every `ci-result` red, and no session had touched them.
Earlier batches also waited 2–4 days. Nothing picks them up. The
`dependency-upkeep` skill (ADR-0037) runs only when someone points a
session at a PR, and `work-ticket`'s "do the next ticket" reads only
`tickets/open/`.

Two of the four fail for a reason unrelated to the bump:

- **#287 (actions) and #288 (sha2 0.11):**
  `tools/sound-parity/footprint.ron` stores the exact action SHAs and the
  text `sha2 = "0.10"`. So any bump of those fails `sound-give-up` and the
  xtask test `giving_up_covers_all_the_chain_added`. The SHAs went in
  with #246, after the last actions bump, so every future actions PR
  fails the same way.
- **#286 (ron 0.12.3):** a patch release moved error columns, and three
  `shuyi-assets` tests that pin columns fail.
- **#289 (lewton 0.10):** a new game now pulls 147 crates, over the 144
  in `tools/compile-budget.ron`.

`dependency-upkeep` names neither the footprint nor the compile budget.

## Nick input

None.

## Scope

**In:**
- The give-up footprint matches what it must remove by key, not by
  literal text: an action by its `uses:` name without the SHA, and a
  dependency by name without its version. Alternatively, a `cargo xtask
  sound-parity --refresh-footprint` rewrites the literal parts, and
  `dependency-upkeep` runs it. Pick one and say why in the notes.
- `dependency-upkeep` gets steps for the three failure kinds above:
  - refresh the footprint;
  - decide the compile budget (raise it, with the crate count and why,
    or refuse the bump);
  - for a moved error column, update the test only, after checking that
    the message itself is unchanged.
- A scheduled workflow (daily) that keeps one GitHub issue, labelled
  `dependabot-waiting`, listing every Dependabot PR open more than 24
  hours, with its failing jobs. It closes the issue when the list is
  empty. `work-ticket` §1 checks that issue before choosing a ticket.
- Work #286–#289 with the result, each as its own Dependabot PR per
  `AGENTS.md` rule 1. Or, if they have been superseded, say so in the
  notes.

**Out (do not do):**
- Auto-merging Dependabot PRs: that is a repository setting for Nick
  (2060).
- The game's Dependabot PRs (game#0146).

## Implementation steps

1. Read `crates/xtask/src/sound_parity.rs` and the footprint format. Change
   the matcher for `uses:` lines and `Cargo.toml` dependency lines to
   compare the name only. Keep the test
   `giving_up_covers_all_the_chain_added` meaningful by having it check
   names.
2. Write `.github/workflows/dependabot-waiting.yml`: `schedule` plus
   `workflow_dispatch`, `permissions: issues: write, pull-requests:
   read`, and `gh pr list --author app/dependabot --json
   number,title,createdAt,statusCheckRollup`. Keep zizmor clean.
3. Update `dependency-upkeep` and `work-ticket` §1, then run `pwsh
   -File scripts/sync-agent-skills.ps1`.
4. Rebase or re-run #286–#289 on top of this change, and work each per
   `dependency-upkeep`.

## Acceptance criteria

- [ ] Changing an action's pinned SHA in `.github/workflows/*.yml` no
      longer fails `sound-give-up`.
- [ ] Changing `sha2`'s version no longer fails
      `giving_up_covers_all_the_chain_added`.
- [ ] The scheduled workflow, run by `workflow_dispatch` on the branch,
      writes the issue listing the open Dependabot PRs.
- [ ] `dependency-upkeep` names the footprint, the compile budget and
      moved error columns.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the footprint matcher with a changed SHA and a changed version.
- Workflow: one dispatch run, linked in the notes.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
