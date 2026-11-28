---
id: "3321"
title: "A game's tag release can publish to Steam, when the developer turns it on"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["3406", "3504"]
nick_input: setup
completed:
---

# 3321 — A game's tag release can publish to Steam, when the developer turns it on

## Context

Nick decided on 2026-11-18 ([decisions](../../docs/decisions.md), *a
release publishes nowhere until the developer picks where*): "should have
options for all major storefronts. otherwise its not set by default".
3406 made a tag's release a draft by default, with itch.io and a public
GitHub Release as options
([ADR-0133](../../docs/adr/0133-a-release-publishes-nowhere-by-default-debug-files-in-a-draft-sealed-packages.md)).
This ticket adds Steam beside them, in the base's
`templates/shuyi-default/base/.github/workflows/release.yml`.

Steam takes builds through SteamPipe: `steamcmd +login <builder>
+run_app_build <app_build.vdf>` uploads depots and sets them live on a
branch. Its builder account needs Steam Guard, which CI meets with a
saved `config.vdf` or a TOTP secret (check on the day). The Steam Deck's
package (`steam-deck`, 3012) is for Steam, and the itch.io job skips it.

## Nick input

**Setup**, only for the real run: a Steam partner account (Steam Direct
costs a fee per app: check the amount on the day), an app id, a builder
account, and its secrets on the scratch or the game's repository, with
the exact clicks written here by the session. Without it, the job is
built and tested by reading, and the real run is a ticket of its own
(`nick_input: setup`).

## Scope

**In:**
- A job `steam` after `release`, run on a tag only when the repository
  variable `STEAM_APP_ID` is set (and a prerelease only to a branch that
  isn't the default, or never: decide and say why), with the secrets it
  names (`STEAM_USERNAME`, and Steam Guard's), and no write permission on
  the repository.
- It reads the packages as the `itch` job does (the same artifacts,
  opening `*.sealed` with `PRIVATE_ASSETS_KEY`), never a debug file, and
  maps exporters to depots (`windows`, `linux`, `macos`, `steam-deck`)
  from repository variables or a file the game commits (decide; an ADR
  if a new file format).
- `steamcmd` downloaded in the job from Valve, its checksum written down
  as the `itch` job does butler's; check its license lets CI download it
  (it ships nothing).
- The base's README and AGENTS.md *Releasing*, `skills/shuyi-ship`
  step 8, `docs/contract.md`, the book's *Releasing*: how to turn it on.
- Tests in `crates/cli/src/contract/new/tests.rs` (read with
  `serde_norway`): the job runs only with its variable, after `release`,
  with no write permission, opens the sealed files, skips `*.debug`.
- **Written once for every store** (the redundancy sweep of 2026-11-28:
  3321, 3322 and 3323 had the same scaffold word for word):
  - the `itch` job's download, *Open the sealed files* and
    per-exporter unpack loop move into one composite action
    (`.github/actions/store-packages/` in the template's base), which
    `itch` and every store job call;
  - the tests above are one table over the store jobs (name, variable,
    exporters), so a store is a row;
  - the docs get one *Publishing to a store* section with a paragraph
    per store.
- **Either order with 3322 and 3323:** if one of them is done, it wrote
  the composite action, the table and the section, so this ticket uses
  them and adds Steam's row and paragraph. If not, this ticket writes
  them, and 3322 and 3323 add a row each.

**Out (do not do):**
- GOG (3322), the Epic Games Store (3323), `shuyi repos` asking (3410).
- Steamworks features in the game (achievements, 3037).

**Estimated size:** about 900 lines (code 300, tests 300, docs 300).

## Implementation steps

1. Read ADR-0133, the base's `release.yml` and its `itch` job.
2. Check on the day SteamPipe's docs (partner.steamgames.com) for CI
   builds and Steam Guard, and steamcmd's download and license.
3. The job, the depot mapping, the docs, the tests.
4. A real run if Nick has set it up; otherwise the ticket for it.

## Acceptance criteria

- [ ] A tag of a game with `STEAM_APP_ID` set pushes its packages to its
      Steam app's depots; without it, the job doesn't run.
- [ ] No debug file is uploaded; the job has no write permission.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- `crates/cli/src/contract/new/tests.rs`: as Scope says.
- actionlint and zizmor pass on the changed workflow.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
