---
id: "2031"
title: "A push to main starts the watcher workflows of projects that pin the engine, with a token that can only run workflows"
type: infra
milestone: MVP
model: sonnet-5
effort: low
status: todo
blocked_by: []
nick_input: setup
completed:
---

# 2031 — A push to main starts the workflows of projects that pin the engine

## Context

1. **The game ticket:** game#1190
   (`github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1190-a-watcher-on-the-engines-main-files-a-pin-move-when-its-public-api-changes.md`).
   Its watcher workflow (`.github/workflows/engine-watch.yml` in the
   game) reads this repository's `main` and files an issue to move the
   game's pin when an engine PR changed the public API (2030's label),
   unblocks a game ticket, or has waited 48 hours. Without a signal from
   here it has to poll on a schedule. Nick (2026-10-25) prefers a
   token-based push to polling. It is not a blocker here.
2. **The job in one sentence:** when `main` moves, tell each project
   that depends on the engine by git revision, right away, so it can
   decide whether to move its pin.
3. **No friction review:** a missing feature.
4. **The generic shape:** a workflow on push to `main` that runs
   `workflow_dispatch` on a list of `owner/repo` and workflow file pairs
   held in a repository variable, passing the new `main` sha as an
   input. It uses a fine-grained token that has only **Actions: write**
   on those repositories. That is the permission `workflow_dispatch`
   needs: it can start and cancel workflow runs but can't push code or
   read secrets. `repository_dispatch` needs Contents: write, which can
   push code, so it is not used. The token is a secret of an
   environment that only `main` may deploy to, so a workflow changed in
   a PR branch can never read it.
5. **An example that isn't tactics:** any game made with `shuyi new`
   that tracks `main` before the first crates.io release (2005), for
   example a farming game, lists its own watcher workflow in the
   variable and hears about each merge. The list is data, so the
   workflow names no project.

**Either order with game#1190:** if game#1190 is done, the
end-to-end check runs here: a push to `main` starts the game's
`engine-watch.yml` with `engine_sha`, and `gh run list` shows it. If not,
nothing to do here beyond the workflow: the dispatch to a workflow that
doesn't exist yet only warns (step 3), and game#1190's acceptance checks
the first dispatch that reaches it.

## Nick input

**Setup** (about three minutes, once a year when the token expires):

1. On GitHub, open your picture (top right) → **Settings** →
   **Developer settings** → **Personal access tokens** →
   **Fine-grained tokens** → **Generate new token**.
2. Name: `shuyi-engine dispatch`. Expiration: the longest offered.
   Resource owner: `Zafnok`. Repository access: **Only select
   repositories** → `visions-of-shuyi`.
3. Permissions → Repository permissions → **Actions: Read and write**.
   Leave everything else at No access. **Generate token** and copy it.
4. In the `shuyi-engine` repository: **Settings** → **Environments** →
   `dependents` (the PR of this ticket creates it; if it isn't there,
   **New environment** named `dependents`, then **Deployment branches
   and tags** → **Selected branches** → add `main`) → **Add
   environment secret**: name `DEPENDENTS_DISPATCH_TOKEN`, paste the
   token, **Add secret**.

The session working this ticket gives Nick these steps when the workflow
is ready. It never sees or handles the token itself.

## Scope

**In:**
- `.github/workflows/notify-dependents.yml`: `on: push: branches:
  [main]` and `workflow_dispatch` (to test it), in a file of its own so
  CI's `cancel-in-progress` on `main` never drops a notice.
- The `dependents` environment, limited to `main`, and the repository
  variable `DEPENDENT_WORKFLOWS` (one `owner/repo workflow-file` per
  line; today `Zafnok/visions-of-shuyi engine-watch.yml`), set with
  `gh api` or `gh variable set`.
- A short ADR (next number): why a push and not polling, why
  `workflow_dispatch` with Actions: write and not `repository_dispatch`
  with Contents: write, the environment limited to `main`, the yearly
  expiry, and that ADR-0037 §6 (no long-lived publishing token) is
  unaffected: this token publishes nothing and can't change code.
- `docs/contract.md` or the README section on depending on the engine
  by git revision: how another project adds itself to the list.

**Out (do not do):**
- Waiting for CI on `main` before notifying. Every PR passed CI before
  merging, and the project's own pin PR runs its own gates.
- Sending the API verdict in the dispatch. The receiving workflow reads
  2030's labels itself, so the payload is only the sha.
- Handling the token in the session (Nick input above).

## Implementation steps

1. The workflow: `permissions: {}` at the top; one job with
   `environment: dependents`, `if: github.ref == 'refs/heads/main'`, and
   actions pinned by SHA as in the other workflows.
2. Its step reads `DEPENDENT_WORKFLOWS` (`vars`), the token (`secrets`)
   and `github.sha` through `env:` (never `${{ }}` inside `run:`). For
   each line it runs `gh workflow run "$FILE" --repo "$REPO" --ref main
   -f engine_sha="$SHA"`.
3. A missing token or an empty list is a `::notice::` and a success
   (forks, clones, before the setup). A failed dispatch is a
   `::warning::` naming the repository and GitHub's message (an expired
   token says 401; a workflow not there yet says 404). It never fails
   the run: a dependent's problem must not mark the engine's `main`
   red. The receiver keeps a slow schedule as a backstop (game#1190).
4. The environment and variable (Scope), the ADR, the docs.
5. After Nick's setup: run the workflow from the Actions tab and check
   the dispatch reached the game (`gh run list --repo
   Zafnok/visions-of-shuyi --workflow engine-watch.yml --event
   workflow_dispatch --limit 1`), per the Either-order clause.

## Acceptance criteria

- [ ] `notify-dependents.yml` exists and passes `zizmor`
      (`security.yml`); its job runs only on `main`, in the `dependents`
      environment.
- [ ] Without the secret, a run ends green with a notice (shown in the
      run log, linked in the Completion notes).
- [ ] With the secret: after a push to `main`, the game's
      `engine-watch.yml` has a `workflow_dispatch` run whose
      `engine_sha` is that push's sha. If game#1190 isn't done yet, the
      run shows the 404 warning instead, and the Completion notes say
      so.
- [ ] The ADR is in `docs/adr/` and its index.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- None in Rust (YAML and a shell step). The checks are the runs in the
  acceptance criteria, as ADR-0037's workflows were checked.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
