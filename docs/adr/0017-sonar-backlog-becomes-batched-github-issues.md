# ADR-0017: SonarCloud in CI, and its backlog as batched GitHub issues (game ADR-0071, adopted)

- **Status:** Accepted
- **Date:** 2026-10-19
- **Related tickets:** 2007
- **Adopts:** [game ADR-0071](https://github.com/Zafnok/visions-of-shuyi/blob/main/docs/adr/0071-sonar-backlog-becomes-batched-github-issues.md)

## Context

The game repository scans its code with SonarCloud in CI (game ticket
0104) and files SonarCloud's overall backlog every week as batched GitHub
issues labelled `sonar` (game ticket 0126, game ADR-0071). Its quality
gate grades only new code, so without the weekly job nothing looks at
what was already on `main`. Two skills go with it: `sonar-triage` decides
what each batch should be, `sonar-fix` fixes one batch per PR.

The engine's code came from the game (game ticket 1131) without any of
this, and with no coverage job. Nick wants the same setup here as a
copy, so both repositories keep running Sonar, each on its own
SonarCloud project. He has created this repository's project
(organization `zafnok`, key `Zafnok_shuyi-engine`; public, so reads are
anonymous) and its `SONAR_TOKEN` secret.

## Decision

Adopt game ADR-0071 as it stands, with its batching, size cap, marker
state, policy file and alternatives. This ADR records only what is
different here; the design and the alternatives it rejected are in game
ADR-0071, linked above, not copied.

1. **Coverage and the scan.** `ci.yml`'s `coverage` job runs `cargo
   llvm-cov --workspace --exclude shuyi-app`, writes a Clippy JSON report,
   and runs `SonarSource/sonarqube-scan-action` when `SONAR_TOKEN` is set,
   with the game's SHA-pinned actions. It is gated on `changes` and is
   part of `ci-result`, like the other code jobs (game ADR-0014).
   `sonar-project.properties` scans `crates/` and `examples/`, without
   `crates/app` (`shuyi-app`): the platform backend is the part that
   touches the window, files, audio and the clock (ADR-0002, ADR-0008),
   which a test on a runner doesn't drive. The game leaves out its app
   crate the same way. Test files are the `tests/` folders,
   `tests.rs` and `*_tests.rs`, this repository's names for them.
2. **The queue.** `.github/workflows/sonar-triage.yml` and
   `tools/sonar-triage/` are the game's, with this project's key. The
   first `-Apply` run creates the `sonar`, `tech-debt` and `sonar:<severity>`
   labels.
3. **A `sonar` issue is its own ticket.** One `sonar`-labelled GitHub
   issue is the ticket for one PR, on a `sonar/<slug>` branch, which
   `cargo xtask ticket-lint` ignores. This is the one exception to
   `AGENTS.md` hard rule 1. The `sonar-fix` skill is its checklist.
4. **Policy.** `tools/sonar-triage/policy.jsonc` starts empty. The game's
   was empty when copied, so there were no generic verdicts to bring. A
   verdict there cites this repository's ADRs, not the game's.
5. **The copies drift apart on purpose.** A change to the script in one
   repository isn't made in the other by default; a fix worth having in
   both is a ticket in each.

## Consequences

- Sonar findings arrive as GitHub issues, the one work surface outside
  `tickets/`.
- Coverage is measured for the engine's crates. No threshold gates it
  yet, and the scan isn't a required check; either is its own ticket.
- The coverage job builds the workspace once more per code PR.
- A fix that changes the public API isn't a cleanup: the game builds on
  this repository at a pinned revision, so it is a ticket of its own
  (`sonar-fix`).
- The script exists twice, in two repositories, and can drift.

## Alternatives considered

- **Share one SonarCloud project with the game.** Findings would mix two
  codebases with different licenses and different owners of the fix.
- **A shared reusable workflow or one copy of the script.** The game is
  proprietary and depends on the engine, never the other way around, so
  the engine can't use the game's files; the engine must stand alone for
  its users. Moving the one copy into the engine would make the game
  pull its CI tooling from a pinned engine revision for ~800 lines.
- **Coverage without SonarCloud.** Nobody would look at the numbers, and
  the game's queue shows the findings are worth fixing.
