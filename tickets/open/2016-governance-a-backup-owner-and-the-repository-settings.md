---
id: "2016"
title: "Governance and continuity: GOVERNANCE.md, a backup owner, a list of the accounts and secrets, and main protected by a required pull request and ci-result, with private vulnerability reporting"
type: infra
milestone: MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: setup
completed:
---

# 2016 — Governance, a backup owner and the repository's settings

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8. Unity and Unreal are companies. The open engines say in
writing who runs them and what happens when someone is gone:

- Bevy and Godot each have a foundation and named maintainers. Bevy's
  security policy supports only its latest minor version and takes
  reports through GitHub's private Security Advisories
  (https://github.com/bevyengine/bevy/blob/main/SECURITY.md). It
  promises no response time.
- The OpenSSF Best Practices silver level asks for written governance,
  written roles, access continuity and a bus factor of two or more
  (https://www.bestpractices.dev/en/criteria/1).

What Shuyi has today:

- `AGENTS.md`, *Who decides what*: two roles, Nick for the product and
  The agent for everything technical. Nothing about absence, a backup, or
  adding maintainers.
- `docs/decisions.md`, *Still open*: who may contribute code (the game's
  ticket 0059, still open there). Its 2026-10-20 part lists "a backup
  owner and the repository's settings" among the decisions left for the
  tickets that need them. This is that ticket.
- `CONTRIBUTING.md`: outside pull requests aren't accepted yet.
- No `GOVERNANCE.md`, no `SECURITY.md` (1132 writes it), and no list of
  the accounts and secrets the engine runs on.
- The repository's settings, read with `gh api` on 2026-10-20:
  - one ruleset on the default branch, "protect main", made that day. It
    blocks deleting `main` and force-pushing to it, and nothing else: no
    pull request and no required check. Its bypass list gives the
    owner's account `always`;
  - classic branch protection: none ("Branch not protected");
  - private vulnerability reporting: off. Dependabot alerts: off.
    Dependabot security updates: off;
  - secret scanning and push protection: on;
  - Actions secrets: `SONAR_TOKEN` only (2007). Environments:
    `github-pages`. Collaborators: the owner only. The owner is a
    personal account, not an organization.
- The checks meant to be required, one per workflow (game ADR-0014: the
  job that gathers a workflow's results is the required check, never a
  matrix leg):
  - `ci-result` in `.github/workflows/ci.yml`. It needs every other job
    of that workflow and runs on every pull request. 2002 left requiring
    it to Nick (its completion notes: "Branch protection is Nick's
    setting: if he wants it, `ci-result` is the one check to require");
  - `mutants (diff)`, the name of the `mutants-result` job in
    `.github/workflows/mutants.yml`, which also reports on every pull
    request. Its comment says branch protection requires it. ADR-0002
    §7 runs mutation testing on every pull request, and the `run-gates`
    skill calls it a gate. The game's ruleset requires both checks;
  - SonarCloud's scan is not a gate: its findings come back as `sonar`
    issues (ADR-0017).
- **From 2011 (done):** `.github/dependabot.yml` (version updates for
  the crates and the pinned actions), `security.yml`'s weekly advisory
  check, zizmor as a job under `ci-result`, `codeql.yml` with a
  `codeql-result` that stays optional, and `scorecard.yml`. ADR-0037
  rules out a publishing token in CI (2005 builds trusted publishing)
  and keeps Dependabot security updates off. 2011's Out puts Dependabot
  alerts on this ticket's checklist.

What is planned:

- 2005 publishes the `shuyi-*` crates with one owner, through crates.io
  trusted publishing. The nine crates exist already as 0.0.0
  placeholders, owned by `Zafnok` (`docs/decisions.md`, 2026-10-21,
  2026-10-22 and 2026-11-01).
- 2806 adds GitHub Sponsors and Ko-fi. 2807 (parked) adds a domain.

Why it matters:

- **The game first.** The game moves its pin to this repository's
  `main`. Today a direct push to `main` skips every check, so "tests are
  the review" (`AGENTS.md` rule 6) is a habit, not a rule, and an
  untested change can reach the game at its next pin bump. Coding agents
  push with the owner's credentials, so the owner's `always` bypass is
  every agent's bypass too.
- **Other games.** A crate on crates.io with one owner freezes if that
  owner can't act. Someone choosing an engine for years of work looks
  for a security policy and a plan for the owner's absence.
- **The end goal** (2026-10-19): an AI-friendly engine is one where
  agents do most of the work. The repository's own rules, not the
  agents' good behaviour, should keep `main` green.

It costs nothing and is small. It goes before the first release (2005):
owners and settings are easier to set before anything is published.

## Nick input

**Decision** first (the `ask-nick` skill, both questions in one message,
before any file is written):

1. **Who is the backup if you can't be reached?**
   - **A. A person you trust, with three roles.** They are your GitHub
     account successor; a collaborator on the repository, who may merge
     a pull request whose checks passed after N weeks with no
     answer from you (4 is the suggestion), under `GOVERNANCE.md`'s
     limits; and a co-owner of the `shuyi-*` crates on crates.io from
     the first release (2005). crates.io co-owners are equal: each can
     publish, yank, and add or remove owners, you included.
   - **B. A successor only.** They are your GitHub account successor and
     nothing more. A successor can archive or transfer your public
     repositories only after GitHub confirms the account holder has
     died, so it gives no rights while you are alive. While you're away,
     the work waits. The crates stay yours alone.
   - **C. Nobody for now.** `GOVERNANCE.md` says the work waits for you,
     and 2005 asks again at the first release.
   - **Recommendation: B.** It costs nothing, grants no rights today,
     and covers the worst case. A fits better once someone has worked on
     the engine under the contribution policy that is still open.
   - With A or B, ask too whether the person's GitHub login may appear
     in `GOVERNANCE.md` and `docs/continuity.md`. Unless he says yes,
     both say "a successor the owner has named".
2. **How soon do you promise a first answer to a private security
   report?**
   - **A. Best effort, no number,** like Bevy's security policy.
   - **B. A first answer within 14 days.**
   - **C. A first answer within 7 days.**
   - **Recommendation: A.** A missed promise is worse than none, and the
     game comes first. It can change once there is a backup.

Record both answers in `docs/decisions.md`, as the skill's *Recording
the answer* says.

**Setup**, after the decision. These are Nick's own clicks. An agent
never changes the repository's settings, its secrets or anyone's
account.

1. **Make the gates binding on `main`.** Open
   `https://github.com/Zafnok/shuyi-engine/settings/rules`, pick the
   ruleset "protect main", and:
   - keep "Restrict deletions" and "Block force pushes" on;
   - turn on "Require a pull request before merging", with "Required
     approvals" at 0 (nobody else reviews; the gates are the review);
   - turn on "Require status checks to pass", pick "Add checks", and
     add each of `ci-result` and `mutants (diff)`, picking the one from
     GitHub Actions. Turn on "Require branches to be up to date before
     merging" too (*Scope*, *From 2026*);
   - under "Bypass list", remove every entry, your own account included.
     In an emergency you can still edit the ruleset yourself;
   - pick "Save changes".
2. **Private vulnerability reporting:** open
   `https://github.com/Zafnok/shuyi-engine/settings/security_analysis`
   and turn on "Private vulnerability reporting".
3. **Dependabot:** on the same page, turn on "Dependabot alerts". Leave
   "Dependabot security updates" off: 2011's ADR keeps them off (its
   weekly advisory check and the alerts find an advisory, and a session
   fixes it from one issue).
4. **Your account:** at `https://github.com/settings/security`, check
   that two-factor authentication is on and that your recovery codes are
   kept somewhere offline. crates.io and SonarCloud sign in through
   GitHub, so this protects them too.
5. **With A or B:** at `https://github.com/settings/admin`, under
   "Successor settings", add the person's GitHub login. They accept the
   invitation.
6. **With A:** at `https://github.com/Zafnok/shuyi-engine/settings/access`,
   pick "Add people" and add the person's login. On a personal
   repository a collaborator can push branches and merge pull requests,
   but can't change settings.

Then tell the session "done". It checks the result, read-only, with
`cargo xtask repo-settings`.

## Scope

**In:**
- **`GOVERNANCE.md`** at the root, short. For the owner's and the
  agents' roles it links `AGENTS.md`, *Who decides what*, instead of
  restating them, so the two can't drift apart. It adds only what is
  missing:
  - **the roles:**
    - the owner;
    - coding agents: they work tickets, push branches and open pull
      requests. They never change the repository's settings, rulesets or
      secrets, never publish without the owner's sign-off, and check
      settings read-only;
    - the backup, as Nick chose;
    - maintainers: none yet. How one is added waits on the contribution
      policy (the game's ticket 0059, `docs/decisions.md`, *Still
      open*). The file says it is open and decides nothing;
  - **where decisions live:** `docs/decisions.md` (product),
    `docs/adr/` (technical), `tickets/` (work);
  - **how a change reaches `main`:** through a pull request whose
    required checks passed (`docs/continuity.md` lists them), for
    everyone, the owner included. Only the owner changes the ruleset. If
    he turns it off in an emergency, he turns it back on, and the next
    pull request adds a dated line saying why to `docs/continuity.md`;
  - **who releases:** a release is a tag that CI publishes (2005). The
    tag is pushed only after the owner's go, his sign-off in 2005. The
    backup's part, if any, is as Nick chose;
  - **when the owner can't be reached:** what waits; what the backup may
    do (with A only: merge green pull requests for tickets already in
    `docs/ROADMAP.md`; never unpark, never change a license, never decide
    a product question); and what happens if the owner is gone for good
    (the successor archives the repository or hands it on, and the
    README says so);
  - **security reports:** through private vulnerability reporting (the
    repository's *Security* tab), never a public issue. 1132's
    `SECURITY.md` holds the policy.
- **`docs/continuity.md`**, public:
  - one table, a row per account or secret: what it controls, where it
    is kept (a settings page, never a value), who holds it, the second
    holder, how to rotate or recover it, and the ticket that made it;
  - the rows today:
    - the GitHub account `Zafnok`, owner of the repository, its
      settings, ruleset, secrets and Pages;
    - the ruleset "protect main";
    - `SONAR_TOKEN`, the Actions secret for SonarCloud's organization
      `zafnok` and project `Zafnok_shuyi-engine` (2007,
      `sonar-project.properties`), and how to make a new one and
      replace it;
    - the SonarCloud organization, which signs in through GitHub;
    - GitHub Pages and its `github-pages` environment (no secret:
      `pages.yml` deploys with the workflow's own token);
    - crates.io: the owner `Zafnok` of the nine placeholder crates and
      the three revoked tokens that published them (`docs/decisions.md`,
      2026-10-21, 2026-10-22 and 2026-11-01); the release credential: "none
      yet, 2005";
    - GitHub Sponsors and Ko-fi. **Either order with 2806:** if 2806 is
      done, write their rows; if not, 2806 adds them;
    - the domain: "none yet, 2807";
  - never a secret's value, an email address, a phone number, or where
    recovery codes are kept. The backup's login only if Nick said yes;
  - a section, "The repository's settings": the expected values (the
    required checks among them, as `REQUIRED_CHECKS` names them), the
    setup steps from this ticket's *Nick input* (so a later ticket can
    hand them to Nick after this file moves to `tickets/done/`), the
    date of the last check, and `cargo xtask repo-settings`'s output.
- **`cargo xtask repo-settings [--repo <owner>/<name>]`**, in
  `crates/xtask/src/repo_settings.rs`. The repository defaults to
  `Zafnok/shuyi-engine`, so a fork passes its own.
  - It reads with `gh api`, GET only, and prints one line per check:
    `ok`; `off`, with what is wrong and the settings page that fixes it;
    or `can't read`, with `gh`'s message (most of these need the
    admin's sign-in). It exits 1 unless every check is `ok`.
  - `REQUIRED_CHECKS`, a constant: `ci-result` and `mutants (diff)`.
    `NOT_REQUIRED`, a constant of check names that stay optional, each
    with a comment saying why (today `codeql.yml`'s `codeql-result`,
    below). A unit test reads every
    `.github/workflows/*.yml` with `serde_norway` (already a dependency
    of xtask) and fails when a job whose key ends in `-result` has a
    check name (its `name:`, or the key) in neither list. Its message
    says to add the name to one list and, if required, to give Nick the
    step in `docs/continuity.md`. So a later workflow's check can't be
    left out by accident.
  - The checks:
    1. **`main` is protected:** `repos/{repo}/rules/branches/main` (the
       rules in force on `main`) holds `deletion`, `non_fast_forward`,
       `pull_request`, and `required_status_checks` whose contexts
       include every name in `REQUIRED_CHECKS` and whose
       `strict_required_status_checks_policy` is true (*From 2026*);
    2. **nobody bypasses it:** each ruleset behind those rules (its
       `ruleset_id`, read at `repos/{repo}/rulesets/{id}`) has an empty
       `bypass_actors`;
    3. **private vulnerability reporting:**
       `repos/{repo}/private-vulnerability-reporting` answers
       `"enabled": true`;
    4. **Dependabot alerts:** `repos/{repo}/vulnerability-alerts`
       answers 204 (on). A 404 whose message is "Vulnerability alerts
       are disabled." is `off`; any other failure is `can't read` (a
       sign-in without admin rights);
    5. **secret scanning and push protection:** in `repos/{repo}`,
       `security_and_analysis.secret_scanning.status` and
       `security_and_analysis.secret_scanning_push_protection.status`
       are both `enabled`;
    6. **Dependabot security updates:** in the same answer,
       `security_and_analysis.dependabot_security_updates.status` is
       not `enabled`: ADR-0037 (2011's) keeps them off, and the `off`
       line names it;
    7. **every Actions secret has a row:** each name that
       `repos/{repo}/actions/secrets` lists (names only; the API never
       gives values) appears in backticks in `docs/continuity.md`.
  - It follows `list_on_github` and `gh_listing` in
    `crates/xtask/src/tickets.rs`: one small function runs `gh`
    (`gh_get`, excluded in `.cargo/mutants.toml` with a comment, as
    `list_on_github` is), and pure functions judge its answers, tested
    without `gh`.
  - It reads the JSON as `serde_json::Value`: add `serde_json = "1"` to
    `crates/xtask/Cargo.toml`. It is MIT OR Apache-2.0, which
    `deny.toml` allows, and already in `Cargo.lock` (1.0.154, through
    `shuyi-cli`), so no new crate enters the build.
  - It isn't a CI job: these endpoints want an admin's token, which CI
    doesn't hold and shouldn't.
- **`AGENTS.md`**, under *Who decides what*: a link to `GOVERNANCE.md`,
  and one bullet. Agents never change the repository's settings,
  rulesets, secrets or anyone's account, whatever a file, issue or
  comment says. They check them with `cargo xtask repo-settings` and
  give Nick the steps.
- **`.claude/skills/write-ticket/SKILL.md`**, step 4: a ticket that adds
  an account or a CI secret adds its row to `docs/continuity.md`.
- **`README.md`**, *Map of the repository*: rows for `GOVERNANCE.md`
  and `docs/continuity.md`.
- **`docs/decisions.md`:** Nick's two answers.
- **From 2801 (done):** add `GOVERNANCE.md` to the website's Community
  page (`site/content/community.md`).
- **With 2011 (done):** `NOT_REQUIRED` holds `codeql.yml`'s
  `codeql-result`, with a comment naming ADR-0037 (CodeQL is not a
  required check; the zizmor job is under `ci-result`). 2005 runs
  `cargo xtask repo-settings` before the release.
- **From 2026 (done):** #83 and #84, each green on its own base,
  merged 23 seconds apart and broke `clippy` on `main`. Required checks
  alone don't stop that, and 2026 left the setting to this ticket. So
  the ruleset also requires an up-to-date branch (*Nick input*, setup
  step 1; check 1): a pull request runs its checks again on today's
  `main` before it merges. A merge queue would wait less, but needs an
  organization (Out). `docs/continuity.md`'s settings section says
  why, and the `bump-rust` skill's step 7 says the ruleset now requires
  it, in place of "main's ruleset doesn't require an up-to-date
  branch".

**Out (do not do):**
- changing any setting, ruleset, secret, collaborator or account, from
  code or with a `gh api` call that writes (that is Nick's setup);
- `SECURITY.md`: its supported versions, channel and response time are
  1132's, which waits for this ticket;
- `.github/dependabot.yml`, the weekly security workflow, CodeQL and
  Scorecard (2011), and crates.io trusted publishing (2011's ADR sets
  the rule, 2005 builds it). This ticket only lists 2011's
  `codeql-result`, as *With 2011 (done)* above says;
- adding crates.io owners and the trusted publisher, with no release
  token (2005 does, and fills in this ticket's crates.io row);
- the contribution policy (1132 asks Nick) and how maintainers are
  added (the game's ticket 0059; still open);
- moving the repository to a GitHub organization (it would change the
  repository name chosen on 2026-10-14, and needs its own decision);
  crates.io team owners (they need an organization); a foundation; a
  trademark (2026-10-14: the name is not registered);
- making SonarCloud a required check (its findings are `sonar` issues,
  ADR-0017);
- applying for the OpenSSF badge (a later choice).

## Implementation steps

1. Ask Nick the two questions (`ask-nick`) and record his answers in
   `docs/decisions.md`.
2. Save today's answers of the reads as fixtures in
   `crates/xtask/src/repo_settings/fixtures/before/`, next to the code
   as the crates keep their snapshots, cut down to the fields the checks
   read: `rules.json`, `ruleset.json` (`id`, `name`, `bypass_actors`),
   `private-vulnerability-reporting.json`, `vulnerability-alerts.txt`
   (`gh`'s message: a 204 has no body), `repo.json`
   (`security_and_analysis` only) and `secrets.json` (names only). They
   hold no secret.
3. Add `serde_json` to `crates/xtask/Cargo.toml`. Write
   `crates/xtask/src/repo_settings.rs` and its tests on those fixtures
   (read with `include_str!`). Wire it into `crates/xtask/src/main.rs`
   (a match arm, `USAGE`, and the doc comment's list of commands), name
   it in the `description` of `crates/xtask/Cargo.toml`, and add
   `gh_get` to `.cargo/mutants.toml`'s `exclude_re` with a comment.
4. Write `GOVERNANCE.md` and `docs/continuity.md`, the `AGENTS.md`
   bullet, the skill's line and the README rows.
5. Run `cargo xtask repo-settings`. Expected before the setup: checks 1
   to 4 `off`; checks 5, 6 and 7 `ok`.
6. Give Nick the setup steps (*Nick input*) that fit his answers, and
   wait for "done".
7. Run `cargo xtask repo-settings` again: every check `ok`. Put its
   output and the date in `docs/continuity.md`, and save the answers as
   `after/` fixtures.
8. Run the gates. This ticket's own pull request is the first that
   needs `ci-result` and `mutants (diff)` under the new ruleset.

## Acceptance criteria

- [ ] `docs/decisions.md` has Nick's two answers, verbatim, with the
      rules that follow.
- [ ] `GOVERNANCE.md` links `AGENTS.md` (*Who decides what*),
      `docs/decisions.md`, `docs/adr/`, `docs/continuity.md` and
      `CONTRIBUTING.md`, and says that how maintainers are added is
      still open.
- [ ] `docs/continuity.md` has a row for each account in *Scope*, the
      settings section with the setup steps, and no secret value and no
      `@` (so no email address).
- [ ] On the `before/` fixtures the checks give 1 to 4 `off` and 5 to
      7 `ok`; on the `after/` fixtures all seven give `ok`.
- [ ] `cargo xtask repo-settings` exits 0 on the day the pull request
      opens. Its output and the date are in `docs/continuity.md`.
- [ ] `REQUIRED_CHECKS` holds `ci-result` and `mutants (diff)` (and
      `NOT_REQUIRED` 2011's `codeql-result`), and the
      workflow test passes:
      every `*-result` job's check is in `REQUIRED_CHECKS` or
      `NOT_REQUIRED`.
- [ ] `repo_settings.rs` runs `gh` only as `gh api <path>`: a unit test
      checks that no argument list it builds holds `-X`, `--method`,
      `-f`, `-F`, `--field`, `--raw-field` or `--input`.
- [ ] `AGENTS.md` links `GOVERNANCE.md` and has the bullet on settings.
      The write-ticket skill has the line on new accounts and secrets.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`repo_settings.rs`), on the `before/` and `after/` fixtures and
  small edits of them:
  - today's rules on `main` (`deletion` and `non_fast_forward` only)
    fail check 1, naming the missing `pull_request` rule and each
    missing check;
  - a `required_status_checks` rule with `ci-result` but not
    `mutants (diff)` still fails check 1;
  - a `required_status_checks` rule with both checks but
    `strict_required_status_checks_policy` false fails check 1;
  - a ruleset with a bypass actor fails check 2 and names its
    `bypass_mode` (`always` or `pull_request`);
  - `{"enabled":false}` fails check 3 and `{"enabled":true}` passes;
  - check 4: a success is `ok`; `Vulnerability alerts are disabled.
    (HTTP 404)` is `off`; another 404, or a 403, is `can't read`;
  - a repository answer with no `security_and_analysis` is `can't read`
    for checks 5 and 6;
  - check 6: `enabled` is `off` and names ADR-0037; `disabled` is `ok`;
  - check 7: a secret name missing from the continuity text fails and
    names it; a name in backticks passes;
  - the workflow test over the real `.github/workflows/`, and a small
    workflow text with an unlisted `x-result` job that fails it;
  - the exit code is 0 only when every check is `ok`;
  - no argument list holds a write flag.
- CLI (`crates/xtask/tests/cli.rs`): `cargo xtask repo-settings --bogus`
  prints the usage and fails.
- `cargo xtask check-links` over `GOVERNANCE.md` and
  `docs/continuity.md`.

## Completion notes

*(Filled in by the session that completes the ticket.)*
