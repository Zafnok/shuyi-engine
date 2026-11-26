# ADR-0152: Publishing the engine crates: the workspace is the list, the pins are exact, and the version moves only at release time

- **Status:** Accepted
- **Date:** 2026-11-25
- **Related tickets:** 3510, 2005 (the first release, which runs on
  this), 2028 (the version checked at the tag), 2804 (`cargo xtask
  changelog`), 2040 (`API_CRATES`), 3511 (the minimum Rust), 3512
  (security releases and yanks)
- **Related ADRs:** ADR-0032 (the layers, checked over `cargo metadata`
  in the same CI job), ADR-0038 (one version on both sides of a PR),
  ADR-0075 (the `alsa-sys` patch)

## Context

2005 publishes the engine on crates.io. Its plan listed nine crates by
hand, in dependency order. The workspace has fourteen engine crates, and
the five the list missed are dependencies of the nine (`shuyi-embed`
and `shuyi-embed-macros` under `shuyi-assets`, `shuyi-script` under
`shuyi`, `shuyi-sound` and `shuyi-sound-out` under `shuyi-app`). Nick
held all fourteen names as 0.0.0 placeholders by 2026-11-24
(`docs/decisions.md`).

Every dependency between the crates was `path` only. `cargo package`
refuses that ("all dependencies must have a version requirement
specified when packaging"), so no crate that depends on another could
be packaged. No rule said that `shuyi-app` 0.2.0 takes `shuyi-core`
0.2.0 and nothing else, so a game could resolve a mixed set. And
`cargo publish --workspace` (Rust 1.90, the pin is 1.100) publishes the
members in their dependency order, after checking each against the
others' packages, so a written order is a second copy that can drift.

Measured on the pin, 2026-11-25:

- Cargo checks a package against the other packages of the same run
  only when they may be published. Under the workspace's `publish =
  false` it looks for `shuyi-embed` 0.1.0 on crates.io, finds the 0.0.0
  placeholder, and fails.
- A package's resolve doesn't carry the root's `[patch.crates-io]` of
  `alsa-sys` (ADR-0075). `shuyi-app` has quad-snd and, behind
  `in-house-sound`, CPAL, each with `links = "alsa"`. So `shuyi-app`'s
  package can't make a lockfile, and neither can a crate above it.
  `--exclude-lockfile` packages it.
- `cargo publish --workspace` with a version already on crates.io fails
  before it uploads anything ("crate … already exists on crates.io
  index"; a dry run says it as a warning). It skips nothing.
- With the workspace entry at `default-features = false`, a member
  that omits `default-features` takes no defaults, and `features =
  ["default"]` is refused for a crate that has no `default` feature.

## Decision

1. **The engine crates are the publish list.** A member is published
   when `layers::is_engine` names it (`shuyi`, `shuyi-*`), and only
   then. They are `API_CRATES` (`crates/xtask/src/api.rs`), and a test
   holds the two sets equal, so a new crate is listed once, there.
   - Each engine crate has `version.workspace = true` and
     `publish.workspace = true`.
   - Every other member says `publish = false` itself (`xtask`, the
     examples, `crates/embed/fixture`, `site/snippets`), so 2005's flip
     of the workspace's `publish` reaches the engine crates alone.
   - The workspace's `publish` stays `false` until 2005. Agents work
     with Nick's credentials, and the flag keeps a stray `cargo publish`
     from going out.
2. **One version, pinned exactly.** All engine crates share the
   workspace's version. Each engine crate another depends on has one
   entry in the root `[workspace.dependencies]`:
   `shuyi-core = { path = "crates/core", version = "=0.1.0",
   default-features = false }`. Cargo warns on an entry nothing uses,
   so `shuyi`, `shuyi-cli` and `shuyi-playdate` have none until a crate
   depends on them.
   - The `=` holds a release together. Each crate is published at once
     with the others and is tested only with them, and under 0.x a minor
     release may break. A caret would let `shuyi-app` 0.2.1 run on a
     `shuyi-core` 0.2.0 it was never built with, or a game mix two
     patches. A fix is a release of every crate (§4), so the exact pin
     costs nothing.
   - An engine crate's normal and build dependencies on engine crates
     say `{ workspace = true, ... }` with their own `features` and
     `optional`. The entry turns defaults off, because cargo ignores a
     member's `default-features = false` when the entry turns them on.
     A member that took the defaults adds `"default"` to its `features`,
     where the crate has one. `cargo tree` gives every package the same
     features before and after the move.
   - Dev-dependencies on engine crates stay path-only. Cargo drops a
     dev-dependency with no version from the package, which keeps the
     test cycles (`shuyi-core`'s tests use `shuyi-assets`) out of the
     published crates.
3. **`cargo xtask check-publish`** (`crates/xtask/src/publish.rs`) holds
   §1 and §2 over the manifests of the members `cargo metadata` lists.
   It also fails an engine crate that depends, normally or to build, on
   a member that isn't published, and a cycle among their normal and
   build dependencies. It runs in the gates' `layers` gate and in CI's
   `layers` job.
   - `--order` prints the engine crates in the order they publish, each
     after what it depends on. The runbook reads it.
   - `--index` asks crates.io's sparse index for each name. It needs the
     network, so the gates leave it out and CI's `layers` job runs it. A
     new crate fails CI until Nick holds its name.
   - The `layers` job then runs `cargo package --no-verify --locked
     --exclude-lockfile` on every crate `--order` prints, in one command.
     It first turns the workspace's `publish` on in its own checkout
     (cargo's rule above). `--no-verify` because the packages build only
     once 2005 moves `templates/`, `targets/` and `skills/` into them.
     `--exclude-lockfile` because of the `alsa` patch, which 2005
     settles for the real publish.
4. **The release publishes with `cargo publish --workspace`,** with no
   written order. A run that stops part way follows
   [`docs/releasing.md`](../releasing.md): the rest goes out with
   `--exclude` for each crate already live. A fix that needs code is
   the next patch of every crate, and the stopped version's half set is
   yanked. A live version is never changed.
5. **The version and `CHANGELOG.md` change only at release time.** A
   ticket's PR never raises the version or edits `CHANGELOG.md`. At the
   tag, the version is chosen from the `Breaking:` lines since the last
   tag (2028 checks it), and `cargo xtask changelog` writes the whole
   `CHANGELOG.md` (2804). ADR-0038 §4 already keeps one version on both
   sides of a PR. The `work-ticket` skill says this in its §3. 2005's
   version ADR states the version policy and cites this ADR for the
   mechanics.

## Consequences

- A crate added to the workspace is published, needs an `API_CRATES`
  entry and its name held on crates.io, and fails CI until both exist.
- Every release is all fourteen crates, even when only one changed.
  Crates.io shows versions with no changes. A game names one version for
  all of them.
- A member that wants an engine crate's defaults writes `"default"`. A
  crate with no `default` feature can't be asked for it, so a later
  default feature reaches no member until that member asks for it. The
  feature trees show it.
- `shuyi-app`'s package resolves only without its lockfile while quad-snd
  and CPAL both link `alsa`. 2005's `cargo publish` verifies each
  package with a build, which resolves every optional dependency, so
  2005 settles it (3079 takes quad-snd out later).
- CI changes `Cargo.toml` in its own checkout for one step. The change
  is one `sed` on the root's `publish = false`, never committed.

## Alternatives considered

- **A written publish list, checked against the workspace.** It is a
  second list to keep, and `cargo publish --workspace` already orders
  the crates.
- **Caret pins (`0.1.0`) between the crates.** These would let a game
  resolve crates that were never released or tested together. One
  version for all makes a caret buy nothing.
- **`version = "=0.1.0"` written in each member's dependency.** Same
  pins, but in about thirty lines to change at each release. The
  workspace entry is one per crate.
- **Turning the workspace's `publish` on now.** That would let the
  package step run without the `sed`. It would also leave nothing to
  stop a stray `cargo publish` from an agent with Nick's token before
  2005's sign-off.
- **Packaging each crate alone.** Every crate above the leaves would
  resolve its engine dependencies against crates.io's 0.0.0
  placeholders and fail.
