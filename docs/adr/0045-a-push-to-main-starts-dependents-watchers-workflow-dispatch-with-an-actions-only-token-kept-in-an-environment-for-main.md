# ADR-0045: A push to main starts dependents' watchers: `workflow_dispatch` with an Actions-only token, kept in an environment for `main`

- **Status:** Accepted
- **Date:** 2026-10-25
- **Related tickets:** 2031, game#1190, 2030
- **Extends:** [ADR-0038](0038-the-public-api-gate-a-listing-diff-and-semver-checks-on-every-pr-and-a-breaking-line-for-each-break.md)
  (the `api:` label a watcher reads)

## Context

A game pinned to the engine by git revision moves its pin when an engine
PR changes the public API (2030's `api:` label), unblocks one of its
tickets, or has waited long enough. The game's watcher workflow
(game#1190) decides that, but it has to hear that `main` moved. Polling
on a schedule hears late and runs mostly for nothing. Nick prefers a push
from here (2026-10-25).

GitHub offers two ways for one repository's workflow to start another's:

- `repository_dispatch`, which needs a token with **Contents: write** on
  the receiver: enough to push code to it;
- `workflow_dispatch`, which needs only **Actions: write**: it can start
  and cancel runs, but can't push code or read secrets.

## Decision

1. **`.github/workflows/notify-dependents.yml`** runs on a push to `main`
   (and by hand, to test it), in a file of its own so `ci.yml`'s
   `cancel-in-progress` never drops a notice. It doesn't wait for CI on
   `main`: every PR passed CI before merging, and the dependent's own pin
   PR runs its own gates.
2. **The list is data.** The repository variable `DEPENDENT_WORKFLOWS`
   holds one `owner/repo workflow-file` per line (blank lines and `#`
   lines skipped). The workflow names no project. For each line it runs
   `gh workflow run <file> --repo <repo> --ref main -f
   engine_sha=<sha>`. The payload is only the sha: the receiver reads the
   labels and tickets itself.
3. **`workflow_dispatch` with a fine-grained token that has only
   Actions: write** on the listed repositories
   (`DEPENDENTS_DISPATCH_TOKEN`). The receiver's workflow takes an
   `engine_sha` input.
4. **The token is a secret of the `dependents` environment**, which only
   `main` may deploy to, and the job runs only on `refs/heads/main`. A
   workflow changed on a PR branch can't read it. `permissions: {}`: the
   job needs nothing of this repository's own token.
5. **A dependent's problem never marks `main` red.** A missing token or
   an empty list is a `::notice::` and a success (forks, clones, before
   the setup). A failed dispatch is a `::warning::` with GitHub's message
   (401: the token expired; 404: the workflow isn't there, or lacks the
   input). The receiver keeps a slow schedule as a backstop.
6. **The token expires** after the longest term GitHub offers, about a
   year. Nick makes a new one (2031's Nick input); until then the runs
   warn with 401 and the backstop schedule carries on.

ADR-0037 §6 (no long-lived publishing token in CI) is unaffected: this
token publishes nothing and can't change code.

## Consequences

- A dependent hears about each merge within a minute, and its schedule
  can be slow.
- A new dependent adds one line to `DEPENDENT_WORKFLOWS` and gets the
  token access to its repository: a token change only the token's owner
  can make, so a project outside Nick's account asks for a line and
  grants nothing; until then it polls.
- One more secret to renew once a year.

## Alternatives considered

- **Polling from each dependent** — late, and a run every few hours for
  nothing. Kept only as the backstop.
- **`repository_dispatch`** — needs Contents: write, which can push code
  to the receiver.
- **A GitHub App** — short-lived tokens, but an app to register, keep its
  private key as a secret, and install per repository: more setup than
  one fine-grained token for a single dependent today.
- **A step in `ci.yml` after the gates on `main`** — `cancel-in-progress`
  would drop the notice of a merge followed quickly by another, and the
  gates already ran on the PR.
