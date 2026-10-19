---
name: sonar-fix
description: Fix one batch of SonarQube findings filed as a GitHub issue labelled sonar. Use when pointed at a sonar(...) issue, asked to fix Sonar code smells, clear a sonar batch, or work the sonar debt queue.
---

# Fix a Sonar batch

One `sonar`-labelled GitHub issue is one PR. The issue body **is** the
ticket (goal, findings table, acceptance criteria): the one exception to
"work one ticket at a time from `tickets/open/`" (`AGENTS.md` hard rule 1,
[ADR-0017](../../../docs/adr/0017-sonar-backlog-becomes-batched-github-issues.md)).
There is no ticket file to write or move.

`work-ticket` still governs everything this skill doesn't change: scope,
the layers, nothing of the game, gates, never weakening a gate, one PR,
then stop.

## Loop

1. **Read the issue.** `gh issue view <n>` (add `--json body --jq .body` for
   the raw table). Its acceptance criteria are the checklist; copy them into
   your first message. Don't re-plan or widen the batch.

   **Parent or part?** A batch over 25 findings is split: a *parent* issue
   with a `## Parts` table and no findings, and one *part* sub-issue per
   slice. Never work a parent: if pointed at one, take its first open
   sub-issue and say which. For a part, also read the parent and its
   comments (`gh issue view <parent> --comments`), where the rule-wide
   decision and the shape of fix already used live, and look at the merged
   PRs of closed sibling parts so your fix matches theirs. List siblings with
   `gh api repos/{owner}/{repo}/issues/<parent>/sub_issues --jq '.[] | "\(.number) \(.state) \(.title)"'`.

2. **Re-check every finding against `HEAD`.** Sonar's data is as old as the
   last analysis of `main`. For each row, open the file at that line and
   confirm the finding is still there and still means what the message says.
   One that no longer reproduces is **named in the PR body**, not silently
   dropped. If more than about a third have gone, stop and run the
   `sonar-triage` skill instead: the issue is stale.

3. **Branch** from `main`: `git switch -c sonar/<short-slug>` (for example
   `sonar/s8863-static-lifetimes`, `sonar/turn-complexity`). Not
   `t<NNNN>-`: `ticket-lint` would look for a ticket file.

4. **Fix, smallest change first.** For a rule-wide batch, give every site the
   same shape of fix; inconsistency across call sites is worse than the
   finding. A cognitive-complexity finding (`rust:S3776`) is a refactor: keep
   behaviour identical, and keep the tests (and the mutation check in CI)
   proving it.

5. **Or accept it.** If a finding is wrong for this repo, add an entry to
   `tools/sonar-triage/policy.jsonc` with a `reason` citing an ADR or a
   ticket, using the verdict rules in the `sonar-triage` skill. Never reach
   for `// NOSONAR`, `#[allow]`, `#[expect]`, a coverage exclusion or a
   lowered gate.

   For a **part**, the first part to land settles the shape of the fix:
   post a one-paragraph comment on the parent saying what it was, so later
   parts copy it.

6. **Gates:** the `run-gates` skill, as for any PR. Mutation testing runs in
   CI only.

7. **Commit** as `[sonar #<n>] imperative summary`, for example
   `[sonar #312] Drop redundant 'static lifetimes`.

8. **PR:** title `[sonar #<n>] <issue title>`, body listing each acceptance
   criterion with what proves it, `Closes #<n>` (and the parent too, if this
   was its last open part), and any finding that no longer reproduced. No
   bare `@word` anywhere (`AGENTS.md` rule 11). If the fix changes the
   public API, say so, as `AGENTS.md` asks of every PR. Then stop: don't
   pick up the next batch, or a sibling part, in the same session.

## Rules that trip agents up here

- **The batch is the scope.** A rule-wide batch over 19 files is still one
  PR; a tempting cleanup nearby is a new ticket (`write-ticket`). For a part,
  the same finding in a file listed in a sibling part is the sibling's.
- **Accepting from a part accepts the rule.** A `policy.jsonc` entry is
  rule-wide unless `paths` narrows it. Say so on the parent before accepting
  from one part.
- **Don't edit the issue body.** The next triage run overwrites it. Say it in
  the PR.
- **Don't re-run the triage script** to refresh the issue mid-fix. The next
  run closes the issue once Sonar has analysed `main` after your merge.
- A Sonar finding never justifies crossing a layer (`AGENTS.md` rule 5,
  ADR-0002 §1) or putting the look into the core (graphics are a skin,
  ADR-0002 §4). If a fix would change what a game sees (a public type or
  function, or how a rule plays), it isn't a cleanup: stop and write a
  ticket (`write-ticket`). The game builds on this repository at a pinned
  revision, and a public API change breaks it at its next pin bump.
