# sonar-triage

Turns SonarQube Cloud's *overall* issue backlog into batched GitHub issues.

The SonarCloud scan in `ci.yml` grades only new code, so nothing looks at what
was already on `main`. This script does: it reads the SonarCloud API, drops
findings that `policy.jsonc` accepts or defers, groups the rest into batches,
and keeps one GitHub issue per batch, labelled `sonar`, that a fix session can
pick up whole.

Copied from the game repository, which keeps its own copy (ticket 2007).
[ADR-0017](../../docs/adr/0017-sonar-backlog-becomes-batched-github-issues.md)
adopts the game's design, and
[game ADR-0071](https://github.com/Zafnok/visions-of-shuyi/blob/main/docs/adr/0071-sonar-backlog-becomes-batched-github-issues.md)
holds it with the alternatives it rejected. The skills that use it:
`sonar-triage` (refresh the queue) and `sonar-fix` (fix one issue).

## Running it

```powershell
# Dry run. Reads only; prints the batches and what it would create, edit or close.
./tools/sonar-triage/sonar-triage.ps1

# Sync GitHub issues for real.
./tools/sonar-triage/sonar-triage.ps1 -Apply
```

Reads are anonymous (the project is public), so a dry run needs no secret.
`gh` must be signed in for anything that touches GitHub (a dry run lists the
open `sonar` issues too). Runs on Windows PowerShell 5.1 and on `pwsh` 7; CI
uses the latter.

| Flag | Default | Effect |
|---|---|---|
| `-Apply` | off | Create, edit and close GitHub issues. Without it nothing is written. |
| `-PushResolutions` | off | Also mark policy-accepted findings Won't Fix in SonarCloud. Needs `SONAR_TOKEN` with issue-admin rights and asks first, because it changes shared state. |
| `-RuleBatchMinFiles` | 3 | A rule found in at least this many files becomes one rule-wide batch. |
| `-FileBatchMin` | 2 | A file left with fewer findings than this rolls into its area's long-tail batch. |
| `-MaxFindingsPerIssue` | 25 | No issue lists more findings than this; a bigger batch splits into a parent and sub-issues (below). |
| `-PolicyPath` | `policy.jsonc` beside the script | For trying a policy change without editing the real file. |
| `-ProjectKey`, `-Organization`, `-HostUrl`, `-Label` | this repo's values | |

`.github/workflows/sonar-triage.yml` runs it every Monday at 06:00 UTC with
`-Apply`, and from the Actions tab. A run from the Actions tab is a **dry run**
unless `dry_run` is unticked.

## How findings are batched

1. A rule found in **`-RuleBatchMinFiles` or more files** becomes one
   rule-wide batch. One rule is one decision, so it is one PR.
2. Everything left groups **per file**.
3. A file left with fewer than `-FileBatchMin` findings rolls into a
   **long-tail** batch for its top-level folder (`crates`, `examples`), so
   the queue doesn't fill with one-finding issues.
4. A batch over **`-MaxFindingsPerIssue`** findings splits. Its own issue
   becomes a **parent** holding the rule-wide decision and a table of parts;
   each part, at most the cap, is its own issue, linked as a GitHub
   **sub-issue** of the parent. Parts follow the directory tree (crate, then
   folder, then file), small neighbours are packed back together in path
   order, and a single file over the cap is cut by line, so fixing one part
   leaves the others alone. A part's marker key is
   `<parent key>@<first path>[#<chunk>]`.

Each issue body opens with `<!-- sonar-triage:v1 key=... -->`. That marker is
the only state the script keeps: it re-reads open `sonar` issues, matches
markers, and creates, edits or closes to match. Re-running is idempotent;
labels added by hand stay, and a title edited by hand is put back. A settled
`-Apply` reports `created 0 / edited 0 / linked 0 / closed 0`.

A `@word` in a Sonar message is put in backticks, so an issue never mentions a
GitHub user (`AGENTS.md` rule 11).

## Saying a finding is wrong

Edit `policy.jsonc`; don't change the code to dodge it and don't add
`// NOSONAR` or `#[allow]`:

```jsonc
{
  "rule": "rust:S3776",
  "verdict": "accept",
  "paths": ["crates/core/src/turn.rs"],
  "reason": "Why this rule is wrong *here*, citing an ADR or a ticket."
}
```

`verdict` is `fix` (the default), `accept` or `defer`. `accept` and `defer`
**require** a `reason`; the script refuses to run otherwise. Suppressed counts
are printed by rule on every run, so nothing disappears silently.

## Known limits

- `/api/rules/show` returns a rule's name but an empty description on this
  plan, so issue bodies link to the rule's SonarCloud page rather than quoting
  it.
- Sonar issue keys change on re-analysis, so batches are keyed by rule or
  path, and a fix session re-checks every finding against `HEAD`.
- The API refuses paging past 10,000 issues; the script stops there.
