# ADR-0130: No cache of a build with the private files, one test over every workflow, no key for Dependabot, and who can read them

- **Status:** Accepted
- **Date:** 2026-11-17
- **Related tickets:** 3407, 2595, 2596, 3050, game#0171
- **Amends:** ADR-0111 §5 (no artifact **and no cache** embeds the
  private files; Dependabot's runs get no key) and ADR-0112 §2 step 6
  (one key, for builds; no Dependabot secret)

## Context

ADR-0111 §5 says a job of a game with private files checks them out
before its cargo commands, and that no workflow of the template uploads
an artifact that embeds them (ADR-0114 lets a release's packages
through). It says nothing about caches. Yet `target/` holds builds that
embed `private/files/` (`include_dir!` through `shuyi::files!` in the
game crate, ADR-0111 §3), and a pull request's run, a fork's included,
may restore its base branch's caches.

#391 (2590) shipped `ci.yml` with `Swatinem/rust-cache` after the
private files' step; #397 (2596) guarded it by copying one line,
`cache-targets: ${{ hashFiles('.github/actions/private-files/action.yml') == '' }}`,
into three files (the base's `ci.yml` and `export.yml`, and the
engine's reusable `playtest.yml`). A test pinned only `ci.yml`'s step,
by searching its text, so a new job, a new workflow or a second cache
step elsewhere passed unnoticed.

The leak is plausible, not seen. The pinned `rust-cache` (v2.9.3,
`d1527c8f…`) cleans `target/` before it saves: `src/save.ts` hands
`cleanTargetDir` the packages outside the workspace root, and the
workspace's members only when `cache-workspace-crates` is `"true"`
(default `"false"`), and `src/cleanup.ts` deletes every other entry of
`deps/`, `build/` and `.fingerprint/`. So the game crate is most likely
not cached. It would be:

- when the cleanup fails: `cleanTargetDir`'s errors are swallowed
  (`catch {}`), and `save.ts` logs at debug level only;
- when a crate outside the workspace root embeds the files;
- when a workspace crate has the exact name of a dependency the cleanup
  keeps;
- when a later version changes the default (Dependabot moves the pin),
  or a game sets `cache-workspace-crates: true`.

Two readers of the private files neither ADR weighed:

- **Dependabot.** ADR-0112 §2 step 6 made a second deploy key and set it
  as a Dependabot secret, so a Dependabot pull request bumping a crate
  checked the private files out and built: the new version's `build.rs`
  and proc macros ran next to them, with the network open, before any
  person had looked at the bump. A third party's release set it off.
- **Collaborators.** Anyone who can push a branch to the game's
  repository can run a workflow of their own that reads
  `PRIVATE_ASSETS_KEY`. That is GitHub's model.

## Decision

### 1. No artifact and no cache embeds the private files

A job that checks out the private files (a step `uses:
./.github/actions/private-files`) caches **no `target/` at all**:

- every `Swatinem/rust-cache` step in it has
  `cache-targets: ${{ hashFiles('.github/actions/private-files/action.yml') == '' }}`,
  so with the action there it caches the cargo registry only;
- any other caching step (`actions/cache`, `actions/cache/save`, an
  action whose name says `cache`, or a `setup-*` step given `cache:`)
  has an `if:` with that guard as one of its `&&` terms;
- every `actions/upload-artifact` or `actions/upload-pages-artifact`
  step has an `if:` whose `||` alternatives are that guard or an input
  the test names (`inputs.upload-private`, ADR-0114 §3);
- its workflow is not run on `pull_request_target` or `workflow_run`,
  which run with the secrets on a fork's changes, and no job of such a
  workflow passes `PRIVATE_ASSETS_KEY` to a called workflow.

The rule doesn't rest on `rust-cache`'s own cleanup of the workspace's
crates: its errors are swallowed, it keeps a crate by name, and its
default can change with a pin bump. The cargo registry holds no private
file, and stays cached.

### 2. One test is the rule's guard

`crates/cli/src/contract/new/tests.rs`'s
`every_job_with_the_private_files_caches_no_build_and_uploads_none`
composes every short name's game and the grid one with
`private-files=yes`, adds the engine's `playtest.yml` and `export.yml`,
parses each workflow with `serde_norway`, and applies §1 to every job
with the action. The guard is one constant there. A job that calls a
workflow and passes it the key (by name or `secrets: inherit`) must call
one the test reads: one of its own game's (`./…`) or the engine's
(`Zafnok/shuyi-engine/.github/workflows/…@…`). Every file under
`templates/**/.github/workflows/` must come from a part of the template
and be in a game the test composes, so a workflow of a part no short
name picks fails until the test makes a game with it. Each rule has a
case of its own that it refuses.

### 3. Dependabot gets no key

ADR-0112 §2 step 6 now makes **one** key, *builds*, whose private half
is the Actions secret `PRIVATE_ASSETS_KEY`. `shuyi repos` sets no
Dependabot secret.

- A Dependabot pull request's gates run on the public files, as a
  fork's do: the private files' action, on a `pull_request` whose
  `github.event.pull_request.user.login` is `dependabot[bot]` (the
  opener, which a re-run keeps, unlike `github.actor`; the condition
  GitHub's *Automating Dependabot with GitHub Actions* uses), warns
  instead of failing.
- The push to `main` after a person merged the bump runs the gates on
  the private files as before.
- `observe` still reads the Dependabot secrets. A game that has
  `PRIVATE_ASSETS_KEY` there, or a deploy key titled `<name> dependabot
  (read-only)`, gets a warning naming `gh secret delete
  PRIVATE_ASSETS_KEY --app dependabot --repo …` and `gh repo deploy-key
  delete <id> --repo …`. `repos` never runs them: it acts on the
  developer's account, and deletes only a key it replaces.

### 4. Who can read the private files

- CI on the game's own repository, which holds the key.
- **Anyone with write access to the game's repository**: a workflow they
  push runs with the key. Write access to the public repository is read
  access to the private one. `shuyi repos`' `--help` and report, the
  private part's `AGENTS.md` and the `shuyi-private-files` skill say so.
- Every dependency's build script and proc macro, in a job that has
  them: why Dependabot's runs get none (§3), and why a bump is merged by
  a person before `main` builds it with them.
- Not a fork's pull request: it gets no secrets, and builds on the
  public files.

## Consequences

- A job, workflow or step added to the template, or to the engine's
  `playtest.yml` or `export.yml`, that caches or uploads beside the
  private files fails a unit test, naming its file, job and step.
- A bump that breaks only the private content shows on `main`, not on
  its pull request. What it buys: no third party's release builds next
  to the private files before a person has acted on it. The template
  ships no `dependabot.yml`, so this matters only to a developer who
  turns Dependabot on.
- A game made with the old `repos` keeps its Dependabot secret and key
  until its developer deletes them; `repos` says how each time it runs.
- The game's own workflows (Visions of Shuyi's `ci.yml`, `playtest.yml`
  and `mutants.yml`) are the game's, game#0171.

## Alternatives considered

- **Trust `rust-cache`'s cleanup** — it most likely drops the game
  crate today, but its errors are silent and its default is one pin bump
  away from changing; no cache at all costs only the build's time.
- **Keep the text searches, one per file** — they pinned `ci.yml`'s one
  step, and missed every other job and workflow.
- **Keep Dependabot's key, and review its pull requests first** — the
  build runs as the pull request opens, before anyone can look.
- **An environment with required reviewers on the private files' job**
  — it keeps the key from a collaborator's workflow, but a private
  repository's environments need a paid plan, and every run would wait
  on a person.
