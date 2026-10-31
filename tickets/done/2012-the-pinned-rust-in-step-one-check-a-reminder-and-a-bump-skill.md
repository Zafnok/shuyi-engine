---
id: "2012"
title: "The pinned Rust in step: cargo xtask check-toolchain over the engine, the templates and rust-version, Dependabot's reminder of a new stable, and a bump-rust skill"
type: infra
milestone: MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-10-23
---

# 2012 — The pinned Rust in step: one check, a reminder and a bump skill

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8 ([`docs/decisions.md`](../../docs/decisions.md), the
last entry). It is one of the protections for the code that Nick put in
the MVP, before the first release (2005).

**What the benchmarks have:**

- Bevy keeps `rust-version` close to the latest stable and checks it:
  an `msrv` job in its CI reads `rust-version` with `cargo metadata`,
  installs that Rust, runs `cargo check`, and a failure is posted on the
  PR ([`ci.yml`](https://github.com/bevyengine/bevy/blob/main/.github/workflows/ci.yml)).
- Unreal and Unity list the compilers and SDKs each release supports
  (Visual Studio, Xcode, the Android NDK), so a game moves its
  toolchain when it moves the engine. A Rust project names its own Rust
  in `rust-toolchain.toml`, so the engine has to keep its copies in step
  itself.
- Cargo's reference asks a project to write down when it raises
  `rust-version` ([Rust version](https://doc.rust-lang.org/cargo/reference/rust-version.html)).
- Dependabot moves the channel in `rust-toolchain.toml` and
  `rust-toolchain` files since August 2025, as its `rust-toolchain`
  ecosystem
  ([changelog](https://github.blog/changelog/2025-08-19-dependabot-now-supports-rust-toolchain-updates/)).
  It reads exact versions (`1.xx.yy`, `1.xx`) and dated nightlies, for
  version updates only. GitHub's options reference lists
  `rust-toolchain` as the `package-ecosystem` value, `cooldown` for it,
  and `groups.<name>.group-by: dependency-name` to put one dependency's
  updates in several directories into one PR.

**What Shuyi has today** (checked 2026-10-21):

- `rust-toolchain.toml`: `channel = "1.98.1"`, `rustfmt` and `clippy`,
  and the `wasm32-unknown-unknown` target.
- `templates/blank/rust-toolchain.toml` and
  `templates/grid/rust-toolchain.toml`: `channel = "1.98.1"`, with the
  comment "The Rust the engine builds with; move it when the engine
  does." They don't list the wasm target.
- `rust-version = "1.98"` in `[workspace.package]` of the root
  `Cargo.toml` and of `templates/blank/Cargo.toml` and
  `templates/grid/Cargo.toml`. Every member crate inherits it
  (`rust-version.workspace = true` in `crates/*/Cargo.toml`,
  `examples/hello-grid/Cargo.toml` and `templates/*/crates/*/Cargo.toml`).
  No other file names a Rust version, apart from `shuyi doctor`'s test
  fixtures.
- `cargo xtask` (`crates/xtask/src/main.rs`) has `ticket-lint`,
  `check-links` and `check-no-std`. Nothing compares these files.
- CI's `templates` job (`.github/workflows/ci.yml`) makes a game with
  `shuyi new` and builds it on the template's pin against this checkout.
  Cargo refuses a package whose `rust-version` is above the Rust it runs
  on, so that job fails when the engine's `rust-version` rises above a
  template's pin. Nothing catches the other cases: the engine's pin
  moves while the templates and `rust-version` stay behind, or a
  template's pin is newer than the engine's.
- `shuyi doctor` (`crates/cli/src/contract/doctor.rs`) checks that
  `rustc` runs, not its version.
- There is no `.github/dependabot.yml`, and nothing in `docs/`, the
  skills or the tickets says who moves the pin or when.
- 2005's version-policy ADR covers semver, 0.x and one version for all
  crates. What `rust-version` promises after the first release belongs
  there, in a *Rust version* section, not here.
- 3020 (parked) pins a nightly for its old ARM builds, with no rule for
  moving it.

**Why it matters:**

- **The game first.** Visions of Shuyi pins Rust 1.98.1 too and
  depends on the engine by a git revision. When the engine raises
  `rust-version`, the game's next pin bump has to move the game's
  toolchain in the same PR, or cargo refuses to build. Nothing says so
  today.
- **The lints.** CI builds with `RUSTFLAGS: -D warnings` and clippy's
  `pedantic` group (the workspace's `[workspace.lints.clippy]`). Each
  stable can add lints. A pin left alone for a year moves in one large
  PR full of them; one release at a time, each bump is small.
- **New games.** A template pinned to another Rust than the engine's
  makes a `shuyi new` game build on a Rust the engine never tested. The
  end goal (2026-10-19: an agent makes a game in under an hour) needs a
  new game that builds the first time.
- **The first release.** After 2005, `rust-version` is a promise to
  every game on crates.io, and it has to be checked to be kept.

This ticket keeps the pins equal, reminds when a new stable is out, and
writes down how a session moves the pin. What `rust-version` promises
after the first release goes in 2005's ADR, and the `msrv` job waits
until `rust-version` is below the pin (Out).

## Nick input

None.

## Scope

**In:**
- **`cargo xtask check-toolchain`** (`crates/xtask/src/toolchain.rs`).
  It returns every disagreement, each naming the file and how to fix
  it, and passes when there is none. It reads:
  - the root `rust-toolchain.toml` and `Cargo.toml`;
  - every `rust-toolchain.toml` under `templates/`, at any depth, and
    every `Cargo.toml` there. Walking the folder, rather than reading
    `templates/<name>/`, keeps the check right when 2501 turns the two
    templates into parts of one. It skips `target` folders, sharing
    `links.rs`'s `SKIPPED_DIRS` rather than copying it. Finding no
    `rust-toolchain.toml` or no `Cargo.toml` with `[workspace.package]`
    under `templates/` is an error, so a move can't make it pass by
    reading nothing;
  - `crates/*/Cargo.toml` and `examples/*/Cargo.toml`;
  - `.github/dependabot.yml`.

  It fails when:
  1. the root `rust-toolchain.toml` has no `channel` under
     `[toolchain]`, or the channel isn't an exact stable release
     `X.Y.Z` (`stable`, `1.98` and `nightly-…` all fail: an exact pin is
     what makes every build use the same Rust);
  2. the `channel` of a template's `rust-toolchain.toml` differs from
     the root's. Only the channel is compared: the root also lists the
     wasm target and the templates don't;
  3. `rust-version` in `[workspace.package]` of the root `Cargo.toml`,
     or of a template's `Cargo.toml` that has that table, is not the
     pin's `X.Y` (`"1.98"` for `"1.98.1"`). Below the pin, the message
     says that nothing in CI builds on an older Rust, so a lower
     `rust-version` needs the `msrv` job first (the `bump-rust` skill).
     Above it, the message says cargo would refuse to build;
  4. a `Cargo.toml` with a `[package]` table (in `crates/*`,
     `examples/*` or under `templates/`) doesn't have
     `rust-version.workspace = true` in it, the form every member uses
     today;
  5. `.github/dependabot.yml` has no update with
     `package-ecosystem: rust-toolchain`, or that update's
     `directories` (or `directory`) leaves out `/` or the folder of a
     template's `rust-toolchain.toml` (as `/templates/<name>`), so a new
     or moved template can't be forgotten.
  A missing or unreadable file is an error, never a pass.
- **No new dependency.** The TOML files are read line by line: a
  `[table]` header, then `key = value` lines, where the key may be
  dotted (`rust-version.workspace`) and the value is a quoted string or
  `true`, with comments, blank lines and other tables skipped. That is
  all these files use. `dependabot.yml` is read with `serde_norway`,
  which `xtask` already has. If the line reader gets awkward, `toml`
  (MIT OR Apache-2.0, both allowed by `deny.toml`'s `[licenses]`) is
  the fallback; `cargo deny check` and `cargo machete` must still pass.
- **Where it runs:**
  - CI's `tickets` job (`.github/workflows/ci.yml`), as a step after
    `cargo xtask check-links`, so it runs on every PR, docs-only ones
    too. Update the job's comment ("The ticket lint, the Markdown links
    and typos") to name it;
  - the `run-gates` skill, *The repository's own checks*, as one line
    beside `check-links`;
  - `crates/xtask/src/main.rs`: the module doc list, `USAGE` and the
    match arm, as `check-no-std` has them; `crates/xtask/Cargo.toml`'s
    `description` names the check.
- **The reminder: Dependabot's `rust-toolchain` ecosystem.** One update
  in `.github/dependabot.yml`:
  ```yaml
  # The pinned Rust (ticket 2012): one PR moves the engine's and the
  # templates' rust-toolchain.toml; the bump-rust skill finishes it.
  - package-ecosystem: rust-toolchain
    directories: ["/", "/templates/blank", "/templates/grid"]
    schedule:
      interval: weekly
    cooldown:
      default-days: 14
    groups:
      rust-toolchain:
        group-by: dependency-name
        patterns: ["*"]
  ```
  The 14-day cooldown lets a quick point release come out first. Check
  each key against GitHub's options reference
  (`docs.github.com/en/code-security/dependabot/working-with-dependabot/dependabot-options-reference`)
  before writing it; whether `patterns` is needed beside `group-by` is
  the one point it leaves open. If grouping across directories doesn't
  work for this ecosystem, leave `groups` out: the skill then says what
  to do with one PR per folder. No new scheduled workflow is written for
  this.
- **With 2011 (done)** (Dependabot for crates and actions): add this
  update to its `.github/dependabot.yml`, put `check-toolchain` after
  its `check-pins` in the `tickets` job and in `run-gates`, and the
  skill points at ADR-0037 §5 for why a Dependabot PR is its own ticket.
  A `rust-toolchain` PR is finished with `bump-rust`, not
  `dependency-upkeep`.
- **Either order with 2010** (the public API diff): its `api` job pins
  a nightly date and the versions of `cargo-semver-checks` and
  `cargo-public-api` (consts in `crates/xtask/src/api.rs`, the same ones
  CI installs). If 2010 is done, the checklist gets a step: when the new
  stable needs it, move that date and those versions in the same PR. If
  not, 2010 adds that step to the skill.
- **The `bump-rust` skill** (`.claude/skills/bump-rust/SKILL.md`, with
  `name` and `description` frontmatter as the other skills have), the
  written rule for who moves the pin and when. It holds:
  - **When it triggers** (its `description`): a Dependabot PR that moves
    a `rust-toolchain.toml`, `check-toolchain` failing, or a request to
    update Rust.
  - **Who and when.** the agent moves the pin; it is a technical choice,
    so Nick isn't asked. The pin follows each stable: Dependabot opens
    the PR once a stable is 14 days old, and a session finishes it
    before the next stable is out (six weeks), so the new lints come
    one release at a time. A bump PR holds nothing else. On a new
    minor, Dependabot's PR fails `check-toolchain` (rule 3) until
    `rust-version` moves: that is the reminder working, not a fault.
  - **The checklist:**
    1. Move every pin on the Dependabot branch: Dependabot moved the
       channels; for a new minor, move `rust-version` in the root
       `Cargo.toml` and in every template's `Cargo.toml` with
       `[workspace.package]`. A point release (`1.98.1` to `1.98.2`)
       moves only the channels. `cargo xtask check-toolchain` passes.
    2. Read the release's notes on `blog.rust-lang.org` and clippy's
       `CHANGELOG.md` for new and newly warn-by-default lints. Run
       clippy and `cargo fmt --all --check` on the workspace and on a
       game made from each template, as CI's `templates` job does
       (`cargo run -p shuyi-cli --locked -- new <template> <folder>
       --engine-path .`; `cargo fmt --all` at the root doesn't reach
       `templates/`). Fix every new warning in the code, in this PR. A
       lint that is wrong for one line gets a local
       `#[allow(clippy::…)]` with a comment saying why, as the code
       already does; never a crate-wide allow (`AGENTS.md` rule 6).
    3. Run the `run-gates` skill on Windows with the GNU host and
       MSYS2, as Nick's machine builds. CI's Windows leg builds with
       MSVC, so only this run checks that the GNU link still works. The
       no-std build and the web build are part of it. A session that
       can't run on the GNU host says so in the PR, and the PR waits
       for that run.
    4. For a new minor, the PR body says: "The game needs Rust X.Y at
       its next pin bump." Write the game ticket that moves the game's
       `rust-toolchain.toml` (and its `rust-version`) in that pin bump,
       as `work-ticket` §6 says for any change the game must follow.
       Once 2014's job builds the game against engine PRs, it shows the
       game failing on its older Rust on this PR: that is expected, and
       the game ticket fixes it.
    5. If Dependabot opened one PR per folder, finish the root's and
       close the others with a comment naming it.
  - **What `rust-version` promises.** Until 2005's version ADR says
    otherwise, `rust-version` equals the pin's `X.Y`, and
    `check-toolchain` enforces it. If that ADR allows a lag, the PR
    that first sets `rust-version` below the pin adds an `msrv` CI job
    (a `cargo check --workspace --locked` on exactly that Rust, as
    Bevy's does) and changes `check-toolchain`'s rule 3 from "equal" to
    "at or below" in the same PR.
  - **Don'ts:** no nightly for the workspace (CI's dated nightlies are
    the exceptions: 2010's `api` job's moves by the step above, and
    2020's `FUZZ_TOOLCHAIN` in `fuzz.yml`, if 2020 is done, moves on
    its own by its comment), no other work in the bump
    PR, no `[workspace.lints]` change to pass a new lint. Turning a lint
    off for the whole workspace is a ticket of its own that says why.
- **Small text changes:**
  - a comment in the root `rust-toolchain.toml` naming the check and
    the skill;
  - one line in `AGENTS.md`, *Environment*: "The pinned Rust
    (`rust-toolchain.toml`) moves with the `bump-rust` skill.";
  - the `README.md` row for `.claude/skills/` adds "the pinned Rust" to
    what the skills cover.

**For the game:** no API change, and the game's toolchain doesn't move
in this PR. The first bump of a minor after it comes with a game ticket
(checklist step 4).

**Out (do not do):**
- the bump itself: the first Dependabot PR after this lands does it,
  under the skill;
- the `msrv` CI job: nothing needs it while `rust-version` equals the
  pin. The skill and 2005's ADR say when it comes;
- what `rust-version` promises after the first release: 2005's version
  ADR, in its *Rust version* section;
- `shuyi doctor` comparing `rustc` with `rust-version`: rustup installs
  the pin from `rust-toolchain.toml`, so a game builds on its pin;
- a Windows GNU leg in CI;
- the game's own `rust-toolchain.toml`: the game moves it in its
  pin-bump PR, from the ticket step 4 writes;
- moving a game made with `shuyi new` to a newer engine and Rust (2523);
- moving the templates: 2501 (parts of one template) and 2005 (if they
  move into a crate) add their new folders to the Dependabot update
  (rule 5 names each one), and 2005 changes the walked folder if it is
  no longer `templates/`;
- 3020's pinned nightly (parked): when it is unparked, 3020 adds its own
  section to the `bump-rust` skill and its folder to the Dependabot
  update;
- nightly features, and wgpu and winit upgrades (3034, ADR-0021 §1);
- Dependabot for crates and actions, its ADR and the
  `dependency-upkeep` skill (2011).

## Implementation steps

1. Write `crates/xtask/src/toolchain.rs`: `pub fn run(repo_root: &Path)
   -> Vec<String>`, with small functions for each rule
   (`channel_of(text)`, `rust_version_of(text)`, `major_minor(channel)`,
   the walk of `templates/`, the member manifests, the Dependabot
   directories), and its tests, in the shape of `no_std.rs`. Make
   `links.rs`'s `SKIPPED_DIRS` `pub(crate)` for the walk.
2. Wire it into `crates/xtask/src/main.rs` and the crate's
   `description`; run it on the repository: it fails only on rule 5
   until step 3.
3. Write `.github/dependabot.yml` (or add to 2011's), after checking
   the keys against the options reference. Run the check again: it
   passes.
4. Add the step to CI's `tickets` job and the line to the `run-gates`
   skill.
5. Write `.claude/skills/bump-rust/SKILL.md`, the comment in
   `rust-toolchain.toml`, the line in `AGENTS.md` and the README row.
6. Run the gates.

## Acceptance criteria

- [x] `cargo xtask check-toolchain` prints `check-toolchain: ok` on the
      repository, and the unit tests show it failing, with the file
      named, for each of the five rules.
- [x] CI's `tickets` job runs it on every PR, docs-only ones too.
- [x] `.github/dependabot.yml` has a `rust-toolchain` update for `/` and
      the folder of every template's `rust-toolchain.toml`, with weekly
      checks and a 14-day cooldown, and the PR description quotes
      GitHub's options reference for `directories`, `cooldown` and
      `groups` (or says why `groups` was left out).
- [x] `.claude/skills/bump-rust/SKILL.md` holds when, who, the checklist
      steps, the `rust-version` rule and the don'ts.
- [x] The `run-gates` skill lists `cargo xtask check-toolchain`, and
      `AGENTS.md` and `README.md` name the `bump-rust` skill.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`toolchain.rs`), on files written to a temporary folder as
  `no_std.rs`'s tests do, each asserting the exact message:
  - all files agree: no error;
  - the root channel `stable`, `1.98` or `nightly-2026-10-01`: rule 1;
  - one template's channel `1.97.0`: rule 2, naming that file only;
  - a template's `rust-version` `1.97`: rule 3; the root's `1.97`
    (below the pin) names the `msrv` job; `1.99` fails too;
  - a member `Cargo.toml` with `rust-version = "1.98"` written out:
    rule 4;
  - `dependabot.yml` without the update, and with a template folder
    left out: rule 5;
  - a template's `rust-toolchain.toml` one folder deeper
    (`templates/one/base/`): found, and rule 5 asks for
    `/templates/one/base`;
  - a missing root `rust-toolchain.toml`, a `templates/` folder with no
    toolchain file and a missing `dependabot.yml`: an error each;
  - a `target` folder under `templates/` holding a wrong
    `rust-toolchain.toml`: skipped;
  - the line reader: a `channel` key in another table, a commented-out
    line, spaces around `=`, and `rust-version.workspace = true` read as
    a dotted key.
- The real repository passes (`run(&crate::repo_root())` is empty), as
  `no_std.rs`'s `the_checked_crates_are_clean` does.
- CI mutation testing runs on the diff: the exact-message asserts are
  what kill its mutants.

## Completion notes

- `cargo xtask check-toolchain` (`crates/xtask/src/toolchain.rs`) checks
  the five rules and prints `check-toolchain: ok` on the repository. Its
  25 unit tests assert the exact message for each rule, the walk (one
  folder deeper, `target` skipped) and the line reader. It shares
  `links.rs`'s `SKIPPED_DIRS` (now `pub(crate)`) and `display_path`. No
  new dependency: the TOML files are read line by line, `dependabot.yml`
  with `serde_norway`.
- It runs in CI's `tickets` job after `check-links`, and is listed in the
  `run-gates` skill.
- `.github/dependabot.yml` is new, with the `rust-toolchain` update alone
  (2011 is not done; it adds its updates beside it). Deviation: no
  `patterns` beside `group-by: dependency-name`. GitHub's options
  reference doesn't require it, and its example for `group-by` in
  *Optimizing the creation of pull requests for Dependabot version
  updates* has none. The reference lists `rust-toolchain` as the
  `package-ecosystem` value and Rust toolchain in its cooldown table.
- The `bump-rust` skill holds who and when, the checklist, the
  `rust-version` rule and the don'ts. 2011 is not done, so it says to
  write a ticket for the bump first. 2010 and 2020 are not done either:
  their tickets now say to add their steps to the skill.
- Docs: the skill is the written rule; the root `rust-toolchain.toml`
  comment, `AGENTS.md` *Environment* and the README's `.claude/skills/`
  row point at it. No other page changes (the `document` skill: no public
  item, format or contract changes).
- Gates run on Windows with the GNU host: fmt, clippy, `xtask` tests and
  docs, deny, machete, and the repository's own checks. The web, no-std
  and template builds weren't rerun, because this change touches none of
  their code.
- For the game: no API change and no toolchain move.
