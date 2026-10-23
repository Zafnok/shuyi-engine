---
id: "2010"
title: "Every PR shows its public API change: a CI job with cargo-public-api diffs and cargo-semver-checks against the base, and a Breaking: line required when it breaks"
type: infra
milestone: MVP
model: opus-5.5
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2010 — Every PR shows its public API change, and a break needs a Breaking: line

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8. Nick put "a check that names API breaks" among the
protections that go in the MVP, before the first release
([`docs/decisions.md`](../../docs/decisions.md), 2026-10-20, *a second
comparison*).

**What the benchmarks have:**

- **Godot:** CI compares its GDExtension API (`extension_api.json`) with
  the last release, and each break it means to make needs an entry in a
  file of expected changes
  (<https://docs.godotengine.org/en/stable/contributing/development/handling_compatibility_breakages.html>).
- **Rust's own tools:**
  - `cargo-semver-checks` (Apache-2.0 OR MIT,
    <https://github.com/obi1kenobi/cargo-semver-checks>) lints a crate's
    rustdoc JSON against a baseline (a git revision, a folder or a
    published version) and names each change that needs a major
    version. release-plz runs it before each release.
  - `cargo-public-api` (MIT,
    <https://github.com/cargo-public-api/cargo-public-api>) lists a
    crate's public items one per line, so two lists can be diffed. It
    reads rustdoc JSON from a nightly toolchain.
  - The Cargo book counts a new variant on an exhaustive enum, or a new
    public field on a struct a game can build, as a major change
    (<https://doc.rust-lang.org/cargo/reference/semver.html>).
- **Bevy** labels breaking PRs and asks for a migration guide written
  by hand. No tool checks it.

**What Shuyi has today:**

- Breaks are reported by hand, from memory:
  - `AGENTS.md`, *Working with the game repository*: "when an API
    changes, say so in the PR";
  - the `sonar-fix` skill (step 8): a Sonar PR that changes the public
    API says so. ADR-0017 (*Consequences*): such a fix is a ticket of
    its own;
  - the `write-ticket` skill: each ticket's Scope says whether the API
    breaks;
  - the `work-ticket` skill (§3, *Public API*): a version bump and a
    `CHANGELOG.md` line, but only after the first crates.io release.
- `.github/workflows/ci.yml` has the jobs `changes`, `fmt`, `clippy`,
  `test`, `docs`, `wasm`, `templates`, `no-std`, `layers`, `deny`,
  `machete`, `coverage`, `tickets` and `ci-result`. None reads the
  public API. `crates/xtask/src/main.rs` has `ticket-lint`,
  `check-links`, `check-docs`, `check-no-std` and `check-layers`.
- The tool contract has a mechanical check: ADR-0022 §5's schema test
  fails when the JSON schemas change without a new `contract::VERSION`.
  The Rust API has nothing like it.
- Every crate is `publish = false` at version 0.1.0 (the root
  `Cargo.toml`). A game made with `shuyi new` depends on six library
  crates: `shuyi-core`, `shuyi-assets`, `shuyi-ui`, `shuyi-platform`,
  `shuyi-app` and `shuyi-cli` (`templates/grid/Cargo.toml`). Its
  `crates/game/src/tools.rs` implements `shuyi_cli::GameTools` and uses
  `shuyi_cli::contract` types, and its `examples/tools.rs` calls
  `shuyi_cli::main_with`, so `shuyi-cli`'s library is game-facing too.
- 7 `#[non_exhaustive]` attributes, all in `shuyi-platform`
  (`crates/platform/src/platform.rs`, `crates/platform/src/app.rs`),
  across about 100 `pub enum`s and 380 `pub struct`s.

**What is planned:**

- 2804 lists each ticket's `Breaking:` lines first in the changelog,
  and adds the step to the `work-ticket` skill. Nothing checks the lines
  get written.
- 2005 writes the version policy as an ADR and publishes. It names no
  tool.
- 2013 writes the ADR on `#[non_exhaustive]`, constructors and `with_`
  setters.
- 2014 builds Visions of Shuyi against each engine PR, report-only. It
  shows what broke in the game, not what changed in the API.
- Many open tickets already ask for this ticket's `Breaking:` line on
  a break (2114, 2338, 2346, 2350, 2355, 2406, 3044 and others).

**Why it matters:**

- **The game first.** It follows `main` by a pinned revision. A break
  nobody wrote down shows up only at its next pin bump, as a compile
  error with no note saying what changed or what to do. A `Breaking:`
  line written when the change is made says it.
- **Other games.** After 2005, games on crates.io trust the version
  number. A silent break in a 0.x patch release breaks them on `cargo
  update`.
- **The end goal (2026-10-19).** Agents, weaker models among them, make
  most changes. "Tests are the review" (`AGENTS.md` rule 6) then covers
  the API too: a tool names the break, and the agent must write it down.

**Limits, said plainly:**

- cargo-semver-checks is a floor, not a proof. It misses some breaks
  (some trait-bound and generic changes). The listing diff in the
  summary shows what it doesn't judge.
- Before 2005, the game's compile at its pin bump catches breaks
  anyway. This gate makes them recorded and explained, not impossible.

## Nick input

None.

## Scope

**In:**

- **`cargo xtask api`** (`crates/xtask/src/api.rs`; its line in
  `main.rs`'s `USAGE` and `match`):
  - **Arguments:** `--base <rev>` (default: `git merge-base HEAD
    main`), `--pr-branch <branch>`, `--summary <file>` (Markdown is
    appended; CI passes `$GITHUB_STEP_SUMMARY`).
  - **The crates:** a const `API_CRATES` with `shuyi-core`,
    `shuyi-assets`, `shuyi-ui`, `shuyi-platform`, `shuyi-app` and
    `shuyi-cli`. Not `xtask` (never shipped) and not the examples. A
    crate on the list that the base doesn't have is reported as new
    (every item added), with no semver check.
  - **The base tree:** `git worktree add --detach target/api-base
    <rev>` (an old one is removed first; `git worktree remove --force`
    at the end, also on failure). The working tree is the head, so a
    local run checks changes not yet committed.
  - **The listing diff:** for each crate, `cargo +<nightly> public-api
    -p <crate> --simplified` in the head and in the base tree, then the
    lines removed and the lines added. A changed signature shows as one
    removed and one added line.
  - **The semver check:** for each crate, `cargo semver-checks
    check-release -p <crate> --baseline-root target/api-base` with
    `--color never`, and the release type set so that only the lints
    that need a major version fail. Every crate is 0.1.0 on both sides
    until 2005, so the version numbers can't say it. The flag that does
    this on the pinned version is written in the ADR, with why.
    - The findings per crate (lint names and items) are parsed from its
      output.
    - A run that fails for any other reason (a build error, a tool
      error) fails `cargo xtask api`. It never counts as "no break".
  - **The rule** (with `--pr-branch`). The crates with a semver finding
    are the breaking crates:
    1. none: pass;
    2. some, and the branch names a ticket (`t<NNNN>-<slug>`, parsed by
       `ticket_id_from_branch` in `crates/xtask/src/tickets.rs`, made
       `pub(crate)` and shared, not copied): that ticket's file in
       `tickets/done/` must have, under `## Completion notes`, a
       `Breaking:` line that names each breaking crate. Otherwise it
       fails, printing the crates, lints and items, and an example line;
    3. some, and the branch names no ticket (`sonar/…`, `dependabot/…`,
       `game-NNNN-…`): fail. A PR with no ticket doesn't break the API
       (ADR-0017 already says so of Sonar fixes). The message says to
       write a ticket (`write-ticket` skill).

    Without `--pr-branch` (a local run) the rule is skipped: it prints
    the summary, and fails only on a tool error.
  - **A `Breaking:` line** is a line under `## Completion notes` that
    starts with `Breaking:`, after an optional `- ` list marker. Its
    form: `Breaking: <crate>[, <crate>…]: <item>: <what a game
    changes>`. A break in a file format, which this job can't see,
    writes the format and the field as the item (2517's rule 4). One
    function parses the lines, `breaking_lines(body: &str) -> Vec<&str>`
    in `tickets.rs`, so 2804's changelog reads the same lines (either
    order with 2804, below).
  - **Lines the listing lost** in a crate with no semver finding are a
    warning in the summary ("removed lines cargo-semver-checks didn't
    call breaking: read them"), never a failure.
  - **The summary:**
    - a table: crate, items removed, items added, semver findings;
    - for each crate with a change, a `<details>` block holding the diff
      in a `diff` fence, cut at 200 lines with an "and N more" line;
    - the rule's verdict and, on a failure, the line to write.
    It is appended to `--summary` and printed.
  - **Missing tools:** without `cargo-semver-checks`, `cargo-public-api`
    or the nightly, it prints the exact install lines and fails:
    `cargo install --locked cargo-semver-checks@<version>`, `cargo
    install --locked cargo-public-api@<version>`, `rustup toolchain
    install nightly-<date> --profile minimal`. The versions and the
    date are consts in `api.rs`, the same ones CI installs.
  - **Testable:** every `git` and `cargo` call goes through one small
    trait (`Runner`) that the tests fake, the way `tickets.rs` splits
    `list_on_github` (the call) from `gh_listing` (the logic).
- **The CI job `api`** in `.github/workflows/ci.yml`:
  - `needs: changes`, `if: github.event_name == 'pull_request' &&
    needs.changes.outputs.crates == 'true'`, on `ubuntu-latest`;
  - checkout with `persist-credentials: false` and `fetch-depth: 2`. As
    in `changes.yml`, HEAD is the PR's merge commit and `HEAD^1` the
    base's tip, so the diff is exactly what merging adds;
  - `rustup show`, the pinned nightly (`--profile minimal`),
    `Swatinem/rust-cache`, and the macroquad system libraries as in the
    `clippy` job (`shuyi-app`'s dependencies need them to document);
  - `RUSTFLAGS: ""` in the `cargo xtask api` step's `env`. Warnings
    are the `clippy` and `test` jobs' gate, and a lint only the
    nightly has must not fail the API job as a build error;
  - both tools at the pinned versions, through `taiki-e/install-action`
    (already pinned by hash in `ci.yml`) if it carries them, else
    `cargo install --locked`;
  - `cargo xtask api --base HEAD^1 --pr-branch "$HEAD_REF" --summary
    "$GITHUB_STEP_SUMMARY"`, with `HEAD_REF: ${{ github.head_ref }}` in
    `env` as the `tickets` job does (never `${{ }}` inside `run:`);
  - `api` added to `ci-result`'s `needs`. A skipped `api` passes there,
    as every skipped job does.
- **`changes.yml`** gets a second output, `crates`: `'true'` when a PR
  changes a path under `crates/`, the root `Cargo.toml`, `Cargo.lock` or
  `rust-toolchain.toml`; always `'true'` on other events. The `code`
  output stays as it is.
- **An ADR** (the next free number; the `write-adr` skill), "The public
  API gate":
  - what is checked: the six crates; not `xtask`, the examples or the
    templates, and not the `--json` contract (ADR-0022 §5 fingerprints
    it);
  - the two tools, how their versions and the nightly are pinned, and
    that they move together with the toolchain pin;
  - the baseline: the PR's base tip;
  - what "breaking" means before 2005: a finding cargo-semver-checks
    rates major, with every crate at 0.1.0, and the flag that says so;
  - the `Breaking:` line: where it goes and its form (2116 uses it for
    a moved golden value too);
  - no break without a ticket;
  - lint levels are never lowered to pass (`AGENTS.md` rule 6). A wrong
    finding gets a `Breaking:` line that says why it is wrong, and an
    issue upstream;
  - what it leaves to others: version numbers to 2005's version policy,
    which cites this ADR instead of restating it; `#[non_exhaustive]`
    and constructors to 2013's ADR.
  - Its row goes in `docs/adr/README.md`.
- **The skills and `AGENTS.md`:**
  - `work-ticket`: §3's *Public API* bullet says CI's `api` job finds
    breaks on every PR, before and after the first release. §5 step 1
    says to add one `Breaking:` line per break, in the form above, and
    §6 says the PR body repeats them.
  - `run-gates`: a section "The public API (CI's `api` job)". It is not
    a local gate (it needs a nightly and two tools). Run `cargo xtask
    api` before pushing a change you think breaks, after installing
    with the lines it prints. *The game at the next pin* points at the
    `Breaking:` lines.
  - `write-ticket`: the line on "whether the API breaks" names the `api`
    job and the `Breaking:` line. *A small change needs no ticket here*
    says such a change (a `game-NNNN-…` branch) never breaks the API:
    the `api` job fails it.
  - `sonar-fix` step 8: the `api` job fails a Sonar PR that breaks the
    API; stop and write a ticket.
  - `AGENTS.md`, *Working with the game repository*: "when an API
    changes, say so in the PR" becomes "when an API changes, write a
    `Breaking:` line in the ticket's completion notes (CI's `api` job
    finds the change)".

**Out (do not do):**

- Committed API listings (`api/<crate>.txt`) and a `--bless` step. They
  would put a nightly in every agent's local gates, on Nick's Windows
  GNU machine too, and churn on every PR that adds a public item. Their
  readers would be reviewers, and this repository has none. The job
  summary shows the diff with nothing to commit.
- A baseline on crates.io, and a check of a release against the last
  published version (2005 and after).
- Version numbers and `CHANGELOG.md` (2005); the changelog that lists
  the lines (2804).
- The rule on `#[non_exhaustive]`, constructors and `with_` setters, and
  adding them to existing types (2013).
- Building the game against the PR (2014).
- Dependabot (2011).
- Migration guides, and the `migration` block under a `Breaking:` line
  or a check that it is there (2523, 2804). `breaking_lines` reads the
  `Breaking:` lines only.
- A comment on the PR. It needs write permission, which a fork's PR
  doesn't get; the job summary is enough.
- Lowering a lint's level, or a skip list, to pass.

**Either order:**

- **Either order with 2804.** If 2804 is done, the `work-ticket` skill
  already has its `Breaking:` step: keep one step, in the form above,
  and make 2804's reader call `breaking_lines`. If not, this ticket adds
  the step and the function, and 2804 reuses both.
- **Either order with 2013.** If 2013 is done, the ADR cites its ADR.
  If not, 2013's ADR cites this one, and adding `#[non_exhaustive]` to
  an existing type is a break, so 2013's PR carries the `Breaking:`
  lines this job asks for.
- **The pinned Rust (2012, done).** A PR that moves the pinned
  toolchain (`rust-toolchain.toml`) moves this job's nightly date and
  tool versions too when the tools need it. The ADR says so, and the
  `bump-rust` skill's checklist gets that step in this PR.
- **Either order with 2116.** 2116's `cargo xtask golden --breaks`
  asks for a `Breaking:` line when a golden table's value changes,
  using `ticket_id_from_branch` and `breaking_lines` too. If 2116 is
  done, reuse the `pub(crate)` `ticket_id_from_branch` and the
  `breaking_lines` it wrote, and its split of calls from logic, instead
  of a second copy. If not, 2116 reuses this ticket's.
- **With 2011 (done).** The `dependency-upkeep` skill's step 3 (*A
  requirement change*) says the `api` job fails a Dependabot PR that
  breaks the API (the rule's case 3), and such an upgrade goes to its
  step 6 (a ticket of its own).
- **Either order with 2513.** If 2513 is done, the job installs the
  system libraries with `bash scripts/setup-linux.sh`, as the other
  jobs do. If not, the job uses the `clippy` job's `apt-get` line, and
  2513 turns it into the script.

**For the game:** no API change. This is CI, `xtask`, an ADR, skills
and `AGENTS.md`. Its later pin bumps read the `Breaking:` lines.

## Implementation steps

1. Pick the versions:
   - the newest `cargo-semver-checks` release that reads the rustdoc
     JSON of the pinned toolchain (1.98.1, `rust-toolchain.toml`);
   - the newest `cargo-public-api`, and the oldest nightly that it
     accepts and that builds the workspace (the fewest new lints).
   Then read how the pinned `cargo-semver-checks` treats a 0.x version
   that didn't change, and pick the release-type flag. Check that
   `--baseline-root` finds a package in a workspace, and that `-p`
   checks a `publish = false` crate. If the pinned version has a
   machine-readable output, parse that instead of its text.
2. Capture the pinned `cargo-semver-checks`'s output for a pass, one
   finding, two lints, and a build error, as fixtures in
   `crates/xtask/tests/fixtures/api/`.
3. Write `api.rs`: the `Runner` trait, the worktree, the listing diff,
   the output parser, the rule and the summary. Move
   `ticket_id_from_branch` to `pub(crate)` and add `breaking_lines` in
   `tickets.rs`.
4. Add the `crates` output to `changes.yml`, the `api` job to `ci.yml`,
   and `api` to `ci-result`.
5. Prove it on a demo PR, closed unmerged, from a branch
   `t2010-api-demo` cut from this ticket's branch once this ticket is in
   `done/` there (`work-ticket` §5), so the ticket lint passes on it:
   - remove a public function from `shuyi-core`: the job fails, naming
     the crate, the lint and the function;
   - add `Breaking: shuyi-core: <function>: …` to this ticket's
     completion notes on that branch: it passes;
   - put the function back and add a new one: it passes, and the
     summary lists the new one as added.
   Link the three runs from this PR's body.
6. Write the ADR, the skills' and `AGENTS.md`'s lines, and the
   *Either order* lines for each of 2804, 2013, 2012, 2116, 2011 and
   2513 that is done.
7. Run the gates.

Size: if the diff runs well past 600 changed lines (fixtures aside),
move the rule and the `Breaking:` line (`breaking_lines`, the rule's
cases and the skills' lines) to a follow-up ticket that 2005 also waits
for, and keep the listing diff, the semver findings and the CI job here.

## Acceptance criteria

- [ ] `cargo xtask api` lists the removed and added items of the six
      crates and their semver findings, in a summary table with a diff
      per changed crate.
- [ ] The rule's cases pass as unit tests, with fake runners.
- [ ] The demo PR's three runs (step 5) are linked from this PR and
      show: fail without a `Breaking:` line, pass with one, and pass
      with an added item that the summary lists.
- [ ] This PR's own `api` run passes (`xtask` is not a checked crate).
- [ ] A PR that changes only docs or tickets skips `api`, and
      `ci-result` passes.
- [ ] Without one of the tools, `cargo xtask api` prints its install
      line and fails.
- [ ] A semver run that fails to build fails the job instead of passing
      (a unit test on the build-error fixture).
- [ ] The ADR exists with its row in `docs/adr/README.md`, and the
      `work-ticket`, `run-gates`, `write-ticket` and `sonar-fix` skills
      and `AGENTS.md` say what the Scope says.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit, `api.rs`:
  - the listing diff: removed only, added only, a changed line as both,
    no change, a crate new in the head;
  - the output parser on the captured fixtures: a pass, one finding,
    findings in two lints, a build error (an error, never a pass);
  - the rule: no break on a ticket branch; a break with a `Breaking:`
    line naming the crate; a line naming another crate (fail); a line
    outside `## Completion notes` (fail); a ticket not in `done/`
    (fail); a break on a `sonar/` branch (fail); no break on a `sonar/`
    branch (pass);
  - the summary for a fixed input, compared with an expected string,
    the 200-line cut included;
  - a missing tool: the install line.
- Unit, `tickets.rs`: `breaking_lines` with and without the `- ` marker,
  with CRLF line endings, and ignoring lines in other sections.
- CI: the `api` job itself, shown by the demo PR.

## Completion notes

*(Filled in by the session that completes the ticket.)*
