---
id: "3407"
title: "Every template workflow job that checks out a game's private files caches no build and uploads none, checked by one test over every workflow"
type: bug
milestone: MVP
model: opus-5.5 / gpt-6.1-sol
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-17
---

# 3407 — Every job that checks out private files caches no build and uploads none

## Context

Found by the retro of PRs #379–#484 (2026-11-17), its problem 2.

[ADR-0111](../../docs/adr/0111-a-games-private-files-a-second-repository-laid-over-its-files-whenever-checked-out.md)
§5 says a workflow job of a game with private files checks them out
before its cargo commands (the template's action
`.github/actions/private-files`), and that "No workflow of the template
uploads an artifact, and none may upload one that embeds the private
files". It says nothing about caches. Yet `target/` holds builds that
embed `private/files/` (`include_dir!` through `shuyi::files!` in the
game crate, ADR-0111 §3), and a pull request's run, a fork's included,
may restore its base branch's caches.

**The leak is plausible, not confirmed.** The pinned
`Swatinem/rust-cache` (v2.9.3, commit
`d1527c8f67ef9af357290475d4af4b0fab8f12e0`, its source read on
2026-11-17) cleans `target/` before it saves. `src/save.ts` passes
`cleanTargetDir` only `getPackagesOutsideWorkspaceRoot`, plus the
workspace's members when `cache-workspace-crates` is `"true"`, and that
input defaults to `"false"` (`action.yml`). `src/cleanup.ts`'s `rmExcept`
then deletes every entry of `deps/`, `build/` and `.fingerprint/` whose
name, with or without its trailing hash, isn't one of those packages. So
the game crate, a workspace member and the one that embeds the files,
is most likely not in the cache. It would be in it:

- when the cleanup fails, because its errors are swallowed
  (`cleanTargetDir`'s `catch {}`, and `save.ts` logs only at debug
  level);
- when a crate outside the workspace root embeds the files;
- when a workspace crate has the exact name of a dependency the
  cleanup keeps;
- when a later version of the action changes the default (Dependabot
  moves its pin), or a game sets `cache-workspace-crates: true`.

So `cache-targets: false` is defence in depth against a leak nobody has
seen, and the rule this ticket writes down is **no `target/` cache at
all** in a job that has the private files. It does not rely on the
action's default cleanup.

The history:

- **#391 (2590)** shipped the template's `ci.yml` with
  `Swatinem/rust-cache` after the private files' step, so a push to
  `main` saved a cache of a `target/` that had held them.
- **#397 (2596)** guarded it by hand-copying one line,
  `cache-targets: ${{ hashFiles('.github/actions/private-files/action.yml') == '' }}`,
  into the rust-cache step of three files:
  - `templates/shuyi-default/base/.github/workflows/ci.yml`;
  - `templates/shuyi-default/base/.github/workflows/export.yml` (its
    engine copy `.github/workflows/export.yml` is byte-equal, by test);
  - the engine's reusable `.github/workflows/playtest.yml` (2595).
- **The test pins only `ci.yml`'s step.** In
  `crates/cli/src/contract/new/tests.rs`,
  `assert_the_ci_runs_the_part_and_keeps_its_files` checks that `ci.yml`
  has one rust-cache step with the guard and that only `export.yml`
  uploads an artifact, by searching the text. A new job, a new workflow
  or a second cache step in `export.yml` or `playtest.yml` passes
  unnoticed.
- `docs/templates.md` already says "no artifact **or cache**" (2596), but
  ADR-0111 §5 was never amended.

Two threats ADR-0111 and
[ADR-0112](../../docs/adr/0112-shuyi-repos-makes-a-games-repositories-keys-and-secrets.md)
never weigh:

- **Dependabot's secret.** `shuyi repos` makes a second deploy key and
  sets it as a **Dependabot** secret of the same name
  (`crates/cli/src/repos.rs`: `make_key`, with `--app dependabot` for
  `Use::Dependabot`; ADR-0112 §2 step 6). So when Dependabot opens a pull
  request bumping a crate, the gates check the private files out and
  build. The new version's `build.rs` and proc macros then run with the
  private files on disk and the network open, before any person has
  looked at the bump. A third party's release triggers it, with no one
  in the loop.
- **Collaborators with write access.** Anyone who can push a branch to
  the game's repository can run a workflow of their own on it that reads
  `PRIVATE_ASSETS_KEY`. Write access to the public repository is read
  access to the private one. That is GitHub's model and nothing here
  changes it, but neither ADR says so, and `shuyi repos` and the
  `shuyi-private-files` skill should, so a developer knows before
  adding one.

**The decision on Dependabot's secret, made here (technical):**
`shuyi repos` stops making it.

- With it gone, a Dependabot pull request's gates run on the public
  files, as a fork's do: the private files' action warns instead of
  failing on such a run.
- The push to `main` after a person merged the bump runs the gates on
  the private files as before.
- What it costs: a bump that breaks only the private content shows on
  `main`, not on its pull request.
- What it buys: no third party's release runs next to the private files
  before a person has acted on it.
- The template ships no `dependabot.yml`, so this matters only to a
  developer who turns Dependabot on.
- A game that already has the secret keeps it. `repos` never deletes it
  (it acts on the developer's account), and says how to.

**The game's own workflows have the same exposure.** In
Visions of Shuyi (pin `e738ffc2fc37662d9b24335ab5b1d4d8743d8a6f`,
checked 2026-11-17 with `gh api
repos/Zafnok/visions-of-shuyi/contents/.github/workflows/<file>`),
`Swatinem/rust-cache` with no `cache-targets` follows its
`./.github/actions/private-assets` with `content: "true"`:

- in six jobs of `ci.yml`;
- in `playtest.yml`;
- in four jobs of `mutants.yml`.

2596 put the game out of its scope ("its own ticket there, if needed"),
and none was written. It is the game's job,
[game#0171](https://github.com/Zafnok/visions-of-shuyi/tree/main/tickets/open),
written alongside this one. It needs nothing from this ticket, and this
ticket nothing from it.

**Either order with 3406:** if 3406 is done, the test's list of inputs
that may open an upload in a private-files job (today
`inputs.upload-private`) is what 3406 left in `export.yml`. If not,
write it against today's `export.yml`; 3406 changes the list in its own
PR.

## Nick input

None.

## Scope

**In:**
- **One test over every workflow** of the template and the engine's
  reusable `playtest.yml` (below). It reads YAML with `serde_norway`
  (already a dependency of `shuyi-cli`), not text. For each job with a
  step whose `uses:` is `./.github/actions/private-files`:
  - every `Swatinem/rust-cache` step has `with.cache-targets` equal to
    the guard expression, held in one constant of the test, so the
    three copies stay one;
  - no `actions/cache`, `actions/cache/save` or other caching step,
    unless its `if:` holds the guard;
  - every `actions/upload-artifact` and `actions/upload-pages-artifact`
    step has an `if:` whose alternatives are the guard
    (`hashFiles('.github/actions/private-files/action.yml') == ''`) and
    inputs from a list the test names (today `inputs.upload-private`,
    ADR-0114 §3);
  - its workflow's `on:` has no `pull_request_target` and no
    `workflow_run`, which run a fork's changes with the secrets.
- **Called workflows.** A job whose own `uses:` calls a workflow
  (`./.github/workflows/export.yml`, the engine's
  `Zafnok/shuyi-engine/.github/workflows/playtest.yml@…`) and passes
  `PRIVATE_ASSETS_KEY` names a workflow the test also reads.
- **Nothing left out.** Every file under `templates/**/.github/workflows/`
  on disk is in a game the test composes, so a workflow of a part no
  short name picks fails until the test makes a game with it.
- A new ADR (`write-adr`) that **amends ADR-0111 §5**:
  - "no artifact **and no cache** embeds the private files": a job
    that has them caches no `target/` at all (`cache-targets` false),
    only the cargo registry;
  - why the rule doesn't rest on rust-cache's own cleanup of the
    workspace's crates: its errors are swallowed, it keeps a crate by
    name, and its default can change with a pin bump (Context);
  - the test is the rule's guard;
  - a section *Who can read the private files*: CI on the game's own
    repository; anyone with write access to it; every dependency's build
    script in a job that has them, which is why Dependabot's runs get
    none; and a fork's pull request, which gets none.
  - It also **amends ADR-0112 §2** step 6: one key, for builds; no
    Dependabot secret.
  - ADR-0111's and ADR-0112's status lines and the index name it.
- **`shuyi repos`** (`crates/cli/src/repos.rs`, its `tests.rs` and
  `fake.rs`):
  - it makes only the builds key;
  - `observe` still reads the Dependabot secrets, and a game that has
    `PRIVATE_ASSETS_KEY` there gets a warning in the report: it is no
    longer used. The warning names the `gh secret delete
    PRIVATE_ASSETS_KEY --app dependabot` and `gh repo deploy-key delete`
    commands that remove it, and `repos` never runs them.
  - Its `--help` and the step list in ADR-0112 change with it.
  - `Use` is private to the module (ADR-0112 §1): no public API changes.
- **The private files' action**
  (`templates/shuyi-default/private-files/.github/actions/private-files/action.yml`):
  - a run with no key on a pull request opened by Dependabot
    (`github.event.pull_request.user.login == 'dependabot[bot]'`, which
    stays true on a re-run, unlike `github.actor`; check GitHub's
    *Automating Dependabot with GitHub Actions* on the day) warns and
    builds on the public files, as a fork's does;
  - its `key` input's description loses the Dependabot sentence.
- The words that say Dependabot gets the secret, made to agree:
  - the base's `ci.yml` comment;
  - the private part's `AGENTS.md` ("for Actions and for Dependabot"
    in the action's error);
  - `docs/templates.md`;
  - the `shuyi-private-files` skill, if it says so (grep
    `skills/` and `templates/` for `Dependabot`);
  - `shuyi repos`' report and the `shuyi-private-files` skill both say
    that write access to the game's repository reads the private one.

**Out (do not do):**
- The game's own workflows and its Dependabot key: game#0171, the game's.
- Where a release's packages and debug files go: 3406.
- A cache of the cargo registry: it holds no private file, and stays.
- Changing GitHub's model of who reads a secret (an environment with
  required reviewers on the private files' job): it needs a paid plan
  for a private repository's environments and a person on each run.
  Name it in the ADR's alternatives.
- The engine's own workflows other than `playtest.yml` and
  `export.yml`: the engine has no private files.

**Estimated size:** about 900 lines (code 150, tests 450, docs 300).

## Implementation steps

1. Read ADR-0111 §3 and §5, ADR-0112 §2, ADR-0114 §3, 2595 and 2596
   (done), the three workflows above, the action, and
   `crates/cli/src/contract/new/tests.rs` from
   `the_private_part_writes_the_repository_its_ignore_line_and_its_ci_step`
   to `no_job_of_a_tag_reads_a_cache_and_only_the_release_writes`.
2. Write the test in `crates/cli/src/contract/new/tests.rs`, as
   `every_job_with_the_private_files_caches_no_build_and_uploads_none`:
   - compose every short name's game, and the grid one with
     `private-files=yes` (as `the_private_part_…` does);
   - collect each composed `.github/workflows/*.yml`, and the engine's
     `.github/workflows/playtest.yml` and `export.yml` read from
     `engine_checkout()`;
   - parse each with `serde_norway::Value`;
   - apply the Scope's rules, with failure messages naming the file,
     the job and the step.
   Then replace the text searches of
   `assert_the_ci_runs_the_part_and_keeps_its_files` that it covers
   (the `cache-targets` line, the one-cache count, the upload `if:`),
   and keep the rest.
3. Prove each rule fails: for each one, a small workflow text in the
   test (a second rust-cache without the guard, an upload with no `if:`,
   an `actions/cache` step, `pull_request_target`, a called workflow the
   test doesn't read) that the checking function refuses. Write the
   checks as a function over `(path, Value)` pairs so these cases need
   no composed game.
4. `shuyi repos`: drop `Use::Dependabot` from the keys `plan` makes;
   keep reading the Dependabot secrets in `observe` for the warning;
   update `tests.rs` (a fresh game makes one key and one secret; a game
   with the old Dependabot secret gets the warning, and nothing is
   deleted).
5. The action's Dependabot case; bless `plans.golden` (`SHUYI_BLESS=1
   cargo test -p shuyi-cli --lib template`).
6. The new ADR, the status lines and the index; the docs in Scope.
7. Run the gates.

## Acceptance criteria

- [x] One test reads every workflow of the template (every part) and the
      engine's `playtest.yml` and `export.yml`, and fails a job that
      checks out the private files and caches `target/`, caches by any
      other step without the guard, or uploads an artifact without the
      guard.
- [x] Each rule has a case in the test that it fails.
- [x] A workflow file under `templates/` that no composed game holds
      fails the test.
- [x] ADR-0111 §5 says "no artifact and no cache", through a new ADR that
      also lists who can read the private files (CI, collaborators with
      write access, dependencies' build scripts) and drops Dependabot's
      secret from ADR-0112 §2.
- [x] `shuyi repos` on a new game sets no Dependabot secret and makes one
      deploy key, and on a game that has the old one warns, with the
      commands, without deleting anything.
- [x] A Dependabot pull request's run without a key warns and builds on
      the public files; a run on the game's own repository without the
      key otherwise still fails.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`crates/cli/src/contract/new/tests.rs`): the test over every
  workflow, and a failing case for each rule.
- Unit (`crates/cli/src/repos/tests.rs`): the plan with no Dependabot
  key; the warning for an old Dependabot secret; a second run makes
  nothing.
- Snapshot: `plans.golden` re-blessed for the action's and the docs'
  changes.

## Completion notes

- **The test.** `every_job_with_the_private_files_caches_no_build_and_uploads_none`
  composes every short name's game and the grid one with
  `private-files=yes`, adds the engine's `playtest.yml` and `export.yml`,
  parses each with `serde_norway`, and runs `private_files_leaks` over
  them (the rules of the Scope, the guard one constant,
  `NO_PRIVATE_FILES`; the allowed upload inputs `UPLOAD_INPUTS`). It also
  checks that the private game's `ci.yml` and `export.yml` and the
  engine's two have a job with the action, so the rules can't pass on
  nothing, and that every file under `templates/**/.github/workflows/`
  belongs to a part of the template and is in a composed game of that
  part. `each_rule_on_a_job_with_the_private_files_refuses_its_leak` and
  `a_workflow_given_the_private_files_key_is_one_the_test_reads` give each
  rule a case it refuses, and cases it keeps. The text checks of
  `assert_the_ci_runs_the_part_and_keeps_its_files` for the
  `cache-targets` line, the one-cache count and the upload `if:` are
  gone; the rest stays.
- **What "other caching step" means** (the ticket left it open): an
  action whose name holds `cache`, or any step given a `with.cache`
  (`setup-node`'s `cache: npm`). A guard is kept only as an `&&` term of
  the `if:`, never under an `||`. A caller on `pull_request_target` or
  `workflow_run` that passes the key is refused too, since a called
  workflow runs on its caller's event.
- **`shuyi repos`:** `Thing::Key(Use)` became `Thing::Key` (the builds
  key, step id `builds-key` as before); `plan` makes only it; `observe`
  still reads the Dependabot secrets; `old_dependabot_key` warns of a
  Dependabot secret or a deploy key titled `<name> dependabot
  (read-only)`, naming `gh secret delete PRIVATE_ASSETS_KEY --app
  dependabot --repo …` and `gh repo deploy-key delete <id> --repo …`.
  `--help` and the report's `next` say that write access to the game's
  repository reads the private one. The fake's `gh secret set` takes an
  Actions secret only. `Use` is private to the module: no public API
  change.
- **The action:** `DEPENDABOT_PR` is `github.event_name ==
  'pull_request' && github.event.pull_request.user.login ==
  'dependabot[bot]'`, the condition GitHub's *Automating Dependabot with
  GitHub Actions* uses in its examples (read on 2026-11-17; the page
  doesn't discuss re-runs, so "the opener, which a re-run keeps" rests on
  what the field is). It joins the fork case in the warning branch; every
  other run on the game's own repository without the key still fails.
  This shell is checked by reading only: no test runs the composite
  action, as before.
- **ADR-0130** amends ADR-0111 §5 and ADR-0112 §2 step 6; both status
  lines and the index name it.
- **Docs:** the base's `ci.yml` comment, the action's descriptions and
  error, the private part's `AGENTS.md`, `docs/templates.md`, the
  `shuyi-private-files` skill (the cache rule, one key, who can read the
  files), the book's quick start (one sentence on write access), and
  3406's either-order clause rewritten as what holds.
- **Deviation:** the step-2 test lives beside two companion tests, not in
  one function, so each rule's cases read on their own.
- Game: nothing to adopt, the game's own workflows and its Dependabot key
  are game#0171's, which needs nothing from this ticket; the game's copy
  of the `shuyi-private-files` skill changes at its next pin bump.
- No ticket: game#0171 (the game's) doesn't name this ticket, and this
  repository can't edit it; it needs nothing from here.
- Behaviour: shuyi-cli: `shuyi repos` makes one deploy key and sets no
  Dependabot secret; its step `dependabot-key` is gone from the report;
  a game with the old Dependabot secret or key gets a warning with the
  commands that delete them; `next` says who can read the private files.
- Behaviour: shuyi-cli: the template's private-files action warns and
  builds on the public files on a Dependabot pull request without a key,
  where it failed before.
- Book: `site/content/learn/book/quick-start.md` says that write access to
  the game's repository reads the private one.
- Size: 1,114 counted lines: the test and its cases are about half; it splits cleanly at the `shuyi repos` and action change (ADR-0130 §3), which is already one piece of the ADR the test guards, so it stays one PR.
