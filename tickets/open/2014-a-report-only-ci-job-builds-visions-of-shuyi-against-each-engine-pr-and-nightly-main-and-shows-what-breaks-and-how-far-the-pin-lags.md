---
id: "2014"
title: "A report-only CI job builds Visions of Shuyi against each engine PR and nightly main, and shows what breaks and how far the pin lags"
type: infra
milestone: MVP
model: opus-5.5
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2014 — The game built against every engine change

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8. Engines and compilers with users build those users
against their own changes:

- Rust's crater builds the crates ecosystem against a compiler change
  (https://github.com/rust-lang/crater).
- Godot's `linux_builds.yml` compiles godot-cpp, and loads extensions
  built for older stable releases
  (https://github.com/godotengine/godot/blob/master/.github/workflows/linux_builds.yml).
- Bevy's `validation-jobs.yml` checks that the example showcase's
  patches still apply (`check-example-showcase-patches-still-work`,
  https://github.com/bevyengine/bevy/blob/main/.github/workflows/validation-jobs.yml).

Nick put "a build of Visions of Shuyi against the engine's `main`"
among the MVP's protections ([`docs/decisions.md`](../../docs/decisions.md),
2026-10-20, *a second comparison*).

Shuyi today:

- [`AGENTS.md`](../../AGENTS.md), *Working with the game repository*:
  a change here "must not break the game at its next pin bump". Only a
  manual step checks it: the `run-gates` skill's *The game at the next
  pin* (`.claude/skills/run-gates/SKILL.md`, lines 97–103) builds the
  game with `cargo xtask engine` in the game's own checkout.
- `.github/workflows/ci.yml` builds only games made fresh from the
  templates (its `templates` job). Nothing builds the game.
- The game (`github.com/Zafnok/visions-of-shuyi`) is public (`gh api
  repos/Zafnok/visions-of-shuyi`, 2026-10-20). Its root `Cargo.toml`
  takes six engine crates by git `rev` (`shuyi-core`, `shuyi-assets`,
  `shuyi-ui`, `shuyi-platform`, `shuyi-app`, `shuyi-cli`), and its
  `shuyi-engine.rev` names the same commit. On 2026-10-20 that was
  6d96109 (the merge of 2219), two commits behind `main` (whose tip
  was the merge of 3006).
- The game's own CI (its `.github/workflows/ci.yml`) runs `cargo test
  --workspace --locked` on `ubuntu-latest` with `RUSTFLAGS: -D
  warnings`. Its only secret is Sonar's. Its bought art
  (`assets-private/`) and voice clips (`voice/`) are git-ignored and
  never in CI. So this repository's CI can build it the way the game's
  CI does, with no token.
- The game's `cargo xtask engine` (its `crates/xtask/src/engine.rs`)
  writes a `[patch."https://github.com/Zafnok/shuyi-engine"]` from a
  hard-coded `CRATES` list. This ticket builds its own patch from `cargo
  metadata` and the game's `Cargo.lock`, so a new engine crate needs no
  edit on either side.
- The ticket lint already checks `game#NNNN` ids in CI: the `tickets`
  job passes `GH_TOKEN`, the game is public, and the latest run on
  `main` (39519457844) printed `ticket-lint: ok` with no "not checked"
  warning. Only the text is stale: `run-gates` lines 89–92 say "CI has
  no access to the game repository".

Planned or done, and not this: the `Breaking:` lines (2010, 2804) and
2005's version policy are written by hand, and 2010 (done, ADR-0038)
diffs the public API of each PR in CI's `api` job and asks for a
`Breaking:` line when it breaks. None of them compiles the game.

Why it matters. The MVP is parity with the game, and the engine is
built for that game first (decisions, 2026-10-20, *Built for one game
first*). Today a break shows only when someone moves the pin, maybe
weeks later, mixed with every change since. 2003 (the `shuyi` crate)
and 2005 (crates.io) will change how the game depends on the engine,
so the job is worth most before them. Its report is also what a PR's
game line and its `Breaking:` lines (2010, 2804) should say.

## Nick input

None. It is a CI choice, and the game repository is already public.

## Scope

**In:**

- **`.github/workflows/game.yml`**, a workflow of its own. Its header
  comment says why it is report-only and why it holds no secret.
  - `on`:
    - `pull_request` with `paths`: `crates/**`, `Cargo.toml`,
      `Cargo.lock`, `rust-toolchain.toml`, `.github/workflows/game.yml`;
    - `schedule`, nightly (`cron: "0 5 * * *"`), which runs on `main`;
    - `workflow_dispatch`, with inputs `nightly` (boolean) and
      `game_repo` (default `Zafnok/visions-of-shuyi`; set only to test
      the skip below). The other events have no inputs, so the job
      reads `${{ inputs.game_repo || 'Zafnok/visions-of-shuyi' }}`.
  - `permissions: {}` at the top; `concurrency:` with `group: game-${{
    github.ref }}` and `cancel-in-progress: true`.
  - **The `game` job**, `ubuntu-latest`, `timeout-minutes: 60`,
    `permissions: contents: read`, `continue-on-error: true`. It is in
    no `needs` of `ci.yml`, and never a required check. Its one output,
    `state`, is `skipped`, `builds` or `broken`.
  - **No secret.** The game is read as any public repository is. If it
    can't be read (`gh api repos/<game_repo> --jq .private`, with
    `GH_TOKEN: ${{ github.token }}`, doesn't print `false`), the job
    prints a `::notice::`, sets `state=skipped` and builds nothing (the
    later steps check that output). Never add a token that reads a
    private game: a public repository's logs would show its code.
  - Its steps. Each cargo run ends in `|| true`: the report decides
    what failed, from cargo's own `build-finished` line. The JSON, the
    `.err` files and the log go in `$RUNNER_TEMP`.
    1. Check out this repository to `engine/` with `fetch-depth: 0`
       (the pin lag needs the history and `origin/main`), and the
       game's default branch to `game/`, both with
       `persist-credentials: false`. They are siblings, so neither's
       `.cargo/config.toml` applies to the other.
    2. The macroquad system libraries: if 2513 is done, `bash
       engine/scripts/setup-linux.sh` (it also installs the engine's
       pinned Rust; step 5 still picks the game's). If not, the same
       `apt-get` line as `ci.yml`'s jobs, and 2513 moves it to the
       script with the others (its test reads every workflow).
    3. `rustup show` in `engine/`, then `rust-cache` for `engine ->
       target` and `game -> target`.
    4. In `engine/`: `cargo xtask game-patch --game ../game --out
       "$RUNNER_TEMP/engine-patch.toml"`.
    5. In `game/`: `rustup show` (the game's `rust-toolchain.toml`
       picks the toolchain, as at a real pin bump), then `cargo check
       --workspace --all-targets --message-format=json --config
       "$RUNNER_TEMP/engine-patch.toml" > "$RUNNER_TEMP/head.json" 2>
       "$RUNNER_TEMP/head.err" || true`.
       - No `--locked`: the patch rewrites the game's `Cargo.lock`, in
         this checkout only.
       - No `RUSTFLAGS`. A path patch shows the engine crates' own
         warnings, which a real pin bump (a git dependency, its lints
         capped) never shows. So the report counts warnings itself
         (below).
       - Then, in `engine/`, `cargo xtask game-report --head
         "$RUNNER_TEMP/head.json" --game ../game --engine . --status`
         writes `broken=true` or `broken=false` to `$GITHUB_OUTPUT`.
    6. On a PR, only when step 5 found a break: `git -C engine checkout
       --detach HEAD^1` (the base's tip: a PR's checkout is its merge
       commit), the same check into `base.json` and `base.err`, then
       `git -C engine checkout -` back to the PR, so step 8 runs this
       PR's `xtask`.
    7. Nightly, or `nightly: true`, and only when step 5 found no
       break: `cargo test --workspace --no-fail-fast --config …`, its
       output and errors into `test.log`.
    8. In `engine/`: `cargo xtask game-report` with every file the
       steps wrote, `--summary "$GITHUB_STEP_SUMMARY"` and `--issue
       "$RUNNER_TEMP/issue.md"`. It writes `state=builds` or
       `state=broken` to `$GITHUB_OUTPUT`, and exits 0 either way.
    9. `actions/upload-artifact` uploads `issue.md`.
    10. The last step fails when `state` is `broken`, so the check
        shows red but blocks nothing.
  - **The `game-issue` job:** `needs: game`, `if: always()`, `permissions:
    contents: read, issues: write`. It runs no cargo, so the job that
    runs the game's build scripts never holds a write token. It reads
    `needs.game.outputs.state`, downloads `issue.md`
    (`actions/download-artifact`, pinned by commit like every action in
    this repository) and, on `main` (the schedule, or a dispatch with
    `nightly: true`), with `GH_TOKEN: ${{ github.token }}`:
    - on `broken`, opens one issue labelled `game-break`, titled "The
      game doesn't build on the engine's main" (the label made with
      `gh label create game-break --force`), or edits the open one;
    - on `builds`, closes the open one with a comment that links the
      run;
    - on `skipped`, or when `state` is empty (the `game` job was
      cancelled), does nothing.

    On another branch it prints what it would do and changes nothing.
- **`crates/xtask/src/game.rs`**, two commands added to `cargo xtask`'s
  `USAGE` in `crates/xtask/src/main.rs`:
  - **`game-patch --game <dir> --out <file>`:**
    - reads the game's `Cargo.lock` by hand (split on `[[package]]`,
      the way the game's `engine.rs` does; no TOML crate) and keeps the
      packages whose `source` starts with
      `git+https://github.com/Zafnok/shuyi-engine`;
    - reads this workspace's members from `cargo metadata --no-deps
      --format-version 1` (each `name` and the folder of its
      `manifest_path`);
    - writes `[patch."https://github.com/Zafnok/shuyi-engine"]` with
      `<name> = { path = "<absolute folder>" }` for each package in both
      lists. Absolute paths, so it doesn't matter where Cargo resolves a
      `--config` file's relative paths from.
  - **`game-report --head <json> [--base <json>] [--tests <log>] --game
    <dir> --engine <dir> --run-url <url> --summary <file> --issue
    <file>`**, and `--head <json> --game <dir> --engine <dir> --status`
    for step 5. Beside each `<name>.json` it reads `<name>.err`,
    cargo's stderr. The build rewrote the game's `Cargo.lock`, so it
    reads the lock as committed (`git show HEAD:Cargo.lock` in
    `--game`):
    - reads cargo's JSON lines (`"reason": "compiler-message"` and
      `"build-finished"`) with `serde_json` and splits the messages by
      `package_id`: a package under `--game` is the game's, one under
      `--engine` the engine's;
    - **a break** is any error, or a warning in a game package (the
      game's CI denies warnings, so a deprecation breaks it). Warnings
      in engine packages are dropped. Errors in engine packages are one
      group, "the engine doesn't build with the game's toolchain";
    - also a break: each engine package the game's lock names that this
      workspace no longer has (the same lists `game-patch` reads), and,
      when there is no `build-finished` line with `"success": true` and
      no error message (cargo stopped before compiling, for example on
      a `rust-version` the game's Rust is too old for), the last 20
      lines of the `.err` file;
    - **groups by engine item:** the first
      `shuyi_[a-z0-9_]+(::[A-Za-z0-9_]+)*` path in the message, its
      children or its span labels; else the error code (`E0599`); else
      `other`. A group shows the first line of each message and up to 5
      places (game file and line), and counts the rest. At most 30
      groups;
    - with `--base`, each group is `new` (only with this PR), `fixed`
      (only at the base) or `already` (both: `main` already breaks it);
    - with `--tests`, the failing tests from the `test <name> ...
      FAILED` lines, by name. A failing test is a break too;
    - **the pin lag:** the pin is the commit after `#` in the game's
      lock source for the engine. `git log --format='%h %s'
      <pin>..origin/main` in `--engine` lists the commits, with the
      ticket ids of their `[NNNN]` subjects. A pin not in the history
      is said so;
    - **the summary**, in Markdown: the game's commit; the engine's and
      the game's `rust-toolchain.toml` channels when they differ (the
      game moves its own at a pin bump, from the game ticket 2012's
      skill writes); the pin lag (a count, then each commit);
      then "The game builds on this change." or the groups; then the
      failing tests. Text from the compiler or from git goes in code
      spans and blocks, so no `@word` becomes a mention (`AGENTS.md`
      rule 11);
    - **the issue body** names only the `shuyi_…` groups, the counts,
      the pin lag and the run's link. The game's file names and
      messages stay in the run's summary and never go into this
      repository's issues (rule 4);
    - the `git` calls go through one function the tests replace.
  - **The dependency:** `serde_json = "1"` (MIT OR Apache-2.0, both in
    `deny.toml`'s allow list). It is in `Cargo.lock` already, through
    `shuyi-cli`, so no new crate enters the lock.
- **Text that changes:**
  - `run-gates` skill, *The game at the next pin*: CI's `game` workflow
    builds the game on each PR that touches the code and nightly on
    `main`, so read its summary first. The local `cargo xtask engine`
    stays, for fixing the game alongside an engine change.
  - `run-gates` skill, the ticket lint's paragraph (today lines 89–92,
    ending "CI has no access to the game repository"): the game is
    public, so CI's ticket lint checks `game#` ids too. Locally it
    needs `gh` or `--game-repo`.
  - `.github/workflows/ci.yml`, the `tickets` job's comment on
    `GH_TOKEN` (lines 279–280): the same.
  - `work-ticket` skill, §6, the bullet "If the game must change too":
    when the `game` job ran, the PR body names the engine items its
    report lists as `new`, or says the game builds, and links the run.
    It never copies the game's code or messages.
  - `AGENTS.md`, *Working with the game repository*, after "must not
    break the game at its next pin bump": one sentence saying CI's
    `game` workflow shows what breaks.

**Either order:**

- **With 2010 (done) and 2804** (the `Breaking:` lines). `work-ticket`
  §5 step 2 already asks for `Breaking:` lines (2010): that step also
  says each item the `game` report lists as `new` has one. If 2804 is
  done, its `migration` block is in the same step; keep one step. 2010's
  rewording of the `AGENTS.md` bullet and its pointer in `run-gates`'
  *The game at the next pin* stay; this ticket's sentences go beside
  them.
- **With 2513** (the Linux setup script): step 2 of the `game` job.
- **With 2011** (Dependabot, done): the `dependency-upkeep` skill's
  step 3 ("build the game against the branch") reads the `game` job's
  summary instead, and keeps the local build for fixing.

**For the game:** no API change, and no change in the game's
repository. This is CI, `xtask`, skill text and `AGENTS.md`. The
game's next pin bump can read the nightly summary for what to follow.

**Out (do not do):**

- Fixing the game or editing its repository (its pin, its `CRATES`
  list). A break the job finds becomes a game ticket, as `work-ticket`
  §6 says.
- Making `game` a required check, or adding it to `ci-result`'s
  `needs`. 2016's branch protection leaves it out too.
- Clippy, `wasm32`, `no_std` and Playdate builds of the game, and
  Windows or macOS runners.
- Private assets, and any secret.
- Public API diffs, semver checks and the `Breaking:` line's form:
  2010. The changelog that lists the lines: 2804.
- Keeping the pinned Rust in step: 2012 (the engine, the templates and
  `rust-version`, and the game ticket its skill writes). The game's own
  `rust-toolchain.toml` moves in the game's pin-bump PR. The report
  only names both channels.
- The Linux setup script and its package list: 2513.
- The game's move to crates.io versions (2005). This ticket reads only
  the git source. Once the game takes the engine from crates.io,
  `game-patch` needs a `[patch.crates-io]`: a ticket of its own then.
- Other games than Visions of Shuyi: a ticket of its own, once other
  public games depend on the engine.
- Editing done ticket 2002's notes: done tickets are history.

## Implementation steps

1. Write `crates/xtask/src/game.rs` with its unit tests, and the two
   commands in `main.rs`. The fixtures are inline strings in the tests,
   as `tickets.rs`'s are: a few made-up lines of cargo JSON for a
   made-up `demo-game` package, never output copied from the real game
   (rule 4).
2. Write `.github/workflows/game.yml`.
3. Push the PR. Its own run (it touches `crates/xtask`) should say
   "The game builds on this change." If `main` already breaks the game
   (the pin lags), it lists those groups as `already` instead: write
   the game ticket that fixes them, as `work-ticket` §6 says, and name
   it in the PR.
4. Push a throwaway commit that renames one public engine item the game
   calls (any `shuyi_core` function it uses), and its callers in the
   engine, so only the game breaks. Its run should show that item as
   the one `new` group. Revert the commit, and link both runs in the
   PR.
5. From the branch, dispatch with `gh workflow run game.yml --ref
   <branch> -f nightly=true`: the tests run, and `game-issue` prints
   the issue it would file. Dispatch again with `-f
   game_repo=Zafnok/does-not-exist`: the notice, and a green run. The
   PR's first run registers the workflow, so GitHub takes a dispatch
   before `game.yml` is on `main`. If it still refuses, say so in the
   PR and make both dispatches after the merge, from a scratch branch
   cut from `main`, where `game-issue` still only prints.
6. Make the text changes, and run the gates.

If the PR grows well past 600 changed lines, move the `game-issue` job
and the issue body to a follow-up ticket (`write-ticket`), and keep
the job summary here.

## Acceptance criteria

- [ ] `game.yml` ran on this ticket's PR, and its summary shows the
      game's commit, the pin lag with each commit, and "The game builds
      on this change." (or only `already` groups, with the game ticket
      that fixes them named in the PR).
- [ ] The throwaway rename showed its item as the one `new` group; both
      runs are linked in the PR, and the rename is reverted.
- [ ] A dispatch with `nightly: true` ran the game's tests, and
      `game-issue` printed the issue without filing it.
- [ ] A dispatch with a game repository that doesn't exist ended green
      with the notice, and `game-issue` did nothing.
- [ ] Only `game-issue` holds `issues: write`, and it runs no cargo.
      `ci-result`'s `needs` in `ci.yml` is unchanged.
- [ ] The issue body names no file or message of the game, and neither
      the issue body nor the summary has an `@` outside code (unit
      tests).
- [ ] The `run-gates` and `work-ticket` skills, `ci.yml`'s `tickets`
      comment and `AGENTS.md` say what Scope lists.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit, in `crates/xtask/src/game.rs`:
  - the lock: engine packages from the git source kept, crates.io and
    path packages left out, CRLF line endings;
  - the patch: written from a fake `cargo metadata` and a fake lock; a
    workspace member the game doesn't use left out; a package the game
    uses that the workspace lacks reported;
  - the messages: an error in a game package, a warning in a game
    package (a break), a warning in an engine package (dropped), an
    error in an engine package (its own group), no successful
    `build-finished` line and no JSON error (the `.err` tail), a clean
    build (no break);
  - `--status`: `broken=true` and `broken=false` lines, and the
    `state=` line of the full report;
  - grouping: a `shuyi_…` path in the message, in a child, in a span
    label; the error code fallback; the caps of 5 places and 30 groups;
  - `--base`: `new`, `fixed` and `already`;
  - `--tests`: failing test names from a made-up log, each a break;
  - the pin lag from a fake `git log`: ids from `[NNNN]` subjects, a
    commit with none, a pin not in the history;
  - the Markdown: no `@` outside code spans and blocks; the issue body
    has only `shuyi_…` keys.
- CI: the workflow's own runs, as the acceptance criteria list.

## Completion notes

*(Filled in by the session that completes the ticket.)*
