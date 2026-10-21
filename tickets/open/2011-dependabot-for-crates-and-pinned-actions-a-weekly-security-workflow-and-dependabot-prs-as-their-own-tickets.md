---
id: "2011"
title: "Dependabot for the crates and the pinned actions (the templates' too), a weekly security workflow with zizmor, CodeQL and Scorecard, and an ADR that makes a Dependabot PR its own ticket and keeps publishing tokens out of CI"
type: infra
milestone: MVP
model: sonnet-5
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2011 — Dependabot for crates and pinned actions, a weekly security workflow, and Dependabot PRs as their own tickets

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8. Bevy keeps its dependencies moving with Dependabot:
cargo and actions, weekly, a 7-day cooldown, and families that must
move together (`wgpu` with `naga`) grouped
(https://github.com/bevyengine/bevy/blob/main/.github/dependabot.yml).
It also runs CodeQL and zizmor on every PR.

The engine's first user already does all of this. The game's ticket
0103 and
[game ADR-0008](https://github.com/Zafnok/visions-of-shuyi/blob/main/docs/adr/0008-ci-quality-gates.md)
gave it:

- `.github/dependabot.yml`: cargo and actions, weekly, a 7-day
  cooldown, minor and patch updates grouped;
- `.github/workflows/security.yml`: cargo-deny, machete, typos, and
  zizmor at medium, on PRs, pushes to `main` and every Monday;
- `.github/workflows/codeql.yml`: rust and actions, with a
  `codeql-result` job;
- `.github/workflows/scorecard.yml`: weekly, report only.

Shuyi today (checked 2026-10-21):

- Every `uses:` in `.github/workflows/` (`ci.yml`, `changes.yml`,
  `mutants.yml`, `pages.yml`, `sonar-triage.yml`) and in
  `templates/blank/.github/workflows/ci.yml` and
  `templates/grid/.github/workflows/ci.yml` is pinned by commit SHA,
  with the version in a comment. Permissions are read-only by default,
  and every checkout sets `persist-credentials: false`. Nothing moves
  the pins. `shuyi new` copies the templates' pins (`actions/checkout`
  v7.0.1, `Swatinem/rust-cache` v2.9.2) into every new game, so they
  get older in every game made.
- `ci.yml`'s `deny` job runs cargo-deny on code PRs and on pushes to
  `main` (`changes.yml` counts any event that isn't a PR as code). No
  workflow runs it on a schedule. A new RustSec advisory goes unseen in
  a week with no push.
- `ci.yml`'s `ci-result` needs every other job of `ci.yml`. 2016 makes
  it required, with `mutants (diff)` from `mutants.yml`.
- The repository's settings: secret scanning and push protection are
  on; Dependabot alerts and Dependabot security updates are off.
- `deny.toml` ignores RUSTSEC-2025-0035 and RUSTSEC-2026-0192, each
  with "revisit on macroquad upgrade". Nothing says when that upgrade
  arrives.
- Some versions must move together:
  - `crates/app/Cargo.toml` pins `quad-snd = "0.2.8"` as "the same
    version macroquad's `audio` feature uses";
  - `crates/cli/web/` vendors upstream JS that must match the crates:
    `mq_js_bundle.js` (macroquad 0.4.16), `quad-storage.js` (0.1.3) and
    `sapp_jsutils.js` (0.1.7). `crates/cli/web/README.md` says to
    re-vendor them when macroquad or quad-storage moves, and
    `THIRD_PARTY_ASSETS.md` names those versions and the controller
    database in `gilrs` 0.11.2.

  A bot that bumps one of these alone breaks the others.
- The templates' `Cargo.toml` files repeat the root's requirements for
  `macroquad`, `serde`, `ron` and `include_dir`, next to `{{...}}`
  placeholders that Dependabot can't parse. When a requirement moves at
  the root, a new game still gets the old one.
- `check_pr_branch` in `crates/xtask/src/tickets.rs` already ignores
  `dependabot/...` branches (test `pr_branch_ignores_non_ticket_branches`).
  It came from the game, and nothing here uses it yet.
- `AGENTS.md` hard rules 1 and 2 leave no room for a PR that a bot
  opens. Work is a ticket, and the one exception is a `sonar` issue
  ([ADR-0017](../../docs/adr/0017-sonar-backlog-becomes-batched-github-issues.md) §3).
- 2005 planned its release workflow "with a crates.io token as a
  secret". This ticket's ADR rules that out, and 2005 moves to trusted
  publishing and waits for this ticket.

Why it matters:

- **The game first.** The game builds on the engine at a pinned
  revision, and its own repository already runs these checks. A
  dependency fix that the engine misses reaches the game at its next
  pin bump. Nick asked for the dependencies' upkeep to be watched
  ([`docs/decisions.md`](../../docs/decisions.md), 2026-10-19: "lets
  check the other deps make sure they have good maintenance").
- **Other games** start with the templates' pins and requirements.
- **The first release** (2005) is where the engine's crates reach other
  people. Nick put Dependabot among the protections that come before it
  (2026-10-20).

## Nick input

None. Dependabot alerts are on 2016's checklist (its step 3), which
Nick runs. Dependabot security updates stay off (the ADR says why), as
2016 leaves them. 2016 requires `ci-result` (and `mutants (diff)`), so
zizmor runs as a job of `ci.yml` that `ci-result` needs. Trusted publishing is set
up on crates.io in 2005's sign-off.

## Scope

**In:**

- **`.github/dependabot.yml`:**
  - `cargo` at `/` only, which covers the workspace (`crates/*`,
    `examples/*`). The templates' manifests hold placeholders.
  - `github-actions` with `directories: ["/",
    "/templates/blank/.github/workflows",
    "/templates/grid/.github/workflows"]`.
  - Both ecosystems run weekly with `cooldown: default-days: 7`. zizmor
    asks for the cooldown (game ticket 0103's completion notes).
  - Cargo groups, in this order (Dependabot puts a crate in the first
    group that matches it):
    1. `backend`: patterns `macroquad`, `miniquad`, `quad-*`,
       `sapp-jsutils` and `gilrs*`, for every update type. These crates
       move together, with the vendored files and `deny.toml`'s ignores.
    2. `cargo-minor-patch`: update types `minor` and `patch`, for every
       other crate. Dependabot opens a 0.x bump that breaks semver on
       its own, outside the group: in the game, png 0.17 → 0.18 came as
       its own PR (game PR #158).
  - One `actions` group, pattern `*`, every update type. A week's
    bumps then come as one PR across the three directories, so the
    templates move with the root.
  - Check each key against GitHub's options reference
    (`docs.github.com/en/code-security/dependabot/working-with-dependabot/dependabot-options-reference`)
    before writing it. If one group can't cover the three directories
    in one PR, `check-pins` rule 2 fails the root's PR until the
    templates follow, and the skill's step 4 handles it.
- **`cargo xtask check-pins`**, in `crates/xtask/src/pins.rs`. It walks
  the tree as `links.rs` does, with the same skipped folders: make
  `links.rs`'s `SKIPPED_DIRS`, `SKIPPED_PATHS` (which skips
  `.claude/worktrees`, other checkouts) and `display_path` `pub(crate)`
  and use them, rather than copying them. It fails, naming the file,
  and the line where there is one, when:
  1. a `uses:` in any workflow file (a `*.yml` or `*.yaml` in a
     `.github/workflows` folder, at the root or under any folder) isn't
     `owner/repo[/path]@` a 40-hex commit SHA followed by a `# v…`
     comment. A `uses:` line is one whose text, after spaces and an
     optional `- `, starts with `uses:`, so comments and scripts that
     mention it don't count. A local `./…` workflow is skipped;
  2. one action (`owner/repo`, whatever the path after it, so
     `github/codeql-action/init`, `/analyze` and `/upload-sarif` count
     as one) has more than one SHA anywhere in the repository;
  3. a `.github/workflows` folder other than the root's isn't in the
     `github-actions` update's `directories` in `.github/dependabot.yml`
     (parsed with `serde_norway`, already a dependency of xtask). Other
     updates in the file (2012's `rust-toolchain`) are ignored. When
     2501 or 2005 moves the templates, this fails until Dependabot
     follows;
  4. a crate that the root's `[workspace.dependencies]` and the
     `[workspace.dependencies]` of a `Cargo.toml` under `templates/`
     both name has a different version requirement in each. Any
     `Cargo.toml` under `templates/` with that table counts, so 2501's
     parts are found too. Only the requirement is compared, not
     features. `shuyi-*` entries (placeholders) are skipped. Both files
     keep each entry on one line. The check reads them line by line and
     names a multi-line entry as an error, so no TOML crate is needed.

  It runs in CI's `tickets` job, after `check-links`, and in the
  `run-gates` skill's *The repository's own checks*. `main.rs`'s doc
  comment, `USAGE` and match arm name it, as `check-no-std` has them,
  and `crates/xtask/Cargo.toml`'s `description` lists it.
- **A `zizmor` job in `ci.yml`**, so `ci-result` (a required check)
  covers it. The game runs zizmor in `security.yml`; here it
  sits beside the other code jobs:
  - `needs: changes`, and runs when `needs.changes.outputs.code` is
    true. Add it to `ci-result`'s `needs`;
  - `zizmorcore/zizmor-action`, pinned by SHA, `min-severity: medium`,
    over `.`, `templates/blank` and `templates/grid`.
    `advanced-security: false`, so findings fail the job and show in
    its log, not only on the Security tab. The job then needs only
    `contents: read`. Check the inputs' names in the action's README at
    the pinned version.
- **`.github/workflows/security.yml`**, the weekly advisory check:
  - triggers: `schedule` (Monday, `0 6 * * 1` as the game has it) and
    `workflow_dispatch`, nothing else. `permissions: contents: read` at
    the top;
  - one job, `advisories`: `EmbarkStudios/cargo-deny-action` (the SHA
    `ci.yml` uses) with `command: check advisories`;
  - when it fails, a last step (`if: failure()`, and `issues: write` on
    this job only) opens one GitHub issue labelled `advisory`, or
    comments on the open one, with the run's URL. It creates the label
    if it's missing. The URL goes in through `env:`, never interpolated
    into the script (zizmor's template-injection audit).

  PRs and pushes keep `ci.yml`'s full `deny` job. machete and typos
  stay in `ci.yml` only.
- **`.github/workflows/codeql.yml`**, the game's: languages `rust` and
  `actions`, `build-mode: none`, PRs, pushes to `main` and Monday
  `0 7 * * 1`, gated on `changes`, with `codeql-result`.
  `security-events: write` on the analyze job only. JavaScript is left
  out: the engine's own JS is `crates/cli/web/gamepad.js` (about 100
  lines over the Gamepad API, with no page input), and the rest is
  vendored upstream files whose findings this repository wouldn't fix
  here. `codeql-result` is not a required check (2016 lists it in
  `NOT_REQUIRED`): a session reads it and the code scanning alerts with
  every other check before a merge.
- **`.github/workflows/scorecard.yml`**, the game's:
  `ossf/scorecard-action`, pushes to `main` and Monday `0 8 * * 1`,
  `publish_results: true`, the SARIF uploaded. Report only.
- **Fix what zizmor and CodeQL report** at medium or above in every
  workflow, the templates' included. For example, zizmor's
  cache-poisoning audit may flag `Swatinem/rust-cache` in `pages.yml`,
  which deploys. A finding that is wrong for this repository goes in
  `.github/zizmor.yml` with a comment saying why, and the PR names it.
  Never lower `min-severity` to pass.
- **An ADR** (the `write-adr` skill, the next free number): "Dependency
  upkeep: Dependabot, a weekly advisory check, workflow audits, and bot
  work as its own ticket". It adopts game ADR-0008's supply-chain rows
  as ADR-0017 adopted game ADR-0071, by linking, not copying. It
  records what is different here:
  1. Dependabot's ecosystems, directories and groups as above, and why
     `backend` is one group whatever the update type.
  2. The templates follow the root: one SHA per action across the
     repository, and the templates' requirements equal to the root's
     (`check-pins`).
  3. The weekly advisory check, and its `advisory` issue as the queue.
     PRs and pushes keep `ci.yml`'s full `deny` job. Dependabot
     security updates stay off: the weekly check and Dependabot alerts
     (2016) find an advisory, a session fixes it from the one `advisory`
     issue, and security updates would open a second PR for the same
     advisory. Turning them on later is one setting.
  4. zizmor in `ci.yml` on code PRs and pushes, as a job `ci-result`
     needs, so a required check covers it (the game has it in
     `security.yml`). CodeQL on PRs, pushes and weekly, with
     `codeql-result`, not required. Scorecard weekly, report only. None
     of the three goes into the templates. A game made with `shuyi new`
     may be private, and CodeQL is free only on public repositories.
  5. **Bot work is its own ticket.** A Dependabot PR, or an `advisory`
     issue, is the ticket for one PR, worked with the
     `dependency-upkeep` skill. This extends ADR-0017 §3, so hard rule 1
     now has two exceptions. A Dependabot PR is merged the way every PR
     is today, once its gates are green and the session's checklist
     comment is on it. Nick doesn't review code, and this changes
     nothing about who merges. Dependabot's `rust-toolchain` PR (2012)
     is its own ticket too. It is not merged as it comes: a session
     finishes it with the `bump-rust` skill (2012), which moves
     `rust-version` and fixes the new lints on the same branch.
     Dependabot's comment commands are never used, because each starts
     with a mention of the bot (`AGENTS.md` rule 11). An `ignore` entry
     in `dependabot.yml` does the same job.
  6. **No long-lived publishing token in CI.** A release workflow
     publishes through the registry's trusted publishing (OIDC) where
     the registry has it. crates.io has it, once a crate's first
     version is out. 2005 builds this.
  7. Alternatives: Renovate (an app to install and more configuration);
     cargo-vet (an audit burden for a repository with one owner);
     Dependabot security updates alone (they move neither the actions
     nor the templates); Dependabot security updates beside the weekly
     check (a second PR for each advisory the `advisory` issue already
     holds); zizmor in `security.yml` as the game has it (not covered
     by `ci-result`, a required check); a weekly run of all of
     `ci.yml` (it costs much more and checks the same code again);
     Dependabot inside the templates (the placeholders, and private
     games).

  Add its row to `docs/adr/README.md`. ADR-0017's row there says
  "extended by ADR-NNNN".
- **The `dependency-upkeep` skill**
  (`.claude/skills/dependency-upkeep/SKILL.md`), shaped like
  `sonar-fix`: no ticket file to write or move, and `work-ticket`
  governs everything the skill doesn't change.
  - **A Dependabot PR:**
    1. Read what moves (`gh pr view <n> --json files,body`): only
       `Cargo.lock`, or a requirement in a `Cargo.toml`, or action pins.
       A PR that moves a `rust-toolchain.toml` (2012's update) is
       worked with the `bump-rust` skill instead.
    2. **`backend` group:**
       - re-vendor the JS as `crates/cli/web/README.md` says, its local
         patch included;
       - search the repository for each old version string and update
         every place that names it (`THIRD_PARTY_ASSETS.md`, the README,
         any table in code);
       - keep `quad-snd` equal to macroquad's;
       - run `cargo deny check advisories`, and remove an ignore that no
         longer matches anything;
       - build Hello, Grid for the web and open it.
    3. **A requirement change:**
       - make the templates' requirement follow (`check-pins` fails
         until it does);
       - build the game against the branch (`run-gates`, *The game at
         the next pin*);
       - if the public API changes (the engine's API shows a type of
         the crate that moved), it is step 6: the game must follow
         (`AGENTS.md`, *Working with the game repository*), so it is a
         ticket of its own.
    4. **Actions:** if Dependabot left a template's pin behind, push the
       bump onto its branch.
    5. Read every check on the PR. When all are green, leave one
       comment that lists what was checked.
    6. **An upgrade that needs real work:** write a ticket
       (`write-ticket`), close the PR with a comment that names it, and
       add an `ignore` entry for that version to `dependabot.yml` if
       Dependabot opens it again.
  - **An `advisory` issue:**
    1. Read the run it links. For each advisory, if a fixed version
       exists, move to it.
    2. If none exists, add an `ignore` to `deny.toml` with a reason
       that ends "revisit on `<crate>` upgrade", the shape of the two
       there today.
    3. Work on an `advisory/<id>` branch (the ticket lint ignores it).
       The PR body says `Closes #<n>`.
    4. Replacing an unmaintained crate is a ticket of its own, never
       this PR.
- **Pointers:**
  - `AGENTS.md`: hard rule 1 names both exceptions and the skill;
    rule 6 adds that CI audits the workflows with zizmor and the code
    with CodeQL;
  - `work-ticket` §1: one line pointing at the skill;
  - `sonar-fix`: "the one exception" becomes "an exception";
  - the comment above `deny.toml`'s ignores points at the skill;
  - `crates/cli/web/README.md`: the vendoring notes say that
    Dependabot's `backend` group is when they apply;
  - the README's *Map of the repository*: a row for
    `.github/dependabot.yml` and the security workflows.
- **Either order:**
  - with 2012 (the pinned Rust): if 2012 landed first,
    `.github/dependabot.yml` already exists with its `rust-toolchain`
    update. Add this ticket's updates beside it, never replace the
    file. Put `check-pins` before its `check-toolchain` in the
    `tickets` job and in `run-gates`. In the `bump-rust` skill, the
    line that says to write a ticket first (until this ADR exists)
    points at this ADR instead. If not, 2012 adds its update and check
    beside these;
  - with 2019: if `tools/web-check` exists, add a `pip` entry for
    `/tools/web-check` (weekly, the same cooldown, its updates in one
    group). If not, 2019 adds it;
  - with 2020: if `fuzz/Cargo.toml` exists, `/fuzz` joins the cargo
    directories. If not, 2020 adds it;
  - with 2010 (the public API diff): if it is done, the skill's step 3
    says that its `api` job fails a Dependabot PR that breaks the API,
    and that such an upgrade goes to step 6. If not, 2010 adds that
    line;
  - with 2014 (the game built against each PR): if it is done, the
    skill's step 3 reads the `game` job's summary instead of building
    the game locally, and keeps the local build for fixing a failure.
    If not, 2014 adds that;
  - with 2016 (the repository's settings): if it is done, put
    `codeql-result` in `repo_settings.rs`'s `NOT_REQUIRED` with a
    comment naming this ADR (its workflow test fails until it is
    there). If not, 2016 lists it.

**Out (do not do):**

- Repository settings: Dependabot alerts, private vulnerability
  reporting and required checks are on 2016's checklist, which Nick
  runs. Secret scanning and push protection are already on.
- Turning on Dependabot security updates (the ADR keeps them off), and
  making `codeql-result` or Scorecard a required check.
- The release workflow and trusted publishing itself: 2005 (the ADR
  sets the rule).
- Attestations and checksums of released game packages: the release
  from a tag (3050).
- A `dependabot.yml`, CodeQL or Scorecard in `templates/`. Games made
  with `shuyi new` get fresh pins from here, nothing more.
- The Rust toolchain: the pins, `rust-version` in the templates, the
  `rust-toolchain` update in `.github/dependabot.yml`, and the
  `bump-rust` skill (2012).
- Dependabot for 2513's dev container: its `image` is a tag that moves
  on its own (`ubuntu-24.04`) and it lists no Features, so there is
  nothing to pin or move.
- machete and typos in `security.yml` (`ci.yml` runs them), and a
  scheduled run of all of `ci.yml`.
- Renovate, cargo-vet, cargo-audit (cargo-deny covers advisories),
  cargo-auditable and SBOMs.
- Auto-merge, or any change to who merges.
- Replacing an unmaintained dependency. The in-house work is parked
  (2402–2403, 2210–2214, 3030–3034).

## Implementation steps

1. Read the game's files with `gh api
   repos/Zafnok/visions-of-shuyi/contents/<path> --jq .content | base64
   -d`: `.github/dependabot.yml`, `.github/workflows/security.yml`,
   `codeql.yml`, `scorecard.yml`. Also read game ticket 0103's
   completion notes (`tickets/done/0103-security-and-supply-chain-gates.md`):
   the cooldown, and the SHA of an annotated tag versus its commit.
2. Write `.github/dependabot.yml`, after checking each key against the
   options reference (or add to 2012's, if it is there).
3. Make `links.rs`'s skipped folders and `display_path` `pub(crate)`.
   Write `crates/xtask/src/pins.rs` with its tests, register it in
   `main.rs` and the crate's `description`, and add the step to CI's
   `tickets` job and the line to `run-gates`. Run it. It must pass on
   the repository as it is.
4. Add the `zizmor` job to `ci.yml` and to `ci-result`'s `needs`. Write
   `security.yml`, `codeql.yml` and `scorecard.yml`. Pin every new
   action by the commit SHA its tag points to (`git ls-remote --tags`;
   the game's pins may be older), with the version in a comment, as
   `ci.yml` does. An action `ci.yml` already uses keeps `ci.yml`'s SHA
   (`check-pins` rule 2).
5. Run zizmor locally offline (`cargo install --locked zizmor`, then
   `zizmor --offline .github templates/blank templates/grid`; check the
   arguments in `zizmor --help`), and fix what it reports. Its online
   audits run in CI. If zizmor doesn't build on the GNU toolchain,
   rely on CI's job and say so in the PR.
6. Write the ADR, the skill and the pointers.
7. Push and read CI. To test the issue step, push a temporary commit
   that adds a `push` trigger for this branch to `security.yml` and
   removes one ignore from `deny.toml`:
   - the run fails and opens one `advisory` issue;
   - a second push comments on that issue rather than opening another;
   - revert the commit, close the issue as a test, and name both runs
     in the PR.
8. After merge, check that GitHub shows the three action directories
   and the cargo directory under *Insights → Dependency graph →
   Dependabot*, and that CodeQL and Scorecard ran on `main`. Note it on
   the PR.

Size: if the diff runs well past 600 changed lines, move `codeql.yml`
and `scorecard.yml` (with the fixes CodeQL asks for and the 2016 clause)
to a follow-up ticket, and keep Dependabot, `check-pins`, the zizmor
job, `security.yml`, the ADR and the skill here.

## Acceptance criteria

- [ ] `.github/dependabot.yml` has cargo at `/` and github-actions at
      `/` and both templates' workflow folders. Both run weekly with a
      7-day cooldown, `backend` comes before `cargo-minor-patch`, and
      the actions have one group.
- [ ] `cargo xtask check-pins` passes on the repository and runs in
      CI's `tickets` job and in `run-gates`.
- [ ] Each of its four failures is shown by a unit test that asserts
      the message, with the file named (and the line, for rules 1, 2
      and 4): an unpinned `uses:`, two SHAs for one action, a template
      workflow folder missing from `dependabot.yml`, and a template's
      `ron` requirement that differs from the root's.
- [ ] CI's `zizmor` job is in `ci-result`'s `needs` and passes at
      medium over the root and both templates. Every entry in
      `.github/zizmor.yml`, if there is one, has a reason and is named
      in the PR.
- [ ] `security.yml` runs only on `schedule` and `workflow_dispatch`,
      and the issue step was shown working (step 7): one issue opened,
      one comment on the second failure, and the temporary commit is
      gone.
- [ ] CodeQL ran for `rust` and `actions` on the PR, `codeql-result` is
      green, and every finding is fixed or named in the PR with why it
      is wrong here.
- [ ] The ADR and its row in `docs/adr/README.md` exist, `AGENTS.md`
      rule 1 names both exceptions, and the `dependency-upkeep` skill
      exists.
- [ ] Nothing under `templates/` gains a workflow or a Dependabot,
      CodeQL or Scorecard file.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`pins.rs`):
  - a `uses:` line with a SHA and a version comment is read; a local
    `./` workflow is skipped; a comment or a `run:` line that mentions
    `uses:` is not read; a tag, a short SHA and a missing comment are
    each reported;
  - two files that use one action at two SHAs are reported with both
    places, and `github/codeql-action/init` and `/analyze` count as one
    action;
  - `dependabot.yml` with and without a template's folder, and with a
    `rust-toolchain` update beside the `github-actions` one;
  - `[workspace.dependencies]` lines read as a plain string and as an
    inline table, `shuyi-*` placeholders skipped, a multi-line entry
    reported, and a template requirement that differs from the root's
    reported;
  - the whole check on a temporary tree (`links.rs`'s `temp_repo`
    pattern) skips `.claude/worktrees` and `target`, and passes on the
    real repository.

  The mutants job runs on this diff, so each branch needs an assertion
  that fails under a mutant.
- CI: the `zizmor` job and the CodeQL run on the PR, the issue step's
  test (step 7), and Scorecard's first run on `main` (step 8).

## Completion notes

*(Filled in by the session that completes the ticket.)*
