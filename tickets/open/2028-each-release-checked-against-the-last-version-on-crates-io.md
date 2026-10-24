---
id: "2028"
title: "Each release after the first checked against the last version on crates.io: cargo-semver-checks before cargo publish refuses a version too small for its breaks"
type: infra
milestone: After MVP
model: sonnet-5
effort: high
status: todo
blocked_by: ["2005"]
nick_input: none
completed:
---

# 2028 — Each release checked against the last version on crates.io

## Context

2010 (done) made CI's `api` job: each PR's public API is checked
against its base's tip, and a break needs a `Breaking:` line
([ADR-0038](../../docs/adr/0038-the-public-api-gate-a-listing-diff-and-semver-checks-on-every-pr-and-a-breaking-line-for-each-break.md)).
It runs `cargo semver-checks` with `--release-type minor`, because
every crate has the same version on both sides of a PR (§4). It left a
baseline on crates.io to 2005 (§9, and its last alternative).

2005 publishes the first release and writes the version policy as an
ADR: one version for `shuyi` and every `shuyi-*` crate, 0.x while the
API moves, and the `Breaking:` lines since the last tag decide the
next version. Its first release has nothing to compare with, so it
left this check to a follow-up (its Out): this ticket.

What a release check sees that the PR check doesn't:

- **The version is picked by hand.** A release whose `Breaking:` lines
  were misread goes out as a patch. Cargo treats a 0.x patch (0.2.0 to
  0.2.1) as compatible and a 0.x minor (0.2.0 to 0.3.0) as breaking
  (<https://doc.rust-lang.org/cargo/reference/semver.html>). So every
  game that asks for `"0.2"` takes that patch on `cargo update`, and
  breaks.
- **The PR check sees each PR alone.** A PR that merged on a stale
  base was checked against a main that no longer exists: main's
  ruleset doesn't require an up-to-date branch (2026's Context). The
  sum of the PRs since the last release is never checked.

`cargo-semver-checks` does this already: `check-release
--baseline-version <X.Y.Z>` downloads that version from crates.io and
lints the working tree against it. release-plz runs it before each
release (2010's Context).

## Nick input

None.

## Scope

**In:**

- **`cargo xtask release-api [--version <X.Y.Z>] [--summary <file>]`**
  (`crates/xtask/src/release_api.rs`, tests in
  `release_api/tests.rs`, as `api.rs` has them; its lines in
  `main.rs`'s `USAGE` and `match`):
  - **The release's version:** `--version`, or by default `version` in
    the root `Cargo.toml`'s `[workspace.package]`, read with
    `toolchain.rs`'s `value_of` (made `pub(crate)`). It must be a plain
    `X.Y.Z`. `--version` asks "would this tree pass as X.Y.Z?" before
    the version is raised and tagged.
  - **The crates:** `API_CRATES` from `api.rs`. After 2005 these are
    the crates `release.yml` publishes; a test keeps the two the same.
  - **The baseline** of each crate: the newest version in crates.io's
    index that is below the release's version, not yanked, not a
    pre-release, and not 0.0.0 (the name holders,
    [`docs/decisions.md`](../../docs/decisions.md), 2026-10-21 and
    2026-10-22).
    - The index is `https://index.crates.io/<path>`, lowercase, with
      the path rule of the Cargo book's *Registry index* (`shuyi-core`
      is `sh/uy/shuyi-core`).
    - It is fetched through `api.rs`'s `Runner` with `curl --silent
      --show-error --write-out "\n%{http_code}" <url>`. A 200 is the
      index: one JSON object per line, read with `serde_json` (`vers`,
      `yanked`). A 404 is a crate crates.io doesn't have. Any other
      code, or curl not starting, is an error.
  - **A crate with no baseline** is new since the last release. It is
    skipped, with a notice in the summary and a `::notice::` line on
    GitHub (beside `main.rs`'s `annotation` for warnings). It never
    fails the command.
  - **The release type** comes from the two versions, by Cargo's rules,
    and is passed as `--release-type`. The leftmost non-zero part of
    the baseline acts as the major:

    | Baseline | Release | Type |
    | --- | --- | --- |
    | 0.2.0 | 0.3.0 | `major` |
    | 0.2.0 | 0.2.1 | `minor` (a 0.y patch may add items) |
    | 1.4.2 | 2.0.0 | `major` |
    | 1.4.2 | 1.5.0 | `minor` |
    | 1.4.2 | 1.4.3 | `patch` |
    | 0.0.3 | 0.0.4 | `major` |

    ADR-0038 §4 found that the pinned tool takes an explicit release
    type as it is. Passing it, rather than letting the tool derive one
    from the manifests, makes `--version` work without editing
    `Cargo.toml`, and keeps the rule in code that tests read.
  - **The semver check:** `cargo semver-checks check-release -p
    <crate> --baseline-version <baseline> --release-type <type> --color
    never`, from the repository root, with the same features as the
    `api` job (the tool's default; no feature flags).
  - **Its output** is read by `api.rs`'s `parse_semver`, which learns
    the bump the tool asks for from its `Summary` line (`semver
    requires new major version`, or `minor`). The PR check keeps its
    behavior: with `--release-type minor` only a major can be asked.
  - **The verdict:**
    - no finding on any crate: pass;
    - a finding: fail. The message names each crate, lint and item,
      and the smallest version that passes, from the crate's baseline:
      a major ask on 0.2.0 needs 0.3.0, on 1.4.2 needs 2.0.0; a minor
      ask on 1.4.2 needs 1.5.0. All crates share one version, so the
      release needs the largest of them. The message says nothing was
      published, and to raise the workspace version (2005's version
      policy) and tag again.
  - **No verdict is never a pass.** A semver run that fails without
    naming a break (a build error, a download error), or an index
    fetch that fails, fails the command, as ADR-0038 §2 says of the PR
    check.
  - **The summary:** a table (crate, its baseline or "new", the release
    type, the findings), the notices and the verdict. It is appended to
    `--summary` and printed, with `api.rs`'s `write_summary`.
  - **Tools:** only `cargo-semver-checks` at `SEMVER_CHECKS`; no
    nightly and no `cargo-public-api`. `api.rs`'s `missing_tools` is
    split so this command asks for that one tool, with the same install
    line.
- **A job `release-api` in `.github/workflows/release.yml`** (2005's
  workflow), on the tag's run:
  - `runs-on: ubuntu-latest`, `permissions: contents: read`, checkout
    with `persist-credentials: false`;
  - `rustup show`, and the macroquad system libraries the way
    `release.yml`'s other jobs install them: the tool builds the
    rustdoc of `shuyi-app` and of its baseline, which need them;
  - `cargo-semver-checks` at `SEMVER_CHECKS` through
    `taiki-e/install-action`, pinned by the same SHA as in `ci.yml`
    (`cargo xtask check-pins` wants one SHA per action);
  - no `Swatinem/rust-cache`: no job of the tag's run reads the Actions
    cache (2005). Building each crate twice without a cache takes
    minutes, which is fine once per release;
  - `cargo xtask release-api --summary "$GITHUB_STEP_SUMMARY"`, with
    `RUSTFLAGS: ""` in the step's `env`, as in the `api` job
    (ADR-0038 §8);
  - the job that runs `cargo publish` adds `release-api` to its
    `needs`, so a refused release publishes nothing.
- **An ADR** (the next free number; the `write-adr` skill), "Each
  release checked against the last version on crates.io":
  - what is checked (`API_CRATES`) and the baseline rule;
  - the release type from the two versions, and why it is passed
    rather than derived;
  - a new crate is skipped;
  - a refused release publishes nothing, and what to do then;
  - a wrong finding is not argued away: the release takes the larger
    version (cheap on 0.x), and an issue goes upstream (ADR-0038 §7);
  - it cites 2005's version policy and ADR-0038 and doesn't restate
    them. ADR-0038 stays as it is.

  Its row goes in `docs/adr/README.md`.
- **The skills and docs:**
  - `run-gates`, *The public API (CI's `api` job)*: a paragraph saying
    `release.yml` runs `cargo xtask release-api` before it publishes.
    It is not a local gate. Run it, with the network, before raising
    the version and tagging, with `--version <X.Y.Z>` for the version
    you mean.
  - `bump-rust`, step 3: `release.yml`'s `cargo-semver-checks` line
    moves with `SEMVER_CHECKS` too (a test keeps them equal).
  - Where 2005 wrote the release steps: before tagging, run `cargo
    xtask release-api --version <X.Y.Z>`; if the tag's run refuses the
    release, nothing was published.
  - `crates/xtask/Cargo.toml`'s `description` names the release check.
- **For the game:** nothing changes. This is `xtask`, a workflow, an
  ADR and skills; no engine crate's API moves. No `Breaking:` line.

**Out (do not do):**

- The first release, the version policy, `release.yml` and its tag
  check (2005).
- Any change to the PR check: `cargo xtask api`, the `api` job and
  `--release-type minor` stay as ADR-0038 says.
- Choosing the next version from the `Breaking:` lines: the version
  policy does that. This check only refuses a version too small for
  what the tool finds.
- Breaks the tool can't see (file formats, golden values, problem
  codes): their `Breaking:` lines carry them, by the version policy.
- A listing diff against crates.io: it needs a nightly and the
  published source, and each PR's summary showed its listing already.
- A check on the PR that raises the version. The tag's run refuses a
  wrong version before anything is published, and `--version` lets a
  session ask first.
- Yanking or fixing a version already published.
- Lowering a lint's level, or skipping a crate, to pass (`AGENTS.md`
  rule 6).
- A crates.io token: the job reads the public index and downloads
  public crates.

## Implementation steps

1. Check with the pinned `cargo-semver-checks` (`SEMVER_CHECKS` in
   `crates/xtask/src/api.rs`):
   - that `check-release -p <crate> --baseline-version <v>` in this
     workspace downloads `<v>` from crates.io;
   - that an explicit `--release-type` is taken as given, `patch` too;
   - the exact `Summary` line for a major ask and for a minor ask.

   Write what you find in the ADR.
2. Capture fixtures in `crates/xtask/tests/fixtures/api/release/`,
   from `shuyi-core` against its last release, with the local paths
   shortened: a pass (the next patch, no change) and a major ask (the
   next patch, a public function removed). A minor ask can't be
   captured while every crate is 0.x: test it on the major ask's
   output with its `Summary` line changed.
3. Write `release_api.rs`:
   - `Version` (a plain `X.Y.Z`, ordered) and its parser;
   - `index_path`, the fetch and the index reader, and `baseline`;
   - `release_type` and `needed`;
   - the run over `API_CRATES`, the verdict and the summary.

   In `api.rs`, extend `parse_semver` with the bump and split
   `missing_tools`. Make `value_of` in `toolchain.rs` `pub(crate)`.
4. Add the command to `main.rs`, the `release-api` job to
   `release.yml`, and `release-api` to the publish job's `needs`.
5. Write the ADR, its row, and the skills' and docs' lines.
6. Prove it locally against the real crates.io, on a scratch tree
   that is never committed, with `--version` one patch above the last
   release:
   - no change: it passes;
   - a public function removed from `shuyi-core`: it fails, naming the
     crate, the lint, the function and the next minor;
   - the same tree, with `--version` at that next minor: it passes.

   Write the three verdicts in the completion notes. If 2005's
   `release.yml` has a dry run that stops before `cargo publish`, run
   the job through it too and link the run.
7. Run the gates.

## Acceptance criteria

- [ ] `cargo xtask release-api` checks each of `API_CRATES` against its
      newest version on crates.io below the release's (not yanked, not
      a pre-release, not 0.0.0) and prints a summary table.
- [ ] A crate with no such version is skipped with a notice, and
      doesn't fail the command.
- [ ] The release type passed to the tool follows the Scope's table
      (unit tests).
- [ ] A release whose version is too small fails, naming each crate,
      lint and item and the smallest version that passes; a release at
      that version passes.
- [ ] A failed index fetch or semver run fails the command; it never
      passes.
- [ ] Without `cargo-semver-checks` at `SEMVER_CHECKS`, the command
      prints its install line and fails; it doesn't ask for the nightly
      or `cargo-public-api`.
- [ ] `release.yml` runs the `release-api` job on the tag, without a
      cache, and its publish job needs it.
- [ ] The three local runs of step 6 are in the completion notes.
- [ ] `cargo xtask api` behaves as before: its tests pass on the old
      fixtures.
- [ ] The ADR exists with its row in `docs/adr/README.md`, and the
      `run-gates` and `bump-rust` skills and the release steps say what
      the Scope says.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit, `release_api/tests.rs`, with fake runners:
  - `index_path` for names of one, two, three and more characters, and
    a name in capitals;
  - `baseline`: the newest below the release; a yanked newest skipped;
    0.0.0 skipped; a pre-release skipped; a version equal to the
    release's or above it skipped; none left (new); a line that isn't
    JSON (an error);
  - the fetch: 200, 404 (new), 500 (an error), curl not starting (an
    error);
  - `release_type`: each row of the Scope's table;
  - `needed`: the Scope's three cases, and the largest over two crates;
  - the run: every crate passes; one crate new; a major ask on one
    crate (the message names it and the version); a semver build error
    (an error); the version read from a made-up `Cargo.toml`; a
    `--version` that isn't `X.Y.Z` (an error);
  - the summary for a fixed input, against an expected string;
  - a missing tool: only `cargo-semver-checks`'s install line.
- Unit, `api/tests.rs`: `parse_semver` reads the bump from the new
  fixtures, and gives what it gave before on the old ones.
- Unit: `release.yml`, read with `serde_norway`, installs
  `cargo-semver-checks@{SEMVER_CHECKS}`, runs `cargo xtask
  release-api`, names each of `API_CRATES`, has no `rust-cache`, and
  its publish job needs `release-api` (as
  `ci_installs_the_pinned_tools_and_nightly` reads `ci.yml`).
- Local: the three runs of step 6, against crates.io.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
