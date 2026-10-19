---
name: sonar-triage
description: Refresh the SonarQube debt queue. Use when asked to triage Sonar findings, file or update sonar GitHub issues, run sonar-triage, or decide whether a Sonar finding is wrong for this repo.
---

# Sonar triage

Turns SonarCloud's overall backlog into batched GitHub issues labelled `sonar`.
Design: [ADR-0017](../../../docs/adr/0017-sonar-backlog-becomes-batched-github-issues.md),
which adopts game ADR-0071.
Tool reference: `tools/sonar-triage/README.md`. This skill is the judgement
part; the script is the mechanical part. A scheduled job already runs it every
Monday, so a manual run is for a new verdict or a check after a fix lands.

## Loop

1. Dry run first, always:
   `pwsh ./tools/sonar-triage/sonar-triage.ps1` (or `powershell` on Windows).
   It reads SonarCloud anonymously and writes nothing.
2. Read the summary line (`fetched N / suppressed N / batches N / issues N`)
   and the create/edit/link/close list under it. Parts of a split batch are
   indented under their parent. Four things deserve a pause:
   - **A batch you don't recognise.** A new rule, or one that has spread.
     Decide its verdict (below) before filing it.
   - **An `edit` on a batch you expected unchanged.** Usually a file moved or
     Sonar re-analysed; harmless, but check it isn't a fix PR half-landed.
   - **A `close` nobody earned.** A batch closes when Sonar reports nothing in
     it. If no one fixed it, the analysis may have failed (the coverage job in
     `ci.yml`) or `sonar-project.properties` may have changed.
   - **Parts reshuffling.** A batch over 25 findings splits into a parent and
     sub-issues along the directory tree. Closing one part should leave the
     others `unchanged`; a run that closes some parts and creates others with
     the same files means the split is unstable, and that is a script bug.
3. Settle any new verdict in `tools/sonar-triage/policy.jsonc` (a PR, like any
   change), then re-run the dry run.
4. Sync: `./tools/sonar-triage/sonar-triage.ps1 -Apply`, or the **Sonar
   triage** workflow from the Actions tab with `dry_run` unticked.
5. Run `-Apply` once more. It must report
   `created 0 / edited 0 / linked 0 / closed 0`. If not, the renderer or the
   split is non-deterministic: a bug to fix, not to re-run past.

## Deciding a verdict

The default is `fix`. A finding is `accept` only if one of these is true:

- Fixing it would **weaken a gate** (coverage, mutation, Clippy pedantic).
- Fixing it would **contradict an accepted ADR** (the layers and graphics
  as a skin, ADR-0002; formats are plugins, ADR-0007; and so on).
- Fixing it would **break a documented repo pattern** whose reason is
  written down. Check the written reason against the code first: a pattern
  can outlive its purpose.

`defer` is for a real finding blocked on a ticket that hasn't landed. Name
that ticket in the reason.

Not reasons to `accept`:

- "The suggestion is ugly." A green dashboard wins.
- "It's only in tests." Test code is in scope; batching keeps it apart.
- "It would be a big diff." That's what rule-wide batches and parts are for.
- "I disagree with the rule in general." Then the rule leaves the Sonar
  quality profile, which is an ADR, not a policy entry.

Every `accept` needs a `reason`, and one that appeals to a repo-wide design
choice cites an ADR or a ticket. The script refuses to run on a suppression
without one.

## Keeping SonarCloud honest

`-PushResolutions` marks accepted findings Won't Fix in SonarCloud with the
policy reason as the comment, so the dashboard matches the repo. It writes to
shared external state, needs `SONAR_TOKEN` with issue-admin rights, and asks
before acting. Run it after the policy settles, never from CI.

## Don't

- File findings by hand. If the script wouldn't file it, neither should you,
  and that includes splitting a big issue by hand: lower
  `-MaxFindingsPerIssue` in a PR instead.
- Edit an issue body by hand: the next `-Apply` overwrites it. Change the
  policy or the renderer.
- Add `// NOSONAR` or `#[allow]` to make a batch go away. The reason belongs
  in `policy.jsonc`.
- Make the Sonar check required as part of a triage run. That is its own
  ticket.
