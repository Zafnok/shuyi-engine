---
id: "2007"
title: "Coverage, SonarCloud, and the Sonar debt queue as batched GitHub issues"
type: infra
milestone: MVP
model: implementation
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-10-19
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

- [x] `sonar-project.properties` names `Zafnok_shuyi-engine` and leaves
      out `crates/app`.
- [x] CI's `coverage` job runs on a code PR, uploads to SonarCloud, and
      is green; `ci-result` needs it.
- [x] `.github/workflows/sonar-triage.yml` and `tools/sonar-triage/` are
      present, and a local dry run reads the new project without error.
- [x] The `sonar-triage` and `sonar-fix` skills name no game path, ADR
      or skill except game ADR-0071 as the origin.
- [x] The ADR exists, links game ADR-0071, and is in the index.
- [x] `AGENTS.md` rule 1 has the `sonar` issue exception.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- The CI run of this PR: the `coverage` job green, with the scan step run
  (not skipped).
- A local dry run of the triage script.
- No Rust changes, so no new Rust tests.

## Completion notes

- Copied from the game repository at `947535088eda43d7c4cf949bff546cbef1765d13`
  (`origin/main`): `sonar-project.properties`, the `coverage` job,
  `sonar-triage.yml`, `tools/sonar-triage/` and the two skills. ADR-0017
  adopts game ADR-0071 and links it.
- **Project key:** confirmed anonymously with SonarCloud's
  `api/components/search` for organization `zafnok`:
  `Zafnok_shuyi-engine`.
- **Changes from the game's copy:**
  - the project key, the ADR path, `AGENTS.md` rule 11 for mentions, and
    example paths that were the game's (`crates/core/src/turn.rs` now);
  - `sonar-project.properties` scans `examples/` too, and its test
    patterns are this repository's (`tests.rs`, `*_tests.rs`, `tests/`);
  - the coverage and Clippy commands take `--locked`, as every other
    cargo command in this repository's CI does;
  - the skills cite ADR-0002 (layers, skins) and ADR-0007 where the game
    cited its own ADRs, and `sonar-fix` adds that a fix changing the
    public API is a ticket, because the game pins this repository;
  - `run-gates` says coverage and Sonar are CI-only, not a local gate.
- **Policy:** `policy.jsonc` starts empty, as the game's is: the game has
  no rule verdicts to bring.
- **Labels:** `sonar`, `tech-debt` and `sonar:<severity>` don't exist on
  `Zafnok/shuyi-engine` yet. The script creates them on its first
  `-Apply` (the Monday run, or the workflow from the Actions tab with
  `dry_run` unticked); none were created by hand.
- **Dry run** (`pwsh ./tools/sonar-triage/sonar-triage.ps1`, before the
  first analysis): `fetched 0 / suppressed 0 / batches 0 / issues 0`,
  then `created 0 / edited 0 / linked 0 / closed 0 / unchanged 0`. The
  first findings arrive after the scan of `main` that follows the merge.
- **Not done here (out of scope):** the Sonar check as a required check,
  and a coverage threshold.
