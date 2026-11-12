---
id: "2057"
title: "Dependabot PRs don't sit red: the give-up footprint reads keys, and a daily list of the waiting ones"
type: infra
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-12
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

- [x] Changing an action's pinned SHA in `.github/workflows/*.yml` no
      longer fails `sound-give-up`.
- [x] Changing `sha2`'s version no longer fails
      `giving_up_covers_all_the_chain_added`.
- [x] The scheduled workflow, run by `workflow_dispatch` on the branch,
      writes the issue listing the open Dependabot PRs.
- [x] `dependency-upkeep` names the footprint, the compile budget and
      moved error columns.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the footprint matcher with a changed SHA and a changed version.
- Workflow: one dispatch run, linked in the notes.

## Completion notes

**What was done.**

- The give-up footprint reads actions and dependencies by key.
  `give_up::Pattern` leaves two parts of an edit's text open: after the
  `@` of a `uses: owner/action@` line, up to the end of the line (the pin
  and its comment); and the value of a `version = "…"`. Each open part
  ends at a fixed terminator (the line break, the quote), so a pattern
  matches one way at most and the edit's "exactly once" check stays as
  strict as before. `footprint.ron` now writes those parts empty: its
  four action lines stop at the `@`, and `sha2`'s line holds
  `version = ""`. Unit tests: `an_edit_reads_an_action_by_its_name_whatever_its_pin`
  and `an_edit_reads_a_dependency_by_its_name_whatever_its_version`, and
  `giving_up_covers_all_the_chain_added` still checks the whole
  repository. Checked by hand: with `taiki-e/install-action`'s pin in
  `mutants.yml` changed to another SHA and version, and `sha2`'s
  requirement changed to `"0.11.1"`, `cargo xtask sound-parity give-up`
  and that test pass.
- **Why keys, not `--refresh-footprint`.** A refresh is one more step a
  session must remember on every bump, and the ticket's whole problem is
  that nobody ran the steps. Keys make a bump of a pin or a version
  leave the footprint right with nothing to run, and the matcher is
  about 70 lines with no new dependency.
- `.github/workflows/dependabot-waiting.yml`: daily at 07:00 UTC and on
  `workflow_dispatch`, it lists the Dependabot PRs open over 24 hours
  with their failing jobs (`statusCheckRollup`) in one issue labelled
  `dependabot-waiting`, creating or rewriting it, and closes it when the
  list is empty. It checks nothing out, and every value read from
  GitHub goes through files and `jq`. zizmor 1.31.1: no findings.
- `dependency-upkeep` gets step 5, the three known failures beside a
  bump: the footprint, the compile budget (raise it with the count and
  the crates, or refuse the bump, as #289 was), and a moved error column
  (check the message, then move the test's position only). It also
  says the daily issue exists. `work-ticket` §1 reads the
  `dependabot-waiting` issue before choosing a ticket.
- The footprint gains an edit for the new skill step, which names
  `sound-parity`: giving up removes that step.
- `README.md`'s repository table names the new workflow;
  `docs/ROADMAP.md` marks 2057 done.

**Deviations.**

- **#286–#289 were already settled** when this started (ticket 2057 is 4
  days old; checked against `main` per `work-ticket` §0): #288 (sha2
  0.11) was merged with a hand edit of the footprint; #286 (ron 0.12.3)
  was superseded by #330, which also moved the column-pinning tests in
  `crates/assets/src/import_settings.rs`; #287 (actions) was superseded
  by #357, merged with a hand edit of the footprint; #289 (lewton 0.10)
  was closed by Nick rather than raising the compile budget, because
  `quad-snd` still builds lewton 0.9. Nothing was left to rebase. No
  Dependabot PR was open at the time.
- **The dispatch run** (acceptance: "run by `workflow_dispatch` on the
  branch"): GitHub dispatches only a workflow already on the default
  branch, so the branch carried a temporary `push` trigger instead, in
  two runs, and the PR removes it. With no Dependabot PR open, the first
  run listed every Dependabot PR (closed ones too) opened before
  2026-11-11, to exercise the issue's writing: it created #373,
  https://github.com/Zafnok/shuyi-engine/actions/runs/43131058687, which
  lists #286–#289 with exactly the red jobs the retro found. The
  second run, with the real logic, found none waiting and closed #373:
  https://github.com/Zafnok/shuyi-engine/actions/runs/43131279836.
- No ADR: the matcher is a detail of ADR-0068's footprint, and the daily
  issue is one more of ADR-0037's queues, alongside the `advisory`
  issue.

Book: none, no public API changed: the change is in `xtask`, a
workflow and two skills.

Game: nothing to adopt, the change is the engine repository's own tooling (game#0146 covers the game's Dependabot PRs).

Nick input: none.
