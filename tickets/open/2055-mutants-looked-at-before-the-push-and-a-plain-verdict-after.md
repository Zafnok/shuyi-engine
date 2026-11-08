---
id: "2055"
title: Mutants looked at before the push, and a plain verdict after
type: infra
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2055 — Mutants looked at before the push, and a plain verdict after

## Context

Mutation testing runs in CI only (`AGENTS.md`: never on Nick's
machine). In the retro of engine PRs #254–#313 (2026-11-08), it was the
biggest source of extra pushes:

- **Missed mutants:** 166 in 21 PRs. 5 of 6 first pushes failed in
  #272–#277 and in #266–#271. #274 took three rounds.
- **In book snippets:** 44 of the 166 were in `site/snippets` (#257,
  #274, #284, #285).
- **Equivalent mutants:** the same pattern cost a round in #297 and #301.
  A provided default method returning `None`, an empty `Vec` or `false`
  is equivalent to its own mutant.
- **False red:** the `mutants (diff)` aggregator reports failure when
  its shards were only cancelled by a newer push (#259, #277, #279,
  #296). The game's `codeql-result` does the same, 11 false reds there.
- **Turnaround:** CI's answer comes in about 9 minutes and the fix loop
  has a median of 16 minutes. The cost is the extra full CI run each
  round, about 51% of all job-minutes went to replaced, failed or
  cancelled runs.

The gate is good: it found real gaps, `||` → `&&` and an unreached clamp
(#278, #313). What's missing is a look before the push. `cargo mutants
--list --in-diff` only lists mutants and runs nothing.

## Nick input

None.

## Scope

**In:**
- A `run-gates` and `work-ticket` §4 step: before the first push, run
  `cargo mutants --list --in-diff <(git diff origin/main)` (it lists,
  it runs nothing) and name, for each listed mutant in new code, the
  test assertion that kills it. Add a test for any with none.
- A section in `run-gates`' *Mutation testing* on equivalent mutants:
  the patterns seen (a provided default returning `None`, `Vec::new()`,
  `false` or `()`, and a `Debug` impl), and how to exclude them in
  `.cargo/mutants.toml` with a reason, in the same commit.
- The `mutants (diff)` aggregator reports *cancelled* (neutral, not
  failure) when every non-passing shard was cancelled, and so does any
  other `*-result` aggregator in `ci.yml` (use `if: ${{ !cancelled() }}`
  or read the shards' conclusions).
- The aggregator writes each MISSED mutant (file, line, replacement) to
  the job summary and as an annotation on the diff.
- An ADR, or a line in an existing one, saying whether `site/snippets`
  stays under the diff mutants gate.

**Out (do not do):**
- Running mutants locally.
- Weakening the gate or adding `#[mutants::skip]` to pass it.
- The game's workflow (game#0150 does it).

## Implementation steps

1. Confirm that `cargo mutants --list --in-diff FILE` runs no tests and
   builds nothing (read the cargo-mutants docs for the version pinned in
   CI), and say so in `run-gates`.
2. Edit `.github/workflows/mutants.yml` (or wherever the diff shards and
   their aggregator live): handle cancelled shards, and write
   `missed.txt` lines to `$GITHUB_STEP_SUMMARY` and as `::warning
   file=…,line=…::` annotations.
3. Check every other aggregator job in `ci.yml` for the same
   cancelled-is-failure pattern and fix it.
4. Write the `run-gates` and `work-ticket` text, then run `pwsh -File
   scripts/sync-agent-skills.ps1`.
5. Decide `site/snippets` with an ADR (`write-adr`).

## Acceptance criteria

- [ ] A push that cancels a running diff-mutants run leaves the older
      run neutral, not failed (shown by a run link in the notes).
- [ ] A missed mutant shows in the job summary with file and line.
- [ ] `run-gates` has the list-before-push step and the equivalent
      mutants section.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Workflow change shown by a CI run on the branch.
- zizmor stays clean.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
