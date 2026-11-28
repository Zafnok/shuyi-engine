---
id: "3323"
title: "A game's tag release can publish to the Epic Games Store, when the developer turns it on"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["3406", "3504"]
nick_input: setup
completed:
---

# 3323 — A game's tag release can publish to the Epic Games Store, when the developer turns it on

## Context

Nick decided on 2026-11-18 ([decisions](../../docs/decisions.md), *a
release publishes nowhere until the developer picks where*): "should have
options for all major storefronts. otherwise its not set by default".
3406 made a tag's release a draft by default, with itch.io and a public
GitHub Release as options
([ADR-0133](../../docs/adr/0133-a-release-publishes-nowhere-by-default-debug-files-in-a-draft-sealed-packages.md)).
This ticket adds the Epic Games Store beside them, in the base's
`templates/shuyi-default/base/.github/workflows/release.yml`, as 3321
does Steam.

Epic takes builds through its BuildPatch Tool, given an organization,
a product, an artifact id and a client id and secret from the Epic Games
Developer Portal (check the tool, the platforms it runs on, its terms and
the store's fee on the day).

## Nick input

**Setup**, only for the real run: an Epic Games Developer Portal account
with a product, and its client id and secret as the repository's secrets,
with the exact clicks written here by the session. Without it, the job is
built and tested by reading, and the real run is a ticket of its own
(`nick_input: setup`).

## Scope

**In:**
- A job `epic` after `release`, run on a tag only when its repository
  variables (the organization, product and artifact ids) are set, with
  the secrets it names, and no write permission on the repository; never
  a prerelease to the live label.
- It reads the packages as the `itch` job does (opening `*.sealed` with
  `PRIVATE_ASSETS_KEY`), never a debug file, for the platforms Epic takes
  (`windows`, `macos`; check Linux on the day).
- The BuildPatch Tool fetched in the job with a written checksum, if its
  terms let CI download it; if they don't, say so in the ticket and the
  docs, and stop at the docs (a manual upload from the draft).
- The base's README and AGENTS.md *Releasing*, `skills/shuyi-ship`
  step 8, `docs/contract.md`, the book's *Releasing*.
- Tests in `crates/cli/src/contract/new/tests.rs`: a row in 3321's
  table of store jobs.
- **Either order with 3321:** if 3321 is done, call its composite action
  (`.github/actions/store-packages/`), add the Epic Games Store's row to its table test
  and a paragraph to its *Publishing to a store* section. If not, write
  those three as 3321's Scope says, and 3321 adds Steam's row.

**Out (do not do):**
- Steam (3321), GOG (3322), `shuyi repos` asking (3410).
- Epic Online Services in the game.

**Estimated size:** about 650 lines (code 250, tests 150, docs 250), with 3321's shared step and table.

## Implementation steps

1. Read ADR-0133 and the base's `release.yml`; read 3321's job if done.
2. Check Epic's developer docs on the day.
3. The job, the docs, the tests.
4. A real run if Nick has set it up; otherwise the ticket for it.

## Acceptance criteria

- [ ] A tag of a game with Epic's variables set uploads its packages as
      a build of the artifact; without them, the job doesn't run.
- [ ] No debug file is uploaded; the job has no write permission.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- `crates/cli/src/contract/new/tests.rs`: as Scope says.
- actionlint and zizmor pass on the changed workflow.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
