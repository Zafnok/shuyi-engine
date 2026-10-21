---
id: "2523"
title: "The shuyi-upgrade skill: a game moved to a newer engine, with shuyi migrations listing each breaking change since its version and where its code still uses the old names"
type: feature
milestone: After MVP
model: opus-5.5
effort: high
status: todo
blocked_by: ["2005", "2804", "1134", "2010", "2012"]
nick_input: none
completed:
---

# 2523 — The shuyi-upgrade skill and shuyi migrations

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8. When a release breaks a project's code, the others help
the project across:

- Bevy publishes a migration guide for each release, with before and
  after code; each entry is drafted in the PR that breaks
  (`bevy.org/learn/migration-guides/`).
- Unity's API Updater rewrites calls to APIs marked obsolete with an
  upgrade rule when a project opens in a newer editor
  (`docs.unity3d.com/Manual/APIUpdater.html`).
- Godot converts a 3.x project after a dry run
  (`--validate-conversion-3to4`), and its release policy says which
  versions get fixes
  (`docs.godotengine.org/en/stable/about/release_policy.html`).
- Unreal marks an API deprecated with a macro that names the version it
  went out of date in, and removes it some releases later.
- Cargo's SemVer guide says to mark a renamed item deprecated and keep
  the old name with a `pub use` before removing it
  (`doc.rust-lang.org/cargo/reference/semver.html`).

What Shuyi has (read on 2026-10-21):

- No `#[deprecated]` anywhere in `crates/`. 2226 plans the first ones
  (`Lang::english` kept one release as a deprecated alias of
  `Lang::source_text`).
- 2005's version-policy ADR (semver, 0.x, "a note in the changelog, a
  pin bump in the game") and 2804's `Breaking:` lines, listed first in
  the changelog. On 2026-10-20 this comparison added to both:
  deprecate where a forwarder is cheap, fix the latest release only,
  and put a `migration` block under each `Breaking:` line (YAML in a
  fenced block: `since`, `what`, `old`, `new`, `before`, `after`, and
  an optional `find`, the strings that find a call site). A block's
  `since` is the workspace version after the PR that breaks. 2804
  parses the blocks and gathers them into a migration guide per
  release.
- 2010 shows each PR's API change in CI and requires a `Breaking:` line
  when it breaks; its `breaking_lines` (`crates/xtask/src/tickets.rs`)
  reads them. Its Out leaves checking that a `migration` block is under
  each line to this ticket.
- The `work-ticket` skill (§3, *Public API*) asks for a version bump
  and a changelog line for a breaking change after the first release,
  and (§5) for the game PR that moves the pin. The game follows each
  break by hand, through its own `engine-change` skill and `cargo xtask
  engine-pin`.
- ADR-0005 §3 lists the 15 skills of 1134's pack. None moves a game to a
  newer engine (`shuyi-ship` is the nearest, and it builds and ships).
  1134's `shuyi new` copies the pack into each new game.
- 2517 plans `shuyi upgrade`, which rewrites a game's content files when
  a format renames a field. It leaves the game's Rust code to this
  ticket.
- 2012 keeps the engine's `rust-toolchain.toml`, its templates' and
  `rust-version` equal, and its `bump-rust` skill moves the engine's
  Rust. It leaves moving a `shuyi new` game to a newer engine and Rust
  to this ticket.
- `cargo shuyi` is an alias that runs the game's own tool program
  (`templates/grid/.cargo/config.toml`). It can't run while the game
  doesn't build on the new engine, which is when an upgrade needs it.
  The `shuyi` binary of `shuyi-cli` runs `new`, `doctor` and `schema`
  with no game (`contract::cli::without_project`,
  `crates/cli/src/contract/cli.rs:71`), and `shuyi-cli` doesn't depend
  on `shuyi-app`, so `cargo install shuyi-cli` builds no window library.
- `shuyi new` makes a game follow the engine's `main` branch by default
  today (`Engine::Main`, `#[default]` at
  `crates/cli/src/contract/new.rs:79`, its limit at line 309). 2005
  makes new games name the released version, so by the time this
  ticket starts a new game's `Cargo.lock` names a release. A game on a
  git revision or an engine checkout is still handled (below).

Why it matters:

- Visions of Shuyi moves to released versions with 2005. From then on
  its pin bump crosses breaks that other tickets made, not only the one
  its own session just wrote. A list of the changes since its version,
  with the lines of its code that still use the old names, makes that
  bump a checklist.
- A game made with `shuyi new` is made by people, or agents, who never
  read a changelog. The end goal (2026-10-19, `docs/decisions.md`) is a
  game an agent makes in under an hour; that game should also cross
  the engine's next release with one skill run.

Until a release carries a `migration` block, the op's list is empty and
its tests use a fake list. That is fine: the tool is ready when the
first break ships.

## Nick input

None. How often the engine releases, and which versions get fixes, are
asked in 2005's sign-off.

## Scope

**In:**
- **The entries, shipped with the tools:**
  - `cargo xtask migrations` (`crates/xtask/src/migrations.rs`, wired
    in `crates/xtask/src/main.rs`: the module doc list, `USAGE` and the
    match arm, as `check-no-std` has them) reads every `migration`
    block in `tickets/done/*.md` and writes `crates/cli/migrations.json`:
    one entry per block, `{ ticket, since, what, old, new, before,
    after, find }`, sorted by `since` as a version, then by `ticket`,
    then by the block's order in its file; pretty JSON with a trailing
    newline.
  - It refuses (exit 1, writing nothing, each error with the ticket's
    file and the block's or line's line number):
    - a block whose `since` is not `X.Y.Z`, or is above the workspace
      `version` in the root `Cargo.toml`;
    - a block with an empty `what`, `old`, `new`, `before` or `after`
      (`find` may be absent or empty);
    - a `find` string missing from its `before` or present in its
      `after`;
    - a `Breaking:` line (found as 2010's `breaking_lines` finds them)
      with no `migration` block after it, before the next `Breaking:`
      line or the end of the file. The tickets in `tickets/done/` that
      have a `Breaking:` line and no block when this ticket is worked
      are listed by id in a const, `BEFORE_BLOCKS`, and skipped; any
      other ticket fails. This is the check 2010 leaves to this ticket.
  - `cargo xtask migrations --check` writes nothing and fails when the
    file differs from what it would write. CI's `tickets` job runs it
    (it reads only tickets, so docs-only PRs run it too), and the
    `run-gates` skill lists it under *The repository's own checks*.
  - **2804's parser:** 2804 parses the blocks in
    `crates/xtask/src/changelog.rs`. Reuse that parser (make it
    `pub(crate)` if it isn't); don't write a second one. If 2804 landed
    without one, write it here and make the changelog use it, and if
    `tickets/README.md` lacks the block's shape, document it there.
  - `serde_json` (MIT OR Apache-2.0, already in `Cargo.lock` through
    `shuyi-cli`; `deny.toml` allows both licenses) and `serde` with
    `derive` join `crates/xtask/Cargo.toml` if 2804 didn't add them, and
    its `description` names the new command.
- **`shuyi migrations [--from <version>] [--json]`**, a contract op
  (`crates/cli/src/contract/migrations.rs`, `Op::Migrations`) that needs
  no game program:
  - it is in `contract::cli::without_project`, beside `new`, `doctor`
    and `schema`, so any version's `shuyi` binary runs it in a game that
    doesn't build;
  - its list is `include_str!` of `crates/cli/migrations.json`, so each
    version of the tools knows every entry up to itself;
  - `pub fn migrations(cwd: &Path, input: &MigrationsInput, list:
    &[Entry], to: &str) -> Answer<MigrationsOutput>` takes the list and
    the tools' version as arguments, so tests pass a fake list and
    version; the CLI passes the embedded list and
    `env!("CARGO_PKG_VERSION")`;
  - **from:** the engine's version in the game's `Cargo.lock` (in the
    first folder from the working folder up that has one): the
    `version` of each `[[package]]` named `shuyi-<crate>` for a crate of
    `new::ENGINE_CRATES` (`crates/cli/src/contract/new.rs:37`). Its
    `source` line says how the game gets the engine: `registry+` or
    `sparse+` (a release), `git+` (a revision), or none (a checkout by
    path). Two engine crates at different versions is an error. `--from`
    overrides the lock's version;
  - **to:** this tool's own version;
  - **the entries:** each one with `from <= since <= to`. The ones with
    `since == from` are marked `maybe_done`: a release has them all, and
    a revision inside that version's window may have some. They are
    listed anyway, because a deprecated old name still builds and its
    call sites still show;
  - **call sites:** for each entry with `find` strings, every place one
    of them occurs in the game's `*.rs` and `Cargo.toml` files: the file
    from the root, `/`-separated, the 1-based line and column (in
    characters), and the line's text. The walk starts at the folder of
    `Cargo.lock` (or the working folder, when `--from` stands in for a
    missing lock) and skips `target/`, every folder whose name starts
    with `.`, and every folder below the root whose `Cargo.toml` has a
    `[workspace]` of its own (another project, such as an engine
    checkout kept inside the game);
  - `MigrationsInput { from: Option<String> }`;
    `MigrationsOutput { from: String, from_source: Option<EngineSource>,
    to: String, entries: Vec<Migration> }` (no source when `--from`
    stands in for a missing lock); `EngineSource` is `Release`,
    `Revision` or `Checkout`; `Migration { ticket, since, what, old, new,
    before, after, find: Vec<String>, maybe_done: bool, found:
    Vec<CallSite> }`; `CallSite { file, line, column, text }`;
  - `ok` when no entry has a call site;
  - **limits:** an entry with no `find` strings ("`Lang::english`: no
    strings find it; the compiler finds its call sites once the pin
    moves"); `to == from` from a release ("these are the tools of the
    game's own engine: run the newer version's `shuyi` to see what it
    changes"); each folder skipped as another project;
  - **errors:** no `Cargo.lock` and no `--from` (the message names
    `--from`); a version that isn't `X.Y.Z`; `from` above `to` ("the
    game's engine, 0.5.0, is newer than these tools, 0.4.0");
  - the text answer: a first line such as `the engine: 0.2.0 (a
    revision) to 0.4.0`, then a line per entry, `0.3.0 2226: <what>`,
    with `(may be done)` after a `maybe_done` one, and its call sites
    below it as `crates/game/src/lib.rs:12:9: <text>`;
  - the op is in `Op`, `Op::ALL`, `Op::name`, `schemas()`
    (`crates/cli/src/contract.rs`), the op table in that file's module
    docs, the usage in `crates/cli/src/lib.rs`, the lines that say which
    ops need no project (the module docs of `crates/cli/src/lib.rs` and
    `crates/cli/src/contract/cli.rs`, and the usage's "new, doctor and
    schema need none"), and `docs/contract.md`'s op table (needs: "a
    `Cargo.lock`, or `--from`");
  - `contract::VERSION` goes up by one, the schemas are written again
    (`crates/cli/schemas/migrations.input.json` and
    `migrations.output.json`), and `crates/cli/schemas/versions.json`
    gets the new fingerprint (ADR-0022 §5);
  - **1133's tools:** 1133 is done by then (2005 waits for it). If its
    tools come from `Op::ALL`, `migrations` is a tool with no more work;
    if it lists them by hand, add it.
- **The skill, `skills/shuyi-upgrade/SKILL.md`,** in 1134's pack and its
  format (frontmatter, when to use it and when not, the steps, the ops
  it names):
  - **Use it** to move a game to a newer engine release or revision.
    **Not** for an engine change the game is making itself (the game's
    own process for that), and not for packs (bake again, ADR-0023 §5).
  - **The game's Rust moves with the engine.** Moving a game to a newer
    engine also moves the game's `rust-toolchain.toml` channel and its
    `rust-version` to at least the new engine's `rust-version`, because
    cargo refuses an engine whose `rust-version` is above the game's
    Rust. The engine's side of the same move is the `bump-rust` skill
    (2012).
  - The steps:
    1. Start on a branch from a clean working tree. Run the game's gates
       (its `AGENTS.md`) and note what already fails.
    2. Pick the target: the newest release (the version on the
       `shuyi-cli = "X.Y.Z"` line of `cargo search shuyi-cli`), or the
       version or revision asked for.
    3. Install the target's tools beside the game, without touching it:
       `cargo install --locked shuyi-cli --version <to> --root
       target/shuyi-upgrade`, or for a revision `cargo install --locked
       --git https://github.com/Zafnok/shuyi-engine --rev <rev>
       shuyi-cli --root target/shuyi-upgrade`. The binary is
       `target/shuyi-upgrade/bin/shuyi` (`shuyi.exe` on Windows).
    4. In the game, before the pin moves, run that binary's `migrations
       --json`: it reads the old version from `Cargo.lock`. Keep the
       answer: it is the plan, and its `from` is the old version.
    5. Move the pin and the Rust. Every `shuyi-*` dependency line (in a
       game from the templates, the root `Cargo.toml`'s
       `[workspace.dependencies]`) goes to the target. For the target's
       Rust, make a scratch game with the binary
       (`target/shuyi-upgrade/bin/shuyi new blank
       target/shuyi-upgrade/fresh`) and read its `rust-toolchain.toml`'s
       `channel` and its `Cargo.toml`'s `rust-version` (2012 keeps them
       equal to the engine's). Where the game's are lower, raise them
       to those.
    6. Build with `cargo clippy --workspace --all-targets`. Fix each
       error from the entry whose `old` or `find` names it, by its
       `before` and `after`. Fix each `deprecated` warning by its note
       now: the old name goes in a later release (2005's policy).
    7. Run `cargo shuyi upgrade` for the game's content files, then
       `cargo shuyi check` (only once 2517 is done; below).
    8. Run the binary's `migrations --from <old version> --json` again
       until it answers `ok`, then the game's gates.
    9. Report: the two versions, the Rust before and after, each entry
       done, each entry left with its call sites and why, and the
       gates' results. A `maybe_done` entry with no call site is done.
  - Its worked example, in the form 1134 set: a fresh game from `shuyi
    new grid`, in which `migrations --from 0.1.0 --json` answers `ok`
    (a fresh game uses no old name).
  - **Either order with 2517:** if 2517 is done, step 7 is in; if not,
    leave step 7 out, and 2517 adds it when it lands. `check-skills` (1134) fails a skill that names an op the
    contract doesn't have.
- **CI's `templates` job** (`.github/workflows/ci.yml`) runs `cargo
  shuyi migrations --from 0.1.0` in each fresh game after its gates, so
  a breaking PR that leaves an old name in a template fails.
- **The templates' `AGENTS.md`** (`templates/grid`, `templates/blank`,
  or wherever 2005 moved them): a row in the table of tools, "`cargo
  shuyi migrations --from 0.1.0` | the engine's breaking changes since
  0.1.0 whose old names the game still uses", and one line under the
  table naming the `shuyi-upgrade` skill for moving to a newer engine.
- **The `work-ticket` skill:** under 2804's step for the `Breaking:` line
  and its `migration` block, "then run `cargo xtask migrations`".
- **A short ADR** (the next free number in `docs/adr/`):
  - a breaking change's migration entry is written once, in the
    ticket's completion notes, in the PR that breaks (2804's block),
    and `cargo xtask migrations` refuses a `Breaking:` line without one;
  - a generated copy ships inside `shuyi-cli`, checked equal in CI, so
    every version of the tools carries every entry up to itself;
  - a game crosses versions with the target version's own `shuyi`,
    installed beside it, because the game's `cargo shuyi` can't build
    until the move is done;
  - the game's Rust moves with the engine, to at least its
    `rust-version`;
  - an entry applies where its `find` strings occur in the game's code;
    versions only narrow the list.
  It extends 2005's version-policy ADR and ADR-0022.
- **For the game:** nothing changes in its files. The library API gains
  a variant of `Op`, a break for a game that matches on it unless `Op`
  is `#[non_exhaustive]` by then (2013). If 2010's job reports it, write
  the `Breaking:` line and its `migration` block (the new arm; no
  `find`, the compiler finds it), and run `cargo xtask migrations`; the
  game's PR that moves the pin follows.

**Out (do not do):**
- The deprecation rule, the fixes-to-the-latest-release rule and what a
  breaking change needs: 2005's version-policy ADR.
- The `migration` block's shape and the per-release migration guide on
  the site and in `CHANGELOG.md`: 2804.
- The `Breaking:` line itself and CI's check that a breaking PR has one:
  2010.
- The engine's own Rust bump and `check-toolchain`: 2012's `bump-rust`
  skill.
- An automatic rewriter of Rust code, like Unity's API Updater: a
  research ticket at most, once real entries show which changes are
  mechanical.
- Fixes for older releases, backports or an LTS (2005's policy: the
  latest release only).
- A game's content files: 2517's `shuyi upgrade`.
- Saves and records: 2110 and 2111.
- Compiling the `before` and `after` snippets: they are fragments.
- Changing the game's `engine-change` skill or `cargo xtask engine-pin`:
  the game's own ticket, if it wants the op.

## Implementation steps

1. Read 2005's version-policy ADR, 2804's `crates/xtask/src/changelog.rs`
   and the block's shape in `tickets/README.md`, 2010's
   `breaking_lines`, ADR-0022, 1134's skill format and worked examples,
   2012's `bump-rust` skill, and `crates/cli/src/contract/cli.rs`,
   `new.rs` and `doctor.rs`.
2. Write `cargo xtask migrations`, its checks, its `--check` and its
   tests. List the tickets for `BEFORE_BLOCKS` (`tickets/done/` files
   with a `Breaking:` line and no block). Run it to write
   `crates/cli/migrations.json`. Add `--check` to CI's `tickets` job,
   and the lines to the `run-gates` and `work-ticket` skills.
3. Write the op: the lock reader, the version compare, the walk, the
   answer and its text. Add it to `Op`, `Op::ALL`, `schemas()`,
   `without_project`, the usage and the module docs. Write its tests
   over a scratch project (made under `std::env::temp_dir()`, as
   `crates/cli/src/contract/import/tests.rs` makes one) with a fake list.
4. Regenerate the schemas and `versions.json`.
5. Check the diff's size. If steps 1 to 4 are already past about 600
   changed lines (the schemas, `migrations.json` and snapshots aside),
   do step 7 in this PR and leave step 6 to a follow-up ticket, written
   with the `write-ticket` skill and blocked by this one; say so in the
   Completion notes, and tick the criteria about the skill, the
   templates and CI's `templates` job only in that ticket.
6. Write the skill and its worked example. Change the templates'
   `AGENTS.md` and CI's `templates` job.
7. Write the ADR, its row in `docs/adr/README.md`, and
   `docs/contract.md`'s row. Run the gates.

## Acceptance criteria

- [ ] `cargo xtask migrations --check` passes on the branch, and its
      unit tests show it failing, with the file and line, on a block
      whose `since` is above the workspace version, on a `find` string
      missing from its `before`, on a `Breaking:` line with no block in
      a ticket outside `BEFORE_BLOCKS`, and on a stale
      `migrations.json`.
- [ ] CI's `tickets` job runs `cargo xtask migrations --check`.
- [ ] Over a scratch game whose `Cargo.lock` names the engine at 0.2.0
      from git, with a fake list at 0.2.0, 0.3.0 and 0.5.0 and the tools
      at 0.4.0, the answer lists the 0.2.0 entry (`maybe_done`) and the
      0.3.0 one, not the 0.5.0 one. Each call site has the right file,
      line and column. Matches under `target/`, `.git/` and a nested
      folder with its own `[workspace]` are not listed. `ok` is false;
      after the call sites are edited to the `after` form, `ok` is true.
- [ ] The same game with the engine from a release (`registry+`) or a
      path answers `from_source` `Release` or `Checkout`, and
      `--from 0.3.0` drops the 0.2.0 entry.
- [ ] No `Cargo.lock` and no `--from` fails with a message naming
      `--from`; `--from 0.6.0` with the tools at 0.4.0 fails.
- [ ] The embedded `crates/cli/migrations.json` parses as the op's list
      (a unit test).
- [ ] CI's `templates` job runs `cargo shuyi migrations --from 0.1.0`
      green on both templates.
- [ ] `check-skills` passes on `skills/shuyi-upgrade`, its steps include
      reading and raising the game's Rust, and its worked example runs
      green in CI.
- [ ] The schemas and `versions.json` match the new `contract::VERSION`.
- [ ] The ADR is written and listed in `docs/adr/README.md`.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `cargo xtask migrations`: parsing blocks, each refusal (the
  `Breaking:` line with no block among them, and a ticket in
  `BEFORE_BLOCKS` skipped), sorting by version (0.10.0 after 0.9.0),
  `--check` on a stale file. The op: reading `Cargo.lock` (a release, a
  revision, a checkout, two engine crates disagreeing, no engine
  crate); comparing versions; the walk's skips; the column on a line
  with non-ASCII text before the match.
- Integration: the scratch game above, through `migrations` and through
  the CLI with `--json`; the embedded list parses.
- CI: the `tickets` job's new step; the `templates` job's new line; the
  skill's worked example.

## Completion notes

*(Filled in by the session that completes the ticket.)*
