# ADR-0038: The public API gate: a listing diff and semver checks on every PR, and a `Breaking:` line for each break

- **Status:** Accepted
- **Date:** 2026-10-23
- **Related tickets:** 2010; 2005 (the version policy, which cites this
  ADR), 2013 (`#[non_exhaustive]` and constructors), 2804 (the changelog
  that lists the `Breaking:` lines), 2116 (a moved golden value), 2012
  (moving the pinned toolchain), 2011 (Dependabot), 2014 (the game built
  against each PR; dropped 2026-10-26), 2030 (the verdict as a PR label)

## Context

Visions of Shuyi follows `main` by a pinned revision. Before this ADR a
break in the engine's API was reported by hand, from memory (`AGENTS.md`:
"when an API changes, say so in the PR"), and one nobody wrote down
showed up at the game's next pin bump as a compile error with no note
saying what changed. After the first crates.io release (2005), games
trust the version number too, and a silent break in a 0.x patch breaks
them on `cargo update`. Nick put "a check that names API breaks" among
the protections in the MVP (`docs/decisions.md`, 2026-10-20).

What others do: Godot's CI compares its GDExtension API with the last
release, and each break needs an entry in a file of expected changes.
Bevy labels breaking PRs and asks for a migration guide by hand, which no
tool checks. In Rust there are two tools:

- `cargo-semver-checks` (Apache-2.0 OR MIT) lints a crate's rustdoc JSON
  against a baseline and names each change that needs a major version.
  It builds rustdoc JSON with the active toolchain, the stable one too.
- `cargo-public-api` (MIT) lists a crate's public items, one per line. It
  reads rustdoc JSON from a nightly, and each release reads one rustdoc
  JSON format only.

Every crate is `publish = false` at 0.1.0 until 2005, on both sides of
every PR, so version numbers can't say what kind of change a PR is.

## Decision

1. **What is checked.** The six crates a game made with `shuyi new`
   depends on: `shuyi-core`, `shuyi-assets`, `shuyi-ui`,
   `shuyi-platform`, `shuyi-app` and `shuyi-cli` (whose library a game's
   tools implement). They are `API_CRATES` in
   `crates/xtask/src/api.rs`. Not `xtask` (never shipped), the examples
   or the templates. Not the `--json` tool contract either: ADR-0022 §5's
   schema test fingerprints it.

2. **`cargo xtask api`** compares the head (the working tree, so a local
   run sees changes not yet committed) with a base checked out by `git
   worktree add --detach target/api-base <rev>`, removed again at the
   end, also on failure. For each crate:
   - the **listing diff**: `cargo +<nightly> public-api -p <crate>
     --simplified` in both trees, and the lines removed and added. A
     changed signature is one of each;
   - the **semver check**: `cargo semver-checks check-release -p <crate>
     --baseline-root target/api-base --release-type minor --color never`,
     its findings parsed from the text it prints (0.51.0 has no
     machine-readable output);
   - a crate the base doesn't have is new: every item is added, and no
     semver check runs.

   It writes a Markdown summary (a table, a `diff` per changed crate cut
   at 200 lines, the warnings, the verdict), appended to `--summary` and
   printed. A semver run that fails without naming a break (a build
   error, a tool error) fails the command: it never counts as "no
   break".

3. **The tools are pinned**, as consts in `api.rs` that CI installs
   (a test checks `ci.yml` says the same):
   - `cargo-semver-checks` 0.51.0, which reads the rustdoc JSON of the
     pinned 1.98.1 (`rust-toolchain.toml`), installed by
     `taiki-e/install-action`;
   - `cargo-public-api` 0.52.0, by `cargo install --locked`. It reads
     rustdoc JSON format 57 only;
   - `nightly-2026-05-23`, the first 1.98 nightly (the workspace's
     `rust-version`), which writes format 57. The oldest nightly that
     builds the workspace has the fewest lints the stable doesn't.

   They move together with the toolchain pin: a PR that moves
   `rust-toolchain.toml` moves the nightly and the tools when the new
   toolchain needs it (the `bump-rust` skill's checklist has that step).
   Without
   a tool, or at another version, the command prints the install line
   and fails.

4. **What "breaking" means.** A crate is breaking when
   `cargo-semver-checks` has a finding on it, with `--release-type
   minor`. In 0.51.0 an explicit release type is taken as it is, whatever
   the version numbers say: `minor` runs only the lints whose required
   update is major, so only those can fail. Without the flag the tool
   derives the release type from the versions, which is "no change;
   assume minor" while both sides are 0.1.0, and would turn every lint
   off once 2005 bumps a crate to 0.2.0. Warning-level lints don't fail;
   the listing diff shows what they and the tool don't judge.

5. **The rule** (CI passes `--pr-branch`):
   1. no breaking crate: pass;
   2. breaking crates, on a branch `t<NNNN>-<slug>` (read by
      `ticket_id_from_branch`): ticket NNNN must be in `tickets/done/`
      and have, under `## Completion notes`, a `Breaking:` line naming
      each breaking crate;
   3. breaking crates on any other branch (`sonar/…`, `dependabot/…`,
      `game-NNNN-…`): fail. **No break without a ticket**: ADR-0017 says
      so of Sonar fixes, and it holds for every change without one.

   A crate whose listing lost lines with no semver finding is a warning
   in the summary, never a failure.

6. **The `Breaking:` line** is a line under a ticket's `## Completion
   notes` that starts with `Breaking:`, after an optional `- ` list
   marker:

   ```text
   Breaking: <crate>[, <crate>…]: <item>: <what a game changes>
   ```

   A break the job can't see writes the format and the field as the
   item (a file format) or the table and the value (a moved golden
   value, 2116). `breaking_lines` in `crates/xtask/src/tickets.rs` is
   the one reader of these lines; the changelog (2804) and the golden
   check (2116) call it. The PR body repeats them.

7. **Lint levels are never lowered to pass** (`AGENTS.md` rule 6): no
   skip list, no `allow`. A wrong finding gets a `Breaking:` line that
   says why it is wrong, and an issue upstream.

8. **CI's `api` job** runs it on pull requests that change `crates/`,
   the root `Cargo.toml`, `Cargo.lock` or `rust-toolchain.toml`
   (`changes.yml`'s `crates` output), with `--base HEAD^1` (the base's
   tip; HEAD is the merge commit), and `RUSTFLAGS: ""`: warnings are the
   `clippy` and `test` jobs' gate, and a lint only the nightly has must
   not fail this job as a build error. It is in `ci-result`'s `needs`.
   It is not a local gate: run it before pushing a change you think
   breaks.

9. **What it leaves to others.** Version numbers and `CHANGELOG.md` are
   2005's version policy, which cites this ADR instead of restating it.
   `#[non_exhaustive]`, constructors and `with_` setters are 2013's ADR.
   What broke in the game is found by the game's pin moves (2014, a
   build of the game against each PR, was dropped on 2026-10-26:
   `docs/decisions.md`).

## The verdict as a label (ticket 2030)

Added after acceptance; the decision above is unchanged.

- `cargo xtask api --verdict <file>` writes one word to `<file>`:
  `breaking` when any crate has a semver finding (§4), `added` when a
  listing changed without one (lost lines included), and `none`
  otherwise. `verdict` and `Label` in `api.rs` compute it. A run that
  can't compare writes nothing.
- CI's `api-label` job puts it on the PR as one label, `api: none`,
  `api: added` or `api: breaking`, and removes the other two. A PR the
  `api` job skips (no change under `crates/` and the rest of §8's paths)
  is `api: none`. A failed `api` job sets no label. The job has no
  checkout and no build, so the `pull-requests: write` token never meets
  the PR's code. A fork's PR has a read-only token and stays unlabelled.
- **Why a label:** a job summary can't be read through GitHub's API from
  another repository, and a public PR's labels can, without a token
  (`gh pr view <N> --json labels -R Zafnok/shuyi-engine`). It is for
  readers outside CI: a game that depends on the engine by git revision
  and wants to know which merged PRs changed what it can call. The game's
  watcher (game#1190) reads it to decide when to move its pin, and falls
  back to its own test on a PR without one.

## Consequences

- A break is named by a tool on every PR, and written down by the agent
  that made it, before and after the first release. The game's pin bump
  reads the `Breaking:` lines.
- The summary shows each PR's whole API change, added items too, with
  nothing to commit.
- The job costs a nightly, two tools and four rustdoc builds per crate
  (two listings, two semver builds); rust-cache keeps the dependencies.
- `cargo-semver-checks` is a floor, not a proof: some trait-bound and
  generic changes pass it. The listing diff and its warning show the
  rest to whoever reads the summary.
- Before 2005, the game's compile at its pin bump catches breaks anyway;
  this gate makes them recorded and explained, not impossible.
- A Sonar, Dependabot or small game-driven PR can't break the API: it
  fails, and the break becomes a ticket.

## Alternatives considered

- **Committed listings (`api/<crate>.txt`) and a `--bless` step** — a
  nightly in every agent's local gates, Nick's Windows GNU machine too,
  churn on every PR that adds an item, and reviewers to read them, which
  this repository doesn't have.
- **Only `cargo-semver-checks`** — it says what breaks, not what was
  added or changed, and misses some breaks; the listing shows them.
- **Only `cargo-public-api`'s diff** — every removed line would be a
  break, a signature that only grew a generic default too; it can't
  judge.
- **Version numbers as the signal** — every crate is 0.1.0 until 2005,
  and after it the release type must still be fixed (§4).
- **A comment on the PR** — it needs write permission, which a fork's
  PR doesn't get; the job summary is enough.
- **A baseline on crates.io** — nothing is published yet; checking a
  release against the last published version is 2005's.
