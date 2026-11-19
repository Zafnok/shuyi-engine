---
id: "3322"
title: "A game's tag release can publish to GOG, when the developer turns it on"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["3406", "3504"]
nick_input: setup
completed:
---

# 3322 — A game's tag release can publish to GOG, when the developer turns it on

## Context

Nick decided on 2026-11-18 ([decisions](../../docs/decisions.md), *a
release publishes nowhere until the developer picks where*): "should have
options for all major storefronts. otherwise its not set by default".
3406 made a tag's release a draft by default, with itch.io and a public
GitHub Release as options
([ADR-0133](../../docs/adr/0133-a-release-publishes-nowhere-by-default-debug-files-in-a-draft-sealed-packages.md)).
This ticket adds GOG beside them, in the base's
`templates/shuyi-default/base/.github/workflows/release.yml`, as 3321
does Steam.

GOG takes builds through its Galaxy Pipeline Builder, a command-line tool
for its partners, driven by a JSON project file per build (check the
tool, its platforms and its terms on the day: GOG's partner docs). GOG
is curated: a game is accepted before it has a product id.

## Nick input

**Setup**, only for the real run: a GOG partner account and a product
the game was accepted for, and its credentials as the repository's
secrets, with the exact clicks written here by the session. Without it,
the job is built and tested by reading, and the real run is a ticket of
its own (`nick_input: setup`).

## Scope

**In:**
- A job `gog` after `release`, run on a tag only when its repository
  variable (the product id) is set, with the secrets it names, and no
  write permission on the repository; never a prerelease to the default
  branch.
- It reads the packages as the `itch` job does (opening `*.sealed` with
  `PRIVATE_ASSETS_KEY`), never a debug file, for `windows`, `linux` and
  `macos`.
- The pipeline builder fetched in the job with a written checksum, if
  its terms let CI download it; if they don't, say so in the ticket and
  the docs, and stop at the docs (a manual upload from the draft).
- The base's README and AGENTS.md *Releasing*, `skills/shuyi-ship`
  step 8, `docs/contract.md`, the book's *Releasing*.
- Tests in `crates/cli/src/contract/new/tests.rs`, as 3321's.

**Out (do not do):**
- Steam (3321), the Epic Games Store (3323), `shuyi repos` asking (3410).
- GOG Galaxy's achievements or cloud saves.

**Estimated size:** about 900 lines (code 300, tests 300, docs 300).

## Implementation steps

1. Read ADR-0133 and the base's `release.yml`; read 3321's job if done.
2. Check GOG's partner docs on the day.
3. The job, the docs, the tests.
4. A real run if Nick has set it up; otherwise the ticket for it.

## Acceptance criteria

- [ ] A tag of a game with GOG's variable set uploads its packages to
      the product's builds; without it, the job doesn't run.
- [ ] No debug file is uploaded; the job has no write permission.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- `crates/cli/src/contract/new/tests.rs`: as Scope says.
- actionlint and zizmor pass on the changed workflow.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
