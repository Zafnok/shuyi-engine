---
id: "2007"
title: "Coverage, SonarCloud, and the Sonar debt queue as batched GitHub issues"
type: infra
milestone: MVP
model: sonnet-5
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2007 — Coverage, SonarCloud, and the Sonar debt queue as batched GitHub issues

## Context

The game repository runs SonarCloud in CI (game ticket 0104: a `coverage`
job with `cargo llvm-cov`, a Clippy report and the SonarCloud scan) and
files SonarCloud's backlog every week as batched GitHub issues labelled
`sonar` (game ticket 0126,
[game ADR-0071](https://github.com/Zafnok/visions-of-shuyi/blob/main/docs/adr/0071-sonar-backlog-becomes-batched-github-issues.md)).
Two skills go with it: `sonar-triage` decides what each batch should be,
and `sonar-fix` fixes one batch per PR. This repository has none of it,
and no coverage job.

This is a **copy** from the game, not a carve: both repositories keep
running Sonar, each on its own SonarCloud project. Nick has created the
project for this repository (organization `zafnok`, key
`Zafnok_shuyi-engine`) and added the `SONAR_TOKEN` secret to it.

Sources, in the game repository (`github.com/Zafnok/visions-of-shuyi`, on
`main`): `sonar-project.properties`, the `coverage` job of
`.github/workflows/ci.yml`, `.github/workflows/sonar-triage.yml`,
`tools/sonar-triage/` and `.claude/skills/sonar-triage/` and
`.claude/skills/sonar-fix/`.

## Nick input

None. The setup (the SonarCloud project and the `SONAR_TOKEN` secret) is
already done.

## Scope

**In:**
- `sonar-project.properties` for this repository's crates, without
  `shuyi-app` (`crates/app`), as the game leaves out its app crate.
- A `coverage` job in `.github/workflows/ci.yml`: coverage of the
  workspace without `shuyi-app`, a Clippy JSON report, and the SonarCloud
  scan when `SONAR_TOKEN` is set. Gated on `changes` and part of
  `ci-result`, like the other heavy jobs.
- `.github/workflows/sonar-triage.yml` and `tools/sonar-triage/`
  (`sonar-triage.ps1`, `policy.jsonc`, `README.md`), with this
  repository's project key and paths.
- The `sonar-triage` and `sonar-fix` skills, pointing at this
  repository's ADR, `AGENTS.md` and skills.
- An ADR adopting game ADR-0071, linking to it rather than copying it.
- `AGENTS.md`: hard rule 1 gets the exception that one `sonar` issue is
  the ticket for one PR; rule 6 names the Sonar scan.
- The README's map of the repository and the roadmap name the new parts.

**Out (do not do):**
- Making the Sonar check required in branch protection, or a quality
  gate failing CI. That is its own ticket, as in the game.
- Running `-Apply` or `-PushResolutions` locally, or creating labels by
  hand. The first `-Apply` run (the Monday schedule, or the workflow
  from the Actions tab with `dry_run` unticked) creates the `sonar`
  labels.
- Fixing any Sonar finding. Those come as `sonar` issues after the first
  analysis of `main`.
- Anything of the game in the copied files: no game path, crate name,
  ADR other than game ADR-0071, or game rule (`AGENTS.md` rule 4).

## Implementation steps

1. `sonar-project.properties`: `sonar.organization=zafnok`,
   `sonar.projectKey=Zafnok_shuyi-engine`, sources `crates` and
   `examples`, tests matched by this repository's test file names
   (`tests/` folders, `tests.rs`, `*_tests.rs`), `crates/app/**` and
   `target/**` excluded, and the `lcov.info` and `clippy.json` report
   paths.
2. `ci.yml`: a `coverage` job modelled on the game's, with the same
   SHA-pinned actions: `cargo llvm-cov --workspace --exclude shuyi-app
   --lcov --output-path lcov.info`, `cargo clippy --workspace
   --all-targets --message-format=json > clippy.json`, and
   `SonarSource/sonarqube-scan-action` with `if: env.SONAR_TOKEN != ''`.
   Add it to `ci-result`'s `needs`.
3. Copy `sonar-triage.yml`, the script, the policy and the README. Change
   the default `-ProjectKey`, the ADR path, the mention rule
   (`AGENTS.md` rule 11), and the example paths. `policy.jsonc` starts
   empty, as the game's is: it has no generic rule verdicts to bring.
4. Copy the two skills. Their references become this repository's:
   the new ADR, `AGENTS.md` rule 1 and rule 4, ADR-0002 for layers and
   skins, the `run-gates`, `work-ticket` and `write-ticket` skills, and
   the game's pin when a fix changes the public API.
5. Write the ADR with the `write-adr` skill and add it to the index.
6. Edit `AGENTS.md` rules 1 and 6, the README's map and
   `docs/ROADMAP.md`.
7. Dry-run the triage script once against the new project
   (`pwsh ./tools/sonar-triage/sonar-triage.ps1`) and put its summary
   line in the PR.

## Acceptance criteria

- [ ] `sonar-project.properties` names `Zafnok_shuyi-engine` and leaves
      out `crates/app`.
- [ ] CI's `coverage` job runs on a code PR, uploads to SonarCloud, and
      is green; `ci-result` needs it.
- [ ] `.github/workflows/sonar-triage.yml` and `tools/sonar-triage/` are
      present, and a local dry run reads the new project without error.
- [ ] The `sonar-triage` and `sonar-fix` skills name no game path, ADR
      or skill except game ADR-0071 as the origin.
- [ ] The ADR exists, links game ADR-0071, and is in the index.
- [ ] `AGENTS.md` rule 1 has the `sonar` issue exception.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- The CI run of this PR: the `coverage` job green, with the scan step run
  (not skipped).
- A local dry run of the triage script.
- No Rust changes, so no new Rust tests.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
