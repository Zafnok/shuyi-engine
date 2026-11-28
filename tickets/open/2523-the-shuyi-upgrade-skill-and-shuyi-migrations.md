---
id: "2523"
title: "The shuyi-upgrade skill: a game moved to a newer engine from the plan shuyi changes answers, and a check that every Breaking line has its migration block"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["2005", "2804", "1134", "2010", "2012", "3454"]
nick_input: none
completed:
---

# 2523 — The shuyi-upgrade skill and the migration check

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
- 2012 keeps the engine's `rust-toolchain.toml` and its templates'
  equal, and 3511 keeps the engine's root `rust-version` at or below
  them. 2012's `bump-rust` skill moves the engine's Rust. 2012 leaves moving a `shuyi new` game to a newer engine and Rust
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

**Amended 2026-11-28 (tickets 3453 and 3454).** This ticket first
planned its own op, `shuyi migrations`, reading a `migrations.json`
embedded in `shuyi-cli`, keyed by released version. 3453 publishes a
change feed on the site from every push to `main` (signatures before
and after, the `Breaking:` and `Behaviour:` lines, the `migration`
blocks, found by commit), and 3454's `shuyi changes` reads it from a
game's pin, a revision or a release, with the call sites of each
block's `find` strings. That op is this skill's plan, so this ticket no
longer writes an op, a JSON list or an embedded copy: it keeps the
check that every `Breaking:` line has its block, and the skill.

Until a release carries a `migration` block, the plan has none and the
check's tests use fake tickets. That is fine: both are ready when the
first break ships.

## Nick input

None. How often the engine releases, and which versions get fixes, are
asked in 2005's sign-off.

## Scope

**In:**
- **The migration check:**
  - `cargo xtask check-migrations` (`crates/xtask/src/migrations.rs`,
    wired in `crates/xtask/src/main.rs`: the module doc list, `USAGE`
    and the match arm, as `check-no-std` has them) reads every
    `migration` block in `tickets/done/*.md` and writes nothing.
  - It refuses (exit 1, each error with the ticket's file and the
    block's or line's line number):
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
  - CI's `tickets` job runs it (it reads only tickets, so docs-only PRs
    run it too), and the `run-gates` skill lists it under *The
    repository's own checks*.
  - **2804's parser:** 2804 (done) parses the blocks in
    `crates/xtask/src/changelog.rs`: `migrations(file, content)` returns
    each ticket's `Migration`s (`pub(crate)`, with `since`, `what`,
    `old`, `new`, `before`, `after` and `find`, and it already checks
    `since` and that each `find` string is in `before` and not in
    `after`). Reuse it; don't write a second one. 3453's feed reads the
    blocks with it too. The block's shape is in `tickets/README.md`,
    *Breaking changes*.
- **The skill, `skills/shuyi-upgrade/SKILL.md`,** in 1134's pack and its
  format (frontmatter, when to use it and when not, the steps, the ops
  it names):
  - **Use it** to move a game to a newer engine release or revision,
    and when the game's `engine-changes` issue (3454's workflow) asks
    for it. **Not** for an engine change the game is making itself (the
    game's own process for that), and not for packs (bake again,
    ADR-0023 §5).
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
       `shuyi-cli = "X.Y.Z"` line of `cargo search shuyi-cli`), the
       newest commit of `main` for a game that follows `main`, or the
       version or revision asked for.
    3. Before the pin moves, while the game still builds, run `cargo
       shuyi changes --to <target> --json` (3454): it reads the old pin
       from `Cargo.lock` and the change feed from the site. Keep the
       answer: it is the plan, and its `from` is the old pin. A game
       whose tools have no `changes` op (an engine from before 3454)
       installs the newest tools beside it, without touching it
       (`cargo install --locked shuyi-cli --root target/shuyi-upgrade`,
       the binary `target/shuyi-upgrade/bin/shuyi`, `shuyi.exe` on
       Windows), and runs that binary's `changes` in the game.
    4. Move the pin and the Rust. Every `shuyi-*` dependency line (in a
       game from the templates, the root `Cargo.toml`'s
       `[workspace.dependencies]`) goes to the target. For the target's
       Rust, read the engine's `rust-toolchain.toml` `channel` and its
       root `Cargo.toml`'s `rust-version` at the target
       (`https://raw.githubusercontent.com/Zafnok/shuyi-engine/<target>/rust-toolchain.toml`
       and `/Cargo.toml`; the templates' stay equal to the engine's pin,
       which is never below its `rust-version`: 2012, 3511). Where the
       game's are lower, raise them to those.
    5. Build with `cargo clippy --workspace --all-targets`. Fix each
       error from the plan: a `changed` API line by its `new`, a
       `removed` one by the entry's `Breaking:` line and its migration
       block's `before` and `after`. Fix each `deprecated` warning by its
       note now: the old name goes in a later release (2005's policy).
       Read each `Behaviour:` line and search the game's tests and
       content for what it names.
    6. Run `cargo shuyi upgrade` for the game's content files, then
       `cargo shuyi check` (only once 2517 is done; below).
    7. Run `cargo shuyi changes --from <old pin> --to <target>` again
       (its `--from` stands in for the lock, which now names the
       target): no migration of the plan may have a call site left.
       Then the game's gates.
    8. Report: the two pins, the Rust before and after, each entry
       done, each entry left with its call sites and why, each
       `Behaviour:` line and what the game did about it, and the gates'
       results. A `partial` entry with no call site and no build error
       is done.
  - Its worked example, `skills/shuyi-upgrade/example.ron`, in the form
    1134 set (the steps of `crates/xtask/src/script.rs`, which CI's
    `skills` job runs with `cargo xtask skill-examples`): a fresh game
    from `shuyi new`, in which `changes --feed <a fixture feed whose
    newest commit is the game's pin> --json` answers `ok`. It is no
    module's skill, so `shuyi new` copies it into every new game.
  - **Either order with 2517:** if 2517 is done, step 6 is in; if not,
    leave step 6 out, and 2517 adds it when it lands. `check-skills` (1134) fails a skill that names an op the
    contract doesn't have.
- **The templates' `AGENTS.md`** (`templates/shuyi-default/base/AGENTS.md`):
  one line under the table of tools naming the `shuyi-upgrade` skill
  for moving to a newer engine (3454 adds the `cargo shuyi changes`
  row).
- **The `work-ticket` skill:** under 2804's step for the `Breaking:` line
  and its `migration` block, "then run `cargo xtask check-migrations`".
- **A short ADR** (the next free number in `docs/adr/`):
  - a breaking change's migration entry is written once, in the
    ticket's completion notes, in the PR that breaks (2804's block),
    and `cargo xtask check-migrations` refuses a `Breaking:` line
    without one;
  - a game's plan is `shuyi changes`, read before the pin moves, from
    the change feed (3453, 3454): one source for every game, whatever
    its pin;
  - the game's Rust moves with the engine, to at least its
    `rust-version`;
  - an entry applies where its `find` strings occur in the game's code,
    or where the compiler names its API line; pins only narrow the
    list.
  It extends 2005's version-policy ADR and 3454's ADR.
- **For the game:** nothing changes in its files or the library API.

**Out (do not do):**
- The op that reads the feed, its MCP tool and the template's weekly
  workflow: 3454. The feed: 3453.
- A list of migrations embedded in `shuyi-cli` (`migrations.json`) and
  a `shuyi migrations` op: the feed replaces both (amended
  2026-11-28).
- A `templates` job step that runs the plan over a fresh game: the job
  already builds each template against the PR's engine with clippy's
  warnings denied, so an old name left in a template fails it, a
  deprecated one too.
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

**Estimated size:** about 700 lines (code 250, tests 250, the skill and
docs 200).

## Implementation steps

1. Read 2005's version-policy ADR, 2804's `crates/xtask/src/changelog.rs`
   and the block's shape in `tickets/README.md`, 2010's
   `breaking_lines`, ADR-0022, 1134's skill format and worked examples,
   2012's `bump-rust` skill, and 3453's and 3454's ADRs and
   `crates/cli/src/contract/changes.rs`.
2. Write `cargo xtask check-migrations`, its checks and its tests. List
   the tickets for `BEFORE_BLOCKS` (`tickets/done/` files with a
   `Breaking:` line and no block). Add it to CI's `tickets` job, and the
   lines to the `run-gates` and `work-ticket` skills.
3. Write the skill and its worked example. Add the templates'
   `AGENTS.md` line.
4. Write the ADR and its row in `docs/adr/README.md`. Run the gates.

## Acceptance criteria

- [ ] `cargo xtask check-migrations` passes on the branch, and its unit
      tests show it failing, with the file and line, on a block whose
      `since` is above the workspace version, on a `find` string missing
      from its `before`, and on a `Breaking:` line with no block in a
      ticket outside `BEFORE_BLOCKS`.
- [ ] CI's `tickets` job runs `cargo xtask check-migrations`.
- [ ] `check-skills` passes on `skills/shuyi-upgrade`, its steps run
      `cargo shuyi changes` before the pin moves and include reading and
      raising the game's Rust, and its worked example runs green in CI.
- [ ] The ADR is written and listed in `docs/adr/README.md`.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `cargo xtask check-migrations`: each refusal (the `Breaking:`
  line with no block among them, and a ticket in `BEFORE_BLOCKS`
  skipped), and a ticket with two `Breaking:` lines and two blocks
  passing.
- CI: the `tickets` job's new step; the skill's worked example.

## Completion notes

*(Filled in by the session that completes the ticket.)*
