---
id: "2860"
title: "Traction as a number: GitHub stars, forks, issues, views and clones and crates.io downloads, collected weekly and kept"
type: infra
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: ["2859"]
nick_input: setup
completed:
---

# 2860 — Traction as a number, collected weekly

## Context

The community files (1132) and outside pull requests come back if the
engine "gets traction" (`docs/decisions.md`, 2026-11-01), and 2859 asks
Nick for the number. Nothing measures it today.

GitHub keeps a repository's traffic (views, unique visitors, clones,
unique cloners, top referrers and paths) for 14 days only
(<https://docs.github.com/en/rest/metrics/traffic>), so it has to be
collected at least weekly from **before** the launch, or the launch week
is lost. Stars, forks, watchers and issues are public through the API,
and crates.io gives each crate's total and recent downloads
(<https://crates.io/data-access>).

The website has no analytics and the engine promises no telemetry. Page
views, feed subscribers and plays of the browser examples stay
unmeasured, and this ticket says so instead of adding a tracker.

## Nick input

**Setup**, only if the workflow's own `GITHUB_TOKEN` can't read the
traffic endpoints (they need push access; the session checks first and
says in the PR):

1. On github.com: Settings → Developer settings → Personal access tokens →
   Fine-grained tokens → Generate new token.
2. Repository access: only `Zafnok/shuyi-engine`.
3. Permissions: Repository → Administration: read-only. Nothing else.
4. Expiration: 1 year. Generate, and copy it.
5. In the repository: Settings → Secrets and variables → Actions → New
   repository secret, with the name the PR gives. Paste and save.

## Scope

**In:**
- A collector, `cargo xtask traction` (in `crates/xtask/`), that reads:
  - GitHub: stars, forks, watchers; issues opened in the window by anyone
    other than `Zafnok` and bots; traffic views and unique visitors,
    clones and unique cloners, the top referrers;
  - crates.io: total and recent downloads of each published `shuyi`
    crate.
- A weekly scheduled workflow, `.github/workflows/traction.yml`, that runs
  it and appends one dated row to a CSV kept on a branch of its own
  (`traction-data`, never `main`), plus the run's row as an artifact. The
  data stays in this repository.
- The traction number from 2859 printed beside the current value in the
  run's summary.
- A short page saying what is measured, what is not (site page views,
  feed subscribers, plays of the web examples) and why: no tracker, no
  telemetry. Placed where the docs map says, with its row there.

**Out (do not do):**
- Any analytics, tracker or pixel on the website or in the engine.
- Any third-party analytics service or sign-up.
- Measuring social posts and threads (Nick reads those).

## Implementation steps

1. Check in a scratch workflow run whether `GITHUB_TOKEN` with
   `administration: read` reaches `/repos/{owner}/{repo}/traffic/views`.
   If not, the workflow reads the secret of the setup above.
2. Write `cargo xtask traction`, parsing the API answers into one row;
   tests over recorded answers in `crates/xtask/tests/fixtures/`, no
   network in tests.
3. Write `traction.yml`: weekly cron and `workflow_dispatch`, least
   permissions (`contents: write` only for the data branch), pinned
   actions, zizmor clean.
4. Run it once by hand for a baseline before the first release (2005).
5. Write the page and its docs-map row (the `document` skill).

## Acceptance criteria

- [ ] A run on `main` appends one row with a date and every field in *In*.
- [ ] A second run adds a second row and keeps the first.
- [ ] The collector's tests pass with no network.
- [ ] zizmor reports no finding on `traction.yml`.
- [ ] The page lists site page views, feed subscribers and plays of the web examples under what is not measured.
- [ ] A baseline row dated before the first release exists (or, if 2005 lands first, the completion notes say so and give the first row's date).
- [ ] The completion notes have `Book:` and `Skills:` lines saying why neither changes.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: a row parsed from recorded GitHub and crates.io answers; issues by
  `Zafnok` and bots not counted; a missing traffic answer (no permission)
  is an error that names the setup step, not a row of zeros.
- Integration: the CSV append keeps earlier rows.

## Completion notes

*(Filled in by the session that completes the ticket.)*
