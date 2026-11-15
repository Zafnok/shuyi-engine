---
id: "3402"
title: "New games keep their worktrees small: the compression hook in the template, `repos` setting git's hooks, and `doctor` saying when either is missing"
type: infra
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-14
---

# 3402 — New games keep their worktrees small: the compression hook in the template, `repos` setting git's hooks, and `doctor` saying when either is missing

## Context

Nick, 2026-11-14: once the worktree track is done, the template should
ship it. The engine is meant to be driven by coding agents, so the
developers of other games will use Codex or Claude Code too, each in
many worktrees. The disk work done here for the engine's own worktrees
should reach them as well.

That track is 2074–2079. Done so far: 2074 (one-off gate builds keep
no incremental cache), 2075 (compression), 2077 (strip debug info) and
2079 (the aliases). Open: 2076 (trim idle worktrees) and 2078 (seed a
new worktree from a warm one). None of it reaches a game today:

- **Compression (2075).** The engine's `scripts/git-hooks/post-checkout`
  compresses a new worktree's `target/` with NTFS, which takes about
  half the disk at no measured cost in build time. The game template
  (`templates/shuyi-default/base`) has no `scripts/git-hooks`. Its
  `scripts/setup-linux.sh` (the block at line 109) already sets `core.hooksPath`
  to `scripts/git-hooks`, but only when that folder exists. And nothing
  sets it on Windows, which is the only place the hook does anything.
  The engine's `cargo xtask gates` compresses a `target/` that lacks
  the attribute. A game has no `xtask`, so nothing tells its developer.
- **Who sets the hooks.** `shuyi new` doesn't run `git init`;
  `shuyi repos` does (`crates/cli/src/repos.rs`, `make`, `Thing::Git`).
  So `repos` is where a new game's checkout first exists.
- **Debug info (2077).** The template's `[profile.dev]` and
  `[profile.dev.package."*"]` already have `debug = false` (ADR-0049),
  and cargo strips debug info by default when none is asked for. So
  2077's `strip = "debuginfo"` probably adds nothing for a game. This
  ticket checks that, rather than trusting it.
- **The alias (2079).** 2079 measured that `cargo run -p <package>`
  builds units the workspace build doesn't (7 for `-p shuyi-cli`). It
  kept `-p`, because `--bin` fails from a member's folder. The
  template's alias is `shuyi = "run -q -p {{crate_name}} --example tools
  --features tools --"`; nobody has counted its units.

Trimming and seeding a game's worktrees are 3403's job. They wait on
2076 and 2078.

ADR-0042 and [`docs/templates.md`](../../docs/templates.md) say "no
template runs code". That means `shuyi new` runs nothing while it makes
a game. A hook is a file `new` writes; git runs it later, and only once
`core.hooksPath` is set. So the rule holds, and the page says so.

## Nick input

None.

## Scope

**In:**
- `templates/shuyi-default/base/scripts/git-hooks/post-checkout`: the
  engine's hook (2075), with its comments written for a game. A game
  has no `cargo xtask gates`: the comment names `cargo shuyi doctor`,
  which reports a `target/` without compression.
- `shuyi repos`: after `git init` (and on a re-run, when the checkout
  exists but `core.hooksPath` is unset), set it to `scripts/git-hooks`
  if that folder exists. Add the hook to the first commit with mode
  `100755` (`git update-index --chmod=+x`), so a clone on Linux or macOS
  doesn't warn. Off Windows the hook does nothing.
- `doctor`: two new optional checks, which never fail it (like `gh` and
  `ssh-keygen`):
  - **git hooks:** in a git checkout whose root has `scripts/git-hooks`,
    whether `core.hooksPath` names it. The help is the one command that
    sets it.
  - **target compression** (Windows only): whether the project's
    `target/` carries NTFS compression. The help is the `compact` line
    the hook runs. It reads the file attribute and never compresses
    anything itself.
- Count the template alias's units on a new game: `cargo test
  --workspace`, then `cargo shuyi check`, counting `Compiling` lines
  (2079's method). Change the alias only if a form builds fewer
  third-party units and still runs `cargo shuyi` from every folder of
  the game, the game's member crates included. Otherwise record the
  count in the notes and keep the alias.
- Check that the template's test programs carry no debug info
  (`objdump -h` shows no `.debug_*` section). If they do, add
  `strip = "debuginfo"` to the template's `[profile.dev]`, inside the
  compile-time budget (ADR-0049's CI job).
- Docs, by the `document` skill:
  - the template's `AGENTS.md` and `README.md` (the hook, and setting
    git's hooks once per clone);
  - `docs/templates.md`'s "no template runs code" line (what a hook
    file is);
  - `docs/contract.md`'s `doctor` checks.

**Out (do not do):**
- A pre-push hook running a game's gates. It isn't disk work, and
  whether a game wants one is its own question.
- Trimming idle worktrees and seeding new ones in a game (3403).
- `shuyi new` running `git init` or `git config` (ADR-0042: `new` only
  writes files).
- Compressing from `doctor`, or from anything but the hook.

**Estimated size:** about 400 lines (code 120, tests 180, docs 100).

## Implementation steps

1. Copy `scripts/git-hooks/post-checkout` to
   `templates/shuyi-default/base/scripts/git-hooks/post-checkout`. Its
   comments name the game's tools, not the engine's. Check that the
   template's file list (the composer's tests and any snapshot of a new
   game's files) takes the new file, and that `check-paths` passes.
2. `crates/cli/src/repos.rs`: in `make` for `Thing::Git`, after `git
   init`, run `git config core.hooksPath scripts/git-hooks` and `git
   update-index --chmod=+x` on each hook, both only when the folder
   exists. Have `observe` see an unset `core.hooksPath`, so a second
   run sets it on a checkout that lacks it. Every call goes through
   `Host`, so `repos/fake.rs` scripts them.
3. `crates/cli/src/contract/doctor.rs`:
   - Add the two optional checks beside `repos_checks`.
   - `Probe` gains what they need: `git config core.hooksPath` through
     `output`, and a way to read whether a folder is compressed (on
     Windows `MetadataExt::file_attributes()` and
     `FILE_ATTRIBUTE_COMPRESSED`, `0x800`, as xtask's `compress.rs`
     reads it for the engine's gates). `ThisMachine` implements it, and
     `Fake` scripts it.
   - Keep the module's doc comment in step.
4. Measure the alias and the debug info (the two **In** items) on a game
   fresh from `shuyi new --template grid`. Write the counts and the
   `objdump` output in the notes.
5. The docs.

## Acceptance criteria

- [x] A game fresh from `shuyi new` has
      `scripts/git-hooks/post-checkout`, and `cargo xtask check-paths`
      passes.
- [x] After `shuyi repos` (its fake host in the tests), `core.hooksPath`
      is `scripts/git-hooks` and the hook is committed as `100755`. A
      second run on a checkout without the setting sets it, and does
      nothing when the setting is there.
- [x] On Windows, `git worktree add` in a game whose hooks are set gives
      a `target/` with the compressed attribute. Checked by hand on
      Nick's machine; `compact /q` output in the notes.
- [x] `doctor` reports both new checks as optional. Unset hooks and an
      uncompressed `target/` each give their help line, and neither
      makes `doctor` fail or exit non-zero.
- [x] The notes give the alias's unit count and the `objdump` check,
      and say what changed, if anything.
- [x] `docs/templates.md` says what a hook file is under "no template
      runs code".
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`repos`, fake host): the hooks set after `git init`, set on a
  re-run when unset, left alone when set, and skipped when the game has
  no `scripts/git-hooks`.
- Unit (`doctor`, `Fake` probe): each new check passing, failing with
  its help, marked optional, and not counted in `doctor`'s verdict.
- Template: a new game's file list includes the hook (the existing
  composer test).
- Manual, in the notes: the compression after `git worktree add`, the
  unit counts and `objdump -h`.

## Completion notes

What was done:

- `templates/shuyi-default/base/scripts/git-hooks/post-checkout`: the
  engine's hook, its comments naming `cargo shuyi doctor` and `shuyi
  repos`. It is committed `100755` here too. The composer's golden list
  (`crates/cli/src/template/plans.golden`) takes it, and `check-paths`
  passes.
- `shuyi repos`: a new step, `hooks`, after `git`, planned only when the
  game has `scripts/git-hooks`. `observe` reads `git config --get
  core.hooksPath` (`State::hooks`: absent, unset, set, or elsewhere). The
  step sets the path when it is unset. The `git` step marks each hook
  `--chmod=+x` between `git add` and the first commit. A path of the
  developer's own is left alone and reported as left.
- `doctor`: two optional checks, `git-hooks` and `target-compression`,
  run in the git checkout `doctor` is run in (`git rev-parse
  --show-toplevel`). `Probe` gains two default methods, `is_folder` and
  `compressed` (`FILE_ATTRIBUTE_COMPRESSED`, `0x800`, now a public
  const of `doctor`). `ThisMachine` implements both, and the tests' `Fake`
  scripts both. An absolute `core.hooksPath` that ends in
  `scripts/git-hooks` passes too, because a worktree shares its main
  checkout's setting, which may name that checkout's folder in full (the
  engine's own setting does).
- Docs: `docs/templates.md` (what a hook file is under "no template runs
  code"), `docs/contract.md`'s `doctor` row, the template's `AGENTS.md`
  (a section on git's hooks) and `README.md` (its file list, and setting
  the hooks once per clone), and `repos --help`.

Measured on a game fresh from `shuyi new --template grid --engine-path
<this worktree>`, on Nick's machine (Windows 11, GNU toolchain), at
`c528e7b2` plus this branch:

- **The alias.** `cargo test --workspace` compiled 112 units. Then
  `cargo run -p moon-garden --example tools --features tools -- check`
  (the alias without `-q`, so cargo prints its `Compiling` lines)
  compiled 2: `shuyi-cli` and `moon-garden`. Both are the workspace's or
  the engine's own crates, with a different feature set (`tools`). No
  third-party unit was built twice, so no form of the alias can build
  fewer. The alias is kept.
- **Debug info.** `objdump -h` on each test program in
  `target/debug/deps` (game, it, layers, moon_garden ×2, rules, stage)
  and on `target/debug/examples/tools.exe` shows no `.debug_*` section.
  The sections of `tools.exe` are `.text .data .rdata .pdata .xdata
  .bss .idata .rsrc .reloc`. `strip = "debuginfo"` would add nothing, so
  the template's profiles are unchanged.
- **Compression.** I ran `git init`, `git add`, `git update-index
  --chmod=+x`, the commit and `git config core.hooksPath
  scripts/git-hooks` by hand in that game (`repos`' own steps), then
  `git worktree add ../moon-wt`. `git ls-files -s` shows the hook as
  `100755`. `compact /q target` in the new worktree said:

  ```text
  Of 1 files within 1 directories
  1 are compressed and 0 are not compressed.
  ```

  With `core.autocrlf=true` (Git for Windows' default), the worktree's
  copy of the hook is checked out with CRLF, because the game has no
  `.gitattributes`. Git for Windows' `sh` runs it anyway: `sh -x` on
  the CRLF copy made and compressed `target/` and exited 0. A clone on
  Linux or macOS gets LF.

Deviations:

- The ticket says `observe` should see an unset `core.hooksPath`. It also
  sees one set to another folder, and leaves that one alone. That keeps
  `repos` from overwriting hooks the developer chose.
- The manual check ran on Nick's machine, in this session, not by Nick.

Breaking: none. `Probe`'s two new methods have defaults, and `State`
and `Thing` gain a field and a variant. `State` is built by `observe`
and `Thing` is matched only inside `repos`. The api job will say if it
disagrees.

Behaviour: shuyi-cli: `shuyi repos` on a game with `scripts/git-hooks` reports a step `hooks` after `git`, and sets `core.hooksPath`; `shuyi doctor` in a git checkout lists the optional checks `git-hooks` and, on Windows, `target-compression`.

Book: the quick start's "On GitHub" (`site/content/learn/book/quick-start.md`) says `repos` sets the hooks and how to set them in a clone.

Game: nothing to adopt. The game has its own hooks and xtask, and neither
`repos` nor the template reaches it. `doctor`'s new checks are optional,
and pass in the game's checkout once its hooks are set.
