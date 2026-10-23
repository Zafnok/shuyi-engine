# ADR-0037: Dependency upkeep: Dependabot, a weekly advisory check, workflow audits, and bot work as its own ticket (game ADR-0008's supply-chain rows, adopted)

- **Status:** Accepted
- **Date:** 2026-10-23
- **Related tickets:** 2011, 2005, 2012, 2016
- **Adopts:** the supply-chain rows of [game ADR-0008](https://github.com/Zafnok/visions-of-shuyi/blob/main/docs/adr/0008-ci-quality-gates.md)
- **Extends:** [ADR-0017](0017-sonar-backlog-becomes-batched-github-issues.md) §3

## Context

The game repository keeps its dependencies moving and its workflows
audited (game ticket 0103, game ADR-0008): Dependabot for cargo and the
actions, weekly with a 7-day cooldown; cargo-deny, machete, typos and
zizmor in `security.yml`; CodeQL for Rust and the workflows; OpenSSF
Scorecard weekly. Bevy does the same (its `.github/dependabot.yml`
groups the crates that move together).

The engine came from the game without any of it. Every action here is
pinned by commit SHA, but nothing moves the pins, and `shuyi new` copies
the templates' pins and requirements into every new game, so they age in
every game made. cargo-deny runs on code PRs and pushes only, so a
RustSec advisory published in a quiet week goes unseen. Some crates must
move together: macroquad, miniquad and the `quad-*` crates with the JS
vendored in `crates/cli/web/`, `quad-snd` with macroquad's `audio`
feature, and `deny.toml`'s two ignores that wait on a macroquad upgrade.

`AGENTS.md` hard rules 1 and 2 leave no room for a PR a bot opens, and
2005 planned to publish with a crates.io token kept as a secret. Nick
put Dependabot among the protections that come before the first release
(`docs/decisions.md`, 2026-10-19 and 2026-10-20).

## Decision

Adopt game ADR-0008's supply-chain rows, as linked above, not copied.
What is different here:

1. **Dependabot** (`.github/dependabot.yml`), both ecosystems weekly with
   `cooldown: default-days: 7` (zizmor asks for a cooldown):
   - `cargo` at `/` only, which covers the workspace. The templates'
     manifests hold `{{...}}` placeholders Dependabot can't parse.
   - Cargo groups, first match wins: `backend` (`macroquad`, `miniquad`,
     `quad-*`, `sapp-jsutils`, `gilrs*`) for every update type, then
     `cargo-minor-patch` for every other crate. `backend` takes majors
     too because its crates, the vendored JS and `deny.toml`'s ignores
     only work at matching versions; a bot that moved one alone would
     break the rest. A 0.x bump that breaks semver elsewhere comes in a
     PR of its own.
   - `github-actions` with `directories` `/` and both templates'
     `.github/workflows`, in one group (`*`, every update type), so a
     week's bumps come as one PR and the templates move with the root.
2. **The templates follow the root**, checked by `cargo xtask
   check-pins` (`crates/xtask/src/pins.rs`) in CI's `tickets` job: every
   `uses:` pinned by a 40-hex SHA with a `# v…` comment; one SHA per
   action (`owner/repo`) in the whole repository; every workflow folder
   other than the root's listed in the `github-actions` update; and each
   crate both the root's and a template's `[workspace.dependencies]` name
   at the same requirement. If Dependabot leaves a template behind, the
   check fails the root's PR until it follows.
3. **The weekly advisory check** (`.github/workflows/security.yml`):
   `cargo deny check advisories` every Monday and on demand. A failure
   opens one GitHub issue labelled `advisory`, or comments on the open
   one; that issue is the queue. PRs and pushes keep `ci.yml`'s full
   `deny` job, and machete and typos stay in `ci.yml`.
   **Dependabot security updates stay off.** The weekly check and
   Dependabot alerts (2016) find an advisory, a session fixes it from the
   one `advisory` issue, and security updates would open a second PR for
   the same advisory. Turning them on later is one setting.
4. **Audits.** zizmor runs as a job of `ci.yml` on code PRs and pushes,
   at `min-severity: medium`, over the root and both templates, with
   `advanced-security: false` so a finding fails the job in its log. It
   is in `ci-result`'s `needs`, so a required check covers it (the game
   has it in `security.yml`). A finding wrong for this repository goes
   in `.github/zizmor.yml` with why. CodeQL (`codeql.yml`) runs for
   `rust` and `actions` on PRs, pushes and weekly, with `codeql-result`,
   not a required check; JavaScript is left out (the engine's own JS is
   `crates/cli/web/gamepad.js`, the rest vendored). Scorecard
   (`scorecard.yml`) runs weekly and on pushes to `main`, report only.
   None of the three goes into the templates: a game made with `shuyi
   new` may be private, and CodeQL is free only on public repositories.
5. **Bot work is its own ticket.** A Dependabot PR, or an `advisory`
   issue, is the ticket for one PR, worked with the `dependency-upkeep`
   skill. This extends ADR-0017 §3: hard rule 1 now has two exceptions.
   A Dependabot PR is merged as every PR is, once its gates are green and
   the session's checklist comment is on it; Nick doesn't review code,
   and nothing changes about who merges. Dependabot's `rust-toolchain` PR
   (2012) is its own ticket too, finished with the `bump-rust` skill
   rather than merged as it comes. Dependabot's comment commands are
   never used, because each starts with a mention of the bot (`AGENTS.md`
   rule 11); an `ignore` entry in `dependabot.yml` does the same job.
6. **No long-lived publishing token in CI.** A release workflow publishes
   through the registry's trusted publishing (OIDC) where the registry
   has it. crates.io has it once a crate's first version is out. 2005
   builds this.

## Consequences

- The engine's crates, the actions and the templates' pins move every
  week, at least 7 days after a release; a new game starts from current
  pins.
- An advisory is seen within a week, as one issue, whether or not anyone
  pushed.
- More PRs arrive that no ticket file names; each is still one branch,
  one PR, with the skill as its checklist.
- A `backend` PR is real work (re-vendoring, the web build), which is why
  it comes as one PR.
- zizmor adds a job to every code PR. CodeQL adds two analyses that
  aren't required, so a session must read them.
- Whether one Dependabot group covers three directories in one PR is
  Dependabot's to decide; if it splits them, `check-pins` and the
  skill's step 4 bring the templates along.

## Alternatives considered

- **Renovate.** An app to install and more configuration, for what
  Dependabot does here.
- **cargo-vet.** An audit burden for a repository with one owner.
- **Dependabot security updates alone.** They move neither the actions
  nor the templates.
- **Dependabot security updates beside the weekly check.** A second PR
  for each advisory the `advisory` issue already holds.
- **zizmor in `security.yml`, as the game has it.** Not covered by
  `ci-result`, the required check.
- **A weekly run of all of `ci.yml`.** It costs much more and checks the
  same code again; only advisories change without a push.
- **Dependabot inside the templates.** The placeholders don't parse, and
  a game may be private.
