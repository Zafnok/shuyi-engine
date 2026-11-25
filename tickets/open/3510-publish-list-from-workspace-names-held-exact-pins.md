---
id: "3510"
title: "The publish list comes from the workspace, every engine crate's name is held, internal pins are exact, and a half-finished publish has a runbook"
type: infra
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3510 — The publish list comes from the workspace, every name is held, internal pins are exact, and a half-finished publish has a runbook

## Context

2005 is the first release on crates.io. Its release workflow publishes
"each crate in dependency order" from a list written by hand: nine
crates (`shuyi-alloc-count`, `shuyi-core`, `shuyi-assets`, `shuyi-ui`,
`shuyi-platform`, `shuyi-app`, `shuyi`, `shuyi-cli`, `shuyi-playdate`).
The workspace now has fourteen engine crates. The five the list misses
are all needed by the nine:

- `shuyi-embed-macros`: `crates/assets/Cargo.toml:51` depends on it,
  not optional (`files!`, ADR-0138).
- `shuyi-embed`: `crates/embed-macros/Cargo.toml:18` depends on it, and
  `crates/cli/Cargo.toml:87` has it as a build dependency.
- `shuyi-script`: `crates/shuyi/Cargo.toml:38`, optional. A published
  crate still needs an optional dependency on crates.io.
- `shuyi-sound` and `shuyi-sound-out`: `crates/app/Cargo.toml:84-85`,
  optional.

Their names are not held either. A blind-spot audit of 2026-11-23 read
crates.io's index: the nine names answer, and these five return 404.
`docs/decisions.md` (2026-10-21, *Only these six*) says a planned name
is published "when its crate is real", and 2026-11-01 held
`shuyi-alloc-count` and `shuyi-playdate` on that rule. All five crates
are real now (`shuyi-embed` and its macros from 2215, `shuyi-script`
from 2101, `shuyi-sound` from 2210 and `shuyi-sound-out` from #240).
Nick held all five as 0.0.0 placeholders on 2026-11-24
(`docs/decisions.md`, *the last five engine crate names are held*),
so this ticket's setup is done.

The publish mechanics have three more gaps:

- **Internal dependencies are path-only.** No `shuyi-*` dependency in
  `crates/*/Cargo.toml` has a `version`, and the root
  `[workspace.dependencies]` has no `shuyi-*` entry. `cargo package`
  refuses such a crate. Checked on the pinned Rust (1.100):
  `cargo package -p shuyi-embed-macros --no-verify` fails with "all
  dependencies must have a version requirement specified when
  packaging. dependency `shuyi-embed` does not specify a version". So
  2005's planned "`cargo publish --dry-run` for each crate, in CI" fails
  for every crate that depends on another.
- **No pin policy between the crates.** 2005's ADR plans one version for
  all of them. Nothing says that `shuyi-app` 0.2.0 must take
  `shuyi-core` 0.2.0 exactly, so a game could resolve a mixed set.
- **No plan for a publish that stops part way.** A version on crates.io
  can't be replaced or deleted. If the run stops at the seventh crate,
  six are live and eight are not, and nothing says what to do.

Rust 1.90 stabilised `cargo publish --workspace` and `cargo package`
of several workspace crates together: cargo orders them by their
dependencies and checks each against the others' packages before any is
on crates.io. The pinned Rust has it.

A second contradiction sits in the `work-ticket` skill, §3 *Public API*:
"After that release, a break also needs a version bump and a line in
`CHANGELOG.md`." The rest of the plan picks the version at release
time: 2028 (`release-api` checks the version picked by hand at the
tag), 2804 (done: `cargo xtask changelog` writes the whole
`CHANGELOG.md` from the tickets' `Breaking:` lines), and ADR-0038 §4
(both sides of a PR have the same version). 2005's ADR bullet *What a
breaking change needs* repeats the skill's "version bump after the
first release". A PR that followed the skill would bump the version
and write a changelog line the tool then overwrites.

Builds on: 2005 (the release, which this ticket's work unblocks), 2040
(done: `API_CRATES`, the fourteen crates' catalog in
`crates/xtask/src/api.rs`), 2804 (done), ADR-0038, ADR-0032 (the layers
checked over `cargo metadata`, `tools/layers.ron`).

## Nick input

None left. The setup this ticket had, holding the five names on
crates.io, was done on 2026-11-24 (`docs/decisions.md`, *the last five
engine crate names are held*), which also answered 3528's question 3.

## Scope

**In:**
- **The engine crates are the publish list.** A workspace member is
  published when `is_engine` (`crates/xtask/src/layers.rs:305`) names
  it, and only then. The set equals `API_CRATES`, so a crate added to
  the workspace is listed once, in `API_CRATES`, as today. No other
  hand-written list of crates.
- **Every other member says `publish = false` itself**: `xtask`,
  `examples/*` and `crates/embed/fixture` (`site/snippets` already
  does; 2093 took the two research fixtures out of the workspace).
  Today they inherit `publish.workspace = true`, so 2005's flip of the
  workspace's `publish` would make them publishable.
- **Exact pins through `[workspace.dependencies]`.** Each engine crate
  gets one entry in the root `Cargo.toml`:
  `shuyi-core = { path = "crates/core", version = "=0.1.0", default-features = false }`.
  Each normal and build dependency on an engine crate becomes
  `shuyi-core = { workspace = true, ... }` with its own `features` and
  `optional`.
  - **Mind `default-features`.** Cargo ignores a member's
    `default-features = false` when the workspace entry turns defaults
    on, so each workspace entry says `default-features = false`, and a
    member that took the defaults until now adds `"default"` to its
    `features`. `cargo tree -e features` over the workspace is the same
    before and after.
  - **Dev-dependencies stay path-only.** Cargo removes a dev-dependency
    with no `version` from the package, so the dev cycles stay out of
    the published crates (`shuyi-core` dev-depends on `shuyi-assets`,
    which depends on `shuyi-core`).
- **`cargo xtask check-publish`**, a new check over `cargo metadata`,
  in the gates beside `check-layers` and in CI's `layers` job. It fails:
  - an engine crate whose `version` isn't `version.workspace = true`;
  - a member that isn't an engine crate without its own
    `publish = false`;
  - a normal or build dependency on an engine crate that doesn't come
    from `[workspace.dependencies]`, or a workspace entry whose
    `version` isn't `=` and the workspace's `version`;
  - a dev-dependency on an engine crate that carries a `version`;
  - an engine crate that depends (normally or to build) on a member
    that isn't published;
  - a dependency cycle among the engine crates' normal and build
    dependencies.
  - `--order` prints the engine crates in the order they publish, one
    per line, for the runbook.
  - `--index` asks crates.io's sparse index
    (`https://index.crates.io/<prefix>/<name>`) for each name and fails
    a name that isn't there. It needs the network, so the gates leave
    it out; CI's `layers` job runs it, which makes a new crate fail CI
    until its name is held.
- **CI packages the engine crates together.** A step in the `layers`
  job runs `cargo package --no-verify --locked` with `-p` for each crate
  `--order` prints, all in one command: cargo resolves each against the
  others' packages, which catches a missing `version` or a dependency
  on an unpublished member. `--no-verify`, because the packages can't
  build until 2005 moves `templates/`, `targets/` and `skills/` into
  them; 2005's own `cargo package` and `cargo publish --dry-run` build
  them. The job gets a `timeout-minutes`.
- **An ADR on publishing:** one version for every engine crate (2005's
  version ADR states the policy; this one the mechanics), the exact
  `=` pins and why, the publish list derived from the workspace, the
  release workflow's `cargo publish --workspace` in place of a written
  order, and that the version and `CHANGELOG.md` change only at release
  time. 2005's version ADR cites it.
- **A runbook for a half-finished publish** (the `document` skill picks
  the file; `docs/releasing.md` with a row in the docs map is the
  likely place). It says, for a run of `cargo publish --workspace` that
  stopped:
  1. Read which crates are live at the version (`cargo xtask
     check-publish --order`, then each crate's page or index entry).
  2. **A failure outside the code** (the network, a rate limit, the
     index slow to show a crate): run the rest from the same commit,
     `cargo publish --workspace --exclude <each crate already live>`,
     or rerun the job if cargo skips versions already published. The
     session checks which of the two cargo does on the pinned Rust and
     writes that one.
  3. **A failure that needs a code change:** never change a live
     version. Fix on `main`, release the next patch version of every
     engine crate from the fixed commit (one version for all), and yank
     the half set of the stopped version, so no game resolves a
     mix. The stopped tag stays, with a note on its GitHub release.
  4. The GitHub release, its `llms-full.txt` and the exported packages
     (2005) are made only once every crate is live.
  5. Never yank to undo a whole release that worked.
- **The `work-ticket` skill's §3 *Public API* bullet** says the version
  is raised at release time from the `Breaking:` lines since the last
  tag (2028), and `CHANGELOG.md` is written by `cargo xtask changelog`
  (2804): a ticket's PR edits neither. Then
  `pwsh -File scripts/sync-agent-skills.ps1 -Check`.
- No public type of an engine crate changes, so ADR-0039 has nothing to
  say here. No game code changes: the game depends on the engine by a
  git revision, and the path dependencies still resolve inside this
  workspace.

**Out (do not do):**
- Publishing any real version, the release workflow, trusted
  publishing, the crates' metadata and moving `templates/`, `targets/`
  and `skills/` into packages: 2005.
- The minimum Rust a published crate promises: 3511.
- Security releases, advisories and yanks other than the half-finished
  publish: 3512.
- A second owner on the crates: 2016 (parked).
- Checking a release against the last version on crates.io: 2028.
- A lint that refuses a version edit in a ticket's PR. The tag check
  in 2005's workflow already ties the tag to the version.

**Estimated size:** about 900 lines (code 300, tests 300, docs and
manifests 300).

## Implementation steps

1. Add `publish = false` to every member that isn't an engine crate.
2. Add the fourteen entries to `[workspace.dependencies]` in the root
   `Cargo.toml`, with `default-features = false` and the exact pin.
   Save `cargo tree -e features --workspace` first.
3. Move each normal and build dependency on an engine crate in
   `crates/*/Cargo.toml` to `workspace = true`, adding `"default"` to
   `features` where the crate took the defaults. Leave dev-dependencies
   as they are. Compare `cargo tree -e features --workspace` with the
   saved one.
4. Write `crates/xtask/src/publish.rs` with `run(root)` and the order
   (a topological sort over normal and build edges), reusing
   `layers::is_engine` and `api::API_CRATES`. Wire `check-publish`
   into `main.rs`, its usage text and `gates.rs` next to `layers`.
5. Add `--index`, with the HTTP request behind a function the tests
   replace, so the tests need no network.
6. In `.github/workflows/ci.yml`'s `layers` job: `cargo xtask
   check-publish`, `cargo xtask check-publish --index` (the names are
   held, see *Nick input*), the one `cargo package --no-verify`
   step, and a `timeout-minutes`.
7. Write the ADR (the `write-adr` skill) and the runbook (the `document`
   skill).
8. Edit the `work-ticket` skill's §3 bullet; run the sync check.
9. Add `check-publish` to the `run-gates` skill's list of checks.
10. Check the fourteen names with `--index`.

## Acceptance criteria

- [ ] `cargo xtask check-publish` passes, and `--order` prints all
      fourteen engine crates, `shuyi-embed` before
      `shuyi-embed-macros` before `shuyi-assets`, and `shuyi-sound`
      before `shuyi-sound-out` before `shuyi-app`.
- [ ] `cargo package --no-verify --locked` of the fourteen together
      passes on the pinned Rust.
- [ ] `cargo tree -e features --workspace` is unchanged by the move to
      `[workspace.dependencies]`.
- [ ] Every member that isn't an engine crate has its own
      `publish = false`.
- [ ] `cargo xtask check-publish --index` passes in CI.
- [ ] The ADR and the runbook exist, and the docs map names the runbook.
- [ ] The `work-ticket` skill no longer asks a PR for a version bump or
      a `CHANGELOG.md` line.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`crates/xtask/src/publish/tests.rs`), on small manifests built
  in the test, each failing the check with its own message: an engine
  crate with its own `version`; an example without `publish = false`;
  a path-only normal dependency on an engine crate; a workspace entry
  pinned `0.1.0` instead of `=0.1.0`; a pin that differs from the
  workspace's version; a dev-dependency with a `version`; an engine
  crate depending on an unpublished member; a cycle of two engine
  crates.
- Unit: the order of a small graph puts each crate after what it
  depends on, and ignores dev edges (a dev cycle passes).
- Unit: `--index` with a fake index fails the one missing name and
  passes the rest.
- Integration: the check passes on this workspace, and its set of
  crates equals `API_CRATES`' names.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
